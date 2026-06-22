# Tex2Doc Desktop UI/UX 新版本总体改进方案 v2

**日期**：2026-06-22  
**基于文档**：`docs-zh/ui/desktop-slint-ui-ux-improvement-report-20260622.md`  
**适用范围**：`crates/desktop-slint` 主界面、多 TAB 页面、UI 绑定、设置持久化、版本号管理与发布辅助脚本。  
**新增要求**：

1. 主界面支持多语言，默认英语，可切换简体中文、繁体中文、法文、日文、德文。
2. 主界面支持风格主题切换，并统一切换颜色体系。
3. 添加版本号管理。版本号规则为 `1.年份后两位.月份.本月修改次数`，每次 git 提交时自动加 1。

---

## 一、总体目标

当前桌面端已经完成多 TAB 化，核心转换流程集中在 Convert 页，Settings / Account / Billing / History 等管理能力已拆分。下一版本的目标是从「功能完整」升级到「产品化可维护」：

1. **多语言产品化**：所有主界面文案从硬编码文本迁移到可维护的 i18n 资源，默认英语，允许用户切换语言并持久化。
2. **主题系统统一化**：建立全局颜色 token，所有页面和组件使用统一主题变量，不再散落硬编码颜色。
3. **版本号自动化**：建立桌面端版本号来源、展示位置和 git commit 自动递增机制，使版本号可追踪、可展示、可发布。
4. **UI/UX 改进可分阶段落地**：避免一次性重写 UI，优先建立基础设施，再逐步替换页面文案与视觉样式。

---

## 二、目标用户体验

### 2.1 默认语言

应用首次启动默认使用英语：

```text
Convert | Settings | Account | Billing | History
```

用户可在 Settings 页切换语言：

```text
Language: English / 简体中文 / 繁體中文 / Français / 日本語 / Deutsch
```

切换后：

1. 当前窗口文案立即刷新。
2. 语言偏好写入本地设置。
3. 下次启动自动恢复用户选择。
4. 日志、报告原文不强制翻译，只翻译 UI 标签、按钮、提示、状态文案。

### 2.2 默认主题

建议默认主题为 `System` 或 `Light`。考虑当前 Slint UI 已偏工具型，建议首期提供：

| 主题 | 用途 | 视觉方向 |
|---|---|---|
| System | 跟随系统 | 默认选项，降低用户认知负担 |
| Light | 日间工作 | 白底、低饱和蓝色强调 |
| Dark | 夜间工作 | 深灰背景、浅色文字、蓝绿色强调 |
| High Contrast | 可访问性 | 高对比、清晰边框、明显 focus |

用户可在 Settings 页切换主题：

```text
Theme: System / Light / Dark / High Contrast
```

切换后：

1. 全部 TAB 页同步更新颜色。
2. 按钮、输入框、状态条、进度条、卡片、表格、历史项使用同一套颜色 token。
3. 主题偏好写入本地设置。

### 2.3 版本号展示

建议在以下位置展示版本号：

1. 窗口标题：`Tex2Doc Desktop 1.26.6.12`
2. Settings 页底部：`Version 1.26.6.12`
3. 诊断包 manifest：记录 `desktop_version`。
4. 转换报告 UI 摘要：仅显示生成工具版本，不污染 DOCX 正文。

---

## 三、多语言方案

### 3.1 支持语言

语言枚举建议如下：

| 代码 | 展示名 | 说明 |
|---|---|---|
| `en` | English | 默认语言 |
| `zh-Hans` | 简体中文 | 中国大陆简体 |
| `zh-Hant` | 繁體中文 | 繁体中文 |
| `fr` | Français | 法文 |
| `ja` | 日本語 | 日文 |
| `de` | Deutsch | 德文 |

### 3.2 文案资源组织

建议新增：

```text
crates/desktop-slint/src/i18n/
├─ mod.rs
├─ catalog.rs
├─ locale.rs
└─ translations/
   ├─ en.json
   ├─ zh-Hans.json
   ├─ zh-Hant.json
   ├─ fr.json
   ├─ ja.json
   └─ de.json
```

资源格式建议采用稳定 key：

```json
{
  "tab.convert": "Convert",
  "tab.settings": "Settings",
  "convert.project": "Project",
  "convert.choose_folder": "Choose Folder",
  "convert.convert": "Convert",
  "settings.language": "Language",
  "settings.theme": "Theme"
}
```

原则：

1. key 使用语义命名，不使用英文原文作为 key。
2. 所有语言文件必须拥有相同 key 集合。
3. 英语作为 fallback。
4. 缺失翻译时显示英语，并在 debug 日志中输出缺失 key。

### 3.3 Slint 集成方式

首期建议使用「Rust 翻译表 + Slint 属性注入」方式，避免一次性重写 Slint 组件。

在 `MainWindow` 增加 UI 文案属性：

```slint
in-out property <string> t-tab-convert;
in-out property <string> t-tab-settings;
in-out property <string> t-convert-button;
in-out property <string> t-language-label;
in-out property <string> t-theme-label;
```

Rust 侧根据当前 locale 批量设置：

```rust
ui.set_t_tab_convert(t("tab.convert"));
ui.set_t_tab_settings(t("tab.settings"));
ui.set_t_convert_button(t("convert.convert"));
```

后续当 Slint 层稳定后，再评估是否迁移到 Slint 原生翻译机制。首期优先保证可控、可测试和快速落地。

### 3.4 文案覆盖范围

第一批必须覆盖：

1. Tab 名称。
2. Convert 页所有标签、按钮、状态提示。
3. Settings 页语言、主题、默认输出目录、默认 profile、默认 quality。
4. Account 页登录、注册、刷新、退出、账号状态。
5. Billing 页套餐、用量、checkout、portal。
6. History 页空状态、打开输出、打开报告。
7. 全局错误提示和成功提示。

第二批覆盖：

1. 诊断导出提示。
2. 更新检查提示。
3. 长报告摘要中的固定模板文案。
4. 快捷键 tooltip。

### 3.5 设置持久化

建议在 `Settings` 中新增：

```rust
pub locale: String,       // default: "en"
pub theme: String,        // default: "system"
```

兼容策略：

1. 旧配置文件没有 `locale` 时默认 `en`。
2. 旧配置文件没有 `theme` 时默认 `system`。
3. 非法值回退默认值并记录日志。

---

## 四、主题系统方案

### 4.1 主题 token

建议新增 `ThemePalette`：

```rust
pub struct ThemePalette {
    pub window_bg: String,
    pub surface: String,
    pub surface_alt: String,
    pub border: String,
    pub text_primary: String,
    pub text_secondary: String,
    pub text_muted: String,
    pub accent: String,
    pub accent_hover: String,
    pub success: String,
    pub warning: String,
    pub danger: String,
    pub progress_track: String,
    pub progress_fill: String,
}
```

Slint 根组件暴露主题属性：

```slint
in-out property <color> color-window-bg;
in-out property <color> color-surface;
in-out property <color> color-border;
in-out property <color> color-text-primary;
in-out property <color> color-text-secondary;
in-out property <color> color-accent;
in-out property <color> color-success;
in-out property <color> color-warning;
in-out property <color> color-danger;
```

页面组件只消费 token，不直接写死颜色。

### 4.2 主题定义

建议首期固定四套主题：

#### Light

```text
window_bg      #F6F8FB
surface        #FFFFFF
surface_alt    #F1F5F9
border         #D7DEE8
text_primary   #172033
text_secondary #42526B
text_muted     #6B778C
accent         #2563EB
success        #0F8A5F
warning        #B7791F
danger         #C2413A
```

#### Dark

```text
window_bg      #111827
surface        #182235
surface_alt    #202B3F
border         #334155
text_primary   #F8FAFC
text_secondary #CBD5E1
text_muted     #94A3B8
accent         #60A5FA
success        #34D399
warning        #FBBF24
danger         #F87171
```

#### High Contrast

```text
window_bg      #000000
surface        #0B0B0B
surface_alt    #161616
border         #FFFFFF
text_primary   #FFFFFF
text_secondary #E6E6E6
text_muted     #CCCCCC
accent         #00A3FF
success        #00FF88
warning        #FFE600
danger         #FF4D4D
```

#### System

`System` 不单独定义颜色。运行时根据系统深浅色能力选择 `Light` 或 `Dark`；如果当前平台检测困难，首期 `System` 可等同 `Light`，并保留后续扩展点。

### 4.3 页面适配顺序

建议按以下顺序替换硬编码颜色：

1. 根窗口背景、内容区域背景。
2. Tab bar：选中态、hover、未选中态、边框。
3. Convert 页：输入框、按钮、进度条、状态文本。
4. Settings 页：语言/主题选择器、保存按钮。
5. Account/Billing 页：状态 badge、用量显示、套餐卡片。
6. History 页：列表项、空状态、操作按钮。

---

## 五、版本号管理方案

### 5.1 版本号规则

版本号格式：

```text
1.YY.M.N
```

含义：

| 段 | 含义 | 示例 |
|---|---|---|
| `1` | 产品主版本 | 当前固定为 1 |
| `YY` | 年份后两位 | 2026 年为 26 |
| `M` | 月份，无前导 0 | 6 月为 6 |
| `N` | 本月修改次数 | 本月第 12 次提交为 12 |

示例：

```text
1.26.6.12
```

当月份变化时，`N` 重新从 1 开始。

### 5.2 版本来源文件

建议新增：

```text
crates/desktop-slint/VERSION
```

内容示例：

```text
1.26.6.12
```

同时保持 `Cargo.toml` 中 package version 用于 Rust 包管理，不强制等同产品展示版本。原因：

1. Cargo 版本受 semver 约束和依赖生态影响。
2. 桌面产品展示版本可以更频繁递增。
3. 减少自动提交 hook 修改 `Cargo.lock` 的风险。

### 5.3 构建期注入

`crates/desktop-slint/build.rs` 读取 `VERSION`，写入环境变量：

```rust
println!("cargo:rustc-env=TEX2DOC_DESKTOP_VERSION={version}");
```

Rust 侧读取：

```rust
pub const DESKTOP_VERSION: &str = env!("TEX2DOC_DESKTOP_VERSION");
```

UI 展示：

```text
Tex2Doc Desktop 1.26.6.12
```

### 5.4 git 提交自动递增

建议新增脚本：

```text
scripts/bump_desktop_version.ps1
scripts/install_desktop_git_hooks.ps1
```

`bump_desktop_version.ps1` 逻辑：

1. 获取当前日期。
2. 计算目标前缀：`1.YY.M`。
3. 读取 `crates/desktop-slint/VERSION`。
4. 如果当前版本前缀等于目标前缀，则 `N += 1`。
5. 如果当前版本前缀不同，则版本改为 `1.YY.M.1`。
6. 写回 `VERSION`。
7. 自动 `git add crates/desktop-slint/VERSION`。

`install_desktop_git_hooks.ps1` 安装 `.git/hooks/pre-commit`：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\bump_desktop_version.ps1
```

注意：

1. hook 应只在真实提交时递增，不在 `cargo check`、`cargo test` 或普通构建时递增。
2. hook 修改版本后应自动 stage `VERSION`。
3. 如果用户使用 GUI Git 工具，仍会触发 `.git/hooks/pre-commit`。
4. CI 环境如果不需要自动递增，可通过环境变量跳过：

```powershell
$env:TEX2DOC_SKIP_VERSION_BUMP = "1"
```

### 5.5 边界情况

| 场景 | 处理策略 |
|---|---|
| 同一天多次提交 | 每次提交 `N + 1` |
| 跨月首次提交 | `N` 重置为 1 |
| amend 提交 | 默认仍递增；如果不希望递增，提交前设置 `TEX2DOC_SKIP_VERSION_BUMP=1` |
| merge commit | 默认递增；如自动合并不希望递增，可在 CI 设置跳过 |
| rebase replay | hook 会重复递增；建议 rebase 前临时跳过 hook |

---

## 六、Settings 页改造

Settings 页新增两个分组。

### 6.1 Appearance

```text
Appearance
├─ Language: [English v]
├─ Theme:    [System v]
└─ Preview:  [primary button] [secondary button] [status badge]
```

要求：

1. 修改 Language 后立即刷新界面。
2. 修改 Theme 后立即刷新界面颜色。
3. 点击 Save Settings 持久化到设置文件。
4. 如果修改后未保存，离开页面不阻止，但下次启动恢复上次保存值；首期也可以沿用当前 opportunistic persist 策略。

### 6.2 About

```text
About
├─ Product: Tex2Doc Desktop
├─ Version: 1.26.6.12
├─ Build: local / release / commit hash
└─ Diagnostics: [Export Diagnostics]
```

建议显示短 commit hash，但版本号仍以 `VERSION` 为准。

---

## 七、实施阶段

### 阶段 D：i18n 基础设施

目标：建立语言枚举、翻译 catalog、设置持久化和 UI 文案注入。

改动：

1. 新增 `i18n` 模块和 `translations/*.json`。
2. `Settings` 增加 `locale` 字段。
3. `main.rs` 启动时加载 locale 并注入 Slint 文案。
4. Settings 页增加 Language 下拉选择。
5. 增加翻译 key 完整性测试。

验收：

```powershell
cargo check -p doc-desktop-slint
cargo test -p doc-desktop-slint
```

### 阶段 E：主题系统

目标：建立主题 token，并完成主界面颜色统一切换。

改动：

1. 新增 `theme` 模块和 `ThemePalette`。
2. `Settings` 增加 `theme` 字段。
3. Slint 根组件增加颜色 token 属性。
4. Convert / Settings / Account / Billing / History 使用 token 替代硬编码颜色。
5. Settings 页增加 Theme 下拉选择。

验收：

1. Light / Dark / High Contrast 切换后全部页面颜色同步变化。
2. 主按钮、次按钮、危险状态、成功状态颜色一致。
3. 进度条、状态提示、历史项边框使用统一 token。

### 阶段 F：版本号自动化

目标：实现 `1.YY.M.N` 产品版本号，提交时自动递增。

改动：

1. 新增 `crates/desktop-slint/VERSION`。
2. `build.rs` 注入 `TEX2DOC_DESKTOP_VERSION`。
3. UI 标题和 Settings/About 展示版本。
4. 新增 `scripts/bump_desktop_version.ps1`。
5. 新增 `scripts/install_desktop_git_hooks.ps1`。
6. 文档说明 hook 安装与跳过方式。

验收：

1. `VERSION` 能被 UI 展示。
2. 执行一次 git commit 前，版本号自动 `N + 1`。
3. 跨月时 `N` 重置为 1。
4. 设置 `TEX2DOC_SKIP_VERSION_BUMP=1` 时不递增。

### 阶段 G：体验细节补齐

目标：结合原 UI/UX 报告继续完善交互体验。

优先项：

1. 异步按钮 busy 状态。
2. 转换阶段进度。
3. 默认输出路径自动填充。
4. 键盘快捷键。
5. History 结构化列表。
6. Account / Billing 状态 badge。

---

## 八、测试策略

### 8.1 单元测试

1. i18n key 完整性：
   - 所有语言文件 key 集合与 `en.json` 一致。
   - 不允许空翻译。
2. locale fallback：
   - 非法 locale 回退 `en`。
   - 缺失 key 回退英语。
3. theme palette：
   - 所有主题必须提供完整 token。
   - 颜色字符串必须可解析。
4. version bump：
   - 同月递增。
   - 跨月重置。
   - 非法 VERSION 文件报错。
   - skip 环境变量生效。

### 8.2 UI smoke

手工验证：

1. 首次启动默认英语。
2. 切换简体中文后所有 Tab 与主按钮更新。
3. 切换繁体中文、法文、日文、德文后界面不溢出。
4. 切换 Dark 主题后所有页面背景、文本、边框可读。
5. High Contrast 下 focus、错误、成功状态清晰。
6. 重启应用后语言和主题保持。
7. 版本号在窗口标题和 Settings/About 中一致。

### 8.3 回归测试

保留转换链路测试：

```powershell
cargo check -p doc-desktop-slint
cargo test -p doc-desktop-slint
.\scripts\test_paper3_frontend_docx.ps1 -GenerateDocx -DocxPath E:\tmp\paper3-frontend-output.docx
```

---

## 九、风险与规避

| 风险 | 影响 | 规避 |
|---|---|---|
| 多语言文本变长导致按钮或布局溢出 | 法文、德文 UI 破版 | 按钮宽度使用 min/max，关键按钮允许换行或缩短文案 |
| Slint 颜色属性传递过多 | 根组件属性膨胀 | 使用 `ThemeTokens` 分组概念，首期属性化，后续再抽组件 |
| 翻译 key 分散难维护 | 文案不一致 | 统一 `catalog.rs` 管理 key 常量 |
| pre-commit 自动修改版本影响 amend/rebase | 版本号跳跃 | 支持 `TEX2DOC_SKIP_VERSION_BUMP=1` |
| Cargo version 与产品 version 混淆 | 发布信息混乱 | Cargo 版本保持 semver，桌面产品版本使用 `VERSION` |
| 主题切换只改局部颜色 | 页面视觉不统一 | 所有页面必须通过 token 取色，不允许新增硬编码颜色 |

---

## 十、推荐落地顺序

建议按以下顺序实施：

1. **先做版本号管理**：范围最小，可快速建立产品版本展示和自动递增。
2. **再做主题 token**：为后续 UI 美化打基础，避免继续新增硬编码颜色。
3. **再做 i18n 基础设施**：文案迁移涉及面广，建议逐页替换。
4. **最后补原 UI/UX 报告中的交互细节**：busy 状态、进度阶段、快捷键、History 结构化。

如果需要最小可交付版本，建议首批只做：

1. `VERSION` + 标题展示 + Settings/About 展示。
2. Settings 页 Language / Theme 下拉框。
3. Convert 页和 Tab 名称的 6 语言文案。
4. Light / Dark 两套主题 token。

---

## 十一、结论

新版本方案的核心是把 UI 从「页面结构完成」推进到「产品基础设施完成」：

1. 多语言能力解决国际用户可用性，默认英语符合通用发行预期。
2. 主题系统解决视觉一致性，使后续美化不再依赖零散样式。
3. 自动版本号解决发布和诊断追踪，确保每次 git 提交都有可识别桌面版本。
4. 方案按阶段拆分，能在保持现有转换功能稳定的前提下逐步交付。
