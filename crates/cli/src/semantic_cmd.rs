//! Semantic command argument types for the doc-engine CLI.

use std::path::PathBuf;

use clap::Args;

// P4.2: Standardized exit codes for CLI commands.
#[derive(Debug, Clone, Copy)]
pub enum CliExitCode {
    Success = 0,
    EInputInvalid = 1,
    EMainTexMissing = 2,
    EProfileLowConfidence = 3,
    ECompatUnsupported = 4,
    EConvertFailed = 5,
    EDocxInvalid = 6,
    EQualityFailed = 7,
}

impl CliExitCode {
    pub fn code(&self) -> i32 {
        *self as i32
    }
}

#[derive(Debug, Clone, Copy)]
pub enum QualityLevel {
    Preview,
    Standard,
    Strict,
}

impl Default for QualityLevel {
    fn default() -> Self {
        Self::Standard
    }
}

impl std::str::FromStr for QualityLevel {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "preview" => Ok(Self::Preview),
            "strict" => Ok(Self::Strict),
            "standard" => Ok(Self::Standard),
            _ => Err(format!("invalid quality level '{}': must be preview|standard|strict", s)),
        }
    }
}

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
    /// 强制输出 JSON 格式
    #[arg(long, default_value_t = false)]
    pub json: bool,
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
    /// 强制输出 JSON 格式
    #[arg(long, default_value_t = false)]
    pub json: bool,
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
    /// 强制输出 JSON 格式
    #[arg(long, default_value_t = false)]
    pub json: bool,
    /// 质量检查级别 (preview|standard|strict)
    #[arg(long, default_value = "standard")]
    pub quality: String,
}

#[derive(Debug, Args)]
pub struct SemanticVerifyArgs {
    /// DOCX 文件路径
    #[arg(long)]
    pub docx_file: PathBuf,
    /// 报告 JSON 路径
    #[arg(long)]
    pub report: Option<PathBuf>,
    /// 强制输出 JSON 格式
    #[arg(long, default_value_t = false)]
    pub json: bool,
    /// 跳过结构层检查
    #[arg(long, default_value_t = false)]
    pub skip_structural: bool,
    /// 跳过文本层检查
    #[arg(long, default_value_t = false)]
    pub skip_textual: bool,
    /// 跳过视觉层检查
    #[arg(long, default_value_t = false)]
    pub skip_visual: bool,
}
