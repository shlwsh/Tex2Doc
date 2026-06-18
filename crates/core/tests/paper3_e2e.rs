//! 端到端验证：把 `examples/paper3/latex/main-jos.tex` 转为 docx 并落到
//! `examples/paper3/output/main-jos-rust.docx`。
//!
//! 运行方式：
//! ```text
//! cargo test -p doc-core --test paper3_e2e -- --nocapture
//! ```
//!
//! 真实调用 [`doc_core::convert_dir`]：挂载项目根 → include 拓扑 → 拼接 →
//! 解析 → 降级 → docx 打包。

use std::path::PathBuf;
use std::time::Instant;

use doc_core::{convert_dir, ConvertOptions};
use doc_latex_reader::{lower_to_document, parse_tex, IncludeGraph};
use doc_semantic_ast::Block;
use doc_utils::VirtualFs;

fn paper3_paths() -> (PathBuf, PathBuf, PathBuf) {
    // tests/ 在 crates/core/tests/ 下，向上三级到仓库根
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(root.pop(), "CARGO_MANIFEST_DIR 解析失败");
    assert!(root.pop(), "CARGO_MANIFEST_DIR 解析失败");
    let project_root = root.join("examples").join("paper3").join("latex");
    let main_tex = project_root.join("main-jos.tex");
    let out_dir = root.join("examples").join("paper3").join("output");
    (project_root, main_tex, out_dir)
}

#[test]
fn paper3_main_jos_to_docx() {
    let (project_root, main_tex, out_dir) = paper3_paths();
    assert!(
        project_root.is_dir(),
        "项目根目录不存在：{}",
        project_root.display()
    );
    assert!(main_tex.is_file(), "主 tex 不存在：{}", main_tex.display());

    let opts = ConvertOptions::default();
    let started = Instant::now();
    let result = convert_dir(&project_root, &main_tex, &opts).expect("convert_dir 失败");
    let elapsed = started.elapsed();

    // 基本健全性检查：docx 是合法 zip，必要部件在
    assert!(!result.docx.is_empty(), "docx 字节流为空");
    assert_eq!(
        &result.docx[..4],
        b"PK\x03\x04",
        "docx 缺少 zip 魔数，转换可能未完成"
    );
    for needle in [
        b"word/document.xml".as_slice(),
        b"word/styles.xml".as_slice(),
    ] {
        assert!(
            result.docx.windows(needle.len()).any(|w| w == needle),
            "docx 包内未找到 {needle:?}"
        );
    }
    let mut docx_zip =
        zip::ZipArchive::new(std::io::Cursor::new(&result.docx)).expect("docx zip 可打开");
    let media_count = docx_zip
        .file_names()
        .filter(|name| name.starts_with("word/media/"))
        .count();
    assert_eq!(media_count, 10, "paper3 应嵌入 10 张 figure 图片");
    let rels = {
        let mut rels_file = docx_zip
            .by_name("word/_rels/document.xml.rels")
            .expect("document.xml.rels 存在");
        let mut s = String::new();
        use std::io::Read;
        rels_file.read_to_string(&mut s).expect("读取 rels");
        s
    };
    assert_eq!(
        rels.matches("relationships/image").count(),
        10,
        "paper3 应生成 10 个图片 relationship"
    );

    // ===== 结构性 + 内容性断言：把"乱码"做成可回归的硬性检查 =====
    let mut vfs = VirtualFs::new();
    vfs.mount_dir(&project_root).expect("mount_dir");
    let main_rel = "main-jos.tex";
    let graph = IncludeGraph::build(&vfs, std::path::Path::new(main_rel)).expect("include graph");
    let joined = graph.join(&vfs).expect("include join");
    let parse = parse_tex(&joined.text);
    let doc = lower_to_document(&parse, Some(&joined));

    // 1) 块统计
    let mut para_count = 0usize;
    let mut list_count = 0usize;
    let mut eq_count = 0usize;
    let mut fig_count = 0usize;
    let mut tbl_count = 0usize;
    let mut heading_count = 0usize;
    let mut raw_count = 0usize;
    for b in &doc.blocks {
        match b {
            Block::Paragraph { runs, .. } => {
                para_count += 1;
                // 调试：dump 每个 paragraph 的 plain text
                let text: String = runs.iter().map(|r| r.text.clone()).collect();
                eprintln!(
                    "  [P{para_count:3}] {}",
                    text.chars().take(80).collect::<String>()
                );
            }
            Block::List { .. } => list_count += 1,
            Block::Equation { .. } => eq_count += 1,
            Block::Figure { path, .. } => {
                fig_count += 1;
                eprintln!("  [FIG] path={path:?}");
            }
            Block::Table { .. } => tbl_count += 1,
            Block::Heading {
                text,
                number,
                level,
                ..
            } => {
                heading_count += 1;
                if let Some(n) = number {
                    eprintln!("  [H{level}] {n} {text}");
                } else {
                    eprintln!("  [H{level}] (no num) {text}");
                }
            }
            Block::RawFallback { .. } => raw_count += 1,
            _ => {}
        }
    }
    eprintln!(
        "📊 块统计：para={para_count} list={list_count} eq={eq_count} fig={fig_count} tbl={tbl_count} h={heading_count} raw={raw_count}"
    );
    // 合理性约束：不能整篇都是 paragraph（结构必须生效）
    assert!(para_count >= 1, "应当至少有一段正文（中文摘要等）");
    assert!(
        list_count >= 1,
        "中文参考文献的 description 列表应被识别为 List"
    );
    // 论文实验/结论里必然有公式；至少有 1 个公式块
    assert!(eq_count >= 1, "正文应至少包含 1 个公式");

    // 2) 文本内容：把全部段落文本拼成单串，做关键短语断言
    let mut all_text = String::new();
    for b in &doc.blocks {
        match b {
            Block::Paragraph { runs, .. } => {
                for r in runs {
                    all_text.push_str(&r.text);
                    all_text.push('\n');
                }
            }
            Block::List { items, .. } => {
                for item in items {
                    for sub in item {
                        if let Block::Paragraph { runs, .. } = sub {
                            for r in runs {
                                all_text.push_str(&r.text);
                                all_text.push('\n');
                            }
                        }
                    }
                }
            }
            Block::Heading { text, .. } => {
                all_text.push_str(text);
                all_text.push('\n');
            }
            Block::Equation { latex, .. } => {
                all_text.push_str(latex);
                all_text.push('\n');
            }
            _ => {}
        }
    }

    // V2：把 metadata 也并入 all_text（标题/作者/摘要等）
    if let Some(t) = &doc.metadata.title {
        all_text.push_str(t);
        all_text.push('\n');
    }
    for a in &doc.metadata.authors {
        all_text.push_str(a);
        all_text.push('\n');
    }
    if let Some(abs) = &doc.metadata.abstract_text {
        all_text.push_str(abs);
        all_text.push('\n');
    }
    for k in &doc.metadata.keywords {
        all_text.push_str(k);
        all_text.push('\n');
    }
    if let Some(cz) = &doc.metadata.citation_zh {
        all_text.push_str(cz);
        all_text.push('\n');
    }

    // 中文摘要关键短语（从 00_abstract.tex 拷贝出来防漂移）
    assert!(
        all_text.contains("微服务架构下"),
        "中文摘要关键短语缺失：'微服务架构下'"
    );
    assert!(
        all_text.contains("网关"),
        "正文应多次出现 '网关'（论文主题）"
    );
    assert!(
        all_text.contains("Grafana Loki") || all_text.contains("Loki"),
        "正文应提到 Grafana Loki / Loki"
    );
    // rjthesis 模板专有结构：作者姓名「石洪雷」+「赵涓涓」
    assert!(all_text.contains("石洪雷"), "作者 '石洪雷' 应出现");
    assert!(all_text.contains("赵涓涓"), "作者 '赵涓涓' 应出现");

    // 3) 反向断言：以下杂质命令名/LaTeX 痕迹**不应**再出现在文本中
    for forbid in [
        "\\AbstractContentZh",
        "\\AbstractContentEn",
        "\\KeywordsZh",
        "\\KeywordsEn",
        "\\documentclass",
        "\\usepackage",
        "\\PassOptionsToClass",
        "\\geometry",
        "\\begin{CJK}",
        "\\end{CJK}",
        "\\hypersetup",
        "\\setlength",
        "\\providecommand",
        "\\newcommand",
        "\\renewcommand",
        "\\fancyhead",
        "\\rjtitle",
        "\\rjauthor",
        "\\rjinfor",
        "\\rjkeywords",
        "\\rjcategory",
        "\\rjmaketitle",
        "\\bibliographystyle",
        "\\bibliography{",
        "\\CCT",
        "\\selectfont",
        "\\fontsize",
        "\\songti",
        "\\kaishu",
        "\\fangsong",
        "\\lishu",
        "\\heiti",
        "\\wuhao",
        "\\xiaowuhao",
        "{ctexart}",
        "{rjthesis}",
    ] {
        assert!(
            !all_text.contains(forbid),
            "正文仍包含 LaTeX 杂质：{forbid:?}（lower 阶段应当剥离）"
        );
    }

    std::fs::create_dir_all(&out_dir).expect("创建输出目录失败");
    // v12: 优先用 DOCX 环境变量指定输出路径,否则回退到默认路径
    let out_file = if let Ok(env_path) = std::env::var("DOCX") {
        PathBuf::from(env_path)
    } else {
        out_dir.join("main-jos-rust.docx")
    };
    if let Some(parent) = out_file.parent() {
        std::fs::create_dir_all(parent).expect("创建 DOCX 父目录失败");
    }
    std::fs::write(&out_file, &result.docx).expect("写出 docx 失败");

    let size = result.docx.len();
    eprintln!(
        "✅ paper3 端到端验证通过：{} bytes -> {}（耗时 {:?}）",
        size,
        out_file.display(),
        elapsed
    );
    eprintln!(
        "   警告条数 = {}；前 3 条 = {:?}",
        result.warnings.len(),
        result.warnings.iter().take(3).collect::<Vec<_>>()
    );
}
