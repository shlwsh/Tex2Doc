//! 质量层错误类型。

use thiserror::Error;

#[derive(Debug, Error)]
pub enum QualityError {
    #[error("docx 解析失败：{0}")]
    Docx(String),

    #[error("PDF 解析失败：{0}")]
    Pdf(String),

    #[error("PDF 文本提取失败：{0}")]
    PdfText(String),

    #[error("PDF 元数据缺失：{0}")]
    PdfMeta(String),

    #[error("PDF 渲染失败：{0}")]
    PdfRender(String),

    #[error("图像处理失败：{0}")]
    Image(String),

    #[error("OCR 失败：{0}")]
    Ocr(String),

    #[error("I/O 错误：{0}")]
    Io(#[from] std::io::Error),

    #[error("XML 解析错误：{0}")]
    Xml(String),
}

pub type Result<T> = std::result::Result<T, QualityError>;
