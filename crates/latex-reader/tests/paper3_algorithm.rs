//! Algorithm block smoke test against paper3.

use std::path::Path;

use doc_latex_reader::algorithm::parse_algorithm_rows;
use doc_semantic_ast::Block;
use doc_utils::VirtualFs;

fn read_file(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|_| panic!("read {}", p.display()))
}

#[test]
fn paper3_algorithms() {
    let latex_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/paper3/latex");
    let mut vfs = VirtualFs::new();
    vfs.mount_dir(Path::new(latex_dir)).expect("mount");
    let graph = doc_latex_reader::IncludeGraph::build(&vfs, Path::new("main-jos.tex"))
        .expect("graph");
    let joined = graph.join(&vfs).expect("join");
    let parse = doc_latex_reader::parse_tex(&joined.text);
    let doc = doc_latex_reader::lower_to_document(&parse, Some(&joined));

    let mut alg_count = 0;
    for b in &doc.blocks {
        if let Block::Algorithm {
            lines,
            io,
            caption,
            number,
            ..
        } = b
        {
            alg_count += 1;
            eprintln!("---- Algorithm {} (cap={:?}) ----", alg_count, caption);
            eprintln!("  io: {:?}", io);
            eprintln!("  number: {:?}", number);
            eprintln!("  lines: {} rows", lines.len());
            for (i, l) in lines.iter().take(10).enumerate() {
                eprintln!(
                    "    [{}] indent={} kw={:?} code={} comment={}",
                    i,
                    l.indent,
                    l.keyword,
                    l.code.chars().take(60).collect::<String>(),
                    l.comment
                );
            }
        }
    }
    assert!(alg_count >= 1, "expected at least 1 algorithm block");
    eprintln!("Total algorithm blocks: {}", alg_count);
}
