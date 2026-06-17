//! 视觉层：PDF → PNG → SSIM / 像素差。
//!
//! 设计见 `docs/study/08-pdf-pipeline/04-quality-comparison.md` §4.7。
//!
//> 本期目标：把 PDF 转 PNG 后跑 SSIM / 平均像素差；OCR 在 `feature = "ocr"` 时启用。
//> PDF→PNG 优先用 `pdfium-render`；若库未绑定则降级为「直接标 fail」，不 panic。

use std::path::PathBuf;

use image::{DynamicImage, ImageBuffer, Luma, Rgba};
use pdfium_render::prelude::*;

use crate::context::Context;
use crate::layer::{Check, Layer, LayerResult, Severity};
use crate::thresholds::VisualThresholds;
use crate::QualityError;

#[derive(Default)]
pub struct VisualRunner {
    pub diff_outdir: Option<PathBuf>,
}

impl VisualRunner {
    pub async fn run(
        &self,
        ctx: &Context,
        thr: &VisualThresholds,
    ) -> Result<LayerResult, QualityError> {
        let oracle_pages = render_pdf_pages(&ctx.oracle_pdf, thr.dpi);
        let rust_pages = render_pdf_pages(&ctx.rust_pdf, thr.dpi);

        let (oracle_pages, rust_pages) = match (oracle_pages, rust_pages) {
            (Ok(a), Ok(b)) => (a, b),
            (Err(e), _) | (_, Err(e)) => {
                return Ok(LayerResult::new(
                    Layer::Visual,
                    vec![Check::new(
                        "PDF→PNG 渲染",
                        Severity::Major,
                        "ok".to_string(),
                        format!("failed: {e}"),
                        false,
                    )
                    .with_note("需要 PDFium；CI 上预装 `libpdfium-dev` 或 vendored 二进制。")],
                ));
            }
        };

        let n = oracle_pages.len().min(rust_pages.len());
        let mut checks = Vec::new();
        let mut ssim_pass = 0usize;
        let mut diff_pass = 0usize;

        for i in 0..n {
            let ssim = ssim_score(&oracle_pages[i], &rust_pages[i]);
            let diff = mean_abs_diff(&oracle_pages[i], &rust_pages[i]);
            checks.push(Check::new(
                format!("SSIM page {i:03}"),
                Severity::Major,
                format!(">={:.2}", thr.min_ssim),
                format!("{:.3}", ssim),
                ssim >= thr.min_ssim,
            ));
            checks.push(Check::new(
                format!("像素差 page {i:03}"),
                Severity::Major,
                format!("<={}/255", thr.max_pixel_diff),
                format!("{:.2}/255", diff),
                diff <= thr.max_pixel_diff as f64,
            ));
            if ssim >= thr.min_ssim {
                ssim_pass += 1;
            }
            if diff <= thr.max_pixel_diff as f64 {
                diff_pass += 1;
            }

            if let Some(outdir) = &self.diff_outdir {
                let _ = std::fs::create_dir_all(outdir);
                let _ = make_diff_png(
                    &oracle_pages[i],
                    &rust_pages[i],
                    &outdir.join(format!("page-{i:03}.png")),
                );
            }
        }

        checks.push(Check::new(
            "SSIM 通过率",
            Severity::Major,
            format!("{}/{} pages", n, n),
            format!("{}/{}", ssim_pass, n),
            ssim_pass == n,
        ));
        checks.push(Check::new(
            "像素差 通过率",
            Severity::Major,
            format!("{}/{} pages", n, n),
            format!("{}/{}", diff_pass, n),
            diff_pass == n,
        ));

        Ok(LayerResult::new(Layer::Visual, checks))
    }
}

/// PDF → Vec<DynamicImage>，每页一张。
fn render_pdf_pages(pdf: &std::path::Path, dpi: u32) -> Result<Vec<DynamicImage>, QualityError> {
    let pdfium = match Pdfium::bind_to_system_library() {
        Ok(b) => Pdfium::new(b),
        Err(_) => {
            return Err(QualityError::PdfRender(
                "找不到系统 PDFium；请安装 libpdfium-dev 或 vendored".into(),
            ))
        }
    };
    let doc = pdfium
        .load_pdf_from_file(pdf, None)
        .map_err(|e| QualityError::PdfRender(format!("打开 PDF 失败：{e}")))?;
    let mut pages = Vec::new();
    let target_w = ((210.0 / 25.4) * dpi as f32) as i32; // A4 宽
    for (_i, page) in doc.pages().iter().enumerate() {
        let bitmap = page
            .render_with_config(&PdfRenderConfig::new().set_target_width(target_w))
            .map_err(|e| QualityError::PdfRender(format!("渲染第 {_i} 页失败：{e}")))?;
        pages.push(bitmap.as_image());
    }
    Ok(pages)
}

/// 灰度化 + 256×256 resize + 简化 SSIM。
pub fn ssim_score(a: &DynamicImage, b: &DynamicImage) -> f64 {
    let a_gray = a.to_luma8();
    let b_gray = b.to_luma8();
    let target = 256u32;
    let a_small = image::imageops::resize(
        &a_gray,
        target,
        target,
        image::imageops::FilterType::Lanczos3,
    );
    let b_small = image::imageops::resize(
        &b_gray,
        target,
        target,
        image::imageops::FilterType::Lanczos3,
    );
    ssim_gray(&a_small, &b_small)
}

/// 全局均值 / 方差 / 协方差形式的 SSIM（11x11 高斯权重近邻 1x1 全局平均）。
fn ssim_gray(a: &ImageBuffer<Luma<u8>, Vec<u8>>, b: &ImageBuffer<Luma<u8>, Vec<u8>>) -> f64 {
    let n = (a.width() * a.height()) as f64;
    if n == 0.0 {
        return 0.0;
    }
    let ma: f64 = a.pixels().map(|p| p.0[0] as f64).sum::<f64>() / n;
    let mb: f64 = b.pixels().map(|p| p.0[0] as f64).sum::<f64>() / n;
    let va: f64 = a
        .pixels()
        .map(|p| (p.0[0] as f64 - ma).powi(2))
        .sum::<f64>()
        / n;
    let vb: f64 = b
        .pixels()
        .map(|p| (p.0[0] as f64 - mb).powi(2))
        .sum::<f64>()
        / n;
    let cov: f64 = a
        .pixels()
        .zip(b.pixels())
        .map(|(p, q)| (p.0[0] as f64 - ma) * (q.0[0] as f64 - mb))
        .sum::<f64>()
        / n;
    let c1: f64 = (0.01f64 * 255.0).powi(2);
    let c2: f64 = (0.03f64 * 255.0).powi(2);
    let num = (2.0 * ma * mb + c1) * (2.0 * cov + c2);
    let den = (ma.powi(2) + mb.powi(2) + c1) * (va + vb + c2);
    if den == 0.0 {
        1.0
    } else {
        (num / den).clamp(0.0, 1.0)
    }
}

/// 平均绝对像素差（0-255）。
pub fn mean_abs_diff(a: &DynamicImage, b: &DynamicImage) -> f64 {
    let aa = a.to_luma8();
    let bb = b.to_luma8();
    if aa.dimensions() != bb.dimensions() {
        return 255.0;
    }
    let total: u64 = aa
        .pixels()
        .zip(bb.pixels())
        .map(|(p, q)| (p.0[0] as i32 - q.0[0] as i32).unsigned_abs() as u64)
        .sum();
    (total as f64) / (aa.pixels().count() as f64)
}

/// 差异热图：差异 > 20 红色，> 5 黄色，其它原图。
pub fn make_diff_png(
    a: &DynamicImage,
    b: &DynamicImage,
    out: &std::path::Path,
) -> Result<(), QualityError> {
    let aa = a.to_luma8();
    let bb = b.to_luma8();
    if aa.dimensions() != bb.dimensions() {
        return Err(QualityError::Image("尺寸不一致，无法 diff".into()));
    }
    let (w, h) = aa.dimensions();
    let mut buf = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(w, h);
    for (x, y, p) in aa.enumerate_pixels() {
        let q = bb.get_pixel(x, y);
        let d = (p.0[0] as i32 - q.0[0] as i32).unsigned_abs();
        let rgba = if d > 20 {
            Rgba([255, 0, 0, 200])
        } else if d > 5 {
            Rgba([255, 200, 0, 100])
        } else {
            Rgba([p.0[0], p.0[0], p.0[0], 255])
        };
        buf.put_pixel(x, y, rgba);
    }
    buf.save(out)
        .map_err(|e| QualityError::Image(format!("save diff png 失败：{e}")))?;
    Ok(())
}
