//! Semantic command argument types for the doc-engine CLI.

use std::path::PathBuf;

use clap::Args;

#[derive(Debug, Args)]
pub struct SemanticDetectArgs {
    /// TeX 项目根目录
    #[arg(long)]
    pub project_root: PathBuf,
    /// 主 .tex 文件相对路径
    #[arg(long, default_value = "main.tex")]
    pub main_tex: PathBuf,
    /// 输出 JSON 报告路径
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct SemanticAnalyzeArgs {
    /// TeX 项目根目录
    #[arg(long)]
    pub project_root: PathBuf,
    /// 主 .tex 文件相对路径
    #[arg(long, default_value = "main.tex")]
    pub main_tex: PathBuf,
    /// Profile 类型
    #[arg(long, default_value = "generic")]
    pub profile: String,
    /// 输出 JSON 报告路径
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct SemanticConvertArgs {
    /// TeX 项目根目录
    #[arg(long)]
    pub project_root: PathBuf,
    /// 主 .tex 文件相对路径
    #[arg(long)]
    pub main_tex: PathBuf,
    /// Profile ID
    #[arg(long, default_value = "auto")]
    pub profile: String,
    /// 语义后端
    #[arg(long, default_value = "auto")]
    pub backend: String,
    /// 输出 DOCX 路径
    #[arg(long)]
    pub out: PathBuf,
    /// 输出报告 JSON 路径
    #[arg(long)]
    pub report: Option<PathBuf>,
    /// 不允许后端回退
    #[arg(long, default_value_t = false)]
    pub no_backend_fallback: bool,
}

#[derive(Debug, Args)]
pub struct SemanticVerifyArgs {
    /// DOCX 文件路径
    #[arg(long)]
    pub docx_file: PathBuf,
    /// 报告 JSON 路径
    #[arg(long)]
    pub report: Option<PathBuf>,
}
