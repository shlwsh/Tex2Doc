# Tex2Doc Desktop Slint 多 TAB 界面重构技术方案

**日期**：2026-06-22  
**范围**：`crates/desktop-slint` 前端界面与必要的 UI 绑定整理  
**目标**：将当前单页堆叠式 Slint UI 升级为多 TAB 工具型界面，主页面聚焦目标文件选择与转换，账号注册、套餐管理、配置、历史与诊断等管理能力拆分到独立页。

## 一、当前界面现状

当前桌面端入口由 `crates/desktop-slint/src/ui/main.slint` 中的 `MainWindow` 承载，主要特征如下：

1. `MainWindow` 同时持有项目路径、API base URL、登录邮箱、密码、账号状态、套餐状态、更新状态、转换选项、转换报告和最近任务等所有 UI 状态。
2. Rust callback 已覆盖本地转换、云端转换、Profile 检测、登录、注册、刷新登录、退出、用量查询、套餐查询、checkout、billing portal、更新检查、文件选择、打开输出、打开报告、导出诊断。
3. 页面布局是单个 `VerticalBox` 纵向堆叠：
   - Account
   - Billing
   - Updates
   - Project
   - Options
   - Conversion actions
   - Report
   - Status
   - Recent Jobs
4. Rust 侧 `crates/desktop-slint/src/main.rs` 直接绑定所有 `ui.on_*` callback，并负责：
   - 启动时从 `Settings::load()` 恢复默认值。
   - 本地转换调用 `job::start_job`。
   - 云端转换调用 `cloud_convert::convert_project_blocking`。
   - 账号注册登录调用 `cloud_account::*_blocking`。
   - 设置与最近任务持久化。

当前结构已经具备完整 MVP 闭环，但管理功能与转换主路径混在一个页面，会造成首屏噪音过高、转换动作不突出、注册和套餐操作不便维护。

## 二、重构目标

### 2.1 产品目标

1. 主页面只服务核心转换工作流：
   - 选择 TeX 项目目录或 zip。
   - 可选填写 cloud main TeX。
   - 选择 Profile、质量等级和输出 DOCX 路径。
   - 执行本地转换或云端转换。
   - 查看转换进度、结果摘要和主要操作按钮。
2. 配置页独立承载默认配置：
   - API base URL。
   - 默认 Profile。
   - 默认质量等级。
   - 默认输出路径策略。
   - 更新 channel。
3. 用户注册与账号页独立承载：
   - 登录、注册、刷新登录、退出。
   - 当前账号状态。
   - 当前用量。
4. 套餐管理页独立承载：
   - 套餐列表。
   - 目标 Plan ID。
   - Checkout。
   - Billing Portal。
   - 套餐状态与用量摘要。
5. 历史与诊断页独立承载：
   - 最近转换任务。
   - 打开输出。
   - 打开报告。
   - 导出诊断包。
6. 更新检查可放在配置页，也可作为后续 Support tab；首期建议放入配置页，避免 tab 数过多。

### 2.2 工程目标

1. 第一阶段优先只改 Slint 结构，不重命名现有 callback，降低 Rust 侧改动。
2. `MainWindow` 对 Rust 暴露的属性和 callback 名称保持兼容，使 `main.rs` 现有绑定可继续工作。
3. Slint UI 内部拆成多个组件，后续再逐步把 Rust callback 绑定按领域拆分。
4. 不在首期引入复杂新控件；如果当前 Slint std-widgets 可用 `TabWidget`，优先使用；否则实现一个轻量 tab bar 加条件渲染。
5. 主转换页必须在 920px 宽度下可用，建议目标窗口调整到 `1080px x 760px`，并为内容页加滚动区域。

## 三、推荐 TAB 信息架构

建议首期使用 5 个 tab：

| Tab | 名称 | 主要职责 | 默认可见优先级 |
| --- | --- | --- | --- |
| 1 | 转换 | 文件选择、本地/云端转换、进度、报告摘要、状态 | 默认打开 |
| 2 | 配置 | API base URL、默认 profile、质量、输出路径策略、更新 channel | 第二优先 |
| 3 | 账号 | 注册、登录、刷新登录、退出、账号状态、用量 | 账号相关 |
| 4 | 套餐 | 套餐查询、Plan ID、Checkout、Billing Portal、套餐状态 | 商业化相关 |
| 5 | 历史与诊断 | 最近任务、打开输出、打开报告、导出诊断包 | 支持与排错 |

### 3.1 转换 Tab

转换页建议布局：

```text
转换
├─ 输入项目
│  ├─ 项目路径 LineEdit
│  ├─ 选择目录
│  └─ 选择 zip
├─ 输出
│  ├─ DOCX 输出路径
│  └─ Save As
├─ 转换选项
│  ├─ Profile ComboBox
│  ├─ Quality ComboBox
│  └─ Main TeX for cloud conversion
├─ 操作栏
│  ├─ Detect Profile
│  ├─ Local Convert
│  └─ Cloud Convert
├─ ProgressIndicator
├─ 报告摘要
│  ├─ Profile
│  ├─ Compatibility
│  ├─ Quality
│  └─ Confidence
└─ 状态输出 TextEdit
```

主转换页不显示登录密码、套餐详情和更新 channel。云端转换按钮旁只显示简短账号提示，例如 `Signed in`、`Not signed in`、`Quota: 7`，完整账号操作跳到账号 tab。

### 3.2 配置 Tab

配置页承载长期偏好，而不是单次转换输入：

```text
配置
├─ 服务地址
│  └─ API base URL
├─ 默认转换参数
│  ├─ Default Profile
│  ├─ Default Quality
│  └─ Default Output Directory / Output Strategy
├─ 更新
│  ├─ Release channel
│  └─ Check Update
└─ 状态
   └─ update-status
```

首期可以继续复用当前 opportunistic persist 机制，即用户触发转换、登录、检测或更新时自动保存相关字段。后续再新增 `save-settings-clicked` callback，把配置保存从业务动作中解耦。

### 3.3 账号 Tab

账号页独立管理注册和登录：

```text
账号
├─ Email
├─ Password
├─ Login
├─ Register
├─ Refresh Session
├─ Logout
├─ Account Status
└─ Usage Status
```

建议优化：

1. 密码输入使用密码模式，如果 Slint 当前 `LineEdit` 支持输入类型，则启用。
2. 登录、注册、刷新、退出按钮要有明显 busy 状态，避免重复点击。
3. 登录成功后清空 `login-password`，降低密码驻留时间。
4. 当前已有 secure token store，应在账号页明确展示 `Stored session found. Click Refresh.` 这类状态。

### 3.4 套餐 Tab

套餐页独立管理商业化操作：

```text
套餐
├─ Current Plan / Usage Summary
├─ Plan ID
├─ Plans
├─ Checkout
├─ Billing Portal
└─ Billing Status
```

首期仍可使用当前 `billing-status` 文本显示套餐列表。后续建议把 `plans_line` 文本升级为结构化 model，让 UI 展示 Plan cards 或表格，但不建议在首期引入该改动。

### 3.5 历史与诊断 Tab

历史页承载转换后的辅助操作：

```text
历史与诊断
├─ Recent Jobs TextEdit
├─ Open Output
├─ Open Report
└─ Export Diagnostics
```

首期继续复用 `recent-jobs` 字符串。后续可以把 `AppState.recent_jobs()` 转成 Slint model，支持选择某一条任务后打开对应输出或报告，而不是只基于当前 `output-path`。

## 四、Slint 实现方案

### 4.1 首期低风险方案：保留 MainWindow 对外契约

首期不要重命名这些对 Rust 暴露的属性和 callback：

```slint
in-out property <string> project-path;
in-out property <string> api-base-url;
in-out property <string> login-email;
in-out property <string> login-password;
in-out property <string> account-status;
in-out property <string> usage-status;
in-out property <string> billing-plan-id;
in-out property <string> billing-status;
in-out property <string> update-channel;
in-out property <string> update-status;
in-out property <string> detected-profile;
in-out property <string> main-tex;
in-out property <string> quality-level;
in-out property <string> output-path;
in-out property <string> status-text;
in-out property <bool> is-converting;
in-out property <float> conversion-progress;
in-out property <string> compatibility-score;
in-out property <string> quality-status;
in-out property <string> profile-confidence;
in-out property <string> recent-jobs;
```

同样保留当前 `convert-clicked`、`cloud-convert-clicked`、`login-clicked`、`register-clicked` 等 callback。这样 Rust 侧 `ui.on_*` 绑定可以保持不变，实际改动集中在 `.slint` 布局文件。

### 4.2 文件拆分建议

建议把单个 `main.slint` 拆分为以下文件：

```text
crates/desktop-slint/src/ui/
├─ main.slint
├─ components/
│  ├─ tab-bar.slint
│  ├─ field-row.slint
│  └─ status-summary.slint
└─ pages/
   ├─ convert-tab.slint
   ├─ settings-tab.slint
   ├─ account-tab.slint
   ├─ billing-tab.slint
   └─ history-tab.slint
```

如果希望首期更小，也可以先只在 `main.slint` 内定义私有 component，验证通过后再拆文件。

### 4.3 Tab 组件实现选择

优先级如下：

1. 如果当前 Slint 版本和 std-widgets 支持 `TabWidget`，直接使用标准 TabWidget。
2. 如果不可用，使用自定义 tab bar：
   - `active-tab: int`。
   - 顶部 `HorizontalBox` 放 5 个 `Button`。
   - 内容区根据 `active-tab` 显示对应页面。

自定义 tab bar 伪代码：

```slint
in-out property <int> active-tab: 0;

HorizontalBox {
    Button { text: "转换"; clicked => { root.active-tab = 0; } }
    Button { text: "配置"; clicked => { root.active-tab = 1; } }
    Button { text: "账号"; clicked => { root.active-tab = 2; } }
    Button { text: "套餐"; clicked => { root.active-tab = 3; } }
    Button { text: "历史与诊断"; clicked => { root.active-tab = 4; } }
}

ConvertTab { visible: root.active-tab == 0; }
SettingsTab { visible: root.active-tab == 1; }
AccountTab { visible: root.active-tab == 2; }
BillingTab { visible: root.active-tab == 3; }
HistoryTab { visible: root.active-tab == 4; }
```

### 4.4 组件属性传递原则

每个 tab 组件只暴露自己需要的属性和 callback。`MainWindow` 作为适配层，把现有 root property 和 callback 转发进去。

示例：

```slint
component ConvertTab inherits Rectangle {
    in-out property <string> project-path;
    in-out property <string> main-tex;
    in-out property <string> detected-profile;
    in-out property <string> quality-level;
    in-out property <string> output-path;
    in-out property <string> status-text;
    in-out property <bool> is-converting;
    in-out property <float> conversion-progress;

    callback convert-clicked(string, string, string, string);
    callback cloud-convert-clicked(string, string, string, string, string, string);
    callback detect-profile-clicked(string);
}
```

`MainWindow` 中使用双向绑定：

```slint
ConvertTab {
    project-path <=> root.project-path;
    main-tex <=> root.main-tex;
    detected-profile <=> root.detected-profile;
    quality-level <=> root.quality-level;
    output-path <=> root.output-path;
    status-text <=> root.status-text;
    is-converting <=> root.is-converting;
    conversion-progress <=> root.conversion-progress;

    convert-clicked(path, profile, quality, output) => {
        root.convert-clicked(path, profile, quality, output);
    }
}
```

这样第一阶段仍不需要修改 `main.rs` 的 callback 名称。

## 五、Rust 侧渐进重构方案

### 5.1 第一阶段：不改 Rust 绑定

只改 Slint 布局和组件结构，保持：

1. `MainWindow::new()` 不变。
2. `ui.set_*` 初始化不变。
3. `ui.on_*` callback 绑定不变。
4. `persist_settings`、`apply_account_session`、`recent_jobs_for_ui` 等函数不动。

该阶段目标是快速交付多 TAB 界面，风险集中在 Slint 编译和视觉布局。

### 5.2 第二阶段：按领域拆分 callback 绑定

当 UI 结构稳定后，再把 `main.rs` 中的大型绑定逻辑拆成领域模块：

```text
crates/desktop-slint/src/ui_bindings/
├─ mod.rs
├─ conversion.rs
├─ account.rs
├─ billing.rs
├─ settings.rs
├─ update.rs
└─ diagnostics.rs
```

建议函数形态：

```rust
pub fn wire_conversion(ui: &MainWindow, app_state: Arc<AppState>);
pub fn wire_account(ui: &MainWindow, app_state: Arc<AppState>);
pub fn wire_billing(ui: &MainWindow, app_state: Arc<AppState>);
pub fn wire_update(ui: &MainWindow);
pub fn wire_diagnostics(ui: &MainWindow);
```

这样可以把当前 `main.rs` 里的 20 多个 `ui.on_*` 绑定拆散，后续维护成本更低。

### 5.3 第三阶段：状态结构化

当前 UI 状态大多是字符串。后续建议逐步引入更明确的 UI state：

1. `AccountViewState`：登录态、display name、plan id、quota。
2. `BillingViewState`：plan 列表、选中 plan、checkout/billing portal busy 状态。
3. `ConversionViewState`：转换模式、progress、report summary、current job id。
4. `SettingsViewState`：配置 dirty 状态、保存状态。

注意：结构化状态会影响 Slint 属性与 Rust setter，因此不要和首期 tab 改造混在一起。

## 六、交互与视觉优化建议

1. 顶部 tab bar 保持工具型克制风格，不做营销页式 hero。
2. 主转换页采用双列或上下分区：
   - 左侧或上半区：输入、输出、选项。
   - 右侧或下半区：报告摘要、状态日志。
3. 转换按钮应比 Detect Profile、Open Output 更突出，但仍保持工具界面风格。
4. 云端转换按钮在未登录时可以禁用并提示去账号 tab 登录。当前只检查 `api-base-url`，后续建议增加 `is-signed-in` 或 `account-ready` 属性。
5. 状态文本不要占据首屏过高空间，默认高度建议 120 到 160px，并允许滚动。
6. 最近任务不放在主转换页首屏，以免干扰新任务启动。
7. 配置页和账号页中的说明文字保持简短，不加入大段教程。
8. 按钮文字建议统一中文或英文。当前 UI 为英文，如果产品面向中文用户，建议本轮一并切换为中文；如果保留国际化计划，则先建立文案常量或 i18n 方案。

## 七、实施步骤

### 阶段 A：多 TAB UI 最小落地

1. 在 `main.slint` 增加 `active-tab`。
2. 增加 tab bar。
3. 把现有 GroupBox 按职责移动到 5 个 tab 内容区。
4. 主转换 tab 只保留 Project、Options、转换按钮、Progress、Report、Status。
5. 配置 tab 移入 API base URL、update channel、Check Update。
6. 账号 tab 移入 email、password、login/register/refresh/logout、usage。
7. 套餐 tab 移入 plan id、Plans、Checkout、Portal、billing status。
8. 历史与诊断 tab 移入 Recent Jobs、Open Output、Open Report、Diagnostics。
9. 保持所有 property 和 callback 名称不变。

### 阶段 B：组件文件拆分

1. 抽出 `ConvertTab`、`SettingsTab`、`AccountTab`、`BillingTab`、`HistoryTab`。
2. `main.slint` 只保留窗口、全局属性、callback 声明和 tab shell。
3. 每个 tab 组件只接收必要属性，减少 root 内部依赖。

### 阶段 C：Rust 绑定拆分

1. 新增 `ui_bindings` 模块。
2. 迁移转换相关 callback 到 `ui_bindings::conversion`。
3. 迁移账号 callback 到 `ui_bindings::account`。
4. 迁移套餐 callback 到 `ui_bindings::billing`。
5. 迁移更新与诊断 callback 到各自模块。
6. 保持行为不变后再考虑 callback 命名整理。

### 阶段 D：结构化状态与商业化体验增强

1. 为账号增加 `is-signed-in`、`account-display-name`、`quota-remaining`。
2. 为套餐增加结构化 plan model。
3. 为转换增加 `conversion-mode`，替代并列的 Convert 和 Cloud Convert 主要按钮。
4. 为设置增加显式保存按钮和保存状态。
5. 为历史任务增加可选择列表，支持按任务打开输出和报告。

## 八、测试与验收标准

### 8.1 编译验证

每个阶段至少运行：

```powershell
cargo check -p doc-desktop-slint
cargo test -p doc-desktop-slint
```

如果涉及共享转换模块，再按实际影响补充相关 crate 测试。

### 8.2 UI smoke

手动验证以下路径：

1. 启动应用后默认打开“转换”tab。
2. 选择项目目录后，项目路径更新，空输出路径自动填充默认 DOCX 路径。
3. 选择 zip 后，输出路径生成规则仍正确。
4. Detect Profile 能更新 Profile 和状态文本。
5. 本地 Convert 能显示进度、报告摘要、状态文本和最近任务。
6. 云端 Convert 在配置 API base URL 且登录后可执行。
7. 账号 tab 中 Login、Register、Refresh、Logout 状态更新正确。
8. 套餐 tab 中 Plans、Checkout、Portal 状态更新正确。
9. 配置 tab 中 Check Update 状态更新正确。
10. 历史与诊断 tab 中 Open Output、Open Report、Diagnostics 可用。

### 8.3 回归关注点

1. tab 切换不应清空当前输入。
2. 后台转换结束后，即使用户切到其他 tab，也必须正确更新状态。
3. `is-converting` 应禁用会破坏当前转换上下文的按钮。
4. 密码字段不应被保存到 `Settings`。
5. token 继续走 `credential_store`，不进入普通 settings JSON。

## 九、风险与规避

| 风险 | 影响 | 规避 |
| --- | --- | --- |
| Slint 标准 `TabWidget` 在当前版本不可用或行为不满足需求 | UI 编译失败或样式不可控 | 首期准备自定义 tab bar fallback |
| 拆组件时 callback 转发遗漏 | 某些按钮点击无效 | 保留 callback 名称，逐页迁移并运行 UI smoke |
| 账号和套餐页隐藏后用户找不到云端转换前置条件 | 云端转换失败率上升 | 主转换页显示简短账号状态和跳转提示 |
| 最近任务仍是字符串 | 历史页可操作性有限 | 首期接受，后续升级为 model |
| `main.rs` 继续过大 | 维护成本仍高 | 第二阶段再拆 Rust 绑定，避免首期一次性风险过大 |
| 工作区已有未提交业务改动 | 容易误伤用户变更 | 每次代码实施前先看 `git status`，只改 UI 相关文件 |

## 十、建议落地顺序

推荐按以下顺序推进：

1. 先实现 `main.slint` 内部多 tab，不拆文件，保持 Rust 绑定完全不变。
2. 通过 `cargo check -p doc-desktop-slint` 和 UI smoke 后，再抽 `pages/*.slint`。
3. UI 组件稳定后，再拆 `main.rs` callback 绑定。
4. 最后再做结构化套餐 model、历史任务 model 和显式设置保存。

这样可以最快满足“多 TAB 页模式”和“主页面聚焦目标文件选择、转换”的要求，同时不把 UI 信息架构改造、Rust 绑定拆分和状态模型重写混在一次高风险变更里。

