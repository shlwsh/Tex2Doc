//! OOXML 扁平结构体
//!
//! 真实工程里这些类型会按 ECMA-376 严格生成；M1 阶段仅保留最小字段。

use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
pub struct Paragraph {
    pub style_id: Option<String>,
    pub runs: Vec<Run>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Run {
    pub text: String,
    pub style_id: Option<String>,
    pub bold: bool,
    pub italic: bool,
}
