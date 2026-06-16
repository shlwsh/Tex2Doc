//! CST → Semantic AST 降级（M3 完整版）
//!
//! ## 扫描策略
//!
//! 1. **顶层行扫描**：`split_inclusive('\n')` 切行。
//! 2. **环境优先**：遇到 `\begin{xxx}…\end{xxx}` 直接整段扣出做专项降级。
//!    支持环境：`itemize` / `enumerate` / `description` / `tabular` / `array` / `figure` / `table` / `equation` / `equation*` / `algorithm` / `flushleft` 等。
//! 3. **段落**：非空行累计到一个 buffer，遇空行 / 段命令 / 新环境 / EOF 触发 flush。
//! 4. **段落内联清洗**：`strip_inline` 处理 `\textbf{...}` 等命令。
//! 5. **数学**：`$…$` 与 `$$…$$` / `\(...\)` / `\[...\]` 整段抽出为 `Equation::latex`。
//! 6. **图片**：`\includegraphics[…]{path}` 在段落中追加 Figure 占位（M3 简化）。
//! 7. **引用 / 链接**：`\href{url}{text}` / `\url{url}` / `\ref{label}` / `\nolinkurl{url}`。
//! 8. **错误降级**：未匹配内容进入 `Block::RawFallback`（绝不 panic）。

use doc_semantic_ast::{Block, Document, Span, TableCell, TableRow, TextRun, TextStyle};
use std::collections::HashMap;

use crate::expand::{expand_macros_in, expand_macros_with_input, MacroMap};
use crate::include::JoinedStream;
use crate::parser::Parse;

/// 编号状态（heading 1.1.1 / figure / table / algorithm 自动计数）。
#[derive(Default)]
pub struct NumberingState {
    /// Heading 计数器：level 1, 2, 3, 4 各自的计数
    heading_counters: [u32; 5],
    /// 下一个 figure 编号
    figure_counter: u32,
    /// 下一个 table 编号
    table_counter: u32,
    /// 下一个 algorithm 编号
    algorithm_counter: u32,
}

type PrefixHandler = fn(&str, Span, &mut NumberingState) -> Block;

/// 顶层段命令：\section / \subsection / \subsubsection / \paragraph / \caption
fn try_top_level_command(
    s: &str,
    span: Span,
    numbering: &mut NumberingState,
) -> Option<(usize, Block)> {
    let prefixes: &[(&str, PrefixHandler)] = &[
        ("\\section", |b, sp, n| Block::Heading {
            level: 1,
            text: b.to_string(),
            number: Some(n.next_heading(1)),
            span: sp,
        }),
        ("\\subsection", |b, sp, n| Block::Heading {
            level: 2,
            text: b.to_string(),
            number: Some(n.next_heading(2)),
            span: sp,
        }),
        ("\\subsubsection", |b, sp, n| Block::Heading {
            level: 3,
            text: b.to_string(),
            number: Some(n.next_heading(3)),
            span: sp,
        }),
        ("\\paragraph", |b, sp, n| Block::Heading {
            level: 4,
            text: b.to_string(),
            number: Some(n.next_heading(4)),
            span: sp,
        }),
        ("\\caption", |b, sp, _n| Block::Paragraph {
            runs: vec![TextRun {
                text: b.to_string(),
                style: TextStyle::default(),
                span: sp,
            }],
            span: sp,
        }),
    ];

    for (prefix, handler) in prefixes {
        if let Some(rest) = s.strip_prefix(prefix) {
            let trimmed = rest.trim();
            if trimmed.strip_prefix('{').is_some() {
                if let Some(end) = find_matching_brace(trimmed, 0) {
                    // end = 内部内容长度（ASCII-safe 情况下 = trimmed[1..end+1].len()）。
                    // slice [1..end+1] 包含完整内部。consumed = prefix + leading-whitespace + `{` + end + `}`
                    let slice_end = end + 1;
                    if slice_end > trimmed.len() || !trimmed.is_char_boundary(slice_end) {
                        return None;
                    }
                    let inner = &trimmed[1..slice_end];
                    let consumed = prefix.len() + (rest.len() - trimmed.len()) + end + 2;
                    return Some((consumed, handler(inner, span, numbering)));
                }
            }
            return Some((prefix.len(), handler("", span, numbering)));
        }
    }
    None
}

impl NumberingState {
    pub fn next_heading(&mut self, level: u8) -> String {
        let lvl = (level as usize).min(4);
        self.heading_counters[lvl] += 1;
        for i in (lvl + 1)..5 {
            self.heading_counters[i] = 0;
        }
        match lvl {
            1 => format!("{}", self.heading_counters[1]),
            2 => format!("{}.{}", self.heading_counters[1], self.heading_counters[2]),
            3 => format!(
                "{}.{}.{}",
                self.heading_counters[1], self.heading_counters[2], self.heading_counters[3]
            ),
            4 => format!(
                "{}.{}.{}.{}",
                self.heading_counters[1],
                self.heading_counters[2],
                self.heading_counters[3],
                self.heading_counters[4]
            ),
            _ => String::new(),
        }
    }

    pub fn next_figure(&mut self) -> String {
        self.figure_counter += 1;
        format!("图 {}", self.figure_counter)
    }

    pub fn next_table(&mut self) -> String {
        self.table_counter += 1;
        format!("表 {}", self.table_counter)
    }

    pub fn next_algorithm(&mut self) -> String {
        self.algorithm_counter += 1;
        // JOS 期刊约定：算法标题用 "Algorithm N: caption" 英文格式
        format!("Algorithm {}", self.algorithm_counter)
    }
}

/// 降级入口。
pub fn lower_to_document(parse: &Parse, joined: Option<&JoinedStream>) -> Document {
    // 内部新建宏表，自包含。
    let mut owned = MacroMap::new();
    lower_with_macros(parse, joined, &mut owned)
}

/// 共享宏表降级（外部可传入已收集的宏表）。
pub fn lower_with_macros(
    parse: &Parse,
    joined: Option<&JoinedStream>,
    macros: &mut MacroMap,
) -> Document {
    let mut numbering = NumberingState::default();
    lower_with_macros_and_numbering(parse, joined, macros, &mut numbering)
}

/// 共享宏表 + 编号状态降级（内部使用，保留编号状态便于测试）。
pub fn lower_with_macros_and_numbering(
    parse: &Parse,
    joined: Option<&JoinedStream>,
    macros: &mut MacroMap,
    numbering: &mut NumberingState,
) -> Document {
    let text = joined
        .map(|j| j.text.clone())
        .unwrap_or_else(|| parse.source.clone());
    // 第一步：VFS感知的宏展开。
    // 处理 \input{file} 递归，把子文件的 \newcommand 引入宏表，
    // 再对全文做宏展开，使 \AbstractContentZh 等环境内宏正确展开。
    let text = if let Some(j) = joined {
        expand_macros_with_input(j, &j.vfs, macros)
    } else {
        expand_macros_in(&text, macros)
    };
    // 第二步：跳过 preamble。
    let text = strip_preamble(&text);
    let mut doc = Document::new();
    let mut buffer = String::new();
    let mut buffer_start = 0u32;
    let default_span = Span::default();
    let mut pos: usize = 0;
    let bytes = text.as_bytes();
    let len = bytes.len();
    // Citation number tracking across the document
    let mut cite_numbers: HashMap<String, usize> = HashMap::new();

    let mut pos: usize = 0;
    let bytes = text.as_bytes();
    let len = bytes.len();
    // Citation number tracking across the document
    let mut cite_numbers: HashMap<String, usize> = HashMap::new();

    while pos < len {
        if !text.is_char_boundary(pos) {
            // 字节级推进到下一个 char 起点
            let mut next = pos + 1;
            while next < len && !text.is_char_boundary(next) {
                next += 1;
            }
            pos = next;
            continue;
        }

        // 跳过空白 / 注释
        if let Some(next) = skip_whitespace_and_comment(text, pos) {
            if next != pos {
                pos = next;
                continue;
            }
        }

        // 环境优先
        if let Some((name, body, end)) = scan_environment(text, pos) {
            flush_paragraph(
                &mut doc,
                &mut buffer,
                &mut buffer_start,
                default_span,
                macros,
            );
            // `flushleft` / `flushright` / `center` / `quote` / `quotation` / `verbatim`
            // 等"段落容器"环境内可能有**多段**内容（V1 折叠成首段导致次段丢失）。
            // 这里直接把 body 递归降级，把所有 sub blocks 全部 push 到 doc。
            // 对于 algorithm 环境，也走这条路但 lower_environment 会返回带编号的图块。
    let multi_block_envs = [
        "flushleft",
        "flushright",
        "center",
        "quote",
        "quotation",
        "verbatim",
    ];
    if multi_block_envs.contains(&name) {
        let p = crate::parser::parse(body);
        let sub = lower_with_macros_and_numbering(&p, None, macros, numbering);
        for b in sub.blocks {
            match b {
                Block::RawFallback { .. } => continue,
                Block::Equation { .. } => continue,
                other => doc.push(other),
            }
        }
    } else if name == "rjabstract" {
        // "摘 要" 标签用纯文本（不加 LaTeX 命令；normalizer 会处理其他残留）
        doc.push(Block::Paragraph {
            runs: vec![TextRun {
                text: "摘  要".to_string(),
                style: TextStyle::Bold,
                span: default_span,
            }],
            span: default_span,
        });
        let blk = lower_environment(name, body, default_span, macros, numbering);
        doc.push(blk);
    } else if name == "rjkeywords" {
        // "关键词" 标签
        doc.push(Block::Paragraph {
            runs: vec![TextRun {
                text: "关键词".to_string(),
                style: TextStyle::Bold,
                span: default_span,
            }],
            span: default_span,
        });
        let blk = lower_environment(name, body, default_span, macros, numbering);
        doc.push(blk);
    } else {
                let blk = lower_environment(name, body, default_span, macros, numbering);
                doc.push(blk);
            }
            pos = end;
            continue;
        }

        // 段落级命令：\section、\subsection 等
        if let Some((consumed, block)) =
            try_top_level_command(&text[pos..], default_span, numbering)
        {
            flush_paragraph(
                &mut doc,
                &mut buffer,
                &mut buffer_start,
                default_span,
                macros,
            );
            doc.push(block);
            pos += consumed;
            continue;
        }

        // \bibliography{refs} → 插入 "References" 标题段落，引导 bib 内容进入 docx。
        // （BibTeX 记录体由 \putbib（未处理）外接 references.bib 提供；V1 语义上
        // 只保证 "References" 标签出现在文档流中。）
        if text[pos..].starts_with("\\bibliography{") {
            flush_paragraph(&mut doc, &mut buffer, &mut buffer_start, default_span, macros);
            doc.push(Block::Paragraph {
                runs: vec![TextRun {
                    text: "References".to_string(),
                    style: TextStyle::Plain,
                    span: default_span,
                }],
                span: default_span,
            });
            // 跳过 \bibliography{...} 到行末
            let line_end = text[pos..].find('\n').map(|n| n + 1).unwrap_or(len - pos);
            pos += line_end;
            continue;
        }

        // \rjkeywords{keywords} → 显式输出"关键词"标签段 + 关键词内容段。
        // 模板：\newcommand{\rjkeywords}[1]{\par\noindent\xiaowuhao {\hei 关键词:}\hspace{1em}{\kai#1}\par\vspace{0.4em}}
        if text[pos..].starts_with("\\rjkeywords{") {
            flush_paragraph(&mut doc, &mut buffer, &mut buffer_start, default_span, macros);
            // 找匹配 `}`
            if let Some(end) = find_matching_brace(text, pos + "\\rjkeywords".len()) {
                let body = &text[pos + "\\rjkeywords".len() + 1..pos + "\\rjkeywords".len() + 1 + end];
            // 标签段
            doc.push(Block::Paragraph {
                runs: vec![TextRun {
                    text: "关键词".to_string(),
                    style: TextStyle::Bold,
                    span: default_span,
                }],
                span: default_span,
            });
                // 关键词内容段
                let stripped = strip_inline(body, &mut cite_numbers).trim().to_string();
                if !stripped.is_empty() {
                    doc.push(Block::Paragraph {
                        runs: vec![TextRun {
                            text: stripped,
                            style: TextStyle::Plain,
                            span: default_span,
                        }],
                        span: default_span,
                    });
                }
                pos = pos + "\\rjkeywords".len() + 1 + end + 1;
                continue;
            }
        }

        // paper3 模板：\noindent{\xiaowuhao\hei 附中文参考文献:} 这样的 section label
        // 不在环境里，是裸 \noindent 段；这里按内容匹配生成对应的 label block。
        // 检查是否进入"附中文参考文献"或"作者简介"段
        if let Some(label_text) = detect_section_label(&text[pos..]) {
            flush_paragraph(&mut doc, &mut buffer, &mut buffer_start, default_span, macros);
            doc.push(Block::Paragraph {
                runs: vec![TextRun {
                    text: label_text.to_string(),
                    style: TextStyle::Bold,
                    span: default_span,
                }],
                span: default_span,
            });
            // 跳到行末
            let line_end = text[pos..].find('\n').map(|n| n + 1).unwrap_or(len - pos);
            pos += line_end;
            continue;
        }

        // 顶层「元数据 / 装饰」命令：吞掉，不进段落流。
        // 见 [`try_top_level_metadata_command`] 注释。
        if let Some((consumed, _)) = try_top_level_metadata_command(&text[pos..]) {
            flush_paragraph(
                &mut doc,
                &mut buffer,
                &mut buffer_start,
                default_span,
                macros,
            );
            pos += consumed;
            continue;
        }

        // 取一行
        let nl = text[pos..].find('\n').map(|n| pos + n + 1).unwrap_or(len);
        let line = &text[pos..nl];
        let stripped = strip_inline(line, &mut cite_numbers);
        let trimmed = stripped.trim();

        if trimmed.is_empty() {
            flush_paragraph(
                &mut doc,
                &mut buffer,
                &mut buffer_start,
                default_span,
                macros,
            );
        } else {
            if buffer.is_empty() {
                buffer_start = pos as u32;
            }
            buffer.push_str(&stripped);
            buffer.push('\n');
        }
        pos = nl;
    }

    flush_paragraph(
        &mut doc,
        &mut buffer,
        &mut buffer_start,
        default_span,
        macros,
    );
    doc
}

// ─────────────────────────────────────────────────────────────────────────────
// Inline math / cite helpers
// ─────────────────────────────────────────────────────────────────────────────

/// A segment of a paragraph's text run - either plain text or an inline math expression.
enum RunPart<'a> {
    Text(&'a str),
    InlineMath(&'a str),
}

/// Split paragraph text into plain text and inline math segments.
/// Detects `$...$` delimiters (NOT `$$...$$` which is block math handled separately).
fn split_inline_math(text: &str) -> Vec<RunPart<'_>> {
    let mut parts = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i] == b'$' {
            // Check it's $...$ (not $$ which is block)
            if i + 1 < len && bytes[i + 1] == b'$' {
                // Double $$ = block math delimiter - treat as literal text
                let mut j = i + 1;
                while j < len && bytes[j] == b'$' {
                    j += 1;
                }
                parts.push(RunPart::Text(&text[i..j]));
                i = j;
                continue;
            }
            // Single $ - find closing $
            let mut j = i + 1;
            while j < len && bytes[j] != b'$' {
                j += 1;
            }
            let math = &text[i + 1..j];
            if !math.is_empty() {
                parts.push(RunPart::InlineMath(math));
            }
            i = j + 1;
        } else {
            let mut j = i + 1;
            while j < len && bytes[j] != b'$' {
                j += 1;
            }
            if j > i {
                parts.push(RunPart::Text(&text[i..j]));
            }
            i = j;
        }
    }
    parts
}

// ─────────────────────────────────────────────────────────────────────────────

fn flush_paragraph(
    doc: &mut Document,
    buffer: &mut String,
    start: &mut u32,
    span: Span,
    _macros: &mut MacroMap,
) {
    if buffer.trim().is_empty() {
        buffer.clear();
        return;
    }
    let body = buffer.trim().to_string();
    let s = *start;

    // V2 接入：把 LaTeX 段落走过 `latex_to_text` normalizer，
    // 输出多 run（plain / italic / bold / sup / sub）。
    // 注意：inline math ($...$) 不再单独提取为 Block::Equation，
    // 直接作为 TextStyle::MathInline 留在段落中（适合中文学术文档）。
    let cite_map: HashMap<String, usize> = HashMap::new();
    let label_map: HashMap<String, String> = HashMap::new();
    let normalized = crate::normalize::latex_to_text(&body, &cite_map, &label_map);
    let runs: Vec<TextRun> = normalized
        .runs
        .into_iter()
        .map(|r| TextRun {
            text: r.text,
            style: r.style,
            span: Span::new(s, s + buffer.len() as u32, span.source),
        })
        .collect();
    if !runs.is_empty() {
        doc.push(Block::Paragraph {
            runs,
            span: Span::new(s, s + buffer.len() as u32, span.source),
        });
    }
    buffer.clear();
}

/// 跳过 ASCII 空白 / 注释。
fn skip_whitespace_and_comment(text: &str, mut pos: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut moved = false;
    loop {
        if pos >= len {
            break;
        }
        let b = bytes[pos];
        if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
            pos += 1;
            moved = true;
            continue;
        }
        if b == b'%' {
            while pos < len && bytes[pos] != b'\n' {
                pos += 1;
            }
            moved = true;
            continue;
        }
        break;
    }
    if moved {
        Some(pos)
    } else {
        None
    }
}

/// 跳过 `\begin{document}` 之前的 preamble（documentclass / usepackage / geometry / ...）。
///
/// LaTeX preamble 全部是「导言设置」，与 Word 文档正文无关，混入段落流会
/// 污染 docx 输出（`{ctexart}`、`\PassOptionsToClass{...}{ctexart}` 等）。本函数
/// 找到 `\begin{document}` 的位置，截取其后的内容；找不到则返回原文（视为退化）。
fn strip_preamble(text: &str) -> &str {
    let needle = "\\begin{document}";
    match text.find(needle) {
        Some(idx) => {
            let after = idx + needle.len();
            let bytes = text.as_bytes();
            let mut p = after;
            while p < bytes.len()
                && (bytes[p] == b' ' || bytes[p] == b'\t' || bytes[p] == b'\n' || bytes[p] == b'\r')
            {
                p += 1;
            }
            &text[p..]
        }
        None => text,
    }
}

/// 顶层「元数据 / 装饰」命令：直接吞掉，不产出块。
///
/// rjthesis / ctexart 模板里 `\rjtitle{...}`、`\rjauthor{...}`、`\rjinfor{...}`、
/// `\fancyhead[...]{...}`、`\hypersetup{...}`、`\bibliographystyle{...}` 等都属此类：
/// 它们设置页眉 / 元数据 / 引用样式，对 Word 文档正文无视觉贡献，留着只会污染段落流。
/// 返回值 `(consumed, true)` 表示成功剥离 `consumed` 字节。
fn try_top_level_metadata_command(s: &str) -> Option<(usize, bool)> {
    const META_CMDS: &[&str] = &[
        "rjtitle",
        "rjauthor",
        "rjinfor",
        "rjhead",
        "rjcategory",
        "rjmaketitle",
        "fancyhead",
        "fancyfoot",
        "fancyhf",
        "bibliographystyle",
        "hypersetup",
        "graphicspath",
        "newCJKfontfamily",
        "providecommand",
        "newcommand",
        "renewcommand",
        "setlength",
        "geometry",
        "PassOptionsToClass",
        "documentclass",
        "usepackage",
        "newif",
        "newcounter",
        "newlength",
        "newenvironment",
        "newtheorem",
        "newlabel",
        "pagestyle",
        "thispagestyle",
        "linespread",
        "fontsize",
        "selectfont",
        "CJKfamily",
        "songti",
        "kaishu",
        "fangsong",
        "heiti",
        "lishu",
        "kai",
        "hei",
        "song",
        "wuhao",
        "xiaowuhao",
        "xiaosihao",
        "sihao",
    ];
    for cmd in META_CMDS {
        if let Some(rest) = s.strip_prefix(&format!("\\{cmd}")) {
            // 跳过可选空白
            let bytes = rest.as_bytes();
            let mut k = 0;
            while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\t') {
                k += 1;
            }
            // 必须以 `[` 或 `{` 起始（rj 类 / fancyhead 类 / hypersetup 类等带可选 `[...]`）
            if k >= bytes.len() || (bytes[k] != b'{' && bytes[k] != b'[') {
                continue;
            }
            // 吃掉所有可选 `[..]` 与 `{..}` 配对，直到行尾或遇到非命令字符
            let mut p = k;
            while p < bytes.len() {
                if bytes[p] == b'[' {
                    if let Some(close) = rest[p..].find(']') {
                        p += close + 1;
                    } else {
                        break;
                    }
                } else if bytes[p] == b'{' {
                    if let Some(off) = find_matching_brace(rest, p) {
                        p = p + 1 + off + 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
                // 跳过组间空白
                while p < bytes.len() && (bytes[p] == b' ' || bytes[p] == b'\t') {
                    p += 1;
                }
            }
            // 把「到行尾」或「下一个未配对字符」也吃掉，避免下一行被串联
            let line_end = rest[p..].find('\n').map(|n| p + n).unwrap_or(rest.len());
            let consumed = (cmd.len() + 1) + line_end;
            return Some((consumed, true));
        }
    }
    None
}

/// 寻找 `\\begin{name}`；找到则返回 `(name, body_inclusive_braces, end_pos)`。
/// 失败返回 None；遇到未闭合自动补齐。
fn scan_environment(text: &str, pos: usize) -> Option<(&str, &str, usize)> {
    let bytes = text.as_bytes();
    if pos >= bytes.len() || bytes[pos] != b'\\' {
        return None;
    }
    // 必须紧跟 "begin"
    if !text[pos..].starts_with("\\begin{") {
        return None;
    }
    let after = pos + "\\begin{".len();
    // 找 name 末尾 - first } after \begin{
    let name_end = text[after..].find('}')? + after;
    let name = &text[after..name_end];
    // 找配对 \end{name}
    let body_start = name_end + 1;
    let bytes = text.as_bytes();

    // Skip optional argument braces/Brackets like {ccc} in \begin{tabular}{ccc}
    // and [font=...] in \begin{description}[...].
    // These are not body content.
    let mut actual_body_start = body_start;
    while actual_body_start < bytes.len() {
        if bytes[actual_body_start] == b'{' {
            if let Some(offset) = find_matching_brace(text, actual_body_start) {
                actual_body_start = actual_body_start + 1 + offset + 1;
            } else {
                break;
            }
        } else if bytes[actual_body_start] == b'[' {
            // Skip optional [...] argument (supports nested {…})
            let mut i = actual_body_start + 1;
            let mut depth = 1;
            let mut found = false;
            while i < bytes.len() {
                match bytes[i] {
                    b'[' => { depth += 1; i += 1; }
                    b']' => {
                        depth -= 1;
                        if depth == 0 {
                            actual_body_start = i + 1;
                            found = true;
                            break;
                        }
                        i += 1;
                    }
                    b'{' => {
                        if let Some(off) = find_matching_brace(text, i) {
                            i = i + 1 + off + 1;
                        } else {
                            break;
                        }
                    }
                    _ => { i += 1; }
                }
            }
            if !found {
                break;
            }
        } else {
            break;
        }
    }

    let end_pat = format!("\\end{{{name}}}");
    let end_pos = text[actual_body_start..]
        .find(&end_pat)
        .map(|p| actual_body_start + p)
        .unwrap_or(text.len());
    let after_end = (end_pos + end_pat.len()).min(text.len());
    let body = &text[actual_body_start..end_pos];
    Some((name, body, after_end))
}

/// 检测当前行是否是 paper3 模板里的"附中文参考文献"或"作者简介"section 标签。
///
/// 形式为 `\noindent{\xiaowuhao\hei 附中文参考文献:}` 或带 `\textbf`。
/// 返回 Some("附中文参考文献") 或 Some("作者简介") 或 None。
fn detect_section_label(s: &str) -> Option<&'static str> {
    const LABELS: &[&str] = &[
        "附中文参考文献",
        "作者简介",
    ];
    let line_end = s.find('\n').unwrap_or(s.len());
    let line = &s[..line_end];
    for label in LABELS {
        if line.contains(label) {
            return Some(label);
        }
    }
    None
}

/// 环境 → 块的降级分派。
fn lower_environment(
    name: &str,
    body: &str,
    span: Span,
    macros: &mut MacroMap,
    numbering: &mut NumberingState,
) -> Block {
    match name {
        "itemize" | "itemize*" => lower_list(body, false, span, macros, numbering),
        "enumerate" | "enumerate*" => lower_list(body, true, span, macros, numbering),
        // description*: 发出 `\item` 标签作为段落，让 "附中文参考文献" / "作者简介" 等
        // 标签文字进 docx；items 内容走标准 list 降级。
        "description" | "description*" => lower_description_with_label(body, span, macros, numbering),
        // JOS 论文参考文献用 `\begin{list}{}{... \item[{[N]}] ... }`，
        // 视为无序 List，items 已有 `[N] —` 前缀（lower_list 中处理）。
        "list" | "list*" => lower_list(body, false, span, macros, numbering),
        "tabular" | "tabular*" | "array" => lower_table(body, span),
        "figure" | "figure*" | "table" | "table*" | "algorithm" | "algorithm*" => {
            lower_captioned_env(name, body, span, macros, numbering)
        }
        "equation" | "equation*" | "align" | "align*" | "gather" | "gather*" => Block::Equation {
            latex: body.trim().to_string(),
            is_block: true,
            span,
        },
        "document" => {
            // 直接递归降级 body
            let mut sub = Document::new();
            let p = crate::parser::parse(body);
            let doc2 = lower_with_macros_and_numbering(&p, None, macros, numbering);
            for b in doc2.blocks {
                sub.push(b);
            }
            // 折叠：返回第一个块；其它块忽略（M3 简化）
            sub.blocks.into_iter().next().unwrap_or(Block::RawFallback {
                text: body.to_string(),
                span,
            })
        }
        // 段落容器类环境：递归降级为段落序列，折叠为第一个非空块。
        // （rjthesis / ctexart 模板里大量使用这类「无视觉变化」的语义容器。）
        // 注意：flushleft/flushright/center/quote/quotation/verbatim 已由主循环直接处理。
        "minipage" | "rjkeywords" | "rjcategory" | "rjhead" | "rjtitle" | "rjauthor"
        | "rjinfor" | "rjmaketitle" => lower_paragraph_container(body, span, macros, numbering),
        "rjabstract" => lower_abstract_paragraph(body, span, macros, numbering),
        _ => Block::RawFallback {
            text: format!("\\begin{{{name}}}…\\end{{{name}}}"),
            span,
        },
    }
}

/// 把段落容器环境（`flushleft` / `quote` / `rjabstract` / ...）的 body
/// 递归降级为块序列，折叠成第一个「非空」块；若全部为空则返回 RawFallback。
///
/// 行为契约：
/// - 多个段落的环境（典型如 `flushleft` 包多行）会被压扁成首个块；
///   这一点与现有 `document` 折叠策略一致，避免新增 Block::Container 变体。
/// - 内容非空时输出 Paragraph（带清洗后的 run），空时输出 RawFallback（占位）。
fn lower_paragraph_container(
    body: &str,
    span: Span,
    macros: &mut MacroMap,
    numbering: &mut NumberingState,
) -> Block {
    let p = crate::parser::parse(body);
    let sub = lower_with_macros_and_numbering(&p, None, macros, numbering);
    for b in sub.blocks {
        match b {
            // V1：与 `rjabstract` 一致，段落容器里若首块是 inline math 抽出的
            // Equation，会把公式当成容器内容；要找到第一个真正「内容」块。
            Block::RawFallback { .. } => continue,
            Block::Equation { .. } => continue,
            other => return other,
        }
    }
    Block::RawFallback {
        text: body.to_string(),
        span,
    }
}

/// rjabstract 处理：把 body 降级为块序列（标签 + 正文 + 后续内容）。
///
/// rjthesis.cls 里 rjabstract 定义为：
///   \begin{flushleft}\xiaowuhao {\hei 摘\hspace{2em}要:} \kai} <body> {\end{flushleft}\xiaowuhao}
/// 因此 \begin{rjabstract} 内部已经包含 "摘 要" 标签 + 正文 + 后续 flushleft 收尾。
/// 本函数只负责把 body 降级，返回第一个非空 Paragraph（与原 lower_abstract_paragraph 一致）。
fn lower_abstract_paragraph(
    body: &str,
    span: Span,
    macros: &mut MacroMap,
    numbering: &mut NumberingState,
) -> Block {
    let p = crate::parser::parse(body);
    let sub = lower_with_macros_and_numbering(&p, None, macros, numbering);
    for b in sub.blocks {
        match b {
            Block::Paragraph { ref runs, .. } if !runs.is_empty() => return b,
            _ => continue,
        }
    }
    Block::RawFallback {
        text: body.to_string(),
        span,
    }
}

/// 在 `body` 中按 `\item` 切分，每段降级为 List item 内的 Block 列表。
fn lower_list(
    body: &str,
    is_ordered: bool,
    span: Span,
    macros: &mut MacroMap,
    numbering: &mut NumberingState,
) -> Block {
    let mut items: Vec<Vec<Block>> = Vec::new();
    let mut current: Option<&str> = None;
    for line in body.split_inclusive('\n') {
        let s = line.trim_end_matches(&['\r', '\n'][..]);
        if s.trim_start().starts_with("\\item") {
            if let Some(buf) = current {
                let blocks = lower_item_body(buf, span, macros, numbering);
                items.push(blocks);
            }
            // \item[label]? 之后的内容
            let after = s.trim_start().trim_start_matches("\\item");
            let after = after.trim_start();
            if let Some(rest) = after.strip_prefix('[') {
                if let Some(close) = rest.find(']') {
                    let label = &rest[..close];
                    let rest2 = rest[close + 1..].trim();
                    let mut owned = format!("{label} — ");
                    owned.push_str(rest2);
                    current = Some(Box::leak(owned.into_boxed_str()));
                    continue;
                }
            }
            // 兜底：`\item {label}` 格式（如作者简介 `\item {\hei 石洪雷}`）。
            // 剥掉外层 `{}` 和 LaTeX 格式化命令，取纯文字作为 item 内容。
            let item_content = strip_item_braces_and_formatting(after.trim());
            current = Some(Box::leak(item_content.into_boxed_str()));
        } else if current.is_some() {
            let buf = current.unwrap();
            let mut owned = String::from(buf);
            owned.push('\n');
            owned.push_str(s);
            current = Some(Box::leak(owned.into_boxed_str()));
        }
    }
    if let Some(buf) = current {
        items.push(lower_item_body(buf, span, macros, numbering));
    }
    Block::List {
        is_ordered,
        items,
        span,
    }
}

/// 解析 `\item[{label}]` 中的 `[...]` 包裹内容，返回 `(label, rest)`。
/// rest = 第一个未匹配的 `]` 之后的内容。
///
/// 策略：维护 `[`/`{` 嵌套深度，遇 `]` 使深度递减。
/// 深度回到 ≤0 时即找到最外层闭合。
///
/// 支持：
/// - `{[5]}` → label=`"[5]"`, rest=`" 冯..."`
/// - `[{[5]}]` → label=`"[5]"`, rest=`" 冯..."` (malformed: outer `{}` stripped)
fn extract_bracketed_label(s: &str) -> Option<(&str, &str)> {
    if !s.starts_with('[') {
        return None;
    }
    let rest_inside = &s[1..]; // skip the opening '['
    let mut depth = 0; // net: +1 for the outer '[', each '{' +1, each '}' -1, each '[' +1, each ']' -1

    for (i, ch) in rest_inside.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => depth -= 1,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth <= 0 {
                    // Found the closing ']' of the outer '['
                    let label_raw = &rest_inside[..i];
                    // Strip outer braces: malformed `\item[{...}]` wraps the label in extra `{}`.
                    // After stripping outer `{` (at position 0) and the matching `}` (at `fc-1`),
                    // the label is label_raw[1..fc-1].
                    let label_clean = if label_raw.starts_with('{') {
                        // Find the position of the first balanced closing `}`.
                        let mut d = 0;
                        let mut first_close_byte = None;
                        for (j, c) in label_raw.char_indices() {
                            match c {
                                '{' => d += 1,
                                '}' => {
                                    d -= 1;
                                    if d == 0 {
                                        first_close_byte = Some(j);
                                    }
                                }
                                _ => {}
                            }
                        }
                        if let Some(close_pos) = first_close_byte {
                            // label_raw[1..close_pos] strips outer { and }
                            if close_pos >= 1 {
                                &label_raw[1..close_pos]
                            } else {
                                ""
                            }
                        } else {
                            label_raw
                        }
                    } else {
                        label_raw
                    };
                    // Skip trailing `}` from malformed `\item[{[5]}]`
                    let rest_raw = rest_inside[i + 1..].trim_start();
                    let rest = if rest_raw.starts_with('}') {
                        &rest_raw[1..]
                    } else {
                        rest_raw
                    };
                    return Some((label_clean.trim(), rest.trim_start()));
                }
            }
            _ => {}
        }
    }
    None
}

/// description 环境特殊处理：
/// 把 `[label]` 格式的 `\item` 标签抽出来作为独立 Paragraph，让"附中文参考文献"等
/// 标签文本进入 docx 流；items 内容作为无序 List。
/// paper3 的附中文参考文献格式：`\begin{description}[font=...]` 后跟 `\item[{[5]}] 冯志勇…`
///
/// 根因：`scan_environment` 已升级为跳过 `[...]` 可选参数，
/// 但 description 环境的 `[font=...]` 参数会被误解析为 item label。
/// 本函数在处理 body 前，先跳过 leading optional `[...]` 参数。
fn lower_description_with_label(
    body: &str,
    span: Span,
    macros: &mut MacroMap,
    numbering: &mut NumberingState,
) -> Block {
    // Strip leading optional `[...]` argument (e.g., `[font=\normalfont,labelwidth=...]`).
    let body = strip_leading_optional_arg(body);
    let body = body.trim_start();

    // 把 body 按 `\item` 分段。第一个 `\item` 之前的行是 section label（如 `附中文参考文献:`）。
    // `split_inclusive('\n')` 产生的第一段可能以 `\item` 开头（说明没有 label），
    // 也可能以其他内容开头（说明第一段是 label）。
    let lines: Vec<&str> = body.split('\n').collect();
    let first_line = lines.first().unwrap_or(&body);

    // section_label：第一个非空、非 `\item` 行提取为 label block
    let mut section_label: Option<String> = None;
    let item_start: usize;
    let first_trimmed = first_line.trim();
    if first_trimmed.is_empty() {
        // 首行是空行（如 description 环境以 `\n` 开头）：无 label，直接从 items 开始
        item_start = 0;
    } else if first_trimmed.starts_with("\\item") {
        item_start = 0;
    } else {
        // 第一个非空行是 section label（如 `{\xiaowuhao\hei 附中文参考文献:}`）
        let label_text = extract_item_label_text(first_trimmed);
        let label_clean = label_text.trim().trim_end_matches('}').trim_end_matches(':');
        if !label_clean.is_empty() {
            section_label = Some(label_clean.to_string());
        }
        item_start = 1;
    }

    let mut label_blocks: Vec<Block> = Vec::new();
    let mut list_items: Vec<Vec<Block>> = Vec::new();
    let mut current_body: Option<&str> = None;

    for line in lines.iter().skip(item_start) {
        let s = line.trim_end_matches(&['\r'][..]);
        if s.trim_start().starts_with("\\item") {
            // 把前一个 \item 的 body 做成 list item
            if let Some(buf) = current_body.take() {
                list_items.push(lower_item_body(buf, span, macros, numbering));
            }

            let after = s.trim_start().trim_start_matches("\\item").trim_start();
            // description 环境格式：`\item[{[5]}] 冯志勇…`
            if after.starts_with('[') {
                if let Some((label_text, rest)) = extract_bracketed_label(after) {
                    if !label_text.is_empty() {
                        let label_clean = strip_label_formatting(&label_text);
                        label_blocks.push(Block::Paragraph {
                            runs: vec![TextRun {
                                text: label_clean,
                                style: TextStyle::Plain,
                                span,
                            }],
                            span,
                        });
                    }
                    current_body = Some(rest);
                    continue;
                }
            }
            // `\item {\hei name}，title` 格式（作者简介）
            if after.starts_with('{') {
                let label_text = extract_item_label_text(after.trim());
                let label_clean = label_text.trim().trim_end_matches('}').trim_end_matches(':');
                if !label_clean.is_empty() {
                    label_blocks.push(Block::Paragraph {
                        runs: vec![TextRun {
                            text: label_clean.to_string(),
                            style: TextStyle::Plain,
                            span,
                        }],
                        span,
                    });
                }
            }
            current_body = Some(after);
        } else if current_body.is_some() {
            // 多行 item 内容
            let buf = current_body.take().unwrap();
            let mut owned = String::from(buf);
            owned.push('\n');
            owned.push_str(s);
            current_body = Some(Box::leak(owned.into_boxed_str()));
        }
    }
    if let Some(buf) = current_body.take() {
        list_items.push(lower_item_body(buf, span, macros, numbering));
    }

    // 先 push section label（如有）
    if let Some(label) = section_label {
        label_blocks.insert(0, Block::Paragraph {
            runs: vec![TextRun {
                text: label,
                style: TextStyle::Plain,
                span,
            }],
            span,
        });
    }

    // 如果有 list items，发出一个 List block
    if !list_items.is_empty() {
        label_blocks.push(Block::List {
            is_ordered: false,
            items: list_items,
            span,
        });
    }

    // 返回第一个块
    label_blocks.into_iter().next().unwrap_or(Block::RawFallback {
        text: body.to_string(),
        span,
    })
}

/// 从 `\item` 行的内容中提取 label 文字。
///
/// LaTeX 里 section label 常写成 `\item {\hei 附中文参考文献}` 或 `{\xiaowuhao\hei 附中文参考文献:}`。
/// 本函数剥掉 LaTeX 格式化命令前缀（如 `\hei`、`\xiaowuhao`），提取内部纯文字。
/// 对于 `{\hei name}，title` 格式，只取 `name` 部分。
///
/// 例如：`\xiaowuhao\hei 附中文参考文献:}` → `"附中文参考文献"`
/// `{\hei 石洪雷}，博士…` → `"石洪雷"`
/// `{\textbf{作者简介}}` → `"作者简介"`
///
/// 返回提取的 label 文本。
fn extract_item_label_text(s: &str) -> String {
    let bytes = s.as_bytes();
    // 如果被外层 `{}` 包裹，先剥掉外壳
    let s = if bytes.first() == Some(&b'{') {
        if let Some(len) = find_matching_brace(s, 0) {
            &s[1..1 + len]
        } else {
            s
        }
    } else {
        s
    };
    let bytes = s.as_bytes();
    // 收集所有非命令文字
    let mut result = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            // 命令：识别命令名 + 跳到 `{...}` 内部（若有）
            let j = i + 1;
            let mut k = j;
            while k < bytes.len() && (bytes[k].is_ascii_alphabetic() || bytes[k] == b'@') {
                k += 1;
            }
            // 跳过命令后空白
            let mut m = k;
            while m < bytes.len() && (bytes[m] == b' ' || bytes[m] == b'\t') {
                m += 1;
            }
            // 命令后是 `{` → 剥掉命令，递归取内部
            if m < bytes.len() && bytes[m] == b'{' {
                if let Some(inner_len) = find_matching_brace(s, m) {
                    let inner = &s[m + 1..m + 1 + inner_len];
                    let inner_result = extract_item_label_text(inner);
                    if !inner_result.is_empty() {
                        result.push_str(&inner_result);
                    }
                    i = m + 1 + inner_len + 1;
                    continue;
                }
            }
            // 命令后无 `{` 或找不到闭合 → 跳过命令，继续
            i = k;
        } else {
            // 非命令字符：原样保留
            if let Some(ch) = s[i..].chars().next() {
                result.push(ch);
                i += ch.len_utf8();
            } else {
                i += 1;
            }
        }
    }
    result
}

/// 剥掉 item 内容最外层的 `{}` 并递归剥掉 LaTeX 格式化命令，还原纯文字。
///
/// 用于处理 `list` 环境中的 `\item {\hei 姓名}` 或 `description` 环境中的裸 item。
/// 例如：`{\hei 石洪雷}` → `"石洪雷"`，`{\textbf{作者简介}}` → `"作者简介"`。
fn strip_item_braces_and_formatting(s: &str) -> String {
    let bytes = s.as_bytes();
    // 如果被 `{}` 包裹，剥掉外壳
    if bytes.first() == Some(&b'{') {
        if let Some(len) = find_matching_brace(s, 0) {
            let inner = &s[1..1 + len];
            return strip_item_braces_and_formatting(inner);
        }
    }
    strip_label_formatting(s)
}

/// 剥掉 label 文本中的 LaTeX 格式化命令前缀，保留纯文字。
///
/// LaTeX 里 item label 常写成 `\item {\hei 附中文参考文献}` 或 `\item {\textbf{作者简介}}`。
/// 本函数把 `\hei{text}` → `text`、`\textbf{text}` → `text`，
/// 处理嵌套的命令如 `\hei {\textbf{text}}`（剥两层）。
fn strip_label_formatting(raw: &str) -> String {
    let bytes = raw.as_bytes();
    // 如果整个字符串被外层 `{}` 包裹，先剥掉外壳（如 `"{[5]}"` → `"[5]"`）
    if bytes.first() == Some(&b'{') {
        if let Some(len) = find_matching_brace(raw, 0) {
            let inner = &raw[1..1 + len];
            return strip_label_formatting(inner);
        }
    }
    let mut out = String::with_capacity(raw.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            let j = i + 1;
            let mut k = j;
            while k < bytes.len() && (bytes[k].is_ascii_alphabetic() || bytes[k] == b'@') {
                k += 1;
            }
            // 命令名
            let cmd = std::str::from_utf8(&bytes[j..k]).unwrap_or("");
            // 跳过命令后空白
            let mut m = k;
            while m < bytes.len() && (bytes[m] == b' ' || bytes[m] == b'\t') {
                m += 1;
            }
            // 命令后是 `{` → 剥掉 `\{cmd}{`，取内部内容
            if m < bytes.len() && bytes[m] == b'{' {
                if let Some(inner_len) = find_matching_brace(raw, m) {
                    let inner = &raw[m + 1..m + 1 + inner_len];
                    // 递归处理内部（处理嵌套的格式化命令）
                    out.push_str(&strip_label_formatting(inner));
                    i = m + 1 + inner_len + 1;
                    continue;
                }
            }
            // 不是格式化命令格式：原样写 `\cmd`
            out.push('\\');
            out.push_str(cmd);
            i = k;
        } else {
            if let Some(ch) = raw[i..].chars().next() {
                out.push(ch);
                i += ch.len_utf8();
            } else {
                i += 1;
            }
        }
    }
    out
}

/// 跳过 body 开头的 optional `[...]` 参数（如 `[font=\normalfont,labelwidth=...]`）。
/// 返回跳过后的子串。
fn strip_leading_optional_arg(body: &str) -> &str {
    let trimmed = body.trim_start();
    if !trimmed.starts_with('[') {
        return body;
    }
    let bytes = trimmed.as_bytes();
    let mut i = 1; // skip '['
    let mut depth = 1;
    while i < bytes.len() {
        match bytes[i] {
            b'[' => { depth += 1; i += 1; }
            b']' => {
                depth -= 1;
                if depth == 0 {
                    let after_close = &trimmed[i + 1..];
                    let after_trimmed = after_close.trim_start();
                    if after_trimmed.starts_with("\\item") {
                        // This [..] is the item label, don't strip
                        return body;
                    }
                    return after_close;
                }
                i += 1;
            }
            b'{' => {
                let mut d = 1;
                let mut j = i + 1;
                while j < bytes.len() && d > 0 {
                    match bytes[j] {
                        b'{' => { d += 1; j += 1; }
                        b'}' => { d -= 1; j += 1; }
                        _ => { j += 1; }
                    }
                }
                i = j;
            }
            _ => { i += 1; }
        }
    }
    body
}

fn lower_item_body(
    buf: &str,
    span: Span,
    macros: &mut MacroMap,
    numbering: &mut NumberingState,
) -> Vec<Block> {
    let stripped = strip_inline(buf, &mut HashMap::new());
    if stripped.trim().is_empty() {
        return Vec::new();
    }
    let p = crate::parser::parse(buf);
    let sub = lower_with_macros_and_numbering(&p, None, macros, numbering);
    let mut out = sub.blocks;
    if out.is_empty() {
        out.push(Block::Paragraph {
            runs: vec![TextRun {
                text: stripped.trim().to_string(),
                style: TextStyle::Plain,
                span,
            }],
            span,
        });
    }
    out
}

/// tabular/array 降级（支持嵌套表格递归）。
///
/// 形如：`{c|c|c}` 列规范 + 主体 `\hline / & / \\\hline / \multicolumn{n}{...}{...}`。
/// 支持单元格内嵌套 `\begin{tabular}...\end{tabular}`（递归降级为文本占位）。
fn lower_table(body: &str, span: Span) -> Block {
    // 主体可能被 `\\` 分行
    let rows_text: Vec<&str> = body.split("\\\\").collect();
    let mut rows: Vec<TableRow> = Vec::new();
    for row in rows_text {
        // Check for \rowcolor at start of row
        let mut current_row_color: Option<String> = None;
        let mut row_text = row;
        if let Some(stripped) = row_text.strip_prefix("\\rowcolor") {
            let rest = stripped.trim_start();
            // Handle \rowcolor[model]{color} or \rowcolor{color}
            let color_text = if rest.starts_with('[') {
                // \rowcolor[model]{color} format
                if let Some(close_bracket) = rest.find(']') {
                    let after_bracket = &rest[close_bracket + 1..];
                    if after_bracket.starts_with('{') {
                        if let Some(close_brace) = after_bracket.find('}') {
                            current_row_color = Some(after_bracket[1..close_brace].to_string());
                            &after_bracket[close_brace + 1..]
                        } else {
                            rest
                        }
                    } else {
                        rest
                    }
                } else {
                    rest
                }
            } else if rest.starts_with('{') {
                if let Some(end) = rest.find('}') {
                    current_row_color = Some(rest[1..end].to_string());
                    &rest[end + 1..]
                } else {
                    rest
                }
            } else {
                rest
            };
            row_text = color_text.trim_start();
        }

        let cells_text: Vec<&str> = row_text.split('&').collect();
        if cells_text.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        let mut cells: Vec<TableCell> = Vec::new();
        for c in cells_text {
            let mut cell_bg_color = current_row_color.clone();

            // Check for \rowcolor{color} or \rowcolor[model]{color} in this cell (before strip_inline removes it)
            let raw_for_colorcheck = c.trim();
            if let Some(stripped) = raw_for_colorcheck.strip_prefix("\\rowcolor") {
                let rest = stripped.trim_start();
                let color_text = if rest.starts_with('[') {
                    // \rowcolor[model]{color} format
                    if let Some(close_bracket) = rest.find(']') {
                        let after_bracket = &rest[close_bracket + 1..];
                        if after_bracket.starts_with('{') {
                            after_bracket
                                .find('}')
                                .map(|close_brace| after_bracket[1..close_brace].to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else if rest.starts_with('{') {
                    rest.find('}')
                        .map(|close_brace| rest[1..close_brace].to_string())
                } else {
                    None
                };
                if let Some(color) = color_text {
                    cell_bg_color = Some(color);
                }
            }

            // Extract raw text for multicolumn check BEFORE strip_inline
            // First split by & to get the cell text, then check for \multicolumn
            let raw_for_multicolumn = c.trim();

            let raw = strip_inline(c, &mut HashMap::new())
                .replace("\\hline", "")
                .trim()
                .to_string();

            // Check for \multicolumn{n}{spec}{text} - must be at START of cell content
            if let Some((n, cell_text)) = parse_multicolumn(raw_for_multicolumn) {
                // V2：把 cell_text 也走 normalizer
                let cite_map: HashMap<String, usize> = HashMap::new();
                let label_map: HashMap<String, String> = HashMap::new();
                let normalized = crate::normalize::latex_to_text(&cell_text, &cite_map, &label_map);
                let runs: Vec<TextRun> = if normalized.runs.is_empty() {
                    vec![TextRun {
                        text: cell_text,
                        style: TextStyle::Plain,
                        span,
                    }]
                } else {
                    normalized
                        .runs
                        .into_iter()
                        .map(|r| TextRun {
                            text: r.text,
                            style: r.style,
                            span,
                        })
                        .collect()
                };
                cells.push(TableCell {
                    runs,
                    colspan: n as u32,
                    rowspan: 1,
                    bg_color: cell_bg_color,
                });
                continue;
            }

            if raw.is_empty() {
                cells.push(TableCell {
                    runs: vec![],
                    colspan: 1,
                    rowspan: 1,
                    bg_color: cell_bg_color,
                });
                continue;
            }
            // 嵌套表格检测
            let cell_runs = if let Some((nested_body, _)) = extract_nested_tabulary(&raw) {
                let nested_table = lower_table(nested_body, span);
                if let Block::Table { rows: nr, .. } = nested_table {
                    // 扁平化嵌套表格为首行文本
                    let first_row = nr
                        .first()
                        .map(|r| {
                            r.cells
                                .iter()
                                .map(|cell| {
                                    cell.runs
                                        .iter()
                                        .map(|run| run.text.as_str())
                                        .collect::<String>()
                                })
                                .collect::<Vec<_>>()
                                .join(" | ")
                        })
                        .unwrap_or_else(|| "[嵌套表格]".to_string());
                    vec![TextRun {
                        text: format!("[表格: {}]", first_row),
                        style: TextStyle::Code,
                        span,
                    }]
                } else {
                    // 退化路径：normalizer 处理 raw
                    let cite_map: HashMap<String, usize> = HashMap::new();
                    let label_map: HashMap<String, String> = HashMap::new();
                    let normalized = crate::normalize::latex_to_text(&raw, &cite_map, &label_map);
                    if normalized.runs.is_empty() {
                        vec![TextRun {
                            text: raw.clone(),
                            style: TextStyle::Plain,
                            span,
                        }]
                    } else {
                        normalized
                            .runs
                            .into_iter()
                            .map(|r| TextRun {
                                text: r.text,
                                style: r.style,
                                span,
                            })
                            .collect()
                    }
                }
            } else {
                // V2 接入：把 cell 文本走 latex_to_text normalizer
                // 否则 \textbf / \textit / $math$ 等会原文泄漏。
                let cite_map: HashMap<String, usize> = HashMap::new();
                let label_map: HashMap<String, String> = HashMap::new();
                let normalized = crate::normalize::latex_to_text(&raw, &cite_map, &label_map);
                let cell_runs: Vec<TextRun> = normalized
                    .runs
                    .into_iter()
                    .map(|r| TextRun {
                        text: r.text,
                        style: r.style,
                        span,
                    })
                    .collect();
                if cell_runs.is_empty() {
                    vec![TextRun {
                        text: raw.clone(),
                        style: TextStyle::Plain,
                        span,
                    }]
                } else {
                    cell_runs
                }
            };
            cells.push(TableCell {
                runs: cell_runs,
                colspan: 1,
                rowspan: 1,
                bg_color: cell_bg_color,
            });
        }
        rows.push(TableRow { cells });
    }
    if rows.is_empty() {
        // 兜底：单行单列 + 原文（走 normalizer）
        let cite_map: HashMap<String, usize> = HashMap::new();
        let label_map: HashMap<String, String> = HashMap::new();
        let normalized = crate::normalize::latex_to_text(body, &cite_map, &label_map);
        let runs: Vec<TextRun> = if normalized.runs.is_empty() {
            vec![TextRun {
                text: body.to_string(),
                style: TextStyle::Plain,
                span,
            }]
        } else {
            normalized
                .runs
                .into_iter()
                .map(|r| TextRun {
                    text: r.text,
                    style: r.style,
                    span,
                })
                .collect()
        };
        rows.push(TableRow {
            cells: vec![TableCell {
                runs,
                colspan: 1,
                rowspan: 1,
                bg_color: None,
            }],
        });
    }
    Block::Table {
        rows,
        caption: None,
        number: None,
        span,
    }
}

/// Parse \multicolumn{n}{spec}{text} → (n, cell_text)
pub fn parse_multicolumn(cell_text: &str) -> Option<(usize, String)> {
    let prefix = "\\multicolumn";
    if !cell_text.starts_with(prefix) {
        return None;
    }
    let rest = &cell_text[prefix.len()..];
    // Skip whitespace
    let rest = rest.trim_start();
    if !rest.starts_with('{') {
        return None;
    }
    let rest = &rest[1..];
    // Parse n
    let n_end = rest.find(|c: char| !c.is_ascii_digit())?;
    let n: usize = rest[..n_end].parse().ok()?;
    let rest = &rest[n_end..].trim_start();
    if !rest.starts_with("}{") {
        return None;
    }
    let rest = &rest[2..];
    // Find the closing } of the spec
    let spec_end = rest.find('}')?;
    let rest = &rest[spec_end + 1..];
    if !rest.starts_with('{') {
        return None;
    }
    let rest = &rest[1..];
    // Find the closing } of the text
    let mut depth = 1;
    let mut end = 0;
    for (i, c) in rest.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    if end == 0 {
        return None;
    }
    let cell_text = rest[..end].trim().to_string();
    Some((n, cell_text))
}

/// 从单元格文本中提取嵌套 tabular 环境内容。
/// 返回 `Some((inner_body, rest))` 其中 inner_body 是 tabular 的主体文本。
/// 查找 `[TAB: ...]` 标记（由 strip_inline 保留的嵌套表格占位符）。
fn extract_nested_tabulary(text: &str) -> Option<(&str, &str)> {
    let start = text.find("[TAB: ")?;
    let after_marker = &text[start + "[TAB: ".len()..];

    // 找列规范的 `}` - 列规范可能包含嵌套的 {}
    let mut depth = 0;
    let mut cb_pos = None;
    for (i, b) in after_marker.bytes().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    cb_pos = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let cb_pos = cb_pos?;
    let content_start = cb_pos + 1;

    // 找匹配的 `]`（结束标记）
    let end_pos = after_marker[content_start..].find(']')?;
    let inner = &after_marker[content_start..content_start + end_pos];
    let rest = &after_marker[content_start + end_pos + 1..];
    Some((inner, rest.trim()))
}

/// `\caption{...}` 在 figure/table 环境中，或 algorithm 环境。
fn lower_captioned_env(
    name: &str,
    body: &str,
    span: Span,
    _macros: &mut MacroMap,
    numbering: &mut NumberingState,
) -> Block {
    // algorithm 环境：发出 "算法 N" 标题段 + AlgLine 序列。
    if name == "algorithm" || name == "algorithm*" {
        let (caption_text, _label) = extract_caption_and_label(body);
        let num = numbering.next_algorithm();
        let (io, cap_from_io, label_from_io) = crate::algorithm::extract_algorithm_io(body);
        let cap = caption_text.or(cap_from_io).unwrap_or_default();
        let cap_normalized = normalize_caption(&cap);
        let _label_final = _label;
        let _ = label_from_io;
        let rows = crate::algorithm::parse_algorithm_rows(body);
        return Block::Algorithm {
            lines: rows,
            io,
            caption: if cap_normalized.is_empty() {
                None
            } else {
                Some(cap_normalized)
            },
            number: Some(num),
            span,
        };
    }

    let (img, caption) = extract_includegraphics_and_caption(body);
    let caption_normalized = caption.as_deref().map(normalize_caption);
    if name.starts_with("figure") {
        Block::Figure {
            path: img.unwrap_or_default(),
            caption: caption_normalized,
            scale: 1.0,
            number: Some(numbering.next_figure()),
            span,
        }
    } else {
        let mut table = lower_table(body, span);
        if let Block::Table {
            caption: c,
            number: n,
            ..
        } = &mut table
        {
            *c = caption_normalized;
            *n = Some(numbering.next_table());
        }
        table
    }
}

/// 把 caption 文本（可能是 raw LaTeX）走一遍 `latex_to_text`，
/// 输出 join_plain 字符串（保留 \\textbf 已经被处理过的内容）。
fn normalize_caption(text: &str) -> String {
    let (cite, label) = (HashMap::new(), HashMap::new());
    let n = crate::normalize::latex_to_text(text, &cite, &label);
    let mut out = String::new();
    for r in n.runs {
        out.push_str(&r.text);
    }
    out.trim().to_string()
}

/// 从 algorithm body 中抽 caption 和 label。
fn extract_caption_and_label(body: &str) -> (Option<String>, String) {
    let mut caption: Option<String> = None;
    let mut label: String = String::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("\\caption") {
            if let Some(text) = extract_brace_arg(trimmed, "caption") {
                caption = Some(text);
            }
        }
        if trimmed.starts_with("\\label") {
            if let Some(lbl) = extract_brace_arg(trimmed, "label") {
                label = lbl;
            }
        }
    }
    (caption, label)
}

/// 抽 \cmd{...} 的 {…} 参数。
fn extract_brace_arg(line: &str, cmd: &str) -> Option<String> {
    let rest = line.strip_prefix(&format!("\\{cmd}"))?;
    let rest = rest.trim_start();
    if !rest.starts_with('{') {
        return None;
    }
    let mut depth = 0;
    for (i, ch) in rest.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(rest[1..i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn extract_includegraphics_and_caption(body: &str) -> (Option<String>, Option<String>) {
    let img: Option<String> = if let Some(args) = find_command_with_brace(body, "includegraphics") {
        // 形如 \includegraphics[width=.7\textwidth]{path}
        if let Some(close) = args.rfind('}') {
            Some(args[close + 1..].trim().to_string())
        } else {
            Some(args.to_string())
        }
    } else {
        None
    };
    let caption: Option<String> =
        find_command_with_brace(body, "caption").map(|args| args.to_string());
    (img, caption)
}

fn find_command_with_brace<'a>(body: &'a str, cmd: &str) -> Option<&'a str> {
    let pat = format!("\\{cmd}");
    let idx = body.find(&pat)?;
    let mut i = idx + pat.len();
    // 跳过可选空白
    while i < body.len() && (body.as_bytes()[i] == b' ' || body.as_bytes()[i] == b'\t') {
        i += 1;
    }
    // 跳过可选方括号参数 `[...]`（允许多个）
    while i < body.len() && body.as_bytes()[i] == b'[' {
        if let Some(close) = body[i..].find(']') {
            i += close + 1;
        } else {
            break;
        }
        while i < body.len() && (body.as_bytes()[i] == b' ' || body.as_bytes()[i] == b'\t') {
            i += 1;
        }
    }
    if i >= body.len() || body.as_bytes()[i] != b'{' {
        return None;
    }
    let start = i + 1;
    let off = find_matching_brace(body, i)?;
    Some(&body[start..start + off])
}

/// 找与 `s[pos]`（应为 `{`）配对的 `}` 偏移（**相对于 `{` 之后的位置**）。
/// 例如 `"{a.png}"`（pos=0）返回 `Some(5)`，调用方用 `&s[pos+1..pos+1+5]`。
fn find_matching_brace(s: &str, pos: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.get(pos) != Some(&b'{') {
        return None;
    }
    let mut depth = 0i32;
    for (i, &b) in bytes.iter().enumerate().skip(pos) {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i - pos - 1);
                }
            }
            _ => {}
        }
    }
    None
}

/// 去掉行内的简单控制序列（V1 简化）。
///
/// 关键：**逐字节**走 `i`，但凡要把字符写入 `out` 时，**必须**走 `chars().next()`
/// 拿到完整的 `char`（可能 1-4 字节），再用 `char.len_utf8()` 推进 `i`——
/// 绝不能 `bytes[i] as char`，那会把 UTF-8 多字节字符的字节当 Latin-1 字符再编码，
/// 形成「mojibake 二次编码」（如「微」`E5 BE AE` 被错误写成 `C3 A5 C2 BE C2 AE`）。
///
/// `cite_numbers` is used to track citation keys and assign sequential numbers.
/// It is passed from `lower_with_macros` so that `\cite{key}` in paragraphs
/// is replaced with `[n]` where n is the citation number across the whole document.
fn strip_inline(line: &str, cite_numbers: &mut HashMap<String, usize>) -> String {
    let mut out = String::with_capacity(line.len());
    let bytes = line.as_bytes();
    let mut i = 0;
    let mut in_math = false;
    while i < bytes.len() {
        let c = bytes[i];
        if in_math {
            if c == b'$' {
                in_math = false;
            }
            // 把当前字符（含多字节）原样写出去
            if let Some(ch) = line[i..].chars().next() {
                out.push(ch);
                i += ch.len_utf8();
            } else {
                i += 1;
            }
            continue;
        }
        if c == b'$' {
            in_math = true;
            out.push('$');
            i += 1;
            continue;
        }
            if c == b'\\' {
                let cmd_start = i + 1;
            let mut j = cmd_start;
            if j < bytes.len() && bytes[j] == b'\\' {
                // 探测 `\\\\` (四个反斜杠) 形式：
                //   - `\\`  + `\\`  → 两次行终止（保留第二个 `\\` 不输出）
                //   - `\\`  + `\` + alpha → 行终止 + 命令（如 `\\\\textbf`）
                // 两种情况都让下次迭代重新判断。
                let k2 = j + 1;
                if k2 < bytes.len() && bytes[k2] == b'\\' {
                    out.push('\n');
                    // i 停在第三个 `\\` 上（位置 k2），下次迭代会再次进入此分支
                    i = k2;
                    continue;
                }
                out.push('\n');
                i = j + 1;
                continue;
            }
            // 命令名：alpha / @ / 单字符转义（\% \$ \& \# \_ \{ \}）
            // 对单字符转义（不在 alpha 集中），j 必须至少推进 1 步，
            // 否则 cmd 会是空串，"cmd.len() == 1" 判断永远不成立。
            if j < bytes.len() {
                let b = bytes[j];
                let is_escape_char = matches!(b, b'%' | b'$' | b'&' | b'#' | b'_' | b'{' | b'}');
                if is_escape_char {
                    j += 1;
                } else {
                    while j < bytes.len() && (bytes[j].is_ascii_alphabetic() || bytes[j] == b'@') {
                        j += 1;
                    }
                }
            }
            let cmd = &line[cmd_start..j];
            // 通用 LaTeX 转义：\% \$ \& \# \_ \{ \}  → 保留为转义形式
            // （即 `\%` 写成两个字符 `\` + `%`），让下游 latex_to_text 的
            // strip_comments 能正确识别 `\%` 为字面 %（奇数个 `\`）。
            if cmd.len() == 1 {
                let esc = cmd.as_bytes()[0];
                let literal = match esc {
                    b'%' => Some("%"),
                    b'$' => Some("$"),
                    b'&' => Some("&"),
                    b'#' => Some("#"),
                    b'_' => Some("_"),
                    b'{' => Some("{"),
                    b'}' => Some("}"),
                    _ => None,
                };
                if let Some(s) = literal {
                    out.push('\\');
                    out.push_str(s);
                    i = j;
                    continue;
                }
            }
            let mut k = j;
            while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\t') {
                k += 1;
            }
            let has_arg = k < bytes.len() && bytes[k] == b'{';
            if cmd == "par" {
                out.push('\n');
                i = j;
                continue;
            }
            if matches!(
                cmd,
                "section"
                    | "subsection"
                    | "subsubsection"
                    | "paragraph"
                    | "textbf"
                    | "textit"
                    | "texttt"
                    | "emph"
            ) && has_arg
            {
                if let Some(off) = find_matching_brace(line, k) {
                    out.push('\\');
                    out.push_str(cmd);
                    out.push('{');
                    out.push_str(&line[k + 1..k + 1 + off]);
                    out.push('}');
                    i = k + 1 + off + 1;
                    continue;
                }
            }
            // \cite{key} → [n] citation number
            if cmd == "cite" && has_arg {
                if let Some(off) = find_matching_brace(line, k) {
                    let body_start = k + 1;
                    let keys_raw = &line[body_start..body_start + off];
                    let keys: Vec<&str> = keys_raw
                        .split(',')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect();
                    let nums: Vec<String> = keys
                        .iter()
                        .map(|k| {
                            let next = cite_numbers.len() + 1;
                            let n = *cite_numbers.entry(k.to_string()).or_insert(next);
                            n.to_string()
                        })
                        .collect();
                    out.push_str(&format!("[{}]", nums.join(",")));
                    // Skip remaining optional {...} args
                    let mut p = k + 1 + off + 1;
                    loop {
                        while p < bytes.len()
                            && (bytes[p] == b' ' || bytes[p] == b'\t' || bytes[p] == b'\n')
                        {
                            p += 1;
                        }
                        if p < bytes.len() && bytes[p] == b'{' {
                            if let Some(o2) = find_matching_brace(line, p) {
                                p = p + 1 + o2 + 1;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    i = p;
                    continue;
                }
            }
            // \ref → emit label so "算法 1" etc. appears in text
            // \footnote → skip (V1)
            // \href{url}{text} → emit text
            // \url{url} / \nolinkurl{url} → emit URL
            // \label → skip (no text content)
            if matches!(
                cmd,
                "ref" | "footnote" | "label"
            ) && has_arg
            {
                if let Some(off) = find_matching_brace(line, k) {
                    // \ref{label} → emit the label text (e.g., "alg:attention" → no visible text,
                    // but \ref{tab:foo} → emit "表 N" — V1: emit label text for discoverability)
                    if cmd == "ref" {
                        let label_start = k + 1;
                        let label_text = &line[label_start..label_start + off];
                        out.push_str(label_text);
                    }
                    // \label / \footnote: skip
                    let mut p = k + 1 + off + 1;
                    loop {
                        while p < bytes.len()
                            && (bytes[p] == b' ' || bytes[p] == b'\t' || bytes[p] == b'\n')
                        {
                            p += 1;
                        }
                        if p < bytes.len() && bytes[p] == b'{' {
                            if let Some(o2) = find_matching_brace(line, p) {
                                p = p + 1 + o2 + 1;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    i = p;
                    continue;
                }
            }
            // \href{url}{text}: emit text
            if cmd == "href" && has_arg {
                if let Some(off) = find_matching_brace(line, k) {
                    let href_body = &line[k + 1..k + 1 + off];
                    let rest = line[k + 1 + off + 1..].trim_start();
                    if rest.starts_with('{') {
                        if let Some(off2) = find_matching_brace(rest, 0) {
                            let text = &rest[1..1 + off2];
                            out.push_str(text);
                            i = k + 1 + off + 1 + 1 + off2 + 1;
                            continue;
                        }
                    }
                    out.push_str(href_body);
                    i = k + 1 + off + 1;
                    continue;
                }
            }
            // \url{url} / \nolinkurl{url}: emit URL
            if matches!(cmd, "url" | "nolinkurl") && has_arg {
                if let Some(off) = find_matching_brace(line, k) {
                    let url_start = k + 1;
                    let url = &line[url_start..url_start + off];
                    out.push_str(url);
                    i = k + 1 + off + 1;
                    continue;
                }
            }
            // tabular/array 环境：保留标记以便后续嵌套检测
            if matches!(cmd, "begin") {
                // 检查是否是 tabular 或 array 环境
                let rest = &line[i..];
                if rest.starts_with("\\begin{tabular}") || rest.starts_with("\\begin{array}") {
                    // 找环境的列规范或第一个 {之后的内容
                    let env_marker = if rest.starts_with("\\begin{tabular}") {
                        "\\begin{tabular}"
                    } else {
                        "\\begin{array}"
                    };
                    let after_marker = &rest[env_marker.len()..];
                    // 找到列规范的 } - 注意可能有嵌套的 {}
                    let mut depth = 0;
                    let mut found_close = false;
                    let mut close_pos = 0;
                    for (idx, b) in after_marker.bytes().enumerate() {
                        match b {
                            b'{' => depth += 1,
                            b'}' => {
                                depth -= 1;
                                if depth == 0 {
                                    close_pos = idx;
                                    found_close = true;
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    if found_close {
                        // 输出标记 + 列规范 + 内容（直到匹配 \end）
                        // 这里我们只输出一个标记占位，实际嵌套检测在 lower_table 中用原始单元格文本
                        out.push_str("[TAB: ");
                        i += env_marker.len() + close_pos + 1;
                        continue;
                    }
                }
                // 其他 \begin 命令走默认处理
            }
            // \rowcolor{...}：在表格中设置行颜色，保留命令以便 lower_table 提取
            if cmd == "rowcolor" {
                out.push_str("\\rowcolor");
                i = j;
                continue;
            }
            // \multicolumn{n}{spec}{text}：保留完整命令以便 lower_table 检测
            if cmd == "multicolumn" {
                // Output \multicolumn{n}{spec}{text} in full
                out.push_str("\\multicolumn");
                if has_arg {
                    // First argument: n
                    if let Some(off) = find_matching_brace(line, k) {
                        out.push('{');
                        out.push_str(&line[k + 1..k + 1 + off]);
                        out.push('}');
                        let mut p = k + 1 + off + 1;
                        // Skip whitespace
                        while p < bytes.len() && (bytes[p] == b' ' || bytes[p] == b'\t') {
                            p += 1;
                        }
                        // Second argument: spec
                        if p < bytes.len() && bytes[p] == b'{' {
                            if let Some(off2) = find_matching_brace(line, p) {
                                out.push('{');
                                out.push_str(&line[p + 1..p + 1 + off2]);
                                out.push('}');
                                p = p + 1 + off2 + 1;
                                // Skip whitespace
                                while p < bytes.len() && (bytes[p] == b' ' || bytes[p] == b'\t') {
                                    p += 1;
                                }
                                // Third argument: text
                                if p < bytes.len() && bytes[p] == b'{' {
                                    if let Some(off3) = find_matching_brace(line, p) {
                                        out.push('{');
                                        out.push_str(&line[p + 1..p + 1 + off3]);
                                        out.push('}');
                                        i = p + 1 + off3 + 1;
                                        continue;
                                    }
                                }
                            }
                        }
                        // Fallback: consume the brace we already ate
                        let _ = k + 1 + off + 1;
                    }
                }
                i = j;
                continue;
            }
            if matches!(cmd, "end") {
                let rest = &line[i..];
                if rest.starts_with("\\end{tabular}") || rest.starts_with("\\end{array}") {
                    out.push(']');
                    i += if rest.starts_with("\\end{tabular}") {
                        12
                    } else {
                        9
                    };
                    continue;
                }
            }
            // 纯装饰 inline 命令（无视觉含义）：整段吞掉，仅保留 \par 触发换行
            if matches!(
                cmd,
                "hspace"
                    | "vspace"
                    | "bigskip"
                    | "smallskip"
                    | "noindent"
                    | "indent"
                    | "quad"
                    | "qquad"
                    | "mbox"
                    | "hbox"
                    | "vbox"
                    | "textsuperscript"
                    | "textsubscript"
                    | "today"
                    | "protect"
                    | "linebreak"
                    | "pagebreak"
                    | "newpage"
                    | "newline"
                    | "hfill"
                    | "vfill"
                    | "dotfill"
            ) {
                if has_arg {
                    if let Some(off) = find_matching_brace(line, k) {
                        i = k + 1 + off + 1;
                        continue;
                    }
                }
                i = j;
                continue;
            }
            // 字体 / 字号切换命令：吞命令、保留参数文本（V1 简化，不带字体信息）
            if matches!(
                cmd,
                "hei"
                    | "song"
                    | "kai"
                    | "kaishu"
                    | "fangsong"
                    | "lishu"
                    | "you"
                    | "wuhao"
                    | "xiaowuhao"
                    | "xiaosihao"
                    | "sihao"
            ) {
                if has_arg {
                    if let Some(off) = find_matching_brace(line, k) {
                        out.push_str(&line[k + 1..k + 1 + off]);
                        i = k + 1 + off + 1;
                        continue;
                    }
                }
                i = j;
                continue;
            }
            if has_arg {
                if let Some(off) = find_matching_brace(line, k) {
                    i = k + 1 + off + 1;
                    continue;
                }
            }
            i = j;
            continue;
        }
        // fallthrough：非 ASCII 字节的字符走 char 迭代，避免 mojibake 二次编码
        if let Some(ch) = line[i..].chars().next() {
            out.push(ch);
            i += ch.len_utf8();
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn lower_heading_and_paragraph() {
        let src = "\\section{Intro}\n\nHello world.\n";
        let p = parse(src);
        let doc = lower_to_document(&p, None);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
        assert!(matches!(doc.blocks[1], Block::Paragraph { .. }));
    }

    #[test]
    fn lower_textbf_kept() {
        let src = "this is \\textbf{bold} text";
        let p = parse(src);
        let doc = lower_to_document(&p, None);
        if let Block::Paragraph { runs, .. } = &doc.blocks[0] {
            // V2：\\textbf{...} 现在会产生一个 Bold run，plain runs 在两侧。
            let combined: String = runs.iter().map(|r| r.text.as_str()).collect();
            assert!(combined.contains("bold"), "runs combined: {combined:?}");
            // 至少一个 run 是 Bold 且包含 "bold"
            let bold_with_text = runs
                .iter()
                .any(|r| r.style == TextStyle::Bold && r.text.contains("bold"));
            assert!(bold_with_text, "no Bold run with 'bold' in {:?}", runs);
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn lower_itemize() {
        let src = "\\begin{itemize}\n\\item alpha\n\\item beta\n\\end{itemize}";
        let p = parse(src);
        let doc = lower_to_document(&p, None);
        match &doc.blocks[0] {
            Block::List {
                is_ordered, items, ..
            } => {
                assert!(!is_ordered);
                assert_eq!(items.len(), 2);
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn lower_enumerate() {
        let src = "\\begin{enumerate}\n\\item one\n\\item two\n\\end{enumerate}";
        let p = parse(src);
        let doc = lower_to_document(&p, None);
        match &doc.blocks[0] {
            Block::List { is_ordered, .. } => assert!(is_ordered),
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn lower_tabular_basic() {
        let src = "\\begin{tabular}{c|c}\nA & B \\\\\nC & D \\\\\n\\end{tabular}";
        let p = parse(src);
        let doc = lower_to_document(&p, None);
        match &doc.blocks[0] {
            Block::Table { rows, .. } => {
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].cells.len(), 2);
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn lower_figure_with_caption() {
        let src =
            "\\begin{figure}\n\\includegraphics[width=.7]{a.png}\n\\caption{Demo}\n\\end{figure}";
        let p = parse(src);
        let doc = lower_to_document(&p, None);
        match &doc.blocks[0] {
            Block::Figure { path, caption, .. } => {
                assert_eq!(path, "a.png");
                assert_eq!(caption.as_deref(), Some("Demo"));
            }
            _ => panic!("expected figure"),
        }
    }

    #[test]
    fn lower_equation_block() {
        let src = "\\begin{equation}\nE = mc^2\n\\end{equation}";
        let p = parse(src);
        let doc = lower_to_document(&p, None);
        match &doc.blocks[0] {
            Block::Equation {
                latex, is_block, ..
            } => {
                assert!(is_block);
                assert!(latex.contains("mc^2"));
            }
            _ => panic!("expected equation"),
        }
    }

    #[test]
    fn lower_href_in_paragraph() {
        let src = "see \\href{https://x}{the doc}";
        let p = parse(src);
        let doc = lower_to_document(&p, None);
        if let Block::Paragraph { runs, .. } = &doc.blocks[0] {
            // href 整段被吞；段落保留 "see "
            assert!(runs[0].text.starts_with("see"));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn lower_unbalanced_recovers() {
        let src = "\\section{Unclosed\n\nbody";
        let p = parse(src);
        let _doc = lower_to_document(&p, None);
        // 不 panic
    }

    // ── M6 修复测试 ──────────────────────────────────────────────────────────

    #[test]
    fn lower_inline_math() {
        // V2: inline math $...$ stays in paragraph, no separate Block::Equation created
        let src = "Einstein said $E = mc^2$ is famous.";
        let p = parse(src);
        let mut macros = crate::expand::MacroMap::new();
        let doc = lower_with_macros(&p, None, &mut macros);
        // NO separate Block::Equation blocks for inline math
        let eq_count = doc
            .blocks
            .iter()
            .filter(|b| {
                matches!(
                    b,
                    Block::Equation {
                        is_block: false,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(
            eq_count, 0,
            "expected 0 inline equation blocks, got {:#?}",
            doc.blocks
        );
        // Paragraph should contain no raw $ delimiters
        let paragraph_text: String = doc
            .blocks
            .iter()
            .filter_map(|b| {
                if let Block::Paragraph { runs, .. } = b {
                    Some(runs.iter().map(|r| r.text.as_str()).collect::<String>())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            !paragraph_text.contains("$"),
            "paragraph should not contain raw $ delimiters, got: {}",
            paragraph_text
        );
    }

    #[test]
    fn lower_inline_math_multiple() {
        // V2: inline math $...$ stays in paragraph, no separate Block::Equation created
        let src = "We have $a + b = c$ and also $x^2$.";
        let p = parse(src);
        let mut macros = crate::expand::MacroMap::new();
        let doc = lower_with_macros(&p, None, &mut macros);
        // NO separate Block::Equation blocks for inline math
        let eq_count = doc
            .blocks
            .iter()
            .filter(|b| {
                matches!(
                    b,
                    Block::Equation {
                        is_block: false,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(eq_count, 0, "expected 0 inline equation blocks");
        // Paragraph should contain no raw $ delimiters
        let paragraph_text: String = doc
            .blocks
            .iter()
            .filter_map(|b| {
                if let Block::Paragraph { runs, .. } = b {
                    Some(runs.iter().map(|r| r.text.as_str()).collect::<String>())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            !paragraph_text.contains("$"),
            "paragraph should not contain raw $ delimiters, got: {}",
            paragraph_text
        );
    }

    #[test]
    fn lower_inline_math_block_math_not_affected() {
        // Block math $$...$$ should NOT be split into inline equations
        let src = "Block: $$x + y = z$$ done.";
        let p = parse(src);
        let mut macros = crate::expand::MacroMap::new();
        let doc = lower_with_macros(&p, None, &mut macros);
        // $$...$$ stays in paragraph as literal text
        let paragraph_text: String = doc
            .blocks
            .iter()
            .filter_map(|b| {
                if let Block::Paragraph { runs, .. } = b {
                    Some(runs.iter().map(|r| r.text.as_str()).collect::<String>())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            paragraph_text.contains("$$"),
            "$$ should remain as-is in paragraph, got: {}",
            paragraph_text
        );
        // No inline equation should be created
        let inline_eq_count = doc
            .blocks
            .iter()
            .filter(|b| {
                matches!(
                    b,
                    Block::Equation {
                        is_block: false,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(inline_eq_count, 0);
    }

    #[test]
    fn lower_cite_single() {
        let src = "As shown in \\cite{smith2020}, we have...";
        let p = parse(src);
        let mut macros = crate::expand::MacroMap::new();
        let doc = lower_with_macros(&p, None, &mut macros);
        let text: String = doc
            .blocks
            .iter()
            .filter_map(|b| {
                if let Block::Paragraph { runs, .. } = b {
                    Some(runs.iter().map(|r| r.text.as_str()).collect::<String>())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            text.contains("[1]"),
            "expected [1] for smith2020, got: {}",
            text
        );
    }

    #[test]
    fn lower_cite_multiple_unique() {
        let src = "As shown in \\cite{smith2020} and \\cite{jones2019}, we have...";
        let p = parse(src);
        let mut macros = crate::expand::MacroMap::new();
        let doc = lower_with_macros(&p, None, &mut macros);
        let text: String = doc
            .blocks
            .iter()
            .filter_map(|b| {
                if let Block::Paragraph { runs, .. } = b {
                    Some(runs.iter().map(|r| r.text.as_str()).collect::<String>())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            text.contains("[1]") && text.contains("[2]"),
            "got: {}",
            text
        );
    }

    #[test]
    fn lower_cite_multiple_same_key() {
        // Same key cited twice → same number
        let src = "First \\cite{smith2020} and later \\cite{smith2020} again.";
        let p = parse(src);
        let mut macros = crate::expand::MacroMap::new();
        let doc = lower_with_macros(&p, None, &mut macros);
        let text: String = doc
            .blocks
            .iter()
            .filter_map(|b| {
                if let Block::Paragraph { runs, .. } = b {
                    Some(runs.iter().map(|r| r.text.as_str()).collect::<String>())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            text.contains("[1]"),
            "expected [1] (same cite twice), got: {}",
            text
        );
        // Should not have [2]
        assert!(
            !text.contains("[2]"),
            "same key twice should be [1][1], got: {}",
            text
        );
    }

    #[test]
    fn lower_cite_comma_separated() {
        // \cite{key1,key2} → [n1,n2]
        let src = "See \\cite{smith2020,jones2019} for details.";
        let p = parse(src);
        let mut macros = crate::expand::MacroMap::new();
        let doc = lower_with_macros(&p, None, &mut macros);
        let text: String = doc
            .blocks
            .iter()
            .filter_map(|b| {
                if let Block::Paragraph { runs, .. } = b {
                    Some(runs.iter().map(|r| r.text.as_str()).collect::<String>())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            text.contains("[1,2]") || (text.contains("[1]") && text.contains("[2]")),
            "expected comma-separated cite numbers, got: {}",
            text
        );
    }

    #[test]
    fn lower_cite_across_paragraphs() {
        // Citation numbers persist across paragraphs
        let src = "First para \\cite{smith2020}.\n\nSecond para \\cite{jones2019}.";
        let p = parse(src);
        let mut macros = crate::expand::MacroMap::new();
        let doc = lower_with_macros(&p, None, &mut macros);
        let text: String = doc
            .blocks
            .iter()
            .filter_map(|b| {
                if let Block::Paragraph { runs, .. } = b {
                    Some(runs.iter().map(|r| r.text.as_str()).collect::<String>())
                } else {
                    None
                }
            })
            .collect();
        // smith2020 = [1], jones2019 = [2] (cross-paragraph numbering)
        assert!(
            text.contains("[1]") && text.contains("[2]"),
            "got: {}",
            text
        );
    }

    #[test]
    fn lower_inline_math_and_cite_together() {
        // V2: inline math stays in paragraph, no separate Block::Equation created
        let src = "According to $E=mc^2$ \\cite{einstein1905}, we get $a+b=c$.";
        let p = parse(src);
        let mut macros = crate::expand::MacroMap::new();
        let doc = lower_with_macros(&p, None, &mut macros);
        // NO separate Block::Equation blocks for inline math
        let eq_count = doc
            .blocks
            .iter()
            .filter(|b| {
                matches!(
                    b,
                    Block::Equation {
                        is_block: false,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(eq_count, 0, "expected 0 inline equation blocks");
        let text: String = doc
            .blocks
            .iter()
            .filter_map(|b| {
                if let Block::Paragraph { runs, .. } = b {
                    Some(runs.iter().map(|r| r.text.as_str()).collect::<String>())
                } else {
                    None
                }
            })
            .collect();
        assert!(text.contains("[1]"), "expected [1] for cite, got: {}", text);
        // Paragraph should contain no raw $ delimiters
        assert!(
            !text.contains("$"),
            "paragraph should not contain raw $ delimiters, got: {}",
            text
        );
    }

    #[test]
    fn lower_abstract_with_chinese_preserved() {
        // Regression: Chinese text in macro bodies must be preserved
        let src = "\\newcommand{\\AbstractContentZh}{微服务架构下，日志来源} \
                    \\begin{rjabstract}\
                    \\AbstractContentZh\
                    \\end{rjabstract}";
        let p = parse(src);
        let mut macros = crate::expand::MacroMap::new();
        let doc = lower_with_macros(&p, None, &mut macros);
        let text: String = doc
            .blocks
            .iter()
            .filter_map(|b| match b {
                Block::Paragraph { runs, .. } => {
                    Some(runs.iter().map(|r| r.text.as_str()).collect::<String>())
                }
                _ => None,
            })
            .collect();
        assert!(
            text.contains("微服务架构下"),
            "Chinese abstract text missing: {}",
            text
        );
    }

    // ── M6: 嵌套表格支持 ──────────────────────────────────────────────────────

    #[test]
    fn lower_nested_table() {
        // Nested tabular in a cell
        let src = "\\begin{tabular}{c|c}\na & \\begin{tabular}{c}inner\\end{tabular} \\\\\n\\end{tabular}";
        let p = parse(src);
        let doc = lower_to_document(&p, None);
        // Should produce a table with nested table handled gracefully
        let table = doc.blocks.iter().find_map(|b| {
            if let Block::Table { rows, .. } = b {
                Some(rows)
            } else {
                None
            }
        });
        assert!(
            table.is_some(),
            "expected a table block, got: {:#?}",
            doc.blocks
        );
        let rows = table.unwrap();
        // First row: "a" and nested table placeholder (flattened)
        assert_eq!(rows[0].cells.len(), 2);
        let nested_cell = &rows[0].cells[1];
        // Nested table content "inner" is extracted and placed in cell
        assert!(
            nested_cell.runs.iter().any(|r| r.text.contains("inner")),
            "nested cell should contain 'inner', got: {:?}",
            nested_cell.runs
        );
    }

    #[test]
    fn lower_nested_table_content_preserved() {
        // Verify nested table content is extracted
        let src = "\\begin{tabular}{c|c}\na & \\begin{tabular}{c}inner\\end{tabular} \\\\\n\\end{tabular}";
        let p = parse(src);
        let doc = lower_to_document(&p, None);
        let table = doc
            .blocks
            .iter()
            .find_map(|b| {
                if let Block::Table { rows, .. } = b {
                    Some(rows)
                } else {
                    None
                }
            })
            .unwrap();
        // Nested cell should contain "inner" from the nested tabular
        let nested_cell = &table[0].cells[1];
        let text: String = nested_cell.runs.iter().map(|r| r.text.as_str()).collect();
        assert!(
            text.contains("inner"),
            "nested table content should be preserved, got: {}",
            text
        );
    }

    #[test]
    fn lower_multicolumn() {
        let src = "\\begin{tabular}{ccc}\\multicolumn{2}{c}{Merged} & C \\\\ A & B & C \\\\ \\end{tabular}";
        let p = parse(src);
        let doc = lower_to_document(&p, None);
        match &doc.blocks[0] {
            Block::Table { rows, .. } => {
                assert_eq!(rows.len(), 2);
                // First cell of first row should have colspan=2
                assert_eq!(rows[0].cells[0].colspan, 2);
                assert_eq!(rows[0].cells[0].runs[0].text, "Merged");
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn lower_rowcolor() {
        // \rowcolor at start of row sets bg_color on all cells
        let src = "\\begin{tabular}{cc}\\rowcolor{lightgray}A & B \\\\ C & D \\\\ \\end{tabular}";
        let p = parse(src);
        let doc = lower_to_document(&p, None);
        match &doc.blocks[0] {
            Block::Table { rows, .. } => {
                assert_eq!(rows.len(), 2);
                // First row should have bg_color
                assert!(rows[0].cells[0].bg_color.is_some());
                assert_eq!(rows[0].cells[0].bg_color.as_deref(), Some("lightgray"));
                assert!(rows[0].cells[1].bg_color.is_some());
                // Second row should not have bg_color
                assert!(rows[1].cells[0].bg_color.is_none());
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn lower_rowcolor_with_model() {
        // \rowcolor[rgb]{0.5,0.5,0.5} style
        let src = "\\begin{tabular}{cc}\\rowcolor[HTML]{FF0000}A & B \\\\n\\end{tabular}";
        let p = parse(src);
        let doc = lower_to_document(&p, None);
        match &doc.blocks[0] {
            Block::Table { rows, .. } => {
                // First row should have bg_color
                assert!(rows[0].cells[0].bg_color.is_some());
                assert_eq!(rows[0].cells[0].bg_color.as_deref(), Some("FF0000"));
            }
            _ => panic!("expected table"),
        }
    }

    // ── M8: 完整编号测试 ──────────────────────────────────────────────────────

    #[test]
    fn lower_heading_auto_number() {
        let src =
            "\\section{First}\n\n\\subsection{Sub}\n\n\\section{Second}\n\n\\subsection{Sub2}";
        let p = parse(src);
        let doc = lower_to_document(&p, None);
        let headings: Vec<_> = doc
            .blocks
            .iter()
            .filter_map(|b| {
                if let Block::Heading {
                    level,
                    text,
                    number,
                    ..
                } = b
                {
                    Some((*level, text.clone(), number.clone()))
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(headings[0], (1, "First".to_string(), Some("1".to_string())));
        assert_eq!(headings[1], (2, "Sub".to_string(), Some("1.1".to_string())));
        assert_eq!(
            headings[2],
            (1, "Second".to_string(), Some("2".to_string()))
        );
        assert_eq!(
            headings[3],
            (2, "Sub2".to_string(), Some("2.1".to_string()))
        );
    }

    #[test]
    fn lower_figure_auto_number() {
        let src = "\\begin{figure}\n\\includegraphics{x.png}\n\\end{figure}\
                \\begin{figure}\n\\includegraphics{y.png}\n\\end{figure}";
        let p = parse(src);
        let doc = lower_to_document(&p, None);
        let figs: Vec<_> = doc
            .blocks
            .iter()
            .filter_map(|b| {
                if let Block::Figure { number, .. } = b {
                    Some(number.clone())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(figs[0], Some("图 1".to_string()));
        assert_eq!(figs[1], Some("图 2".to_string()));
    }

    #[test]
    fn lower_table_auto_number() {
        let src = "\\begin{table}\n\\begin{tabular}{cc}\nA & B \\\\\nC & D \\\\\n\\end{tabular}\n\\end{table}\
                \\begin{table}\n\\begin{tabular}{cc}\nE & F \\\\\n\\end{tabular}\n\\end{table}";
        let p = parse(src);
        let doc = lower_to_document(&p, None);
        let tbls: Vec<_> = doc
            .blocks
            .iter()
            .filter_map(|b| {
                if let Block::Table { number, .. } = b {
                    Some(number.clone())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(tbls[0], Some("表 1".to_string()));
        assert_eq!(tbls[1], Some("表 2".to_string()));
    }
}
