//! 商业化质量评分与报告体系。
//!
//! 对应技术方案第 1.2 节"QualityRun 多维评分体系"：
//! - 替换 `score_from_docx_size()` 的 DOCX 大小 heuristic
//! - 对齐方案第 3 节六维度权重：[parse:20%, semantic:25%, docx:20%, visual:20%, editable:10%, performance:5%]
//!
//! ## 设计原则
//!
//! 1. `QualityRun` 是对外暴露的统一质量报告结构，可序列化 JSON 返回给 API。
//! 2. 内部仍使用现有的 `Quality` + `LayerResult` + `Check` 做底层检查。
//! 3. `QualityRun::from_quality_report()` 将底层结果映射到六维评分。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::layer::{Check, LayerResult, Severity};

/// 六维质量评分（与方案第 3 节对齐）。
///
/// | 维度 | 权重 | 说明 |
/// |------|------|------|
/// | parse | 20% | 解析完整性 |
/// | semantic | 25% | 语义保真 |
/// | docx | 20% | DOCX 结构 |
/// | visual | 20% | 版面一致 |
/// | editable | 10% | 可编辑性 |
/// | performance | 5% | 性能与稳定 |
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DimensionScores {
    /// 解析完整性：include、宏、环境、错误恢复、未知命令比例。
    pub parse: u8,
    /// 语义保真：标题、段落、列表、表格、图、公式、引用、参考文献。
    pub semantic: u8,
    /// DOCX 结构：OOXML 合法性、style、relationship、media、field。
    pub docx: u8,
    /// 版面一致：页边距、字号、行距、表格宽度、浮动体位置、PDF 视觉差异。
    pub visual: u8,
    /// 可编辑性：Word 打开、段落样式、交叉引用可更新、公式可编辑。
    pub editable: u8,
    /// 性能与稳定：耗时、内存、fallback、重试、超时。
    pub performance: u8,
}

impl DimensionScores {
    /// 六维权重常量（与方案第 3 节对齐）。
    pub const WEIGHTS: [f64; 6] = [0.20, 0.25, 0.20, 0.20, 0.10, 0.05];

    /// 根据六维评分计算加权总分（0-100）。
    pub fn weighted_score(&self) -> u8 {
        let scores = [
            self.parse as f64,
            self.semantic as f64,
            self.docx as f64,
            self.visual as f64,
            self.editable as f64,
            self.performance as f64,
        ];
        let total: f64 = Self::WEIGHTS
            .iter()
            .zip(scores.iter())
            .map(|(w, s)| w * s)
            .sum();
        total.round() as u8
    }
}

/// 质量问题的严重级别（对外暴露版）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Blocking,
    Warning,
    Info,
}

impl IssueSeverity {
    pub fn from_severity(s: Severity) -> Self {
        match s {
            Severity::Critical => Self::Blocking,
            Severity::Major => Self::Blocking,
            Severity::Minor => Self::Warning,
            Severity::Info => Self::Info,
        }
    }
}

/// 一条质量问题（对外暴露版）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityIssue {
    /// 问题名称/标识。
    pub name: String,
    /// 严重级别。
    pub severity: IssueSeverity,
    /// 问题描述。
    pub description: String,
    /// 修复建议。
    pub suggestion: Option<String>,
    /// 来源层级：structural / textual / visual。
    pub layer: String,
}

impl QualityIssue {
    /// 从内部 `Check` 转换为 `QualityIssue`。
    pub fn from_check(check: &Check) -> Self {
        let suggestion = check.note.clone().or_else(|| {
            if !check.passed {
                Some(format!(
                    "Expected: {}, Actual: {}",
                    check.expected, check.actual
                ))
            } else {
                None
            }
        });
        Self {
            name: check.name.clone(),
            severity: IssueSeverity::from_severity(check.severity),
            description: format!("Expected: {}, Actual: {}", check.expected, check.actual),
            suggestion,
            layer: check.severity.as_layer_str().to_string(),
        }
    }

    /// 是否为阻断性问题。
    pub fn is_blocking(&self) -> bool {
        self.severity == IssueSeverity::Blocking
    }
}

impl Severity {
    fn as_layer_str(&self) -> &'static str {
        match self {
            Severity::Critical => "structural",
            Severity::Major => "structural",
            Severity::Minor => "textual",
            Severity::Info => "visual",
        }
    }
}

/// 语义损失事件：记录 fallback、降级和不可解析内容。
///
/// 对应方案第 2.1 节"宏/环境能力矩阵"中的 `semantic_loss_events`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticLossEvent {
    /// 宏命令或环境名。
    pub name: String,
    /// 位置（源文件:行号）。
    pub location: Option<String>,
    /// 降级方式：native / lowered / text_fallback / unsupported。
    pub support_level: String,
    /// 影响级别：blocking / degraded / ignorable。
    pub impact_level: String,
    /// 用户可见的降级描述。
    pub description: String,
    /// 用户可操作的修复建议。
    pub user_hint: Option<String>,
}

impl SemanticLossEvent {
    /// 简便构造。
    pub fn new(
        name: impl Into<String>,
        support_level: impl Into<String>,
        impact_level: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            location: None,
            support_level: support_level.into(),
            impact_level: impact_level.into(),
            description: description.into(),
            user_hint: None,
        }
    }

    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.user_hint = Some(hint.into());
        self
    }
}

/// Word 兼容性检查结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordCompatibility {
    /// overall: passed / warnings / failed。
    pub status: String,
    /// Word 打开时是否有自动修复提示。
    pub auto_repair_detected: bool,
    /// 打开过程中记录的错误列表。
    pub errors: Vec<String>,
    /// 检查方式：word / libreoffice / schema_only。
    pub check_method: String,
}

impl Default for WordCompatibility {
    fn default() -> Self {
        Self {
            status: "unchecked".to_string(),
            auto_repair_detected: false,
            errors: Vec::new(),
            check_method: "none".to_string(),
        }
    }
}

/// 质量运行的输出产物。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityArtifacts {
    /// quality report JSON 路径。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_json: Option<PathBuf>,
    /// quality report Markdown 路径。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_md: Option<PathBuf>,
    /// 原始 DOCX 路径。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docx: Option<PathBuf>,
    /// 转换日志路径。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log: Option<PathBuf>,
}

impl Default for QualityArtifacts {
    fn default() -> Self {
        Self {
            report_json: None,
            report_md: None,
            docx: None,
            log: None,
        }
    }
}

/// 统一质量运行报告（对外 API 返回格式）。
///
/// 对应方案第 8 节"质量报告建议结构"。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityRun {
    /// 关联的 job id。
    pub job_id: String,
    /// 引擎版本。
    pub engine_version: String,
    /// 使用的 profile。
    pub profile: String,
    /// 使用的后端信息。
    pub backend: BackendSummary,
    /// 加权总分（0-100）。
    pub quality_score: u8,
    /// 六维评分。
    pub dimension_scores: DimensionScores,
    /// 阻断性问题列表。
    pub blocking_issues: Vec<QualityIssue>,
    /// 警告列表。
    pub warnings: Vec<QualityIssue>,
    /// 语义损失事件列表。
    pub semantic_loss_events: Vec<SemanticLossEvent>,
    /// Word 兼容性结果。
    pub word_compatibility: WordCompatibility,
    /// 输出产物信息。
    pub artifacts: QualityArtifacts,
}

impl QualityRun {
    /// 从内部 `LayerResult` 列表构建 `QualityRun`。
    ///
    /// layer → dimension 映射：
    /// - structural → parse(100%) + docx(60%) + visual(30%)
    /// - textual → semantic(100%)
    /// - visual → visual(50%) + editable(40%)
    ///
    /// 另外计算：
    /// - 阻断项：从 Critical/Major 检查收集
    /// - 警告：从 Minor 检查收集
    pub fn from_layer_results(
        job_id: impl Into<String>,
        engine_version: impl Into<String>,
        profile: impl Into<String>,
        backend: BackendSummary,
        layers: Vec<LayerResult>,
        semantic_losses: Vec<SemanticLossEvent>,
    ) -> Self {
        let mut parse_checks = Vec::new();
        let mut semantic_checks = Vec::new();
        let mut docx_checks = Vec::new();
        let mut visual_checks = Vec::new();
        let mut editable_checks = Vec::new();

        for layer in &layers {
            match layer.layer {
                crate::layer::Layer::Structural => {
                    parse_checks.extend(layer.checks.clone());
                    docx_checks.extend(layer.checks.clone());
                    visual_checks.extend(layer.checks.clone());
                }
                crate::layer::Layer::Textual => {
                    semantic_checks.extend(layer.checks.clone());
                }
                crate::layer::Layer::Visual => {
                    visual_checks.extend(layer.checks.clone());
                    editable_checks.extend(layer.checks.clone());
                }
            }
        }

        let calc_dim_score = |checks: &[Check], weight: f64| -> u8 {
            if checks.is_empty() {
                return 100;
            }
            let passed_count = checks.iter().filter(|c| c.passed).count();
            let base = passed_count as f64 / checks.len() as f64;
            ((base * 100.0 * weight).round() as u8).min(100)
        };

        let scores = DimensionScores {
            parse: calc_dim_score(&parse_checks, 1.0),
            semantic: calc_dim_score(&semantic_checks, 1.0),
            docx: calc_dim_score(&docx_checks, 1.0),
            visual: calc_dim_score(&visual_checks, 1.0),
            editable: calc_dim_score(&editable_checks, 1.0),
            performance: 100, // TODO: 从性能指标填充
        };

        let quality_score = scores.weighted_score();

        // 收集阻断项和警告
        let mut blocking_issues = Vec::new();
        let mut warnings = Vec::new();
        for layer in &layers {
            for check in &layer.checks {
                if !check.passed {
                    let issue = QualityIssue::from_check(check);
                    if issue.is_blocking() {
                        blocking_issues.push(issue);
                    } else {
                        warnings.push(issue);
                    }
                }
            }
        }

        Self {
            job_id: job_id.into(),
            engine_version: engine_version.into(),
            profile: profile.into(),
            backend,
            quality_score,
            dimension_scores: scores,
            blocking_issues,
            warnings,
            semantic_loss_events: semantic_losses,
            word_compatibility: WordCompatibility::default(),
            artifacts: QualityArtifacts {
                report_json: None,
                report_md: None,
                docx: None,
                log: None,
            },
        }
    }
}

/// 后端摘要。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendSummary {
    /// 请求的后端类型。
    pub requested: String,
    /// 实际选择的后端。
    pub selected: String,
    /// 若发生了 fallback，记录来源。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_from: Option<String>,
    /// 选择/切换原因。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl BackendSummary {
    pub fn new(requested: &str, selected: &str) -> Self {
        Self {
            requested: requested.to_string(),
            selected: selected.to_string(),
            fallback_from: None,
            reason: None,
        }
    }

    pub fn with_fallback(mut self, from: &str, reason: &str) -> Self {
        self.fallback_from = Some(from.to_string());
        self.reason = Some(reason.to_string());
        self
    }
}

/// 质量运行退出码（替代原有的 `i32` 退出码）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityExitCode {
    /// 所有层通过。
    AllPassed,
    /// structural 或 textual 层失败。
    StructuralOrTextualFail,
    /// 仅 visual 层失败。
    VisualFailOnly,
}

impl From<QualityExitCode> for i32 {
    fn from(code: QualityExitCode) -> Self {
        match code {
            QualityExitCode::AllPassed => 0,
            QualityExitCode::StructuralOrTextualFail => 1,
            QualityExitCode::VisualFailOnly => 2,
        }
    }
}

impl QualityRun {
    /// 计算质量退出码。
    pub fn exit_code(&self) -> QualityExitCode {
        if !self.blocking_issues.is_empty() {
            // 检查是否有 structural/textual 来源的阻断
            let has_structural_blocking = self
                .blocking_issues
                .iter()
                .any(|i| i.layer == "structural");
            if has_structural_blocking {
                return QualityExitCode::StructuralOrTextualFail;
            }
        }
        // TODO: 检查 textual 层阻断
        if !self.warnings.is_empty() || self.word_compatibility.status == "failed" {
            return QualityExitCode::VisualFailOnly;
        }
        QualityExitCode::AllPassed
    }
}
