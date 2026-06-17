//! Paper3 端到端 dump：解压 zip → VFS → IncludeGraph → lower → 打印块列表。
//!
//! 运行：`PAPER3_ZIP=$PWD/examples/paper3/upload_full.zip cargo test -p doc-latex-reader --test dump_paper3 -- --nocapture`

use std::io::Read;
use std::path::Path;

#[test]
fn dump_paper3_blocks() {
    let zip_path = std::env::var("PAPER3_ZIP").unwrap_or_else(|_| {
        // 从 crates/latex-reader/ 回到工程根，再指向 examples/paper3/upload_full.zip
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/paper3/upload_full.zip");
        p.to_string_lossy().to_string()
    });
    let main_path = std::env::var("PAPER3_MAIN").unwrap_or_else(|_| "main-jos.tex".to_string());

    use doc_utils::VirtualFs;
    let mut vfs = VirtualFs::new();
    if Path::new(&zip_path).exists() {
        let zip_bytes = std::fs::read(&zip_path).expect("read zip");
        let mut archive =
            zip::ZipArchive::new(std::io::Cursor::new(&zip_bytes[..])).expect("open zip");
        for i in 0..archive.len() {
            let mut f = archive.by_index(i).expect("entry");
            if f.is_dir() {
                continue;
            }
            let name = f.name().to_string().replace('\\', "/");
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).expect("read entry");
            vfs.insert(name, buf);
        }
    } else {
        let latex_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/paper3/latex");
        vfs.mount_dir(&latex_dir)
            .expect("mount examples/paper3/latex");
    }
    let graph = doc_latex_reader::IncludeGraph::build(&vfs, Path::new(&main_path)).expect("graph");
    let joined = graph.join(&vfs).expect("join");

    let parse = doc_latex_reader::parse_tex(&joined.text);
    let doc = doc_latex_reader::lower_to_document(&parse, Some(&joined));

    eprintln!("[dump_paper3] blocks: {}", doc.blocks.len());
    for (i, b) in doc.blocks.iter().enumerate() {
        let label = match b {
            doc_semantic_ast::Block::Heading { level, text, .. } => {
                format!("H{level}={text}")
            }
            doc_semantic_ast::Block::Paragraph { runs, .. } => {
                let t: String = runs.iter().map(|r| r.text.as_str()).collect();
                let previews: Vec<String> = runs
                    .iter()
                    .take(6)
                    .map(|r| {
                        let s: String = r.text.chars().take(40).collect();
                        format!("[{:?}:{}]", r.style, s)
                    })
                    .collect();
                format!(
                    "P({}) runs={}={} | first_runs={:?}",
                    t.chars().count(),
                    runs.len(),
                    t.chars().take(80).collect::<String>(),
                    previews
                )
            }
            doc_semantic_ast::Block::Figure { path, caption, .. } => {
                format!("F={path} cap={}", caption.as_deref().unwrap_or(""))
            }
            doc_semantic_ast::Block::Table { rows, .. } => format!("T(rows={})", rows.len()),
            doc_semantic_ast::Block::List {
                is_ordered, items, ..
            } => {
                format!("L(o={is_ordered},n={})", items.len())
            }
            doc_semantic_ast::Block::Equation {
                latex, is_block, ..
            } => format!("E(b={is_block},{})", &latex[..latex.len().min(40)]),
            doc_semantic_ast::Block::TheoremLike { kind, body, .. } => {
                format!("M({kind:?})={}", body.chars().take(50).collect::<String>())
            }
            doc_semantic_ast::Block::Bibliography { .. } => "B".to_string(),
            doc_semantic_ast::Block::Algorithm { .. } => "Alg".to_string(),
            doc_semantic_ast::Block::RawFallback { text, .. } => format!(
                "R({})={}",
                text.chars().count(),
                text.chars().take(50).collect::<String>()
            ),
        };
        eprintln!("[{:3}] {}", i, label);
    }
}
