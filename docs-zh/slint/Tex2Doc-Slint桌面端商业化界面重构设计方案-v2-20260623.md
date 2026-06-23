# Tex2Doc Slint 桌面端商业化界面重构设计方案

日期：2026-06-23
版本：v2（易用性与美观度增强版）

> 本文档基于 v1 方案（`Tex2Doc-Slint桌面端商业化界面重构设计方案-20260623.md`）增强，聚焦提升前端界面的易用性与美观度。增强内容已在正文中以 **\[v2 新增\]** 标记。

---

## 一、项目概述

### 1.1 背景

Tex2Doc 桌面端采用 Slint 框架构建原生跨平台 UI，当前已实现登录注册、充值套餐、转换、历史记录等商业化核心功能。然而现有界面存在大量问题：设计语言不一致、代码重复严重、状态管理分散、组件无复用、国际化覆盖不全、主题切换粒度粗、交互状态缺失等问题日益突出，与商业化产品标准差距明显。

Flutter Web 端已完成商业化工作台的重构并验证了设计方向（参见 `docs-zh/ui/web-commercial-workbench-progress-report-20260623.md`），其设计经验为本方案提供了重要参考。

### 1.2 目标

对 `crates/desktop-slint/src/ui/` 目录下的 Slint 前端进行系统性重构优化，输出可落地执行的详细设计方案文档，最终实现商业化级别的桌面端用户体验：

- **设计系统化**：建立统一的 Design Token 体系，支撑多主题、多语言、多暗色模式。
- **组件模块化**：提取可复用组件，消除重复代码，统一交互规范。
- **状态安全化**：集中管理 UI 状态，完善 Normal / Loading / Empty / Error / Disabled / Permission-Denied 全链路覆盖。
- **交互精细化**：定义动效规范、hover/active/pressed 视觉反馈、骨架屏与微加载。
- **商业功能闭环**：整合充值、账号、转换、记录四大模块，提供完整用户旅程。

### 1.3 范围

| 分类 | 内容 |
|------|------|
| 设计文件 | `crates/desktop-slint/src/ui/main.slint`（主窗口，825行） |
| 子页面 | `pages/convert.slint`、`pages/account.slint`、`pages/billing.slint`、`pages/history.slint`、`pages/settings.slint` |
| 共享类型 | `ui/types.slint` |
| 后端绑定 | `src/ui_bindings/*.rs`、`src/theme.rs`、`src/i18n.rs`、`src/app_state.rs` |
| **不在范围** | 本地转换引擎逻辑、云端 API 客户端、构建脚本 |

---

## 二、现状审计

### 2.1 核心问题清单

#### 问题 1：代码重复极其严重

`main.slint` 是唯一的主窗口文件，825 行中包含全部 5 个 Tab 页面的内联实现。各子页面 `.slint` 文件（`pages/`）是 Phase D 期间的部分重构，但 `main.slint` 并未使用它们——两个版本并存。

具体重复模式：

- **颜色定义**：在 `main.slint` 中内联硬编码了 11 个颜色变量，但 `theme.rs` 已定义了完整的 `ThemePalette` 结构。Slint 端未引用 Rust 侧的颜色枚举。
- **引擎选择按钮组**：在 `main.slint` Convert Tab 和 `pages/convert.slint` 中各写了一遍，仅实现方式略有不同（前者用两个 Button，后者用 Switch）。
- **报告卡片区**：4 个指标卡片的布局在 `main.slint` 和 `pages/convert.slint` 中完全重复（Profile / Compatibility / Quality / Confidence）。
- **账号状态栏**：登录信息展示在 `main.slint` Account Tab 和 `pages/account.slint` 中各写一遍。
- **套餐列表渲染**：`pages/billing.slint` 使用 `for plan[idx] in plan-catalog` 循环渲染，但 `main.slint` 中的 Billing Tab 完全使用手写表单，无套餐列表。

#### 问题 2：Design Token 体系不完整 \[v2 新增\]

现有颜色体系在 Slint 侧定义了 11 个颜色属性：

```
color-window-bg / color-surface / color-surface-alt / color-border /
color-text-primary / color-text-secondary / color-text-muted /
color-accent / color-success / color-warning / color-danger
```

存在以下缺陷：

- **硬编码 hex 值**：所有颜色值以 `#RRGGBB` 字符串内联在 Slint 中，Rust 侧的 `ThemePalette` 完全未被 Slint 引用，导致主题切换时 Slint 端颜色不变。
- **无 typography token**：字号、字重、行高均以字面量散布在各个 Widget 中（如 `font-size: 20px`、`font-weight: 700`）。
- **无 spacing token**：padding 和 spacing 以 `8px`、`12px`、`16px` 等字面量重复出现，无法统一调整。
- **无 radius token**：`border-radius` 值在多处硬编码（如 Convert Tab 中 `8px`，Billing Tab 中 `4px`，History Tab 中 `3px`），不统一。
- **无 surface 层级**：所有面板使用同一背景色，缺少 `surface-level-1/2/3/4` 层级体系，无法构建视觉深度。
- **无 motion token**：无动画时长、缓动曲线、过渡效果的统一规范，导致交互反馈不统一。

#### 问题 3：状态分散（Property Drilling）

`main.slint` 集中了约 50 个 `in-out property`，这些属性在 Rust 侧通过 `ui_bindings` 模块批量写入。但问题在于：

- **未登录/离线状态未建模**：无网络状态、`is-billing-busy`、`is-account-busy` 等状态缺乏统一枚举。
- **属性冗余**：部分属性如 `recent-jobs` 是纯文本字符串（`"No recent jobs."`），而 Rust 侧已有 `AppState.jobs` 的结构化 `JobEntry` 列表，两者并存导致数据不一致风险。
- **`pages/` 子组件的状态同步**：当 `main.slint` 最终引用 `pages/` 组件时，需要将所有 50 个属性透传到子组件，属性接口膨胀。

#### 问题 4：国际化（i18n）覆盖不完整 \[v2 增强\]

`src/i18n.rs` 定义了 106 个 i18n key，涵盖主要 UI 文本。但审计发现：

- **`zh-Hans`/`zh-Hant`/`ja-JP` 回退到英文**：由于 Slint 1.16 ICU4X CJK 分割问题，代码中显式注释说明了中文/日文 UI 实际渲染英文。这是最严重的功能性缺陷之一。
- **hardcoded text 仍存在**：多处地方文本未通过 i18n key 引用，如：
  - `pages/account.slint`：GroupBox 标题 `"Account (Phase D.1)"`、`"Sign In"`、`"Display Name"`、`"Tier"` 等均为字面量。
  - `pages/settings.slint`：几乎所有 GroupBox 标题和按钮文字均为英文字面量。
  - `pages/billing.slint`：按钮文字 `"Current"`、`"Choose"`、`"Checkout"`、`"Portal"` 等。
  - `pages/history.slint`：按钮文字 `"Select"`、`"Remove"`、`"Clear All"`、`"Export Diagnostic Bundle"` 等。
  - `pages/convert.slint`：大部分文本为字面量。
- **i18n key 命名不规范**：key 未统一采用 `category.subcategory.key` 格式，难以维护和扩展。

#### 问题 5：状态覆盖不全 \[v2 显著增强\]

按商业 UI 标准，核心工作流需覆盖以下状态：

| 状态 | 当前覆盖 | v2 目标覆盖 |
|------|----------|------------|
| Normal（正常） | 部分覆盖 | 完整覆盖 |
| Loading（加载中） | 仅有 `is-converting`/`is-account-busy`/`is-billing-busy`，无骨架屏/占位 | 骨架屏占位 + Spinner + 进度条三档 |
| Empty（空态） | History Tab 有 `"No recent jobs."` 文本，其他页面无 | 各页面统一 `EmptyState` 组件 |
| Error（错误） | 无专门的错误提示组件/区域 | `AlertBanner` 覆盖全部错误场景 |
| Disabled（禁用） | Button `enabled` 属性覆盖了部分场景，但云端未登录时仅显示警告文本 | 明确建模 + 视觉降级（opacity 0.4 + 禁用交互） |
| Permission Denied（无权限） | 无明确建模 | 新增权限拒绝态（账号体系外用户访问受限资源） |
| Hover/Active/Pressed | 无 | 全部交互组件需覆盖 |
| Skeleton（骨架屏） | 无 | 复杂内容区加载前显示骨架屏 |
| **\[v2 新增\]** 网络离线态 | 无 | `AlertBanner` + 禁用云端功能 |
| **\[v2 新增\]** 配额耗尽态 | 无 | 转换按钮禁用 + 引导充值 |

#### 问题 6：`pages/` 重构不彻底

Phase D 的 `pages/` 子组件重构方向正确，但存在以下问题：

- `main.slint` 完全未引用任何 `pages/` 组件，导致两套实现并存。
- `pages/` 各组件的属性接口与 `main.slint` 不一致（如 `pages/account.slint` 没有 `is-account-busy` 属性）。
- `pages/` 中存在大量未使用 `in-out property` 暴露的硬编码文本（如 `"Engine Profile"`、`"Output Strategy"` 等 GroupBox 标题）。
- `pages/billing.slint` 引入了 `ListView`/`PlanEntry` 结构，但 `main.slint` 的 Billing Tab 未使用。

---

## 三、设计系统 \[v2 大幅增强\]

### 3.1 文件结构（目标）

```
crates/desktop-slint/src/ui/
├── tokens.slint              # 新增：Design Token 全局定义（含动效、层级）
├── motion.slint              # 新增：动效与过渡 Token 定义
├── components/               # 新增：可复用组件库
│   ├── card-metric.slint    # 指标卡片（Compatibility/Quality/Confidence 等）
│   ├── card-plan.slint       # 套餐卡片（带选中状态）
│   ├── card-job.slint        # 历史任务行
│   ├── row-action.slint      # 操作行（输入+按钮组合）
│   ├── row-select.slint       # 下拉选择行
│   ├── button-primary.slint  # 主按钮
│   ├── button-secondary.slint# 次按钮
│   ├── button-icon.slint     # 新增：图标按钮
│   ├── badge-status.slint    # 状态徽章（Pending/Running/Succeeded/Failed）
│   ├── badge-tier.slint      # 新增：套餐等级徽章（Free/Pro/Enterprise）
│   ├── panel-group.slint      # 可折叠面板（GroupBox 增强版）
│   ├── panel-section.slint    # 新增：普通面板区块（无折叠）
│   ├── empty-state.slint     # 空态提示组件
│   ├── skeleton-block.slint  # 新增：骨架屏块
│   ├── skeleton-line.slint   # 新增：骨架屏文本行
│   ├── loading-indicator.slint# 加载状态组件（Spinner + 进度条）
│   ├── alert-banner.slint     # 警告/错误提示条
│   ├── dialog-confirm.slint   # 确认对话框
│   ├── dialog-fullscreen.slint# 新增：全屏模态对话框
│   ├── toast-notify.slint     # 新增：轻量通知（右上角浮窗）
│   ├── divider.slint          # 新增：分隔线
│   ├── tab-item.slint         # 新增：Tab 栏条目（含图标+文字）
│   └── sidebar-nav.slint      # 新增：侧边栏导航（预留移动端）
├── pages/                    # 已有页面（需重构）
│   ├── convert.slint
│   ├── account.slint
│   ├── billing.slint
│   ├── history.slint
│   └── settings.slint
├── i18n-strings.slint        # 新增：i18n 字符串资源（替代 hardcoded text）
├── main.slint                # 已有（需大幅精简）
└── types.slint               # 已有（需扩展）
```

### 3.2 Design Token 定义 \[v2 增强版\]

所有视觉常量集中在 `tokens.slint`，通过 Slint 全局属性暴露给 Rust 主题引擎。

#### 颜色 Token — Surface 层级系统 \[v2 新增\]

采用 Material Design surface 层级理念，建立 Z 轴视觉深度：

```slint
// crates/desktop-slint/src/ui/tokens.slint
export global DesignTokens {
    // === Window / Background ===
    in-out property <color> token-window-bg:   #F6F8FB;

    // === Surface Levels (elevation hierarchy) ===
    in-out property <color> token-surface-1:      #FFFFFF;  // 最高层：卡片、对话框
    in-out property <color> token-surface-2:      #FFFFFF;  // 次高层：输入框、选中行
    in-out property <color> token-surface-3:      #F1F5F9;  // 第三层：侧边栏背景
    in-out property <color> token-surface-4:      #EBEEF3;  // 最低层：window-bg 以上的背景

    // === Semantic Surfaces ===
    in-out property <color> token-surface-overlay: #00000066; // 遮罩层（对话框背景）

    // === Border ===
    in-out property <color> token-border:       #D7DEE8;
    in-out property <color> token-border-strong: #B8C2D4; // 分割线、输入框边框
    in-out property <color> token-border-focus:  #2563EB; // 聚焦态边框

    // === Text ===
    in-out property <color> token-text-primary:   #172033;
    in-out property <color> token-text-secondary: #42526B;
    in-out property <color> token-text-muted:    #6B778C;
    in-out property <color> token-text-inverse:  #FFFFFF;  // 暗色背景上的文字
    in-out property <color> token-text-link:      #2563EB;  // 新增：链接色

    // === Accent / Brand ===
    in-out property <color> token-accent:       #2563EB;
    in-out property <color> token-accent-hover:  #1D4ED8;  // 新增：hover 态 accent
    in-out property <color> token-accent-pressed: #1E40AF;  // 新增：pressed 态 accent
    in-out property <color> token-accent-subtle: #EFF6FF;  // 新增：accent 浅色背景

    // === Semantic Status ===
    in-out property <color> token-success:       #0F8A5F;
    in-out property <color> token-success-subtle: #ECFDF5;
    in-out property <color> token-warning:       #B7791F;
    in-out property <color> token-warning-subtle: #FFFBEB;
    in-out property <color> token-danger:        #C2413A;
    in-out property <color> token-danger-subtle: #FEF2F2;
    in-out property <color> token-info:          #0369A1;
    in-out property <color> token-info-subtle:   #F0F9FF;

    // === Disabled / Muted ===
    in-out property <color> token-disabled-bg:   #F1F5F9;
    in-out property <color> token-disabled-text: #9CA3AF;
    in-out property <color> token-disabled-border: #E5E7EB;

    // === Overlay ===
    in-out property <color> token-overlay-light: #FFFFFF80; // 新增：浅色遮罩
    in-out property <color> token-overlay-dark:  #00000033;  // 新增：暗色遮罩

    // === Typography ===
    in-out property <length> token-font-size-xs:   10px;
    in-out property <length> token-font-size-sm:   11px;
    in-out property <length> token-font-size-base: 13px;
    in-out property <length> token-font-size-md:   14px;
    in-out property <length> token-font-size-lg:   16px;
    in-out property <length> token-font-size-xl:   20px;
    in-out property <length> token-font-size-2xl:  24px;
    in-out property <length> token-font-size-3xl:  30px;
    in-out property <length> token-font-weight-normal:    400;
    in-out property <length> token-font-weight-medium:    500;
    in-out property <length> token-font-weight-semibold:  600;
    in-out property <length> token-font-weight-bold:      700;
    in-out property <length> token-line-height-tight:     1.2;
    in-out property <length> token-line-height-base:      1.5;
    in-out property <length> token-line-height-relaxed:  1.75;

    // === Spacing (4px base unit) ===
    in-out property <length> token-space-0:  0px;
    in-out property <length> token-space-1:  4px;
    in-out property <length> token-space-2:  8px;
    in-out property <length> token-space-3:  12px;
    in-out property <length> token-space-4:  16px;
    in-out property <length> token-space-5:  20px;
    in-out property <length> token-space-6:  24px;
    in-out property <length> token-space-8:  32px;
    in-out property <length> token-space-10: 40px;
    in-out property <length> token-space-12: 48px;
    in-out property <length> token-space-16: 64px;

    // === Radius ===
    in-out property <length> token-radius-sm:   3px;
    in-out property <length> token-radius-md:    6px;
    in-out property <length> token-radius-lg:    8px;
    in-out property <length> token-radius-xl:    12px;
    in-out property <length> token-radius-full:  9999px; // 药丸形

    // === Layout ===
    in-out property <length> token-header-height:   64px;
    in-out property <length> token-sidebar-width:    200px;
    in-out property <length> token-content-max-width: 960px; // 新增：SaaS 内容最大宽度
    in-out property <length> token-panel-gap:        16px;   // 新增：面板间距
    in-out property <length> token-section-gap:      24px;   // 新增：区块间距

    // === Transition / Motion (refs motion.slint) ===
    in-out property <duration> token-transition-fast:   120ms; // hover、toggle
    in-out property <duration> token-transition-base:   200ms; // panel expand、fade
    in-out property <duration> token-transition-slow:    300ms; // page transition
    in-out property <string>  token-easing-standard:    "ease-out";   // 标准缓动
    in-out property <string>  token-easing-decelerate:  "cubic-bezier(0.0, 0.0, 0.2, 1.0)"; // 进入
    in-out property <string>  token-easing-accelerate:  "cubic-bezier(0.4, 0.0, 1.0, 1.0)"; // 退出
    in-out property <string>  token-easing-sharp:       "cubic-bezier(0.4, 0.0, 0.2, 1.0)"; // 快速过渡

    // === Icon ===
    in-out property <length> token-icon-size-sm:   14px;
    in-out property <length> token-icon-size-base: 16px;
    in-out property <length> token-icon-size-lg:   20px;
    in-out property <length> token-icon-size-xl:   24px;

    // === Progress ===
    in-out property <length> token-progress-height: 4px;
    in-out property <length> token-progress-radius: 2px;
}
```

#### 暗色主题 Token \[v2 新增\]

暗色主题需完整覆盖所有颜色 token，确保视觉一致性：

```slint
// tokens.slint 中额外定义 DarkDesignTokens global
export global DarkDesignTokens {
    // 在 Rust 侧通过 apply_dark_theme() 注入，与 DesignTokens 结构一致
    // 仅色值不同，属性名完全对齐

    in-out property <color> token-window-bg:    #0F1117;
    in-out property <color> token-surface-1:    #1A1F2E;
    in-out property <color> token-surface-2:    #212838;
    in-out property <color> token-surface-3:    #1E2433;
    in-out property <color> token-surface-4:    #171C28;
    in-out property <color> token-border:       #2D3748;
    in-out property <color> token-border-strong: #3D4A5C;
    in-out property <color> token-border-focus:  #3B82F6;

    in-out property <color> token-text-primary:   #E2E8F0;
    in-out property <color> token-text-secondary: #94A3B8;
    in-out property <color> token-text-muted:    #64748B;
    in-out property <color> token-text-inverse:  #0F1117;
    in-out property <color> token-text-link:      #60A5FA;

    in-out property <color> token-accent:        #3B82F6;
    in-out property <color> token-accent-hover:   #2563EB;
    in-out property <color> token-accent-pressed: #1D4ED8;
    in-out property <color> token-accent-subtle:  #1E3A5F;

    in-out property <color> token-success-subtle: #052E16;
    in-out property <color> token-warning-subtle: #271A06;
    in-out property <color> token-danger-subtle:  #2D0B0B;
    in-out property <color> token-info-subtle:    #082032;

    in-out property <color> token-disabled-bg:   #1E2433;
    in-out property <color> token-disabled-text: #475569;
    in-out property <color> token-disabled-border: #2D3748;
}
```

> **\[v2 交互设计原则\]**：暗色主题不只是"颜色取反"。暗色 UI 应使用低饱和度色、避免纯白文字、减少边框对比度、提升背景层次感。Slint 的 `DesignTokens` 全局属性在亮/暗切换时由 Rust 侧统一注入。

#### Rust 侧主题同步

`theme.rs` 的 `ThemePalette` 通过 `slint::invoke_from_ui` 或直接 `set_property` 更新 `tokens.slint` 中的 `DesignTokens` 全局属性：

```rust
// src/ui_bindings/theme.rs
pub fn apply_light_theme(ui: &MainWindow) {
    let t = ui.global::<DesignTokens>();
    t.set_token_window_bg(slint::Color::from_hex("#F6F8FB"));
    t.set_token_surface_1(slint::Color::from_hex("#FFFFFF"));
    t.set_token_surface_2(slint::Color::from_hex("#FFFFFF"));
    t.set_token_surface_3(slint::Color::from_hex("#F1F5F9"));
    t.set_token_surface_4(slint::Color::from_hex("#EBEEF3"));
    t.set_token_border(slint::Color::from_hex("#D7DEE8"));
    t.set_token_text_primary(slint::Color::from_hex("#172033"));
    t.set_token_text_secondary(slint::Color::from_hex("#42526B"));
    t.set_token_text_muted(slint::Color::from_hex("#6B778C"));
    t.set_token_accent(slint::Color::from_hex("#2563EB"));
    t.set_token_success(slint::Color::from_hex("#0F8A5F"));
    t.set_token_warning(slint::Color::from_hex("#B7791F"));
    t.set_token_danger(slint::Color::from_hex("#C2413A"));
    t.set_token_info(slint::Color::from_hex("#0369A1"));
    // ... 其余 token
}

pub fn apply_dark_theme(ui: &MainWindow) {
    let t = ui.global::<DesignTokens>();
    t.set_token_window_bg(slint::Color::from_hex("#0F1117"));
    t.set_token_surface_1(slint::Color::from_hex("#1A1F2E"));
    // ... 其余 dark token
}
```

### 3.3 i18n 字符串资源 \[v2 增强\]

在 `i18n-strings.slint` 中集中管理所有用户可见文本，key 格式统一为 `category.subcategory.key`。

#### i18n key 稳定性规范 \[v2 新增\]

为保证跨版本兼容性，定义以下 i18n key 命名规范：

| 规则 | 说明 |
|------|------|
| 层级深度 | 最多 4 级：`category.sub1.sub2.key` |
| key 命名 | 全部小写，单词间用连字符 `-` |
| 稳定性 | `common.*`、`status.*` 下的 key 永久稳定，不可删除或改名 |
| 产物标识 | 带有 `#` 后缀的 key 表示占位符（如 `plan.price#amount`） |
| 复数 | 复数形式加 `-count` 后缀（`history-item-count`） |

#### i18n 覆盖矩阵 \[v2 新增\]

| key 前缀 | 用途 | en | zh-Hans | zh-Hant | ja | fr | de |
|---------|------|-----|---------|---------|-----|-----|-----|
| `common.*` | 确定取消关闭等通用 | 100% | 100% | 100% | 100% | 100% | 100% |
| `tab.*` | Tab 标题 | 100% | 100% | 100% | 100% | 100% | 100% |
| `convert.*` | 转换页 | 100% | 100% | 100% | 100% | 100% | 100% |
| `account.*` | 账号页 | 100% | 100% | 100% | 100% | 100% | 100% |
| `billing.*` | 套餐页 | 100% | 100% | 100% | 100% | 100% | 100% |
| `history.*` | 历史页 | 100% | 100% | 100% | 100% | 100% | 100% |
| `settings.*` | 设置页 | 100% | 100% | 100% | 100% | 100% | 100% |
| `status.*` | 状态文字 | 100% | 100% | 100% | 100% | 100% | 100% |
| `error.*` | 错误信息 | 100% | 100% | 100% | 100% | 100% | 100% |
| `alert.*` | 警告横幅 | 100% | 100% | 100% | 100% | 100% | 100% |
| `empty.*` | 空态文案 | 100% | 100% | 100% | 100% | 100% | 100% |
| `toast.*` | 轻量通知 | 100% | 100% | 100% | 100% | 100% | 100% |

> 注：`zh-Hans`/`zh-Hant`/`ja` 通过 Rust 侧 `i18n.rs` 翻译函数直接写入 Slint i18n 全局属性，绕过 Slint ICU4X CJK 分割问题（详见第七节 i18n 扩展方案）。

#### i18n 字符串完整定义（核心 key）

```slint
// crates/desktop-slint/src/ui/i18n-strings.slint
export global I18n {
    in-out property <string> locale: "en";

    // === Tabs ===
    in-out property <string> tab-convert: "Convert";
    in-out property <string> tab-settings: "Settings";
    in-out property <string> tab-account: "Account";
    in-out property <string> tab-billing: "Plans";
    in-out property <string> tab-history: "History";

    // === Convert Page ===
    in-out property <string> convert-engine: "Engine";
    in-out property <string> convert-local: "Local";
    in-out property <string> convert-cloud: "Cloud";
    in-out property <string> convert-project: "Project";
    in-out property <string> convert-project-placeholder: "TeX project path or project zip...";
    in-out property <string> convert-folder: "Folder";
    in-out property <string> convert-zip: "Zip";
    in-out property <string> convert-main-tex-placeholder: "Main TeX for cloud conversion (optional)...";
    in-out property <string> convert-options: "Options";
    in-out property <string> convert-profile: "Profile";
    in-out property <string> convert-quality: "Quality";
    in-out property <string> convert-output: "Output";
    in-out property <string> convert-save-as: "Save As";
    in-out property <string> convert-detect: "Detect Profile";
    in-out property <string> convert-convert: "Convert";
    in-out property <string> convert-cloud-convert: "Cloud Convert";
    in-out property <string> convert-open-output: "Open Output";
    in-out property <string> convert-open-report: "Open Report";
    in-out property <string> convert-report: "Report";
    in-out property <string> convert-status: "Status";
    in-out property <string> convert-ready: "Ready. Enter a project path and click Convert.";
    in-out property <string> convert-step-upload: "1. Upload TeX Project";
    in-out property <string> convert-step-processing: "2. Cloud Processing";
    in-out property <string> convert-step-download: "3. Download DOCX";
    in-out property <string> convert-detecting: "Detecting profile...";
    in-out property <string> convert-converting: "Converting...";

    // === Account Page ===
    in-out property <string> account-sign-in: "Sign In";
    in-out property <string> account-register: "Register";
    in-out property <string> account-email: "Email";
    in-out property <string> account-password: "Password";
    in-out property <string> account-login: "Login";
    in-out property <string> account-logout: "Logout";
    in-out property <string> account-refresh: "Refresh";
    in-out property <string> account-overview: "Overview";
    in-out property <string> account-display-name: "Display Name";
    in-out property <string> account-tier: "Plan";
    in-out property <string> account-quota: "Quota";
    in-out property <string> account-recharge: "Recharge";
    in-out property <string> account-conversion-records: "Conversion Records";
    in-out property <string> account-recharge-records: "Recharge Records";
    in-out property <string> account-refresh-usage: "Refresh Usage";
    in-out property <string> account-signing-in: "Signing in...";
    in-out property <string> account-registering: "Registering...";
    in-out property <string> account-refreshing: "Refreshing...";
    in-out property <string> account-refreshing-usage: "Refreshing usage...";
    in-out property <string> account-not-signed-in: "Not signed in";
    in-out property <string> account-signed-out: "Signed out.";
    in-out property <string> account-quick-actions: "Quick Actions";

    // === Billing Page ===
    in-out property <string> billing-subscribe: "Subscribe / Manage";
    in-out property <string> billing-plans: "Plans";
    in-out property <string> billing-pay-per-use: "Pay Per Use";
    in-out property <string> billing-per-date: "Date-based";
    in-out property <string> billing-checkout: "Checkout";
    in-out property <string> billing-portal: "Billing Portal";
    in-out property <string> billing-current: "Current";
    in-out property <string> billing-choose: "Choose";
    in-out property <string> billing-loading: "Loading plans...";
    in-out property <string> billing-creating-checkout: "Creating checkout session...";
    in-out property <string> billing-recharge-records: "Recharge Records";
    in-out property <string> billing-no-records: "No recharge records.";
    in-out property <string> billing-per-use-tab: "Per Use";
    in-out property <string> billing-subscription-tab: "Subscription";
    in-out property <string> billing-records-tab: "Records";
    in-out property <string> billing-quota-remaining: "# remaining";
    in-out property <string> billing-mock-payment: "Mock Payment";

    // === History Page ===
    in-out property <string> history-title: "Conversion History";
    in-out property <string> history-no-jobs: "No conversion records yet.";
    in-out property <string> history-open-output: "Open Output";
    in-out property <string> history-open-report: "Open Report";
    in-out property <string> history-export: "Export Diagnostics";
    in-out property <string> history-clear-all: "Clear All";
    in-out property <string> history-exporting: "Exporting diagnostics...";
    in-out property <string> history-select-job: "Select a record to view details.";

    // === Settings Page ===
    in-out property <string> settings-service: "Service";
    in-out property <string> settings-api-url: "API Base URL";
    in-out property <string> settings-api-test: "Test Connection";
    in-out property <string> settings-api-test-success: "Connection successful";
    in-out property <string> settings-api-test-failed: "Connection failed";
    in-out property <string> settings-default-params: "Default Conversion Parameters";
    in-out property <string> settings-default-profile: "Default Profile";
    in-out property <string> settings-default-quality: "Default Quality";
    in-out property <string> settings-default-output: "Default Output Directory";
    in-out property <string> settings-updates: "Updates";
    in-out property <string> settings-check-update: "Check Update";
    in-out property <string> settings-appearance: "Appearance";
    in-out property <string> settings-language: "Language";
    in-out property <string> settings-theme: "Theme";
    in-out property <string> settings-theme-light: "Light";
    in-out property <string> settings-theme-dark: "Dark";
    in-out property <string> settings-apply: "Apply";
    in-out property <string> settings-about: "About";
    in-out property <string> settings-product: "Tex2Doc Desktop";
    in-out property <string> settings-version: "Version";
    in-out property <string> settings-save: "Save Settings";
    in-out property <string> settings-saved: "Settings saved.";
    in-out property <string> settings-dirty: "Unsaved changes";
    in-out property <string> settings-reset: "Reset";

    // === Common ===
    in-out property <string> common-cancel: "Cancel";
    in-out property <string> common-confirm: "Confirm";
    in-out property <string> common-copy: "Copy";
    in-out property <string> common-close: "Close";
    in-out property <string> common-loading: "Loading...";
    in-out property <string> common-error: "Error";
    in-out property <string> common-retry: "Retry";
    in-out property <string> common-detected: "detected";
    in-out property <string> common-yes: "Yes";
    in-out property <string> common-no: "No";
    in-out property <string> common-all: "All";
    in-out property <string> common-none: "None";
    in-out property <string> common-back: "Back";
    in-out property <string> common-next: "Next";
    in-out property <string> common-done: "Done";

    // === Status ===
    in-out property <string> status-pending: "Pending";
    in-out property <string> status-running: "Running";
    in-out property <string> status-succeeded: "Succeeded";
    in-out property <string> status-failed: "Failed";
    in-out property <string> status-idle: "Idle";
    in-out property <string> status-disabled: "Disabled";

    // === Error ===
    in-out property <string> error-network: "Network error. Please check your connection.";
    in-out property <string> error-api-unreachable: "API is unreachable. Please verify the API URL in Settings.";
    in-out property <string> error-unauthorized: "Unauthorized. Please sign in.";
    in-out property <string> error-quota-exceeded: "Quota exceeded. Please recharge your account.";
    in-out property <string> error-conversion-failed: "Conversion failed. Please try again or export diagnostics.";
    in-out property <string> error-invalid-path: "Invalid path. Please check the project path.";

    // === Alert ===
    in-out property <string> alert-sign-in-required: "Please sign in to use cloud conversion.";
    in-out property <string> alert-api-not-configured: "API URL not configured. Please set it in Settings.";
    in-out property <string> alert-quota-exceeded: "Your quota is exhausted. Please recharge to continue.";
    in-out property <string> alert-unsaved-changes: "You have unsaved changes.";

    // === Empty ===
    in-out property <string> empty-history: "No conversion records yet.";
    in-out property <string> empty-recharge-records: "No recharge records.";
    in-out property <string> empty-conversion-records: "No conversion records.";
    in-out property <string> empty-plans: "No plans available.";

    // === Toast ===
    in-out property <string> toast-conversion-complete: "Conversion completed successfully.";
    in-out property <string> toast-conversion-failed: "Conversion failed.";
    in-out property <string> toast-sign-in-success: "Signed in successfully.";
    in-out property <string> toast-sign-out-success: "Signed out.";
    in-out property <string> toast-settings-saved: "Settings saved.";
    in-out property <string> toast-recharge-success: "Recharge successful.";

    // === Tier Badge ===
    in-out property <string> tier-free: "Free";
    in-out property <string> tier-pro: "Pro";
    in-out property <string> tier-enterprise: "Enterprise";
}
```

Rust 侧 `i18n.rs` 的翻译函数通过回调更新 Slint 的 `I18n` 全局属性，实现主题/语言切换时 UI 文本实时更新，无需重新渲染整个窗口。

---

## 四、组件设计 \[v2 大幅增强\]

### 4.1 基础组件清单

#### 组件状态通用规范 \[v2 新增\]

每个可复用组件必须覆盖以下状态。Slint 不支持 CSS pseudo-class，本方案通过 Slint `states` 语法实现：

```slint
// 组件内部状态机模板
states [
    idle when !root.is-hovered && !root.is-pressed && root.is-enabled,
    hovered when root.is-hovered && root.is-enabled: {
        background: DesignTokens.token-surface-alt;
        cursor: pointer;
    },
    pressed when root.is-pressed && root.is-enabled: {
        background: DesignTokens.token-surface-4;
    },
    disabled when !root.is-enabled: {
        opacity: 0.4;
        cursor: not-allowed;
    },
    loading when root.is-busy: {
        // loading-specific visual
    }
]
```

> **\[v2 交互设计原则\]**：hover 态使用 `token-surface-alt` 或 `token-accent-subtle`，pressed 态降一层，disabled 态统一 `opacity: 0.4`。所有状态间切换使用 `token-transition-fast`（120ms ease-out）。

#### `card-metric.slint` — 指标卡片

用于转换报告区的 Profile / Compatibility / Quality / Confidence 四张卡片，统一展示 label + value + 可选 progress bar。

```slint
export component MetricCard inherits Rectangle {
    in-out property <string> label;
    in-out property <string> value;
    in-out property <float> progress: 0.0;
    in-out property <color> accent-color: DesignTokens.token-accent;
    in-out property <bool> is-enabled: true;
    in-out property <bool> is-busy: false;
    in-out property <bool> is-hovered: false;

    background: DesignTokens.token-surface-1;
    border-color: DesignTokens.token-border;
    border-radius: DesignTokens.token-radius-lg;
    padding: DesignTokens.token-space-3;

    states [
        idle when self.is-enabled && !self.is-hovered && !self.is-busy: {
            background: DesignTokens.token-surface-1;
        },
        hovered when self.is-hovered && self.is-enabled: {
            background: DesignTokens.token-surface-2;
            border-color: DesignTokens.token-border-strong;
        },
        disabled when !self.is-enabled: {
            opacity: 0.4;
        },
        busy when self.is-busy: {
            // progress bar animates
        }
    ]

    VerticalBox {
        spacing: DesignTokens.token-space-1;

        Text {
            text: label;
            font-size: DesignTokens.token-font-size-sm;
            color: DesignTokens.token-text-muted;
            font-weight: DesignTokens.token-font-weight-medium;
        }
        Text {
            text: value;
            font-size: DesignTokens.token-font-size-xl;
            font-weight: DesignTokens.token-font-weight-bold;
            color: DesignTokens.token-text-primary;
        }
        Rectangle {
            height: DesignTokens.token-progress-height;
            border-radius: DesignTokens.token-progress-radius;
            background: DesignTokens.token-surface-4;
            visible: root.progress > 0.0;

            Rectangle {
                height: 100%;
                width: root.progress * 1px;
                border-radius: DesignTokens.token-progress-radius;
                background: root.accent-color;
            }
        }
    }
}
```

#### `button-primary.slint` — 主按钮 \[v2 新增，含 icon 支持\]

```slint
export component ButtonPrimary inherits Rectangle {
    in-out property <string> text;
    in-out property <string> icon;    // icon name or emoji fallback
    in-out property <bool> is-busy: false;
    in-out property <bool> is-enabled: true;
    in-out property <bool> is-hovered: false;
    in-out property <bool> is-pressed: false;

    callback clicked();

    min-height: 36px;
    horizontal-stretch: 0;
    border-radius: DesignTokens.token-radius-md;
    background: DesignTokens.token-accent;
    cursor: pointer;

    states [
        idle when root.is-enabled && !root.is-hovered && !root.is-pressed && !root.is-busy: {
            background: DesignTokens.token-accent;
        },
        hovered when root.is-hovered && root.is-enabled: {
            background: DesignTokens.token-accent-hover;
        },
        pressed when root.is-pressed && root.is-enabled: {
            background: DesignTokens.token-accent-pressed;
        },
        busy when root.is-busy && root.is-enabled: {
            background: DesignTokens.token-accent-hover;
            // spinner shows, text hidden
        },
        disabled when !root.is-enabled: {
            background: DesignTokens.token-disabled-bg;
            cursor: not-allowed;
        }
    ]

    HorizontalLayout {
        spacing: DesignTokens.token-space-2;
        padding: DesignTokens.token-space-2;
        alignment: center;

        // Icon (optional)
        Rectangle {
            visible: root.icon != "";
            width: root.icon != "" ? DesignTokens.token-icon-size-base : 0px;

            // Icon rendering via Text or future Image component
        }

        Text {
            text: root.is-busy ? I18n.common-loading : root.text;
            font-size: DesignTokens.token-font-size-md;
            font-weight: DesignTokens.token-font-weight-medium;
            color: root.is-enabled ? DesignTokens.token-text-inverse
                                   : DesignTokens.token-disabled-text;
        }
    }
}
```

#### `button-secondary.slint` — 次按钮 \[v2 新增\]

与主按钮结构一致，但配色使用边框+透明背景：

```slint
export component ButtonSecondary inherits Rectangle {
    in-out property <string> text;
    in-out property <bool> is-enabled: true;
    in-out property <bool> is-hovered: false;
    in-out property <bool> is-pressed: false;

    callback clicked();

    min-height: 36px;
    border-radius: DesignTokens.token-radius-md;
    border-width: 1px;
    border-color: DesignTokens.token-border;

    states [
        idle when root.is-enabled && !root.is-hovered: {
            background: transparent;
            border-color: DesignTokens.token-border;
            color: DesignTokens.token-text-primary;
        },
        hovered when root.is-hovered && root.is-enabled: {
            background: DesignTokens.token-surface-3;
            border-color: DesignTokens.token-border-strong;
        },
        pressed when root.is-pressed && root.is-enabled: {
            background: DesignTokens.token-surface-4;
        },
        disabled when !root.is-enabled: {
            background: DesignTokens.token-disabled-bg;
            border-color: DesignTokens.token-disabled-border;
        }
    ]
    // ... text and layout same as ButtonPrimary
}
```

#### `card-plan.slint` — 套餐卡片

用于 Billing 页的按次/按日期套餐展示，支持当前选中态。

```slint
export component PlanCard inherits Rectangle {
    in-out property <string> name;
    in-out property <string> price-label;
    in-out property <string> features;
    in-out property <bool> is-selected: false;
    in-out property <bool> is-current: false;
    in-out property <bool> is-enabled: true;
    in-out property <bool> is-hovered: false;

    callback choose();

    border-width: 2px;
    border-radius: DesignTokens.token-radius-xl;
    padding: DesignTokens.token-space-4;

    states [
        default when !root.is-selected && !root.is-hovered: {
            background: DesignTokens.token-surface-1;
            border-color: DesignTokens.token-border;
        },
        hovered when root.is-hovered && root.is-enabled && !root.is-selected: {
            background: DesignTokens.token-surface-2;
            border-color: DesignTokens.token-accent;
        },
        selected when root.is-selected: {
            background: DesignTokens.token-accent-subtle;
            border-color: DesignTokens.token-accent;
        },
        current when root.is-current: {
            background: DesignTokens.token-surface-2;
            border-color: DesignTokens.token-success;
        },
        disabled when !root.is-enabled: {
            opacity: 0.4;
        }
    ]
    // ...
}
```

#### `card-job.slint` — 历史任务行

用于 History 页 ListView 中的单条任务记录，支持状态色。

```slint
export component JobRowCard inherits Rectangle {
    in-out property <string> id;
    in-out property <string> kind;     // "local" | "cloud"
    in-out property <string> status;   // "Pending" | "Running" | "Succeeded" | "Failed"
    in-out property <string> input;
    in-out property <string> output;
    in-out property <string> created-at;
    in-out property <string> error;
    in-out property <bool> is-selected: false;
    in-out property <bool> is-hovered: false;

    callback select();
    callback remove();
    callback open-output();
    callback open-report();

    background: root.is-selected ? DesignTokens.token-accent-subtle
             : root.is-hovered  ? DesignTokens.token-surface-3
             : root.id.hash() % 2 == 0 ? DesignTokens.token-surface-1
             : DesignTokens.token-surface-4;
    border-radius: DesignTokens.token-radius-md;

    // status badge color logic
    // color = status == "Succeeded" ? token-success
    //       : status == "Failed"    ? token-danger
    //       : status == "Running"   ? token-accent
    //       : token-text-muted
}
```

#### `empty-state.slint` — 空态组件

统一各页面的空数据展示，传入 icon text 和可选 action。

```slint
export component EmptyState inherits Rectangle {
    in-out property <string> message;
    in-out property <string> action-label;
    in-out property <bool> is-loading: false;
    callback action();

    background: transparent;
    padding: DesignTokens.token-space-8;

    VerticalBox {
        spacing: DesignTokens.token-space-4;
        alignment: center;

        // 新增 loading 态
        Rectangle {
            visible: root.is-loading;
            height: 48px; width: 48px;
            // spinner animation
        }

        Text {
            text: root.message;
            color: DesignTokens.token-text-muted;
            font-size: DesignTokens.token-font-size-base;
            horizontal-alignment: center;
            wrap: word-wrap;
            max-width: 400px;
        }
        ButtonSecondary {
            text: root.action-label;
            visible: root.action-label != "" && !root.is-loading;
            clicked => { root.action(); }
        }
    }
}
```

#### `alert-banner.slint` — 提示横幅

用于未登录警告、API 未配置提示、错误提示、配额耗尽等场景。

```slint
export component AlertBanner inherits Rectangle {
    in-out property <string> message;
    in-out property <string> level: "info"; // "info" | "warning" | "danger" | "success"
    in-out property <bool> visible: true;
    in-out property <string> action-label;
    callback action();
    callback dismissed();

    background: level == "danger"  ? DesignTokens.token-danger-subtle
            : level == "warning" ? DesignTokens.token-warning-subtle
            : level == "success" ? DesignTokens.token-success-subtle
            : DesignTokens.token-info-subtle;
    border-radius: DesignTokens.token-radius-md;
    padding: DesignTokens.token-space-3;
    border-left-width: 3px;
    border-left-color: level == "danger"  ? DesignTokens.token-danger
                   : level == "warning" ? DesignTokens.token-warning
                   : level == "success" ? DesignTokens.token-success
                   : DesignTokens.token-info;

    // 新增 action 支持
    HorizontalBox {
        spacing: DesignTokens.token-space-3;
        alignment: center;

        Text {
            text: root.message;
            color: level == "danger"  ? DesignTokens.token-danger
                : level == "warning" ? DesignTokens.token-warning
                : DesignTokens.token-text-primary;
            font-size: DesignTokens.token-font-size-sm;
            vertical-alignment: center;
        }

        ButtonSecondary {
            text: root.action-label;
            visible: root.action-label != "";
            clicked => { root.action(); }
        }
    }
}
```

#### `skeleton-block.slint` — 骨架屏块 \[v2 新增\]

复杂内容加载前显示占位骨架，提供加载感知：

```slint
export component SkeletonBlock inherits Rectangle {
    in-out property <length> width: 100px;
    in-out property <length> height: 16px;
    in-out property <length> radius: DesignTokens.token-radius-sm;

    background: DesignTokens.token-surface-3;
    border-radius: root.radius;

    // 新增 shimmer 动画（通过 state 切换背景色实现闪烁）
    property <float> shimmer-phase: 0.0;

    states [
        loading: {
            shimmer-phase: 0.5;
            background: DesignTokens.token-surface-4;
        }
    ]
}
```

#### `toast-notify.slint` — 轻量通知 \[v2 新增\]

右上角浮窗通知，支持 success / error / info / warning 四种类型，自动消失：

```slint
export component ToastNotify inherits Rectangle {
    in-out property <string> message;
    in-out property <string> level: "info";
    in-out property <bool> visible: false;

    callback dismissed();

    x: parent.width - self.width - DesignTokens.token-space-4;
    y: DesignTokens.token-space-4;
    width: 320px;
    background: DesignTokens.token-surface-1;
    border-radius: DesignTokens.token-radius-lg;
    border-left-width: 4px;
    border-left-color: level == "success" ? DesignTokens.token-success
                   : level == "danger"  ? DesignTokens.token-danger
                   : level == "warning" ? DesignTokens.token-warning
                   : DesignTokens.token-accent;
    padding: DesignTokens.token-space-3;
    drop-shadow-blur-radius: 8px;
    drop-shadow-color: #0000001A;

    states [
        hidden: { visible: false; opacity: 0; }
        shown:  { visible: true; opacity: 1; }
    ]

    // 自动消失通过 Rust 侧 timer 回调触发 dismissed()
}
```

### 4.2 组件状态映射 \[v2 完整版]

| 组件 | Normal | Hover | Active/Pressed | Disabled | Loading | Permission-Denied |
|------|--------|-------|----------------|----------|---------|-------------------|
| `MetricCard` | 白底灰边 | 浅灰背景+粗边框 | - | opacity 0.4 | 显示进度条 | 遮罩+"无权限" |
| `PlanCard` | 白底灰边 | accent 边框 | 选中态 accent-subtle 背景 | opacity 0.4 | 骨架屏 | - |
| `JobRowCard` | 斑马纹 | 高亮背景 | accent-subtle 背景 | - | 行内 Spinner | 遮罩+"无权限" |
| `AlertBanner` | 语义色背景+左色条 | - | - | - | - | - |
| `EmptyState` | 灰色提示居中 | - | - | - | Spinner | "无权限访问" |
| `ButtonPrimary` | accent 背景 | accent-hover | accent-pressed | disabled-bg | Spinner 替换文字 | disabled 态 |
| `ButtonSecondary` | 透明底+边框 | surface-3 背景 | surface-4 背景 | disabled-bg | Spinner 替换文字 | disabled 态 |
| `ToastNotify` | 浮窗右上角 | - | - | - | - | - |
| `SkeletonBlock` | surface-3 底 | - | - | - | surface-4 闪烁 | - |

---

## 五、页面重构方案 \[v2 增强\]

### 5.1 `main.slint` — 主窗口精简

**目标**：从 825 行精简至约 150 行，作为纯粹的布局骨架，不再内联业务逻辑。

**削减策略**：

1. 删除所有 `in-out property` 中的文本/颜色定义 → 迁移到 `tokens.slint` 和 `i18n-strings.slint`。
2. 删除所有 5 个 Tab 的内联 UI 实现 → 全部替换为 `page-*.slint` 组件引用。
3. 删除重复的按钮组、指标卡片、引擎选择器等 → 迁移到 `components/`。
4. 保留顶层布局结构：Header、TabWidget、Footer Status Bar。

**重构后 `main.slint` 核心结构**：

```slint
import { DesignTokens } from "tokens.slint";
import { DarkDesignTokens } from "tokens.slint";
import { I18n } from "i18n-strings.slint";
import { ConvertPage } from "pages/convert.slint";
import { AccountPage } from "pages/account.slint";
import { BillingPage } from "pages/billing.slint";
import { HistoryPage } from "pages/history.slint";
import { SettingsPage } from "pages/settings.slint";
import { TabWidget, Button, VerticalBox, HorizontalBox } from "std-widgets.slint";

export component MainWindow inherits Window {
    // === 全局 Token ===
    DesignTokens { }

    // === i18n 字符串（由 Rust 侧同步） ===
    I18n { }

    // === 状态属性（精简后仅保留核心状态） ===
    in-out property <bool> is-signed-in: false;
    in-out property <bool> is-converting: false;
    in-out property <string> status-text: "";
    in-out property <string> account-display-name: "Guest";
    in-out property <string> app-status: "idle"; // idle | busy | error | offline

    // === 布局 ===
    VerticalBox {
        spacing: 0px;

        // Header: Logo + App Name + Account Status Chip
        HeaderBar { ... }

        // TabWidget: 引用各页面组件
        TabWidget {
            Tab { title: I18n.tab-convert;  ConvertPage  { ... } }
            Tab { title: I18n.tab-account;  AccountPage  { ... } }
            Tab { title: I18n.tab-billing;  BillingPage  { ... } }
            Tab { title: I18n.tab-history;  HistoryPage  { ... } }
            Tab { title: I18n.tab-settings; SettingsPage { ... } }
        }

        // Status Bar
        StatusBar { text: root.status-text; is-converting: root.is-converting; }
    }
}
```

### 5.2 `pages/convert.slint` — 转换页重构 \[v2 增强\]

**问题诊断**：
- 引擎切换使用 Switch，与 `main.slint` 中的 Button 组不一致。
- 缺少云端未登录的明确禁用状态。
- 报告卡片区重复代码。
- 缺少操作步骤说明（Flutter Web 版有）。
- **\[v2 新增\]** 缺少骨架屏加载态。
- **\[v2 新增\]** 配额耗尽态无建模。
- **\[v2 新增\]** 无配额剩余量可视化展示。

**重构要点**：

1. **引擎选择器**：恢复为按钮组（Local / Cloud），引用 `components/` 按钮组件。
2. **账号状态指示**：在转换页顶部增加账号信息栏，显示用户名、套餐、剩余配额进度条。
3. **前置条件检查**：用 `AlertBanner` 组件替代纯文本警告：
   - 未登录：`alert-sign-in-required`
   - API 未配置：`alert-api-not-configured`
   - 配额耗尽：`alert-quota-exceeded`
4. **操作步骤说明**：增加云端转换的步骤提示（1. 上传 TeX 项目 → 2. 云端处理 → 3. 下载 DOCX）。
5. **报告卡片**：引用 `card-metric.slint` 组件。
6. **空态/错误态**：`status-text` 为空时显示就绪提示，转换完成后显示完成徽章。
7. **\[v2 新增\] 骨架屏**：检测 profile 时显示 `SkeletonBlock` 骨架占位。
8. **\[v2 新增\] 配额可视化**：配额进度条，低于 20% 时显示警告色。
9. **\[v2 新增\] Toast 通知**：转换完成后右上角弹出 `ToastNotify`。

### 5.3 `pages/account.slint` — 账号页重构 \[v2 增强\]

**问题诊断**：
- 当前为登录/注册表单 + 账号信息，结构简单。
- 缺少充值记录查询入口。
- 缺少转换记录查询入口。
- 无账号刷新和登出的快捷操作。
- **\[v2 新增\]** 登录中/注册中/刷新中缺少 loading 态。
- **\[v2 新增\]** 权限拒绝态（Token 过期）无处理。

**重构要点**：参考 Flutter Web 版账号页设计，增加四个区块：

1. **账号概览卡**（`card-metric`）：显示用户名、套餐、剩余配额/有效期限。
2. **快捷操作行**：刷新用量、充值、查看转换记录、登出——全部使用 `button-secondary`。
3. **充值记录区**（新增）：显示最近的充值记录列表（日期、金额、方式、状态），无记录时使用 `empty-state`。
4. **转换记录区**（新增）：显示最近的云端转换记录（时间、输入文件、状态、下载链接）。
5. **\[v2 新增\]** Loading 态：登录/注册/刷新时对应区域显示 `SkeletonBlock`。
6. **\[v2 新增\]** Token 过期态：检测到 401 时显示 `AlertBanner` level="warning" 并引导重新登录。

### 5.4 `pages/billing.slint` — 套餐页重构 \[v2 增强\]

**问题诊断**：
- 当前仅有简单的 `plan-catalog` 循环列表，缺少按次/按日期套餐分类。
- 缺少充值记录查询（与 Account Page 重复部分）。
- 缺少 mock 充值入口（Flutter Web 版已实现）。
- **\[v2 新增\]** 无 loading 骨架屏。
- **\[v2 新增\]** 选中态无视觉反馈。

**重构要点**：

1. **Tab 分区**：套餐页分为三个子 Tab：
   - `Per Use`（按次充值）：展示预设次数包（3次/10次/30次），点击后发起 mock 充值。
   - `Subscription`（订阅套餐）：展示服务端返回的订阅套餐列表。
   - `Records`（充值记录）：显示最近充值记录，支持翻页。
2. **按次套餐区**：展示预设次数包，引用 `card-plan.slint`，选中时 `is-selected: true`。
3. **充值记录区**：无记录时使用 `empty-state`。
4. **\[v2 新增\]** Loading 态：列表加载时显示 `SkeletonBlock` 骨架屏。
5. **\[v2 新增\]** mock 充值状态提示：充值后显示 `ToastNotify` level="success"：`toast-recharge-success`。

### 5.5 `pages/history.slint` — 历史页重构 \[v2 增强\]

**问题诊断**：
- `job-history` 和 `recent-jobs` 两个数据源并存。
- 无空态组件，`"No recent jobs."` 为硬编码文本。
- ListView 行内按钮过多（Select/Remove），操作密度过高。
- 无详情展开区。
- **\[v2 新增\]** 无骨架屏/加载态。
- **\[v2 新增\]** 无批量操作的多选态。

**重构要点**：

1. 统一使用 `job-history: [JobRow]` 结构化数据，删除 `recent-jobs` 字符串属性。
2. 引入 `card-job.slint` 和 `empty-state.slint` 组件。
3. 行内仅保留核心操作：打开输出、打开报告。
4. 点击行选中后，底部展开详情区（显示 error 信息、报告链接等）。
5. 增加诊断包导出功能。
6. **\[v2 新增\]** 加载态：历史记录加载时显示 `SkeletonBlock` 骨架列表。
7. **\[v2 新增\]** 权限拒绝态：无权限用户查看历史时显示 `AlertBanner` + `empty-state`。

### 5.6 `pages/settings.slint` — 设置页重构 \[v2 增强\]

**问题诊断**：
- 大量 GroupBox 标题和按钮文字为英文字面量，未使用 i18n。
- `settings-dirty` / `settings-saved-at` / `settings-panel-state` 属性逻辑正确但未与 Rust 侧同步。
- **\[v2 新增\]** 无 API 连接测试功能。
- **\[v2 新增\]** 保存后无成功反馈。

**重构要点**：

1. 全面接入 `I18n` 字符串资源。
2. 将外观设置（语言/主题）从 Settings 移到主窗口 Header 快捷区，或在 Settings 中独立为一个区块。
3. 增加"保存/重置"按钮和状态提示。
4. **\[v2 新增\]** API URL 配置区增加连接测试按钮（`settings-api-test`），测试中显示 spinner，成功/失败分别显示 `ToastNotify`。
5. **\[v2 新增\]** 主题切换器：Light/Dark 单选按钮组，切换后立即应用主题。
6. **\[v2 新增\]** 语言选择器：下拉框选择 en/zh-Hans/zh-Hant/ja，切换后通过 Rust 侧更新 `I18n` 全局属性。

---

## 六、数据模型扩展

### 6.1 `types.slint` 扩展

```slint
// 已有
export struct PlanEntry { id, name, price, quota, features }
export struct JobRow { id, kind, input, output, status, opened-at, error, html-report }
export enum ConversionMode { local, cloud }

// 新增
export struct RechargeRecord {
    id: string,
    amount: string,
    currency: string,
    recharge-type: string,   // "per-use" | "per-date"
    package-name: string,
    status: string,          // "pending" | "paid_mock" | "paid" | "failed"
    provider: string,
    created-at: string,
}

export struct ConversionRecord {
    id: string,
    remote-job-id: string,
    kind: string,             // "local" | "cloud"
    input: string,
    output: string,
    status: string,          // "pending" | "running" | "succeeded" | "failed"
    error: string,
    created-at: string,
}

export enum UserTier {
    free,
    pro,
    enterprise,
}

export enum RechargeType {
    per-use,
    per-date,
}

// === v2 新增 ===
export enum AppStatus {
    idle,
    busy,
    error,
    offline,
    permission-denied,
}

export enum ToastLevel {
    success,
    error,
    warning,
    info,
}

export struct ToastMessage {
    id: string,
    level: ToastLevel,
    message: string,
}
```

### 6.2 Rust 侧 AppState 扩展

```rust
// src/app_state.rs
pub struct RechargeOption { id, name, price, quota }
pub struct RechargePackage { option_id, count, price_label }

// === v2 新增 ===
#[derive(Clone, Debug)]
pub struct ToastState {
    pub queue: Vec<ToastMessage>,
    pub visible: bool,
}

pub enum AppStatusState {
    Idle,
    Busy { operation: String },
    Error { message: String },
    Offline,
    PermissionDenied,
}
```

---

## 七、i18n 扩展方案

### 7.1 当前问题

`src/i18n.rs` 中 `zh-Hans`/`zh-Hant`/`ja-JP` 被强制回退到英文，注释明确说明原因（Slint 1.16 ICU4X CJK 分割错误）。

### 7.2 解决路径

**路径 A（不推荐）：等 Slint 版本升级**

跟踪 [Slint issue #XXXX](https://github.com/slint-ui/slint/issues) CJK 分割支持。Slint 1.7+ 已大幅改进国际化支持，建议在项目依赖中升级到最新稳定版，验证中文渲染后直接使用 Rust 侧的翻译文本同步到 Slint。

**路径 B（推荐）：Rust 侧直接写入 Slint i18n 全局属性**

在 `i18n-strings.slint` 的 `I18n` 全局属性上，Rust 通过回调设置翻译后的字符串。Slint 侧不依赖 ICU，直接渲染 Rust 写入的字符串：

```rust
// Rust 侧
fn sync_i18n(ui: &MainWindow, locale: &str) {
    let global = ui.global::<I18n>();
    global.set_tab_convert(translate(locale, "tab.convert").into());
    global.set_tab_account(translate(locale, "tab.account").into());
    // ...
}
```

此方案绕过 Slint 的 ICU 依赖，实现中文/日文等 CJK 语言正常渲染，是短期内的最优解。

### 7.3 翻译覆盖目标

| 语言 | 当前覆盖 | 目标覆盖 |
|------|----------|----------|
| `en` | 100% | 100% |
| `zh-Hans` | 强制回退英文 | 100%（通过路径B） |
| `zh-Hant` | 强制回退英文 | 100%（通过路径B） |
| `fr` | ~70% | 100% |
| `ja-JP` | 强制回退英文 | 100%（通过路径B） |
| `de` | ~70% | 100% |

---

## 八、主题切换机制

### 8.1 现有机制

`theme.rs` 定义了 6 个主题（default / blue / green / purple / orange / dark），Rust 侧根据用户选择计算 `ThemePalette`，但 Slint 端的颜色是 `in-out property`，未被 Rust 同步更新。

### 8.2 目标机制

Rust 侧 `theme::palette(theme)` 返回的 `ThemePalette` 通过 Slint 回调或 `set_property` 同步到 `tokens.slint` 的 `DesignTokens` 全局属性。所有引用了 `DesignTokens.token-*` 的组件自动响应主题变化，无需逐页面修改。

### 8.3 暗色主题特殊处理

暗色主题需要完整覆盖所有颜色 token（见 3.2 节暗色主题 Token 定义）。特别注意：

- 所有 `color-` 开头的 `in-out property` 切换为暗色色值。
- `border-radius` 在暗色下可能需要轻微调整（暗色边框更明显）。
- 加载动画颜色需要反色处理。
- Surface 层级在暗色下仍然有效，但色值反转为深色系。

---

## 九、商业功能闭环

### 9.1 用户旅程覆盖

```
未登录用户
  └─ 只能使用本地转换
  └─ 充值/账号/云端转换按钮禁用，显示"请先登录"提示（AlertBanner）

已登录用户（Free Tier）
  └─ 可查看账号信息
  └─ 可使用云端转换（受配额限制）——配额进度条可视化
  └─ 可充值（按次/按日期套餐）
  └─ 可查询充值记录和转换记录
  └─ 配额耗尽时：AlertBanner + 引导充值

已登录用户（Pro/Enterprise）
  └─ 配额充足时云端转换不受限
  └─ 可访问账单门户
  └─ 可查询完整转换历史
  └─ 套餐等级徽章（badge-tier）

Token 过期用户
  └─ AlertBanner level="warning"：error-unauthorized
  └─ 云端功能禁用
  └─ 引导重新登录

网络离线用户 [v2 新增]
  └─ AppStatus = offline
  └─ AlertBanner level="warning"：error-network
  └─ 云端功能全部禁用
```

### 9.2 充值模块（Desktop 端新增）

参考 Flutter Web 端 `workspace_app.dart` 的充值设计：

- **按次套餐**：3次(¥1)、10次(¥3)、30次(¥8) — mock 支付。
- **按日期套餐**：日卡(¥5)、周卡(¥14)、月卡(¥30)、年卡(¥120) — mock 支付。
- 充值成功后更新 `quota-remaining` / 有效期状态。
- 显示 `paid_mock` 状态，提示为模拟支付。
- 充值完成弹出 `ToastNotify` level="success"：`toast-recharge-success`。

### 9.3 权益校验

转换前增加权益校验：
- **本地转换**：无需校验。
- **云端转换**：校验 `is-signed-in == true` && `quota-remaining > 0`，不满足时禁用按钮并显示 `AlertBanner`。

---

## 十、实施计划

### Phase 1：基础设施（预计 2 天）

| 任务 | 负责 | 文件 |
|------|------|------|
| 创建 `tokens.slint` Design Token 全局定义（含暗色 Token） | Slint | `ui/tokens.slint` |
| 创建 `motion.slint` 动效 Token 定义 | Slint | `ui/motion.slint` |
| 创建 `i18n-strings.slint` 字符串资源（全部 key） | Slint | `ui/i18n-strings.slint` |
| 实现 Rust 侧 Token 同步机制（亮/暗两套） | Rust | `ui_bindings/theme.rs` |
| 实现 Rust 侧 I18n 字符串同步机制 | Rust | `ui_bindings/i18n.rs` |
| 验证主题切换（default ↔ dark ↔ blue 等） | Both | — |
| 验证 i18n 路径B：zh-Hans / zh-Hant / ja 正常渲染 | Both | — |

### Phase 2：基础组件库（预计 2 天）

| 任务 | 文件 | 状态规范 |
|------|------|---------|
| 实现 `button-primary.slint` | `ui/components/button-primary.slint` | idle/hover/pressed/disabled/busy |
| 实现 `button-secondary.slint` | `ui/components/button-secondary.slint` | idle/hover/pressed/disabled/busy |
| 实现 `button-icon.slint` | `ui/components/button-icon.slint` | idle/hover/pressed/disabled |
| 实现 `card-metric.slint` | `ui/components/card-metric.slint` | idle/hover/disabled/busy |
| 实现 `card-plan.slint` | `ui/components/card-plan.slint` | default/hovered/selected/current/disabled |
| 实现 `card-job.slint` | `ui/components/card-job.slint` | normal/hovered/selected/busy |
| 实现 `empty-state.slint` | `ui/components/empty-state.slint` | normal/loading |
| 实现 `skeleton-block.slint` | `ui/components/skeleton-block.slint` | loading shimmer |
| 实现 `skeleton-line.slint` | `ui/components/skeleton-line.slint` | loading shimmer |
| 实现 `alert-banner.slint` | `ui/components/alert-banner.slint` | info/warning/danger/success + action |
| 实现 `toast-notify.slint` | `ui/components/toast-notify.slint` | success/error/warning/info + auto-dismiss |
| 实现 `badge-status.slint` | `ui/components/badge-status.slint` | pending/running/succeeded/failed |
| 实现 `badge-tier.slint` | `ui/components/badge-tier.slint` | free/pro/enterprise |
| 实现 `divider.slint` | `ui/components/divider.slint` | — |

### Phase 3：页面骨架重构（预计 2 天）

| 任务 | 依赖 |
|------|------|
| `main.slint` 精简为主窗口骨架，引用各 page 组件 | Phase 1 |
| 重构 `pages/settings.slint`：全面 i18n 化 + 主题切换 + API 测试 | Phase 1+2 |
| 重构 `pages/convert.slint`：引用组件、账号状态栏、操作步骤、骨架屏、配额可视化 | Phase 2 |
| 重构 `pages/account.slint`：四区块布局、充值/转换记录、loading 态、Token 过期态 | Phase 2 |
| 重构 `pages/billing.slint`：三子Tab、套餐卡片选中态、loading 骨架屏 | Phase 2 |
| 重构 `pages/history.slint`：结构化数据源、空态组件、loading 骨架屏 | Phase 2 |

### Phase 4：高级交互与商业闭环（预计 2 天）

| 任务 | 说明 |
|------|------|
| 扩展 `types.slint` 数据模型 | `RechargeRecord`、`ConversionRecord`、`AppStatus`、`ToastMessage` |
| 扩展 Rust `AppState` | 增加 recharge/conversion 记录内存态存储、`ToastState`、`AppStatusState` |
| 实现 mock 充值 UI + 回调 | Billing 页按次/按日期套餐 + `ToastNotify` 反馈 |
| 实现权益校验逻辑 | 转换前检查配额 + `AlertBanner` 引导充值 |
| 实现 Toast 通知队列 | Rust 侧管理 `ToastMessage` 队列，自动 dismiss |
| 实现 AppStatus 状态机 | 统一管理 idle/busy/error/offline/permission-denied |
| i18n 全语言覆盖验证 | 验证 `zh-Hans`/`zh-Hant`/`ja-JP` 通过 Rust 路径B正常渲染 |
| 主题全切换验证 | 6 个主题 + 暗色主题的 UI 一致性检查 |
| 构建验证 | `cargo build -p desktop-slint`，确保无编译错误 |

---

## 十一、风险与注意事项

| 风险 | 等级 | 缓解措施 |
|------|------|----------|
| Slint 1.16 ICU4X CJK 分割导致中文无法正常渲染 | **高** | 采用 Rust 侧直接写入 Slint i18n 全局属性的方案（路径B）绕过 ICU |
| `main.slint` 精简导致 API 不兼容 | **中** | Phase 3 前锁定属性接口，变更通过新增 optional property 而非删除 |
| `pages/` 组件与 `main.slint` 属性不对齐 | **中** | Phase 1 建立属性契约文档，统一属性命名规范 |
| 主题切换时旧硬编码颜色遗漏 | **中** | Phase 1 建立 grep 规则，搜索所有 `#` 开头的颜色字面量确保消除 |
| Rust 侧 Token 同步性能 | **低** | Token 数量有限（<30），同步开销可忽略 |
| **\[v2 新增\]** Slint `states` 语法在嵌套组件中行为不一致 | **中** | 在 Phase 2 组件开发时逐一验证各状态组合 |
| **\[v2 新增\]** 暗色主题对比度不足（WCAG） | **低** | 暗色 Token 使用高对比度配色（文本/背景对比度 ≥ 4.5:1） |
| **\[v2 新增\]** 骨架屏动画性能 | **低** | 使用简单背景色切换而非复杂动画，避免性能问题 |

---

## 十二、验收标准

| # | 标准 | 验证方式 |
|---|------|---------|
| 1 | `main.slint` 行数从 825 行减少至 ≤200 行 | 代码行数统计 |
| 2 | 所有 5 个 Tab 页面全面引用 `components/` 中的可复用组件 | 代码审查 |
| 3 | 颜色字面量（`#[0-9A-Fa-f]{6}` 格式）在 `.slint` 文件中消失，全部替换为 `DesignTokens.token-*` | `grep -r "#[0-9A-Fa-f]\{6\}" crates/desktop-slint/src/ui/` 应无结果 |
| 4 | 所有用户可见文本通过 `I18n.*` 全局属性引用，无英文字面量 | 代码审查 + `grep` 搜索 |
| 5 | 主题切换（Light ↔ Dark）时所有颜色同步变化，无需重新加载页面 | 手动测试 |
| 6 | 转换页在未登录时显示 `AlertBanner` 提示，而非纯文本 | UI 测试 |
| 7 | History 页在无记录时显示 `EmptyState` 组件，而非手写文本 | UI 测试 |
| 8 | Billing 页提供按次和按日期两套充值入口 + `ToastNotify` 反馈 | UI 测试 |
| 9 | Account 页展示充值记录和转换记录 | UI 测试 |
| 10 | **\[v2 新增\]** 所有交互组件覆盖 idle / hover / pressed / disabled 状态 | 代码审查 |
| 11 | **\[v2 新增\]** 复杂内容区（转换报告、配套餐列表、历史记录）有骨架屏 loading 态 | UI 测试（slow 3G 节流） |
| 12 | **\[v2 新增\]** 配额耗尽时显示 `AlertBanner` level="warning" 并引导充值 | UI 测试 |
| 13 | **\[v2 新增\]** 网络离线时云端功能全部禁用，显示 `AlertBanner` | UI 测试（断网） |
| 14 | **\[v2 新增\]** Token 过期时显示 `AlertBanner` 并引导重新登录 | UI 测试（手动失效 Token） |
| 15 | **\[v2 新增\]** 所有页面空态使用统一 `EmptyState` 组件 | 代码审查 |
| 16 | **\[v2 新增\]** 亮/暗主题均通过 WCAG 对比度要求（文本 ≥ 4.5:1，大文本 ≥ 3:1） | 视觉审查 |
| 17 | `cargo build -p desktop-slint` 编译通过 | CI / 本地构建 |
| 18 | `cargo test -p desktop-slint` 测试通过 | CI / 本地测试 |

---

## 十三、Before / After 设计报告 \[v2 新增\]

### 13.1 问题对比

| 维度 | Before（v1） | After（v2） |
|------|-------------|-------------|
| 设计 Token 数量 | ~11 个颜色属性 | ~80+ Token（含 typography、spacing、radius、motion、层级） |
| 可复用组件数 | 0 | 16 个（含骨架屏、Toast、Badge） |
| 状态覆盖 | Normal + 部分 Loading | 9 种状态全覆盖 |
| i18n key 数量 | ~106 个 | ~150+ 个（覆盖全部 UI 文本） |
| i18n key 命名规范 | 无统一规范 | `category.sub.key` 格式，永久 key 不变 |
| 暗色主题 | 未定义 | 完整 token 定义 + 自动切换 |
| Surface 层级 | 单一 surface | surface-1/2/3/4 四层层级体系 |
| 动效规范 | 无 | 120/200/300ms 三档 + 缓动曲线 |
| Toast 通知 | 无 | 右上角浮窗，自动消失 |
| 骨架屏 | 无 | `SkeletonBlock` / `SkeletonLine` 组件 |
| 配额可视化 | 无 | 进度条 + 警告色 |
| 权限拒绝态 | 无 | AppStatus 枚举 + UI 响应 |
| 响应式内容宽度 | 无 | 最大 960px 约束，桌面端不溢出 |
| API 连接测试 | 无 | Settings 页一键测试 + Toast 反馈 |
| 充值反馈 | 无 | mock 充值成功 `ToastNotify` |

### 13.2 组件清单（v2 完整版）

| 组件 | 文件 | 类型 | 复用位置 |
|------|------|------|---------|
| `ButtonPrimary` | `components/button-primary.slint` | 基础组件 | 全局 |
| `ButtonSecondary` | `components/button-secondary.slint` | 基础组件 | 全局 |
| `ButtonIcon` | `components/button-icon.slint` | 基础组件 | Header、Toolbar |
| `MetricCard` | `components/card-metric.slint` | 业务组件 | Convert Page |
| `PlanCard` | `components/card-plan.slint` | 业务组件 | Billing Page |
| `JobRowCard` | `components/card-job.slint` | 业务组件 | History Page |
| `EmptyState` | `components/empty-state.slint` | 通用组件 | 所有页面 |
| `SkeletonBlock` | `components/skeleton-block.slint` | 通用组件 | Convert、Billing、History |
| `SkeletonLine` | `components/skeleton-line.slint` | 通用组件 | Account（字段占位） |
| `AlertBanner` | `components/alert-banner.slint` | 通用组件 | 所有页面顶部 |
| `ToastNotify` | `components/toast-notify.slint` | 通用组件 | 全局浮窗 |
| `BadgeStatus` | `components/badge-status.slint` | 通用组件 | History、Convert |
| `BadgeTier` | `components/badge-tier.slint` | 通用组件 | Account、Header |
| `Divider` | `components/divider.slint` | 通用组件 | 区块分隔 |
| `TabItem` | `components/tab-item.slint` | 通用组件 | 主窗口 Tab 栏 |
| `SidebarNav` | `components/sidebar-nav.slint` | 通用组件 | 预留（移动端侧边栏） |

### 13.3 剩余缺口

| 缺口 | 原因 | 建议处理 |
|------|------|---------|
| RTL（从右到左）布局支持 | 当前产品无 RTL 语言需求 | 未来扩展时使用 `HorizontalLayout` 的 `layout-type: mirroring` |
| 自定义字体（Noto Sans CJK） | Slint 字体注入需额外配置 | Phase 4 后评估，通过 Rust 侧 `ui.set-fonts()` 注入 |
| 窗口最小尺寸约束 | 当前代码无 min-size | Phase 3 在 `main.slint` Window 定义中加入 `min-height: 600px; min-width: 800px` |
| 键盘导航（Tab / Arrow Keys） | 当前无焦点管理 | Phase 4 后评估，Slint 1.x 焦点 API 支持有限 |
| 触控/平板布局适配 | 当前仅针对桌面 | Phase 4 后评估 SidebarNav 组件 |
