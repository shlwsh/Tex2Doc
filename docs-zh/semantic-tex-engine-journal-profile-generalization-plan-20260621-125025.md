# Semantic TeX Engine 期刊 Profile 泛化能力改进方案
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



**方案版本**：20260621-125025  
**适用范围**：新语义引擎路径，不影响原有 Rust 版本 doc 转换引擎  
**目标阶段**：首期期刊模板泛化支持  
**状态**：待审核，审核通过后进入开发  

---

## 一、背景与目标

当前项目已经具备新语义引擎的基础框架：

- `compiler-engine` 已提供 `RuleBased`、`XeLaTeXHook`、`LuaTeXNode` 三类语义采集路径。
- `semantic-collector` 已定义统一语义事件、布局图、引用图和 backend 类型。
- `compatibility-analyzer` 已能扫描 document class、package、环境和自定义宏。
- `rule-engine` 已提供未知宏规则处理和后续 AI fallback 扩展点。
- `compiler-engine/profiles` 已有 JSON/TOML Profile 加载雏形。

但当前泛化能力仍偏“单项目/少数模板适配”，主要问题是：

1. Profile 类型仍以 Rust enum 为主，外置规则只是辅助，无法完整承载期刊模板规则。
2. 自动检测只关注 XeTeX/LuaTeX 特征，没有独立的期刊模板检测层。
3. 兼容性分析的 `ProfileKind` 粒度太粗，只区分 generic、中文学术、JOS、医学。
4. RuleEngine 能处理未知宏，但尚未按期刊 Profile 加载宏映射规则。
5. LuaTeX/XeLaTeX runtime hook 采集的是通用事件，还没有根据模板 Profile 增强特定宏。
6. DOCX 输出层缺少 profile-driven style mapping、caption policy、reference policy 和降级策略。

本方案目标是将新语义引擎升级为：

```text
TeX Source
  ↓
JournalDetector
  ↓
ProfileRegistry
  ↓
CompatibilityAnalyzer(profile-aware)
  ↓
Semantic Backend Selector
  ↓
RuleBased / XeLaTeXHook / LuaTeXNode
  ↓
Profile-aware RuleEngine
  ↓
Semantic AST / DocumentGraph
  ↓
Profile-aware DOCX Renderer
```

首期必须支持以下期刊或模板的泛化处理：

| 期刊/模板 | 检测信号 | Profile ID |
|---|---|---|
| IEEE JOS | `\documentclass[journal]{IEEEtran}` | `jos-paper` |
| ACL/TACL | `\documentclass[aclang]{acl}` | `tacl` |
| CVPR/ICCV | `\documentclass[conference]{IEEEtran}` | `cvpr` |
| Nature | `\documentclass{nature}` | `nature` |
| Springer | `\documentclass{springer}` | `springer` |
| 中文学报 | `\documentclass{ctexart}` | `chinese-academic` |
| arXiv | 无特定 / 任意 | `generic` |

---

## 二、设计原则

### 2.1 新旧路径隔离

本次改进只作用于新语义引擎：

- 保留原有 Rust 规则版 doc 转换引擎。
- 不修改旧路径对 paper3 的转换行为。
- 新 Profile、检测器、规则 registry 归属于 `compiler-engine`、`semantic-collector`、`compatibility-analyzer`、`rule-engine` 这一组新语义引擎 crate。
- 如需复用旧路径组件，只通过复制、适配器或只读调用方式进行，不把旧路径逻辑反向绑定到新路径。

### 2.2 Profile 是期刊泛化的核心

Profile 不应只是页面尺寸或字体配置，而应承载：

- 模板检测信号。
- 编译后端偏好。
- 宏包兼容性规则。
- 文档结构规则。
- caption、citation、reference 策略。
- DOCX style mapping。
- 未知宏处理规则。
- 降级策略。
- 验收阈值。

### 2.3 语义优先，版式补充

首期不追求从 XDV 或 node tree 完整反推语义。正确路线是：

```text
源码规则解析提供稳定语义
LuaTeX/XeLaTeX hook 补充运行期事件
XDV/LuaTeX node tree 提供布局校准
Profile 决定如何解释和渲染这些信息
```

### 2.4 generic 永远可用

当无法识别期刊模板，或者检测置信度不足时，必须降级到 `generic`，而不是失败。

---

## 三、当前实现基线

### 3.1 已有能力

| 模块 | 当前能力 | 泛化缺口 |
|---|---|---|
| `compiler-engine/src/lib.rs` | 统一编译入口、三后端选择、引用图、报告 | 自动选择只看 TeX runtime 特征，不识别期刊模板 |
| `compiler-engine/src/profiles.rs` | JSON/TOML Profile 加载，内置 generic/chinese/JOS/medical | schema 太薄，无法描述模板检测、宏规则、style mapping |
| `compatibility-analyzer` | 扫描 class/package/environment/custom macro | ProfileKind 太粗，缺少 tacl/cvpr/nature/springer |
| `rule-engine` | builtin rule + unknown macro fallback | 未按 Profile 加载规则，缺少期刊宏语义映射 |
| `semantic-collector` | 统一语义事件、LayoutGraph、ReferenceGraph | 事件类型需扩展 profile、confidence、origin |
| `LuaTeXNodeBackend` | 采集 paragraph、heading、引用、图片、equation 和 node tree | 还不是完整宏展开语义采集 |
| `XeLaTeXHookBackend` | 采集 label/ref/cite/includegraphics | hook 覆盖面较窄，仍依赖源码 parser |

### 3.2 不足结论

当前架构具备泛化扩展点，但首期泛化能力尚不充分。真正需要补齐的是：

1. 期刊检测器。
2. 可扩展 Profile schema。
3. Profile-aware 兼容性分析。
4. Profile-aware 规则引擎。
5. Profile-aware 后端选择。
6. Profile-aware DOCX 渲染策略。
7. 期刊 fixture 和验收指标。

---

## 四、首期目标 Profile 设计

### 4.1 Profile ID 规范

首期使用以下稳定 ID：

```text
jos-paper
tacl
cvpr
nature
springer
chinese-academic
generic
```

当前已有 `generic-article`，建议做兼容别名：

```text
generic -> generic-article
generic-article -> generic
```

内部报告和 CLI 输出优先展示 `generic`。

### 4.2 期刊检测规则

新增 `JournalDetector`，负责从 TeX 源码中抽取：

- `\documentclass` 名称。
- `\documentclass[...]` options。
- `\usepackage` / `\RequirePackage`。
- 模板特有宏。
- bibliography 风格。
- CJK/XeTeX/LuaTeX 运行期特征。

首期检测矩阵：

| Profile ID | document class | options | 辅助信号 | 默认后端 |
|---|---|---|---|---|
| `jos-paper` | `IEEEtran` | `journal` | `\IEEEauthorblockN`、`\markboth`、IEEE citation | `LuaTeXNode`，失败后 `XeLaTeXHook` |
| `tacl` | `acl` | `aclang` | `\aclfinalcopy`、`\aclpaperid`、`acl_natbib` | `LuaTeXNode` |
| `cvpr` | `IEEEtran` | `conference` | `cvpr`、`iccv`、`\cvprfinalcopy`、`\iccvfinalcopy` | `LuaTeXNode` |
| `nature` | `nature` | 任意 | `\bibliographystyle{naturemag}`、`nature` class | `LuaTeXNode` |
| `springer` | `springer` | 任意 | `svjour3`、`llncs`、`\institute`、`\titlerunning` | `LuaTeXNode` |
| `chinese-academic` | `ctexart` | 任意 | `ctex`、`xeCJK`、`\setCJKmainfont` | `XeLaTeXHook` |
| `generic` | 任意 | 任意 | 无强信号 | `LuaTeXNode`，失败后 `RuleBased` |

注意：用户给出的 Springer 检测信号是 `\documentclass{springer}`，首期必须支持该精确信号；同时建议兼容 `svjour3`、`llncs`，便于后续扩展。

### 4.3 检测置信度

新增：

```rust
pub struct JournalDetection {
    pub profile_id: String,
    pub confidence: f32,
    pub matched_signals: Vec<MatchedSignal>,
    pub fallback: bool,
}

pub struct MatchedSignal {
    pub kind: SignalKind,
    pub value: String,
    pub weight: f32,
    pub source_path: String,
    pub line: Option<usize>,
}

pub enum SignalKind {
    DocumentClass,
    DocumentClassOption,
    Package,
    Macro,
    BibliographyStyle,
    EngineFeature,
}
```

建议权重：

| 信号 | 权重 |
|---|---:|
| documentclass 精确匹配 | 0.70 |
| documentclass option 匹配 | 0.20 |
| 模板特有宏匹配 | 0.10 |
| bibliography style 匹配 | 0.05 |
| package 辅助匹配 | 0.05 |

选择策略：

```text
score >= 0.75: 自动选择该 Profile
0.50 <= score < 0.75: 选择该 Profile，但报告 warning
score < 0.50: 降级 generic
```

---

## 五、Profile Schema 扩展方案

### 5.1 文件位置

首期建议继续放在：

```text
crates/compiler-engine/profiles/
```

新增：

```text
generic.toml
jos-paper.toml
tacl.toml
cvpr.toml
nature.toml
springer.toml
chinese-academic.toml
```

后续再迁移到 workspace 根目录 `profiles/`，方便用户自定义。

### 5.2 TOML Schema

建议扩展为：

```toml
id = "tacl"
display_name = "ACL/TACL Paper"
schema_version = "1.0"

document_classes = ["acl"]
aliases = ["acl-paper", "tacl"]

[detection]
min_confidence = 0.75
fallback_profile = "generic"

[[detection.signals]]
kind = "documentclass"
value = "acl"
weight = 0.70

[[detection.signals]]
kind = "documentclass_option"
value = "aclang"
weight = 0.20

[[detection.signals]]
kind = "macro"
value = "aclfinalcopy"
weight = 0.10

[backend]
preferred = "luatex-node"
fallback = ["xelatex-hook", "rule-based"]
requires_xetex = false
prefers_luatex = true

[page_setup]
kind = "letter"
columns = 2
margin_top_mm = 25.0
margin_bottom_mm = 25.0
margin_left_mm = 19.0
margin_right_mm = 19.0

[font_policy]
latin_main = "Times New Roman"
cjk_main = ""
math = "Cambria Math"
notes = "ACL/TACL Word-compatible fallback"

[caption_policy]
figure_prefix = "Figure"
table_prefix = "Table"
equation_prefix = ""
numbering = "arabic"
placement = "profile-default"

[citation_policy]
style = "author-year"
bibliography_style = "acl_natbib"
reference_section_title = "References"
supports_natbib = true
supports_biblatex = false

[semantic_policy]
unknown_macro = "rule-engine"
preserve_raw_fallback = true
collect_runtime_events = true
collect_layout_graph = true
enable_ref_fields = true
enable_omml_equations = true

[[macro_rules]]
name = "citet"
semantic = "citation"
args = 1
style = "textual"

[[macro_rules]]
name = "citep"
semantic = "citation"
args = 1
style = "parenthetical"

[[style_map]]
semantic = "heading.level1"
docx_style = "Heading1"

[[style_map]]
semantic = "paragraph.body"
docx_style = "BodyText"

[quality]
min_compatibility_score = 75
max_raw_fallback_blocks = 10
require_reference_graph = true
require_docx_openable = true
```

### 5.3 Profile 加载优先级

```text
显式 --profile-path
  ↓
显式 --profile-id
  ↓
JournalDetector 自动检测
  ↓
generic 降级
```

如果同时提供 `--profile-id` 和检测结果不一致：

- 以显式 `--profile-id` 为准。
- 在报告中记录 `profile_override` warning。

---

## 六、核心模块改造方案

### 6.1 新增 JournalDetector

建议新增模块：

```text
crates/compiler-engine/src/journal_detector.rs
```

职责：

- 读取 `VirtualFs` 中所有 tex-like 文件。
- 去除注释。
- 解析 `documentclass` 的 options 和 class。
- 解析 package 和模板特有宏。
- 按所有 Profile 的 detection rules 打分。
- 返回最佳 Profile 和诊断信息。

关键接口：

```rust
pub struct JournalDetector {
    registry: ProfileRegistry,
}

impl JournalDetector {
    pub fn detect(&self, vfs: &VirtualFs) -> JournalDetectionReport;
}

pub struct JournalDetectionReport {
    pub selected_profile_id: String,
    pub confidence: f32,
    pub candidates: Vec<JournalDetection>,
    pub diagnostics: Vec<JournalDiagnostic>,
}
```

### 6.2 扩展 ProfileRegistry

建议将 `profiles.rs` 从“文件 loader”升级为 registry：

```rust
pub struct ProfileRegistry {
    profiles: HashMap<String, ProfileSpecFile>,
    aliases: HashMap<String, String>,
}

impl ProfileRegistry {
    pub fn load_default() -> Result<Self, ProfileLoadError>;
    pub fn get(&self, id: &str) -> Option<&ProfileSpecFile>;
    pub fn resolve_alias(&self, id: &str) -> Option<&str>;
    pub fn all(&self) -> impl Iterator<Item = &ProfileSpecFile>;
}
```

首期可先保留现有 `load_profile()`，在其上增加 registry，不强行删除旧接口。

### 6.3 扩展 EngineProfile

当前 `EngineProfile` 是 enum：

```rust
GenericArticle
ChineseAcademic
JosPaper
MedicalJournal
```

首期可以有两种实现路线：

#### 路线 A：最小变更

继续扩展 enum：

```rust
Generic
ChineseAcademic
JosPaper
Tacl
Cvpr
Nature
Springer
MedicalJournal
```

优点：改动小。  
缺点：每新增期刊都要改 Rust 代码。

#### 路线 B：推荐路线

引入动态 Profile：

```rust
pub enum EngineProfile {
    Builtin(BuiltinProfile),
    External(String),
}

pub enum BuiltinProfile {
    Generic,
    ChineseAcademic,
    JosPaper,
    Tacl,
    Cvpr,
    Nature,
    Springer,
    MedicalJournal,
}
```

或者进一步改为：

```rust
pub struct EngineProfile {
    pub id: String,
}
```

首期建议采用路线 A 快速落地，同时在文档和代码中预留路线 B。  
如果开发周期允许，直接采用路线 B 更利于长期泛化。

### 6.4 Profile-aware 后端选择

当前 AutoBackend 选择逻辑主要依据：

- CTeX/xeCJK/fontspec/XeTeX 信号。
- LuaTeX 信号。
- 本机 runtime 可用性。

建议改为：

```text
JournalDetector 先选 Profile
  ↓
Profile.backend.preferred 给出首选后端
  ↓
TemplateSignals 修正必须使用 XeLaTeX/LuaLaTeX 的情况
  ↓
RuntimeAvailabilitySnapshot 判断是否可用
  ↓
按 fallback 链降级
```

Profile 默认后端建议：

| Profile | 首选 | fallback |
|---|---|---|
| `jos-paper` | `luatex-node` | `xelatex-hook`, `rule-based` |
| `tacl` | `luatex-node` | `xelatex-hook`, `rule-based` |
| `cvpr` | `luatex-node` | `xelatex-hook`, `rule-based` |
| `nature` | `luatex-node` | `xelatex-hook`, `rule-based` |
| `springer` | `luatex-node` | `xelatex-hook`, `rule-based` |
| `chinese-academic` | `xelatex-hook` | `luatex-node`, `rule-based` |
| `generic` | `luatex-node` | `xelatex-hook`, `rule-based` |

修正规则：

- 如果检测到 `ctex`、`xeCJK`、`\setCJKmainfont`，优先 `xelatex-hook`。
- 如果检测到 `\directlua` 或 `luatexja`，优先 `luatex-node`。
- 如果 runtime 不可用，按 fallback 链继续。
- fallback 必须写入 `CompileReport.backend` 和 diagnostics。

### 6.5 Profile-aware CompatibilityAnalyzer

扩展 `ProfileKind`：

```rust
pub enum ProfileKind {
    Generic,
    ChineseAcademic,
    JosPaper,
    Tacl,
    Cvpr,
    Nature,
    Springer,
    MedicalJournal,
}
```

或改为：

```rust
pub struct ProfileKind {
    pub id: String,
    pub supported_document_classes: Vec<String>,
    pub supported_packages: Vec<String>,
    pub limited_packages: Vec<String>,
    pub unsupported_packages: Vec<String>,
}
```

Profile-aware 检查规则：

| Profile | 强支持 | warning | unsupported |
|---|---|---|---|
| `jos-paper` | `IEEEtran[journal]`、`amsmath`、`graphicx` | `algorithm2e`、`tabularx` | `beamer`、`standalone` |
| `tacl` | `acl[aclang]`、`natbib` | `biblatex`、`tikz` | `minted` 默认 unsupported |
| `cvpr` | `IEEEtran[conference]`、`graphicx`、`amsmath` | `algorithmicx`、`subcaption` | `pstricks` |
| `nature` | `nature`、`natbib` | `biblatex`、复杂 floats | `pstricks` |
| `springer` | `springer`、`svjour3`、`llncs` | `algorithm2e`、`longtable` | `beamer` |
| `chinese-academic` | `ctexart`、`xeCJK`、`fontspec` | `gbt7714`、`biblatex` | `minted` |
| `generic` | 任意 article-like | 所有非核心宏包 | beamer/standalone 可 warning 或 unsupported |

### 6.6 Profile-aware RuleEngine

当前 RuleEngine 只加载 builtin rules。建议新增：

```text
crates/compiler-engine/profiles/{profile_id}/rules.toml
```

或直接内嵌到 profile TOML 的 `[[macro_rules]]`。

首期重点宏：

#### JOS / IEEE

```text
\IEEEauthorblockN
\IEEEauthorblockA
\IEEEkeywords
\markboth
\IEEEpeerreviewmaketitle
\cite
\citep
\citet
```

#### ACL/TACL

```text
\aclfinalcopy
\aclpaperid
\citet
\citep
\citealp
\shorttitle
\name
\address
```

#### CVPR/ICCV

```text
\cvprfinalcopy
\iccvfinalcopy
\cvprPaperID
\confName
\confYear
\author
\affiliation
```

#### Nature

```text
\corres
\equalcont
\author
\affil
\maketitle
\bibliographystyle{naturemag}
```

#### Springer

```text
\institute
\titlerunning
\authorrunning
\email
\orcidID
\keywords
```

#### 中文学报

```text
\zihao
\songti
\heiti
\kaishu
\fangsong
\CTEXsetup
\ctexset
\keywords
\zhabstract
\enabstract
```

RuleOutput 建议扩展：

```rust
pub enum RuleOutput {
    InlineText { content_arg: usize },
    BlockHeading { level: u8, content_arg: usize },
    MetadataField { key: String, content_arg: usize },
    AuthorList { content_arg: usize },
    Affiliation { content_arg: usize },
    KeywordList { content_arg: usize, separator: String },
    Citation { keys_arg: usize, style: CitationStyle },
    Ignore,
    PreserveRaw,
}
```

### 6.7 Runtime Hook 扩展

#### XeLaTeX Hook

当前 XeLaTeX hook 覆盖 label/ref/cite/includegraphics。首期建议按 Profile 注入额外 hook：

```text
common hook
  + profile hook
  + fallback cleanup
```

例如 `chinese-academic`：

- `\section` / `\subsection` hook 可选开启。
- `\caption` hook 补充 caption runtime event。
- `\keywords` / `\zhabstract` / `\enabstract` 映射为 metadata。

#### LuaTeX Hook

当前 LuaTeX hook 已采集 node tree 和部分宏。首期扩展：

- 增加 `profile_id` 写入 schema header。
- 增加 `origin = "runtime-luatex"`。
- 增加 profile-specific macro hook。
- 对 citation 命令支持 `\citet`、`\citep`。
- 对 author/affiliation/keyword 命令输出 metadata event。

建议事件扩展：

```rust
pub enum SemanticEvent {
    Heading { ... },
    Paragraph { ... },
    Figure { ... },
    Table { ... },
    Equation { ... },
    Citation { ... },
    Caption { ... },
    Label { ... },
    Reference { ... },
    Metadata { key: String, value: String, span: Option<SourceSpan> },
    EnvironmentBegin { name: String, span: Option<SourceSpan> },
    EnvironmentEnd { name: String, span: Option<SourceSpan> },
}
```

---

## 七、DOCX 渲染策略

### 7.1 Profile 到 Word 样式映射

每个 Profile 必须定义：

```text
heading.level1 -> Heading1 或 profile 专属样式
heading.level2 -> Heading2
paragraph.body -> BodyText
caption.figure -> FigureCaption
caption.table -> TableCaption
equation.display -> Equation
bibliography.item -> Bibliography
metadata.title -> Title
metadata.author -> Author
```

首期不用追求完全复刻期刊版式，但必须满足：

- DOCX 可打开。
- 标题层级正确。
- 图表公式引用可读。
- 参考文献区存在。
- caption 前缀符合 Profile。
- 中文学报使用中文 caption 前缀。

### 7.2 Citation 策略

首期最低要求：

| Profile | Citation style |
|---|---|
| `jos-paper` | numeric / IEEE-like |
| `tacl` | author-year / natbib-like |
| `cvpr` | numeric / compressed |
| `nature` | numeric / superscript 可降级 |
| `springer` | numeric 或 author-year，按 profile 配置 |
| `chinese-academic` | numeric-compressed / GBT7714-like |
| `generic` | numeric |

当前如果 DOCX writer 暂不支持全部 citation 格式，应在报告中输出降级信息。

### 7.3 表格和图片降级

首期规则：

- 普通 `tabular` 转 DOCX table。
- `tabularx`、`longtable`、`multirow` 标记为 limited support。
- TikZ 仍可先标记为 unsupported 或 rasterize fallback，取决于是否实现 rasterizer。
- `includegraphics` 维持媒体资产复制和关系写入。

---

## 八、开发任务拆分

### P0：方案与基线确认

- 确认本方案。
- 确认不改旧 Rust doc 转换引擎。
- 为首期期刊准备最小 fixture。

输出：

```text
docs-zh/semantic-tex-engine-journal-profile-generalization-plan-20260621-125025.md
```

### P1：Profile Schema 与 Registry

任务：

1. 扩展 `ProfileSpecFile`。
2. 新增 `ProfileRegistry`。
3. 新增首期 7 个 TOML Profile。
4. 保留现有 JSON Profile 加载兼容。
5. 增加单元测试。

验收：

```text
cargo test -p doc-compiler-engine profile
```

必须通过：

- `list_profile_ids()` 包含 7 个首期 ID。
- `generic` alias 可解析。
- TOML schema 可反序列化。

### P2：JournalDetector

任务：

1. 新增 `journal_detector.rs`。
2. 实现 documentclass options 解析。
3. 实现 package/macro/bibliography style 信号扫描。
4. 实现 profile scoring。
5. 接入 `CompileReport`。

验收：

```text
cargo test -p doc-compiler-engine journal_detector
```

测试样例：

```latex
\documentclass[journal]{IEEEtran}      -> jos-paper
\documentclass[aclang]{acl}            -> tacl
\documentclass[conference]{IEEEtran}   -> cvpr
\documentclass{nature}                 -> nature
\documentclass{springer}               -> springer
\documentclass{ctexart}                -> chinese-academic
\documentclass{article}                -> generic
```

### P3：Profile-aware Backend Selector

任务：

1. 将 JournalDetector 结果传入 backend selector。
2. Profile 指定 preferred backend 和 fallback chain。
3. 保留现有 TemplateSignals 修正逻辑。
4. 在 report 中记录 profile detection、backend selection 和 fallback 原因。

验收：

```text
cargo test -p doc-compiler-engine backend_selector
```

关键测试：

- `ctexart` 即使 generic LuaTeX 可用，也优先 `XeLaTeXHook`。
- `acl[aclang]` 优先 `LuaTeXNode`。
- runtime 不可用时降级 `RuleBased`。

### P4：CompatibilityAnalyzer Profile 化

任务：

1. 扩展 `ProfileKind` 或改为动态 profile policy。
2. 按 Profile 判断 class/package/environment。
3. 把 min score 移到 Profile quality 配置。
4. 报告中输出 profile mismatch 和降级建议。

验收：

```text
cargo test -p doc-compatibility-analyzer
```

### P5：RuleEngine Profile 规则

任务：

1. 支持从 Profile 加载 `macro_rules`。
2. 实现 citation、metadata、author、affiliation、keyword 等 RuleOutput。
3. 将规则路由接入 `apply_rule_engine_to_document`。
4. 未识别宏继续保留 RawFallback 或 InlineText fallback。

验收：

```text
cargo test -p doc-rule-engine
cargo test -p doc-compiler-engine rule_engine
```

### P6：Runtime Hook Profile 扩展

任务：

1. LuaTeX hook 写入 profile schema header。
2. 支持 profile-specific macro hook 注入。
3. XeLaTeX hook 支持 profile-specific macro hook 注入。
4. 增加 sidecar 中 profile/origin/source 信息。

验收：

```text
cargo test -p doc-compiler-engine runtime_hook
```

有 TeX runtime 的环境下增加集成测试：

```text
cargo test -p doc-compiler-engine luatex_runtime_collects_semantic_events
```

### P7：期刊 fixture 与端到端验证

目录建议：

```text
examples/journals/
├── jos-paper/minimal.tex
├── tacl/minimal.tex
├── cvpr/minimal.tex
├── nature/minimal.tex
├── springer/minimal.tex
├── chinese-academic/minimal.tex
└── generic/minimal.tex
```

新增验证脚本：

```text
scripts/verify_journal_profiles.sh
```

输出：

```text
examples/journals/output/
├── jos-paper.docx
├── tacl.docx
├── cvpr.docx
├── nature.docx
├── springer.docx
├── chinese-academic.docx
└── generic.docx
```

---

## 九、首期验收标准

### 9.1 功能验收

| 项目 | 标准 |
|---|---|
| Profile 检测 | 7 类模板均能自动识别，arXiv/未知模板降级 generic |
| 后端选择 | 根据 Profile 与 runtime 可用性选择并记录原因 |
| 兼容分析 | 按 Profile 输出 score、warnings、unsupported |
| 语义采集 | heading、paragraph、figure、table、equation、citation、reference 均保留 |
| RuleEngine | 至少支持各 Profile 的 citation/metadata/keyword 代表性宏 |
| DOCX 输出 | 每个 fixture 生成可打开 DOCX |
| 报告 | 输出 profile、backend、compatibility、fallback、raw fallback 数量 |

### 9.2 质量指标

| 指标 | 首期阈值 |
|---|---:|
| profile detection accuracy | 100% for 7 minimal fixtures |
| compatibility report generated | 100% |
| DOCX openable | 100% |
| raw fallback blocks | minimal fixture <= 5 |
| unresolved references | minimal fixture <= 2 |
| runtime fallback report | 100% recorded when fallback happens |

### 9.3 回归要求

必须保证：

```text
cargo test -p doc-compiler-engine
cargo test -p doc-compatibility-analyzer
cargo test -p doc-rule-engine
```

同时 paper3 原有三路径验证脚本不应退化。

---

## 十、风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| IEEEtran 同时用于 JOS 和 CVPR | 检测误判 | 强制使用 options：`journal` -> JOS，`conference` -> CVPR |
| ACL/TACL class 版本差异 | 检测漏判 | 使用 class + macro + package 多信号评分 |
| Nature/Springer 模板变体多 | 首期覆盖不足 | 首期只保证指定信号，后续扩展 `svjour3`、`llncs`、`sn-jnl` |
| 中文 CTeX 在 LuaLaTeX 下版式差异 | DOCX 结构不稳 | `chinese-academic` 默认走 XeLaTeXHook |
| Profile schema 膨胀 | 实现复杂 | 首期只实现 detection/backend/compatibility/macro/style/quality 六类核心配置 |
| 规则和渲染耦合 | 难维护 | Profile 只描述策略，具体转换仍在 RuleEngine/Renderer 中实现 |
| runtime 不可用 | 集成测试不稳定 | 所有 runtime 测试允许 skip，并要求 RuleBased fallback 可用 |

---

## 十一、建议开发顺序

建议按以下顺序开发：

1. Profile schema 扩展和 7 个 TOML Profile。
2. JournalDetector 自动识别。
3. Profile-aware backend selector。
4. CompatibilityAnalyzer Profile 化。
5. RuleEngine 加载 Profile 宏规则。
6. LuaTeX/XeLaTeX hook 按 Profile 增强。
7. DOCX style/citation/caption policy 接入。
8. 7 个 minimal fixture 和验证脚本。

每完成一步都应更新开发报告到 `docs-zh`，并记录：

- 已完成内容。
- 涉及文件。
- 测试命令和结果。
- 与旧路径隔离情况。
- 剩余风险。

---

## 十二、首期最小实现边界

首期不做：

- 不承诺完整 TikZ 可编辑 DOCX。
- 不承诺复杂宏包完全兼容。
- 不承诺 Nature/Springer 所有官方变体全部覆盖。
- 不把 LuaTeX node tree 直接作为唯一语义来源。
- 不移除或重构旧 Rust doc 转换路径。

首期必须做：

- 7 类 Profile 自动检测。
- 7 类 Profile 文件化配置。
- Profile-aware backend 选择和 fallback。
- Profile-aware 兼容性报告。
- 代表性宏规则接入 RuleEngine。
- 生成可审计报告。
- minimal fixture 可生成 DOCX。

---

## 十三、结论

提升泛化能力的关键不是继续堆单个 TeX 宏解析，而是建立 Profile 驱动的模板识别、规则加载、兼容性分析和渲染策略体系。

首期建议以 `JournalDetector + ProfileRegistry + Profile-aware RuleEngine + Profile-aware Backend Selector` 为核心，覆盖 IEEE JOS、ACL/TACL、CVPR/ICCV、Nature、Springer、中文学报和 generic 降级路径。这样可以在不影响旧 Rust doc 转换引擎的前提下，把新语义引擎从 paper3/JOS 单点适配推进到多期刊可扩展架构。
