//! `TexFacade` 顶层门面。
//!
//! 设计见 `docs/study/08-pdf-pipeline/02-tex-facade.md` §2.4.3。
//!
//! 职责：
//! 1. 探测本机可用引擎（xelatex / tectonic / latexmk）；
//! 2. 信号量限流（默认并发 2，§2.8）；
//! 3. 缓存读取 / 写入（命中则跳过 compile）；
//! 4. 失败统一返回 `Result`，**不 panic**。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::Semaphore;

use crate::backend::{EngineKind, TexBackend, TexProject};
use crate::cache::{self, Cache};
use crate::error::TexError;
use crate::latexmk::LatexmkBackend;
use crate::tectonic::TectonicBackend;
use crate::xelatex::XelatexBackend;

/// `doc-tex-facade` 的顶层入口。
pub struct TexFacade {
    /// 可用引擎列表（按探测顺序：xelatex → tectonic → latexmk）。
    backends: Vec<Arc<dyn TexBackend>>,
    /// 缓存对象。
    cache: Cache,
    /// 信号量。
    sem: Semaphore,
}

impl TexFacade {
    /// 默认构造：探测 xelatex → tectonic → latexmk，**至少一个能找到**。
    ///
    /// 找不到任何引擎 → 返回 `Err(TexError::NoEngine)`，不 panic。
    pub async fn probe(project_for_cache: &TexProject) -> Result<Self> {
        let mut backends: Vec<Arc<dyn TexBackend>> = Vec::new();
        if let Some(b) = XelatexBackend::probe() {
            if b.is_available().await {
                backends.push(Arc::new(b));
            }
        }
        if let Some(b) = TectonicBackend::probe() {
            if b.is_available().await {
                backends.push(Arc::new(b));
            }
        }
        if let Some(b) = LatexmkBackend::probe() {
            if b.is_available().await {
                backends.push(Arc::new(b));
            }
        }
        if backends.is_empty() {
            return Err(TexError::NoEngine.into());
        }
        Ok(Self {
            backends,
            cache: Cache::for_workdir(&project_for_cache.workdir),
            sem: Semaphore::new(Self::default_concurrency()),
        })
    }

    /// 显式指定单个引擎（不探测其它）。
    pub fn with_backend(b: Arc<dyn TexBackend>, project_for_cache: &TexProject) -> Self {
        Self {
            backends: vec![b],
            cache: Cache::for_workdir(&project_for_cache.workdir),
            sem: Semaphore::new(Self::default_concurrency()),
        }
    }

    /// 链式设置并发上限（默认 2）。
    pub fn with_concurrency(mut self, n: usize) -> Self {
        self.sem = Semaphore::new(n.max(1));
        self
    }

    /// 显式指定缓存根目录。
    pub fn with_cache_root(mut self, root: PathBuf) -> Self {
        self.cache = Cache::at(root);
        self
    }

    /// 单次编译硬超时（5 分钟）。超过即返回 `TexError::CompileFailed`。
    /// 防止卡死的 xelatex 把 CI runner 拖死。
    const COMPILE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

    /// 编译并返回 PDF 路径（**命中缓存**则秒返）。
    pub async fn compile_to_pdf(&self, project: &TexProject) -> Result<PathBuf> {
        // 1. 选引擎
        let backend = self.pick_backend(project)?;

        // 2. 算缓存键 + 查缓存
        let key = cache::compute_key(project).context("计算缓存键失败")?;
        if let Some(pdf) = self.cache.lookup(backend.kind(), key) {
            tracing::info!(
                key = %key.hex(),
                engine = backend.kind().as_str(),
                "tex-facade cache hit"
            );
            return Ok(pdf);
        }

        // 3. 信号量限流
        let _permit = self
            .sem
            .acquire()
            .await
            .context("信号量获取失败")?;

        // 4. 实际编译（带硬超时）
        let compile = backend.compile(project);
        let run = match tokio::time::timeout(Self::COMPILE_TIMEOUT, compile).await {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                return Err(e.context(format!("{} 编译失败", backend.kind().as_str())));
            }
            Err(_) => {
                return Err(TexError::CompileFailed {
                    engine: backend.kind(),
                    passes: 0,
                    output: PathBuf::from("(timeout)"),
                    log: format!("编译超过 {}s 仍未完成，已强制放弃", Self::COMPILE_TIMEOUT.as_secs()),
                }
                .into());
            }
        };

        // 5. 写缓存（失败不阻断，仅 warning）
        match self
            .cache
            .store(run.engine, key, &run.pdf_path, &run.log)
            .await
        {
            Ok(cached) => Ok(cached),
            Err(e) => {
                tracing::warn!(error = %e, "tex-facade 写缓存失败，返回原始 PDF");
                Ok(run.pdf_path)
            }
        }
    }

    /// 抽取 PDF 文本（`bin/doc-engine verify` 阶段用）。
    pub async fn extract_text(&self, pdf: &Path) -> Result<String> {
        crate::extract::extract_text(pdf).await
    }

    /// 探测本机可用引擎清单。
    pub fn available_engines(&self) -> Vec<EngineKind> {
        self.backends.iter().map(|b| b.kind()).collect()
    }

    /// 选引擎：优先 `project.preferred`，否则取 `backends` 第一个。
    fn pick_backend(&self, project: &TexProject) -> Result<Arc<dyn TexBackend>> {
        if let Some(pref) = project.preferred {
            if let Some(b) = self.backends.iter().find(|b| b.kind() == pref) {
                return Ok(b.clone());
            }
            return Err(TexError::EngineUnavailable(pref).into());
        }
        self.backends
            .first()
            .cloned()
            .ok_or_else(|| TexError::NoEngine.into())
    }

    /// 默认并发上限：2（见 §2.8）。
    pub fn default_concurrency() -> usize {
        2
    }
}
