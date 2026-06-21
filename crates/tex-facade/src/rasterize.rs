//! TikZ graphics rasterization pipeline.
//!
//! Converts `\begin{tikzpicture}...\end{tikzpicture}` LaTeX environments
//! into PNG images via tectonic → PDF → image conversion.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{TexError, TexResult};

/// Rasterize a TikZ environment body to PNG.
///
/// Wraps the TikZ source in a minimal `standalone` document, compiles it
/// with tectonic, then converts the first PDF page to PNG.
pub fn rasterize_tikz_to_png(
    tikz_body: &str,
    work_dir: &Path,
) -> TexResult<PathBuf> {
    let wrapper = build_tikz_wrapper(tikz_body);
    let tex_path = work_dir.join("__docx_tikz_temp.tex");
    std::fs::write(&tex_path, &wrapper)
        .map_err(|e| TexError::Io(e))?;
    let pdf_path = compile_tikz_tex(&tex_path, work_dir)?;
    let png_path = pdf_to_png(&pdf_path, work_dir)?;
    Ok(png_path)
}

/// Compile a TikZ TeX file to PDF using tectonic.
fn compile_tikz_tex(tex_path: &Path, work_dir: &Path) -> TexResult<PathBuf> {
    let tectonic = match which::which("tectonic") {
        Ok(p) => p,
        Err(_) => return Err(TexError::NoEngine),
    };
    let pdf_path = work_dir.join(
        tex_path.file_stem().and_then(|s| s.to_str()).unwrap_or("output")
    ).with_extension("pdf");

    let output = Command::new(&tectonic)
        .args([
            "--outdir",
            work_dir.to_str().unwrap(),
            "--keep-logs",
            tex_path.to_str().unwrap(),
        ])
        .current_dir(work_dir)
        .output()
        .map_err(|e| TexError::Io(e))?;

    if !output.status.success() || !pdf_path.exists() {
        let log_path = work_dir.join(
            tex_path.file_stem().and_then(|s| s.to_str()).unwrap_or("main")
        ).with_extension("log");
        let log = std::fs::read_to_string(&log_path)
            .map(|l| {
                let tail_len = l.len().min(4096);
                l[l.len().saturating_sub(tail_len)..].to_string()
            })
            .unwrap_or_default();
        return Err(TexError::CompileFailed {
            engine: crate::backend::EngineKind::Tectonic,
            passes: 1,
            output: pdf_path,
            log,
        });
    }

    Ok(pdf_path)
}

/// Build a minimal LaTeX document wrapping TikZ source.
fn build_tikz_wrapper(tikz_body: &str) -> String {
    format!(
        r#"\documentclass[border=1pt]{{standalone}}
\usepackage{{tikz}}
\usetikzlibrary{{shapes,arrows,positioning,calc}}
\begin{{document}}
\begin{{tikzpicture}}
{}
\end{{tikzpicture}}
\end{{document}}"#,
        tikz_body
    )
}

/// Convert first page of a PDF to PNG using `mutool`, ImageMagick `convert`, or Ghostscript.
fn pdf_to_png(pdf_path: &Path, out_dir: &Path) -> TexResult<PathBuf> {
    let png_path = out_dir.join("__docx_tikz_temp.png");

    // Try mutool first (fastest, comes with mupdf)
    if which::which("mutool").is_ok() {
        let output = Command::new("mutool")
            .args([
                "convert",
                "-o", png_path.to_str().unwrap(),
                "-F", "png",
                "-r", "150",
                pdf_path.to_str().unwrap(),
            ])
            .output();
        if let Ok(out) = output {
            if out.status.success() && png_path.exists() {
                return Ok(png_path);
            }
        }
    }

    // Fallback: ImageMagick `convert`
    if which::which("convert").is_ok() {
        let output = Command::new("convert")
            .args([
                "-density", "150",
                "-quality", "90",
                pdf_path.to_str().unwrap(),
                "-resize", "800",
                png_path.to_str().unwrap(),
            ])
            .output();
        if let Ok(out) = output {
            if out.status.success() && png_path.exists() {
                return Ok(png_path);
            }
        }
    }

    // Fallback: Ghostscript
    if which::which("gs").is_ok() {
        let out_file = format!("-sOutputFile={}", png_path.to_str().unwrap());
        let output = Command::new("gs")
            .args([
                "-dNOPAUSE",
                "-dBATCH",
                "-sDEVICE=png16m",
                "-r150",
                "-dTextAlphaBits=4",
                &out_file,
                pdf_path.to_str().unwrap(),
            ])
            .output();
        if let Ok(out) = output {
            if out.status.success() && png_path.exists() {
                return Ok(png_path);
            }
        }
    }

    Err(TexError::NoTextExtractor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn tikz_wrapper_document_structure() {
        let body = r"\node at (0,0) {Hello};";
        let wrapper = build_tikz_wrapper(body);
        assert!(wrapper.contains(r"\documentclass"));
        assert!(wrapper.contains(r"\usepackage{tikz}"));
        assert!(wrapper.contains(r"\begin{document}"));
        assert!(wrapper.contains(r"\begin{tikzpicture}"));
        assert!(wrapper.contains(body));
        assert!(wrapper.contains(r"\end{tikzpicture}"));
        assert!(wrapper.contains(r"\end{document}"));
    }

    #[test]
    fn pdf_to_png_detects_converters() {
        let has_mutool = which::which("mutool").is_ok();
        let has_convert = which::which("convert").is_ok();
        let has_gs = which::which("gs").is_ok();
        assert!(
            has_mutool || has_convert || has_gs,
            "At least one PDF-to-PNG converter (mutool/convert/gs) must be available"
        );
    }

    #[test]
    fn rasterize_tikz_graceful_failure() {
        // Test that the function handles missing tectonic gracefully
        if which::which("tectonic").is_err() {
            let work_dir = temp_dir().join("tex2doc-tikz-test2");
            std::fs::create_dir_all(&work_dir).ok();
            let result = rasterize_tikz_to_png(r"\node {X};", &work_dir);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, TexError::NoEngine));
        }
    }
}
