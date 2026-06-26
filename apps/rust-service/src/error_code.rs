//! 转换任务专用错误码体系。
//!
//! 所有外部可见的转换失败都应使用此枚举的变体作为 error_code，
//! 以确保错误码稳定、可归类、可向用户解释。
//!
//! 对应方案第 8 节"错误码与质量报告建议"中的完整列表。

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 转换任务错误码（对外暴露的稳定错误码）。
///
/// 每个变体对应一个机器可读的错误码字符串，便于：
/// - 前端按错误码显示用户友好的提示
/// - 服务端按错误码聚合失败原因统计
/// - 国际化（i18n）按错误码映射用户文案
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversionErrorCode {
    // ========== P0 上传与预检 ==========
    #[error("zip 文件不合法或存在安全风险")]
    UploadInvalidZip,

    #[error("主 tex 文件不存在")]
    MainTexNotFound,

    #[error("关键宏包不支持")]
    PreflightUnsupportedPackage,

    // ========== P1 解析与编译 ==========
    #[error("语义解析失败")]
    SemanticParseFailed,

    #[error("TeX 运行时不可用")]
    BackendRuntimeUnavailable,

    #[error("DOCX 渲染失败")]
    DocxRenderFailed,

    // ========== P2 质量与兼容性 ==========
    #[error("Word 打开或校验失败")]
    WordCompatibilityFailed,

    #[error("质量门禁未通过")]
    QualityGateFailed,

    // ========== P3 服务与资源 ==========
    #[error("额度不足")]
    QuotaExhausted,

    #[error("转换任务超时")]
    JobTimeout,

    #[error("worker 执行异常")]
    WorkerJoinError,

    // ========== 遗留兼容（现有调用迁移后保留） ==========
    #[error("上传记录不存在")]
    UploadNotFound,

    #[error("转换引擎执行失败")]
    ConvertFailed,

    #[error("生成的 DOCX 无效")]
    InvalidDocx,

    #[error("用户认证失败")]
    Unauthorized,

    #[error("资源未找到")]
    NotFound,

    #[error("内部服务端错误")]
    InternalError,
}

impl ConversionErrorCode {
    /// 返回机器可读的错误码字符串（如 "upload_invalid_zip"）。
    pub fn as_code(&self) -> &'static str {
        match self {
            Self::UploadInvalidZip => "upload_invalid_zip",
            Self::MainTexNotFound => "main_tex_not_found",
            Self::PreflightUnsupportedPackage => "preflight_unsupported_package",
            Self::SemanticParseFailed => "semantic_parse_failed",
            Self::BackendRuntimeUnavailable => "backend_runtime_unavailable",
            Self::DocxRenderFailed => "docx_render_failed",
            Self::WordCompatibilityFailed => "word_compatibility_failed",
            Self::QualityGateFailed => "quality_gate_failed",
            Self::QuotaExhausted => "quota_exhausted",
            Self::JobTimeout => "job_timeout",
            Self::WorkerJoinError => "worker_join_error",
            Self::UploadNotFound => "upload_not_found",
            Self::ConvertFailed => "convert_failed",
            Self::InvalidDocx => "invalid_docx",
            Self::Unauthorized => "unauthorized",
            Self::NotFound => "not_found",
            Self::InternalError => "internal_error",
        }
    }

    /// 返回错误的 HTTP 状态码。
    pub fn http_status(&self) -> u16 {
        match self {
            // 上传/预检错误 → 400
            Self::UploadInvalidZip
            | Self::MainTexNotFound
            | Self::PreflightUnsupportedPackage
            | Self::SemanticParseFailed
            | Self::DocxRenderFailed
            | Self::InvalidDocx => 400,

            // 资源不存在 → 404
            Self::UploadNotFound | Self::NotFound => 404,

            // 认证失败 → 401
            Self::Unauthorized => 401,

            // 额度不足 → 402
            Self::QuotaExhausted => 402,

            // 服务端错误 → 500
            Self::BackendRuntimeUnavailable
            | Self::WordCompatibilityFailed
            | Self::QualityGateFailed
            | Self::JobTimeout
            | Self::WorkerJoinError
            | Self::ConvertFailed
            | Self::InternalError => 500,
        }
    }

    /// 返回用户友好的默认提示（可进一步 i18n 化）。
    pub fn user_hint(&self) -> &'static str {
        match self {
            Self::UploadInvalidZip => "请重新打包项目，避免嵌套危险路径或损坏的压缩包。",
            Self::MainTexNotFound => "请指定 main.tex 路径或检查压缩包结构。",
            Self::PreflightUnsupportedPackage => "可尝试使用 strict 质量级别或移除不支持的宏包后重试。",
            Self::SemanticParseFailed => "查看报告中源文件的问题位置，尝试简化或修复相关宏命令。",
            Self::BackendRuntimeUnavailable => "服务端 TeX 运行时暂时不可用，请稍后重试。",
            Self::DocxRenderFailed => "DOCX 渲染失败，请联系支持并附上 job id。",
            Self::WordCompatibilityFailed => "已生成报告但不建议直接投稿，查看阻断项修复建议。",
            Self::QualityGateFailed => "查看阻断项和修复建议后可再次尝试，或联系支持。",
            Self::QuotaExhausted => "您的转换额度已用完，请购买或兑换额度后重试。",
            Self::JobTimeout => "任务超时，请尝试精简项目或使用企业队列。",
            Self::WorkerJoinError => "服务端执行异常，请稍后重试。",
            Self::UploadNotFound => "上传记录不存在，请重新上传。",
            Self::ConvertFailed => "转换执行失败，请查看详细错误或联系支持。",
            Self::InvalidDocx => "生成的 DOCX 文件无效，请联系支持。",
            Self::Unauthorized => "认证失败，请重新登录。",
            Self::NotFound => "请求的资源不存在。",
            Self::InternalError => "服务端内部错误，请稍后重试或联系支持。",
        }
    }

    /// 判断是否为服务端可自行恢复的错误（可重试）。
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::BackendRuntimeUnavailable
                | Self::JobTimeout
                | Self::WorkerJoinError
                | Self::InternalError
        )
    }

    /// 判断是否需要退款（用户侧错误）。
    pub fn should_refund(&self) -> bool {
        !matches!(
            self,
            Self::BackendRuntimeUnavailable
                | Self::JobTimeout
                | Self::WorkerJoinError
                | Self::InternalError
        )
    }

    /// 从字符串反序列化（兼容现有数据库中的 ad-hoc 错误码）。
    pub fn from_code(code: &str) -> Option<Self> {
        Some(match code {
            "upload_invalid_zip" => Self::UploadInvalidZip,
            "main_tex_not_found" => Self::MainTexNotFound,
            "preflight_unsupported_package" => Self::PreflightUnsupportedPackage,
            "semantic_parse_failed" => Self::SemanticParseFailed,
            "backend_runtime_unavailable" => Self::BackendRuntimeUnavailable,
            "docx_render_failed" => Self::DocxRenderFailed,
            "word_compatibility_failed" => Self::WordCompatibilityFailed,
            "quality_gate_failed" => Self::QualityGateFailed,
            "quota_exhausted" => Self::QuotaExhausted,
            "job_timeout" => Self::JobTimeout,
            "worker_join_error" => Self::WorkerJoinError,
            "upload_not_found" => Self::UploadNotFound,
            "convert_failed" => Self::ConvertFailed,
            "invalid_docx" => Self::InvalidDocx,
            "unauthorized" => Self::Unauthorized,
            "not_found" => Self::NotFound,
            "internal_error" => Self::InternalError,
            _ => return None,
        })
    }
}
