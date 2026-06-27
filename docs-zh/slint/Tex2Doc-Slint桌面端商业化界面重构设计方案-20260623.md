# Tex2Doc Slint 桌面端商业化界面重构设计方案
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



日期：2026-06-23

## 一、项目概述

### 1.1 背景

Tex2Doc 桌面端采用 Slint 框架构建原生跨平台 UI，当前已实现登录注册、充值套餐、转换、历史记录等商业化核心功能。然而现有界面存在大量问题：设计语言不一致、代码重复严重、状态管理分散、组件无复用、国际化覆盖不全、主题切换粒度粗等问题日益突出，与商业化产品标准差距明显。

Flutter Web 端已完成商业化工作台的重构并验证了设计方向（参见 `docs-zh/ui/web-commercial-workbench-progress-report-20260623.md`），其设计经验为本方案提供了重要参考。

### 1.2 目标

对 `crates/desktop-slint/src/ui/` 目录下的 Slint 前端进行系统性重构优化，输出可落地执行的详细设计方案文档，最终实现商业化级别的桌面端用户体验：

- **设计系统化**：建立统一的 Design Token 体系，支撑多主题、多语言。
- **组件模块化**：提取可复用组件，消除重复代码。
- **状态安全化**：集中管理 UI 状态，消除属性蔓延。
- **交互精细化**：完善加载/空态/错误/禁用等状态覆盖。
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

#### 问题 2：Design Token 体系不完整

现有颜色体系在 Slint 侧定义了 11 个颜色属性：

```
color-window-bg / color-surface / color-surface-alt / color-border /
color-text-primary / color-text-secondary / color-text-muted /
color-accent / color-success / color-warning / color-danger
```

但存在以下缺陷：

- **硬编码 hex 值**：所有颜色值以 `#RRGGBB` 字符串内联在 Slint 中，Rust 侧的 `theme.rs` `ThemePalette` 完全未被 Slint 引用，导致主题切换时 Slint 端颜色不变。
- **无 typography token**：字号、字重、行高均以字面量散布在各个 Widget 中（如 `font-size: 20px`、`font-weight: 700`）。
- **无 spacing token**：padding 和 spacing 以 `8px`、`12px`、`16px` 等字面量重复出现，无法统一调整。
- **无 radius token**：`border-radius` 值在多处硬编码（如 Convert Tab 中 `8px`，Billing Tab 中 `4px`，History Tab 中 `3px`），不统一。

#### 问题 3：状态分散（Property Drilling）

`main.slint` 集中了约 50 个 `in-out property`，这些属性在 Rust 侧通过 `ui_bindings` 模块批量写入。但问题在于：

- **未登录/离线状态未建模**：无网络状态、`is-billing-busy`、`is-account-busy` 等状态缺乏统一枚举。
- **属性冗余**：部分属性如 `recent-jobs` 是纯文本字符串（`"No recent jobs."`），而 Rust 侧已有 `AppState.jobs` 的结构化 `JobEntry` 列表，两者并存导致数据不一致风险。
- **`pages/` 子组件的状态同步**：当 `main.slint` 最终引用 `pages/` 组件时，需要将所有 50 个属性透传到子组件，属性接口膨胀。

#### 问题 4：国际化（i18n）覆盖不完整

`src/i18n.rs` 定义了 106 个 i18n key，涵盖主要 UI 文本。但审计发现：

- **`zh-Hans`/`zh-Hant`/`ja-JP` 回退到英文**：由于 Slint 1.16 ICU4X CJK 分割问题，代码中显式注释说明了中文/日文 UI 实际渲染英文。这是最严重的功能性缺陷之一。
- **hardcoded text 仍存在**：多处地方文本未通过 i18n key 引用，如：
  - `pages/account.slint`：GroupBox 标题 `"Account (Phase D.1)"`、`"Sign In"`、`"Display Name"`、`"Tier"` 等均为字面量。
  - `pages/settings.slint`：几乎所有 GroupBox 标题和按钮文字均为英文字面量。
  - `pages/billing.slint`：按钮文字 `"Current"`、`"Choose"`、`"Checkout"`、`"Portal"` 等。
  - `pages/history.slint`：按钮文字 `"Select"`、`"Remove"`、`"Clear All"`、`"Export Diagnostic Bundle"` 等。
  - `pages/convert.slint`：大部分文本为字面量。

#### 问题 5：状态覆盖不全

按商业 UI 标准，核心工作流需覆盖以下状态：

| 状态 | 当前覆盖 |
|------|----------|
| Normal（正常） | 部分覆盖 |
| Loading（加载中） | 仅有 `is-converting`/`is-account-busy`/`is-billing-busy`，无骨架屏/占位 |
| Empty（空态） | History Tab 有 `"No recent jobs."` 文本，其他页面无 |
| Error（错误） | 无专门的错误提示组件/区域 |
| Disabled（禁用） | Button `enabled` 属性覆盖了部分场景，但云端未登录时仅显示警告文本 |
| Permission Denied（无权限） | 无明确建模 |

#### 问题 6：`pages/` 重构不彻底

Phase D 的 `pages/` 子组件重构方向正确，但存在以下问题：

- `main.slint` 完全未引用任何 `pages/` 组件，导致两套实现并存。
- `pages/` 各组件的属性接口与 `main.slint` 不一致（如 `pages/account.slint` 没有 `is-account-busy` 属性）。
- `pages/` 中存在大量未使用 `in-out property` 暴露的硬编码文本（如 `"Engine Profile"`、`"Output Strategy"` 等 GroupBox 标题）。
- `pages/billing.slint` 引入了 `ListView`/`PlanEntry` 结构，但 `main.slint` 的 Billing Tab 未使用。

---

## 三、设计系统

### 3.1 文件结构（目标）

```
crates/desktop-slint/src/ui/
├── tokens.slint              # 新增：Design Token 全局定义
├── components/               # 新增：可复用组件库
│   ├── card-metric.slint    # 指标卡片（Compatibility/Quality/Confidence 等）
│   ├── card-plan.slint       # 套餐卡片（带选中状态）
│   ├── card-job.slint        # 历史任务行
│   ├── row-action.slint      # 操作行（输入+按钮组合）
│   ├── row-select.slint       # 下拉选择行
│   ├── button-primary.slint  # 主按钮
│   ├── button-secondary.slint# 次按钮
│   ├── badge-status.slint    # 状态徽章（Pending/Running/Succeeded/Failed）
│   ├── panel-group.slint      # 可折叠面板（GroupBox 增强版）
│   ├── empty-state.slint     # 空态提示组件
│   ├── loading-indicator.slint# 加载状态组件
│   ├── alert-banner.slint     # 警告/错误提示条
│   └── dialog-confirm.slint   # 确认对话框
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

### 3.2 Design Token 定义

所有视觉常量集中在 `tokens.slint`，通过 Slint 全局属性暴露给 Rust 主题引擎。

#### 颜色 Token

```slint
// crates/desktop-slint/src/ui/tokens.slint
export global DesignTokens {
    // Window / Surface
    in-out property <color> token-window-bg:   #F6F8FB;
    in-out property <color> token-surface:      #FFFFFF;
    in-out property <color> token-surface-alt:  #F1F5F9;
    in-out property <color> token-border:       #D7DEE8;

    // Text
    in-out property <color> token-text-primary:   #172033;
    in-out property <color> token-text-secondary: #42526B;
    in-out property <color> token-text-muted:    #6B778C;

    // Accent / Semantic
    in-out property <color> token-accent:   #2563EB;
    in-out property <color> token-success: #0F8A5F;
    in-out property <color> token-warning: #B7791F;
    in-out property <color> token-danger:  #C2413A;

    // Typography
    in-out property <length> token-font-size-xs:  10px;
    in-out property <length> token-font-size-sm:  11px;
    in-out property <length> token-font-size-base: 13px;
    in-out property <length> token-font-size-md:  14px;
    in-out property <length> token-font-size-lg:  16px;
    in-out property <length> token-font-size-xl:  20px;
    in-out property <length> token-font-size-2xl: 24px;
    in-out property <length> token-font-weight-normal: 400;
    in-out property <length> token-font-weight-medium: 500;
    in-out property <length> token-font-weight-semibold: 600;
    in-out property <length> token-font-weight-bold: 700;

    // Spacing (4px base unit)
    in-out property <length> token-space-1:  4px;
    in-out property <length> token-space-2:  8px;
    in-out property <length> token-space-3:  12px;
    in-out property <length> token-space-4:  16px;
    in-out property <length> token-space-5:  20px;
    in-out property <length> token-space-6:  24px;
    in-out property <length> token-space-8:  32px;

    // Radius
    in-out property <length> token-radius-sm: 3px;
    in-out property <length> token-radius-md: 6px;
    in-out property <length> token-radius-lg: 8px;
    in-out property <length> token-radius-xl: 12px;

    // Elevation (border-based, no real shadow in Slint)
    in-out property <color> token-shadow-color:   #00000014;
    in-out property <length> token-header-height: 64px;
    in-out property <length> token-sidebar-width: 200px;
}
```

#### Rust 侧主题同步

`theme.rs` 的 `ThemePalette` 通过 Slint 回调或 `slint::invoke_from_ui` 更新 `tokens.slint` 中的 `DesignTokens` 全局属性：

```rust
// src/ui_bindings/theme.rs
pub fn apply_theme(ui: &MainWindow, theme: &str) {
    let p = theme::palette(theme);
    ui.global::<DesignTokens>().set_token_window_bg(p.window_bg.into());
    ui.global::<DesignTokens>().set_token_surface(p.surface.into());
    // ... 同步其余 token
}
```

### 3.3 i18n 字符串资源

在 `i18n-strings.slint` 中集中管理所有用户可见文本，key 格式为 `category.subcategory.key`：

```slint
// crates/desktop-slint/src/ui/i18n-strings.slint
export global I18n {
    in-out property <string> locale: "en";

    // Tab titles
    in-out property <string> tab-convert: "Convert";
    in-out property <string> tab-settings: "Settings";
    in-out property <string> tab-account: "Account";
    in-out property <string> tab-billing: "Plans";
    in-out property <string> tab-history: "History";

    // Convert page
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

    // Account page
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

    // Billing page
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

    // History page
    in-out property <string> history-title: "Conversion History";
    in-out property <string> history-no-jobs: "No conversion records yet.";
    in-out property <string> history-open-output: "Open Output";
    in-out property <string> history-open-report: "Open Report";
    in-out property <string> history-export: "Export Diagnostics";
    in-out property <string> history-clear-all: "Clear All";
    in-out property <string> history-exporting: "Exporting diagnostics...";

    // Settings page
    in-out property <string> settings-service: "Service";
    in-out property <string> settings-api-url: "API Base URL";
    in-out property <string> settings-default-params: "Default Conversion Parameters";
    in-out property <string> settings-default-profile: "Default Profile";
    in-out property <string> settings-default-quality: "Default Quality";
    in-out property <string> settings-default-output: "Default Output Directory";
    in-out property <string> settings-updates: "Updates";
    in-out property <string> settings-check-update: "Check Update";
    in-out property <string> settings-appearance: "Appearance";
    in-out property <string> settings-language: "Language";
    in-out property <string> settings-theme: "Theme";
    in-out property <string> settings-apply: "Apply";
    in-out property <string> settings-about: "About";
    in-out property <string> settings-product: "Tex2Doc Desktop";
    in-out property <string> settings-save: "Save Settings";
    in-out property <string> settings-saved: "Settings saved.";
    in-out property <string> settings-dirty: "Unsaved changes";

    // Common
    in-out property <string> common-cancel: "Cancel";
    in-out property <string> common-confirm: "Confirm";
    in-out property <string> common-copy: "Copy";
    in-out property <string> common-close: "Close";
    in-out property <string> common-loading: "Loading...";
    in-out property <string> common-error: "Error";
    in-out property <string> common-retry: "Retry";
    in-out property <string> common-detected: "detected";

    // Status
    in-out property <string> status-pending: "Pending";
    in-out property <string> status-running: "Running";
    in-out property <string> status-succeeded: "Succeeded";
    in-out property <string> status-failed: "Failed";
}
```

Rust 侧 `i18n.rs` 的翻译函数通过回调更新 Slint 的 `I18n` 全局属性，实现主题/语言切换时 UI 文本实时更新，无需重新渲染整个窗口。

---

## 四、组件设计

### 4.1 基础组件清单

#### `card-metric.slint` — 指标卡片

用于转换报告区的 Profile / Compatibility / Quality / Confidence 四张卡片，统一展示 label + value + 可选 progress bar。

```slint
export component MetricCard inherits Rectangle {
    in-out property <string> label;
    in-out property <string> value;
    in-out property <float> progress: 0.0;
    in-out property <color> accent-color: DesignTokens.token-accent;

    background: DesignTokens.token-surface;
    border-color: DesignTokens.token-border;
    border-radius: DesignTokens.token-radius-lg;
    padding: DesignTokens.token-space-3;

    VerticalBox {
        spacing: DesignTokens.token-space-1;

        Text {
            text: label;
            font-size: DesignTokens.token-font-size-sm;
            color: DesignTokens.token-text-muted;
        }
        Text {
            text: value;
            font-size: DesignTokens.token-font-size-xl;
            font-weight: DesignTokens.token-font-weight-bold;
            color: DesignTokens.token-text-primary;
        }
        ProgressIndicator {
            progress: root.progress;
            visible: root.progress > 0.0;
            height: 4px;
        }
    }
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

    callback choose();

    background: is-selected ? DesignTokens.token-surface-alt : DesignTokens.token-surface;
    border-color: is-selected ? DesignTokens.token-accent : DesignTokens.token-border;
    border-radius: DesignTokens.token-radius-lg;

    // ...
}
```

#### `card-job.slint` — 历史任务行

用于 History 页 ListView 中的单条任务记录，支持状态色。

```slint
export component JobRowCard inherits Rectangle {
    in-out property <string> id;
    in-out property <string> kind;  // "local" | "cloud"
    in-out property <string> status; // "Pending" | "Running" | "Succeeded" | "Failed"
    in-out property <string> input;
    in-out property <string> output;
    in-out property <string> created-at;
    in-out property <string> error;

    callback select();
    callback remove();
    callback open-output();
    callback open-report();

    // status badge color logic
    // color = status == "Succeeded" ? token-success
    //       : status == "Failed"    ? token-danger
    //       : status == "Running"  ? token-accent
    //       : token-text-muted
}
```

#### `empty-state.slint` — 空态组件

统一各页面的空数据展示，传入 icon text 和可选 action。

```slint
export component EmptyState inherits Rectangle {
    in-out property <string> message;
    in-out property <string> action-label;
    callback action();

    background: transparent;
    VerticalBox {
        spacing: DesignTokens.token-space-3;
        alignment: center;
        Text {
            text: message;
            color: DesignTokens.token-text-muted;
            font-size: DesignTokens.token-font-size-base;
        }
        Button {
            text: action-label;
            visible: action-label != "";
            clicked => { root.action(); }
        }
    }
}
```

#### `alert-banner.slint` — 提示横幅

用于未登录警告、API 未配置提示、错误提示等场景。

```slint
export component AlertBanner inherits Rectangle {
    in-out property <string> message;
    in-out property <string> level: "info"; // "info" | "warning" | "danger" | "success"
    in-out property <bool> visible: true;
    callback dismissed();

    background: level == "danger"  ? DesignTokens.token-danger.with-alpha(0.1)
            : level == "warning" ? DesignTokens.token-warning.with-alpha(0.1)
            : level == "success" ? DesignTokens.token-success.with-alpha(0.1)
            : DesignTokens.token-accent.with-alpha(0.1);
    border-radius: DesignTokens.token-radius-md;

    Text {
        text: message;
        color: level == "danger"  ? DesignTokens.token-danger
            : level == "warning" ? DesignTokens.token-warning
            : DesignTokens.token-accent;
        font-size: DesignTokens.token-font-size-sm;
    }
}
```

### 4.2 组件状态映射

| 组件 | Normal | Hover | Active/Pressed | Disabled | Loading |
|------|--------|-------|----------------|----------|---------|
| `MetricCard` | 白底灰边 | 浅灰背景 | - | 降低透明度 | 显示进度条 |
| `PlanCard` | 白底灰边 | 浅灰背景 | 选中时 accent 边 | 灰色文字 | - |
| `JobRowCard` | 斑马纹 | 高亮背景 | - | - | 显示 Spinner |
| `AlertBanner` | 语义色背景 | - | - | - | - |
| `EmptyState` | 灰色提示 | - | - | - | 骨架屏 |

---

## 五、页面重构方案

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
import { I18n } from "i18n-strings.slint";
import { ConvertPage } from "pages/convert.slint";
import { AccountPage } from "pages/account.slint";
import { BillingPage } from "pages/billing.slint";
import { HistoryPage } from "pages/history.slint";
import { SettingsPage } from "pages/settings.slint";
import { TabWidget, Button, VerticalBox, HorizontalBox } from "std-widgets.slint";

export component MainWindow inherits Window {
    // === 全局 Token（由 Rust 侧同步） ===
    DesignTokens { }

    // === i18n 字符串（由 Rust 侧同步） ===
    I18n { }

    // === 状态属性（精简后仅保留核心状态） ===
    in-out property <bool> is-signed-in: false;
    in-out property <bool> is-converting: false;
    in-out property <string> status-text: "";
    in-out property <string> account-display-name: "Guest";

    // ... 其余属性通过子页面组件透传

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

### 5.2 `pages/convert.slint` — 转换页重构

**问题诊断**：
- 引擎切换使用 Switch，与 `main.slint` 中的 Button 组不一致。
- 缺少云端未登录的明确禁用状态。
- 报告卡片区重复代码。
- 缺少操作步骤说明（Flutter Web 版有）。

**重构要点**：

1. **引擎选择器**：恢复为按钮组（Local / Cloud），与 `main.slint` 保持一致，或统一抽象为 `EngineSelector` 组件。
2. **账号状态指示**：在转换页顶部增加账号信息栏，显示用户名、套餐、剩余配额。
3. **前置条件检查**：用 `AlertBanner` 组件替代纯文本警告，显示更清晰。
4. **操作步骤说明**：增加云端转换的步骤提示（1. 上传 TeX 项目 → 2. 云端处理 → 3. 下载 DOCX）。
5. **报告卡片**：引用 `card-metric.slint` 组件。
6. **空态/错误态**：`status-text` 为空时显示就绪提示，转换完成后显示完成徽章。

**重构后 Convert Page 属性接口**：

```slint
export component ConvertPage inherits Rectangle {
    in-out property <string> project-path;
    in-out property <string> main-tex;
    in-out property <string> output-path;
    in-out property <string> detected-profile: "auto";
    in-out property <string> quality-level: "standard";
    in-out property <string> status-text: "";
    in-out property <bool> use-cloud-engine: false;
    in-out property <bool> is-converting: false;
    in-out property <float> conversion-progress: 0.0;
    in-out property <string> compatibility-score: "--";
    in-out property <float> compatibility-progress: 0.0;
    in-out property <string> quality-status: "--";
    in-out property <float> quality-progress: 0.0;
    in-out property <string> profile-confidence: "--";
    in-out property <float> profile-confidence-progress: 0.0;
    in-out property <bool> is-signed-in: false;
    in-out property <string> account-display-name: "Guest";
    in-out property <string> account-tier: "free";
    in-out property <int> quota-remaining: 0;
    in-out property <int> quota-total: 0;
    in-out property <string> api-base-url;

    // Callbacks
    callback choose-project-folder-clicked(string, string);
    callback choose-project-zip-clicked(string, string);
    callback choose-output-clicked(string);
    callback detect-profile-clicked(string);
    callback convert-clicked(string, string, string, string);
    callback cloud-convert-clicked(string, string, string, string, string, string);
    callback open-output-clicked(string);
    callback open-report-clicked(string);

    // 内部布局...
}
```

### 5.3 `pages/account.slint` — 账号页重构

**问题诊断**：
- 当前为登录/注册表单 + 账号信息，结构简单。
- 缺少充值记录查询入口。
- 缺少转换记录查询入口。
- 无账号刷新和登出的快捷操作。

**重构要点**：参考 Flutter Web 版账号页设计，增加四个区块：

1. **账号概览卡**（`card-metric`）：显示用户名、套餐、剩余配额/有效期限。
2. **快捷操作行**：刷新用量、充值、查看转换记录、登出。
3. **充值记录区**（新增）：显示最近的充值记录列表（日期、金额、方式、状态）。
4. **转换记录区**（新增）：显示最近的云端转换记录（时间、输入文件、状态、下载链接）。

**Account Page 属性扩展**：

```slint
in-out property <[RechargeRecord]> recharge-records;
in-out property <[ConversionRecord]> conversion-records;
// RechargeRecord: { id, amount, currency, type, status, created-at }
// ConversionRecord: { id, input, status, created-at, output-path }
```

### 5.4 `pages/billing.slint` — 套餐页重构

**问题诊断**：
- 当前仅有简单的 `plan-catalog` 循环列表，缺少按次/按日期套餐分类。
- 缺少充值记录查询（与 Account Page 重复部分）。
- 缺少 mock 充值入口（Flutter Web 版已实现）。

**重构要点**：

1. **Tab 分区**：套餐页分为两个子 Tab：
   - `Plans`（订阅套餐）：展示服务端返回的订阅套餐列表。
   - `Recharge`（充值）：按次充值 + 按日期充值。
2. **按次套餐区**：展示预设次数包（3次/10次/30次），点击后发起 mock 充值。
3. **按日期套餐区**：展示日卡/周卡/月卡/年卡。
4. **充值记录区**：显示最近充值记录，支持翻页。
5. **mock 充值状态提示**：充值后显示 `paid_mock` 状态提示。

### 5.5 `pages/history.slint` — 历史页重构

**问题诊断**：
- `job-history` 和 `recent-jobs` 两个数据源并存。
- 无空态组件，`"No recent jobs."` 为硬编码文本。
- ListView 行内按钮过多（Select/Remove），操作密度过高。
- 无详情展开区。

**重构要点**：

1. 统一使用 `job-history: [JobRow]` 结构化数据，删除 `recent-jobs` 字符串属性。
2. 引入 `card-job.slint` 和 `empty-state.slint` 组件。
3. 行内仅保留核心操作：打开输出、打开报告。
4. 点击行选中后，底部展开详情区（显示 error 信息、报告链接等）。
5. 增加诊断包导出功能。

### 5.6 `pages/settings.slint` — 设置页重构

**问题诊断**：
- 大量 GroupBox 标题和按钮文字为英文字面量，未使用 i18n。
- `settings-dirty` / `settings-saved-at` / `settings-panel-state` 属性逻辑正确但未与 Rust 侧同步。

**重构要点**：

1. 全面接入 `I18n` 字符串资源。
2. 将外观设置（语言/主题）从 Settings 移到主窗口 Header 快捷区，或在 Settings 中独立为一个区块。
3. 增加"保存/重置"按钮和状态提示。
4. API URL 配置区增加连接测试按钮。

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
```

### 6.2 Rust 侧 AppState 扩展

参考 Flutter Web 端 `commercial_api.dart` 的模型，在 Rust 侧增加：

```rust
// src/app_state.rs
pub struct RechargeOption { id, name, price, quota }
pub struct RechargePackage { option-id, count, price-label }
```

---

## 七、i18n 扩展方案

### 7.1 当前问题

`src/i18n.rs` 中 `zh-Hans`/`zh-Hant`/`ja-JP` 被强制回退到英文，注释明确说明原因（Slint 1.16 ICU4X CJK 分割错误）。

### 7.2 解决路径

**路径 A（推荐）：等 Slint 版本升级**

跟踪 [Slint issue #XXXX](https://github.com/slint-ui/slint/issues) CJK 分割支持。Slint 1.7+ 已大幅改进国际化支持，建议在项目依赖中升级到最新稳定版，验证中文渲染后直接使用 Rust 侧的翻译文本同步到 Slint。

**路径 B：Rust 侧直接写入 Slint i18n 全局属性**

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

```rust
// src/ui_bindings/theme.rs
pub fn apply_theme(ui: &MainWindow, theme: &str) {
    let p = theme::palette(theme);
    let tokens = ui.global::<DesignTokens>();
    tokens.set_token_window_bg(p.window_bg.into());
    tokens.set_token_surface(p.surface.into());
    tokens.set_token-surface_alt(p.surface_alt.into());
    tokens.set_token-border(p.border.into());
    tokens.set_token-text_primary(p.text_primary.into());
    tokens.set_token-text-secondary(p.text_secondary.into());
    tokens.set_token-text-muted(p.text_muted.into());
    tokens.set_token-accent(p.accent.into());
    tokens.set_token-success(p.success.into());
    tokens.set_token-warning(p.warning.into());
    tokens.set_token-danger(p.danger.into());
}
```

### 8.3 暗色主题特殊处理

暗色主题（`dark`）需要在 Slint 侧额外处理：
- 所有 `color-` 开头的 `in-out property` 切换为暗色色值。
- `border-radius` 在暗色下可能需要轻微调整（暗色边框更明显）。
- 加载动画颜色需要反色处理。

---

## 九、商业功能闭环

### 9.1 用户旅程覆盖

```
未登录用户
  └─ 只能使用本地转换
  └─ 充值/账号/云端转换按钮禁用，显示"请先登录"提示

已登录用户（Free Tier）
  └─ 可查看账号信息
  └─ 可使用云端转换（受配额限制）
  └─ 可充值（按次/按日期套餐）
  └─ 可查询充值记录和转换记录

已登录用户（Pro/Enterprise）
  └─ 配额充足时云端转换不受限
  └─ 可访问账单门户
  └─ 可查询完整转换历史
```

### 9.2 充值模块（Desktop 端新增）

参考 Flutter Web 端 `workspace_app.dart` 的充值设计：

- **按次套餐**：3次(¥1)、10次(¥3)、30次(¥8) — mock 支付。
- **按日期套餐**：日卡(¥5)、周卡(¥14)、月卡(¥30)、年卡(¥120) — mock 支付。
- 充值成功后更新 `quota-remaining` / 有效期状态。
- 显示 `paid_mock` 状态，提示为模拟支付。

### 9.3 权益校验

转换前增加权益校验：
- **本地转换**：无需校验。
- **云端转换**：校验 `is-signed-in == true` && `quota-remaining > 0`，不满足时禁用按钮并显示 `AlertBanner`。

---

## 十、实施计划

### Phase 1：基础设施（预计 2 天）

| 任务 | 负责 | 文件 |
|------|------|------|
| 创建 `tokens.slint` Design Token 全局定义 | Slint | `ui/tokens.slint` |
| 创建 `i18n-strings.slint` 字符串资源 | Slint | `ui/i18n-strings.slint` |
| 实现 Rust 侧 Token 同步机制 | Rust | `ui_bindings/theme.rs` |
| 实现 Rust 侧 I18n 字符串同步机制 | Rust | `ui_bindings/i18n.rs` |
| 验证主题切换（default ↔ dark） | Both | — |

### Phase 2：组件库建设（预计 2 天）

| 任务 | 文件 |
|------|------|
| 实现 `card-metric.slint` | `ui/components/card-metric.slint` |
| 实现 `card-plan.slint` | `ui/components/card-plan.slint` |
| 实现 `card-job.slint` | `ui/components/card-job.slint` |
| 实现 `empty-state.slint` | `ui/components/empty-state.slint` |
| 实现 `alert-banner.slint` | `ui/components/alert-banner.slint` |
| 实现 `panel-group.slint` | `ui/components/panel-group.slint` |

### Phase 3：页面重构（预计 3 天）

| 任务 | 依赖 |
|------|------|
| `main.slint` 精简为主窗口骨架，引用各 page 组件 | Phase 1 |
| 重构 `pages/convert.slint`：引用组件、账号状态栏、操作步骤 | Phase 2 |
| 重构 `pages/account.slint`：四区块布局、充值/转换记录 | Phase 2 |
| 重构 `pages/billing.slint`：套餐Tab + 充值Tab | Phase 2 |
| 重构 `pages/history.slint`：结构化数据源、空态组件 | Phase 2 |
| 重构 `pages/settings.slint`：全面 i18n 化 | Phase 1 |

### Phase 4：商业功能与验证（预计 2 天）

| 任务 | 说明 |
|------|------|
| 扩展 `types.slint` 数据模型 | `RechargeRecord`、`ConversionRecord` |
| 扩展 Rust `AppState` | 增加 recharge/conversion 记录内存态存储 |
| 实现 mock 充值 UI + 回调 | Billing 页按次/按日期套餐 |
| 实现权益校验逻辑 | 转换前检查配额 |
| i18n 全语言覆盖验证 | 验证 `zh-Hans`/`zh-Hant`/`ja-JP` 通过 Rust 路径B正常渲染 |
| 主题全切换验证 | 6 个主题的 UI 一致性检查 |
| 构建验证 | `cargo build -p desktop-slint`，确保无编译错误 |

---

## 十一、风险与注意事项

| 风险 | 等级 | 缓解措施 |
|------|------|----------|
| Slint 1.16 ICU4X CJK 分割导致中文无法正常渲染 | **高** | 采用 Rust 侧直接写入 Slint i18n 全局属性的方案（路径B）绕过 ICU |
| `main.slint` 精简导致 API 不兼容 | **中** | Phase 3 前锁定属性接口，变更通过新增 optional property 而非删除 |
| `pages/` 组件与 `main.slint` 属性不对齐 | **中** | Phase 1 建立属性契约文档，统一属性命名规范 |
| 主题切换时旧硬编码颜色遗漏 | **中** | Phase 1 建立 grep 规则，搜索所有 `#` 开头的颜色字面量确保消除 |
| Rust 侧 Token 同步性能 | **低** | Token 数量有限（<20），同步开销可忽略 |

---

## 十二、验收标准

1. `main.slint` 行数从 825 行减少至 ≤200 行。
2. 所有 5 个 Tab 页面全面引用 `components/` 中的可复用组件。
3. 颜色字面量（`#[0-9A-Fa-f]{6}` 格式）在 `.slint` 文件中消失，全部替换为 `DesignTokens.token-*`。
4. 所有用户可见文本通过 `I18n.*` 全局属性引用。
5. 主题切换（default ↔ dark）时所有颜色同步变化，无需重新加载页面。
6. 转换页在未登录时显示 `AlertBanner` 提示，而非纯文本。
7. History 页在无记录时显示 `EmptyState` 组件，而非手写文本。
8. Billing 页提供按次和按日期两套充值入口。
9. Account 页展示充值记录和转换记录。
10. `cargo build -p desktop-slint` 编译通过。
11. `cargo test -p desktop-slint` 测试通过。
