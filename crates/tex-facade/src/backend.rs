//! `crates/tex-facade` 的核心类型与 `TexBackend` trait。
//!
//! 见 `docs/study/08-pdf-pipeline/02-tex-facade.md` §2.4。

use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;

/// 一次 TeX 编译任务的输入。
///
/// 由 V2 路径 A 的上层（`bin/doc-engine` 的 `tex-compile` 子命令）构造，
/// 传给 [`TexFacade::compile_to_pdf`](crate::facade::TexFacade::compile_to_pdf)。
#[derive(Debug, Clone)]
pub struct TexProject {
    /// 主入口 .tex 绝对路径，例如 `examples/paper3/latex/main-jos.tex`。
    pub main_file: PathBuf,

    /// 工作目录：含 `main_file`、所有 `\input`/`\include` 子文件、figures/、.bib、.bbl。
    ///
    /// `xelatex` 的 `-output-directory` 也指向这里，编译产物（`.aux` / `.log` / `.pdf`）
    /// 全部就地落盘，方便 `walkdir` 收尾清理。
    pub workdir: PathBuf,

    /// 编译引擎偏好；`None` = 由 [`TexFacade::probe`](crate::facade::TexFacade::probe) 自动探测。
    pub preferred: Option<EngineKind>,

    /// 期望最终 PDF 文件名；`None` = 同 `main_file` 主名 + `.pdf`。
    pub output_name: Option<String>,

    /// 编译最大尝试轮数（含 bibtex 重跑），默认 2（见 §2.5.1 收敛策略）。
    pub max_passes: u32,
}

impl TexProject {
    /// 构造一个仅含 `main_file` 的 `TexProject`，其它字段取默认。
    ///
    /// `workdir` 推断为 `main_file.parent()`——这是 `xelatex -output-directory` 的常用值。
    pub fn from_main(main_file: impl Into<PathBuf>) -> Self {
        let main_file = main_file.into();
        let workdir = main_file.parent().map(PathBuf::from).unwrap_or_default();
        Self {
            main_file,
            workdir,
            preferred: None,
            output_name: None,
            max_passes: 2,
        }
    }

    /// 在 `self` 上链式修改 `preferred`。
    pub fn with_preferred(mut self, kind: EngineKind) -> Self {
        self.preferred = Some(kind);
        self
    }

    /// 在 `self` 上链式修改 `max_passes`。
    pub fn with_max_passes(mut self, n: u32) -> Self {
        self.max_passes = n.max(1);
        self
    }
}

/// 支持的 TeX 引擎枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EngineKind {
    /// `xelatex`——CTeX / CJK 首选（设计稿默认）。
    Xelatex,
    /// `tectonic`——自动重试 + 自动 bibtex；首次需联网。
    Tectonic,
    /// `latexmk`——兼容性最广的入口。
    Latexmk,
}

impl EngineKind {
    /// 用于日志 / 缓存目录的稳定小写名。
    pub fn as_str(self) -> &'static str {
        match self {
            EngineKind::Xelatex => "xelatex",
            EngineKind::Tectonic => "tectonic",
            EngineKind::Latexmk => "latexmk",
        }
    }
}

/// 一次 TeX 编译任务的结果。
///
/// 失败时上层拿不到 `TexRun`——错误走 [`TexError::CompileFailed`](crate::error::TexError::CompileFailed)。
#[derive(Debug, Clone)]
pub struct TexRun {
    /// 实际使用的引擎（探测得到，可能与 `TexProject.preferred` 不同）。
    pub engine: EngineKind,

    /// 主入口 .tex 绝对路径。
    pub main_file: PathBuf,

    /// 编译产出的 PDF 绝对路径（**未命中缓存时**就是 `workdir/output.pdf`；
    /// **命中缓存**时是缓存目录里的 `output.pdf`）。
    pub pdf_path: PathBuf,

    /// 主 .log 文件末尾 4KB，便于排错。
    pub log: String,

    /// 第二轮编译是否动了 `.aux`：`false` = 收敛，可提前结束。
    pub aux_modified: bool,

    /// 实际耗时（毫秒），不含信号量等待。
    pub elapsed_ms: u64,
}

/// TeX 引擎后端的统一接口。
///
/// 三个内置实现：[`XelatexBackend`](crate::xelatex::XelatexBackend) /
/// [`TectonicBackend`](crate::tectonic::TectonicBackend) /
/// [`LatexmkBackend`](crate::latexmk::LatexmkBackend)。
/// 第三方可通过实现本 trait 插入新引擎（如 `pdflatex` 兜底）。
#[async_trait]
pub trait TexBackend: Send + Sync {
    /// 后端代表的引擎种类。
    fn kind(&self) -> EngineKind;

    /// 引擎的稳定小写名（默认 = `self.kind().as_str()`）。
    fn name(&self) -> &'static str {
        self.kind().as_str()
    }

    /// 返回 `true` 表示本机已安装且能用。
    ///
    /// 探测逻辑：检查二进制是否存在 + 跑一次 `--version` 确认非僵尸。
    async fn is_available(&self) -> bool;

    /// 跑一次完整编译（含 bibtex 跑两轮），返回最终 PDF 路径。
    async fn compile(&self, project: &TexProject) -> Result<TexRun>;
}
