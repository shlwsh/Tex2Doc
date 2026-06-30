# Tex2Doc 商业化 UI/UX 设计系统与前端重构方案
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



日期：2026-06-23  
范围：Flutter 客户端、Slint 桌面端、项目 UI 设计技能

## 1. 现有问题审查

### Flutter

- 单文件承载主页面、状态、表单和结果区，组件边界不清晰。
- 页面以 demo card 纵向堆叠为主，缺少商业 SaaS/桌面工具常见的信息架构。
- 色彩、间距、圆角和状态色散落在页面组件里。
- 用户可见文案硬编码，缺少中英文切换能力。
- 缺少统一的空、加载、错误、禁用、权限不足状态表达。
- 响应式只有最大宽度约束，没有桌面/平板/移动的信息布局差异。

### Slint

- `MainWindow` 已有部分颜色 token 和 i18n 属性，但页面仍大量内联。
- 主题只支持旧的 `system/light/dark/high-contrast`，不能覆盖商业化多色调要求。
- `pages/*` 中仍存在 hardcoded copy 和局部颜色，后续需要继续迁移到统一 tokens。
- 部分状态文案在 Rust callback 中拼接，仍需逐步转为 i18n key + 数据模板。

## 2. 商业化设计系统

### 色彩

统一按语义命名：

- Primary：主要动作和选中状态。
- Secondary：辅助信息和弱引导。
- Background / Surface：页面背景与工作区表面。
- Text Primary / Secondary / Disabled：正文层级。
- Border / Divider：边框与分隔。
- Success / Warning / Error / Info：状态反馈。

Flutter 已通过 `ThemeData`、`ColorScheme` 和 `AppColorTokens` 管理。Slint 已扩展 `theme.rs` palette。

### 主题

支持：

- Default
- Blue
- Green
- Purple
- Orange
- Dark

Flutter 通过 `AppThemeTone` 切换并持久化到 `shared_preferences`。Slint 通过 `theme::normalize_theme()` 和 `theme::palette()` 支持同名主题。

### 字体与排版

Flutter 统一：

- Display：32 / 700
- Title：22、18 / 700
- Body：16、14
- Caption：12
- Button：14 / 600

Slint 当前沿用属性级字号，后续建议继续抽出 `font-size-title/body/caption` tokens。

### 间距与圆角

Flutter 已建立：

- `AppSpacing`: 4 / 8 / 12 / 16 / 24 / 32 / 48
- `AppRadius`: 6 / 8 / 10
- `AppMotion`: 120ms / 180ms

Slint 主窗口继续使用 8 / 10 / 12 / 16 节奏，并通过 palette 控制表面与边框。

## 3. Flutter 实现

新增文件：

- `flutter_app/lib/ui/app_tokens.dart`
- `flutter_app/lib/ui/app_theme.dart`
- `flutter_app/lib/ui/app_i18n.dart`
- `flutter_app/lib/ui/app_components.dart`

重构点：

- `DocEngineApp` 改为 Stateful root，支持主题和语言持久化。
- `MaterialApp` 接入 `flutter_localizations` 与 `AppStringsDelegate`。
- 页面改为 `Sidebar + TopBar + Workspace` 产品壳。
- 桌面宽屏使用侧边导航与双栏工作区，平板/移动自动堆叠。
- `CommercialApiPanel` 保留注册、登录、用量、套餐业务逻辑。
- `ConvertPanel` 保留 ZIP 选择、本地转换和下载业务逻辑。
- 状态统一使用 `LoadingState`、`EmptyState`、`ErrorState`、`StatusPill`。

## 4. Slint 实现

修改文件：

- `crates/desktop-slint/src/theme.rs`
- `crates/desktop-slint/src/ui/main.slint`
- `crates/desktop-slint/src/i18n.rs`
- `crates/desktop-slint/src/main.rs`

重构点：

- 主题从旧枚举扩展为商业色调：`default/blue/green/purple/orange/dark`。
- 旧设置 `system/light/high-contrast` 自动回退到 `default`，避免破坏已有配置。
- `MainWindow.ui-theme` 默认改为 `default`。
- 设置页主题下拉列表同步新主题。
- i18n key 增加 `theme.default`、`theme.blue`、`theme.green`、`theme.purple`、`theme.orange`、`theme.dark`。

## 5. 技能沉淀

新增项目技能：

```text
.claude/skills/commercial-ui-design/SKILL.md
.claude/skills/commercial-ui-design/agents/openai.yaml
```

用途：

- 审查 Flutter/Slint 界面是否商业化。
- 指导设计系统、主题、i18n、状态、响应式和验证流程。
- 为后续 UI 迭代提供稳定工作流。

## 6. 后续建议

1. 将 Slint `MainWindow` 内联 tab 逐步拆到 `ui/pages/*` 并实际复用。
2. 为 Slint 增加真正的 reusable `AppCard`、`MetricCard`、`StatusPill`、`FieldRow` 组件。
3. 将 Rust callback 中拼接的可见文案转为 i18n 模板。
4. Flutter 后续可增加真实导航状态、账号 session 恢复、云转换页面和套餐表格。
5. 增加 Playwright/截图级视觉回归，覆盖桌面、平板、移动和深色模式。
