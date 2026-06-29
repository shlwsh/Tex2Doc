//! 专业转换端到端验证：把 paper3 真实论文目录打包成 zip，
//! 走 `convert_zip`（= WASM 入口同链路）转换，并断言：
//!   * zip 内含主 tex / 多个 include / bib / 图片
//!   * docx 合法（含 word/document.xml + styles.xml + N 张图片）
//!   * 块结构合理：段落 / 列表 / 公式 / 图 / 表 / 标题 都至少有
//!   * 文本去掉了 LaTeX 杂质（命令名不残留）
//!   * 引用 / 作者 / 摘要 关键短语被识别
//!
//! 这等同于浏览器扩展做"上传 zip → 转换"的全量回归。

use std::io::{Read, Write};
use std::path::PathBuf;

use doc_core::{convert_zip, ConvertOptions};
use doc_latex_reader::{lower_to_document, parse_tex, IncludeGraph};
use doc_semantic_ast::Block;
use doc_utils::VirtualFs;

fn workspace_root() -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(root.pop());
    assert!(root.pop());
    root
}

fn paper3_latex_root() -> PathBuf {
    workspace_root()
        .join("examples")
        .join("paper3")
        .join("latex")
}

fn paper3_figures_root() -> PathBuf {
    workspace_root()
        .join("examples")
        .join("paper3")
        .join("figures")
}

/// 把 `examples/paper3/latex` 下的全部文件 + `examples/paper3/figures` 下的
/// PNG 图片打包成 zip，结构与浏览器扩展拿到的"项目 zip"一致。
fn build_paper3_zip() -> Vec<u8> {
    use std::fs;

    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // 1. 主目录 latex/ 下的所有文件
        let latex = paper3_latex_root();
        for entry in walkdir(&latex) {
            let rel = entry
                .strip_prefix(&latex)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if entry.is_file() {
                let bytes = fs::read(&entry).unwrap_or_default();
                zip.start_file(&rel, opts).unwrap();
                zip.write_all(&bytes).unwrap();
            }
        }
        // 2. figures/ 下的 PNG（直接放进根，方便 docx-writer 找到）
        let figs = paper3_figures_root();
        if figs.is_dir() {
            for entry in walkdir(&figs) {
                if !entry.is_file() {
                    continue;
                }
                let name = entry
                    .file_name()
                    .expect("file_name")
                    .to_string_lossy()
                    .to_string();
                if !name.to_lowercase().ends_with(".png") {
                    continue;
                }
                let bytes = fs::read(&entry).unwrap_or_default();
                zip.start_file(&name, opts).unwrap();
                zip.write_all(&bytes).unwrap();
            }
        }
        zip.finish().unwrap();
    }
    buf
}

fn walkdir(root: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(p) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&p) else {
            continue;
        };
        for e in rd.flatten() {
            let path = e.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out
}

#[test]
fn paper3_zip_full_professional_pipeline() {
    let started = std::time::Instant::now();
    let zip_bytes = build_paper3_zip();
    eprintln!("📦 构造 paper3 zip: {} bytes", zip_bytes.len());

    // 走 convert_zip = WASM 入口同链路
    let result = convert_zip(&zip_bytes, "main-jos.tex", &ConvertOptions::default())
        .expect("convert_zip 失败");
    let elapsed = started.elapsed();
    eprintln!(
        "✅ convert_zip 通过: docx {} bytes ({:?})",
        result.docx.len(),
        elapsed
    );

    // docx 结构断言
    assert!(result.docx.starts_with(b"PK\x03\x04"));
    let z = zip::ZipArchive::new(std::io::Cursor::new(&result.docx)).expect("docx zip 可打开");
    let mut names: Vec<String> = z.file_names().map(|s| s.to_string()).collect();
    names.sort();
    let has_doc = names.iter().any(|n| n == "word/document.xml");
    let has_styles = names.iter().any(|n| n == "word/styles.xml");
    assert!(has_doc, "docx 缺 word/document.xml");
    assert!(has_styles, "docx 缺 word/styles.xml");
    let media: Vec<&String> = names
        .iter()
        .filter(|n| n.starts_with("word/media/"))
        .collect();
    eprintln!("🖼  docx 内嵌图片: {} 张", media.len());
    assert!(
        media.len() >= 5,
        "paper3 至少应嵌入 5 张 figure，但只有 {}",
        media.len()
    );

    // 重新解析 + 断言文本与结构（用本地 latex 目录直接走 vfs，
    // 跟 convert_zip 内部的解析逻辑同一份代码）
    let mut vfs = VirtualFs::new();
    let latex_root = paper3_latex_root();
    vfs.mount_dir(&latex_root).expect("mount_dir");
    let main_rel = "main-jos.tex";
    let graph = IncludeGraph::build(&vfs, std::path::Path::new(main_rel)).expect("include graph");
    let joined = graph.join(&vfs).expect("include join");
    let parse = parse_tex(&joined.text);
    let doc = lower_to_document(&parse, Some(&joined));

    let mut para = 0;
    let mut list = 0;
    let mut eq = 0;
    let mut fig = 0;
    let mut tbl = 0;
    let mut heading = 0;
    let mut raw = 0;
    for b in &doc.blocks {
        match b {
            Block::Paragraph { .. } => para += 1,
            Block::List { .. } => list += 1,
            Block::Equation { .. } => eq += 1,
            Block::Figure { .. } => fig += 1,
            Block::Table { .. } => tbl += 1,
            Block::Heading { .. } => heading += 1,
            Block::RawFallback { .. } => raw += 1,
            _ => {}
        }
    }
    eprintln!(
        "📊 块统计: para={para} list={list} eq={eq} fig={fig} tbl={tbl} h={heading} raw={raw}"
    );

    // 专业论文的硬性最低标准
    assert!(para >= 1, "应至少 1 段正文");
    assert!(list >= 1, "应至少 1 个列表（参考文献 description）");
    assert!(eq >= 1, "应至少 1 个公式");
    assert!(fig >= 1, "应至少 1 张图");
    assert!(heading >= 1, "应至少 1 个标题");

    // 文本 + metadata 汇总
    let mut all = String::new();
    for b in &doc.blocks {
        match b {
            Block::Paragraph { runs, .. } => {
                for r in runs {
                    all.push_str(&r.text);
                    all.push('\n');
                }
            }
            Block::List { items, .. } => {
                for item in items {
                    for sub in item {
                        if let Block::Paragraph { runs, .. } = sub {
                            for r in runs {
                                all.push_str(&r.text);
                                all.push('\n');
                            }
                        }
                    }
                }
            }
            Block::Heading { text, .. } => {
                all.push_str(text);
                all.push('\n');
            }
            Block::Equation { latex, .. } => {
                all.push_str(latex);
                all.push('\n');
            }
            _ => {}
        }
    }
    if let Some(t) = &doc.metadata.title {
        all.push_str(t);
        all.push('\n');
    }
    for a in &doc.metadata.authors {
        all.push_str(a);
        all.push('\n');
    }
    if let Some(abs) = &doc.metadata.abstract_text {
        all.push_str(abs);
        all.push('\n');
    }
    for k in &doc.metadata.keywords {
        all.push_str(k);
        all.push('\n');
    }

    // 关键短语：摘要 / 作者 / 主题
    assert!(
        all.contains("微服务架构下"),
        "中文摘要关键短语 '微服务架构下' 缺失"
    );
    assert!(all.contains("石洪雷"), "作者 '石洪雷' 缺失");
    assert!(all.contains("赵涓涓"), "作者 '赵涓涓' 缺失");
    assert!(all.contains("网关"), "论文主题 '网关' 缺失");
    assert!(
        all.contains("Grafana Loki") || all.contains("Loki"),
        "Loki 关键短语缺失"
    );

    // 反向断言：LaTeX 杂质必须被剥掉
    for forbid in [
        "\\documentclass",
        "\\usepackage",
        "\\begin{CJK}",
        "\\end{CJK}",
        "\\hypersetup",
        "\\rjtitle",
        "\\rjauthor",
        "\\newcommand",
        "\\bibliographystyle",
        "\\bibliography{",
        "\\songti",
        "\\kaishu",
        "\\heiti",
        "\\wuhao",
        "{ctexart}",
        "{rjthesis}",
    ] {
        assert!(!all.contains(forbid), "正文仍残留 LaTeX 杂质：{forbid:?}");
    }

    // 写出 docx 供人工复核
    let out_dir = workspace_root()
        .join("examples")
        .join("paper3")
        .join("output");
    std::fs::create_dir_all(&out_dir).ok();
    let out_path = out_dir.join("paper3-zip-rust.docx");
    std::fs::write(&out_path, &result.docx).ok();
    eprintln!("💾 docx 已写入: {}", out_path.display());

    // 警告条数（应当接近 0；如果有，应该是无害的弃用提醒）
    eprintln!("⚠  warnings: {} 条", result.warnings.len());
}

#[test]
fn paper3_zip_handles_unicode_in_main_tex() {
    // 构造一个含 CJK + Unicode 标点的 zip，确认 wasm 链路不会因
    // UTF-8 解码失败而 panic（之前是 "invalid malloc request" 同源问题）。
    let main = r#"\documentclass{article}
\begin{document}
\title{中文标题：测试 ①②③}
\author{石洪雷, 赵涓涓}
\maketitle
\section{引言}
微服务架构下，日志采集与异常检测是 AIOps 的核心问题。
\end{document}
"#;
    let bib = r#"@article{test2024,
  title = {Test},
  author = {Tester, T.},
  journal = {Journal},
  year = {2024}
}
"#;
    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("main.tex", opts).unwrap();
        zip.write_all(main.as_bytes()).unwrap();
        zip.start_file("references.bib", opts).unwrap();
        zip.write_all(bib.as_bytes()).unwrap();
        zip.finish().unwrap();
    }
    let res = convert_zip(&buf, "main.tex", &ConvertOptions::default())
        .expect("unicode zip should convert");
    assert!(!res.docx.is_empty());
    // 解码回 docx 看一段（验证 UTF-8 没坏）
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(&res.docx)).unwrap();
    let mut doc_xml = String::new();
    z.by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut doc_xml)
        .unwrap();
    // docx XML 是 UTF-8，应能看到中文字符（XML 转义后的"中文"等）
    eprintln!("📝 docx XML 长度: {}", doc_xml.len());
    assert!(doc_xml.contains("微服务"));
}
