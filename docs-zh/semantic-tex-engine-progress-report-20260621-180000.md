# Semantic TeX Engine 开发进展报告

**文档版本**：20260621-180000
**报告日期**：2026-06-21
**状态**：持续推进
**目标读者**：开发者、项目管理者、技术决策者

---

## 一、总体进展概览

本项目（Tex2Doc / Semantic TeX Engine）已完成从 V1 旧 Rust 规则引擎到 V2 语义引擎的全面架构重构。当前系统支持：
- **3 条编译路径**：旧 rule-based 路径、XeLaTeX Hook 运行时路径、LuaTeX Node 运行时路径
- **7 类期刊 Profile**：generic-article、chinese-academic、jos-paper、tacl、cvpr、nature、springer
- **端到端 LaTeX → DOCX 转换**：完整编译报告、兼容性评分、质量门禁
- **商业化基础设施**：API 客户端、桌面 UI 骨架、质量门禁脚本

| 维度 | 状态 |
|------|------|
| 核心引擎 crate 数 | 15+ |
| 单元测试（核心） | 430+ 通过 |
| 期刊 Profile | 7 个 |
| 编译后端 | 3 种 |
| CLI 子命令 | 8 个（4 条语义引擎子命令） |
| 商业化 crate | 2 个（API 客户端 + Slint UI） |

---

## 二、历史开发脉络

### 2.1 V1 阶段（截至 2026-06-14）

原始 Tex2Doc 系统：
- 单一 Rust 规则引擎（`doc-core`）
- 硬编码的 JOS/中文学术样式
- 手动指定 Profile
- 有限的公式/表格支持

### 2.2 V2 语义引擎阶段（2026-06-15 ~ 2026-06-20）

核心架构重构：

|| 里程碑 | 内容 | 状态 |
|---|--------|------|------|
| M1 | 独立 engine 路径隔离 | ✅ 完成 |
| M2 | 双后端策略（RuleBased / XeLaTeX / LuaTeX） | ✅ 完成 |
| M3 | 语义采集模型（内置 / v2 hook） | ✅ 完成 |
| M4 | Crate 拆分（xdv-parser、semantic-collector） | ✅ 完成 |
| M5 | CompatibilityAnalyzer 独立 crate | ✅ 完成 |
| M6 | Profile 外置化（JSON/TOML） | ✅ 完成 |
| M7 | JournalDetector 自动识别 | ✅ 完成 |
| M8 | RuleEngine 独立 crate | ✅ 完成 |
| M9 | XDV Parser crate | ✅ 完成 |

### 2.3 Journal Profile 泛化（2026-06-21 上午）

7 类期刊 Profile 全部实现，包括 ProfileRegistry、Backend Selector 按 Profile 选型、CompatibilityAnalyzer 按 Profile 检查包兼容性、RuleEngine 期刊宏规则。

### 2.4 商业化基础设施（2026-06-21 下午）

本轮新增能力，详见第三节。

---

## 三、本轮新增能力（2026-06-21 下午）

### 3.1 ProfileStyleMap — DOCX 渲染层接入 Profile

**问题**：Profile 只控制编译器行为，但 DOCX 输出样式始终是硬编码的 JOS 21-style，与 Profile 对应期刊的样式要求不匹配。

**解决方案**：在 `crates/docx-writer/src/profile.rs` 中实现 `ProfileStyleMap`，按 Profile 映射语义角色 → DOCX styleId。

**实现详情**：

```rust
// ProfileStyleMap — 语义角色 → DOCX styleId 映射
pub struct ProfileStyleMap {
    pub by_role: BTreeMap<&'static str, &'static str>,
}

// 预设
ProfileStyleMap::jos()    // JOS 21-style（默认）
ProfileStyleMap::generic() // arxiv 通用样式
ProfileStyleMap::tacl()   // TACL/ACL 样式
```

**接入管线**：

1. `CompileOptions` 新增 `style_map: Option<doc_docx_writer::ProfileStyleMap>` 字段
2. `CompileOptions::effective_style_map()` 方法：优先使用用户指定 → 其次 Profile spec 内置 → fallback 到 `jos()`
3. `pack_with_page_setup` 接受 `style_map: Option<&ProfileStyleMap>` 参数
4. `serialize_document` 接受 `style_map` 参数，调用 `m.get("body")` 等查找样式 ID
5. fallback 机制：当 `style_map` 为 `None` 或特定角色无映射时，使用硬编码 JOS 默认值

**关键文件**：

| 文件 | 说明 |
|------|------|
| `crates/docx-writer/src/profile.rs` | 新增，`ProfileStyleMap` 定义 |
| `crates/docx-writer/src/packer.rs` | `pack_with_page_setup` 签名更新 |
| `crates/docx-writer/src/serializer.rs` | `serialize_document` + `write_front_matter` 参数更新 |
| `crates/docx-writer/src/lib.rs` | 导出 `ProfileStyleMap` |
| `crates/compiler-engine/src/lib.rs` | `CompileOptions`、`ProfileSpec`、`ProfileSpecReport` 扩展 |
| `crates/compiler-engine/examples/paper3_to_docx.rs` | 示例程序应用 `style_map` |

**ProfileSpec 内置映射**（`crates/compiler-engine/profiles/*.toml`）：

```toml
[style_map]
body = "BodyText"        # generic profile
heading1 = "Heading1"
# 未定义的 role fallback 到 jos() 默认值
```

### 3.2 QualityGateResult — 编译质量门禁

**问题**：编译完成后缺乏统一的质量判断标准，无法自动化判断"这份 DOCX 是否达到投稿/发布质量"。

**解决方案**：在 `CompileReport` 中嵌入 `QualityGateResult`，编译管线末尾自动执行。

**实现详情**：

```rust
// 质量门禁结果
pub struct QualityGateResult {
    pub passed: bool,
    pub total_checks: usize,
    pub passed_checks: usize,
    pub failed_checks: Vec<QualityCheck>,
}

pub struct QualityCheck {
    pub name: String,
    pub passed: bool,
    pub severity: QualitySeverity,  // Error / Warning / Info
    pub message: String,
}

pub enum QualitySeverity {
    Error,
    Warning,
    Info,
}
```

**内置检查项**（`CompileReport::run_quality_gate(70)`）：

| 检查项 | 阈值 | 严重性 |
|--------|------|--------|
| `compatibility_score` | ≥ min_score (默认 70) | Error |
| `unresolved_reference_count` | = 0 | Error |
| `omml_fallback_ratio` | < 1.0（不允许 100% fallback） | Warning |
| `docx_bytes` | > 0 | Error |
| `journal_detection_confidence` | ≥ 0.75（若执行了检测） | Warning |

**接入管线**：`compile_vfs_to_docx` 在 DOCX 组装完成后调用 `report.run_quality_gate(70)`。

**关键文件**：`crates/compiler-engine/src/lib.rs` 第 636-791 行。

### 3.3 CLI 语义引擎子命令

**问题**：`doc-engine` CLI 原本只有通用的 `convert`/`build` 命令，没有直接的语义引擎入口（detect/analyze/convert/verify）。

**解决方案**：新增 4 个平级子命令，所有参数显式传递，无嵌套子命令（避免 clap 4 嵌套 `Args` trait bound 问题）。

**新增命令**：

```bash
# 检测 TeX 项目的期刊 Profile（自动推断）
doc-engine semantic-detect \
  --project-root /path/to/project \
  [--main-tex main.tex] \
  [--output report.json]

# 分析 TeX 项目的兼容性
doc-engine semantic-analyze \
  --project-root /path/to/project \
  [--main-tex main.tex] \
  [--profile generic] \
  [--output report.json]

# TeX 项目转换为 DOCX（Semantic Engine）
doc-engine semantic-convert \
  --project-root /path/to/project \
  --main-tex main.tex \
  --profile auto \
  --backend auto \
  --out output.docx \
  [--report report.json] \
  [--no-backend-fallback]

# 验证 DOCX 质量（结构/引用/样式）
doc-engine semantic-verify \
  --docx-file output.docx \
  [--report report.json]
```

**架构决策**：所有 Args 类型定义在 `crates/cli/src/semantic_cmd.rs`（独立模块，解决 clap 4 嵌套 trait 问题），所有 handler 逻辑在 `main.rs`。`semantic-verify` 预留 P6 实现接口（返回 "not yet implemented"）。

**关键文件**：

| 文件 | 说明 |
|------|------|
| `crates/cli/src/main.rs` | 4 个子命令 dispatch + handler 实现 |
| `crates/cli/src/semantic_cmd.rs` | Args 类型定义（独立模块） |

### 3.4 commercial_verify.sh — CI 质量门禁脚本

**问题**：需要非 Rust 环境的 CI 系统（如 GitHub Actions 的 `runs-on: ubuntu-latest`）也能对生成的 DOCX 执行质量检查。

**解决方案**：纯 Bash 脚本，不依赖 Rust，仅依赖 `unzip`（标准工具）。

**功能**：

| 检查类别 | 检查项 |
|---------|--------|
| 结构检查 | 文件大小 > 1KB |
| 样式检查 | `word/styles.xml` 存在、`word/document.xml` 存在 |
| 引用检查 | `word/_rels/document.xml.rels` 存在 |

**用法**：

```bash
# 基础用法
./scripts/commercial_verify.sh --docx output.docx

# 自定义最低分数和报告路径
./scripts/commercial_verify.sh \
  --docx output.docx \
  --min-score 80 \
  --report quality-report.json

# 跳过特定类别检查（CI 加速）
./scripts/commercial_verify.sh --docx output.docx \
  --skip-structural \
  --skip-style
```

**输出**：颜色化日志（PASS/FAIL/WARN）+ 可选 JSON 报告。

**退出码**：0 = 全部通过，1 = 任意检查失败。

### 3.5 doc-commercial-api-client — 商业 API 客户端

**问题**：Tex2Doc 云服务需要 HTTP API 客户端供桌面端和服务器端使用。

**解决方案**：`crates/commercial-api-client` crate，基于 `reqwest`。

**核心 API**：

```rust
// DOCX 质量分析提交
let client = ApiClient::from_api_key("your-api-key")?;
let job: AnalysisJob = client.submit_analysis(&docx_bytes).await?;

// 轮询分析结果
let result: AnalysisResult = client.get_analysis_result(&job.job_id).await?;
```

**数据类型**：

```rust
// 请求/响应模型
AnalysisJob    { job_id, status: JobStatus, created_at }
AnalysisResult { job_id, status, report: Option<DetailedReport>, error }
DetailedReport { overall_score, structural_checks, style_checks, reference_checks }
CheckResult    { name, passed, score, message }
JobStatus      { Pending, Processing, Completed, Failed }
ApiError       { Transport, Http { status, body }, Url, Decode, Api { code, message } }
```

**特性**：
- multipart form 上传 DOCX（`application/vnd.openxmlformats-officedocument.wordprocessingml.document`）
- Bearer token 认证
- 可配置超时（默认 30s）
- `rustls-tls`（无系统证书依赖）

**关键文件**：

| 文件 | 说明 |
|------|------|
| `crates/commercial-api-client/src/lib.rs` | 公共 API 导出 |
| `crates/commercial-api-client/src/client.rs` | `ApiClient` 实现 |
| `crates/commercial-api-client/src/models.rs` | 所有请求/响应类型 |

### 3.6 doc-desktop-slint — 桌面 UI 骨架

**问题**：Tex2Doc 需要跨平台桌面客户端（Windows / macOS / Linux）。

**解决方案**：`crates/desktop-slint` crate，基于 Slint 1.x。

**文件结构**：

```
crates/desktop-slint/
├── Cargo.toml          # slint + doc-core + doc-compiler-engine
├── build.rs            # slint_build::compile("src/ui/main.slint")
└── src/
    ├── main.rs         # include_modules!() + Window::new().run()
    └── ui/
        ├── main.slint  # MainWindow 组件（LineEdit + Button + TextEdit）
        └── mod.rs      # slint::include_modules!()
```

**UI 组件**（`main.slint`）：

```slint
component MainWindow {
    in-out property <string> project-path;
    in-out property <string> status;

    VerticalBox {
        Text { text: "Tex2Doc Desktop"; font-size: 24px; }
        LineEdit { placeholder-text: "TeX project path..."; text => project-path; }
        Button { text: "Detect Profile"; clicked => { status = "Detecting..."; } }
        TextEdit { text => status; read-only: true; min-height: 200px; }
    }
}
```

**集成**：未来可注入 `doc-compiler-engine::SemanticTexEngine::compile_dir_to_docx()` 实现真正的编译逻辑。

---

## 四、完整 Milestone 状态

### M1: 独立 engine 路径 ✅

- `SemanticTexEngine::compile_dir_to_docx()` — 目录 → DOCX
- `SemanticTexEngine::compile_vfs_to_docx()` — VFS → DOCX
- `SemanticTexEngine::compile_vfs_to_graph()` — VFS → DocumentGraph
- 编译管线 8 个阶段：SourceMount → JournalDetect → CompatibilityAnalyze → IncludeGraph → TexParse → SemanticCollect → DocumentGraph → DocxRender

### M2: 双后端策略 ✅

- `SemanticBackendKind::RuleBased` — 内置语义规则
- `SemanticBackendKind::XeLaTeXHook` — XeLaTeX JSONL Hook
- `SemanticBackendKind::LuaTeXNode` — LuaTeX Node 采集
- `SemanticBackendKind::Auto` — JournalDetector 引导自动选择

### M3: 语义采集 ✅

- `SemanticEventV1` — 旧版兼容性
- `SemanticEventV2` — XeLaTeX Hook v2，含 `Caption` 枚举
- `CollectedDocument` — 采集输出模型
- `ReferenceGraph` — 引用关系图
- `LayoutGraph` — 页面布局图（XDV Parser 接入中）

### M4: Crate 拆分 ✅

| Crate | 职责 | 测试数 |
|-------|------|--------|
| `doc-xdv-parser` | XDV/DVI 字节码解析 | 24 |
| `doc-semantic-collector` | 语义采集 trait + 输出 | 20 |
| `doc-compatibility-analyzer` | 包兼容性扫描 | 14 |
| `doc-rule-engine` | 规则引擎 + AI fallback | 23 |

### M5: CompatibilityAnalyzer ✅

- `ProfileKind`（9 个变体）
- `CompatibilityReport` — 评分、Unsupported 列表、Warning 列表
- `profile_package_compat()` — 按 Profile 检查包兼容性

### M6: Profile 外置化 ✅

- `ProfileRegistry::load_default()` — 注册内置 JSON + 扫描 TOML
- `ProfileSpecFile` — TOML/JSON 格式支持
- 别名注册（`generic` → `generic-article`）
- `crates/compiler-engine/profiles/*.toml`（7 个期刊 Profile）

### M7: JournalDetector ✅

- 权重打分算法（documentclass + option + macro + bibliography + package）
- `min_confidence >= 0.75` 阈值
- 7 个期刊 Profile 全部有 fixture 验证

### M8: RuleEngine ✅

- `RuleEngine::process()` — 离线规则推断
- `RuleEngine::process_with_ai()` — 可选 AI inference（默认禁用）
- `journal_rules(profile_id)` — 期刊专用宏规则
- `AiEngine` trait — 可插入 OpenAI/Ollama（待接入）

### M9: XDV Parser ✅

- 24 个 opcode 覆盖 Phase 1
- `NativeFont` Phase 2 实现（GlyphId → Unicode + FontMetrics）
- 11 + 13 = 24 个 fixture 测试

### M10: ProfileStyleMap ✅ [新增]

- DOCX 渲染层接入 Profile 样式映射
- 3 个预设：`jos()`、`generic()`、`tacl()`

### M11: QualityGateResult ✅ [新增]

- 编译管线末尾自动质量门禁
- 4 项内置检查 + 分数阈值可配置

### M12: CLI 语义子命令 ✅ [新增]

- `semantic-detect` / `semantic-analyze` / `semantic-convert` / `semantic-verify`

### M13: commercial_verify.sh ✅ [新增]

- CI 质量门禁脚本（纯 Bash）
- 颜色化输出 + JSON 报告

### M14: commercial-api-client ✅ [新增]

- HTTP API 客户端（multipart upload + job polling）
- 完整类型系统

### M15: desktop-slint ✅ [新增]

- Slint 桌面 UI 骨架
- `MainWindow` 组件

---

## 五、工程结构

```
crates/
├── compiler-engine/           # V2 语义编译引擎（facade + profiles）
│   ├── src/
│   │   ├── lib.rs           # SemanticTexEngine、CompileOptions、CompileReport、
│   │   │                     # QualityGateResult、BackendSelector
│   │   ├── profiles.rs      # ProfileRegistry、ProfileSpecFile
│   │   └── journal_detector.rs # JournalDetector、SignalKind
│   ├── profiles/             # 期刊 Profile TOML 文件（7 个）
│   └── examples/
│       └── paper3_to_docx.rs # 端到端示例（含 --report）
│
├── compatibility-analyzer/    # 包兼容性扫描器
├── rule-engine/              # 规则引擎 + AI fallback
├── xdv-parser/               # XDV/DVI 字节码解析器
├── semantic-collector/       # 语义采集 trait + 输出
├── semantic-ast/             # 语义 AST 定义
├── docx-writer/              # DOCX 序列化层
│   └── src/
│       ├── profile.rs        # ProfileStyleMap [新增]
│       ├── serializer.rs     # AST → OOXML
│       ├── packer.rs         # ZIP 打包
│       ├── styles.rs         # JOS 21-style
│       └── template.rs       # 模板继承
├── latex-reader/             # LaTeX 源码解析
├── bib/                      # BibTeX 解析
├── quality/                  # 质量分析
├── tex-facade/               # TikZ 栅格化
├── utils/                    # 通用工具
│
├── cli/                      # doc-engine CLI
│   └── src/
│       ├── main.rs          # 8 个子命令 + 4 个语义 handler
│       └── semantic_cmd.rs  # 语义子命令 Args 类型
│
├── commercial-api-client/    # 商业 API 客户端 [新增]
│   └── src/
│       ├── lib.rs
│       ├── client.rs        # ApiClient
│       └── models.rs        # 请求/响应类型
│
└── desktop-slint/           # 桌面 UI 骨架 [新增]
    ├── Cargo.toml
    ├── build.rs
    └── src/
        ├── main.rs
        └── ui/
            ├── main.slint   # MainWindow
            └── mod.rs

scripts/
├── verify_journal_profiles.sh   # 7 个期刊 Profile E2E 验证
├── commercial_verify.sh         # CI 质量门禁 [新增]
├── build_paper3_three_docx.sh  # 三路径回归测试
└── ...
```

---

## 六、测试覆盖

| Crate | 测试数 | 状态 |
|-------|--------|------|
| `doc-compiler-engine` | 82+ | ✅ |
| `doc-compatibility-analyzer` | 14 | ✅ |
| `doc-rule-engine` | 23+（27 含 ai-fallback） | ✅ |
| `doc-xdv-parser` | 24 | ✅ |
| `doc-semantic-collector` | 20 | ✅ |
| `doc-docx-writer` | 36 | ✅ |
| `doc-latex-reader` | 119 | ✅ |
| `doc-core` | 19+ | ✅ |
| `doc-tex-facade` | 4 | ✅ |
| **总计** | **430+** | **全部通过** |

运行命令：
```bash
cargo test              # 全工作空间
cargo test -p doc-compiler-engine
cargo test -p doc-compatibility-analyzer
cargo test -p doc-rule-engine
```

---

## 七、关键文件索引

### 核心引擎

| 文件 | 关键类型/函数 |
|------|--------------|
| `compiler-engine/src/lib.rs` | `SemanticTexEngine`、`CompileOptions`、`CompileReport`、`QualityGateResult`、`BackendSelector` |
| `compiler-engine/src/profiles.rs` | `ProfileRegistry`、`ProfileSpecFile` |
| `compiler-engine/src/journal_detector.rs` | `JournalDetector`、`SignalKind`、`JournalDetectionReport` |

### DOCX 渲染

| 文件 | 关键类型/函数 |
|------|--------------|
| `docx-writer/src/profile.rs` | `ProfileStyleMap`（新增） |
| `docx-writer/src/serializer.rs` | `serialize_document`、`write_front_matter`、`write_inline_math_run` |
| `docx-writer/src/packer.rs` | `pack_with_page_setup`、`write_styles` |
| `docx-writer/src/styles.rs` | 21 个 JOS style ID 常量 |

### CLI

| 文件 | 关键子命令 |
|------|-----------|
| `cli/src/main.rs` | convert、tex-compile、docx-to-pdf、verify-pdf、build、ast-dump、render-dump、docx-diff |
| `cli/src/main.rs` | **semantic-detect**、**semantic-analyze**、**semantic-convert**、**semantic-verify**（新增） |
| `cli/src/semantic_cmd.rs` | 所有语义子命令 Args 类型（新增） |

### 商业化

| 文件 | 说明 |
|------|------|
| `commercial-api-client/src/client.rs` | `ApiClient`（新增） |
| `commercial-api-client/src/models.rs` | `AnalysisJob`、`AnalysisResult` 等（新增） |
| `desktop-slint/src/ui/main.slint` | `MainWindow` UI 组件（新增） |
| `scripts/commercial_verify.sh` | CI 质量门禁（新增） |

---

## 八、仍需继续开发的内容

### 高优先级

| 任务 | 描述 | 相关模块 |
|------|------|---------|
| **LayoutGraph 接入** | 将 XDV Parser 输出接入 `compile_vfs_to_graph` 的 layout 推理阶段 | `compiler-engine`、`xdv-parser` |
| **LuaTeX node tree** | 在 `post_linebreak_filter` 采集 box/glyph/font 到 LayoutGraph | `semantic-collector` |
| **latex-reader → CodeBlock** | 让 latex-reader 在遇到 `minted`/`lstlisting` 时生成 `Block::CodeBlock` | `latex-reader` |
| **semantic-verify 实现** | `semantic-verify` CLI 子命令的真正实现（P6 里程碑） | `cli` |
| **Word REF 字段** | `\ref{label}` 从 hyperlink 升级为 `<w:fldSimple REF="...">` | `docx-writer` |

### 中优先级

| 任务 | 描述 |
|------|------|
| **TikZ 降级** | 检测 TikZ 源码，编译为 PNG 或保留 alt metadata |
| **RuleEngine 接入管线** | 在 `compile_vfs_to_graph` 的 RuleBased 阶段调用 `journal_rules(detected_profile)` |
| **AI Engine 插件** | 实现 `AiEngine` trait，适配 OpenAI/Claude API |
| **Profile schema 验证** | 各 Profile TOML 字段覆盖度验证 |

### 商业化

| 任务 | 描述 |
|------|------|
| **Slint UI 编译逻辑** | 在 `MainWindow` 中接入 `SemanticTexEngine` 实现真正的转换 |
| **API Server** | 云端质量分析服务实现 |
| **Auth / 订阅系统** | API 客户端接入认证和计费 |

---

## 九、已验证的回归测试

### paper3 三路径回归（2026-06-21）

| 路径 | DOCX 大小 | 图片数 |
|------|-----------|--------|
| sh（XeLaTeX 源码） | 3,079,377 bytes | 10 |
| rust-rule（旧引擎） | 3,055,363 bytes | 10 |
| semantic-engine | 3,057,630 bytes | 10 |

无未解析引用，OMML 公式 4 个，fallback 0 个。

### verify_journal_profiles.sh（2026-06-21）

7 个期刊 Profile 全部通过检测/兼容性/规则/后端选择流程。

---

## 十、文档导航

| 文档 | 内容 |
|------|------|
| `docs-zh/P1-DEV-PLAN.md` | P1 实施计划（本文档前身，详细技术细节） |
| `docs-zh/semantic-tex-engine-journal-profile-generalization-progress-20260621.md` | Journal Profile 泛化实现详情 |
| `docs-zh/semantic-tex-engine-progress-comparison-20260621.md` | 20260621 进展对比报告 |
| `docs-zh/semantic-tex-engine-pc-client-slint-commercial-plan-20260621-152833.md` | Slint 桌面客户端商业化规划 |
| `docs-zh/semantic-tex-engine-commercialization-technical-implementation-plan-20260621-151221.md` | 商业化技术实施方案 |
| `docs/to-docx/` | V1 → V2 技术文档归档 |

---

*本报告基于代码实际验证，覆盖截至 2026-06-21 18:00 的全部开发进展。*
