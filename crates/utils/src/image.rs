//! 图片解码 / 探测 / 重压缩
//!
//! V1 策略：
//! - 通过 [`image::guess_format`] 探测格式。
//! - 仅支持 PNG / JPEG；其它格式返回 `Unsupported`。
//! - 重采样：V1 不做（占位接口），仅做格式归一化与字节透传 + 重新编码（保证路径与 zip 兼容）。

use std::collections::HashMap;
use std::io::Cursor;

use image::ImageFormat;

use crate::error::{DocError, DocResult};

/// 支持的图片格式（V1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedFormat {
    Png,
    Jpeg,
}

impl SupportedFormat {
    /// 探测支持的格式；不支持时返回 `Unsupported`。
    pub fn probe(bytes: &[u8]) -> DocResult<Self> {
        let fmt = image::guess_format(bytes)
            .map_err(|e| DocError::ImageDecode(format!("guess_format: {e}")))?;
        Ok(match fmt {
            ImageFormat::Png => Self::Png,
            ImageFormat::Jpeg => Self::Jpeg,
            other => {
                return Err(DocError::Unsupported(format!(
                    "图片格式 {other:?} 在 V1 不支持"
                )));
            }
        })
    }

    /// 转回 `image::ImageFormat`。
    pub fn to_image_format(self) -> ImageFormat {
        match self {
            Self::Png => ImageFormat::Png,
            Self::Jpeg => ImageFormat::Jpeg,
        }
    }
}

/// 图片元信息（V1 最小集）。
#[derive(Debug, Clone, Copy)]
pub struct ImageMeta {
    pub width: u32,
    pub height: u32,
    pub format: SupportedFormat,
}

/// 读取图片头部元信息（不解码像素）。
pub fn read_meta(bytes: &[u8]) -> DocResult<ImageMeta> {
    let format = SupportedFormat::probe(bytes)?;
    let reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| DocError::ImageDecode(e.to_string()))?;
    let (w, h) = reader
        .into_dimensions()
        .map_err(|e| DocError::ImageDecode(e.to_string()))?;
    Ok(ImageMeta {
        width: w,
        height: h,
        format,
    })
}

/// 将图片以原格式重新编码（保证 ZIP 内的 `word/media/*` 字节合法）。
pub fn renormalize(bytes: &[u8]) -> DocResult<(SupportedFormat, Vec<u8>)> {
    let meta = read_meta(bytes)?;
    let img = image::load_from_memory(bytes).map_err(|e| DocError::ImageDecode(e.to_string()))?;
    let mut out = Vec::new();
    let fmt = meta.format.to_image_format();
    img.write_to(&mut Cursor::new(&mut out), fmt)
        .map_err(|e| DocError::ImageDecode(e.to_string()))?;
    Ok((meta.format, out))
}

/// 内存图片资产集合（路径 → 原始字节），由 VFS 填充后传入 docx-writer。
///
/// 键为 VFS 中的规范路径（如 `figs/a.png`），不含 `word/media/` 前缀。
#[derive(Debug, Default, Clone)]
pub struct ImageAssets {
    inner: HashMap<String, Vec<u8>>,
}

impl ImageAssets {
    /// 创建空资产集合。
    pub fn new() -> Self {
        Self::default()
    }

    /// 插入一组图片（由 VFS 扫描调用方批量填充）。
    pub fn insert(&mut self, path: String, bytes: Vec<u8>) {
        self.inner.insert(path, bytes);
    }

    /// 按路径查找图片字节；命中时返回 `Some(&[u8])`。
    pub fn get(&self, path: &str) -> Option<&[u8]> {
        self.inner.get(path).map(Vec::as_slice)
    }

    /// 返回所有已注册图片的（路径，字节）迭代器。
    pub fn iter(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.inner.iter().map(|(k, v)| (k.as_str(), v.as_slice()))
    }

    /// 返回图片数量。
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb, RgbImage};

    fn dummy_png() -> Vec<u8> {
        let img: RgbImage = ImageBuffer::from_pixel(2, 2, Rgb([255, 0, 0]));
        let mut out = Vec::new();
        img.write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
            .unwrap();
        out
    }

    #[test]
    fn probe_png() {
        let bytes = dummy_png();
        assert_eq!(
            SupportedFormat::probe(&bytes).unwrap(),
            SupportedFormat::Png
        );
    }

    #[test]
    fn read_meta_size() {
        let bytes = dummy_png();
        let meta = read_meta(&bytes).unwrap();
        assert_eq!((meta.width, meta.height), (2, 2));
    }

    #[test]
    fn renormalize_roundtrip() {
        let bytes = dummy_png();
        let (fmt, out) = renormalize(&bytes).unwrap();
        assert_eq!(fmt, SupportedFormat::Png);
        assert!(!out.is_empty());
        let meta = read_meta(&out).unwrap();
        assert_eq!((meta.width, meta.height), (2, 2));
    }
}
