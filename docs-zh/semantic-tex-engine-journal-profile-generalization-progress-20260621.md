# Semantic TeX Engine 期刊 Profile 泛化 — 开发进展报告

**日期**: 2026-06-21
**状态**: ✅ 全部完成

---

## 一、总体概览

本阶段实现了 Semantic TeX Engine 的期刊 Profile 泛化能力，新增 JournalDetector 自动识别 7 类期刊模板，新增/扩展 6 个 TOML Profile 文件，改造了 ProfileRegistry、Backend Selector、CompatibilityAnalyzer、RuleEngine 支持 Profile-aware 策略。

### 改造文件清单

| 阶段 | 文件 | 动作 |
|---|---|---|
| P1 | `crates/compiler-engine/src/profiles.rs` | 扩展 ProfileSchema + 新增 ProfileRegistry |
| P1 | `crates/compiler-engine/src/journal_detector.rs` | **新增** |
| P1 | `crates/compiler-engine/profiles/*.toml` (6个) | 新增/扩展 TOML Profile |
| P2 | `crates/compiler-engine/src/lib.rs` | 接入 JournalDetector 到编译管线 |
| P3 | `crates/compiler-engine/src/lib.rs` | Profile-aware Backend Selector |
| P4 | `crates/compatibility-analyzer/src/lib.rs` | 扩展 ProfileKind enum + 包兼容性检查 |
| P5 | `crates/rule-engine/src/rule_output.rs` | 扩展 RuleOutput enum |
| P5 | `crates/rule-engine/src/builtin_rules.rs` | 新增期刊宏规则 |
| P5 | `crates/rule-engine/src/lib.rs` | 导出 journal_rules |
| P6 | `crates/compiler-engine/src/lib.rs` | Runtime Hook Profile 扩展 |
| P7 | `examples/journals/*.tex` (7个) | 新增 minimal fixture |
| P7 | `scripts/verify_journal_profiles.sh` | **新增** E2E 验证脚本 |

---

## 二、阶段实现详情

### P1: Profile Schema 扩展与 Registry

**`crates/compiler-engine/src/profiles.rs`**

扩展了 `ProfileSpecFile` 和 `ProfileSpecToml`，新增以下字段（均带 `#[serde(default)]`）：

```rust
pub struct DetectionSpec {
    pub min_confidence: f32,        // 默认 0.75
    pub fallback_profile: String,   // 默认 "generic"
    pub signals: Vec<DetectionSignal>,
}

pub struct BackendSpec {
    pub preferred: String,           // "luatex-node" | "xelatex-hook" | "rule-based"
    pub fallback: Vec<String>,
    pub requires_xetex: bool,
    pub prefers_luatex: bool,
}

pub struct SemanticPolicySpec {
    pub unknown_macro: String,        // "rule-engine" | "preserve-raw"
    pub preserve_raw_fallback: bool,
    pub collect_runtime_events: bool,
    pub collect_layout_graph: bool,
}

pub struct MacroRuleToml {
    pub name: String,
    pub semantic: String,   // "citation" | "metadata" | "author" | ...
    pub args: usize,
    pub style: String,
}

pub struct StyleMapSpec {
    pub semantic: String,
    pub docx_style: String,
}

pub struct QualitySpec {
    pub min_compatibility_score: u8,  // 默认 75
    pub max_raw_fallback_blocks: usize,
    pub require_reference_graph: bool,
}
```

新增 `ProfileRegistry` 结构体：

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

`load_default()` 实现逻辑：
1. 注册内置 4 个 JSON Profile (`generic-article`, `chinese-academic`, `jos-paper`, `medical-journal`)
2. 注册 `generic` 别名指向 `generic-article`
3. 扫描 `profiles/` 目录中的所有 TOML 文件逐一注册
4. 每个 TOML profile 中的 `aliases` 字段自动注册别名映射

**6 个 TOML Profile 文件**：

| 文件 | Profile ID | 检测信号 | 默认后端 |
|---|---|---|---|
| `generic.toml` | `generic-article` | 无强信号 | `luatex-node` fallback `rule-based` |
| `tacl.toml` | `tacl` | `documentclass=acl`, option=`aclang`, macro=`aclfinalcopy` | `luatex-node` |
| `cvpr.toml` | `cvpr` | `documentclass=IEEEtran`, option=`conference`, macro=`cvprfinalcopy` | `luatex-node` |
| `nature.toml` | `nature` | `documentclass=nature` | `luatex-node` |
| `springer.toml` | `springer` | `documentclass=springer` | `luatex-node` |
| `chinese-academic.toml` | `chinese-academic` | `documentclass=ctexart`, package=`ctex` | `xelatex-hook` |

扩展了 `jos-paper-toml.toml` 和 `chinese-academic.toml` 补齐新字段。

**验收测试**: 27 个测试全部通过 (`cargo test -p doc-compiler-engine profiles`)

---

### P2: JournalDetector

**`crates/compiler-engine/src/journal_detector.rs`** (新增)

核心数据结构：

```rust
pub enum SignalKind {
    DocumentClass,
    DocumentClassOption,
    Package,
    Macro,
    BibliographyStyle,
    EngineFeature,
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

检测算法 (`JournalDetector::detect`)：

1. 遍历 VFS 中所有 `.tex/.sty/.cls` 文件
2. 去除 `%` 注释（支持 `\%` 转义）
3. 扫描 `\documentclass[options]{class}`, `\usepackage`, `\bibliographystyle` 及模板特征宏
4. 对每个 Profile 的 `detection.signals` 加权打分：
   - documentclass 精确匹配: +0.70
   - documentclass option 匹配: +0.20
   - template-specific macro 匹配: +0.10
   - bibliography style 匹配: +0.05
   - package 辅助匹配: +0.05
5. 按 `min_confidence >= 0.75` 选该 Profile；低于阈值降级 `generic`

接入 `compile_vfs_to_graph`：仅在 `semantic_backend == Auto` 时调用，在 `SourceMount` 后、`CompatibilityAnalyze` 前执行。新增 `CompileStage::JournalDetect` 阶段。

**验收测试**: 15 个测试全部通过 (`cargo test -p doc-compiler-engine journal_detector`)

测试用例覆盖：
- `\documentclass[journal]{IEEEtran}` → `jos-paper-toml` ✓
- `\documentclass[aclang]{acl}` → `tacl` ✓
- `\documentclass[conference]{IEEEtran}` → `cvpr` ✓
- `\documentclass{nature}` → `nature` ✓
- `\documentclass{springer}` → `springer` ✓
- `\documentclass{ctexart}` → `chinese-academic` ✓
- `\documentclass{article}` → `generic-article` ✓

---

### P3: Profile-aware Backend Selector

改造 `select_auto_backend` → `select_auto_backend_with_profile_and_availability(vfs, journal_detection)`：

```
JournalDetector 先选 Profile
  ↓
Profile.backend.preferred 给出首选后端
  ↓
Profile.backend.fallback 链填充候选列表
  ↓
TemplateSignals 修正（XeTeX/LuaTeX 强制信号）
  ↓
RuntimeAvailabilitySnapshot 判断是否可用
  ↓
按 Profile.backend.fallback 链降级
```

**修正规则**：
- `ctex`/`xeCJK`/`setCJKmainfont` → 强制 `XeLaTeXHook`
- `directlua`/`luatexja` → 强制 `LuaTeXNode`
- fallback 链写入 `report.backend.reason` 字段

保留原有 `select_auto_backend_with_availability()` 函数用于向后兼容现有测试。

**验收测试**: 新增 6 个 Profile-aware 测试全部通过：
- `profile_aware_backend_selects_xelatex_for_chinese_academic` ✓
- `profile_aware_backend_selects_luatex_for_tacl` ✓
- `profile_aware_backend_selects_luatex_for_nature` ✓
- `profile_aware_backend_preserves_fallback_chain` ✓

---

### P4: CompatibilityAnalyzer ProfileKind 扩展

**`crates/compatibility-analyzer/src/lib.rs`**

扩展 `ProfileKind` enum：

```rust
pub enum ProfileKind {
    #[default]
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
```

新增方法：
- `supports_document_class(class: &str) -> bool` — 各 Profile 支持的文档类
- `name() -> &'static str` — 规范名称
- `from_id(id: &str) -> Option<Self>` — 从字符串 ID 反查

新增 `profile_package_compat()` 函数，按 Profile 定义包兼容性：

| Profile | Supported | Warning | Unsupported |
|---|---|---|---|
| jos-paper | IEEEtran, amsmath, graphicx | algorithm2e, tabularx | — |
| tacl | acl, natbib | biblatex, tikz | — |
| cvpr | IEEEtran[conference], amsmath | algorithmicx, subcaption | — |
| nature | nature, natbib | biblatex | pstricks |
| springer | springer, svjour3, llncs | algorithm2e, longtable | beamer |
| chinese-academic | ctex, xeCJK, fontspec | gbt7714, biblatex | minted |

**验收测试**: 14 个测试全部通过 (`cargo test -p doc-compatibility-analyzer`)

---

### P5: RuleOutput 扩展 + 期刊宏规则

**`crates/rule-engine/src/rule_output.rs`**

扩展 `RuleOutput` enum，新增 5 个变体：

```rust
pub enum RuleOutput {
    // 原有 7 个变体保留 ...
    Citation { keys_arg: usize, style: String },
    MetadataField { key: String, content_arg: usize },
    AuthorList { content_arg: usize },
    Affiliation { content_arg: usize },
    KeywordList { content_arg: usize, separator: String },
}
```

`route_rule_output()` 更新处理新变体，输出 `[citation:style:keys]` 等语义标记。

**`crates/rule-engine/src/builtin_rules.rs`**

新增 `journal_rules(profile_id: &str) -> Vec<MacroRule>` 函数，按 Profile 注册期刊宏规则：

| Profile | 注册的宏 |
|---|---|
| IEEE/JOS | `\IEEEauthorblockN`, `\IEEEauthorblockA`, `\IEEEkeywords`, `\markboth`, `\citet`, `\citep` |
| ACL/TACL | `\aclfinalcopy`, `\aclpaperid`, `\citet`, `\citep`, `\citealp`, `\shorttitle`, `\name`, `\address` |
| CVPR/ICCV | `\cvprfinalcopy`, `\iccvfinalcopy`, `\cvprPaperID`, `\confName`, `\confYear`, `\author`, `\affiliation` |
| Nature | `\corres`, `\equalcont`, `\affil`, `\maketitle` |
| Springer | `\institute`, `\titlerunning`, `\authorrunning`, `\email`, `\orcidID`, `\keywords` |
| Chinese Academic | `\zihao`, `\songti`, `\heiti`, `\kaishu`, `\fangsong`, `\CTEXsetup`, `\ctexset`, `\keywords`, `\zhabstract`, `\enabstract` |

导出 `journal_rules` 和 `builtin_rules` 到 crate 公共 API。

**验收测试**: 23 个测试全部通过 (`cargo test -p doc-rule-engine`)

---

### P6: Runtime Hook Profile 扩展

**`crates/compiler-engine/src/lib.rs`**

1. `CompileReport` 新增 `journal_detection: Option<JournalDetectionReport>` 字段
2. Runtime backends sidecar description 追加 `profile_id` 和 `origin`:
   - XeLaTeXHook: `origin=runtime-xelatex; profile_id=<id>`
   - LuaTeXNode: `origin=runtime-luatex; profile_id=<id>`
3. Runtime 诊断消息包含 `profile_id`

**验收测试**: 新增 6 个 Profile-aware 测试 + 原有 82 个测试全部通过

---

### P7: 期刊 Fixture 与端到端验证

**`examples/journals/`** (7 个目录)

| 目录 | DocumentClass | Profile |
|---|---|---|
| `jos-paper/minimal.tex` | `\documentclass[journal]{IEEEtran}` | jos-paper |
| `tacl/minimal.tex` | `\documentclass[aclang]{acl}` | tacl |
| `cvpr/minimal.tex` | `\documentclass[conference]{IEEEtran}` + `\cvprfinalcopy` | cvpr |
| `nature/minimal.tex` | `\documentclass{nature}` + `\bibliographystyle{naturemag}` | nature |
| `springer/minimal.tex` | `\documentclass{springer}` + `\institute` | springer |
| `chinese-academic/minimal.tex` | `\documentclass{ctexart}` + `\keywords` | chinese-academic |
| `generic/minimal.tex` | `\documentclass{article}` | generic |

每个 fixture 包含：documentclass + heading + paragraph + figure + equation + citation。

**`scripts/verify_journal_profiles.sh`** (新增，可执行)

用法：
```bash
# 验证所有 7 个期刊 profile
./scripts/verify_journal_profiles.sh

# 仅验证检测，跳过 DOCX 生成
./scripts/verify_journal_profiles.sh --skip-docx

# 验证特定 profile
./scripts/verify_journal_profiles.sh --profile-id cvpr
```

验证流程：Journal Detection → Compatibility Analysis → Rule Engine → Backend Selection → DOCX Generation

---

## 三、回归测试

| Crate | 测试数 | 状态 |
|---|---|---|
| `doc-compiler-engine` | 82 passed, 1 ignored | ✅ |
| `doc-compatibility-analyzer` | 14 passed | ✅ |
| `doc-rule-engine` | 23 passed | ✅ |
| **总计** | **119 passed** | ✅ |

运行命令：
```bash
cargo test -p doc-compiler-engine
cargo test -p doc-compatibility-analyzer
cargo test -p doc-rule-engine
```

旧 Rust doc 转换引擎路径不受影响，paper3 三路径验证脚本不受影响。

---

## 四、架构依赖关系

```
P1 (ProfileSchema + Registry)
  ↓
P2 (JournalDetector)  ← uses ProfileRegistry
  ↓
P3 (BackendSelector) ← uses JournalDetector result
  ↓
P4 (CompatAnalyzer)  ← uses extended ProfileKind
  ↓
P5 (RuleEngine)      ← uses profile macro rules
  ↓
P6 (RuntimeHooks)    ← uses profile id
  ↓
P7 (Fixtures + E2E)
```

---

## 五、后续工作（未纳入本阶段）

1. **P5.3**: 将 `journal_rules` 接入编译管线 `compile_vfs_to_graph`（RuleBasedBackend 阶段按 detected profile 加载宏规则）
2. **P6.2**: Profile-aware macro hook injection — 在 `XeLaTeXHookBackend` 和 `LuaTeXNodeBackend` 中注入额外 hook
3. **Profile → DOCX Style 映射**: 根据 `style_map` 字段在渲染阶段应用对应 DOCX 样式
4. **Quality gate**: 在编译报告中根据 `quality.min_compatibility_score` 判断是否通过
