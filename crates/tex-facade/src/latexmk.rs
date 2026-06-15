//! `LatexmkBackend`——兼容性最广的入口。
//!
//! 设计见 `docs/study/08-pdf-pipeline/02-tex-facade.md` §2.5.3。
//! macOS TeX Live、Linux texlive-latex-extra 通常自带 `latexmk`；用它最稳。

use std::path::PathBuf;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::process::Command;

use crate::backend::{EngineKind, TexBackend, TexProject, TexRun};
use crate::error::TexError;

/// `latexmk` 后端。
#[derive(Debug, Clone)]
pub struct LatexmkBackend {
    /// 来自 `which::which("latexmk")` 的绝对路径。
    pub bin: PathBuf,
}

impl LatexmkBackend {
    /// 探测本机 `latexmk`。
    pub fn probe() -> Option<Self> {
        which::which("latexmk").ok().map(|bin| Self { bin })
    }
}

#[async_trait]
impl TexBackend for LatexmkBackend {
    fn kind(&self) -> EngineKind {
        EngineKind::Latexmk
    }

    async fn is_available(&self) -> bool {
        if tokio::fs::metadata(&self.bin).await.is_err() {
            return false;
        }
        match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            Command::new(&self.bin).arg("--version").output(),
        )
        .await
        {
            Ok(Ok(o)) => o.status.success(),
            _ => false,
        }
    }

    async fn compile(&self, project: &TexProject) -> Result<TexRun> {
        let started = std::time::Instant::now();
        let output_name = project
            .output_name
            .clone()
            .unwrap_or_else(|| {
                project
                    .main_file
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| format!("{s}.pdf"))
                    .unwrap_or_else(|| "output.pdf".into())
            });
        let pdf_path = project.workdir.join(&output_name);

        // latexmk 决定 pass 数；自动 bibtex/biber
        let mut child = Command::new(&self.bin)
            .arg("-xelatex")
            .arg("-interaction=nonstopmode")
            .arg("-halt-on-error")
            .arg("-pdf")
            .arg(&project.main_file)
            .current_dir(&project.workdir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .with_context(|| format!("启动 latexmk 失败：{}", self.bin.display()))?;
        let status = child
            .wait()
            .await
            .with_context(|| format!("等待 latexmk 失败：{}", self.bin.display()))?;

        if !status.success() || !pdf_path.exists() {
            let log_path = project.workdir.join(
                project
                    .main_file
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| format!("{s}.log"))
                    .unwrap_or_else(|| "main.log".into()),
            );
            let log = match tokio::fs::read(&log_path).await {
                Ok(bytes) => {
                    let tail_len = bytes.len().min(4096);
                    String::from_utf8_lossy(&bytes[bytes.len() - tail_len..]).into_owned()
                }
                Err(_) => String::new(),
            };
            return Err(TexError::CompileFailed {
                engine: EngineKind::Latexmk,
                passes: 1,
                output: pdf_path.clone(),
                log,
            }
            .into());
        }

        Ok(TexRun {
            engine: EngineKind::Latexmk,
            main_file: project.main_file.clone(),
            pdf_path,
            log: String::new(),
            aux_modified: false,
            elapsed_ms: started.elapsed().as_millis() as u64,
        })
    }
}
