//! insta 快照测试
//!
//! 对 `parse_tex + lower_to_document` 的输出做稳定字符串快照，
//! 防止 AST 字段调整引发的回归。

use doc_latex_reader::{lower_to_document, parse_tex};
use doc_semantic_ast::Document;

fn format_doc(doc: &Document) -> String {
    // 简化格式：每行一个块
    let mut out = String::new();
    for (i, b) in doc.blocks.iter().enumerate() {
        out.push_str(&format!("[{i}] {:?}\n", b));
    }
    out
}

#[test]
fn snapshot_simple() {
    let src = "\\section{Intro}\n\nThis is a paragraph.\n\n\\textbf{Bold} here.\n";
    let p = parse_tex(src);
    let doc = lower_to_document(&p, None);
    insta::assert_snapshot!("simple_doc", format_doc(&doc));
}

#[test]
fn snapshot_list() {
    let src = "\\begin{itemize}\\item A\\item B\\item C\\end{itemize}";
    let p = parse_tex(src);
    let doc = lower_to_document(&p, None);
    insta::assert_snapshot!("list_doc", format_doc(&doc));
}

#[test]
fn snapshot_table() {
    let src = "\\begin{tabular}{c|c}A & B\\\\C & D\\\\\\end{tabular}";
    let p = parse_tex(src);
    let doc = lower_to_document(&p, None);
    insta::assert_snapshot!("table_doc", format_doc(&doc));
}
