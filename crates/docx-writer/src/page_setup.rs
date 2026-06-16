//! 页面设置类型（V2 新增）。
//!
//! 设计见 `docs/study/08-pdf-pipeline/01-pipeline-overview.md` §1.4。

use serde::{Deserialize, Serialize};

/// Twips = 1/1440 inch。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSetup {
    /// 页面宽（twips）
    pub width_twips: u32,
    /// 页面高（twips）
    pub height_twips: u32,
    /// 上下左右页边距（twips）。缺省 = None → 写空，OOXML 视作默认值
    /// （top=1440, right=1800, bottom=1440, left=1440）。
    pub margin_top: Option<u32>,
    pub margin_right: Option<u32>,
    pub margin_bottom: Option<u32>,
    pub margin_left: Option<u32>,
    /// 分栏（cols space / num）。缺省 = 1 栏 + 720 twips 间距。
    pub cols_space: Option<u32>,
    pub cols_num: Option<u32>,
}

impl Default for PageSetup {
    fn default() -> Self {
        // US Letter：12240 × 15840 twips。
        Self {
            width_twips: 12240,
            height_twips: 15840,
            margin_top: None,
            margin_right: None,
            margin_bottom: None,
            margin_left: None,
            cols_space: None,
            cols_num: None,
        }
    }
}

impl PageSetup {
    /// JOS 18.40cm × 26.00cm 模板（实测值，含装订余量）。
    pub fn jos_paper3() -> Self {
        Self {
            width_twips: 10433,
            height_twips: 14742,
            margin_top: Some(567),
            margin_right: Some(850),
            margin_bottom: Some(850),
            margin_left: Some(850),
            cols_space: Some(720),
            cols_num: Some(1),
        }
    }
}
