//! `TectonicBackend`——自动重试 + 自动 bibtex 的"开箱即用"引擎。
//!
//! 设计见 `docs/study/08-pdf-pipeline/02-tex-facade.md` §2.5.2。
//! 优势：自动重试 + 自动 bibtex；劣势：首次会下载包（CI 上需预热镜像，
//! 详见 [05-implementation-roadmap.md §5.7 关键风险 #3](../../docs/study/08-pdf-pipeline/05-implementation-roadmap.md)）。
//!
//! `TECTONIC_OFFLINE=1` 环境变量 → 关网（已下载的 TeXLive 镜像仍可用）。

use std::path::PathBuf;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::process::Command;

use crate::backend::{EngineKind, TexBackend, TexProject, TexRun};
use crate::error::TexError;

/// `tectonic` 后端。
#[derive(Debug, Clone)]
pub struct TectonicBackend {
    /// 来自 `which::which("tectonic")` 的绝对路径。
    pub bin: PathBuf,
    /// 网络是否可达 tectonic 资源 CDN；`TECTONIC_OFFLINE=1` 时为 `false`。
    pub allow_network: bool,
}

impl TectonicBackend {
    /// 探测本机 `tectonic`；`TECTONIC_OFFLINE` 自动透传。
    pub fn probe() -> Option<Self> {
        which::which("tectonic").ok().map(|bin| {
            let allow_network = std::env::var("TECTONIC_OFFLINE")
                .ok()
                .map(|v| v != "1" && !v.eq_ignore_ascii_case("true"))
                .unwrap_or(true);
            Self { bin, allow_network }
        })
    }

    /// 当前 allow_network 状态（**测试可见**）。
    pub fn allow_network(&self) -> bool {
        self.allow_network
    }

    /// 构造 tectonic 命令行参数（**仅测试可见**）。
    pub fn build_args(&self, project: &TexProject) -> Vec<String> {
        vec![
            "--outdir".to_string(),
            project.workdir.display().to_string(),
            "--keep-logs".to_string(),
            "--print".to_string(),
            project.main_file.display().to_string(),
        ]
    }
}

#[async_trait]
impl TexBackend for TectonicBackend {
    fn kind(&self) -> EngineKind {
        EngineKind::Tectonic
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
        let output_name = project.output_name.clone().unwrap_or_else(|| {
            project
                .main_file
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| format!("{s}.pdf"))
                .unwrap_or_else(|| "output.pdf".into())
        });
        let pdf_path = project.workdir.join(&output_name);

        // tectonic 单命令跑完所有 pass + bibtex，无需手动多轮
        let mut cmd = Command::new(&self.bin);
        cmd.args(self.build_args(project))
            .current_dir(&project.workdir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if !self.allow_network {
            cmd.env("TECTONIC_OFFLINE", "1");
        }

        let mut child = Command::new(&self.bin)
            .args(self.build_args(project))
            .current_dir(&project.workdir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .with_context(|| format!("启动 tectonic 失败：{}", self.bin.display()))?;
        let status = child
            .wait()
            .await
            .with_context(|| format!("等待 tectonic 失败：{}", self.bin.display()))?;

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
                engine: EngineKind::Tectonic,
                passes: 1,
                output: pdf_path.clone(),
                log,
            }
            .into());
        }

        Ok(TexRun {
            engine: EngineKind::Tectonic,
            main_file: project.main_file.clone(),
            pdf_path,
            log: String::new(),
            aux_modified: false,
            elapsed_ms: started.elapsed().as_millis() as u64,
        })
    }
}
