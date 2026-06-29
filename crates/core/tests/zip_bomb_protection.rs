//! 回归测试：保护 WASM 与 CLI 端转换不被 zip bomb 触发 `invalid malloc request`。
//!
//! 历史问题：
//! `convert_zip` 里 `Vec::with_capacity(f.size() as usize)` 在
//! 1) 32 位 wasm 下把 `u64` 截断为 `u32` / 0，触发 wasm-bindgen malloc 越界；
//! 2) 上传文件声明 `u64::MAX` 时直接分配失败，panic 报
//!    "invalid malloc request"。
//!
//! 本测试构造一个声明大小超过 `MAX_ZIP_ENTRY_BYTES`（128 MiB）的 zip，
//! 验证 `convert_zip` 返回一个清晰的 `CoreError::Io`，而不是 panic。

use std::io::Write;

const NEEDLE_LFH: [u8; 4] = [0x50, 0x4B, 0x03, 0x04]; // PK\x03\x04

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// 构造一个合法的 zip（含 main.tex），但把中央目录里的 "uncompressed size"
/// 与 local header 里的对应字段同时改成接近 `u64::MAX` 的值，模拟一个
/// 声称无比巨大、实际只解压出几字节的 zip bomb。
fn make_zip_bomb() -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("main.tex", opts).unwrap();
        zip.write_all(b"\\documentclass{article}\n\\begin{document}Hi\\end{document}\n")
            .unwrap();
        zip.finish().unwrap();
    }
    // 把所有 LFH 的 "compressed size" + "uncompressed size"（偏移 18..26）改成超大值
    let mut p = 0usize;
    while let Some(pos) = find_subslice(&buf[p..], &NEEDLE_LFH) {
        let abs = p + pos;
        let huge: [u8; 8] = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00];
        // LFH: 字段 18..26 = compressed size, 26..34 = uncompressed size
        buf[abs + 18..abs + 18 + 8].copy_from_slice(&huge);
        buf[abs + 26..abs + 26 + 8].copy_from_slice(&huge);
        p = abs + 30;
    }
    // 同步修改中央目录（CDH 字段 20..28 = compressed size, 28..36 = uncompressed size）
    // CDH signature: PK\x01\x02
    let cdh_needle: [u8; 4] = [0x50, 0x4B, 0x01, 0x02];
    let mut p = 0usize;
    while let Some(pos) = find_subslice(&buf[p..], &cdh_needle) {
        let abs = p + pos;
        let huge: [u8; 8] = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00];
        buf[abs + 20..abs + 20 + 8].copy_from_slice(&huge);
        buf[abs + 28..abs + 28 + 8].copy_from_slice(&huge);
        p = abs + 46;
    }
    // EOCD "size of central directory" 不必动；zip reader 对单条 entry
    // 的 size 校验优先于 EOCD 完整性。
    buf
}

#[test]
fn convert_zip_rejects_zip_bomb_declaration() {
    let bomb = make_zip_bomb();
    let res = doc_core::convert_zip(&bomb, "main.tex", &doc_core::ConvertOptions::default());
    // 我们的修复期望：要么被 zip 库校验挡住（IO 错），要么被 size limit 挡住。
    // 关键是不能 panic / abort / `invalid malloc request`。
    let err = match res {
        Ok(r) => panic!(
            "convert_zip should reject oversized zip entries, got docx.len={}",
            r.docx.len()
        ),
        Err(e) => e,
    };
    let msg = format!("{err}");
    assert!(
        msg.contains("超过")
            || msg.contains("上限")
            || msg.contains("unsafe_path")
            || msg.contains("InvalidZip")
            || msg.contains("zip"),
        "expected zip/size error, got: {msg}"
    );
}

#[test]
fn convert_zip_accepts_normal_small_zip() {
    // 走完整 happy path：构造一个含 main.tex 的正常 zip，验证仍能成功转换。
    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("main.tex", opts).unwrap();
        zip.write_all(b"\\documentclass{article}\n\\begin{document}Hello\\end{document}\n")
            .unwrap();
        zip.finish().unwrap();
    }
    let res = doc_core::convert_zip(&buf, "main.tex", &doc_core::ConvertOptions::default())
        .expect("normal zip should convert");
    assert!(!res.docx.is_empty());
}

#[test]
fn convert_zip_handles_long_main_tex() {
    // 中等大小主文件（数十 KB）不应触发 size limit。
    let mut main = String::with_capacity(64 * 1024);
    main.push_str("\\documentclass{article}\n\\begin{document}\n");
    for i in 0..1024 {
        main.push_str(&format!("This is paragraph {i} with some text.\n\n"));
    }
    main.push_str("\\end{document}\n");

    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("main.tex", opts).unwrap();
        zip.write_all(main.as_bytes()).unwrap();
        zip.finish().unwrap();
    }
    let res = doc_core::convert_zip(&buf, "main.tex", &doc_core::ConvertOptions::default())
        .expect("long main.tex should still convert");
    assert!(!res.docx.is_empty());
}
