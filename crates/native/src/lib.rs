//! Doc-engine 原生桥接（cdylib）—— 给 Flutter 桌面端 `dart:ffi` 调用。
//!
//! 暴露三个 `extern "C"` 函数（均为 `unsafe fn`，调用方必须 `unsafe { ... }`）：
//! - `doc_engine_version() -> *const c_char`    返回版本字符串
//! - `doc_engine_last_error() -> *const c_char` 取出最后一次错误
//! - `doc_engine_convert_zip(...) -> i32`      zip → docx，0 成功 / 1 失败
//! - `doc_engine_free(ptr)`                      释放 Rust 分配的内存
//!
//! SAFETY：所有函数均为 FFI 边界，需由 dart:ffi 调用方保证：
//! - 传入的有效指针必须非空且指向正确长度的合法内存
//! - 传入的 usize 参数必须为准确长度
//! - 调用方在不再需要 docx bytes 后必须调用 `doc_engine_free`

use std::cell::RefCell;
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::slice;
use std::str;

#[cfg(test)]
use std::ffi::CStr;

use doc_core::{convert_zip, ConvertOptions};

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_last_error(msg: &str) {
    let c = CString::new(msg).unwrap_or_else(|_| CString::new("<invalid utf-8>").unwrap());
    LAST_ERROR.with(|cell| *cell.borrow_mut() = Some(c));
}

/// 返回静态版本字符串，永不为空指针。
///
/// # Safety
/// 返回值永不为空指针。
#[no_mangle]
pub unsafe extern "C" fn doc_engine_version() -> *const c_char {
    static V: std::sync::OnceLock<CString> = std::sync::OnceLock::new();
    V.get_or_init(|| {
        CString::new(format!("doc-native/{}", env!("CARGO_PKG_VERSION")))
            .expect("version string is ASCII")
    })
    .as_ptr()
}

/// 取出最后一次错误。
///
/// # Safety
/// 无错误时返回空指针。调用方须检查返回值。
#[no_mangle]
pub unsafe extern "C" fn doc_engine_last_error() -> *const c_char {
    LAST_ERROR.with(|cell| {
        cell.borrow()
            .as_ref()
            .map(|c| c.as_ptr())
            .unwrap_or(std::ptr::null())
    })
}

/// 释放 Rust 分配的字节。
///
/// # Safety
/// ptr 必须是 `doc_engine_convert_zip` 返回的指针。
#[no_mangle]
pub unsafe extern "C" fn doc_engine_free(ptr: *mut u8) {
    if !ptr.is_null() {
        libc_free(ptr);
    }
}

extern "C" {
    fn malloc(size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
    fn memcpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8;
}

unsafe fn libc_free(p: *mut u8) {
    free(p);
}

/// 主转换函数：zip 字节 → docx 字节。
///
/// # Safety
/// 所有指针参数必须有效且非空。
/// 成功后必须对返回的 docx 和 warnings 字节各调用一次 `doc_engine_free`。
#[no_mangle]
pub unsafe extern "C" fn doc_engine_convert_zip(
    zip_ptr: *const u8,
    zip_len: usize,
    main_tex_ptr: *const u8,
    main_tex_len: usize,
    out_docx_ptr: *mut *mut u8,
    out_docx_len: *mut usize,
    out_warnings_ptr: *mut *mut u8,
    out_warnings_len: *mut usize,
) -> c_int {
    if zip_ptr.is_null()
        || main_tex_ptr.is_null()
        || out_docx_ptr.is_null()
        || out_docx_len.is_null()
        || out_warnings_ptr.is_null()
        || out_warnings_len.is_null()
    {
        set_last_error("空指针入参");
        return 1;
    }

    let zip_bytes = slice::from_raw_parts(zip_ptr, zip_len);
    let main_tex_bytes = slice::from_raw_parts(main_tex_ptr, main_tex_len);
    let main_tex = match str::from_utf8(main_tex_bytes) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&format!("主文件路径非 UTF-8：{e}"));
            return 1;
        }
    };

    let options = ConvertOptions::default();
    let result = match convert_zip(zip_bytes, main_tex, &options) {
        Ok(r) => r,
        Err(e) => {
            set_last_error(&format!("convert_zip 失败：{e}"));
            return 1;
        }
    };

    // 分配 docx 字节
    let docx_len = result.docx.len();
    let docx_buf = unsafe { malloc(docx_len) };
    if docx_buf.is_null() {
        set_last_error("docx malloc 失败");
        return 1;
    }
    unsafe { memcpy(docx_buf, result.docx.as_ptr(), docx_len) };
    *out_docx_ptr = docx_buf;
    *out_docx_len = docx_len;

    // 分配 warnings JSON
    let warnings_json = serde_json::to_vec(&result.warnings).unwrap_or_else(|_| b"[]".to_vec());
    let w_len = warnings_json.len();
    let w_buf = unsafe { malloc(w_len) };
    if w_buf.is_null() {
        unsafe { libc_free(docx_buf) };
        *out_docx_ptr = std::ptr::null_mut();
        *out_docx_len = 0;
        set_last_error("warnings malloc 失败");
        return 1;
    }
    unsafe { memcpy(w_buf, warnings_json.as_ptr(), w_len) };
    *out_warnings_ptr = w_buf;
    *out_warnings_len = w_len;
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_nonempty() {
        let p = unsafe { doc_engine_version() };
        let s = unsafe { CStr::from_ptr(p) }.to_str().unwrap();
        assert!(s.starts_with("doc-native/"));
    }

    #[test]
    fn free_null_is_safe() {
        unsafe { doc_engine_free(std::ptr::null_mut()) };
    }
}
