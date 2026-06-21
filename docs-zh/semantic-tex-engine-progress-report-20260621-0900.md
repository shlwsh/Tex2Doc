# Semantic TeX Engine 进展报告与商业化规划（20260621-0900）

**基准版本**：`docs-zh/semantic-tex-engine-progress-report-20260621-083000.md`
**当前版本**：2026-06-21 上午（整合 3 路探索代理精确发现）
**报告生成**：2026-06-21 09:00

---

## 一、总体进展概览

| 维度 | 基准（20260621 08:00） | 当前（20260621 09:00） | 变化 |
|------|----------------------|----------------------|------|
| P1 代码块检测 | ✅ 完成 | ✅ 完成（探索核实） | — |
| P1 RuleEngine 集成 | ✅ 完成 | ✅ 完成（探索核实） | — |
| P1 Word REF 字段 | ✅ 完成 | ✅ 完成 | — |
| P2 XDV → LayoutGraph | ✅ 完成 | ✅ 完成（探索核实） | — |
| P2 LayoutGraph 接入 | ✅ 完成 | ✅ 完成（探索核实） | — |
| **探索代理核实** | — | ✅ 3 路并行深度核实完成 | 新增 |
| M3/M4/M2 计划 | 初版 | ✅ 精确修正版 | 修订 |
| 工作空间测试 | 320+ 全通过 | 320+ 全通过 | 无回归 |

---

## 二、三路探索代理关键发现摘要

### 2.1 latex-reader 下沉结果（[Explore latex-reader lower.rs](4d8c4abf-ff42-455d-ab73-0868812decf8)）

- **Inline math 当前行为**：`$...$` 和 `\(` 都经 `replace_inline_math()` → `clean_math()` → Unicode 规范化文本（`\alpha`→`α`），**不是 OMML 公式**
- `split_inline_math()` 函数**已存在但从未被调用**（位于 `lower.rs` 第 669 行）
- 主循环通过 `latex_to_text()` normalizer 统一处理，`flush_paragraph()` 是唯一注入点
- **M3-1 结论**：只需在 `flush_paragraph()` 中接通 `split_inline_math()`，无需新增任何基础设施

### 2.2 xdv-parser 与 layout graph 探索结果（[Explore xdv-parser and layout graph](f03c5972-43b5-4265-b08e-d984aeeb4170)）

- `FontDefExt` / `NativeGlyph` / `NativeNode` **三个数据结构已定义**（`model.rs`），缺的是 `opcode.rs` Phase 2 解析
- LuaTeX hook **只用了** `post_linebreak_filter` + macro redefinition，没有 node 级别 callbacks
- **M4-2 结论**：需要全新注册 hlist_filter/vlist_filter 并写第二个 JSONL 流
- **M4-3 结论**：数据结构完备，只需实现 opcode 解析
- **M2-1 结论**：`XeLaTeXHookBackend` 骨架完整，仅需加 `-output-format=xdv` 参数

### 2.3 TikZ / tex-facade / compatibility 探索结果（[Explore TikZ, tex-facade, compatibility](3646619d-090b-4298-a1b9-8b77d3b05236)）

- `tex-facade` **完全没有 rasterize 能力**，需要新建 `rasterize.rs`
- `XeLaTeXHookBackend` 只 hook 了 `\includegraphics`，**不感知 TikZ 环境**
- **M4-1 结论**：需在 `lower_environment()` 中新增 `tikzpicture` 分支 + `rasterize_tikz_to_png()`
- **兼容性评分**：TikZ 当前为 `unsupported`，实现 rasterize 后降为 `warning`
- `TectonicBackend` 单命令编译 + 自动处理 passes + bibtex，**优于** `XelatexBackend` 多轮轮询

---

## 三、M3/M4/M2 计划精确修订版

### 已确认可立即实施的任务（基于探索结果）

| 任务 | 实施难度 | 风险 | 关键文件 | 预估工时 |
|------|---------|------|---------|---------|
| **M3-1** Inline math → OMML | 低 | 低 | `lower.rs`（`flush_paragraph`） | 1 day |
| **M3-2** TOML Profile | 低 | 低 | `profiles.rs` + `profiles/*.toml` | 1 day |
| **M3-3** multirow/colspan | 中 | 中 | `lower_table()` | 2 days |
| **M2-1** XeLaTeX XDV | 低 | 低 | `collect_runtime_events()` | 0.5 day |
| **M2-3** RuleOutput 路由 | 中 | 中 | `resolve_raw_fallback()` | 2 days |
| **M4-3** XDV NativeFont | 中 | 高（opcode 标准文档） | `opcode.rs` + `layout_graph.rs` | 3 days |
| **M4-1** TikZ rasterize | 中 | 中（依赖 tex-facade 新建） | `rasterize.rs` + `lower_environment()` | 3 days |
| **M4-2** LuaTeX node 采集 | 高 | 高（Lua API 版本兼容性） | `LUALATEX_SEMANTIC_HOOK` 扩展 | 4 days |
| **M2-2** AI fallback | 中 | 中（feature gate 设计） | `rule-engine` 新 feature | 2 days |

---

## 四、商业化与产品化规划

### 4.1 当前产品能力定位

Tex2Doc 定位为**学术论文 TeX→DOCX 专业转换工具**，核心差异：

- 纯 Rust 本地化解析（无需重型 TeX 发行版，WASM 可在浏览器运行）
- 语义块 AST 模型（结构化、可扩展）
- 多引擎后端（Rule-based / XeLaTeX / LuaLaTeX 自适应）
- 兼容分析器（预检测文档兼容性）

### 4.2 Web 服务商业化方案

#### 目标形态

提供 **tex2doc.app**（Web 端）+ **API 服务**，面向：
- 研究人员（单次转换）
- 实验室/课题组（批量、共享）
- 出版服务商（高频、大批量）
- 期刊平台集成（API 嵌入）

#### 定价套餐设计

| 维度 | 免费版（Free） | 专业版（Pro） | 团队版（Team） | 企业版（Enterprise） |
|------|------------|------------|------------|-----------------|
| **价格** | ¥0 | ¥29/月 | ¥99/月 | ¥399/月 或询价 |
| **转换次数** | 10 次/月 | 200 次/月 | 1000 次/月 | 不限 |
| **单文件大小** | 2 MB | 20 MB | 50 MB | 200 MB |
| **批量转换** | ❌ | ❌ | ✅ 50 次/批 | ✅ 200 次/批 |
| **期刊模板** | 3 种 | 20 种 | 50 种 | 不限 + 自定义上传 |
| **行内公式** | Unicode 文本 | ✅ OMML 降级 | ✅ OMML 降级 | ✅ OMML 降级 |
| **TikZ 图形** | ❌ | ❌ | ✅ PNG rasterize | ✅ PNG rasterize |
| **复杂表格** | 基本 | ✅ multirow/colspan | ✅ multirow/colspan | ✅ multirow/colspan |
| **协作功能** | ❌ | ❌ | ✅ 团队共享模板 | ✅ SSO + 审计日志 |
| **优先级** | 共享队列 | 优先队列 | 专属快车道 | 独立计算资源 |
| **支持** | 社区 | 邮件支持 | 工作日响应 | 7×24 SLA |
| **发票** | ❌ | 电子发票 | 电子发票 | 纸质发票 |

#### 按量付费（Pay-as-you-go）

| 套餐 | 价格 | 说明 |
|------|------|------|
| 单次包 | ¥3/次 | 无月费，有效期 30 天 |
| 100 次包 | ¥199 | 无月费，有效期 180 天 |
| 500 次包 | ¥799 | 无月费，有效期 365 天 |

#### 计费核心技术指标

```rust
// 转换定价因子
struct ConversionPricing {
    base_fee: f64,           // 基础费：¥0.5
    page_count_fee: f64,     // 按页：¥0.02/页
    math_run_fee: f64,       // 行内公式：¥0.05/个（OMML 降级）
    tikz_raster_fee: f64,    // TikZ rasterize：¥0.3/张
    table_cell_fee: f64,     // 复杂表格单元格：¥0.01/个
    turnaround: Turnaround,  // 即时 / 2h / 24h
}
```

### 4.3 多终端部署架构

```
                         ┌─────────────────────────────────────────┐
                         │            tex2doc.app (Web)            │
                         │   Flutter Web + Rust WASM (核心库)       │
                         └──────────────┬──────────────────────────┘
                                        │ HTTPS API
              ┌─────────────────────────┼──────────────────────────┐
              │                         │                          │
    ┌─────────▼──────────┐    ┌─────────▼──────────┐    ┌────────▼──────────┐
    │   Flutter iOS/Android │    │  Flutter Windows/Linux/macOS │   │   Chrome MV3 扩展   │
    │   移动端 App         │    │  桌面端 App          │    │   本地免上传转换   │
    │   (共享 WASM 核心)   │    │   (本地运行，无需联网) │    │   (文件越大越值)   │
    └─────────┬──────────┘    └─────────┬──────────┘    └────────┬──────────┘
              │                         │                          │
              └─────────────────────────┼──────────────────────────┘
                                        │
                         ┌──────────────▼──────────────────────┐
                         │           API Gateway               │
                         │  (Rate limit / Auth / Billing)       │
                         └──────┬────────────┬─────────────────┘
                                │            │
              ┌─────────────────▼──┐  ┌──────▼──────────────────┐
              │  Free Tier Workers  │  │  Pro/Team Workers     │
              │  (共享资源，限速)    │  │  (独立容器，优先级)    │
              └────────────────────┘  └───────────────────────────┘
                                │            │
              ┌─────────────────▼────────────▼──────────────────┐
              │            Compute Cluster                      │
              │  ┌─────────┐  ┌─────────┐  ┌─────────────────┐ │
              │  │XeLaTeX  │  │LuaLaTeX │  │  Tectonic       │ │
              │  │Pod      │  │Pod      │  │  Pod (无 TeX)    │ │
              │  └─────────┘  └─────────┘  └─────────────────┘ │
              └─────────────────────────────────────────────────┘
                                │
              ┌─────────────────▼──────────────────────────────┐
              │         Storage / Cache / CDN                   │
              │  PDF 缓存 / 用户文件 / 产物 CDN 分发             │
              └─────────────────────────────────────────────────┘
```

**各终端定价策略**：

| 终端 | 部署模式 | 定价策略 | 商业逻辑 |
|------|--------|---------|---------|
| **Web** | SaaS 云端 | 订阅制（见套餐表） | 主流量入口，货币化核心 |
| **移动 App** | 本地 WASM | 免费 + 内购（次数包） | 低边际成本获客 |
| **桌面端** | 本地可执行文件 | ¥99 买断 / 免费试用 | 高价值用户直接变现 |
| **Chrome 扩展** | 本地免上传 | 免费 + 内购 | 拦截用户意图，引导至 App |
| **API** | REST/GraphQL | 按调用量计费 | 平台型，开发者生态 |

### 4.4 泛化转换能力：多期刊支持路线图

#### 阶段一：模板 Profile 体系（对应 M3-2 完成后）

将 Profile 从 Rust 内置迁移为 TOML 外置，支持用户自定义：

```
profiles/
├── generic.toml          # 通用学术论文
├── jos-paper.toml        # IEEE/JOS 格式
├── tacl.toml             # TACL/ACL 格式
├── nature.toml           # Nature 格式
├── springer.toml         # Springer 格式
├── chinese-academic.toml # 中文学术论文
└── template.toml         # 用户上传自定义
```

每个 Profile 定义：
- 编译引擎偏好（XeLaTeX / LuaLaTeX / Tectonic）
- 兼容性阈值
- 特殊宏映射规则
- 输出格式选项（REF 字段 / hyperlink / OMML 等级）

#### 阶段二：智能期刊自动检测

```rust
// 自动检测 LaTeX 文档对应的期刊模板
struct JournalDetector {
    // 基于 documentclass 和 usepackage 推断
    // 基于 \begin{thebibliography} 格式推断引用风格
    // 基于 macro 使用模式推断模板类型
}
```

检测规则库（`journals/rules/`）：

| 期刊/模板 | 检测信号 | Profile ID |
|---------|---------|-----------|
| IEEE JOS | `\documentclass[journal]{IEEEtran}` | `jos-paper` |
| ACL/TACL | `\documentclass[aclang]{acl}` | `tacl` |
| CVPR/ICCV | `\documentclass[conference]{IEEEtran}` | `cvpr` |
| Nature | `\documentclass{nature}` | `nature` |
| Springer | `\documentclass{springer}` | `springer` |
| 中文学报 | `\documentclass{ctexart}` | `chinese-academic` |
| arXiv | 无特定 / 任意 | `generic` (降级) |

#### 阶段三：ML 辅助的语义块识别

当规则无法匹配时，调用轻量级 ML 模型辅助判断宏的语义类型：

```rust
// AI 辅助的未知宏分类（对应 M2-2 AI fallback）
enum MacroSemanticType {
    Figure,      // 图形引用
    Table,       // 表格引用
    Equation,    // 公式引用
    Citation,    // 引用（未识别格式）
    Heading,     // 小节标题
    Paragraph,   // 正文段落
    Ignore,      // 格式宏（忽略）
}
```

### 4.5 商业化时间线

```
Q3 2026（当前 ~6 月）
├── 完成 M3-1、M3-2、M3-3（文档高保真）
├── 完成 M2-1、M2-3（工程化基础）
├── 发布 tex2doc.app MVP（Web 端）
├── 实现期刊自动检测 v1（5 种模板）
└── 内部 Beta 测试（邀请制）

Q4 2026
├── 完成 M4-1（M4-3 视情况）
├── 完成 M2-2 AI fallback
├── 发布专业版订阅（Pro 套餐）
├── 发布移动端 App（iOS + Android）
├── 集成支付（Stripe / 微信 / 支付宝）
└── 启动开发者 API

Q1 2027
├── 完成 M4-2（LuaTeX node 采集）
├── 发布桌面端 App（Electron/Flutter）
├── 发布 Chrome 扩展
├── 团队版（Team）上线
├── 10+ 期刊模板
└── 高校/实验室批量授权

Q2 2027
├── ML 辅助语义识别
├── 企业版（Enterprise）定制化
├── 期刊平台 API 集成
├── 100+ 期刊模板
└── SOC 2 / 隐私合规（GDPR）
```

### 4.6 竞争优势与护城河

| 护城河 | 描述 | 建设方式 |
|-------|------|---------|
| **Rust WASM 核心** | 浏览器内运行，无需上传，保护隐私 | 持续优化核心库性能 |
| **语义 AST 模型** | 强类型结构化表示，可扩展到 Markdown/HTML/Typst | 扩展 Writer 层 |
| **多引擎自适应** | Rule-based / XeLaTeX / LuaLaTeX 自动选择 | 完善兼容性分析器 |
| **期刊模板生态** | 100+ 期刊 profile 沉淀 | 社区贡献 + 商业合作 |
| **TikZ rasterize** | 学术图形免手动重绘 | 技术护城河（难实现） |
| **学术引用图谱** | ReferenceGraph 关联分析 | 数据网络效应 |

---

## 五、下步行动计划

### 立即可执行（本周）

1. **M3-1**：`flush_paragraph()` 接通 `split_inline_math()`，实现 `$...$` → OMML
2. **M3-2**：Profile TOML 外置化
3. **更新进展报告**：生成 `docs-zh/semantic-tex-engine-progress-report-20260621-0900.md`

### 短期（1-2 周）

4. **M3-3**：复杂表格 multirow/colspan
5. **M2-1**：XeLaTeX XDV 输出采集
6. **M2-3**：RuleOutput → Block 路由

### 中期（1 个月）

7. **M4-1**：TikZ rasterize pipeline
8. **M4-3**：XDV NativeFont Phase 2 解析
9. **Web 产品**：tex2doc.app MVP 部署

### 长期（3 个月）

10. **M4-2**：LuaTeX node 采集
11. **M2-2**：AI fallback feature
12. **移动/桌面端**：Flutter App 发布
13. **支付集成**：订阅 + 按量付费

---

## 六、变更文件清单

| 操作 | 文件 | 说明 |
|------|------|------|
| 新增 | `docs-zh/semantic-tex-engine-progress-report-20260621-0900.md` | 本报告 |
| 修改 | `.cursor/plans/semantic-tex-engine-remaining-impl_f01698d8.plan.md` | M3/M4/M2 计划精确修订版 |
