use doc_latex_reader::{expand::MacroMap, lower_with_macros, parse_tex};
use doc_semantic_ast::{Block, Document};
use doc_utils::VirtualFs;
use std::path::Path;
use std::fs;
use doc_latex_reader::IncludeGraph;

fn blocks_to_text(doc: &Document) -> String {
    let mut text = String::new();
    for block in &doc.blocks {
        match block {
            Block::Paragraph { runs, .. } => {
                for r in runs {
                    text.push_str(&r.text);
                }
                text.push('\n');
            }
            Block::Heading { text: t, .. } => {
                text.push_str(t);
                text.push('\n');
            }
            _ => {}
        }
    }
    text
}

/// 走 IncludeGraph（与 doc-core 一致）的 paper3 main-jos 解析
#[test]
fn paper3_main_jos_via_include_graph() {
    let latex_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/paper3/latex");
    let mut vfs = VirtualFs::new();
    vfs.mount_dir(Path::new(latex_dir)).expect("mount_dir");
    let graph = IncludeGraph::build(&vfs, Path::new("main-jos.tex")).expect("build graph");
    let joined = graph.join(&vfs).expect("join");
    let parse = parse_tex(&joined.text);
    let mut macros = MacroMap::new();
    let doc = lower_with_macros(&parse, Some(&joined), &mut macros);
    let text = blocks_to_text(&doc);
    let snippet: String = text.chars().take(800).collect();
    fs::write(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/paper3/build/test-output.txt"), &snippet).ok();
    eprintln!("---- DOC BLOCKS (first 800) ----\n{}", &snippet);
    // 找 "In microservice" 或 "Gateway-Traffic" 周围
    for needle in ["In microservice", "Gateway-Traffic", "Abstract", "shihonglei", "References"] {
        if let Some(idx) = text.find(needle) {
            // idx 是字节偏移；调整 s/e 到 char 边界
            let s = {
                let mut p = idx.saturating_sub(50);
                while p > 0 && !text.is_char_boundary(p) {
                    p -= 1;
                }
                p
            };
            let e = {
                let mut p = (idx + 200).min(text.len());
                while p < text.len() && !text.is_char_boundary(p) {
                    p += 1;
                }
                p
            };
            let around = &text[s..e];
            eprintln!("---- Around {needle} ----\n{around}\n----");
        } else {
            eprintln!("---- NOT FOUND: {needle}");
        }
    }
    assert!(
        text.contains("微服务架构下") || text.contains("网关流量"),
        "expected abstract"
    );
}
