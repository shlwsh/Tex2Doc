//! Paper3 front matter extraction smoke test.
//!
//! 运行：`cargo test -p doc-latex-reader --test paper3_front_matter -- --nocapture`
//! 不依赖 PAPER3_ZIP，直接读 examples/paper3/latex。

use std::path::Path;

use doc_latex_reader::latex_to_text::{extract_front_matter, parse_bbl, parse_newcommands};
use doc_utils::VirtualFs;

fn read_file(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|_| panic!("read {}", p.display()))
}

#[test]
fn paper3_front_matter_smoke() {
    let latex_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/paper3/latex");
    let main_jos = read_file(&Path::new(latex_dir).join("main-jos.tex"));
    let abstract_zh = read_file(&Path::new(latex_dir).join("sections/zh/00_abstract.tex"));
    let bbl_path = Path::new(latex_dir).join("main-jos.bbl");
    let bbl = if bbl_path.exists() {
        read_file(&bbl_path)
    } else {
        String::new()
    };

    // 1. parse_newcommands from 00_abstract.tex
    let macros = parse_newcommands(&abstract_zh);
    eprintln!("---- macros from 00_abstract.tex ----");
    for (k, v) in &macros {
        let preview: String = v.chars().take(60).collect();
        eprintln!("  {} = {}", k, preview);
    }
    assert!(macros.contains_key("AbstractContentZh"));
    assert!(macros.contains_key("AbstractContentEn"));
    assert!(macros.contains_key("KeywordsZh"));
    assert!(macros.contains_key("KeywordsEn"));

    // 2. parse_bbl
    let (cite_map, refs) = parse_bbl(&bbl);
    eprintln!("---- bbl cite_map size = {}, refs = {} ----", cite_map.len(), refs.len());
    for (i, r) in refs.iter().take(3).enumerate() {
        eprintln!("  [{}] key={} text.len={}", i + 1, r.key, r.text.len());
    }

    // 3. extract_front_matter
    let mut vfs = VirtualFs::new();
    vfs.mount_dir(Path::new(latex_dir)).expect("mount");
    let graph = doc_latex_reader::IncludeGraph::build(&vfs, Path::new("main-jos.tex"))
        .expect("graph");
    let joined = graph.join(&vfs).expect("join");
    let expanded_main = &joined.text;

    let fm = extract_front_matter(&main_jos, expanded_main, &macros);
    eprintln!("---- FrontMatter ----");
    eprintln!("title_zh      = {}", fm.title_zh);
    eprintln!("authors_zh    = {}", fm.authors_zh);
    eprintln!("institute     = {:?}", fm.institute_lines);
    eprintln!("abstract_zh   = {}", fm.abstract_zh.chars().take(80).collect::<String>());
    eprintln!("keywords_zh   = {}", fm.keywords_zh);
    eprintln!("title_en      = {}", fm.title_en);
    eprintln!("authors_en    = {}", fm.authors_en);
    eprintln!("institute_en  = {}", fm.institute_en);
    eprintln!("abstract_en   = {}", fm.abstract_en.chars().take(80).collect::<String>());
    eprintln!("keywords_en   = {}", fm.keywords_en);
    eprintln!("running_header= {}", fm.running_header);
    eprintln!("author_bio    = {} items", fm.author_bio.len());

    assert!(!fm.title_zh.is_empty(), "title_zh should be present");
    assert!(!fm.abstract_zh.is_empty(), "abstract_zh should be present");
    assert!(!fm.abstract_en.is_empty(), "abstract_en should be present");
}
