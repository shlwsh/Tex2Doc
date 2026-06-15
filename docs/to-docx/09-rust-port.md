# 09 · Rust 重构指南

> **目标读者**：要把 `scripts/build_docx.sh` 流水线用 Rust 重写的工程师。
> 本章提供 crate 选型、模块切分、数据结构、关键算法伪代码、与 Python 实现的对应表。读完本章你应能在 ~1500–2000 行 Rust 中复刻等价的 build+verify 工具。

## 9.1 总体建议

1. **保持现有 shell 入口薄**。Rust 重写后，shell 脚本只剩 `mvgit` 风格的几行：
   ```bash
   #!/usr/bin/env bash
   set -euo pipefail
   cargo run --release --quiet --bin p3docx -- build --root "$ROOT"
   cargo run --release --quiet --bin p3docx -- verify --root "$ROOT"
   ```
   或干脆删 shell，让 `make` / CI 直接调 `cargo run`。

2. **不要试图 1:1 模拟 Python 的 dataclass / 动态派发**。Rust 端用 `enum Block` + 模式匹配，写起来更短。

3. **docx 渲染用"字符串模板"而非 DOM**。原因是 OOXML 元素嵌套深、属性多，手写 DOM 既慢又难调试；模板字符串反而最容易维护和复盘。

4. **错误处理用 `anyhow::Result`**。避免 `Box<dyn Error>` 的麻烦。

5. **格式 JSON 是 single source of truth**。把它建模为 `FormatData` struct，所有数值都从它读，不要硬编码。

6. **不依赖 LibreOffice / Pandoc / Word**。`lopdf` + `image` + `zip` 三件套就够。

## 9.2 crate 选型

```toml
[dependencies]
anyhow        = "1"             # 错误处理
serde         = { version = "1", features = ["derive"] }
serde_json    = "1"             # 读格式定义 JSON
regex         = "1"             # LaTeX/引用/marker 匹配
zip           = { version = "0.6", default-features = false, features = ["deflate"] }
image         = { version = "0.24", default-features = false, features = ["png", "jpeg"] }
lopdf         = { version = "0.30", default-features = false }  # PDF 文本抽取（verify）
chrono        = { version = "0.4", default-features = false, features = ["clock"] }
clap          = { version = "4", features = ["derive"] }  # CLI
walkdir       = "2"             # 找 vN 文件算版本号
once_cell     = "1"             # 全局常量

[dev-dependencies]
pretty_assertions = "1"
```

> `xml` crate 看着诱人，但本项目**手写字符串模板**反而更可控。

## 9.3 模块切分

```text
crates/
└── p3docx/
    ├── Cargo.toml
    └── src/
        ├── lib.rs                  # 顶层模块导出
        ├── main.rs                 # CLI 入口（build / verify）
        ├── model.rs                # Manuscript / Block / AlgRow 等 IR 类型
        ├── format.rs               # 格式 JSON 解析 + DocxProfile + 样式常量
        ├── tex/
        │   ├── mod.rs
        │   ├── lexer.rs            # find_matching_brace, command_arg, strip_comments
        │   ├── expander.rs         # expand_inputs, parse_newcommands
        │   ├── bbl.rs              # parse_bbl
        │   ├── normalizer.rs       # latex_to_text, clean_math
        │   ├── parser.rs           # parse_sections, parse_table/figure/algorithm/equation
        │   └── frontmatter.rs      # extract_* 系列
        ├── builder/
        │   ├── mod.rs              # DocxBuilder 主类
        │   ├── styles.rs           # styles_xml + 21 个 style() 模板
        │   ├── paragraph.rs        # add_paragraph / add_kept_paragraph / add_spacer
        │   ├── table.rs            # add_table + 边框/单元格模板
        │   ├── image.rs            # add_image + EMU 计算
        │   ├── algorithm.rs        # 渲染 algorithm block
        │   ├── header_footer.rs    # 三个 header + 三个 footer
        │   └── inline.rs           # inline_runs_xml + 上下标切 run
        ├── package.rs              # [Content_Types].xml + .rels + ZIP 写盘
        ├── verify.rs               # 33 项校验
        └── shell_compat.rs         # 复刻 shell 入口行为（check_deps / 版本号 / 命名）
```

文件总行数估计：~1800 行 Rust（含注释与空白）。

## 9.4 数据结构

```rust
// model.rs

#[derive(Debug, Clone)]
pub struct Manuscript {
    pub title_zh: String,
    pub authors_zh: String,
    pub institute_lines: Vec<String>,
    pub abstract_zh: String,
    pub keywords_zh: String,
    pub category: String,
    pub citation_zh: String,
    pub citation_en: String,
    pub title_en: String,
    pub authors_en: String,
    pub institute_en: String,
    pub abstract_en: String,
    pub keywords_en: String,
    pub running_header: String,
    pub first_footer_text: String,
    pub blocks: Vec<Block>,
    pub references: Vec<String>,       // 英文文献，按 bbl 顺序
    pub cn_references: Vec<String>,
    pub author_bio: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Block {
    Heading { level: u8, text: String },
    Paragraph { text: String },
    ListItem { text: String },
    Table { caption: String, label: String, rows: Vec<Vec<String>> },
    Figure { caption: String, label: String, image_path: Option<std::path::PathBuf>, width_factor: f32 },
    Algorithm { caption: String, label: String, lines: Vec<String>, io: Vec<(String, String)>, rows: Vec<AlgRow> },
    Equation { text: String, label: String, caption: String },
}

#[derive(Debug, Clone)]
pub struct AlgRow {
    pub line_no: u32,
    pub indent: u8,
    pub guides: Vec<u8>,
    pub end_guides: Vec<u8>,
    pub code: String,
    pub comment: String,
    pub keyword: Option<String>,
}
```

## 9.5 关键算法伪代码

### 9.5.1 `find_matching_brace`

```rust
pub fn find_matching_brace(text: &str, open_index: usize) -> Result<usize, ParseError> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut i = open_index;
    while i < bytes.len() {
        let c = bytes[i] as char;
        let prev = if i == 0 { '\0' } else { bytes[i - 1] as char };
        if c == '{' && prev != '\\' {
            depth += 1;
        } else if c == '}' && prev != '\\' {
            if depth == 0 {
                return Err(ParseError::UnmatchedBrace(i));
            }
            depth -= 1;
            if depth == 0 { return Ok(i); }
        }
        i += 1;
    }
    Err(ParseError::UnclosedBrace(open_index))
}
```

### 9.5.2 `command_arg`

```rust
pub fn command_arg(text: &str, command: &str, start: usize) -> Option<CommandArg> {
    let token = format!("\\{}", command);
    let pos = text[start..].find(&token)? + start;
    let brace_rel = text[pos + token.len()..].find('{')?;
    let brace = pos + token.len() + brace_rel;
    let end = find_matching_brace(text, brace).ok()?;
    Some(CommandArg {
        inner: text[brace + 1..end].to_string(),
        cmd_start: pos,
        after: end + 1,
    })
}

pub struct CommandArg {
    pub inner: String,
    pub cmd_start: usize,
    pub after: usize,
}
```

### 9.5.3 `parse_algorithm_rows` 状态机

完整 ~120 行 Rust，复刻 §2.13.1。**建议直接照搬** Python 的循环结构，Rust 改写时只换 mut 变量与 Option 包装。

```rust
struct RowBuilder {
    rows: Vec<AlgRow>,
    counter: u32,
}

fn parse_algorithm_rows(
    source: &str,
    cite_map: &HashMap<String, usize>,
    label_map: &HashMap<String, String>,
    pos: usize,
    indent: u8,
    active_guides: Vec<u8>,
    builder: &mut RowBuilder,
) -> usize {
    let mut cur = pos;
    let mut statement_start = cur;

    let emit = |raw: &str, builder: &mut RowBuilder, indent: u8, active_guides: &Vec<u8>| {
        let (text, comment) = algorithm_statement_text(raw, cite_map, label_map);
        if text.is_empty() { return; }
        builder.counter += 1;
        builder.rows.push(AlgRow {
            line_no: builder.counter,
            indent,
            guides: active_guides.clone(),
            end_guides: vec![],
            code: text,
            comment,
            keyword: None,
        });
    };

    while cur < source.len() {
        if source.as_bytes()[cur] == b'}' {
            emit(&source[statement_start..cur], builder, indent, &active_guides);
            return cur + 1;
        }
        if source[cur..].starts_with("\\ForEach") || source[cur..].starts_with("\\If") {
            emit(&source[statement_start..cur], builder, indent, &active_guides);
            let cmd = if source[cur..].starts_with("\\ForEach") { "ForEach" } else { "If" };
            let parsed = command_args_at(source, cur, cmd, 2).expect("malformed");
            let cond = latex_to_text(&parsed.args[0], cite_map, label_map);
            let code = if cmd == "ForEach" { format!("foreach {} do", cond) } else { format!("if {} then", cond) };
            builder.counter += 1;
            builder.rows.push(AlgRow { line_no: builder.counter, indent, guides: active_guides.clone(), end_guides: vec![], code, comment: String::new(), keyword: Some(cmd.into()) });
            let mut new_guides = active_guides.clone();
            new_guides.push(indent);
            let new_indent = indent + 1;
            let end_in_body = parse_algorithm_rows(&parsed.args[1], cite_map, label_map, 0, new_indent, new_guides, builder);
            if let Some(last) = builder.rows.last_mut() {
                if last.line_no == builder.counter {  // 最后一行是递归中的最后一条
                    last.end_guides.push(indent);
                }
            }
            cur = parsed.after;
            statement_start = cur;
            continue;
        }
        // ... 同理处理 \Return 和 \;
        cur += 1;
    }
    emit(&source[statement_start..], builder, indent, &active_guides);
    cur
}
```

### 9.5.4 `inline_runs_xml` 状态机

```rust
pub fn inline_runs_xml(text: &str, enable_superscript: bool, enable_subscript: bool) -> String {
    if !enable_superscript { return run_xml(text, false, false); }
    let mut out = String::new();
    let mut buf = String::new();
    let mut chars = text.char_indices().peekable();
    let citation_re = Regex::new(r"^\[[0-9][0-9,\-\s]*\]").unwrap();
    let sup_brace_re = Regex::new(r"^\^\{([^{}]+)\}").unwrap();
    let sup_single_re = Regex::new(r"^\^([A-Za-z0-9*]+)").unwrap();
    let sub_brace_re = Regex::new(r"^_\{([^{}]+)\}").unwrap();
    let sub_single_re = Regex::new(r"^_([A-Za-z0-9]+)").unwrap();

    while let Some((i, c)) = chars.next() {
        let rest = &text[i..];
        if let Some(m) = citation_re.find(rest) {
            if !buf.is_empty() { out.push_str(&run_xml(&buf, false, false)); buf.clear(); }
            out.push_str(&run_xml(m.as_str(), true, false));
            for _ in 0..m.end() { chars.next(); }
            continue;
        }
        if c == '^' {
            if let Some(m) = sup_brace_re.captures(rest) {
                if !buf.is_empty() { out.push_str(&run_xml(&buf, false, false)); buf.clear(); }
                out.push_str(&run_xml(&m[1], true, false));
                for _ in 0..m.get(0).unwrap().len() { chars.next(); }
                continue;
            }
            if let Some(m) = sup_single_re.captures(rest) {
                if !buf.is_empty() { out.push_str(&run_xml(&buf, false, false)); buf.clear(); }
                out.push_str(&run_xml(&m[1], true, false));
                for _ in 0..m.get(0).unwrap().len() { chars.next(); }
                continue;
            }
        }
        if enable_subscript && c == '_' {
            if let Some(m) = sub_brace_re.captures(rest) {
                if !buf.is_empty() { out.push_str(&run_xml(&buf, false, false)); buf.clear(); }
                out.push_str(&run_xml(&m[1], false, true));
                for _ in 0..m.get(0).unwrap().len() { chars.next(); }
                continue;
            }
            if let Some(m) = sub_single_re.captures(rest) {
                if !buf.is_empty() { out.push_str(&run_xml(&buf, false, false)); buf.clear(); }
                out.push_str(&run_xml(&m[1], false, true));
                for _ in 0..m.get(0).unwrap().len() { chars.next(); }
                continue;
            }
        }
        buf.push(c);
    }
    if !buf.is_empty() { out.push_str(&run_xml(&buf, false, false)); }
    out
}

fn run_xml(text: &str, sup: bool, sub: bool) -> String {
    if text.is_empty() { return String::new(); }
    let rpr = match (sup, sub) {
        (true, _) => Some(r#"<w:vertAlign w:val="superscript"/>"#),
        (_, true) => Some(r#"<w:vertAlign w:val="subscript"/>"#),
        _ => None,
    };
    match rpr {
        Some(r) => format!(r#"<w:r><w:rPr>{}</w:rPr><w:t xml:space="preserve">{}</w:t></w:r>"#, r, xml_escape(text)),
        None => format!(r#"<w:r><w:t xml:space="preserve">{}</w:t></w:r>"#, xml_escape(text)),
    }
}
```

### 9.5.5 文档主体

```rust
pub fn populate(builder: &mut DocxBuilder, ms: &Manuscript, profile: &DocxProfile) {
    builder.add_paragraph(&ms.title_zh, "JOSTitleZh", Some("left"));
    builder.add_paragraph(&ms.authors_zh, "JOSAuthorZh", Some("left"));
    for line in &ms.institute_lines {
        builder.add_paragraph(line, "JOSInstituteZh", Some("left"));
    }
    builder.add_spacer(profile.after_institute_twips);
    builder.add_paragraph(&format!("{} {}", profile.zh_abstract_label, ms.abstract_zh), "JOSAbstractZh", None);
    builder.add_paragraph(&format!("{} {}", profile.zh_keywords_label, spaced_keywords(&ms.keywords_zh)), "JOSKeywords", None);
    builder.add_paragraph(&format!("{} {}", profile.category_label, ms.category), "JOSBodyNoIndent", None);
    builder.add_spacer(profile.before_citation_twips);

    for line in wrap_text_units(&ms.citation_zh, profile.citation_wrap_units) {
        builder.add_paragraph(&line, "JOSCitation", None);
    }
    for line in wrap_text_units(&ms.citation_en, profile.citation_wrap_units) {
        builder.add_paragraph(&line, "JOSCitation", None);
    }
    builder.add_spacer(profile.before_english_title_twips);
    builder.add_paragraph(&ms.title_en, "JOSEnglishTitle", None);
    builder.add_paragraph(&ms.authors_en, "JOSCitation", None);
    builder.add_paragraph(&ms.institute_en, "JOSCitation", None);
    builder.add_spacer(profile.before_english_abstract_twips);
    builder.add_paragraph(&format!("{}   {}", profile.en_abstract_label, ms.abstract_en), "JOSAbstractEn", None);
    builder.add_paragraph(&format!("{} {}", profile.en_keywords_label, spaced_keywords(&ms.keywords_en)), "JOSKeywords", None);

    for block in &ms.blocks {
        match block {
            Block::Heading { level, text } => {
                let style = match level { 1 => "JOSHeading1", 2 => "JOSHeading2", _ => "JOSHeading3" };
                builder.add_paragraph(text, style, None);
            }
            Block::Paragraph { text } | Block::ListItem { text } => {
                builder.add_paragraph(text, "JOSBody", None);
            }
            Block::Table { caption, rows, .. } => {
                if !caption.is_empty() {
                    builder.add_kept_paragraph(caption, "JOSCaption", Some("center"), true, true);
                }
                builder.add_table(rows);
            }
            Block::Figure { caption, image_path, width_factor, .. } => {
                builder.add_image(image_path.as_deref(), *width_factor, caption);
                builder.add_paragraph(caption, "JOSCaption", Some("center"));
            }
            Block::Algorithm { caption, lines, .. } => {
                builder.add_kept_paragraph(caption, "JOSCaption", Some("center"), !lines.is_empty(), true);
                for (i, line) in lines.iter().enumerate() {
                    builder.add_kept_paragraph(line, "JOSCode", None, i + 1 < lines.len(), true);
                }
            }
            Block::Equation { text, caption, .. } => {
                let suffix = if caption.is_empty() { String::new() } else { format!("    {}", caption) };
                builder.add_paragraph(&format!("{}{}", text, suffix), "JOSCode", Some("center"));
            }
        }
    }

    builder.add_paragraph("本文撰写与实验脚本生成过程中使用了大语言模型辅助，作者对全部内容与数据负责。", "JOSBody", None);
    builder.add_paragraph("References", "JOSReferenceHeading", None);
    for (i, r) in ms.references.iter().enumerate() {
        builder.add_paragraph(&format!("[{}] {}", i + 1, r), "JOSReference", None);
    }
    builder.add_paragraph("附中文参考文献:", "JOSReferenceHeading", None);
    for r in &ms.cn_references {
        builder.add_paragraph(r, "JOSReference", None);
    }
    builder.add_paragraph("作者简介", "JOSReferenceHeading", None);
    for b in &ms.author_bio {
        builder.add_paragraph(b, "JOSReference", None);
    }
}
```

## 9.6 单元测试策略

写完一个函数就写一组对照测试（推荐 `pretty_assertions`）：

| 函数 | 测试用例 |
|------|---------|
| `find_matching_brace` | `"{a{b}c}"` start 0 → 6；嵌套 5 层；`\}` 不算闭合 |
| `command_arg` | `\foo{bar}` → inner="bar"；`\foo{bar{nested}}` → inner="bar{nested}" |
| `strip_comments` | 整行 `% x` 删除；`% not escaped` 之后删除；`\%` 保留 |
| `expand_inputs` | 递归展开；文件不存在返回空串 |
| `parse_bbl` | 51 条条目；`[opt]` 形式兼容；空 bbl 返回空 |
| `compress_numbers` | `[1,2,3]→"1-3"`；`[3,1,2]→"1-3"`；`[1,2,4,5,7]→"1-2,4-5,7"` |
| `clean_math` | `L=\{l_1,\ldots,l_N\}` → `L={l_1,…,l_N}`；`\pm` → `±` |
| `inline_runs_xml` | `Top-$K$` 切上标；`x_1` 切下标；`[1-2]` 切上标 |
| `wrap_text_units` | 52 units 截断；URL 单独 token |
| `populate` | front matter 段落数 = 17；诚信声明 1 条；References + 中文参考 + 作者简介各 1 标题 |
| `verify_jos_docx` | 8 张图、6 个表、22 marker 全覆盖、LaTeX 残留=0 |

## 9.7 与 Python 实现的对应表

| Python 函数 | Rust 模块 | 备注 |
|------------|----------|------|
| `check_deps` | `shell_compat::check_deps` | 用 `which` crate 或自己 `Command::new` |
| `next_version` | `shell_compat::next_version` | walkdir |
| `xml` | `xml_escape` | |
| `run_xml` / `direct_run_xml` | `builder::inline::run_xml` | |
| `inline_runs_xml` | `builder::inline::inline_runs_xml` | |
| `table_inline_runs_xml` | `builder::inline::table_inline_runs_xml` | |
| `clean_formula_display_text` | `tex::normalizer::clean_formula_display_text` | |
| `fix_display_text` | `tex::normalizer::fix_display_text` | |
| `read_text` | `fs::read_to_string` | |
| `strip_comments` | `tex::lexer::strip_comments` | |
| `find_matching_brace` | `tex::lexer::find_matching_brace` | |
| `command_arg` | `tex::lexer::command_arg` | |
| `replace_command_arg` | `tex::normalizer::replace_command_arg` | |
| `parse_newcommands` | `tex::expander::parse_newcommands` | |
| `parse_bbl` / `clean_bibitem` | `tex::bbl::*` | |
| `compress_numbers` | `tex::normalizer::compress_numbers` | |
| `clean_math` | `tex::normalizer::clean_math` | |
| `latex_to_text` | `tex::normalizer::latex_to_text` | 递归实现 |
| `expand_inputs` | `tex::expander::expand_inputs` | 递归实现 |
| `find_envs` | `tex::lexer::find_envs` | |
| `collect_labels` | `tex::parser::collect_labels` | |
| `extract_tabular` | `tex::parser::extract_tabular` | |
| `split_cells` | `tex::parser::split_cells` | |
| `parse_table` | `tex::parser::parse_table` | |
| `parse_figure` | `tex::parser::parse_figure` | |
| `maybe_convert_pdf` | `tex::parser::maybe_convert_pdf` | 用 `pdfium-render` |
| `command_args_at` | `tex::lexer::command_args_at` | |
| `strip_algorithm_metadata` | `tex::parser::strip_algorithm_metadata` | |
| `algorithm_statement_text` | `tex::parser::algorithm_statement_text` | |
| `parse_algorithm_rows` | `tex::parser::parse_algorithm_rows` | 见 §9.5.3 |
| `parse_algorithm` | `tex::parser::parse_algorithm` | |
| `parse_equation` | `tex::parser::parse_equation` | |
| `add_text_blocks` | `tex::parser::add_text_blocks` | |
| `parse_enumerate` | `tex::parser::parse_enumerate` | |
| `parse_sections` | `tex::parser::parse_sections` | 核心循环 |
| `extract_cn_references` | `tex::frontmatter::extract_cn_references` | |
| `extract_author_bio` | `tex::frontmatter::extract_author_bio` | |
| `extract_line_with` | `tex::frontmatter::extract_line_with` | |
| `normalize_inline_text` | `tex::frontmatter::normalize_inline_text` | |
| `normalize_institute_line` | `tex::frontmatter::normalize_institute_line` | |
| `extract_command_text` | `tex::frontmatter::extract_command_text` | |
| `extract_footnote_text` | `tex::frontmatter::extract_footnote_text` | |
| `extract_english_front_matter` | `tex::frontmatter::extract_english_front_matter` | |
| `first_author_name` | `tex::frontmatter::first_author_name` | |
| `derived_running_header` | `tex::frontmatter::derived_running_header` | |
| `build_manuscript` | `tex::build_manuscript` | 顶层组合 |
| `DocxBuilder` | `builder::DocxBuilder` | 主类 |
| `add_paragraph` / `add_kept_paragraph` | `builder::paragraph::*` | |
| `add_tabbed_paragraph` | `builder::paragraph::add_tabbed_paragraph` | |
| `add_spacer` | `builder::paragraph::add_spacer` | |
| `add_table` | `builder::table::add_table` | |
| `add_image` | `builder::image::add_image` | |
| `add_section_break` | `builder::paragraph::add_section_break` | |
| `document_xml` | `builder::DocxBuilder::document_xml` | |
| `rels_xml` | `builder::DocxBuilder::rels_xml` | |
| `style()` | `builder::styles::style` | 21 个调用 |
| `styles_xml` | `builder::styles::styles_xml` | |
| `content_types_xml` | `package::content_types_xml` | |
| `package_rels_xml` | `package::package_rels_xml` | |
| `settings_xml` | `package::settings_xml` | |
| `page_field_xml` | `builder::header_footer::page_field_xml` | |
| `header_xml` | `builder::header_footer::header_xml` | |
| `first_header_xml` | `builder::header_footer::first_header_xml` | |
| `footer_xml` | `builder::header_footer::footer_xml` | |
| `write_docx` | `package::write_docx` | 见 §6.10 |
| `spaced_keywords` | `tex::frontmatter::spaced_keywords` | |
| `token_width_units` | `tex::frontmatter::token_width_units` | |
| `wrap_text_units` | `tex::frontmatter::wrap_text_units` | |
| `split_citation_text` | `tex::frontmatter::split_citation_text` | |
| `populate` | `populate` | 顶层组合 |
| `main` | `main.rs::main` | CLI 入口 |
| verify 全部 | `verify::*` | 见 §8 |

## 9.8 CLI 设计

```rust
// main.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "p3docx", version, about = "Build/verify JOS-format DOCX from LaTeX")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Build {
        #[arg(long)] root: std::path::PathBuf,
        #[arg(long, default_value = "docs/format/jos_2025_docx_format_definitions.json")]
        format: std::path::PathBuf,
        #[arg(long)] version: Option<u32>,
    },
    Verify {
        #[arg(long)] root: std::path::PathBuf,
        #[arg(long)] docx: std::path::PathBuf,
        #[arg(long)] pdf: std::path::PathBuf,
        #[arg(long)] format: std::path::PathBuf,
        #[arg(long)] report: std::path::PathBuf,
        #[arg(long, long = "json-report")] json_report: Option<std::path::PathBuf>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Build { root, format, version } => {
            p3docx::build(&root, &format, version)?;
        }
        Cmd::Verify { root, docx, pdf, format, report, json_report } => {
            let passed = p3docx::verify(&root, &docx, &pdf, &format, &report, json_report.as_deref())?;
            if !passed { std::process::exit(1); }
        }
    }
    Ok(())
}
```

## 9.9 实施时间表（参考）

| 阶段 | 任务 | 代码量（估） |
|------|------|------------|
| 1 | model.rs + format.rs + 简单 main.rs 跑通"输出空 docx" | 300 |
| 2 | tex/lexer + expander + frontmatter（不依赖任何 docx 细节） | 400 |
| 3 | tex/parser（核心：parse_sections + table/figure/algorithm/equation） | 700 |
| 4 | builder/styles + paragraph + inline + table + image + header_footer | 600 |
| 5 | package.rs（写 ZIP） | 200 |
| 6 | shell_compat（check_deps + next_version） | 100 |
| 7 | verify.rs（33 项校验） | 500 |
| 8 | 单测 + 调试 | （不计入 LOC） |
| **合计** | | **~2800** |

## 9.10 验证"与 Python 等价"的方法

1. 在同一份 `latex/main-jos.tex` 上运行 Python 与 Rust 两版。
2. 用 `verify_jos_docx.py` 同时校验两份 docx——**都应通过**。
3. 对比 docx 的 `text` 字段（`pdftotext` 后的字符串），`normalize` 后应相等。
4. 对比图片尺寸、表格结构、上标/下标 run 数——这些是稳定指标。

如果有不一致，**先 diff `verify` 输出**——33 项校验的 `actual` 字段会精确指出哪里偏了。

## 9.11 已知坑

1. **rust 字符串模板里写 `<` `>` `&`** 时一定要 `xml_escape`——单元测试覆盖一下包含这些字符的 case。
2. **`image::open` 对某些 PNG 报错**——JOS 项目中所有 PNG 都是 matplotlib 出品的，标准 PNG，理论上不会出错。
3. **`lopdf` 中文支持**——若 PDF 用 CTeX 字体（CID 字体），`extract_text` 输出可能是空格乱码。建议先 normalize 然后做模糊比较。
4. **regex crate 不支持 lookahead/lookbehind 在所有 DFA 模式**——但本项目用的正则都很简单，没问题。
5. **`zip` crate 的 `start_file` 不可重复**——同名 part 多次调用会覆盖最后一次。Rust 实现务必保证每个 part 写一次。
6. **`zip` 默认压缩**会改变文件大小——可接受，但不要在测试里 hard-code 文件大小。

## 9.12 推荐的开发顺序

1. **写 model.rs**（300 行）——给所有数据结构定型。
2. **写 format.rs**（200 行）——读 JSON 到强类型。
3. **写 builder/styles.rs**（200 行）——独立可测试：输出一份固定 docx 检查 styles.xml 正确。
4. **写 package.rs**（200 行）——能写出"骨架 docx"（无正文）。
5. **写 tex/lexer + expander**（400 行）——纯函数，单元测试覆盖。
6. **写 tex/normalizer + bbl**（400 行）——纯函数，单元测试覆盖。
7. **写 tex/frontmatter**（200 行）——单测。
8. **写 tex/parser**（700 行）——这是最复杂的一块。
9. **写 populate + builder 其他**（500 行）——把 IR 串成 OOXML。
10. **写 verify**（500 行）——单测 + 与 Python 结果对比。
11. **收尾**——CLI、shell 兼容、错误处理。

完成这 11 步，你就有了一个**与 Python 实现语义等价、产物可被 Word 正确打开**的 Rust 工具链。
