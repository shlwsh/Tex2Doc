# Tex2Doc Desktop 界面交互性、易用性、信息展示美化改进建议报告
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



**日期**：2026-06-22
**分析师**：资深产品经理视角
**评估范围**：`crates/desktop-slint` 全部 UI 页面（含 5 个 Tab、6 个 Slint 组件、类型定义及 Rust 绑定）

---

## 一、总评与定位

当前界面已完成**阶段 A~C 的多 TAB 重构**（从单页纵向堆叠升级为 Convert / Settings / Account / Billing / History 五个标签页），技术架构合理，具备完整 MVP 闭环。核心转换工作流已聚焦到 Convert Tab，其余管理能力拆分清晰。

**本次评估聚焦于**：在现有技术架构稳定的前提下，从**交互体验（Interaction）**、**易用性（Usability）**、**信息展示与美观（Information & Aesthetics）** 三个维度提出改进建议，帮助产品从「功能可用」升级为「体验优良」。

---

## 二、交互性（Interaction）改进建议

### 2.1 按钮状态反馈不足

**现状**：转换、登录、注册、Checkout 等异步操作按钮在触发后没有清晰的 `busy` 状态，用户点击后按钮仍然可点击或直接变为禁用，无法感知操作是否正在进行。

**改进建议**：

| 页面 | 操作 | 当前行为 | 建议改进 |
|---|---|---|---|
| Convert | Convert / Cloud Convert | 点击后 `is-converting=true` 禁用按钮自身，但未显示 loading/spinner | 按钮文字改为「Converting...」「Uploading...」，配合 Slint `AnimatedImage` 或文字动画模拟进度感 |
| Account | Login / Register / Refresh | 只更新 `account-status` 文本，按钮无变化 | 在点击瞬间将按钮文字改为「Signing in...」并禁用，结束后恢复 |
| Billing | Checkout / Portal | 点击后等待外部浏览器跳转 | 增加 toast 提示「Opening checkout... Please complete in browser」 |
| Settings | Save Settings | `settings-panel-state` 控制，但 Save 按钮点击后无明确 loading | 将按钮改为「Saving...」状态 |

**实现参考**：`ConvertTab` 中 `convert-clicked` callback 已有 `root.is-converting = true` 前置逻辑，但缺少按钮自身的 loading 文字变化。建议在每个异步操作按钮前增加一个 `is-busy-<action>` 属性来控制按钮文案。

### 2.2 文件选择交互路径冗长

**现状**：`Choose Folder` 和 `Choose Zip` 两个按钮各自触发独立的文件对话框，但输出路径需要再点一次 `Save As`。且主 TeX 文件输入框与项目路径在同一 GroupBox 内，信息密度过高。

**改进建议**：

1. **自动路径填充**：选择项目文件夹后，如果输出路径为空，应自动生成默认 DOCX 路径（`项目名.docx`），无需用户再点 `Save As`。当前 Rust 绑定中 `choose-project-folder-clicked` 已有 `output_path` 参数但 Slint 层未自动触发填充逻辑。

2. **Zip 包名作为默认输出名**：选择 zip 后，将 zip 文件名（去掉扩展名）作为默认 DOCX 输出名。

3. **主 TeX 输入框独立化**：主 TeX 文件名输入框（`main.tex`）属于云端转换的进阶选项，建议在 UI 上以折叠/展开小组件（`LineEdit` 叠加 `Text` 标签）呈现，不要与基础项目路径混在一起。

### 2.3 键盘导航与快捷键缺失

**现状**：界面无任何键盘快捷键支持，所有操作依赖鼠标。

**改进建议**（按优先级）：

| 快捷键 | 动作 | 理由 |
|---|---|---|
| `Ctrl+O` | 选择项目文件夹 | 行业惯例 |
| `Ctrl+Shift+S` | 保存设置 | 行业惯例 |
| `Ctrl+Enter` | 执行转换（当前 focus 的 Tab） | 提高效率 |
| `Tab` | 在输入框之间切换（需 Slint 支持） | 基础可访问性 |
| `Escape` | 取消正在进行的转换 | 防止用户误操作 |

> 注：Slint 对键盘事件的支持有限，建议首期先在 `LineEdit` 上增加 `accepted` 事件处理（即回车键触转换），作为最小可行的键盘优化。

### 2.4 转换进度感知薄弱

**现状**：转换分为 3 个阶段（`[1/3] Reading...`），但 `ProgressIndicator` 只有整体进度的 `0.0→1.0`，没有分阶段进度反馈，用户无法感知「正在解析」「正在转换」「正在写入」各花了多久。

**改进建议**：

1. **阶段进度细分**：在 `conversion-progress` 之外增加 `conversion-stage`（`enum { Reading, Converting, Writing }`），UI 根据阶段显示不同文字。
2. **时间估算**：如果 Rust 侧能提供预估时间，显示「预计还需 10 秒」可显著提升用户体验。
3. **取消按钮**：`is-converting=true` 时，当前仅禁用按钮，建议增加一个明确的 `Cancel` 按钮，终止后台线程并清理状态。

---

## 三、易用性（Usability）改进建议

### 3.1 转换 Tab 的主次操作层级不清晰

**现状**：转换 Tab 操作栏有 5 个按钮（Detect Profile / Convert / Cloud Convert / Open Output / Open Report），且 `Cloud Convert` 在 `conversion-mode=cloud` 时又被另一个 Convert 按钮覆盖，导致两个主要转换按钮并列，用户困惑。

```
[Detect Profile]  [Convert]  [Cloud Convert]  [Open Output]  [Open Report]
```

**问题分析**：

- `Convert` 和 `Cloud Convert` 两个按钮同时存在，在 engine mode=cloud 时 `Convert` 被禁用但仍可见，造成「有两个转换按钮」的认知负担
- `Open Output` 和 `Open Report` 属于转换完成后的辅助操作，不应与转换主按钮并列

**改进建议**：

采用**单主按钮 + 引擎切换开关**的设计：

```
┌─ Engine ─────────────────────┐
│ [Local]  [Cloud]    Account: pro@tex.com (Quota: 47/200) │
└─────────────────────────────────┘

[Detect Profile]        [Convert ▶]        [Open Output]  [Open Report]
```

- 主按钮文字根据当前引擎模式动态变化：Local 模式下显示「Convert」，Cloud 模式下显示「Cloud Convert」
- 账号信息（Display Name + Quota）在转换 Tab 顶部直接可见，消除云端转换前需要跳转账号 tab 的认知摩擦
- `Open Output` 和 `Open Report` 可折叠到 Report 组内，初始隐藏，转换完成后才展开

### 3.2 账号 Tab 的操作入口混乱

**现状**：Account Tab 包含两套登录体系（`Sign In` GroupBox + `Account` GroupBox），且 `API Base URL` 在账号页和 Settings 页重复出现。

**改进建议**：

1. **合并 Sign In 与 Account**：删除第二个 `Account` GroupBox，将 `Usage` 按钮移入 `Sign In` GroupBox 内。
2. **API Base URL 统一放在 Settings Tab**：账号 Tab 不再出现 API Base URL 字段，由 Settings Tab 统一管理，Sign In GroupBox 直接读取 `root.api-base-url`。
3. **密码字段安全性**：当前密码输入框无密码模式（`input-type`），建议在 Slint 中为 `LineEdit` 设置 `input-type: password`（如果 Slint 支持）。同时，登录成功后应清空 `login-password`（当前代码中未执行）。

### 3.3 套餐 Tab 的 GroupBox 命名重复

**现状**：Billing Tab 中有两个 `Plan` GroupBox（Plans 列表和 Plan ID 输入区），命名高度相似，用户容易混淆。

**改进建议**：

| 当前标题 | 建议改为 |
|---|---|
| `Plans (Phase D.2)` | `Plan Catalog` |
| `Plan` | `Quick Actions` 或 `Subscribe / Manage` |

### 3.4 Settings Tab 中的 Profile 和 Quality 重复

**现状**：Profile 和 Quality 在 Convert Tab 的 Options 组和 Settings Tab 的 Engine Profile 组中各出现一次，完全一致。这在用户填写后跳转到 Settings 又改一次时容易产生「到底以哪个为准」的困惑。

**改进建议**：

1. **明确默认值概念**：Settings Tab 的 Profile/Quality 是「默认转换参数」，应在 UI 上明确标注「Default Profile」「Default Quality」文字。
2. **Convert Tab 优先**：用户当前在 Convert Tab 选择的值应覆盖 Settings 中的默认值。建议在 Convert Tab 操作按钮时，以 Convert Tab 当前值为准。
3. **视觉区分**：Settings Tab 的输入控件样式与 Convert Tab 保持一致，但 label 颜色稍淡（`color: #888`）以表达「次要」语义。

### 3.5 云端转换的前置条件提示不直观

**现状**：`Cloud Convert` 按钮的 `enabled` 条件是：

```slint
enabled: api-base-url != "" && project-path != "" && output-path != "" && !is-converting
```

未登录时按钮直接禁用，用户不知道是哪个条件未满足。

**改进建议**：

将按钮 disabled 状态时的 tooltip 或 adjacent 文字动态化：

```slint
Button {
    text: "Cloud Convert";
    enabled: api-base-url != "" && project-path != "" && output-path != "" && !is-converting
        && conversion-mode == ConversionMode.cloud
        && is-signed-in;
    // 增加一行文字提示
    Text {
        visible: !self.enabled;
        text: is-signed-in ? "Fill all fields above" : "Sign in to use cloud engine";
        color: #c04040;
        font-size: 11px;
    }
}
```

### 3.6 历史记录 JobRow 选中体验差

**现状**：`HistoryTab` 中的 `JobRow` 选中后需要手动点 `Select` 按钮，然后再通过 `selected-job-index` 触发后续操作。没有行直接点击交互。

**改进建议**：

1. **点击行直接选中**：在 `ListView` 的行上增加 `clicked` 事件，选中该行后高亮。
2. **双击打开输出**：增加双击事件，直接触发 `open-output-clicked`。
3. **右键菜单**（如果 Slint 支持）：提供 Open Output / Open Report / Remove 三个快捷操作。

---

## 四、信息展示与美观（Aesthetics）改进建议

### 4.1 视觉风格缺乏品牌个性

**现状**：界面完全依赖 Slint `std-widgets` 默认样式，无配色方案、无圆角系统、无字体层级规划，整体呈现「灰底白底控件堆叠」的办公软件感。

**改进建议**：

#### 4.1.1 配色方案

| 角色 | 颜色 | 应用场景 |
|---|---|---|
| Primary | `#2563EB`（蓝） | 主要按钮背景、主 Tab 高亮 |
| Secondary | `#6366F1`（靛蓝） | 云端引擎开关激活态 |
| Success | `#16A34A`（绿） | 转换成功、Succeeded 状态 |
| Warning | `#D97706`（橙） | 转换中、Pending 状态 |
| Error | `#DC2626`（红） | Failed 状态、验证错误 |
| Background | `#F8FAFC`（浅灰） | 窗口背景 |
| Surface | `#FFFFFF`（白） | GroupBox/Card 背景 |
| Text Primary | `#1E293B`（深灰） | 主要文字 |
| Text Secondary | `#64748B`（中灰） | 标签、次要文字 |
| Border | `#E2E8F0`（边框） | GroupBox、分组线 |

#### 4.1.2 圆角与间距

当前所有元素均无圆角。建议：

- GroupBox/Card：`border-radius: 8px`
- Button：`border-radius: 6px`
- 输入框：`border-radius: 4px`
- 页面 padding：`20px`
- 组间距：`16px`

#### 4.1.3 字体层级

| 层级 | 字号 | 样式 | 用途 |
|---|---|---|---|
| H1（页面标题） | 18px | Bold | Tab 标题（通过 Tab title） |
| H2（分组标题） | 13px | Bold | GroupBox 标题 |
| Body | 12px | Regular | 正文文字 |
| Caption | 11px | Regular | 标签、次要说明 |
| Mono | 11px | Monospace | 路径、JSON、Status 日志 |

### 4.2 Report 组信息密度低

**现状**：Report 组显示 4 个指标（Profile / Compatibility / Quality / Confidence），每个只有一行文字，空旷感强。

**改进建议**：

将 Report 组改造为**指标卡片（Metric Card）** 布局：

```
┌──────────────┬──────────────┬──────────────┬──────────────┐
│   Profile    │ Compatibility│   Quality    │  Confidence  │
│   generic    │     78%      │  standard(85)│     High     │
│  (detected)  │   ████████░░ │   ████████░░ │  ████████░░  │
└──────────────┴──────────────┴──────────────┴──────────────┘
```

每个指标卡：
- 顶部：指标名称（`Text { font-size: 11px; color: #64748B }`）
- 中部：数值大字（`Text { font-size: 18px; font-weight: 700 }`）
- 底部：小型进度条（使用 `ProgressBar`，数值归一化到 0-1）

颜色语义：Compatibility 高（绿）/中（橙）/低（红）用渐进色。

### 4.3 账号信息展示不友好

**现状**：Account Tab 中 Display Name / Tier / Quota 以普通文字显示，无图形化。

**改进建议**：

```
┌─ Account ─────────────────────────────────────────────────┐
│  👤  zhangsan@email.com     │  Plan: Pro  │  Quota: 47/200 │
│                              │             │  ████████░░░  │
└────────────────────────────────────────────────────────────┘
```

- 将账号信息合并为一个横向信息卡，去掉 GroupBox 包裹
- Quota 使用小型进度条（`47/200` → 约 23%）
- Tier 以彩色徽章（Badge）呈现：Free=灰 / Pro=蓝 / Team=紫

### 4.4 套餐卡片（Plan Cards）美化

**现状**：Billing Tab 的 Plan 列表使用平铺的 `Rectangle` 实现，卡片感不足，features 文字直接跟在价格后面，扫描效率低。

**改进建议**：

```
┌─────────────────────────────────────────────────────────────┐
│ FREE                                  $0/mo                   │
│ 5 cloud conversions / month                                 │
│ Local conversions: Unlimited                               │
│                                       [Current]  [Choose ▶] │
└─────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────┐
│ PRO (Recommended)                    $19/mo                 │
│ 200 cloud conversions / month         Priority queue          │
│                                       [Current]  [Choose ▶] │
└─────────────────────────────────────────────────────────────┘
```

- Recommended 方案加左边竖线色标（`#2563EB`）
- Tier 名称左对齐 Bold，价格右对齐 Bold
- Features 列表化（多行 `• feature text`）
- Choose 按钮改为紧凑小按钮，放在卡片右侧

### 4.5 状态日志（Status TextEdit）视觉改进

**现状**：`TextEdit` 只读显示日志，`min-height: 120px`，字体等宽，缺少视觉层次。

**改进建议**：

1. **语法高亮**（如果 Slint 支持）：用不同颜色区分 `[1/3]` 阶段标签、`--ERROR--`、`--DONE--`。
2. **时间戳**：Rust 侧推送日志时增加时间戳前缀（`[14:32:01] `），UI 显示更专业。
3. **日志折叠**：对于长日志（>50 行），初始只显示最后 20 行，顶部有「Show all」展开按钮。
4. **复制按钮**：在 TextEdit 旁增加一个「Copy」按钮，用户可以一键复制完整日志用于反馈。

### 4.6 Tab 标题与内容对齐

**现状**：5 个 Tab 标题为英文（Convert / Settings / Account / Billing / History & Diag.），与代码中中文注释不一致，且国内用户习惯中文 Tab。

**改进建议**：

| 当前标题 | 建议中文 |
|---|---|
| Convert | 转换 |
| Settings | 配置 |
| Account | 账号 |
| Billing | 套餐 |
| History & Diag. | 历史与诊断 |

> 如果产品有国际化需求，建议通过 Slint 的 `tr()` 机制实现 i18n，但首期直接替换为中文文字成本最低。

### 4.7 窗口标题与品牌

**现状**：`MainWindow` 标题为 `title: "Tex2Doc Desktop"`，无版本号、无账号状态指示。

**改进建议**：

```
title: is-signed-in ? "Tex2Doc — " + account-display-name
                     : "Tex2Doc Desktop (v1.x.x)"
```

让用户始终知道当前登录状态。

---

## 五、建议优先级矩阵

按**实施成本**（低/中/高）和**用户体验提升**（小/中/大）两维度排列：

| 优先级 | 建议 | 成本 | 体验提升 |
|---|---|---|---|
| P0 | 主转换按钮合并为单按钮（动态文案） | 低 | 大 |
| P0 | Cloud Convert 前置条件友好提示 | 低 | 大 |
| P0 | 转换 Tab 顶部显示账号状态与 Quota | 低 | 大 |
| P0 | 中文 Tab 标题 | 低 | 中 |
| P1 | 按钮 busy 状态（loading 文案） | 低 | 中 |
| P1 | 选中项目后自动填充输出路径 | 中 | 大 |
| P1 | 密码字段安全处理（清空 + 密码模式） | 低 | 中 |
| P1 | Report 指标卡片化 + 进度条 | 中 | 中 |
| P1 | 日志区增加时间戳 | 低 | 中 |
| P2 | 配色与圆角品牌化改造 | 中 | 中 |
| P2 | 套餐 Plan 卡片美化 | 中 | 小 |
| P2 | 账号信息卡（Quota 进度条 + Badge） | 中 | 小 |
| P2 | 键盘快捷键支持（回车触转换） | 低 | 小 |
| P3 | 取消转换按钮 | 中 | 中 |
| P3 | 历史任务双击打开 | 中 | 小 |
| P3 | 窗口标题动态显示账号 | 低 | 小 |
| P3 | 配色系统定义（CSS 变量风格） | 中 | 中 |

---

## 六、总结

Tex2Doc Desktop 当前已完成从单页到多 TAB 的架构重构，核心功能链路完整。在现有稳定架构上，**P0~P1 改进可在 1~2 个工作日内完成**，主要聚焦于：

1. **Convert Tab 操作区重构**：单主按钮 + 账号状态可见
2. **按钮状态与提示**：loading 文案 + 前置条件友好提示
3. **Report 组仪表化**：指标卡片 + 进度条
4. **中文本地化**：Tab 标题 + 关键提示文字

**P2 及以后的改进**（配色品牌化、套餐卡片、键盘快捷键、取消按钮等）建议在 P0~P1 完成后，作为下一迭代周期的工作项，在实际用户反馈中验证优先级。

---

*报告生成时间：2026-06-22*
