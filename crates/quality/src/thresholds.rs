//! 阈值集合。
//!
//! 设计见 `docs/study/08-pdf-pipeline/04-quality-comparison.md` §4.9。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralThresholds {
    pub min_tables: u32, // 默认 5
    pub expected_images: u32, // 默认 8
    pub min_captions: u32, // 默认 6
    pub min_char_ratio: f64, // 默认 0.75
    pub expected_page_w: u32, // 默认 10433 twips
    pub expected_page_h: u32, // 默认 14742 twips
    pub max_pdf_size_bytes: u64, // 默认 5 MB
    pub min_embedded_fonts: usize, // 默认 2
}

impl Default for StructuralThresholds {
    fn default() -> Self {
        Self {
            min_tables: 5,
            expected_images: 8,
            min_captions: 6,
            min_char_ratio: 0.75,
            expected_page_w: 10433,
            expected_page_h: 14742,
            max_pdf_size_bytes: 5 * 1024 * 1024,
            min_embedded_fonts: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextualThresholds {
    pub min_char_ratio: f64,             // 默认 0.75
    pub min_marker_coverage: f64,        // 默认 1.0 (22/22)
    pub min_section_coverage: f64,       // 默认 1.0 (7/7)
}

impl Default for TextualThresholds {
    fn default() -> Self {
        Self {
            min_char_ratio: 0.75,
            min_marker_coverage: 1.0,
            min_section_coverage: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualThresholds {
    pub min_ssim: f64,                   // 默认 0.95
    pub max_pixel_diff: u8,              // 默认 3 (0-255)
    pub min_ocr_similarity: f64,         // 默认 0.85 (feature=ocr 时)
    pub dpi: u32,                        // 默认 150
}

impl Default for VisualThresholds {
    fn default() -> Self {
        Self {
            min_ssim: 0.95,
            max_pixel_diff: 3,
            min_ocr_similarity: 0.85,
            dpi: 150,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Thresholds {
    pub structural: StructuralThresholds,
    pub textual: TextualThresholds,
    pub visual: VisualThresholds,
}
