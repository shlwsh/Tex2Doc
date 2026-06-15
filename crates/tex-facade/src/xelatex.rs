//! `XelatexBackend`——CTeX / CJK 首选引擎。
//!
//! 设计见 `docs/study/08-pdf-pipeline/02-tex-facade.md` §2.5.1。
//! 命令序列：`xelatex -interaction=nonstopmode -halt-on-error -output-directory=<workdir> <main>`，
//! 跑 `max_passes` 轮；收敛判定：第二轮 `.aux` 字节级 hash 与第一轮一致 → 提前返回。

use std::path::PathBuf;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::process::Command;

use crate::backend::{EngineKind, TexBackend, TexProject, TexRun};
use crate::error::TexError;

/// `xelatex` 后端。
#[derive(Debug, Clone)]
pub struct XelatexBackend {
    /// 来自 `which::which("xelatex")` 的绝对路径。
    pub bin: PathBuf,
}

impl XelatexBackend {
    /// 探测本机 `xelatex`，找不到返回 `None`。
    pub fn probe() -> Option<Self> {
        which::which("xelatex").ok().map(|bin| Self { bin })
    }
}

#[async_trait]
impl TexBackend for XelatexBackend {
    fn kind(&self) -> EngineKind {
        EngineKind::Xelatex
    }

    async fn is_available(&self) -> bool {
        // 二进制存在 + --version 跑得动（5s 超时——避免 MiKTeX/TeXLive 首次跑卡死探测）
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
        let log_path = project.workdir.join(
            project
                .main_file
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| format!("{s}.log"))
                .unwrap_or_else(|| "main.log".into()),
        );

        // bibtex 步：.bbl 不存在且 .bib 存在才跑；paper3 已有 .bbl，跳过
        let main_stem = project
            .main_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("main");
        let bbl_path = project.workdir.join(format!("{main_stem}.bbl"));
        let bib_path = project.workdir.join(format!("{main_stem}.bib"));
        let need_bibtex = !bbl_path.exists() && bib_path.exists();

        let mut pass = 0u32;

        // 第一轮总是 xelatex
        let first = self
            .run_xelatex(project)
            .await
            .context("xelatex 第一轮失败")?;
        pass += 1;
        if !first {
            let log = read_log_tail(&log_path).await;
            return Err(TexError::CompileFailed {
                engine: EngineKind::Xelatex,
                passes: pass,
                output: pdf_path.clone(),
                log,
            }
            .into());
        }
        let mut prev_aux_hash = Some(aux_hash(project).await);
        let mut aux_modified = true;

        // bibtex 步（仅当 .bbl 缺失）
        if need_bibtex {
            let _ = run_bibtex(&self.bin, project).await;
        }

        // 后续轮：跑 max_passes-1 次
        while pass < project.max_passes {
            let ok = self
                .run_xelatex(project)
                .await
                .context("xelatex 后续轮失败")?;
            pass += 1;
            if !ok {
                let log = read_log_tail(&log_path).await;
                return Err(TexError::CompileFailed {
                    engine: EngineKind::Xelatex,
                    passes: pass,
                    output: pdf_path.clone(),
                    log,
                }
                .into());
            }
            let cur = aux_hash(project).await;
            aux_modified = Some(cur) != prev_aux_hash;
            if !aux_modified {
                break;
            }
            prev_aux_hash = Some(cur);
        }

        if !pdf_path.exists() {
            let log = read_log_tail(&log_path).await;
            return Err(TexError::CompileFailed {
                engine: EngineKind::Xelatex,
                passes: pass,
                output: pdf_path.clone(),
                log,
            }
            .into());
        }

        Ok(TexRun {
            engine: EngineKind::Xelatex,
            main_file: project.main_file.clone(),
            pdf_path,
            log: read_log_tail(&log_path).await,
            aux_modified,
            elapsed_ms: started.elapsed().as_millis() as u64,
        })
    }
}

impl XelatexBackend {
    /// 跑一次 xelatex；返回 `Ok(true)` 表示成功，`Ok(false)` 表示引擎退出非 0 但日志里有内容可读。
    ///
    /// 用 `spawn() + wait()` 而非 `status()`——避免阻塞 tokio executor，
    /// 让上层 `tokio::time::timeout` 能真正起作用。
    async fn run_xelatex(&self, project: &TexProject) -> Result<bool> {
        let mut child = Command::new(&self.bin)
            .args(self.build_args(project))
            .current_dir(&project.workdir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .with_context(|| format!("启动 xelatex 失败：{}", self.bin.display()))?;
        let status = child
            .wait()
            .await
            .with_context(|| format!("等待 xelatex 失败：{}", self.bin.display()))?;
        Ok(status.success())
    }

    /// 构造 xelatex 命令行参数（**仅测试可见**——单元测试 `xelatex_command_construction` 用）。
    pub fn build_args(&self, project: &TexProject) -> Vec<String> {
        vec![
            "-interaction=nonstopmode".to_string(),
            "-halt-on-error".to_string(),
            format!("-output-directory={}", project.workdir.display()),
            project.main_file.display().to_string(),
        ]
    }
}

/// 用 `bibtex`（不 `biber`）跑一次。它通常与 xelatex 同包发布。
async fn run_bibtex(xelatex_bin: &std::path::Path, project: &TexProject) -> Result<bool> {
    // 在 Windows 上 bibtex 与 xelatex 同目录（TeX Live / MiKTeX 都不分）
    let parent = xelatex_bin.parent();
    let bibtex = match parent {
        Some(p) => p.join(if cfg!(windows) { "bibtex.exe" } else { "bibtex" }),
        None => PathBuf::from("bibtex"),
    };
    let main_stem = project
        .main_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");
    let status = Command::new(&bibtex)
        .arg(main_stem)
        .current_dir(&project.workdir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await?;
    Ok(status.success())
}

/// 读主 .log 文件末尾 4KB（不存在返回空串）。
async fn read_log_tail(log_path: &std::path::Path) -> String {
    match tokio::fs::read(log_path).await {
        Ok(bytes) => {
            let tail_len = bytes.len().min(4096);
            String::from_utf8_lossy(&bytes[bytes.len() - tail_len..]).into_owned()
        }
        Err(_) => String::new(),
    }
}

/// 计算 `.aux` 文件的 blake3 哈希（不存在返回 `[0; 32]`）。
async fn aux_hash(project: &TexProject) -> [u8; 32] {
    let main_stem = project
        .main_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");
    let aux = project.workdir.join(format!("{main_stem}.aux"));
    match tokio::fs::read(&aux).await {
        Ok(bytes) => *blake3::hash(&bytes).as_bytes(),
        Err(_) => [0u8; 32],
    }
}
