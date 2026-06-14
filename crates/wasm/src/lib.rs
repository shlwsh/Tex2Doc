//! Doc-engine WASM bridge.
//!
//! 暴露一个最简 JS 接口 [`convert_zip`]：
//! - 输入：项目 zip 字节流 + 主 .tex 相对路径
//! - 输出：docx 字节流（`Uint8Array`）
//!
//! 所有解析 / 降级 / 打包逻辑委托给 `doc-core::convert_zip`，
//! 本 crate 不持有任何文件系统资源，适合浏览器 / Flutter Web WASM 环境。

#![forbid(unsafe_code)]

use std::fmt;

use serde::Serialize;
use wasm_bindgen::prelude::*;

/// JS 可见的友好错误：把 `CoreError` 平铺到字符串。
#[wasm_bindgen]
#[derive(Debug)]
pub struct WasmError {
    message: String,
}

#[wasm_bindgen]
impl WasmError {
    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.message.clone()
    }
}

impl fmt::Display for WasmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl From<doc_core::CoreError> for WasmError {
    fn from(e: doc_core::CoreError) -> Self {
        Self {
            message: format!("{e}"),
        }
    }
}

impl From<String> for WasmError {
    fn from(s: String) -> Self {
        Self { message: s }
    }
}

/// 转换选项（JS 透传）。除 `bibStyle` 之外的字段都保留为可扩展位。
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ConvertOptionsJs {
    /// `"numeric"`（默认）或 `"author-year"`
    #[serde(default)]
    pub bib_style: Option<String>,
}

impl Default for ConvertOptionsJs {
    fn default() -> Self {
        Self {
            bib_style: Some("numeric".to_string()),
        }
    }
}

/// 转换结果：docx 字节 + 警告条数 + 字节数。
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ConvertResultJs {
    pub docx: Vec<u8>,
    pub docx_len: usize,
    pub warnings: Vec<String>,
}

/// 同步转换入口：把 zip 字节流转换为 docx 字节流。
///
/// - `zip_bytes`: 完整项目 zip 字节（包含主 .tex、include 的 .tex、.bib、图片等）
/// - `main_tex_path`: zip 内主 .tex 的相对 POSIX 路径（例：`main-jos.tex`）
/// - `options_js`: 可选 JSON 字符串（V1 仅消费 `bib_style`）
///
/// 成功返回 `ConvertResultJs { docx, docx_len, warnings }`。
/// 失败抛出 `WasmError`。
#[wasm_bindgen]
pub fn convert_zip(
    zip_bytes: &[u8],
    main_tex_path: &str,
    options_js: Option<String>,
) -> Result<JsValue, JsValue> {
    set_panic_hook();

    let opts: doc_core::ConvertOptions = match options_js.as_deref() {
        None | Some("") => doc_core::ConvertOptions::default(),
        Some(s) => parse_options(s).map_err(to_js_err)?,
    };

    let res = doc_core::convert_zip(zip_bytes, main_tex_path, &opts)
        .map_err(|e| to_js_err(WasmError::from(e)))?;

    let out = ConvertResultJs {
        docx_len: res.docx.len(),
        docx: res.docx,
        warnings: res.warnings,
    };
    serde_wasm_bindgen::to_value(&out).map_err(|e| to_js_err(WasmError {
        message: format!("序列化结果失败：{e}"),
    }))
}

/// 便捷入口：返回 docx 的 `Uint8Array`（不附带元信息）。
///
/// 适合前端只关心文件内容的场景；`download` API 需要 `Uint8Array` / `Blob`。
#[wasm_bindgen]
pub fn convert_zip_to_docx(
    zip_bytes: &[u8],
    main_tex_path: &str,
    options_js: Option<String>,
) -> Result<js_sys::Uint8Array, JsValue> {
    set_panic_hook();

    let opts: doc_core::ConvertOptions = match options_js.as_deref() {
        None | Some("") => doc_core::ConvertOptions::default(),
        Some(s) => parse_options(s).map_err(to_js_err)?,
    };

    let res = doc_core::convert_zip(zip_bytes, main_tex_path, &opts)
        .map_err(|e| to_js_err(WasmError::from(e)))?;
    Ok(js_sys::Uint8Array::from(res.docx.as_slice()))
}

/// 版本号（编译期常量）。
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

fn parse_options(s: &str) -> Result<doc_core::ConvertOptions, WasmError> {
    let js: ConvertOptionsJs = serde_json::from_str(s)
        .map_err(|e| WasmError {
            message: format!("options JSON 解析失败：{e}"),
        })?;
    let mut out = doc_core::ConvertOptions::default();
    match js.bib_style.as_deref() {
        None | Some("numeric") | Some("") => {
            out.bib_style = doc_core::BibStyle::Numeric;
        }
        Some("author-year") | Some("authoryear") => {
            out.bib_style = doc_core::BibStyle::AuthorYear;
        }
        Some(other) => {
            return Err(WasmError {
                message: format!("未知 bib_style：{other}"),
            });
        }
    }
    Ok(out)
}

fn to_js_err(e: WasmError) -> JsValue {
    JsValue::from_str(&e.message)
}
