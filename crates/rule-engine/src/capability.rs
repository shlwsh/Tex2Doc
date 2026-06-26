//! 宏/环境能力矩阵。
//!
//! 对应技术方案第 2.1 节"宏/环境能力矩阵"：
//! - 记录每个宏/环境的支持级别
//! - 支持级别的变更历史
//! - 用户可见的提示信息

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 宏/环境支持级别。
///
/// | 级别 | 说明 | 对用户的影响 |
/// |------|------|-------------|
/// | Native | 完整语义支持 | 无需处理 |
/// | Lowered | 已转换为语义 AST | 自动处理 |
/// | TextFallback | 仅提取文本 | 内容可见但样式丢失 |
/// | Unsupported | 完全忽略 | 用户需要手动处理 |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportLevel {
    /// 完整语义支持，输出与 LaTeX 等价的 DOCX。
    Native,
    /// 已转换为语义 AST（保留结构信息）。
    Lowered,
    /// 仅提取文本内容，丢失样式信息。
    TextFallback,
    /// 完全不支持，输出忽略或占位符。
    Unsupported,
}

impl SupportLevel {
    /// 返回用户可读的描述。
    pub fn description(&self) -> &'static str {
        match self {
            Self::Native => "完整支持",
            Self::Lowered => "已转换（结构保留）",
            Self::TextFallback => "文本降级（样式丢失）",
            Self::Unsupported => "不支持",
        }
    }

    /// 返回在报告中的可见性级别。
    pub fn report_level(&self) -> &'static str {
        match self {
            Self::Native => "hidden",
            Self::Lowered => "info",
            Self::TextFallback => "warning",
            Self::Unsupported => "error",
        }
    }
}

/// 宏/环境影响级别。
///
/// | 级别 | 说明 | 是否阻断转换 |
/// |------|------|------------|
/// | Blocking | 导致转换失败或输出损坏 | 是 |
/// | Degraded | 输出可用但质量下降 | 否 |
/// | Ignorable | 基本无影响 | 否 |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactLevel {
    /// 导致转换失败或输出损坏。
    Blocking,
    /// 输出可用但质量下降。
    Degraded,
    /// 基本无影响。
    Ignorable,
}

impl ImpactLevel {
    pub fn description(&self) -> &'static str {
        match self {
            Self::Blocking => "阻断",
            Self::Degraded => "降级",
            Self::Ignorable => "可忽略",
        }
    }
}

/// 单个宏/环境的能力记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroCapability {
    /// 宏命令名（不含反斜杠），如 `textbf`、`includegraphics`。
    pub name: String,
    /// 环境名（如 `enumerate`），无则为 None。
    pub environment: Option<String>,
    /// 支持级别。
    pub support_level: SupportLevel,
    /// 影响级别。
    pub impact_level: ImpactLevel,
    /// 用户可见的描述。
    pub description: Option<String>,
    /// 用户可操作的修复建议。
    pub user_hint: Option<String>,
    /// 版本标记（用于追踪支持级别的变更）。
    pub since_version: Option<String>,
}

impl MacroCapability {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            environment: None,
            support_level: SupportLevel::Unsupported,
            impact_level: ImpactLevel::Ignorable,
            description: None,
            user_hint: None,
            since_version: None,
        }
    }

    pub fn for_environment(name: impl Into<String>) -> Self {
        let name_str = name.into();
        Self {
            name: name_str.clone(),
            environment: Some(name_str),
            support_level: SupportLevel::Unsupported,
            impact_level: ImpactLevel::Ignorable,
            description: None,
            user_hint: None,
            since_version: None,
        }
    }

    pub fn with_support(mut self, level: SupportLevel) -> Self {
        self.support_level = level;
        self
    }

    pub fn with_impact(mut self, level: ImpactLevel) -> Self {
        self.impact_level = level;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.user_hint = Some(hint.into());
        self
    }
}

/// 能力矩阵：记录所有已知宏/环境的支持状态。
///
/// 用于：
/// - 生成能力报告
/// - 向用户解释降级原因
/// - 追踪支持级别的演进
#[derive(Debug, Clone, Default)]
pub struct CapabilityMatrix {
    /// 按宏/环境名索引的映射。
    capabilities: BTreeMap<String, MacroCapability>,
}

impl CapabilityMatrix {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一个宏/环境的能力。
    pub fn register(&mut self, capability: MacroCapability) {
        let key = if let Some(ref env) = capability.environment {
            format!("environment:{}", env)
        } else {
            format!("macro:{}", capability.name)
        };
        self.capabilities.insert(key, capability);
    }

    /// 查找宏/环境的能力。
    pub fn lookup(&self, name: &str, is_environment: bool) -> Option<&MacroCapability> {
        let key = if is_environment {
            format!("environment:{}", name)
        } else {
            format!("macro:{}", name)
        };
        self.capabilities.get(&key)
    }

    /// 获取所有能力记录。
    pub fn all(&self) -> impl Iterator<Item = &MacroCapability> {
        self.capabilities.values()
    }

    /// 按支持级别筛选。
    pub fn by_support_level(
        &self,
        level: SupportLevel,
    ) -> impl Iterator<Item = &MacroCapability> {
        self.capabilities.values().filter(move |c| c.support_level == level)
    }

    /// 按影响级别筛选。
    pub fn by_impact_level(
        &self,
        level: ImpactLevel,
    ) -> impl Iterator<Item = &MacroCapability> {
        self.capabilities.values().filter(move |c| c.impact_level == level)
    }

    /// 生成能力覆盖率报告。
    pub fn coverage_report(&self) -> CapabilityCoverageReport {
        let total = self.capabilities.len();
        let native = self.by_support_level(SupportLevel::Native).count();
        let lowered = self.by_support_level(SupportLevel::Lowered).count();
        let text_fallback = self.by_support_level(SupportLevel::TextFallback).count();
        let unsupported = self.by_support_level(SupportLevel::Unsupported).count();

        let blocking = self.by_impact_level(ImpactLevel::Blocking).count();
        let degraded = self.by_impact_level(ImpactLevel::Degraded).count();
        let ignorable = self.by_impact_level(ImpactLevel::Ignorable).count();

        CapabilityCoverageReport {
            total,
            by_support: SupportLevelCounts { native, lowered, text_fallback, unsupported },
            by_impact: ImpactLevelCounts { blocking, degraded, ignorable },
            support_rate: if total > 0 {
                (native + lowered) as f64 / total as f64
            } else {
                0.0
            },
        }
    }
}

/// 能力覆盖率报告。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityCoverageReport {
    pub total: usize,
    pub by_support: SupportLevelCounts,
    pub by_impact: ImpactLevelCounts,
    /// 有效支持率（Native + Lowered）/ Total。
    pub support_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportLevelCounts {
    pub native: usize,
    pub lowered: usize,
    pub text_fallback: usize,
    pub unsupported: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactLevelCounts {
    pub blocking: usize,
    pub degraded: usize,
    pub ignorable: usize,
}

/// 内置标准宏/环境能力矩阵。
pub fn builtin_capability_matrix() -> CapabilityMatrix {
    use SupportLevel as S;
    use ImpactLevel as I;

    let mut matrix = CapabilityMatrix::new();

    // 文本格式（有 hint 的）
    for (name, desc, hint) in [
        ("textbf", "加粗文本", "使用 Word 加粗快捷键 Ctrl+B"),
        ("textit", "斜体文本", "使用 Word 斜体快捷键 Ctrl+I"),
        ("emph", "强调文本", "转换为斜体"),
        ("underline", "下划线文本", "使用 Word 下划线快捷键 Ctrl+U"),
        ("texttt", "等宽字体", "使用 Courier New 等宽字体"),
    ] {
        let mut cap = MacroCapability::new(name)
            .with_support(S::Native)
            .with_impact(I::Ignorable)
            .with_description(desc)
            .with_hint(hint);
        matrix.register(cap);
    }

    // 文本格式（无 hint 的）
    for (name, desc) in [
        ("sout", "删除线文本"),
        ("textsf", "无衬线字体"),
    ] {
        matrix.register(
            MacroCapability::new(name)
                .with_support(S::Native)
                .with_impact(I::Ignorable)
                .with_description(desc),
        );
    }

    // 列表环境
    for (name, desc) in [
        ("itemize", "无序列表",),
        ("enumerate", "有序列表",),
        ("description", "描述列表",),
    ] {
        matrix.register(
            MacroCapability::for_environment(name)
                .with_support(S::Native)
                .with_impact(I::Ignorable)
                .with_description(desc),
        );
    }

    // 表格
    for (name, desc, hint) in [
        ("tabular", "表格", Some("支持基本列类型 l/c/r/p")),
        ("tabularx", "自适应宽度表格", Some("转换为固定宽度表格")),
        ("longtable", "跨页表格", Some("拆分为多个表格")),
        ("booktabs", "专业表格（三线表）", Some("保留三线表样式")),
    ] {
        let mut cap = MacroCapability::for_environment(name)
            .with_support(S::Lowered)
            .with_impact(I::Degraded);
        if name == "tabular" {
            cap = cap.with_support(S::Native);
        }
        cap = cap.with_description(desc);
        if let Some(h) = hint {
            cap = cap.with_hint(h);
        }
        matrix.register(cap);
    }

    // 图片/浮动体
    for (name, desc, hint) in [
        ("figure", "图片浮动体", Some("保留 caption 和 label")),
        ("table", "表格浮动体", Some("保留 caption 和 label")),
        ("includegraphics", "图片插入", Some("支持 width/height/scale 选项")),
        ("subcaption", "子图 caption", Some("转换为独立 caption")),
    ] {
        let mut cap = MacroCapability::new(name)
            .with_support(S::Lowered)
            .with_impact(I::Degraded)
            .with_description(desc);
        if let Some(h) = hint {
            cap = cap.with_hint(h);
        }
        matrix.register(cap);
    }

    // 数学公式
    for (name, desc) in [
        ("equation", "行间公式（自动编号）"),
        ("equation*", "行间公式（不编号）"),
        ("align", "多行对齐公式"),
        ("align*", "多行对齐公式（不编号）"),
        ("gather", "居中公式组"),
        ("matrix", "矩阵"),
        ("bmatrix", "方括号矩阵"),
    ] {
        matrix.register(
            MacroCapability::for_environment(name)
                .with_support(S::Lowered)
                .with_impact(I::Degraded)
                .with_description(desc),
        );
    }

    // 参考文献
    for (name, desc, hint) in [
        ("bibliography", "参考文献", Some("保留文献列表格式")),
        ("bibliographystyle", "参考文献样式", Some("应用 Word 样式")),
        ("cite", "引用", Some("转换为交叉引用")),
        ("citep", "括号引用", Some("转换为作者[年份]格式")),
        ("citet", "正文引用", Some("转换为作者[年份]格式")),
    ] {
        let mut cap = MacroCapability::new(name)
            .with_support(S::Lowered)
            .with_impact(I::Degraded)
            .with_description(desc);
        if let Some(h) = hint {
            cap = cap.with_hint(h);
        }
        matrix.register(cap);
    }

    // 文本结构
    for (name, desc) in [
        ("section", "一级标题"),
        ("subsection", "二级标题"),
        ("subsubsection", "三级标题"),
        ("paragraph", "段落标题"),
        ("textbf", "粗体"),
        ("textit", "斜体"),
    ] {
        matrix.register(
            MacroCapability::new(name)
                .with_support(S::Native)
                .with_impact(I::Ignorable)
                .with_description(desc),
        );
    }

    matrix
}
