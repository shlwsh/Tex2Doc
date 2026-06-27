# P1 开发计划（实施版）
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



**文档版本**：20260621-164900
**状态**：已实施
**目标读者**：开发者、项目管理者
**覆盖范围**：Semantic TeX Engine 新语义引擎路径的全部已实现功能

---

## 一、概述

本文档记录 Semantic TeX Engine（即 `doc-compiler-engine` 语义编译引擎）已完成实施的全部功能模块。文档按功能领域组织，每个领域包含：实现状态、关键文件、使用示例和测试验证。

### 1.1 已实现的核心模块

| 模块 | 状态 | 关键 Crate |
|------|------|-----------|
| Journal Profile System | ✅ DONE | `compiler-engine` |
| CompileOptions | ✅ DONE | `compiler-engine` |
| SemanticTexEngine | ✅ DONE | `compiler-engine` |
| CompileReport | ✅ DONE | `compiler-engine` |
| QualityGateResult | ✅ DONE | `compiler-engine` |
| ProfileStyleMap | ✅ DONE | `docx-writer` |
| CLI (doc-engine) | ✅ DONE | `cli` |
| commercial_api_client | ✅ DONE | `commercial-api-client` |
| desktop_slint | ✅ DONE | `desktop-slint` |
| verify_journal_profiles.sh | ✅ DONE | scripts/ |
| commercial_verify.sh | ✅ DONE | scripts/ |

### 1.2 工程结构概览

```
crates/
├── compiler-engine/           # 新语义引擎核心
│   ├── src/
│   │   ├── lib.rs            # SemanticTexEngine、CompileOptions、CompileReport、QualityGateResult
│   │   ├── profiles.rs       # ProfileRegistry、ProfileSpecFile、TOML 加载
│   │   ├── journal_detector.rs # JournalDetector、SignalKind、JournalDetectionReport
│   ├── profiles/              # 期刊 Profile TOML 配置文件（7 个）
│   └── examples/
│       └── paper3_to_docx.rs # 语义引擎示例程序
├── compatibility-analyzer/     # 兼容性分析器
│   └── src/lib.rs            # CompatibilityAnalyzer、ProfileKind、CompatibilityReport
├── rule-engine/               # 规则引擎
│   ├── src/
│   │   ├── rule_output.rs   # RuleOutput enum（扩展版）
│   │   └── builtin_rules.rs  # builtin_rules()、journal_rules(profile_id)
├── docx-writer/              # DOCX 写入器
│   └── src/
│       └── profile.rs        # ProfileStyleMap（jos()、generic()）
├── semantic-collector/        # 语义事件采集
│   └── src/lib.rs           # SemanticBackendKind、SemanticEvent、ReferenceGraph
├── semantic-ast/             # 语义 AST
├── quality/                  # 质量验证（结构/文本/视觉三层）
├── commercial-api-client/    # 商业 API 客户端
│   └── src/
│       ├── client.rs        # ApiClient（multipart upload、job polling）
│       └── models.rs        # 请求/响应类型、JobStatus
├── cli/                      # 命令行工具
│   └── src/
│       ├── main.rs          # doc-engine 主程序（8 个子命令）
│       ├── semantic_cmd.rs   # SemanticDetect/Analyze/Convert/Verify 参数
│       ├── cmd.rs           # Convert、Build 参数
│       ├── tex_compile.rs   # TexCompile 参数
│       ├── docx2pdf.rs      # DocxToPdf 参数
│       ├── pdf_verify.rs     # VerifyPdf 参数
│       ├── ast_dump.rs      # AstDump 参数
│       ├── render_dump.rs   # RenderDump 参数
│       └── docx_diff.rs     # DocxDiff 参数
├── desktop-slint/            # Slint PC 客户端骨架
│   ├── src/main.rs
│   ├── src/ui/main.slint
│   └── Cargo.toml
└── server/                   # HTTP 服务器

scripts/
├── verify_journal_profiles.sh # 7 个期刊 Profile 验证脚本
└── commercial_verify.sh       # 商业化质量门禁脚本

examples/journals/            # 期刊 minimal fixture（7 个）
├── jos-paper/minimal.tex
├── tacl/minimal.tex
├── cvpr/minimal.tex
├── nature/minimal.tex
├── springer/minimal.tex
├── chinese-academic/minimal.tex
└── generic/minimal.tex
```

---

## 二、Journal Profile System（期刊 Profile 系统）

### 2.1 概述

Journal Profile System 是语义引擎泛化能力的核心。它定义了 7 个期刊 Profile，支持自动检测和显式指定，并通过 ProfileRegistry 统一管理所有 Profile 配置。

### 2.2 实现状态：✅ DONE

**关键文件**：

- `crates/compiler-engine/src/profiles.rs` — ProfileSchema、ProfileRegistry、TOML 加载
- `crates/compiler-engine/profiles/*.toml` — 7 个 TOML Profile 文件

**Profile 列表**：

| Profile ID | 显示名 | Document Class | 默认后端 |
|------------|--------|---------------|----------|
| `jos-paper` / `jos-paper-toml` | IEEE JOS Paper | `IEEEtran[journal]`、`rjthesis` | LuaTeXNode |
| `tacl` | ACL/TACL Paper | `acl[aclang]` | LuaTeXNode |
| `cvpr` | CVPR/ICCV Paper | `IEEEtran[conference]` | LuaTeXNode |
| `nature` | Nature Article | `nature` | LuaTeXNode |
| `springer` | Springer Article | `springer`、`svjour3`、`llncs` | LuaTeXNode |
| `chinese-academic` | 中文学术论文 | `ctexart` | XeLaTeXHook |
| `generic` / `generic-article` | 通用论文 | 任意 | LuaTeXNode |

**ProfileRegistry 核心 API**：

```rust
pub struct ProfileRegistry {
    profiles: HashMap<String, ProfileSpecFile>,
    aliases: HashMap<String, String>,
}

impl ProfileRegistry {
    pub fn load_default() -> Result<Self, ProfileLoadError>;
    pub fn get(&self, id: &str) -> Option<&ProfileSpecFile>;
    pub fn resolve_alias(&self, id: &str) -> Option<&str>;
    pub fn all_ids(&self) -> Vec<&str>;
    pub fn register(&mut self, spec: ProfileSpecFile);
    pub fn register_alias(&mut self, alias: String, canonical: String);
}
```

**ProfileSpecFile 扩展字段**：

```rust
pub struct ProfileSpecFile {
    pub id: String,
    pub display_name: String,
    pub detection: DetectionSpec,     // 检测信号与置信度
    pub backend: BackendSpec,         // 后端选择策略
    pub semantic_policy: SemanticPolicySpec, // 语义策略
    pub macro_rules: Vec<MacroRuleToml>, // 宏规则
    pub style_map: Vec<StyleMapSpec>,    // DOCX 样式映射
    pub quality: QualitySpec,            // 质量阈值
    // ... 其他字段
}
```

**检测置信度规则**：

- `score >= 0.75`：自动选择该 Profile
- `0.50 <= score < 0.75`：选择该 Profile，报告 warning
- `score < 0.50`：降级 generic

**测试验证**：

```bash
cargo test -p doc-compiler-engine profiles
# 27 tests passed
```

---

## 三、JournalDetector（期刊检测器）

### 3.1 概述

JournalDetector 从 TeX 源码中提取检测信号，对所有注册的 Profile 加权评分，返回最匹配的 Profile 及置信度。

### 3.2 实现状态：✅ DONE

**关键文件**：

- `crates/compiler-engine/src/journal_detector.rs` — JournalDetector 实现

**核心数据结构**：

```rust
pub enum SignalKind {
    DocumentClass,          // \documentclass
    DocumentClassOption,    // \documentclass[option]
    Package,                // \usepackage
    Macro,                  // 模板特有宏
    BibliographyStyle,      // \bibliographystyle
    EngineFeature,          // XeTeX/LuaTeX 特征
}

pub struct MatchedSignal {
    pub kind: SignalKind,
    pub value: String,
    pub weight: f32,
    pub source_path: String,
    pub line: Option<usize>,
}

pub struct JournalDetection {
    pub profile_id: String,
    pub confidence: f32,
    pub matched_signals: Vec<MatchedSignal>,
    pub fallback: bool,
}

pub struct JournalDetectionReport {
    pub selected_profile_id: String,
    pub confidence: f32,
    pub candidates: Vec<JournalDetection>,
    pub diagnostics: Vec<JournalDiagnostic>,
}
```

**检测信号权重**：

| 信号类型 | 权重 |
|----------|------|
| documentclass 精确匹配 | 0.70 |
| documentclass option 匹配 | 0.20 |
| 模板特有宏匹配 | 0.10 |
| bibliography style 匹配 | 0.05 |
| package 辅助匹配 | 0.05 |

**检测算法**：

1. 遍历 VFS 中所有 `.tex/.sty/.cls` 文件
2. 去除 `%` 注释（支持 `\%` 转义）
3. 扫描 `\documentclass[options]{class}`、`\usepackage`、`\bibliographystyle` 及模板特征宏
4. 对每个 Profile 的 `detection.signals` 加权打分
5. 按 `min_confidence >= 0.75` 选该 Profile；低于阈值降级 generic

**使用示例**：

```rust
use doc_compiler_engine::JournalDetector;
use doc_utils::VirtualFs;

let vfs = VirtualFs::new();
vfs.insert("main.tex", source.as_bytes().to_vec());

let detector = JournalDetector::new();
let report = detector.detect(&vfs);

println!("Profile: {}", report.selected_profile_id);
println!("Confidence: {:.2}", report.confidence);
```

**CLI 使用**：

```bash
cargo run -p doc-cli -- doc-engine semantic-detect \
  --project-root examples/journals/tacl \
  --main-tex minimal.tex \
  --output tacl-detection.json
```

**测试用例覆盖**：

```bash
cargo test -p doc-compiler-engine journal_detector
# 15 tests passed
```

测试用例验证：

| 输入 | 预期 Profile | 置信度 |
|------|-------------|--------|
| `\documentclass[journal]{IEEEtran}` | `jos-paper-toml` | >= 0.80 |
| `\documentclass[aclang]{acl}` | `tacl` | >= 0.80 |
| `\documentclass[conference]{IEEEtran}` + `\cvprfinalcopy` | `cvpr` | >= 0.85 |
| `\documentclass{nature}` + `\bibliographystyle{naturemag}` | `nature` | >= 0.75 |
| `\documentclass{springer}` + `\institute` | `springer` | >= 0.75 |
| `\documentclass{ctexart}` + `\setCJKmainfont` | `chinese-academic` | >= 0.75 |
| `\documentclass{article}` | `generic-article` | fallback |

---

## 四、EngineProfile Enum（内置 Profile 枚举）

### 4.1 概述

EngineProfile 是编译选项中使用的内置 Profile 枚举，提供 4 个基础 Profile 作为默认选项。

### 4.2 实现状态：✅ DONE

**关键文件**：

- `crates/compiler-engine/src/lib.rs` — EngineProfile 定义

**定义**：

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineProfile {
    /// General article/report/book style documents.
    #[default]
    GenericArticle,
    /// Chinese academic papers, including CTeX-based templates.
    ChineseAcademic,
    /// Journal of Software / 软件学报 oriented profile.
    JosPaper,
    /// Medical journal manuscripts with strict title/abstract/table needs.
    MedicalJournal,
}
```

**与 ProfileKind 的转换**：

```rust
impl From<EngineProfile> for ProfileKind {
    fn from(ep: EngineProfile) -> Self {
        match ep {
            EngineProfile::GenericArticle => ProfileKind::GenericArticle,
            EngineProfile::ChineseAcademic => ProfileKind::ChineseAcademic,
            EngineProfile::JosPaper => ProfileKind::JosPaper,
            EngineProfile::MedicalJournal => ProfileKind::MedicalJournal,
        }
    }
}
```

**ProfileSpec 包含的信息**：

```rust
pub struct ProfileSpec {
    pub profile: EngineProfile,
    pub id: &'static str,
    pub display_name: &'static str,
    pub document_classes: &'static [&'static str],
    pub page_setup: PageSetupProfile,
    pub font_policy: FontPolicySpec,
    pub caption_policy: CaptionPolicySpec,
    pub citation_policy: CitationPolicySpec,
    pub style_map: Option<doc_docx_writer::ProfileStyleMap>,
}
```

---

## 五、CompileOptions（编译选项）

### 5.1 概述

CompileOptions 是所有编译 API 的配置入口，控制 Profile、后端选择、样式映射和页面设置。

### 5.2 实现状态：✅ DONE

**关键文件**：

- `crates/compiler-engine/src/lib.rs` — CompileOptions 定义

**定义**：

```rust
pub struct CompileOptions {
    /// 内置 Profile 枚举（向后兼容）
    pub profile: EngineProfile,
    /// 语义采集后端：Auto / RuleBased / XeLaTeXHook / LuaTeXNode
    pub semantic_backend: SemanticBackendKind,
    /// 是否允许后端回退
    pub allow_backend_fallback: bool,
    /// 是否启用引用链接
    pub enable_reference_links: bool,
    /// 是否启用域引用
    pub enable_ref_fields: bool,
    /// 是否启用 OMML 公式
    pub enable_omml_equations: bool,
    /// DOCX 模板字节
    pub template_bytes: Option<Vec<u8>>,
    /// 页面设置（覆盖 Profile 默认值）
    pub page_setup: Option<doc_docx_writer::PageSetup>,
    /// 是否采集标准 AST
    pub collect_standard_ast: bool,
    /// 是否启用参考文献
    pub enable_bibliography: bool,
    /// Profile-specific 样式映射（覆盖 Profile 默认值）
    pub style_map: Option<doc_docx_writer::ProfileStyleMap>,
}
```

**默认值**：

```rust
impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            profile: EngineProfile::ChineseAcademic,
            semantic_backend: SemanticBackendKind::Auto,
            allow_backend_fallback: true,
            enable_reference_links: true,
            enable_ref_fields: false,
            enable_omml_equations: true,
            template_bytes: None,
            page_setup: None,
            collect_standard_ast: true,
            enable_bibliography: true,
            style_map: None,
        }
    }
}
```

**便捷方法**：

```rust
impl CompileOptions {
    /// 返回生效的页面设置：显式设置 > Profile 默认值
    pub fn effective_page_setup(&self) -> Option<doc_docx_writer::PageSetup>;

    /// 返回生效的样式映射：显式设置 > Profile 默认值
    pub fn effective_style_map(&self) -> Option<doc_docx_writer::ProfileStyleMap>;
}
```

---

## 六、SemanticTexEngine（语义编译引擎）

### 6.1 概述

SemanticTexEngine 是语义编译管线的顶层 API，提供 `compile_dir_to_docx`、`compile_vfs_to_docx`、`compile_vfs_to_graph` 三个编译入口。

### 6.2 实现状态：✅ DONE

**关键文件**：

- `crates/compiler-engine/src/lib.rs` — SemanticTexEngine 实现

**API 签名**：

```rust
pub struct SemanticTexEngine;

impl SemanticTexEngine {
    pub fn new() -> Self;

    /// 从内存源码编译为 DOCX
    pub fn compile_source_to_docx(
        &self,
        main_tex: &str,
        source: &str,
        options: &CompileOptions,
    ) -> Result<CompileArtifact, EngineError>;

    /// 从项目目录编译为 DOCX
    pub fn compile_dir_to_docx(
        &self,
        project_root: &Path,
        main_tex: &Path,
        options: &CompileOptions,
    ) -> Result<CompileArtifact, EngineError>;

    /// 从 ZIP 包编译为 DOCX
    pub fn compile_zip_to_docx(
        &self,
        zip_bytes: &[u8],
        main_tex_path: &str,
        options: &CompileOptions,
    ) -> Result<CompileArtifact, EngineError>;

    /// 从 VFS 编译为 DOCX
    pub fn compile_vfs_to_docx(
        &self,
        main_tex: &str,
        vfs: &mut VirtualFs,
        options: &CompileOptions,
    ) -> Result<CompileArtifact, EngineError>;

    /// 从 VFS 编译为 DocumentGraph（中间表示）
    pub fn compile_vfs_to_graph(
        &self,
        main_tex: &str,
        vfs: &mut VirtualFs,
        options: &CompileOptions,
    ) -> Result<DocumentGraph, EngineError>;
}
```

**编译产物**：

```rust
pub struct CompileArtifact {
    pub docx_bytes: Vec<u8>,
    pub report: CompileReport,
}
```

**编译管线阶段**：

```
SourceMount
  ↓
JournalDetect (新增)
  ↓
CompatibilityAnalyze
  ↓
SemanticCollect (RuleBased / XeLaTeXHook / LuaTeXNode)
  ↓
RuleEngine
  ↓
ReferenceGraph
  ↓
DocxRender
  ↓
QualityGate
```

**使用示例**：

```rust
use doc_compiler_engine::{SemanticTexEngine, CompileOptions, EngineProfile};

let engine = SemanticTexEngine::new();
let options = CompileOptions {
    profile: EngineProfile::Tacl,
    semantic_backend: SemanticBackendKind::Auto,
    ..Default::default()
};

let result = engine.compile_dir_to_docx(
    project_root,
    main_tex,
    &options,
)?;

std::fs::write("output.docx", &result.docx_bytes)?;
serde_json::to_writer(
    std::fs::File::create("report.json")?,
    &result.report,
)?;
```

---

## 七、CompileReport（编译报告）

### 7.1 概述

CompileReport 记录编译全过程的详细信息，包括 Profile 检测、后端选择、兼容性分析和阶段状态。

### 7.2 实现状态：✅ DONE

**关键文件**：

- `crates/compiler-engine/src/lib.rs` — CompileReport 定义

**报告结构**：

```rust
pub struct CompileReport {
    pub profile: EngineProfile,                    // 使用的 Profile
    pub profile_spec: ProfileSpecReport,          // Profile 规格摘要
    pub backend: BackendSelectionReport,          // 后端选择详情
    pub compatibility: CompatibilityReport,        // 兼容性分析结果
    pub journal_detection: Option<JournalDetectionReport>, // 期刊检测结果
    pub stages: Vec<StageReport>,                 // 各阶段执行记录
    pub diagnostics: Vec<EngineDiagnostic>,       // 诊断信息
    // 统计计数
    pub block_count: usize,
    pub image_asset_count: usize,
    pub semantic_event_count: usize,
    pub layout_node_count: usize,
    pub sidecar_count: usize,
    pub reference_label_count: usize,
    pub reference_edge_count: usize,
    pub citation_count: usize,
    pub unresolved_reference_count: usize,
    pub bookmark_count: usize,
    pub hyperlink_count: usize,
    pub omml_equation_count: usize,
    pub omml_equation_fallback_count: usize,
    pub docx_bytes: usize,
    pub quality_gate: Option<QualityGateResult>,  // 质量门禁结果
}
```

**阶段报告**：

```rust
pub struct StageReport {
    pub stage: CompileStage,
    pub status: StageStatus,
    pub message: String,
}

pub enum CompileStage {
    SourceMount,
    JournalDetect,
    CompatibilityAnalyze,
    SourceParse,
    SemanticCollect,
    RuleEngine,
    ReferenceGraph,
    DocxRender,
    QualityGate,
}

pub enum StageStatus {
    Skipped,
    Running,
    Succeeded,
    Failed,
    Warning,
}
```

**BackendSelectionReport**：

```rust
pub struct BackendSelectionReport {
    pub requested: SemanticBackendKind,
    pub selected: SemanticBackendKind,
    pub fallback_used: bool,
    pub reason: String,
    pub runtime_available: RuntimeAvailabilitySnapshot,
}
```

**使用示例**：

```rust
let result = engine.compile_vfs_to_docx("main.tex", &mut vfs, &options)?;

println!("=== Compile Report ===");
println!("Profile: {:?}", result.report.profile);
println!("Backend: {:?}", result.report.backend.selected);
println!("Compatibility Score: {}", result.report.compatibility.score);
println!("DOCX Size: {} bytes", result.report.docx_bytes);

if let Some(qg) = &result.report.quality_gate {
    println!("Quality Gate: {} / {} passed", qg.passed_checks, qg.total_checks);
}
```

---

## 八、QualityGateResult（质量门禁结果）

### 8.1 概述

QualityGateResult 封装编译后的质量检查结果，提供 passed/total 统计和每个检查项的详情。

### 8.2 实现状态：✅ DONE

**关键文件**：

- `crates/compiler-engine/src/lib.rs` — QualityGateResult 定义

**数据结构**：

```rust
pub struct QualityGateResult {
    pub passed: bool,              // 所有检查是否通过
    pub total_checks: usize,       // 总检查数
    pub passed_checks: usize,     // 通过数
    pub failed_checks: Vec<QualityCheck>, // 失败检查列表
}

pub struct QualityCheck {
    pub name: String,             // 检查项名称
    pub passed: bool,             // 是否通过
    pub severity: QualitySeverity, // 严重级别
    pub message: String,          // 描述信息
}

pub enum QualitySeverity {
    Error,    // 阻塞级
    Warning,   // 警告级
    Info,      // 信息级
}
```

**内置检查项**：

| 检查项 | 失败级别 | 说明 |
|--------|----------|------|
| `compatibility_score` | Error | 兼容性分数 >= min_score |
| `unresolved_references` | Warning | 未解析引用数 == 0 |
| `omml_equation_fallback` | Warning | 非全量 OMML fallback |
| `docx_non_empty` | Error | DOCX 文件非空 |

**运行质量门禁**：

```rust
let mut report = CompileReport::new(options.profile);
// ... 执行编译管线 ...
report.run_quality_gate(min_score: 75);

if let Some(qg) = &report.quality_gate {
    if !qg.passed {
        for check in &qg.failed_checks {
            eprintln!("[{:?}] {}: {}", check.severity, check.name, check.message);
        }
    }
}
```

**测试验证**：

```bash
cargo test -p doc-compiler-engine quality_gate
cargo test -p doc-compiler-engine
# 82 tests passed, 1 ignored
```

---

## 九、ProfileStyleMap（Profile 样式映射）

### 9.1 概述

ProfileStyleMap 定义从语义角色到具体 DOCX 样式 ID 的映射，用于不同期刊模板生成符合预期的 Word 样式。

### 9.2 实现状态：✅ DONE

**关键文件**：

- `crates/docx-writer/src/profile.rs` — ProfileStyleMap 实现

**数据结构**：

```rust
pub struct ProfileStyleMap {
    /// Map from role name (e.g. "body", "heading1") to style ID (e.g. "BodyText")
    #[serde(flatten)]
    pub by_role: BTreeMap<String, String>,
}

impl ProfileStyleMap {
    /// Journal of Software (软件学报) paper style map
    pub fn jos() -> Self;

    /// Generic article style map (Word built-in styles)
    pub fn generic() -> Self;

    /// Look up the style ID for a given role
    pub fn get(&self, role: &str) -> Option<&str>;
}
```

**JOS Profile 样式映射**（25 个角色）：

| 语义角色 | DOCX 样式 ID |
|----------|-------------|
| `body` | `JOSBody` |
| `heading1` | `JOSHeading1` |
| `heading2` | `JOSHeading2` |
| `heading3` | `JOSHeading3` |
| `caption` | `JOSCaption` |
| `citation` | `JOSCitation` |
| `reference` | `JOSReference` |
| `table_header` | `TableHeader` |
| `table_text` | `JOSTableText` |
| `title_zh` | `JOSTitleZh` |
| `author_zh` | `JOSAuthorZh` |
| `abstract_zh` | `JOSAbstractZh` |
| `keywords` | `JOSKeywords` |
| ... | ... |

**Generic Profile 样式映射**（4 个角色）：

| 语义角色 | DOCX 样式 ID |
|----------|-------------|
| `body` | `BodyText` |
| `heading1` | `Heading1` |
| `heading2` | `Heading2` |
| `heading3` | `Heading3` |

**通过 CompileOptions 注入**：

```rust
let options = CompileOptions {
    style_map: Some(doc_docx_writer::ProfileStyleMap::jos()),
    ..Default::default()
};
```

**通过 ProfileSpec 透传**：

CompileOptions.effective_style_map() 优先使用显式设置，否则使用 `profile.spec().style_map`。

---

## 十、CompatibilityAnalyzer（兼容性分析器）

### 10.1 概述

CompatibilityAnalyzer 扫描 TeX 源码中的 document class、package、环境和自定义宏，输出兼容性评分和问题列表。

### 10.2 实现状态：✅ DONE

**关键文件**：

- `crates/compatibility-analyzer/src/lib.rs` — CompatibilityAnalyzer 实现

**核心类型**：

```rust
pub struct CompatibilityReport {
    pub score: u8,                        // 0-100 兼容性分数
    pub scanned_files: usize,             // 扫描文件数
    pub document_classes: Vec<String>,    // 检测到的文档类
    pub packages: Vec<String>,            // 检测到的宏包
    pub custom_macro_count: usize,        // 自定义宏数量
    pub unsupported: Vec<CompatibilityIssue>, // 不支持的功能
    pub warnings: Vec<CompatibilityIssue>,    // 警告项
}

pub struct CompatibilityIssue {
    pub code: String,     // 机器可读代码，如 "unsupported_package"
    pub feature: String,  // 功能名，如 "minted"
    pub message: String,  // 人类可读描述
}
```

**ProfileKind 扩展**（支持 9 个 Profile）：

```rust
pub enum ProfileKind {
    Generic,
    GenericArticle,
    ChineseAcademic,
    JosPaper,
    Tacl,
    Cvpr,
    Nature,
    Springer,
    MedicalJournal,
}

impl ProfileKind {
    pub fn supports_document_class(&self, class: &str) -> bool;
    pub fn name(&self) -> &'static str;
    pub fn from_id(id: &str) -> Option<Self>;
}
```

**Profile-aware 包兼容性**：

| Profile | 强支持 | 警告 | 不支持 |
|---------|--------|------|--------|
| jos-paper | IEEEtran, amsmath, graphicx | algorithm2e, tabularx | — |
| tacl | acl, natbib | biblatex, tikz | — |
| cvpr | IEEEtran[conference], amsmath | algorithmicx, subcaption | — |
| nature | nature, natbib | biblatex | pstricks |
| springer | springer, svjour3, llncs | algorithm2e, longtable | beamer |
| chinese-academic | ctex, xeCJK, fontspec | gbt7714, biblatex | minted |

**测试验证**：

```bash
cargo test -p doc-compatibility-analyzer
# 14 tests passed
```

---

## 十一、RuleEngine（规则引擎）

### 11.1 概述

RuleEngine 提供宏的语义解释规则，将未知宏映射为 RuleOutput，并通过 `journal_rules(profile_id)` 提供期刊特定宏规则。

### 11.2 实现状态：✅ DONE

**关键文件**：

- `crates/rule-engine/src/rule_output.rs` — RuleOutput 扩展 enum
- `crates/rule-engine/src/builtin_rules.rs` — builtin_rules() + journal_rules()

**RuleOutput 扩展变体**（新增 5 个）：

```rust
pub enum RuleOutput {
    // 原有 7 个变体 ...
    Citation { keys_arg: usize, style: String },
    MetadataField { key: String, content_arg: usize },
    AuthorList { content_arg: usize },
    Affiliation { content_arg: usize },
    KeywordList { content_arg: usize, separator: String },
}
```

**journal_rules(profile_id) 返回的期刊宏规则**：

| Profile | 注册的宏 |
|---------|----------|
| jos-paper | `\IEEEauthorblockN`, `\IEEEauthorblockA`, `\IEEEkeywords`, `\markboth`, `\citet`, `\citep` |
| tacl | `\aclfinalcopy`, `\aclpaperid`, `\citet`, `\citep`, `\citealp`, `\shorttitle`, `\name`, `\address` |
| cvpr | `\cvprfinalcopy`, `\iccvfinalcopy`, `\cvprPaperID`, `\confName`, `\confYear`, `\author`, `\affiliation` |
| nature | `\corres`, `\equalcont`, `\affil`, `\maketitle` |
| springer | `\institute`, `\titlerunning`, `\authorrunning`, `\email`, `\orcidID`, `\keywords` |
| chinese-academic | `\zihao`, `\songti`, `\heiti`, `\kaishu`, `\fangsong`, `\CTEXsetup`, `\ctexset`, `\keywords`, `\zhabstract`, `\enabstract` |

**使用示例**：

```rust
use doc_rule_engine::{builtin_rules, journal_rules, RuleEngine};

let profile_id = "tacl";
let mut engine = RuleEngine::new();

// 加载内置规则
for rule in builtin_rules() {
    engine.registry_mut().register(rule);
}

// 加载期刊规则
for rule in journal_rules(profile_id) {
    engine.registry_mut().register(rule);
}
```

**测试验证**：

```bash
cargo test -p doc-rule-engine
# 23 tests passed
```

---

## 十二、CLI — doc-engine 命令行工具

### 12.1 概述

`doc-engine` 是 Semantic TeX Engine 的官方命令行入口，提供 8 个子命令，涵盖转换、编译、验证和诊断。

### 12.2 实现状态：✅ DONE

**关键文件**：

- `crates/cli/src/main.rs` — 主程序和子命令路由
- `crates/cli/src/semantic_cmd.rs` — 语义子命令参数
- `crates/cli/src/cmd.rs` — 转换和构建参数
- `crates/cli/src/tex_compile.rs` — TeX 编译参数
- `crates/cli/src/docx2pdf.rs` — DOCX 转 PDF 参数
- `crates/cli/src/pdf_verify.rs` — PDF 验证参数
- `crates/cli/src/ast_dump.rs` — AST 转储参数
- `crates/cli/src/render_dump.rs` — 渲染转储参数
- `crates/cli/src/docx_diff.rs` — DOCX 对比参数

### 12.3 子命令列表

#### 12.3.1 convert — ZIP 转 DOCX（V1 路径）

```bash
doc-engine convert \
  --zip paper.zip \
  --main-tex main.tex \
  --out output.docx \
  --page-setup a4 \
  --header-text "Tex2Doc"
```

#### 12.3.2 tex-compile — TeX 编译

```bash
doc-engine tex-compile \
  --project-root . \
  --main-tex main.tex \
  --backend xelatex
```

#### 12.3.3 docx-to-pdf — DOCX 转 PDF

```bash
doc-engine docx-to-pdf \
  --docx output.docx \
  --out output.pdf
```

#### 12.3.4 verify-pdf — PDF 验证

```bash
doc-engine verify-pdf \
  --pdf output.pdf \
  --reference reference.pdf
```

#### 12.3.5 build — 完整流水线（ZIP → DOCX → PDF → 验证）

```bash
doc-engine build \
  --zip paper.zip \
  --main-tex main.tex \
  --outdir ./build \
  --skip-visual
```

#### 12.3.6 ast-dump — AST 转储

```bash
doc-engine ast-dump \
  --project-root . \
  --main-tex main.tex \
  --out ast.json
```

#### 12.3.7 render-dump — 渲染转储

```bash
doc-engine render-dump \
  --project-root . \
  --main-tex main.tex \
  --out render.json
```

#### 12.3.8 docx-diff — DOCX 对比

```bash
doc-engine docx-diff \
  --baseline baseline.docx \
  --candidate candidate.docx \
  --out diff-report.html
```

### 12.4 语义子命令（Semantic-*）

#### 12.4.1 semantic-detect — 期刊 Profile 检测

```bash
doc-engine semantic-detect \
  --project-root examples/journals/tacl \
  --main-tex minimal.tex \
  --output tacl-detection.json
```

**输出示例**：

```
profile: tacl
confidence: 0.95

Diagnostics:
  [Info] profile detected with high confidence

All candidates:
  tacl: 0.95
  generic: 0.10
  jos-paper: 0.05
```

#### 12.4.2 semantic-analyze — 兼容性分析

```bash
doc-engine semantic-analyze \
  --project-root . \
  --main-tex main.tex \
  --profile tacl \
  --output compatibility.json
```

#### 12.4.3 semantic-convert — 语义引擎转换

```bash
doc-engine semantic-convert \
  --project-root examples/journals/tacl \
  --main-tex minimal.tex \
  --profile auto \
  --backend auto \
  --out output/tacl.docx \
  --report output/tacl.report.json
```

**参数说明**：

| 参数 | 说明 | 默认值 |
|------|------|--------|
| `--project-root` | TeX 项目根目录 | 必需 |
| `--main-tex` | 主 .tex 文件 | main.tex |
| `--profile` | Profile ID 或 `auto` | auto |
| `--backend` | 语义后端 | auto |
| `--out` | 输出 DOCX 路径 | 必需 |
| `--report` | 报告 JSON 路径 | 可选 |
| `--no-backend-fallback` | 禁止后端回退 | false |

#### 12.4.4 semantic-verify — DOCX 质量验证

```bash
doc-engine semantic-verify \
  --docx-file output/tacl.docx \
  --report output/quality.json
```

> 注意：`semantic-verify` 当前标记为 P6 milestone 尚未完全实现。

---

## 十三、commercial_api_client（商业 API 客户端）

### 13.1 概述

commercial_api_client 是面向 Tex2Doc 商业化 SaaS 的 Rust HTTP 客户端，支持 multipart 上传、任务轮询和响应模型。

### 13.2 实现状态：✅ DONE

**关键文件**：

- `crates/commercial-api-client/src/client.rs` — ApiClient 实现
- `crates/commercial-api-client/src/models.rs` — 请求/响应类型

### 13.3 ApiClient 核心 API

```rust
pub struct ClientConfig {
    pub base_url: url::Url,      // 默认 https://api.tex2doc.cn/v1
    pub api_key: String,
    pub timeout: Duration,        // 默认 30s
}

pub struct ApiClient {
    config: ClientConfig,
    http: Client,
}

impl ApiClient {
    pub fn new(config: ClientConfig) -> Result<Self, ApiError>;
    pub fn from_api_key(api_key: impl Into<String>) -> Result<Self, ApiError>;

    /// 提交 DOCX 进行质量分析（multipart upload）
    pub async fn submit_analysis(&self, docx: &[u8]) -> Result<AnalysisJob, ApiError>;

    /// 轮询任务状态
    pub async fn poll_job(&self, job_id: &str) -> Result<AnalysisResult, ApiError>;

    /// 获取详细报告
    pub async fn get_report(&self, job_id: &str) -> Result<DetailedReport, ApiError>;
}
```

### 13.4 请求/响应模型

```rust
// 请求
pub struct SubmitRequest {
    pub callback_url: Option<String>,
}

// 任务状态
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

// 任务
pub struct AnalysisJob {
    pub job_id: String,
    pub status: JobStatus,
    pub created_at: String,
}

// 分析结果
pub struct AnalysisResult {
    pub job_id: String,
    pub status: JobStatus,
    pub report: Option<DetailedReport>,
    pub error: Option<String>,
}

// 详细报告
pub struct DetailedReport {
    pub overall_score: f32,
    pub structural_checks: Vec<CheckResult>,
    pub style_checks: Vec<CheckResult>,
    pub reference_checks: Vec<CheckResult>,
}

// 单项检查结果
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub score: f32,
    pub message: String,
}

// 错误类型
pub enum ApiError {
    Transport(String),
    Http { status: StatusCode, body: String },
    Url(ParseError),
    Decode(String),
    Api { code: String, message: String },
}
```

### 13.5 使用示例

```rust
use doc_commercial_api_client::{ApiClient, ApiError};

#[tokio::main]
async fn main() -> Result<(), ApiError> {
    let client = ApiClient::from_api_key("your-api-key")?;

    // 提交分析
    let docx = std::fs::read("output.docx")?;
    let job = client.submit_analysis(&docx).await?;
    println!("Job created: {}", job.job_id);

    // 轮询直到完成
    loop {
        let result = client.poll_job(&job.job_id).await?;
        match result.status {
            JobStatus::Completed => {
                if let Some(report) = result.report {
                    println!("Overall score: {:.1}", report.overall_score);
                }
                break;
            }
            JobStatus::Failed => {
                eprintln!("Analysis failed: {:?}", result.error);
                break;
            }
            _ => {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    }

    Ok(())
}
```

### 13.6 特性

- **multipart upload**：DOCX 通过 multipart/form-data 上传
- **Job polling**：支持轮询任务状态直到完成
- **错误处理**：区分 Transport、HTTP、Decode、API 四类错误
- **超时控制**：可配置的请求超时（默认 30s）

---

## 十四、desktop_slint（Slint PC 客户端骨架）

### 14.1 概述

desktop_slint 是基于 Rust + Slint 的跨平台桌面客户端骨架，面向 Windows/macOS/Linux，用于商业化发布。

### 14.2 实现状态：✅ DONE（骨架阶段）

**关键文件**：

- `crates/desktop-slint/src/main.rs` — 入口
- `crates/desktop-slint/src/ui/main.slint` — UI 定义
- `crates/desktop-slint/src/ui/mod.rs` — UI 模块
- `crates/desktop-slint/Cargo.toml` — 依赖配置
- `crates/desktop-slint/build.rs` — Slint 构建脚本

### 14.3 工程结构

```
crates/desktop-slint/
├── Cargo.toml
├── build.rs
└── src/
    ├── main.rs
    └── ui/
        ├── mod.rs
        └── main.slint
```

### 14.4 Cargo.toml

```toml
[package]
name = "doc-desktop-slint"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies]
slint = "1"
doc-core = { path = "../core" }
doc-compiler-engine = { path = "../compiler-engine" }

[build-dependencies]
slint-build = "1"
```

### 14.5 main.rs（入口）

```rust
use slint::include_modules;

include_modules!();

fn main() {
    let ui = MainWindow::new().unwrap();
    ui.run().unwrap();
}
```

### 14.6 main.slint（UI 骨架）

```slint
import { Button, VerticalBox, LineEdit, TextEdit, GridBox } from "std-widgets.slint";

export component MainWindow inherits Window {
    in-out property <string> project-path;
    in-out property <string> status;
    
    GridBox {
        padding: 24px;
        spacing: 12px;
        
        Text {
            text: "Tex2Doc Desktop";
            font-size: 24px;
            font-weight: 700;
            row: 0;
            colspan: 2;
        }
        
        LineEdit {
            placeholder-text: "TeX project path...";
            text <=> project-path;
            row: 1;
            colspan: 2;
        }
        
        Button {
            text: "Detect Profile";
            clicked => { 
                status = "Detecting...";
            }
            row: 2;
            colspan: 2;
        }
        
        TextEdit {
            text <=> status;
            read-only: true;
            min-height: 200px;
            row: 3;
            colspan: 2;
        }
    }
}
```

### 14.7 当前状态

- ✅ Slint 依赖和构建配置完成
- ✅ 主窗口 UI 骨架实现（路径输入、检测按钮、状态文本）
- ⏳ 业务逻辑（文件选择、Profile 检测、转换）待实现
- ⏳ 账号系统、商业 API 集成待实现

### 14.8 后续计划

详细商业化路线图见 `docs-zh/semantic-tex-engine-pc-client-slint-commercial-plan-20260621-152833.md`。

---

## 十五、scripts/verify_journal_profiles.sh（期刊 Profile 验证脚本）

### 15.1 概述

`verify_journal_profiles.sh` 是端到端验证脚本，对 7 个期刊 Profile 执行自动检测、兼容性分析、规则引擎验证、后端选择和 DOCX 生成。

### 15.2 实现状态：✅ DONE

**关键文件**：

- `scripts/verify_journal_profiles.sh` — 验证脚本

### 15.3 使用方法

```bash
# 验证所有 7 个期刊 profile
./scripts/verify_journal_profiles.sh

# 验证特定 profile
./scripts/verify_journal_profiles.sh --profile-id cvpr

# 跳过 DOCX 生成（仅运行单元测试）
./scripts/verify_journal_profiles.sh --skip-docx

# 跳过 TeX runtime 步骤（CI 环境友好）
./scripts/verify_journal_profiles.sh --skip-runtime

# 组合使用
./scripts/verify_journal_profiles.sh --profile-id tacl --skip-docx
```

### 15.4 验证流程

脚本对每个 Profile 执行 5 个验证步骤：

1. **Journal Detection**：`cargo test -p doc-compiler-engine journal_detector`
2. **Compatibility Analysis**：`cargo test -p doc-compatibility-analyzer`
3. **Rule Engine**：`cargo test -p doc-rule-engine`
4. **Profile-aware Backend Selection**：profile_aware / backend_selector 测试
5. **DOCX Generation**：调用 `paper3_to_docx` 示例生成 DOCX

### 15.5 输出

- **控制台输出**：每个步骤的 PASS/FAIL 状态
- **JSON 摘要**：`examples/journals/output/verify-summary.json`

```json
{
  "version": "1.0",
  "project_root": "/home/ros/work/Tex2Doc",
  "profiles": {
    "jos-paper": "passed",
    "tacl": "passed",
    "cvpr": "passed",
    "nature": "passed",
    "springer": "passed",
    "chinese-academic": "passed",
    "generic": "passed"
  },
  "summary": {
    "total": 7,
    "passed": 7,
    "failed": 0,
    "skipped_docx": false,
    "skipped_runtime": false
  }
}
```

### 15.6 退出码

| 退出码 | 含义 |
|--------|------|
| 0 | 所有 Profile 验证通过 |
| 1 | 一个或多个 Profile 验证失败 |

---

## 十六、scripts/commercial_verify.sh（商业化质量门禁脚本）

### 16.1 概述

`commercial_verify.sh` 是商业化部署的质量门禁脚本，对 DOCX 文件执行结构、样式和引用检查。

### 16.2 实现状态：✅ DONE

**关键文件**：

- `scripts/commercial_verify.sh` — 质量门禁脚本

### 16.3 使用方法

```bash
# 基本用法
./scripts/commercial_verify.sh --docx output.docx

# 指定最低分数
./scripts/commercial_verify.sh --docx output.docx --min-score 80

# 输出 JSON 报告
./scripts/commercial_verify.sh \
  --docx output.docx \
  --min-score 75 \
  --report quality-report.json

# 跳过特定检查
./scripts/commercial_verify.sh \
  --docx output.docx \
  --skip-structural \
  --skip-style
```

### 16.4 检查项

#### 结构检查（Structural）

| 检查项 | 条件 | 级别 |
|--------|------|------|
| `file_size` | 文件大小 > 1024 bytes | 通过/失败 |

#### 样式检查（Style）

| 检查项 | 条件 | 级别 |
|--------|------|------|
| `styles_present` | `word/styles.xml` 存在 | 通过/失败 |
| `document_present` | `word/document.xml` 存在 | 通过/失败 |

#### 引用检查（References）

| 检查项 | 条件 | 级别 |
|--------|------|------|
| `rels_present` | `word/_rels/document.xml.rels` 存在 | 通过/失败 |

### 16.5 输出报告格式

```json
{
  "version": "1.0",
  "docx": "output.docx",
  "min_score": 75,
  "results": {
    "file_size": "pass",
    "styles_present": "pass",
    "document_present": "pass",
    "rels_present": "pass"
  },
  "summary": { "passed": 4, "failed": 0 }
}
```

### 16.6 退出码

| 退出码 | 含义 |
|--------|------|
| 0 | 所有检查通过 |
| 1 | 一个或多个检查失败 |

---

## 十七、回归测试总览

### 17.1 测试统计

| Crate | 测试数 | 状态 |
|-------|--------|------|
| `doc-compiler-engine` | 82 passed, 1 ignored | ✅ |
| `doc-compatibility-analyzer` | 14 passed | ✅ |
| `doc-rule-engine` | 23 passed | ✅ |
| **总计** | **119 passed** | ✅ |

### 17.2 测试命令

```bash
# 运行所有测试
cargo test -p doc-compiler-engine
cargo test -p doc-compatibility-analyzer
cargo test -p doc-rule-engine

# 运行特定模块测试
cargo test -p doc-compiler-engine profiles
cargo test -p doc-compiler-engine journal_detector
cargo test -p doc-compiler-engine quality_gate
cargo test -p doc-compiler-engine profile_aware
cargo test -p doc-compiler-engine backend_selector

# 运行所有 workspace 测试
cargo test --workspace
```

---

## 十八、Implemented Commands（已实现的命令）

### 18.1 doc-engine 主命令

```bash
doc-engine --help
```

输出：

```
doc-engine 0.x.y
Doc-engine CLI (V2 Tex2Doc engine)

USAGE:
  doc-engine <COMMAND>

COMMANDS:
  convert          ZIP 转 DOCX（V1 路径）
  tex-compile      TeX 编译
  docx-to-pdf      DOCX 转 PDF
  verify-pdf       PDF 验证
  build            完整流水线
  ast-dump         AST 转储
  render-dump      渲染转储
  docx-diff        DOCX 对比
  semantic-detect  检测 TeX 项目的期刊 Profile
  semantic-analyze 分析 TeX 项目的兼容性
  semantic-convert TeX 项目转换为 DOCX（Semantic Engine）
  semantic-verify  验证 DOCX 质量（结构 / 引用 / 样式）
  help             打印此帮助信息或某个命令的帮助
```

### 18.2 语义子命令详情

#### semantic-detect

```bash
doc-engine semantic-detect --help
```

```
--project-root <PROJECT_ROOT>  TeX 项目根目录
--main-tex <MAIN_TEX>         主 .tex 文件相对路径 [默认: main.tex]
--output <OUTPUT>             输出 JSON 报告路径
```

#### semantic-analyze

```bash
doc-engine semantic-analyze --help
```

```
--project-root <PROJECT_ROOT>  TeX 项目根目录
--main-tex <MAIN_TEX>         主 .tex 文件 [默认: main.tex]
--profile <PROFILE>           Profile 类型 [默认: generic]
--output <OUTPUT>              输出 JSON 报告路径
```

#### semantic-convert

```bash
doc-engine semantic-convert --help
```

```
--project-root <PROJECT_ROOT>   TeX 项目根目录
--main-tex <MAIN_TEX>          主 .tex 文件
--profile <PROFILE>             Profile ID [默认: auto]
--backend <BACKEND>            语义后端 [默认: auto]
--out <OUT>                    输出 DOCX 路径
--report <REPORT>              输出报告 JSON 路径
--no-backend-fallback           不允许后端回退
```

#### semantic-verify

```bash
doc-engine semantic-verify --help
```

```
--docx-file <DOCX_FILE>  DOCX 文件路径
--report <REPORT>         报告 JSON 路径
```

> 注意：`semantic-verify` 当前返回 "not yet implemented (P6 milestone)"。

---

## 十九、关键文件索引

### 19.1 核心库

| 文件 | 描述 |
|------|------|
| `crates/compiler-engine/src/lib.rs` | SemanticTexEngine、CompileOptions、CompileReport、QualityGateResult、EngineProfile |
| `crates/compiler-engine/src/profiles.rs` | ProfileRegistry、ProfileSpecFile、TOML 加载 |
| `crates/compiler-engine/src/journal_detector.rs` | JournalDetector、SignalKind、JournalDetectionReport |
| `crates/compatibility-analyzer/src/lib.rs` | CompatibilityAnalyzer、ProfileKind、CompatibilityReport |
| `crates/rule-engine/src/rule_output.rs` | RuleOutput 扩展 enum |
| `crates/rule-engine/src/builtin_rules.rs` | builtin_rules()、journal_rules() |
| `crates/docx-writer/src/profile.rs` | ProfileStyleMap（jos()、generic()） |
| `crates/semantic-collector/src/lib.rs` | SemanticBackendKind、SemanticEvent、ReferenceGraph |

### 19.2 CLI

| 文件 | 描述 |
|------|------|
| `crates/cli/src/main.rs` | doc-engine 主程序、8 个子命令路由 |
| `crates/cli/src/semantic_cmd.rs` | SemanticDetect/Analyze/Convert/Verify 参数 |
| `crates/cli/src/cmd.rs` | Convert、Build 参数 |
| `crates/cli/src/tex_compile.rs` | TexCompile 参数 |
| `crates/cli/src/docx2pdf.rs` | DocxToPdf 参数 |
| `crates/cli/src/pdf_verify.rs` | VerifyPdf 参数 |
| `crates/cli/src/ast_dump.rs` | AstDump 参数 |
| `crates/cli/src/render_dump.rs` | RenderDump 参数 |
| `crates/cli/src/docx_diff.rs` | DocxDiff 参数 |

### 19.3 商业化

| 文件 | 描述 |
|------|------|
| `crates/commercial-api-client/src/client.rs` | ApiClient（multipart upload、job polling） |
| `crates/commercial-api-client/src/models.rs` | 请求/响应类型、JobStatus |
| `crates/desktop-slint/src/main.rs` | Slint 入口 |
| `crates/desktop-slint/src/ui/main.slint` | 主窗口 UI 定义 |

### 19.4 脚本

| 文件 | 描述 |
|------|------|
| `scripts/verify_journal_profiles.sh` | 7 个期刊 Profile E2E 验证 |
| `scripts/commercial_verify.sh` | 商业化质量门禁 |

### 19.5 TOML Profile 文件

| 文件 | Profile ID |
|------|-----------|
| `crates/compiler-engine/profiles/generic.toml` | generic-article |
| `crates/compiler-engine/profiles/jos-paper-toml.toml` | jos-paper |
| `crates/compiler-engine/profiles/tacl.toml` | tacl |
| `crates/compiler-engine/profiles/cvpr.toml` | cvpr |
| `crates/compiler-engine/profiles/nature.toml` | nature |
| `crates/compiler-engine/profiles/springer.toml` | springer |
| `crates/compiler-engine/profiles/chinese-academic.toml` | chinese-academic |

### 19.6 示例 Fixture

| 目录 | DocumentClass | Profile |
|------|-------------|---------|
| `examples/journals/jos-paper/minimal.tex` | `\documentclass[journal]{IEEEtran}` | jos-paper |
| `examples/journals/tacl/minimal.tex` | `\documentclass[aclang]{acl}` | tacl |
| `examples/journals/cvpr/minimal.tex` | `\documentclass[conference]{IEEEtran}` | cvpr |
| `examples/journals/nature/minimal.tex` | `\documentclass{nature}` | nature |
| `examples/journals/springer/minimal.tex` | `\documentclass{springer}` | springer |
| `examples/journals/chinese-academic/minimal.tex` | `\documentclass{ctexart}` | chinese-academic |
| `examples/journals/generic/minimal.tex` | `\documentclass{article}` | generic |

---

## 二十、文档导航

| 文档 | 描述 |
|------|------|
| `docs-zh/semantic-tex-engine-journal-profile-generalization-plan-20260621-125025.md` | 期刊 Profile 泛化能力改进方案 |
| `docs-zh/semantic-tex-engine-journal-profile-generalization-progress-20260621.md` | 期刊 Profile 泛化开发进展报告 |
| `docs-zh/semantic-tex-engine-commercialization-technical-implementation-plan-20260621-151221.md` | 商业化技术实现方案 |
| `docs-zh/semantic-tex-engine-pc-client-slint-commercial-plan-20260621-152833.md` | PC 客户端商业化技术方案 |
| `docs-zh/P1-DEV-PLAN.md` | 本文档 — P1 开发计划（实施版） |

---

**文档更新日期**：2026-06-21
**维护者**：Tex2Doc 开发团队
