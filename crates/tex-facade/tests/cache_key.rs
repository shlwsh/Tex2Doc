//! `cache_key` 单元测试（plan §2.4 前 3 项）。
//!
//! 见 `docs/study/08-pdf-pipeline/02-tex-facade.md` §2.6.1。

use std::fs;

use doc_tex_facade::{compute_key, CacheKey, EngineKind, TexProject};

/// 起一个临时工作目录，里面有 `main.tex` 与 `figures/a.png`。
fn make_project(tmp: &std::path::Path, main_content: &str) -> TexProject {
    fs::create_dir_all(tmp.join("figures")).unwrap();
    fs::write(tmp.join("figures").join("a.png"), b"\x89PNG_FAKE").unwrap();
    let main = tmp.join("main.tex");
    fs::write(&main, main_content).unwrap();
    TexProject::from_main(&main)
}

#[test]
fn compute_key_is_deterministic() {
    let tmp = tempfile::tempdir().unwrap();
    let p = make_project(tmp.path(), r"\documentclass{article}\input{01_intro}");
    let k1 = compute_key(&p).unwrap();
    let k2 = compute_key(&p).unwrap();
    assert_eq!(k1, k2, "同输入两次哈希必须一致");
}

#[test]
fn compute_key_ignores_path() {
    // 同一字节内容、不同路径 → 同 hash
    let tmp1 = tempfile::tempdir().unwrap();
    let tmp2 = tempfile::tempdir().unwrap();
    let p1 = make_project(tmp1.path(), r"\documentclass{article}");
    let p2 = make_project(tmp2.path(), r"\documentclass{article}");
    let k1 = compute_key(&p1).unwrap();
    let k2 = compute_key(&p2).unwrap();
    assert_eq!(
        k1, k2,
        "同字节内容、不同路径 → 同 hash（§2.7 跨平台要求）"
    );
}

#[test]
fn compute_key_changes_on_content() {
    // 注意：必须用两个独立的 tmpdir，否则第二次写文件会覆盖第一次，导致字节一致
    let tmp1 = tempfile::tempdir().unwrap();
    let tmp2 = tempfile::tempdir().unwrap();
    let p1 = make_project(tmp1.path(), r"\documentclass{article}");
    let p2 = make_project(tmp2.path(), r"\documentclass{ report }"); // 多一个空格
    let k1 = compute_key(&p1).unwrap();
    let k2 = compute_key(&p2).unwrap();
    assert_ne!(
        k1, k2,
        "改 1 字节 → 哈希必须变（缓存内容敏感）"
    );
}

#[test]
fn cache_key_hex_is_64_chars() {
    let key = CacheKey([0u8; 32]);
    assert_eq!(key.hex().len(), 64);
}

#[test]
fn engine_kind_as_str_matches_design() {
    assert_eq!(EngineKind::Xelatex.as_str(), "xelatex");
    assert_eq!(EngineKind::Tectonic.as_str(), "tectonic");
    assert_eq!(EngineKind::Latexmk.as_str(), "latexmk");
}

#[test]
fn referenced_tex_files_lists_inputs() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("01_intro.tex"),
        r"\section{Intro}",
    )
    .unwrap();
    fs::write(
        tmp.path().join("02_related.tex"),
        r"\section{Related}",
    )
    .unwrap();
    let main = tmp.path().join("main.tex");
    fs::write(
        &main,
        r"\documentclass{article}
\input{01_intro}
\include{02_related}
",
    )
    .unwrap();

    let refs = doc_tex_facade::referenced_tex_files(&main).unwrap();
    assert_eq!(refs.len(), 2, "应解析到 2 个 \\input/\\include 子文件");
    let names: Vec<String> = refs
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert!(names.contains(&"01_intro.tex".to_string()));
    assert!(names.contains(&"02_related.tex".to_string()));
}
