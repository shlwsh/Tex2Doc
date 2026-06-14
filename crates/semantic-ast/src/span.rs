//! 源码位置（span）
//!
//! - `start` / `end`：字节偏移（拼接流中的绝对位置）。
//! - `source`：来源文件标识（来自 include 拓扑）。

use serde::{Deserialize, Serialize};

/// 源文件标识。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceId(pub u32);

/// 文本区间。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    pub start: u32,
    pub end: u32,
    pub source: SourceId,
}

impl Span {
    /// 构造 span。
    pub const fn new(start: u32, end: u32, source: SourceId) -> Self {
        Self { start, end, source }
    }

    /// 区间长度。
    pub const fn len(&self) -> u32 {
        self.end - self.start
    }

    /// 是否为空区间。
    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }
}
