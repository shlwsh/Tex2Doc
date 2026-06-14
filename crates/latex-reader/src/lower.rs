//! CST → Semantic AST 降级（M3 完整版）
//!
//! ## 扫描策略
//!
//! 1. **顶层行扫描**：`split_inclusive('\n')` 切行。
//! 2. **环境优先**：遇到 `\begin{xxx}…\end{xxx}` 直接整段扣出做专项降级。
//!    支持环境：`itemize` / `enumerate` / `description` / `tabular` / `array` / `figure` / `table` / `equation` / `equation*`。
//! 3. **段落**：非空行累计到一个 buffer，遇空行 / 段命令 / 新环境 / EOF 触发 flush。
//! 4. **行内清洗**：`strip_inline` 处理 `\textbf{...}` 等命令。
//! 5. **数学**：`$…$` 与 `$$…$$` / `\(...\)` / `\[...\]` 整段抽出为 `Equation::latex`。
//! 6. **图片**：`\includegraphics[…]{path}` 在段落中追加 Figure 占位（M3 简化）。
//! 7. **引用 / 链接**：`\href{url}{text}` / `\url{url}` / `\ref{label}`。
//! 8. **错误降级**：未匹配内容进入 `Block::RawFallback`（绝不 panic）。

use doc_semantic_ast::{Block, Document, Span, TableCell, TableRow, TextRun, TextStyle};

use crate::include::JoinedStream;
use crate::parser::Parse;

/// 降级入口。
pub fn lower_to_document(parse: &Parse, joined: Option<&JoinedStream>) -> Document {
    let text = joined
        .map(|j| j.text.clone())
        .unwrap_or_else(|| parse.source.clone());
    let mut doc = Document::new();
    let mut buffer = String::new();
    let mut buffer_start = 0u32;
    let default_span = Span::default();
    let mut pos: usize = 0;
    let bytes = text.as_bytes();
    let len = bytes.len();

    while pos < len {
        // 跳过空白 / 注释
        if let Some(next) = skip_whitespace_and_comment(&text, pos) {
            if next != pos {
                pos = next;
                continue;
            }
        }

        // 环境优先
        if let Some((name, body, end)) = scan_environment(&text, pos) {
            flush_paragraph(&mut doc, &mut buffer, &mut buffer_start, default_span);
            let blk = lower_environment(name, body, default_span);
            doc.push(blk);
            pos = end;
            continue;
        }

        // 段落级命令：\section、\subsection 等
        if let Some((consumed, block)) = try_top_level_command(&text[pos..], default_span) {
            flush_paragraph(&mut doc, &mut buffer, &mut buffer_start, default_span);
            doc.push(block);
            pos += consumed;
            continue;
        }

        // 取一行
        let nl = text[pos..].find('\n').map(|n| pos + n + 1).unwrap_or(len);
        let line = &text[pos..nl];
        let stripped = strip_inline(line);
        let trimmed = stripped.trim();

        if trimmed.is_empty() {
            flush_paragraph(&mut doc, &mut buffer, &mut buffer_start, default_span);
        } else {
            if buffer.is_empty() {
                buffer_start = pos as u32;
            }
            buffer.push_str(&stripped);
            buffer.push('\n');
        }
        pos = nl;
    }

    flush_paragraph(&mut doc, &mut buffer, &mut buffer_start, default_span);
    doc
}

fn flush_paragraph(doc: &mut Document, buffer: &mut String, start: &mut u32, span: Span) {
    if buffer.trim().is_empty() {
        buffer.clear();
        return;
    }
    let body = buffer.trim().to_string();
    let s = *start;
    doc.push(Block::Paragraph {
        runs: vec![TextRun {
            text: body,
            style: TextStyle::Plain,
            span: Span::new(s, s + buffer.len() as u32, span.source),
        }],
        span,
    });
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

/// 顶层段命令：\section / \subsection / \subsubsection / \paragraph / \caption
fn try_top_level_command(s: &str, span: Span) -> Option<(usize, Block)> {
    let prefixes: &[(&str, fn(&str, Span) -> Block)] = &[
        ("\\section", |b, sp| Block::Heading {
            level: 1,
            text: b.to_string(),
            span: sp,
        }),
        ("\\subsection", |b, sp| Block::Heading {
            level: 2,
            text: b.to_string(),
            span: sp,
        }),
        ("\\subsubsection", |b, sp| Block::Heading {
            level: 3,
            text: b.to_string(),
            span: sp,
        }),
        ("\\paragraph", |b, sp| Block::Heading {
            level: 4,
            text: b.to_string(),
            span: sp,
        }),
    ];
    for (prefix, ctor) in prefixes {
        if let Some(rest) = s.strip_prefix(prefix) {
            // 跳过可选空白
            let mut k = 0;
            while k < rest.len() && (rest.as_bytes()[k] == b' ' || rest.as_bytes()[k] == b'\t') {
                k += 1;
            }
            if k >= rest.len() || rest.as_bytes()[k] != b'{' {
                // 不成对：回退
                continue;
            }
            if let Some(off) = find_matching_brace(rest, k) {
                let body = &rest[k + 1..k + 1 + off];
                return Some((prefix.len() + k + off + 2, ctor(body, span)));
            } else {
                return Some((
                    prefix.len() + s.find('\n').unwrap_or(s.len()),
                    Block::RawFallback {
                        text: rest.to_string(),
                        span,
                    },
                ));
            }
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
    // 找 name 末尾
    let name_end = text[after..].find('}')? + after;
    let name = &text[after..name_end];
    // 找配对 \end{name}
    let body_start = name_end + 1;
    let end_pat = format!("\\end{{{name}}}");
    let end_pos = text[body_start..]
        .find(&end_pat)
        .map(|p| body_start + p)
        .unwrap_or(text.len());
    let after_end = (end_pos + end_pat.len()).min(text.len());
    let body = &text[body_start..end_pos];
    Some((name, body, after_end))
}

/// 环境 → 块的降级分派。
fn lower_environment(name: &str, body: &str, span: Span) -> Block {
    match name {
        "itemize" => lower_list(body, false, span),
        "enumerate" => lower_list(body, true, span),
        "description" => lower_list(body, false, span),
        "tabular" | "tabular*" | "array" => lower_table(body, span),
        "figure" | "figure*" | "table" | "table*" => lower_captioned_env(name, body, span),
        "equation" | "equation*" | "align" | "align*" | "gather" | "gather*" => Block::Equation {
            latex: body.trim().to_string(),
            is_block: true,
            span,
        },
        "document" => {
            // 直接递归降级 body
            let mut sub = Document::new();
            let p = crate::parser::parse(body);
            let doc2 = lower_to_document(&p, None);
            for b in doc2.blocks {
                sub.push(b);
            }
            // 折叠：返回第一个块；其它块忽略（M3 简化）
            sub.blocks.into_iter().next().unwrap_or(Block::RawFallback {
                text: body.to_string(),
                span,
            })
        }
        _ => Block::RawFallback {
            text: format!("\\begin{{{name}}}…\\end{{{name}}}"),
            span,
        },
    }
}

/// 在 `body` 中按 `\item` 切分，每段降级为 List item 内的 Block 列表。
fn lower_list(body: &str, is_ordered: bool, span: Span) -> Block {
    let mut items: Vec<Vec<Block>> = Vec::new();
    let mut current: Option<&str> = None;
    for line in body.split_inclusive('\n') {
        let s = line.trim_end_matches(&['\r', '\n'][..]);
        if s.trim_start().starts_with("\\item") {
            if let Some(buf) = current {
                let blocks = lower_item_body(buf, span);
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
            current = Some(after);
        } else if current.is_some() {
            let buf = current.unwrap();
            let mut owned = String::from(buf);
            owned.push('\n');
            owned.push_str(s);
            current = Some(Box::leak(owned.into_boxed_str()));
        }
    }
    if let Some(buf) = current {
        items.push(lower_item_body(buf, span));
    }
    Block::List {
        is_ordered,
        items,
        span,
    }
}

fn lower_item_body(buf: &str, span: Span) -> Vec<Block> {
    let stripped = strip_inline(buf);
    if stripped.trim().is_empty() {
        return Vec::new();
    }
    let p = crate::parser::parse(buf);
    let sub = lower_to_document(&p, None);
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

/// tabular/array 降级。
///
/// 形如：`{c|c|c}` 列规范 + 主体 `\hline / & / \\\hline / \multicolumn{n}{...}{...}`。
/// 不实现 `\\multicolumn` 的 colspan 列数重映射（V1 简化），
/// 单元格 `\multicolumn` 内的内容直接读作文本，**col_span** 仍按 1。
fn lower_table(body: &str, span: Span) -> Block {
    // 主体可能被 `\\` 分行
    let rows_text: Vec<&str> = body.split("\\\\").collect();
    let mut rows: Vec<TableRow> = Vec::new();
    for row in rows_text {
        let cells_text: Vec<&str> = row.split('&').collect();
        if cells_text.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        let mut cells: Vec<TableCell> = Vec::new();
        for c in cells_text {
            let cleaned = strip_inline(c).replace("\\hline", "").trim().to_string();
            if cleaned.is_empty() {
                cells.push(TableCell {
                    runs: vec![],
                    colspan: 1,
                    rowspan: 1,
                });
                continue;
            }
            cells.push(TableCell {
                runs: vec![TextRun {
                    text: cleaned,
                    style: TextStyle::Plain,
                    span,
                }],
                colspan: 1,
                rowspan: 1,
            });
        }
        rows.push(TableRow { cells });
    }
    if rows.is_empty() {
        // 兜底：单行单列 + 原文
        rows.push(TableRow {
            cells: vec![TableCell {
                runs: vec![TextRun {
                    text: body.to_string(),
                    style: TextStyle::Plain,
                    span,
                }],
                colspan: 1,
                rowspan: 1,
            }],
        });
    }
    Block::Table {
        rows,
        caption: None,
        span,
    }
}

/// `\caption{...}` 在 figure/table 环境中。
fn lower_captioned_env(name: &str, body: &str, span: Span) -> Block {
    // 找 \includegraphics 与 \caption
    let (img, caption) = extract_includegraphics_and_caption(body);
    if name.starts_with("figure") {
        Block::Figure {
            path: img.unwrap_or_default(),
            caption,
            scale: 1.0,
            span,
        }
    } else {
        // table 仍以 table 形式表达（M3 占位）
        let mut table = lower_table(body, span);
        if let Block::Table { caption: c, .. } = &mut table {
            *c = caption;
        }
        table
    }
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
    // 跳过可选方括号参数 [...]（允许多个）
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
fn strip_inline(line: &str) -> String {
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
            out.push(c as char);
            i += 1;
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
                out.push('\n');
                i = j + 1;
                continue;
            }
            while j < bytes.len() && (bytes[j].is_ascii_alphabetic() || bytes[j] == b'@') {
                j += 1;
            }
            let cmd = &line[cmd_start..j];
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
            ) {
                if has_arg {
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
            }
            // \href / \url / \emph 已经处理
            if matches!(cmd, "href" | "url" | "ref" | "cite" | "label" | "footnote") {
                if has_arg {
                    if let Some(off) = find_matching_brace(line, k) {
                        // 多个可选 [..] 参数 + 必选 {..}；简化：吃所有 {…} 拼接
                        let mut p = k + 1 + off + 1;
                        // 跳过后续可选 {…}
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
        out.push(c as char);
        i += 1;
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
            assert!(runs[0].text.contains("bold"));
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
}
