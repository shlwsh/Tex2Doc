//! `docx-pdf` 顶层门面 + 后端 trait。
//!
//! 设计见 `docs/study/08-pdf-pipeline/03-docx-to-pdf.md` §3.4。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;

/// 后端实现种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendKind {
    /// 本机 LibreOffice/soffice（默认）。
    LibreOffice,
    /// 远程 PDF 转换 API（占位，M3 末补）。
    Api,
    /// Windows Word COM（占位，仅 feature gate）。
    WordCom,
}

impl BackendKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BackendKind::LibreOffice => "libreoffice",
            BackendKind::Api => "api",
            BackendKind::WordCom => "word-com",
        }
    }
}

/// 一次 docx→pdf 转换的结果。
#[derive(Debug, Clone)]
pub struct DocxToPdfRun {
    pub backend: BackendKind,
    pub docx: PathBuf,
    pub pdf: PathBuf,
    pub elapsed_ms: u64,
    pub page_count: u32,
    pub file_size: u64,
    pub embedded_fonts: Vec<String>,
    pub has_tounicode: bool,
}

/// docx→pdf 后端 trait。
///
/// 默认实现 [`crate::libreoffice::LibreOfficeBackend`]；调用方可通过
/// [`DocxToPdf::with_backend`] 注入自定义后端（如远程 API）。
#[async_trait]
pub trait DocxToPdfBackend: Send + Sync {
    fn kind(&self) -> BackendKind;
    fn name(&self) -> &'static str {
        self.kind().as_str()
    }
    /// 返回 true 表示本机已安装且能用。
    async fn is_available(&self) -> bool;
    /// 跑一次 docx → pdf；产出 PDF 写在 `outdir` 里，文件名与 `docx` 同 stem。
    async fn convert(&self, docx: &Path, outdir: &Path) -> Result<DocxToPdfRun>;
}

/// 顶层门面：持有一组后端，串行尝试（首个可用者用之）。
pub struct DocxToPdf {
    backends: Vec<Arc<dyn DocxToPdfBackend>>,
    config: Config,
    /// 全局串行化：soffice 默认 user profile 不可并发，跑多次会卡。
    sem: tokio::sync::Mutex<()>,
}

/// 转换配置。
#[derive(Debug, Clone)]
pub struct Config {
    /// 单次 soffice 调用超时（默认 120s）。
    pub timeout: Duration,
    /// 失败后重试次数（默认 3，含首次）。
    pub max_retries: u32,
    /// 重试基础延迟（默认 1s；指数退避 = base * 2^attempt）。
    pub retry_base_delay: Duration,
    /// 是否保留临时 user-profile（默认 false；CI 上泄漏会撑爆 runner 磁盘）。
    pub keep_temp: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(120),
            max_retries: 3,
            retry_base_delay: Duration::from_secs(1),
            keep_temp: false,
        }
    }
}

impl DocxToPdf {
    /// 默认构造：探测 LibreOffice。
    ///
    /// 找不到 LibreOffice → 返回 `Err(PdfError::NoBackend)`，不 panic。
    pub fn probe() -> Result<Self> {
        let mut backends: Vec<Arc<dyn DocxToPdfBackend>> = Vec::new();
        #[cfg(feature = "libreoffice")]
        {
            if let Some(lo) = crate::libreoffice::LibreOfficeBackend::probe() {
                if lo.is_available_sync() {
                    backends.push(Arc::new(lo));
                }
            }
        }
        if backends.is_empty() {
            return Err(crate::error::PdfError::NoBackend.into());
        }
        Ok(Self {
            backends,
            config: Config::default(),
            sem: tokio::sync::Mutex::new(()),
        })
    }

    /// 显式指定单个后端。
    pub fn with_backend(b: Arc<dyn DocxToPdfBackend>) -> Self {
        Self {
            backends: vec![b],
            config: Config::default(),
            sem: tokio::sync::Mutex::new(()),
        }
    }

    /// 链式设置 Config。
    pub fn with_config(mut self, c: Config) -> Self {
        self.config = c;
        self
    }

    /// 探测到的后端名（按优先级顺序）。
    pub fn available_backends(&self) -> Vec<BackendKind> {
        self.backends.iter().map(|b| b.kind()).collect()
    }

    /// 同步探测：返回首个能用的后端（如果 `probe()` 失败则返回 `None`）。
    pub fn first_available(&self) -> Option<&Arc<dyn DocxToPdfBackend>> {
        self.backends.first()
    }

    /// docx → pdf 转换。
    ///
    /// 走首个后端；失败时**不**回退到下一个后端（避免 LibreOffice 假成功但 PDF 缺失的陷阱）。
    pub async fn convert(&self, docx: &Path, outdir: &Path) -> Result<DocxToPdfRun> {
        let backend = self
            .first_available()
            .ok_or_else(|| crate::error::PdfError::NoBackend)?;

        // 全局串行：soffice 默认 user profile 不可并发。
        let _permit = self.sem.lock().await;

        let cfg = &self.config;
        let mut last_err: Option<anyhow::Error> = None;
        for attempt in 0..cfg.max_retries {
            if attempt > 0 {
                let delay = cfg.retry_base_delay * (1u32 << (attempt - 1).min(8));
                tracing::warn!(attempt, ?delay, "docx-pdf 重试");
                tokio::time::sleep(delay).await;
            }
            match tokio::time::timeout(cfg.timeout, backend.convert(docx, outdir)).await {
                Ok(Ok(mut run)) => {
                    // 二次校验：meta::inspect 检查页数 / ToUnicode
                    if let Ok(meta) = crate::meta::inspect(&run.pdf) {
                        run.page_count = meta.page_count;
                        run.file_size = meta.file_size;
                        run.embedded_fonts = meta.embedded_fonts;
                        run.has_tounicode = meta.has_tounicode;
                    }
                    return Ok(run);
                }
                Ok(Err(e)) => {
                    tracing::warn!(attempt, error = %e, "docx-pdf 后端返回错误");
                    last_err = Some(e);
                }
                Err(_) => {
                    tracing::warn!(attempt, "docx-pdf 超时");
                    last_err = Some(
                        crate::error::PdfError::Timeout {
                            timeout_secs: cfg.timeout.as_secs(),
                        }
                        .into(),
                    );
                }
            }
        }
        Err(last_err.unwrap_or_else(|| crate::error::PdfError::NoBackend.into()))
    }
}
