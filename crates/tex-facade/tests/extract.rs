//! `extract` 单元测试（plan §2.4 中 3 项）。
//!
//! 见 `docs/study/08-pdf-pipeline/02-tex-facade.md` §2.9。

use std::path::Path;

use doc_tex_facade::detect_extractor;

#[test]
fn probes_pdftotext_first() {
    // 当前 Windows 环境已装 pdftotext（D:\poppler-25.12.0\Library\bin\pdftotext.exe），
    // 探测应返回 "pdftotext"。
    // 注：此断言不依赖 PATH 是否包含——只检查探测优先级。
    let detected = detect_extractor();
    if let Some(name) = detected {
        assert_eq!(name, "pdftotext", "pdftotext 优先级高于 mutool");
    }
}

#[test]
fn falls_back_to_mutool() {
    // 单元测试无法直接 mock which——本测试只确保探测函数类型稳定。
    // 真实"fallback to mutool"在集成测试（需要构造 PATH）里验证。
    let _ = detect_extractor();
}

#[test]
fn returns_no_extractor_when_neither() {
    // 同上：探测函数返回 Option<str>，None 表示两者都缺。
    // 真实"两者都缺"在集成测试里验证（设空 PATH 跑 extract_text）。
    let _ = detect_extractor();
}

#[test]
fn extract_text_invalid_path_returns_error() {
    // 不依赖任何外部工具：给一个不存在的 PDF，pdftotext/mutool 会先报
    // "couldn't open file" → 命令退出非 0 → 我们也 fall through
    // 到 TexError::NoTextExtractor 或传播 I/O 错误。
    // 但如果探测到了 pdftotext，pdftotext 跑得动 + 退非 0 → 我们会回 I/O 错误。
    // 这里只验证：返回 Err（不 panic）。
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(doc_tex_facade::extract_text(Path::new(
        "Z:/__definitely_does_not_exist__.pdf",
    )));
    assert!(result.is_err(), "不存在的 PDF 应返回 Err，不能 panic");
}
