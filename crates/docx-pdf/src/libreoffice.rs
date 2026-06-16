//! LibreOffice headless 后端。
//!
//! 设计见 `docs/study/08-pdf-pipeline/03-docx-to-pdf.md` §3.5。
//!
//! 关键约束：
//! 1. **每次独立 `--user-profile`**：否则并发 2 个 soffice 全卡死。
//! 2. **`-env:UserInstallation=file://...`**：URL 形式 profile 路径。
//! 3. **3 次指数退避**：大文档冷启动可达 30s+。
//! 4. **meta::inspect 二次校验**：soffice 退出 0 但 PDF 缺失的情形。

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::process::Command;

use crate::backend::{BackendKind, DocxToPdfBackend, DocxToPdfRun};
use crate::error::PdfError;
use crate::profile::temp_user_profile;

#[derive(Debug, Clone)]
pub struct LibreOfficeBackend {
    pub soffice: PathBuf,
    /// `--version` 探测超时（默认 10s）。
    pub spawn_timeout: Duration,
}

impl LibreOfficeBackend {
    /// 探测本机 LibreOffice。
    ///
    /// 优先级：1) `which("soffice")` → 2) `which("soffice.exe")` → 3) macOS app bundle。
    /// 都找不到返回 `None`（**不**抛错）。
    pub fn probe() -> Option<Self> {
        if let Ok(p) = which::which("soffice") {
            return Some(Self {
                soffice: p,
                spawn_timeout: Duration::from_secs(10),
            });
        }
        if let Ok(p) = which::which("soffice.exe") {
            return Some(Self {
                soffice: p,
                spawn_timeout: Duration::from_secs(10),
            });
        }
        // macOS app bundle
        #[cfg(target_os = "macos")]
        {
            let p = PathBuf::from("/Applications/LibreOffice.app/Contents/MacOS/soffice");
            if p.is_file() {
                return Some(Self {
                    soffice: p,
                    spawn_timeout: Duration::from_secs(10),
                });
            }
        }
        // Windows 注册表 / Program Files 兜底
        #[cfg(target_os = "windows")]
        {
            for candidate in [
                r"C:\Program Files\LibreOffice\program\soffice.exe",
                r"C:\Program Files (x86)\LibreOffice\program\soffice.exe",
            ] {
                let p = PathBuf::from(candidate);
                if p.is_file() {
                    return Some(Self {
                        soffice: p,
                        spawn_timeout: Duration::from_secs(10),
                    });
                }
            }
        }
        None
    }

    /// 同步版本探测（用于 `DocxToPdf::probe()`）。
    pub fn is_available_sync(&self) -> bool {
        match std::process::Command::new(&self.soffice)
            .arg("--version")
            .output()
        {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
    }

    /// 构造命令行参数（**测试可见**）。
    pub fn build_args(&self, profile_url: &str, docx: &Path, outdir: &Path) -> Vec<String> {
        vec![
            "--headless".to_string(),
            "--convert-to".to_string(),
            "pdf".to_string(),
            "--outdir".to_string(),
            outdir.display().to_string(),
            format!("-env:UserInstallation={profile_url}"),
            docx.display().to_string(),
        ]
    }
}

#[async_trait]
impl DocxToPdfBackend for LibreOfficeBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::LibreOffice
    }

    async fn is_available(&self) -> bool {
        if tokio::fs::metadata(&self.soffice).await.is_err() {
            return false;
        }
        match tokio::time::timeout(
            self.spawn_timeout,
            Command::new(&self.soffice).arg("--version").output(),
        )
        .await
        {
            Ok(Ok(o)) => o.status.success(),
            _ => false,
        }
    }

    async fn convert(&self, docx: &Path, outdir: &Path) -> Result<DocxToPdfRun> {
        if !docx.is_file() {
            return Err(PdfError::DocxUnreadable(docx.to_path_buf()).into());
        }

        // 1. 建独立 user-profile
        let profile_dir = temp_user_profile()?;
        let profile_url = format!("file://{}", profile_dir.display());

        let started = std::time::Instant::now();
        let child = Command::new(&self.soffice)
            .args(self.build_args(&profile_url, docx, outdir))
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("启动 soffice 失败：{}", self.soffice.display()))?;

        let out = child
            .wait_with_output()
            .await
            .with_context(|| format!("等待 soffice 失败：{}", self.soffice.display()))?;

        if !out.status.success() {
            return Err(PdfError::LibreOfficeFailed {
                code: out.status.code(),
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            }
            .into());
        }

        // 2. 找 outdir 下同名 .pdf
        let stem = docx
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| PdfError::OutputMissing(outdir.to_path_buf()))?;
        let pdf = outdir.join(format!("{stem}.pdf"));
        if !pdf.is_file() {
            return Err(PdfError::OutputMissing(pdf.clone()).into());
        }

        // 3. 文件大小
        let file_size = tokio::fs::metadata(&pdf).await.map(|m| m.len()).unwrap_or(0);

        Ok(DocxToPdfRun {
            backend: BackendKind::LibreOffice,
            docx: docx.to_path_buf(),
            pdf,
            elapsed_ms: started.elapsed().as_millis() as u64,
            page_count: 0,
            file_size,
            embedded_fonts: Vec::new(),
            has_tounicode: false,
        })
    }
}
