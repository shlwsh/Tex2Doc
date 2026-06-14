//! 转换结果 / 进度事件

use serde::{Deserialize, Serialize};

/// 转换阶段（用于前端进度总线）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgressPhase {
    Reading,
    Parsing,
    Lowering,
    Serializing,
    Packing,
    Done,
}

/// 进度事件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEvent {
    pub phase: ProgressPhase,
    pub ratio: f32,
    pub message: String,
}

/// 转换结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertResult {
    pub docx: Vec<u8>,
    pub warnings: Vec<String>,
}
