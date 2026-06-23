# Tex2Doc Slint 桌面端商业化界面重构 — Flutter 一致性版

> 文档版本：v3
> 日期：2026-06-23
> 状态：**详细实现技术方案**
> 基准 Flutter 设计：`docs-zh/flutter/flutter-desktop-ui-design-20260623.md`

---

## 一、项目概述

### 1.1 背景与目标

Flutter 桌面应用已完成商业化界面重构（参见 `docs-zh/flutter/flutter-desktop-ui-design-20260623.md`），验证了设计方向和交互模式。Slint 桌面端需要**系统性重构**，以实现与 Flutter 桌面端在风格、功能和界面主题上的完全一致。

本方案是 v2 方案（`Tex2Doc-Slint桌面端商业化界面重构设计方案-v2-20260623.md`）的**Flutter 对齐版**，在 v2 基础上新增：
- 导航结构与 Flutter 完全对齐（侧边栏 248px、5 个模块、顶栏账号头像）
- 登录/注册窗口独立化（Tab 切换、最大宽度 420px）
- 独立转换记录和充值记录页面（DataTable 展示）
- 右上角账号头像 + PopupMenu（查看详情/修改密码/退出）
- 修改密码弹窗（三个密码输入框 + 客户端校验）
- 账号详情弹窗（头像 + 邮箱 + 套餐 + 注册时间）
- 充值 Mock 支付界面（按次/按日期两套按钮）

### 1.2 技术栈

| 技术 | 版本/说明 |
|------|----------|
| Slint | 1.16+ |
| Rust | 2021 edition |
| 状态管理 | Rust 侧 AppState + Slint in-out property |
| 设计系统 | Slint `DesignTokens` global + Flutter 颜色/字号/间距对齐 |
| 国际化 | Rust 侧翻译写入 Slint `I18n` global（绕过 ICU4X CJK 问题） |
| 主题 | 6 色系 + 暗色，Rust 侧同步到 Slint token |

### 1.3 范围

| 文件 | 操作 |
|------|------|
| `ui/tokens.slint` | 扩展，与 Flutter `app_tokens.dart` / `app_theme.dart` 对齐 |
| `ui/motion.slint` | 扩展，新增动画 token |
| `ui/i18n-strings.slint` | 扩展，覆盖全部 UI 文本，含 Flutter 新增 key |
| `ui/types.slint` | 扩展，新增 `ConversionRecord`、`RechargeRecord`、`ToastMessage` 等 |
| `ui/components/*.slint` | 新增 18 个组件 |
| `ui/pages/auth.slint` | 新增：独立登录/注册窗口（Tab 切换） |
| `ui/pages/sidebar.slint` | 新增：侧边栏导航（248px） |
| `ui/pages/topbar.slint` | 新增：顶栏（含语言/主题下拉、账号头像） |
| `ui/pages/convert.slint` | 重构：与 Flutter `_ConvertPanel` 对齐 |
| `ui/pages/account.slint` | 重构：账号总览 + 快捷操作（不含登录/注册） |
| `ui/pages/billing.slint` | 重构：充值按钮（按次/按日期 mock） |
| `ui/pages/history.slint` | 重构：DataTable 转换记录 |
| `ui/pages/recharge-records.slint` | 新增：充值记录 DataTable |
| `ui/pages/convert-records.slint` | 新增：转换记录 DataTable |
| `ui/main.slint` | 重构：从 517 行精简为 ~180 行布局骨架 |
| `src/theme.rs` | 扩展：`apply_theme_to_tokens()` 写入 Slint DesignTokens |
| `src/i18n.rs` | 扩展：全部 Flutter i18n key + `text_for()` 追加 zh-Hans/zh-Hant/ja-JP |
| `src/app_state.rs` | 扩展：`ConversionRecord`、`RechargeRecord`、`ToastState`、`AppStatusState` |
| `src/main.rs` | 扩展：新增 callback 绑定（change-password、view-profile 等） |
| `src/ui_bindings/` | 扩展：新 callback 绑定 |

---

## 二、Flutter 设计系统映射

### 2.1 设计令牌一一映射

Flutter 的设计令牌在 Slint 中以完全相同的语义值定义，确保视觉一致。

#### Flutter `app_tokens.dart` → Slint `tokens.slint`

| Flutter | Flutter 值 | Slint Token | Slint 值 |
|---------|-----------|------------|---------|
| `AppSpacing.xxs` | 4 | `token-space-0` | 4px |
| `AppSpacing.xs` | 8 | `token-space-2` | 8px |
| `AppSpacing.sm` | 12 | `token-space-3` | 12px |
| `AppSpacing.md` | 16 | `token-space-4` | 16px |
| `AppSpacing.lg` | 24 | `token-space-6` | 24px |
| `AppSpacing.xl` | 32 | `token-space-8` | 32px |
| `AppSpacing.xxl` | 48 | `token-space-12` | 48px |
| `AppRadius.sm` | 6 | `token-radius-sm` | 6px |
| `AppRadius.md` | 8 | `token-radius-md` | 8px |
| `AppRadius.lg` | 10 | `token-radius-lg` | 10px |
| `AppMotion.fast` | 120ms | `token-transition-fast` | 120ms |
| `AppMotion.normal` | 180ms | `token-transition-base` | 180ms |
| `AppMotion.curve` | easeOutCubic | `token-easing-standard` | "ease-out" |
| `AppBreakpoints.tablet` | 1040px | `token-sidebar-width` | 248px |

#### Flutter `app_theme.dart` 色值 → Slint `tokens.slint`

Flutter `ColorScheme.fromSeed(seed)` + `AppColorTokens` 的色值，在 Slint 中精确对应：

| Flutter 用途 | Flutter 浅色值 | Flutter 暗色值 | Slint Token（浅色） | Slint Token（暗色） |
|-------------|--------------|--------------|-------------------|-------------------|
| 背景 | `#F6F8FB` | `#0B1120` | `token-window-bg` | `token-window-bg` |
| 卡片表面 | `#FFFFFF` | `#111827` | `token-surface-1` | `token-surface-1` |
| 侧边栏/区域 | `#F1F5F9` | `#182235` | `token-surface-3` | `token-surface-3` |
| 边框 | `#D8DEE8` | `#334155` | `token-border` | `token-border` |
| 主文字 | `#172033` | `#F8FAFC` | `token-text-primary` | `token-text-primary` |
| 次文字 | `#42526B` | `#CBD5E1` | `token-text-secondary` | `token-text-secondary` |
| 辅助文字 | `#94A3B8` | `#64748B` | `token-text-muted` | `token-text-muted` |
| **accent（default seed）** | `#2563EB` | `#60A5FA` | `token-accent` | `token-accent` |
| **accent（blue seed）** | `#1D4ED8` | `#60A5FA` | - | - |
| **accent（green seed）** | `#047857` | `#34D399` | - | - |
| **accent（purple seed）** | `#7C3AED` | `#A78BFA` | - | - |
| **accent（orange seed）** | `#EA580C` | `#FB923C` | - | - |
| **accent（dark seed）** | `#60A5FA` | `#60A5FA` | - | - |
| `AppColorTokens.success` | `#0F8A5F` | `#34D399` | `token-success` | `token-success` |
| `AppColorTokens.warning` | `#B7791F` | `#FBBF24` | `token-warning` | `token-warning` |
| `AppColorTokens.info` | `#2563EB` | `#60A5FA` | `token-info` | `token-info` |
| `AppColorTokens.divider` | `#D8DEE8` | `#334155` | `token-border` | `token-border` |
| `AppColorTokens.disabledText` | `#94A3B8` | `#64748B` | `token-disabled-text` | `token-disabled-text` |

#### Flutter 字号 → Slint token

| Flutter textTheme | 字号/字重 | Slint Token |
|------------------|----------|------------|
| `displaySmall` | 32/700 | `token-font-size-3xl` + `token-font-weight-bold` |
| `titleLarge` | 22/700 | `token-font-size-xl` + `token-font-weight-bold` |
| `titleMedium` | 18/700 | `token-font-size-lg` + `token-font-weight-bold` |
| `bodyLarge` | 16 | `token-font-size-lg` |
| `bodyMedium` | 14 | `token-font-size-md` |
| `bodySmall` | 12 | `token-font-size-base` |
| `labelLarge` | 14/600 | `token-font-size-md` + `token-font-weight-semibold` |

### 2.2 目标 DesignToken 完整定义

```slint
// crates/desktop-slint/src/ui/tokens.slint

export global DesignTokens {
    // === Window / Background ===
    in-out property <color> token-window-bg:           #F6F8FB;
    in-out property <color> token-surface-1:           #FFFFFF;  // 卡片/对话框
    in-out property <color> token-surface-2:           #FFFFFF;  // 输入框/选中行
    in-out property <color> token-surface-3:           #F1F5F9;  // 侧边栏背景
    in-out property <color> token-surface-4:           #EBEEF3;  // 区块背景
    in-out property <color> token-surface-overlay:      #00000066;

    // === Border ===
    in-out property <color> token-border:              #D7DEE8;
    in-out property <color> token-border-strong:        #B8C2D4;
    in-out property <color> token-border-focus:       #2563EB;

    // === Text ===
    in-out property <color> token-text-primary:        #172033;
    in-out property <color> token-text-secondary:      #42526B;
    in-out property <color> token-text-muted:         #94A3B8;
    in-out property <color> token-text-inverse:       #FFFFFF;
    in-out property <color> token-text-link:           #2563EB;
    in-out property <color> token-text-disabled:       #9CA3AF;

    // === Accent (default tone) ===
    in-out property <color> token-accent:             #2563EB;
    in-out property <color> token-accent-hover:        #1D4ED8;
    in-out property <color> token-accent-pressed:     #1E40AF;
    in-out property <color> token-accent-subtle:       #EFF6FF;

    // === Semantic Status ===
    in-out property <color> token-success:            #0F8A5F;
    in-out property <color> token-success-subtle:     #ECFDF5;
    in-out property <color> token-warning:             #B7791F;
    in-out property <color> token-warning-subtle:      #FFFBEB;
    in-out property <color> token-danger:              #C2413A;
    in-out property <color> token-danger-subtle:       #FEF2F2;
    in-out property <color> token-info:              #2563EB;
    in-out property <color> token-info-subtle:        #EFF6FF;

    // === Disabled ===
    in-out property <color> token-disabled-bg:        #F1F5F9;
    in-out property <color> token-disabled-text:     #9CA3AF;
    in-out property <color> token-disabled-border:    #E5E7EB;

    // === Overlay ===
    in-out property <color> token-overlay-light:      #FFFFFF80;
    in-out property <color> token-overlay-dark:       #00000033;

    // === Typography ===
    in-out property <length> token-font-size-xs:      10px;
    in-out property <length> token-font-size-sm:      11px;
    in-out property <length> token-font-size-base:   12px;  // Flutter bodySmall
    in-out property <length> token-font-size-md:       14px;  // Flutter bodyMedium
    in-out property <length> token-font-size-lg:      16px;  // Flutter bodyLarge / titleMedium base
    in-out property <length> token-font-size-xl:      20px;  // Flutter titleLarge
    in-out property <length> token-font-size-2xl:     22px;
    in-out property <length> token-font-size-3xl:      32px;  // Flutter displaySmall
    in-out property <length> token-font-weight-normal:    400;
    in-out property <length> token-font-weight-medium:    500;
    in-out property <length> token-font-weight-semibold:  600;
    in-out property <length> token-font-weight-bold:      700;
    in-out property <length> token-line-height-tight:    1.2;
    in-out property <length> token-line-height-base:    1.5;
    in-out property <length> token-line-height-relaxed: 1.75;

    // === Spacing (Flutter app_tokens.dart 对齐) ===
    in-out property <length> token-space-0:  0px;
    in-out property <length> token-space-1:  4px;   // xxs
    in-out property <length> token-space-2:  8px;    // xs
    in-out property <length> token-space-3:  12px;   // sm
    in-out property <length> token-space-4:  16px;   // md
    in-out property <length> token-space-5:  20px;
    in-out property <length> token-space-6:  24px;  // lg
    in-out property <length> token-space-7:  28px;
    in-out property <length> token-space-8:  32px;  // xl
    in-out property <length> token-space-10: 40px;
    in-out property <length> token-space-12: 48px;  // xxl

    // === Radius ===
    in-out property <length> token-radius-sm:   3px;
    in-out property <length> token-radius-md:   6px;  // Flutter AppRadius.sm
    in-out property <length> token-radius-lg:   8px;  // Flutter AppRadius.md
    in-out property <length> token-radius-xl:   10px; // Flutter AppRadius.lg
    in-out property <length> token-radius-full: 9999px;

    // === Layout ===
    in-out property <length> token-header-height:    64px;   // 顶栏高度
    in-out property <length> token-sidebar-width:    248px;  // Flutter 侧边栏
    in-out property <length> token-content-max-width: 1180px;
    in-out property <length> token-panel-gap:        16px;
    in-out property <length> token-section-gap:      24px;

    // === Motion ===
    in-out property <duration> token-transition-fast:    120ms;
    in-out property <duration> token-transition-base:   180ms; // Flutter normal
    in-out property <duration> token-transition-slow:    300ms;
    in-out property <string>  token-easing-standard:   "ease-out";
    in-out property <string>  token-easing-decelerate:  "cubic-bezier(0.0, 0.0, 0.2, 1.0)";
    in-out property <string>  token-easing-accelerate:   "cubic-bezier(0.4, 0.0, 1.0, 1.0)";
    in-out property <string>  token-easing-sharp:       "cubic-bezier(0.4, 0.0, 0.2, 1.0)";

    // === Icon ===
    in-out property <length> token-icon-size-sm:   14px;
    in-out property <length> token-icon-size-base: 16px;
    in-out property <length> token-icon-size-lg:  20px;
    in-out property <length> token-icon-size-xl:  24px;

    // === Progress ===
    in-out property <length> token-progress-height: 4px;
    in-out property <length> token-progress-radius: 2px;
}
```

### 2.3 暗色主题 Token

暗色主题色值与 Flutter `app_theme.dart` 精确对齐：

```slint
export global DarkDesignTokens {
    // 与 DesignTokens 结构完全一致，仅色值不同
    in-out property <color> token-window-bg:       #0B1120;  // Flutter dark scaffold
    in-out property <color> token-surface-1:        #111827;  // Flutter dark surface
    in-out property <color> token-surface-2:      #111827;
    in-out property <color> token-surface-3:      #182235;  // Flutter surfaceContainerHighest dark
    in-out property <color> token-surface-4:       #1F2937;
    in-out property <color> token-border:           #334155;  // Flutter dark outline
    in-out property <color> token-border-strong:   #475569;
    in-out property <color> token-border-focus:   #60A5FA;

    in-out property <color> token-text-primary:     #F8FAFC;
    in-out property <color> token-text-secondary:  #CBD5E1;
    in-out property <color> token-text-muted:     #64748B;
    in-out property <color> token-text-inverse:   #0B1120;
    in-out property <color> token-text-link:       #60A5FA;
    in-out property <color> token-text-disabled:   #475569;

    // accent 跟随 tone，暗色统一用 dark seed
    in-out property <color> token-accent:         #60A5FA;
    in-out property <color> token-accent-hover:    #93C5FD;
    in-out property <color> token-accent-pressed:  #3B82F6;
    in-out property <color> token-accent-subtle:   #1E3A5F;

    in-out property <color> token-success:         #34D399;
    in-out property <color> token-success-subtle:   #052E16;
    in-out property <color> token-warning:         #FBBF24;
    in-out property <color> token-warning-subtle:   #271A06;
    in-out property <color> token-danger:         #F87171;
    in-out property <color> token-danger-subtle:  #2D0B0B;
    in-out property <color> token-info:            #60A5FA;
    in-out property <color> token-info-subtle:     #082032;

    in-out property <color> token-disabled-bg:      #1F2937;
    in-out property <color> token-disabled-text:   #475569;
    in-out property <color> token-disabled-border:  #374151;
}
```

---

## 三、国际化（i18n）扩展

### 3.1 Flutter i18n key 完整追加

Flutter `app_i18n.dart` 的所有 key 追加到 Slint `I18n` global，格式从 `.` 改为 `_`（Slint 命名规范）：

```slint
// crates/desktop-slint/src/ui/i18n-strings.slint

export global I18n {
    in-out property <string> locale: "en";

    // === Flutter app.* ===
    in-out property <string> app-title: "Tex2Doc";
    in-out property <string> app-subtitle: "Commercial LaTeX to DOCX conversion workspace";
    in-out property <string> app-subtitle-zh: "LaTeX 到 DOCX 的商业级转换工作台";
    in-out property <string> topbar-platform: "Platform";
    in-out property <string> topbar-platform-zh: "平台";

    // === Flutter nav.* ===
    in-out property <string> nav-account: "Account";
    in-out property <string> nav-account-zh: "账号";
    in-out property <string> nav-recharge: "Recharge";
    in-out property <string> nav-recharge-zh: "充值";
    in-out property <string> nav-convert: "Convert";
    in-out property <string> nav-convert-zh: "转换";
    in-out property <string> nav-convert-records: "Conversion Records";
    in-out property <string> nav-convert-records-zh: "转换记录";
    in-out property <string> nav-recharge-records: "Recharge Records";
    in-out property <string> nav-recharge-records-zh: "充值记录";

    // === Flutter auth.* ===
    in-out property <string> auth-login-tab: "Login";
    in-out property <string> auth-login-tab-zh: "登录";
    in-out property <string> auth-register-tab: "Register";
    in-out property <string> auth-register-tab-zh: "注册";
    in-out property <string> auth-login-title: "Sign In";
    in-out property <string> auth-login-title-zh: "登录账号";
    in-out property <string> auth-register-title: "Create Account";
    in-out property <string> auth-register-title-zh: "注册账号";
    in-out property <string> auth-sign-in-first: "Please sign in to continue.";
    in-out property <string> auth-sign-in-first-zh: "请先登录以继续操作。";

    // === Flutter account.* ===
    in-out property <string> account-email: "Email";
    in-out property <string> account-email-zh: "邮箱";
    in-out property <string> account-password: "Password";
    in-out property <string> account-password-zh: "密码";
    in-out property <string> account-confirm-password: "Confirm Password";
    in-out property <string> account-confirm-password-zh: "确认密码";
    in-out property <string> account-plan: "Plan";
    in-out property <string> account-plan-zh: "套餐";
    in-out property <string> account-sign-in-gate: "Sign in or register first.";
    in-out property <string> account-sign-in-gate-zh: "请先登录或注册。";
    in-out property <string> account-saved-in-short: "Signed in";
    in-out property <string> account-saved-in-short-zh: "已登录";
    in-out property <string> account-overview-title: "Account Overview";
    in-out property <string> account-overview-title-zh: "账号总览";
    in-out property <string> account-overview-description: "Review account profile, plan quota, recharge records, and conversions.";
    in-out property <string> account-overview-description-zh: "查看当前账号、套餐额度、充值记录与转换记录。";
    in-out property <string> account-query-records: "Query Account Records";
    in-out property <string> account-query-records-zh: "查询账号记录";
    in-out property <string> account-overview-loaded: "Loaded {recharges} recharge records and {conversions} conversions.";
    in-out property <string> account-overview-loaded-zh: "已加载 {recharges} 条充值记录、{conversions} 条转换记录。";
    in-out property <string> account-registered: "Registered {email}, plan {plan}";
    in-out property <string> account-registered-zh: "已注册 {email}，套餐 {plan}";
    in-out property <string> account-signed-in: "Signed in {email}, plan {plan}";
    in-out property <string> account-signed-in-zh: "已登录 {email}，套餐 {plan}";
    in-out property <string> account-usage: "Plan {plan}: {used}/{limit}, remaining {remaining}; entitlement {entitlement}";
    in-out property <string> account-usage-zh: "套餐 {plan}：{used}/{limit}，剩余 {remaining}；权益 {entitlement}";
    in-out property <string> account-logout: "Sign Out";
    in-out property <string> account-logout-zh: "退出登录";
    in-out property <string> account-change-password: "Change Password";
    in-out property <string> account-change-password-zh: "修改密码";
    in-out property <string> account-view-profile: "View Profile";
    in-out property <string> account-view-profile-zh: "查看详情";
    in-out property <string> account-profile-title: "Account Details";
    in-out property <string> account-profile-title-zh: "账号详情";
    in-out property <string> account-current-plan: "Current Plan";
    in-out property <string> account-current-plan-zh: "当前套餐";
    in-out property <string> account-member-since: "Member Since";
    in-out property <string> account-member-since-zh: "注册时间";
    in-out property <string> account-quota-used: "Quota Used";
    in-out property <string> account-quota-used-zh: "已用额度";
    in-out property <string> account-quota-remaining: "Quota Remaining";
    in-out property <string> account-quota-remaining-zh: "剩余额度";
    in-out property <string> account-change-password-title: "Change Password";
    in-out property <string> account-change-password-title-zh: "修改密码";
    in-out property <string> account-old-password: "Current Password";
    in-out property <string> account-old-password-zh: "当前密码";
    in-out property <string> account-new-password: "New Password";
    in-out property <string> account-new-password-zh: "新密码";
    in-out property <string> account-confirm-new-password: "Confirm New Password";
    in-out property <string> account-confirm-new-password-zh: "确认新密码";
    in-out property <string> account-password-mismatch: "Passwords do not match";
    in-out property <string> account-password-mismatch-zh: "两次输入的密码不一致";
    in-out property <string> account-password-changed: "Password changed successfully";
    in-out property <string> account-password-changed-zh: "密码修改成功";
    in-out property <string> account-password-change-failed: "Failed to change password";
    in-out property <string> account-password-change-failed-zh: "密码修改失败";
    in-out property <string> account-password-too-short: "Password must be at least 6 characters";
    in-out property <string> account-password-too-short-zh: "密码长度不能少于 6 位";
    in-out property <string> account-display-name: "Display Name";
    in-out property <string> account-display-name-zh: "显示名称";
    in-out property <string> account-api-base-url: "API Base URL";
    in-out property <string> account-api-base-url-zh: "API 地址";

    // === Flutter recharge.* ===
    in-out property <string> recharge-title: "Recharge";
    in-out property <string> recharge-title-zh: "充值";
    in-out property <string> recharge-description: "Buy conversion rights by count or duration. Mock payment settles immediately.";
    in-out property <string> recharge-description-zh: "按次或按日期购买转换权益，当前使用 mock 支付完成到账。";
    in-out property <string> recharge-count-title: "By count";
    in-out property <string> recharge-count-title-zh: "按次充值";
    in-out property <string> recharge-date-title: "By duration";
    in-out property <string> recharge-date-title-zh: "日期充值";
    in-out property <string> recharge-query-records: "Query recharge records";
    in-out property <string> recharge-query-records-zh: "查询充值记录";
    in-out property <string> recharge-records: "Recharge records";
    in-out property <string> recharge-records-zh: "充值记录";
    in-out property <string> recharge-sign-in-required: "Sign in before recharge.";
    in-out property <string> recharge-sign-in-required-zh: "请先登录后再充值。";
    in-out property <string> recharge-mock-provider: "Mock payment enabled";
    in-out property <string> recharge-mock-provider-zh: "mock 支付已启用";
    in-out property <string> recharge-mock-paid: "Mock payment settled CNY {amount} through {provider}.";
    in-out property <string> recharge-mock-paid-zh: "mock 支付完成，到账 ¥{amount}，渠道 {provider}。";

    // === Flutter convert.* ===
    in-out property <string> convert-title: "Document conversion";
    in-out property <string> convert-title-zh: "文档转换";
    in-out property <string> convert-description: "Upload a TeX project ZIP, choose the main file, and export DOCX.";
    in-out property <string> convert-description-zh: "上传 TeX 项目 ZIP，选择主文件并生成 DOCX。";
    in-out property <string> convert-step-upload: "1. Package the full LaTeX project as a ZIP.";
    in-out property <string> convert-step-upload-zh: "1. 将完整 LaTeX 项目打包为 ZIP 后上传。";
    in-out property <string> convert-step-main-tex: "2. Enter the main TeX path inside the ZIP.";
    in-out property <string> convert-step-main-tex-zh: "2. 填写 ZIP 内主 TeX 文件相对路径。";
    in-out property <string> convert-step-convert: "3. Run the cloud semantic engine and download DOCX.";
    in-out property <string> convert-step-convert-zh: "3. 启动云端语义引擎并下载 DOCX。";
    in-out property <string> convert-package-hint: "The ZIP root should include the main tex, bib, images, cls/sty and other dependencies; do not upload only one tex file.";
    in-out property <string> convert-package-hint-zh: "ZIP 根目录应包含主 tex、bib、图片、cls/sty 等依赖；不要只上传单个 tex 文件。";
    in-out property <string> convert-signed-in-ready: "Signed in. Conversion is available.";
    in-out property <string> convert-signed-in-ready-zh: "已登录，可使用转换功能。";
    in-out property <string> convert-main-tex: "Main TeX file";
    in-out property <string> convert-main-tex-zh: "主 TeX 文件";
    in-out property <string> convert-main-tex-hint: "main-jos.tex";
    in-out property <string> convert-main-tex-hint-zh: "main-jos.tex";
    in-out property <string> convert-no-file: "No file selected";
    in-out property <string> convert-no-file-zh: "未选择文件";
    in-out property <string> convert-sign-in-required: "Register or sign in first to use the cloud semantic engine.";
    in-out property <string> convert-sign-in-required-zh: "请先注册或登录，以使用云端语义引擎转换。";
    in-out property <string> convert-converting: "Converting...";
    in-out property <string> convert-converting-zh: "正在转换...";
    in-out property <string> convert-success: "Completed {size} KB in {elapsed} ms";
    in-out property <string> convert-success-zh: "完成 {size} KB，用时 {elapsed} ms";
    in-out property <string> convert-cloud-success: "Cloud semantic engine completed {size} KB in {elapsed} ms";
    in-out property <string> convert-cloud-success-zh: "云端语义引擎完成 {size} KB，用时 {elapsed} ms";
    in-out property <string> convert-output: "Output";
    in-out property <string> convert-output-zh: "产物";
    in-out property <string> convert-query-records: "Query conversion records";
    in-out property <string> convert-query-records-zh: "查询转换记录";
    in-out property <string> convert-records: "Conversion records";
    in-out property <string> convert-records-zh: "转换记录";
    in-out property <string> convert-records-loaded: "Loaded {count} conversion records.";
    in-out property <string> convert-records-loaded-zh: "已加载 {count} 条转换记录。";
    in-out property <string> convert-logs: "Conversion logs";
    in-out property <string> convert-logs-zh: "转换日志";
    in-out property <string> convert-log-rejected-size: "File exceeds 10 MB and was rejected.";
    in-out property <string> convert-log-rejected-size-zh: "文件超过 10 MB，已拒绝上传。";
    in-out property <string> convert-log-file-selected: "Selected {file}, {size} MB.";
    in-out property <string> convert-log-file-selected-zh: "已选择 {file}，大小 {size} MB。";
    in-out property <string> convert-log-started: "Started conversion with main file {main}.";
    in-out property <string> convert-log-started-zh: "开始转换，主文件 {main}。";
    in-out property <string> convert-log-uploading: "Uploading ZIP to the commercial API.";
    in-out property <string> convert-log-uploading-zh: "正在上传 ZIP 到商业 API。";
    in-out property <string> convert-log-uploaded: "Upload completed, upload_id={upload}.";
    in-out property <string> convert-log-uploaded-zh: "上传完成，upload_id={upload}。";
    in-out property <string> convert-log-job-created: "Conversion job created, job_id={job}.";
    in-out property <string> convert-log-job-created-zh: "转换任务已创建，job_id={job}。";
    in-out property <string> convert-log-polling: "Polling job status: {status}.";
    in-out property <string> convert-log-polling-zh: "轮询任务状态：{status}。";
    in-out property <string> convert-log-finished: "Conversion completed. DOCX is ready.";
    in-out property <string> convert-log-finished-zh: "转换完成，DOCX 已可下载。";
    in-out property <string> convert-log-failed: "Conversion failed: {error}";
    in-out property <string> convert-log-failed-zh: "转换失败：{error}";
    in-out property <string> convert-log-records-loaded: "Conversion records loaded.";
    in-out property <string> convert-log-records-loaded-zh: "已查询转换记录。";

    // === Flutter metrics.* ===
    in-out property <string> metrics-quota: "Cloud quota";
    in-out property <string> metrics-quota-zh: "云端额度";
    in-out property <string> metrics-entitlement: "Conversion entitlement";
    in-out property <string> metrics-entitlement-zh: "转换权益";
    in-out property <string> metrics-count-balance: "count balance {count}";
    in-out property <string> metrics-count-balance-zh: "按次余额 {count}";
    in-out property <string> metrics-date-valid-until: "valid until {time}";
    in-out property <string> metrics-date-valid-until-zh: "有效期至 {time}";
    in-out property <string> metrics-preview-quota: "preview quota";
    in-out property <string> metrics-preview-quota-zh: "预览额度";
    in-out property <string> metrics-engine: "Engine status";
    in-out property <string> metrics-engine-zh: "引擎状态";
    in-out property <string> metrics-document: "Document output";
    in-out property <string> metrics-document-zh: "文档产物";

    // === Flutter empty.* ===
    in-out property <string> empty-no-data: "No data yet. Results will appear here after an operation.";
    in-out property <string> empty-no-data-zh: "暂无数据。完成一次操作后这里会显示结果。";

    // === Flutter error.* ===
    in-out property <string> error-network: "Network or service error. Check the API base URL.";
    in-out property <string> error-network-zh: "网络或服务异常，请检查 API 地址。";

    // === Flutter common.* ===
    in-out property <string> common-register: "Register";
    in-out property <string> common-register-zh: "注册";
    in-out property <string> common-login: "Login";
    in-out property <string> common-login-zh: "登录";
    in-out property <string> common-refresh: "Refresh";
    in-out property <string> common-refresh-zh: "刷新";
    in-out property <string> common-plans: "Plans";
    in-out property <string> common-plans-zh: "套餐";
    in-out property <string> common-upload: "Choose ZIP";
    in-out property <string> common-upload-zh: "选择 ZIP";
    in-out property <string> common-convert: "Start conversion";
    in-out property <string> common-convert-zh: "开始转换";
    in-out property <string> common-download: "Download DOCX";
    in-out property <string> common-download-zh: "下载 DOCX";
    in-out property <string> common-ready: "Ready";
    in-out property <string> common-ready-zh: "就绪";
    in-out property <string> common-loading: "Working...";
    in-out property <string> common-loading-zh: "处理中...";
    in-out property <string> common-empty: "No data";
    in-out property <string> common-empty-zh: "暂无数据";
    in-out property <string> common-error: "Error";
    in-out property <string> common-error-zh: "出错";
    in-out property <string> common-disabled: "Disabled";
    in-out property <string> common-disabled-zh: "不可用";
    in-out property <string> common-permission-denied: "Permission denied";
    in-out property <string> common-permission-denied-zh: "权限不足";
    in-out property <string> common-confirm: "OK";
    in-out property <string> common-confirm-zh: "确定";
    in-out property <string> common-cancel: "Cancel";
    in-out property <string> common-cancel-zh: "取消";
    in-out property <string> common-save: "Save";
    in-out property <string> common-save-zh: "保存";
    in-out property <string> common-close: "Close";
    in-out property <string> common-close-zh: "关闭";
    in-out property <string> common-copy: "Copy";
    in-out property <string> common-copy-zh: "复制";
    in-out property <string> common-retry: "Retry";
    in-out property <string> common-retry-zh: "重试";

    // === Flutter settings.* ===
    in-out property <string> settings-theme: "Theme";
    in-out property <string> settings-theme-zh: "主题";
    in-out property <string> settings-language: "Language";
    in-out property <string> settings-language-zh: "语言";
    in-out property <string> settings-appearance: "Appearance";
    in-out property <string> settings-appearance-zh: "外观";
    in-out property <string> settings-service: "Service";
    in-out property <string> settings-service-zh: "服务";
    in-out property <string> settings-api-base-url: "API Base URL";
    in-out property <string> settings-api-base-url-zh: "API 地址";
    in-out property <string> settings-default-params: "Default Conversion Parameters";
    in-out property <string> settings-default-params-zh: "默认转换参数";
    in-out property <string> settings-default-profile: "Default Profile";
    in-out property <string> settings-default-profile-zh: "默认模板";
    in-out property <string> settings-default-quality: "Default Quality";
    in-out property <string> settings-default-quality-zh: "默认质量";
    in-out property <string> settings-default-output-dir: "Default Output Directory";
    in-out property <string> settings-default-output-dir-zh: "默认输出目录";
    in-out property <string> settings-updates: "Updates";
    in-out property <string> settings-updates-zh: "更新";
    in-out property <string> settings-check-update: "Check Update";
    in-out property <string> settings-check-update-zh: "检查更新";
    in-out property <string> settings-checking-update: "Checking for updates...";
    in-out property <string> settings-checking-update-zh: "正在检查更新...";
    in-out property <string> settings-apply-appearance: "Apply Appearance";
    in-out property <string> settings-apply-appearance-zh: "应用外观";
    in-out property <string> settings-about: "About";
    in-out property <string> settings-about-zh: "关于";
    in-out property <string> settings-product: "Product";
    in-out property <string> settings-product-zh: "产品";
    in-out property <string> settings-version: "Version";
    in-out property <string> settings-version-zh: "版本";

    // === Flutter theme.* ===
    in-out property <string> theme-default: "Default";
    in-out property <string> theme-default-zh: "默认";
    in-out property <string> theme-blue: "Blue";
    in-out property <string> theme-blue-zh: "蓝色";
    in-out property <string> theme-green: "Green";
    in-out property <string> theme-green-zh: "绿色";
    in-out property <string> theme-purple: "Purple";
    in-out property <string> theme-purple-zh: "紫色";
    in-out property <string> theme-orange: "Orange";
    in-out property <string> theme-orange-zh: "橙色";
    in-out property <string> theme-dark: "Dark";
    in-out property <string> theme-dark-zh: "深色";

    // === Flutter status.* ===
    in-out property <string> status-engine-ready: "Conversion engine is ready";
    in-out property <string> status-engine-ready-zh: "转换引擎已就绪";
    in-out property <string> status-engine-error: "Conversion engine failed to initialize";
    in-out property <string> status-engine-error-zh: "转换引擎初始化失败";
    in-out property <string> status-signed-out: "Signed out";
    in-out property <string> status-signed-out-zh: "未登录";
    in-out property <string> status-working: "Working...";
    in-out property <string> status-working-zh: "处理中...";
    in-out property <string> status-sign-in-first: "Sign in before refreshing usage.";
    in-out property <string> status-sign-in-first-zh: "请先登录后再刷新用量。";

    // === Existing (from v2, keep) ===
    in-out property <string> tab-convert: "Convert";
    in-out property <string> tab-convert-zh: "转换";
    in-out property <string> tab-settings: "Settings";
    in-out property <string> tab-settings-zh: "配置";
    in-out property <string> tab-account: "Account";
    in-out property <string> tab-account-zh: "账号";
    in-out property <string> tab-billing: "Plans";
    in-out property <string> tab-billing-zh: "套餐";
    in-out property <string> tab-history: "History";
    in-out property <string> tab-history-zh: "历史";
    in-out property <string> empty-history: "No conversion records yet.";
    in-out property <string> empty-history-zh: "暂无转换记录。";
    in-out property <string> empty-plans: "No plans available.";
    in-out property <string> empty-plans-zh: "暂无套餐。";
    in-out property <string> empty-recharge-records: "No recharge records.";
    in-out property <string> empty-recharge-records-zh: "暂无充值记录。";
    in-out property <string> alert-sign-in-required: "Please sign in to use cloud conversion.";
    in-out property <string> alert-sign-in-required-zh: "请先登录以使用云端转换。";
    in-out property <string> alert-api-not-configured: "API URL not configured. Please set it in Settings.";
    in-out property <string> alert-api-not-configured-zh: "API 地址未配置，请在设置中配置。";
    in-out property <string> alert-quota-exceeded: "Your quota is exhausted. Please recharge to continue.";
    in-out property <string> alert-quota-exceeded-zh: "配额已耗尽，请充值后继续使用。";
    in-out property <string> status-idle: "Idle";
    in-out property <string> status-idle-zh: "空闲";
    in-out property <string> status-pending: "Pending";
    in-out property <string> status-pending-zh: "等待中";
    in-out property <string> status-running: "Running";
    in-out property <string> status-running-zh: "运行中";
    in-out property <string> status-succeeded: "Succeeded";
    in-out property <string> status-succeeded-zh: "成功";
    in-out property <string> status-failed: "Failed";
    in-out property <string> status-failed-zh: "失败";

    // === Toast ===
    in-out property <string> toast-conversion-complete: "Conversion completed successfully.";
    in-out property <string> toast-conversion-complete-zh: "转换成功完成。";
    in-out property <string> toast-conversion-failed: "Conversion failed.";
    in-out property <string> toast-conversion-failed-zh: "转换失败。";
    in-out property <string> toast-sign-in-success: "Signed in successfully.";
    in-out property <string> toast-sign-in-success-zh: "登录成功。";
    in-out property <string> toast-sign-out-success: "Signed out.";
    in-out property <string> toast-sign-out-success-zh: "已退出登录。";
    in-out property <string> toast-settings-saved: "Settings saved.";
    in-out property <string> toast-settings-saved-zh: "设置已保存。";
    in-out property <string> toast-recharge-success: "Recharge successful.";
    in-out property <string> toast-recharge-success-zh: "充值成功。";
    in-out property <string> toast-password-changed: "Password changed successfully.";
    in-out property <string> toast-password-changed-zh: "密码修改成功。";
}
```

---

## 四、页面结构与组件架构

### 4.1 整体布局架构（与 Flutter 完全对齐）

Flutter 的应用布局结构：

```
DocEngineApp
└── _WorkspaceShell (StatefulWidget)
    ├── 未登录 → AuthWindow (Tab: 登录/注册, maxWidth=420px)
    └── 已登录 → Scaffold
                   ├── Sidebar (248px, 左侧)
                   │   ├── Logo + App Name
                   │   └── NavItem (5个: 账号/充值/转换/转换记录/充值记录)
                   └── Column
                       ├── TopBar (64px)
                       │   ├── Logo + App Name + Platform
                       │   ├── ThemeDropdown
                       │   ├── LocaleDropdown
                       │   └── AccountAvatarButton (CircleAvatar + PopupMenu)
                       │       ├── [disabled] 邮箱 + 套餐
                       │       ├── 查看详情 → ProfileDialog
                       │       ├── 修改密码 → ChangePasswordDialog
                       │       └── 退出登录
                       └── _NavContent
                           ├── 账号 → _AccountPanel (账号总览 + 快捷操作)
                           ├── 充值 → _RechargePanel (按次/按日期充值按钮)
                           ├── 转换 → _ConvertPanel (上传ZIP + 主文件 + 转换)
                           ├── 转换记录 → ConvertRecordsPanel (DataTable)
                           └── 充值记录 → RechargeRecordsPanel (DataTable)
```

Slint 端的等价布局结构：

```
AppWindow (主窗口, 1170x780px)
├── [未登录时] AuthPage (独立登录/注册, 居中卡片 max-width=420px)
└── [已登录时] WorkspaceShell
    ├── Sidebar (248px宽, 左侧)
    │   ├── Logo + App Name + Subtitle
    │   └── NavItem x5 (账号/充值/转换/转换记录/充值记录)
    └── Column
        ├── TopBar (64px高)
        │   ├── Logo + App Name + Platform
        │   ├── ThemeDropdown
        │   ├── LocaleDropdown
        │   └── AccountAvatarButton (CircleAvatar + PopupMenu)
        │       ├── [disabled] 邮箱 + 套餐
        │       ├── view-profile → ProfileDialog
        │       ├── change-password → ChangePasswordDialog
        │       └── logout
        └── PageContent (根据 selected-nav 渲染)
            ├── 账号 → AccountPanel
            ├── 充值 → RechargePanel
            ├── 转换 → ConvertPanel
            ├── 转换记录 → ConvertRecordsPanel
            └── 充值记录 → RechargeRecordsPanel
```

### 4.2 `main.slint` 重构（精简为布局骨架）

目标：从 517 行精简至 ~180 行，作为纯粹的布局骨架。

```slint
// crates/desktop-slint/src/ui/main.slint

import { DesignTokens } from "tokens.slint";
import { DarkDesignTokens } from "tokens.slint";
import { I18n } from "i18n-strings.slint";
import { MotionTokens } from "motion.slint";
import { AuthPage } from "pages/auth.slint";
import { WorkspaceShell } from "pages/workspace-shell.slint";
import { TabWidget } from "std-widgets.slint";

export component MainWindow inherits Window {
    title: root.is-signed-in
        ? I18n.app-title + " " + root.app-version + " - " + root.account-display-name
        : I18n.app-title + " " + root.app-version;
    icon: @image-url("assets/app_icon.jpg");
    preferred-width: 1170px;
    preferred-height: 780px;
    background: DesignTokens.token-window-bg;

    // === 全局 Token 实例 ===
    DesignTokens { }
    DarkDesignTokens { }
    MotionTokens { }
    I18n { locale: root.ui-locale; }

    // === 核心状态属性 ===
    in-out property <bool> is-signed-in: false;
    in-out property <string> selected-nav: "account"; // account | recharge | convert | convert-records | recharge-records
    in-out property <string> ui-locale: "en";
    in-out property <string> ui-theme: "default";
    in-out property <string> app-version: "";
    in-out property <string> account-display-name: "Guest";
    in-out property <string> account-email: "";
    in-out property <string> account-tier: "free";
    in-out property <string> account-created-at: "";
    in-out property <int> quota-remaining: 0;
    in-out property <int> quota-total: 0;
    in-out property <string> api-base-url: "";

    // === 转换状态 ===
    in-out property <bool> is-converting: false;
    in-out property <float> conversion-progress: 0.0;
    in-out property <string> compatibility-score: "--";
    in-out property <float> compatibility-progress: 0.0;
    in-out property <string> quality-status: "--";
    in-out property <float> quality-progress: 0.0;
    in-out property <string> profile-confidence: "--";
    in-out property <float> profile-confidence-progress: 0.0;
    in-out property <string> detected-profile: "auto";
    in-out property <string> quality-level: "standard";

    // === 充值状态 ===
    in-out property <[RechargeRecord]> recharge-records: [];
    in-out property <[ConversionRecord]> conversion-records: [];
    in-out property <bool> is-recharging: false;
    in-out property <string> recharge-status: "";

    // === Toast 状态 ===
    in-out property <string> toast-message: "";
    in-out property <string> toast-level: "info"; // success | error | warning | info
    in-out property <bool> toast-visible: false;

    // === 业务 Callback ===
    callback auth-login(string, string, string);
    callback auth-register(string, string, string);
    callback logout();
    callback view-profile-clicked();
    callback change-password-clicked(string, string, string);
    callback recharge-clicked(string, string);
    callback cloud-convert-clicked(string, string, string, string, string);
    callback pick-zip-clicked();
    callback query-conversion-records();
    callback query-recharge-records();
    callback theme-changed(string);
    callback locale-changed(string);
    callback toast-dismissed();

    // === 布局 ===
    if !root.is-signed-in: AuthPage {
        api-base-url: root.api-base-url;
        auth-login(email, password, base-url) => { root.auth-login(email, password, base-url); }
        auth-register(email, password, base-url) => { root.auth-register(email, password, base-url); }
        theme-changed(t) => { root.theme-changed(t); }
        locale-changed(l) => { root.locale-changed(l); }
    }

    if root.is-signed-in: WorkspaceShell {
        selected-nav: root.selected-nav;
        is-signed-in: root.is-signed-in;
        account-display-name: root.account-display-name;
        account-email: root.account-email;
        account-tier: root.account-tier;
        account-created-at: root.account-created-at;
        quota-remaining: root.quota-remaining;
        quota-total: root.quota-total;
        api-base-url: root.api-base-url;
        ui-locale: root.ui-locale;
        ui-theme: root.ui-theme;
        is-converting: root.is-converting;
        conversion-progress: root.conversion-progress;
        compatibility-score: root.compatibility-score;
        compatibility-progress: root.compatibility-progress;
        quality-status: root.quality-status;
        quality-progress: root.quality-progress;
        profile-confidence: root.profile-confidence;
        profile-confidence-progress: root.profile-confidence-progress;
        detected-profile: root.detected-profile;
        quality-level: root.quality-level;
        recharge-records: root.recharge-records;
        conversion-records: root.conversion-records;
        is-recharging: root.is-recharging;
        recharge-status: root.recharge-status;
        toast-message: root.toast-message;
        toast-level: root.toast-level;
        toast-visible: root.toast-visible;

        nav-changed(section) => { root.selected-nav = section; }
        logout => { root.logout(); }
        view-profile-clicked => { root.view-profile-clicked(); }
        change-password-clicked(old, new1, new2) => { root.change-password-clicked(old, new1, new2); }
        recharge-clicked(type, pkg) => { root.recharge-clicked(type, pkg); }
        cloud-convert-clicked(zip, main-tex, profile, quality, output) => { root.cloud-convert-clicked(zip, main-tex, profile, quality, output); }
        pick-zip-clicked => { root.pick-zip-clicked(); }
        query-conversion-records => { root.query-conversion-records(); }
        query-recharge-records => { root.query-recharge-records(); }
        theme-changed(t) => { root.theme-changed(t); }
        locale-changed(l) => { root.locale-changed(l); }
        toast-dismissed => { root.toast-dismissed(); }
    }
}
```

---

## 五、新增组件库详细设计

### 5.1 组件总清单（18 个新组件）

| # | 组件 | 文件 | 类型 | Flutter 对应 |
|---|------|------|------|------------|
| 1 | `AuthPage` | `pages/auth.slint` | 页面 | `AuthWindow` |
| 2 | `WorkspaceShell` | `pages/workspace-shell.slint` | 页面 | `_WorkspaceShell` |
| 3 | `Sidebar` | `pages/sidebar.slint` | 布局 | `_Sidebar` |
| 4 | `SidebarNavItem` | `components/sidebar-nav-item.slint` | 组件 | `_NavItem` |
| 5 | `TopBar` | `pages/topbar.slint` | 布局 | `_TopBar` |
| 6 | `ThemeDropdown` | `components/theme-dropdown.slint` | 组件 | `_ThemeDropdown` |
| 7 | `LocaleDropdown` | `components/locale-dropdown.slint` | 组件 | `_LocaleDropdown` |
| 8 | `AccountAvatarButton` | `components/account-avatar-button.slint` | 组件 | `_AccountAvatarButton` |
| 9 | `ProfileDialog` | `components/profile-dialog.slint` | 组件 | `_ProfileDialog` |
| 10 | `ChangePasswordDialog` | `components/change-password-dialog.slint` | 组件 | `_ChangePasswordDialog` |
| 11 | `AccountPanel` | `pages/account-panel.slint` | 页面 | `_AccountPanel` |
| 12 | `RechargePanel` | `pages/recharge-panel.slint` | 页面 | `_RechargePanel` |
| 13 | `ConvertPanel` | `pages/convert-panel.slint` | 页面 | `_ConvertPanel` |
| 14 | `ConvertRecordsPanel` | `pages/convert-records.slint` | 页面 | `ConvertRecordsPanel` |
| 15 | `RechargeRecordsPanel` | `pages/recharge-records.slint` | 页面 | `RechargeRecordsPanel` |
| 16 | `RecordsTable` | `components/records-table.slint` | 组件 | `_RecordsTable` |
| 17 | `StatusChip` | `components/status-chip.slint` | 组件 | `_StatusChip` |
| 18 | `ToastNotify` | `components/toast-notify.slint` | 组件 | Toast snackbar |

### 5.2 `AuthPage` — 独立登录/注册窗口

与 Flutter `AuthWindow` 完全对齐，单卡片居中，最大宽度 420px，Tab 切换登录/注册。

```slint
// crates/desktop-slint/src/ui/pages/auth.slint

import { Button, VerticalBox, HorizontalBox, LineEdit, TabWidget } from "std-widgets.slint";
import { DesignTokens } from "../tokens.slint";
import { I18n } from "../i18n-strings.slint";

export component AuthPage inherits Rectangle {
    in property <string> api-base-url: "";
    callback auth-login(string, string, string);
    callback auth-register(string, string, string);
    callback theme-changed(string);
    callback locale-changed(string);

    background: DesignTokens.token-window-bg;

    VerticalBox {
        alignment: center;

        // 居中卡片 max-width=420px
        Rectangle {
            background: DesignTokens.token-surface-1;
            border-color: DesignTokens.token-border;
            border-radius: DesignTokens.token-radius-lg;
            border-width: 1px;
            max-width: 420px;
            min-width: 380px;
            padding: DesignTokens.token-space-8;

            VerticalBox {
                spacing: DesignTokens.token-space-6;

                // Logo + Title
                HorizontalBox {
                    spacing: DesignTokens.token-space-4;
                    alignment: center;

                    Image {
                        source: @image-url("assets/app_icon.jpg");
                        width: 48px;
                        height: 48px;
                        image-fit: cover;
                        border-radius: DesignTokens.token-radius-md;
                    }

                    VerticalBox {
                        spacing: DesignTokens.token-space-1;
                        Text {
                            text: I18n.app-title;
                            font-size: DesignTokens.token-font-size-xl;
                            font-weight: DesignTokens.token-font-weight-bold;
                            color: DesignTokens.token-text-primary;
                        }
                        Text {
                            text: ui-locale == "en" ? I18n.app-subtitle : I18n.app-subtitle-zh;
                            font-size: DesignTokens.token-font-size-base;
                            color: DesignTokens.token-text-secondary;
                        }
                    }
                }

                // Tab 栏
                TabWidget {
                    // Tab 0: 登录
                    Tab {
                        title: ui-locale == "en" ? I18n.auth-login-tab : I18n.auth-login-tab-zh;
                        _LoginForm { }
                    }
                    // Tab 1: 注册
                    Tab {
                        title: ui-locale == "en" ? I18n.auth-register-tab : I18n.auth-register-tab-zh;
                        _RegisterForm { }
                    }
                }
            }
        }
    }
}

component _LoginForm inherits VerticalBox {
    spacing: DesignTokens.token-space-4;

    LineEdit {
        placeholder-text: ui-locale == "en" ? I18n.account-api-base-url : I18n.account-api-base-url-zh;
        text <=> parent.api-base-url;
    }
    LineEdit {
        placeholder-text: ui-locale == "en" ? I18n.account-email : I18n.account-email-zh;
        text <=> _email;
    }
    LineEdit {
        placeholder-text: ui-locale == "en" ? I18n.account-password : I18n.account-password-zh;
        input-type: password;
        text <=> _password;
    }

    Button {
        text: ui-locale == "en" ? I18n.common-login : I18n.common-login-zh;
        primary: true;
        clicked => { root.auth-login(_email, _password, api-base-url); }
    }
}

component _RegisterForm inherits VerticalBox {
    spacing: DesignTokens.token-space-4;

    LineEdit {
        placeholder-text: ui-locale == "en" ? I18n.account-api-base-url : I18n.account-api-base-url-zh;
        text <=> parent.api-base-url;
    }
    LineEdit {
        placeholder-text: ui-locale == "en" ? I18n.account-email : I18n.account-email-zh;
        text <=> _email;
    }
    LineEdit {
        placeholder-text: ui-locale == "en" ? I18n.account-password : I18n.account-password-zh;
        input-type: password;
        text <=> _password;
    }
    LineEdit {
        placeholder-text: ui-locale == "en" ? I18n.account-confirm-password : I18n.account-confirm-password-zh;
        input-type: password;
        text <=> _confirm-password;
    }

    Button {
        text: ui-locale == "en" ? I18n.common-register : I18n.common-register-zh;
        primary: true;
        clicked => { root.auth-register(_email, _password, api-base-url); }
    }
}
```

### 5.3 `WorkspaceShell` — 已登录工作台布局

```slint
// crates/desktop-slint/src/ui/pages/workspace-shell.slint

import { Sidebar } from "sidebar.slint";
import { TopBar } from "topbar.slint";
import { AccountPanel } from "account-panel.slint";
import { RechargePanel } from "recharge-panel.slint";
import { ConvertPanel } from "convert-panel.slint";
import { ConvertRecordsPanel } from "convert-records.slint";
import { RechargeRecordsPanel } from "recharge-records.slint";
import { ToastNotify } from "../components/toast-notify.slint";
import { DesignTokens } from "../tokens.slint";

export component WorkspaceShell inherits Rectangle {
    in-out property <string> selected-nav: "account";
    in-out property <bool> is-signed-in: false;
    // ... (所有属性透传)

    callback nav-changed(string);
    callback logout();
    callback view-profile-clicked();
    callback change-password-clicked(string, string, string);
    callback recharge-clicked(string, string);
    callback cloud-convert-clicked(string, string, string, string, string);
    callback pick-zip-clicked();
    callback query-conversion-records();
    callback query-recharge-records();
    callback theme-changed(string);
    callback locale-changed(string);
    callback toast-dismissed();

    background: DesignTokens.token-window-bg;

    Row {
        spacing: 0px;

        // 左侧边栏 248px
        Sidebar {
            selected-nav: root.selected-nav;
            is-signed-in: root.is-signed-in;
            account-display-name: root.account-display-name;
            nav-changed(section) => { root.nav-changed(section); }
        }

        // 右侧内容区
        VerticalBox {
            spacing: 0px;

            // 顶栏 64px
            TopBar {
                account-display-name: root.account-display-name;
                account-email: root.account-email;
                account-tier: root.account-tier;
                ui-locale: root.ui-locale;
                ui-theme: root.ui-theme;
                logout => { root.logout(); }
                view-profile-clicked => { root.view-profile-clicked(); }
                change-password-clicked(old, new1, new2) => { root.change-password-clicked(old, new1, new2); }
                theme-changed(t) => { root.theme-changed(t); }
                locale-changed(l) => { root.locale-changed(l); }
            }

            // 页面内容
            Rectangle {
                background: DesignTokens.token-window-bg;

                if root.selected-nav == "account": AccountPanel { /* ... */ }
                if root.selected-nav == "recharge": RechargePanel { /* ... */ }
                if root.selected-nav == "convert": ConvertPanel { /* ... */ }
                if root.selected-nav == "convert-records": ConvertRecordsPanel { /* ... */ }
                if root.selected-nav == "recharge-records": RechargeRecordsPanel { /* ... */ }
            }
        }
    }

    // Toast 通知（浮窗）
    ToastNotify {
        visible: root.toast-visible;
        message: root.toast-message;
        level: root.toast-level;
        dismissed => { root.toast-dismissed(); }
    }
}
```

### 5.4 `Sidebar` — 左侧导航（248px，与 Flutter 完全对齐）

```slint
// crates/desktop-slint/src/ui/pages/sidebar.slint

import { VerticalBox } from "std-widgets.slint";
import { DesignTokens } from "../tokens.slint";
import { I18n } from "../i18n-strings.slint";
import { SidebarNavItem } from "../components/sidebar-nav-item.slint";

export component Sidebar inherits Rectangle {
    in property <string> selected-nav: "account";
    in property <bool> is-signed-in: false;
    in property <string> account-display-name: "Guest";
    callback nav-changed(string);

    background: DesignTokens.token-surface-1;
    border-color: DesignTokens.token-border;
    border-width: 0px;
    width: DesignTokens.token-sidebar-width; // 248px

    VerticalBox {
        padding: DesignTokens.token-space-6; // 24px
        spacing: DesignTokens.token-space-2;

        // Logo + App Name
        HorizontalBox {
            spacing: DesignTokens.token-space-3;

            Image {
                source: @image-url("assets/app_icon.jpg");
                width: 44px;
                height: 44px;
                image-fit: cover;
                border-radius: DesignTokens.token-radius-md;
            }

            VerticalBox {
                spacing: 2px;
                Text {
                    text: I18n.app-title;
                    font-size: DesignTokens.token-font-size-lg;
                    font-weight: DesignTokens.token-font-weight-bold;
                    color: DesignTokens.token-text-primary;
                }
                Text {
                    text: ui-locale == "en" ? I18n.app-subtitle : I18n.app-subtitle-zh;
                    font-size: DesignTokens.token-font-size-base;
                    color: DesignTokens.token-text-secondary;
                    wrap: word-wrap;
                }
            }
        }

        // 导航项之间间距
        Rectangle { height: DesignTokens.token-space-6; }

        // 5 个导航项（与 Flutter nav.* 对齐）
        SidebarNavItem {
            nav-id: "account";
            icon: "person_outline";
            label: ui-locale == "en" ? I18n.nav-account : I18n.nav-account-zh;
            selected: root.selected-nav == "account";
            clicked => { root.nav-changed("account"); }
        }
        SidebarNavItem {
            nav-id: "recharge";
            icon: "payments_outlined";
            label: ui-locale == "en" ? I18n.nav-recharge : I18n.nav-recharge-zh;
            selected: root.selected-nav == "recharge";
            clicked => { root.nav-changed("recharge"); }
        }
        SidebarNavItem {
            nav-id: "convert";
            icon: "sync_alt";
            label: ui-locale == "en" ? I18n.nav-convert : I18n.nav-convert-zh;
            selected: root.selected-nav == "convert";
            clicked => { root.nav-changed("convert"); }
        }
        SidebarNavItem {
            nav-id: "convert-records";
            icon: "history";
            label: ui-locale == "en" ? I18n.nav-convert-records : I18n.nav-convert-records-zh;
            selected: root.selected-nav == "convert-records";
            clicked => { root.nav-changed("convert-records"); }
        }
        SidebarNavItem {
            nav-id: "recharge-records";
            icon: "receipt_long";
            label: ui-locale == "en" ? I18n.nav-recharge-records : I18n.nav-recharge-records-zh;
            selected: root.selected-nav == "recharge-records";
            clicked => { root.nav-changed("recharge-records"); }
        }
    }
}
```

### 5.5 `SidebarNavItem` — 导航条目（与 Flutter `_NavItem` 对齐）

```slint
// crates/desktop-slint/src/ui/components/sidebar-nav-item.slint

import { DesignTokens } from "../tokens.slint";

export component SidebarNavItem inherits Rectangle {
    in property <string> nav-id: "";
    in property <string> icon: "";    // icon name (emoji fallback)
    in property <string> label: "";
    in property <bool> selected: false;
    callback clicked();

    min-height: 36px;
    border-radius: DesignTokens.token-radius-md;
    padding: DesignTokens.token-space-2 DesignTokens.token-space-3;
    cursor: pointer;

    background: root.selected ? DesignTokens.token-accent-subtle : transparent;

    states [
        hovered when !root.selected && root.pressed: {
            background: DesignTokens.token-surface-3;
        }
    ]

    HorizontalBox {
        spacing: DesignTokens.token-space-3;
        alignment: center;

        // Icon (使用 Text 渲染 emoji icon)
        Text {
            text: _icon-emoji(root.icon);
            font-size: DesignTokens.token-icon-size-lg;
            width: 20px;
            horizontal-alignment: center;
        }

        Text {
            text: root.label;
            font-size: DesignTokens.token-font-size-md;
            font-weight: DesignTokens.token-font-weight-semibold;
            color: root.selected
                ? DesignTokens.token-accent
                : DesignTokens.token-text-primary;
        }
    }

    // 点击区域
    TouchArea {
        clicked => { root.clicked(); }
    }
}

// Icon emoji 映射
function _icon-emoji(name: string) -> string {
    if (name == "person_outline") return "👤";
    if (name == "payments_outlined") return "💰";
    if (name == "sync_alt") return "🔄";
    if (name == "history") return "📋";
    if (name == "receipt_long") return "🧾";
    return "●";
}
```

### 5.6 `TopBar` — 顶栏（64px，与 Flutter `_TopBar` 对齐）

```slint
// crates/desktop-slint/src/ui/pages/topbar.slint

import { HorizontalBox, VerticalBox, Rectangle } from "std-widgets.slint";
import { DesignTokens } from "../tokens.slint";
import { I18n } from "../i18n-strings.slint";
import { ThemeDropdown } from "../components/theme-dropdown.slint";
import { LocaleDropdown } from "../components/locale-dropdown.slint";
import { AccountAvatarButton } from "../components/account-avatar-button.slint";

export component TopBar inherits Rectangle {
    in property <string> account-display-name: "Guest";
    in property <string> account-email: "";
    in property <string> account-tier: "free";
    in property <string> ui-locale: "en";
    in property <string> ui-theme: "default";

    callback logout();
    callback view-profile-clicked();
    callback change-password-clicked(string, string, string);
    callback theme-changed(string);
    callback locale-changed(string);

    background: DesignTokens.token-surface-1;
    border-color: DesignTokens.token-border;
    border-width: 0px;
    height: DesignTokens.token-header-height; // 64px

    HorizontalBox {
        padding: DesignTokens.token-space-4 DesignTokens.token-space-6;
        spacing: DesignTokens.token-space-4;

        // Logo + App Name + Platform
        HorizontalBox {
            spacing: DesignTokens.token-space-3;

            Image {
                source: @image-url("assets/app_icon.jpg");
                width: 36px;
                height: 36px;
                image-fit: cover;
                border-radius: DesignTokens.token-radius-md;
            }

            VerticalBox {
                spacing: 2px;
                Text {
                    text: I18n.app-title;
                    font-size: DesignTokens.token-font-size-lg;
                    font-weight: DesignTokens.token-font-weight-bold;
                    color: DesignTokens.token-text-primary;
                }
                Text {
                    text: (ui-locale == "en" ? I18n.topbar-platform : I18n.topbar-platform-zh) + ": Desktop";
                    font-size: DesignTokens.token-font-size-base;
                    color: DesignTokens.token-text-secondary;
                }
            }
        }

        // Spacer
        Rectangle { horizontal-stretch: 1; }

        // 主题下拉
        ThemeDropdown {
            value: root.ui-theme;
            theme-changed(t) => { root.theme-changed(t); }
        }

        // 语言下拉
        LocaleDropdown {
            value: root.ui-locale;
            locale-changed(l) => { root.locale-changed(l); }
        }

        // 账号头像按钮
        AccountAvatarButton {
            account-display-name: root.account-display-name;
            account-email: root.account-email;
            account-tier: root.account-tier;
            logout => { root.logout(); }
            view-profile-clicked => { root.view-profile-clicked(); }
            change-password-clicked(old, new1, new2) => { root.change-password-clicked(old, new1, new2); }
        }
    }
}
```

### 5.7 `AccountAvatarButton` — 账号头像按钮（与 Flutter `_AccountAvatarButton` 对齐）

```slint
// crates/desktop-slint/src/ui/components/account-avatar-button.slint

import { HorizontalBox, VerticalBox, Rectangle, TextEdit, Button, Dialog } from "std-widgets.slint";
import { DesignTokens } from "../tokens.slint";
import { I18n } from "../i18n-strings.slint";
import { ProfileDialog } from "profile-dialog.slint";
import { ChangePasswordDialog } from "change-password-dialog.slint";

export component AccountAvatarButton inherits Rectangle {
    in property <string> account-display-name: "Guest";
    in property <string> account-email: "";
    in property <string> account-tier: "free";

    callback logout();
    callback view-profile-clicked();
    callback change-password-clicked(string, string, string);

    cursor: pointer;

    HorizontalBox {
        spacing: DesignTokens.token-space-2;
        padding: DesignTokens.token-space-2 DesignTokens.token-space-3;
        border-radius: DesignTokens.token-radius-full;
        background: DesignTokens.token-surface-3;

        // CircleAvatar (用圆形 Rectangle 模拟)
        Rectangle {
            width: 32px;
            height: 32px;
            border-radius: 16px;
            background: DesignTokens.token-accent;

            Text {
                text: _initial(root.account-display-name);
                font-size: DesignTokens.token-font-size-md;
                font-weight: DesignTokens.token-font-weight-bold;
                color: DesignTokens.token-text-inverse;
                horizontal-alignment: center;
                vertical-alignment: center;
            }
        }

        // 向下箭头
        Text {
            text: "▾";
            font-size: DesignTokens.token-font-size-md;
            color: DesignTokens.token-text-muted;
            vertical-alignment: center;
        }
    }

    // 点击弹出菜单（使用 TouchArea + PopupMenu 模式）
    TouchArea {
        clicked => { root._show-menu = !root._show-menu; }
    }

    // PopupMenu（简化为内联）
    if root._show-menu: Rectangle {
        x: parent.width - self.width;
        y: parent.height + 4px;
        width: 240px;
        background: DesignTokens.token-surface-1;
        border-color: DesignTokens.token-border;
        border-radius: DesignTokens.token-radius-lg;
        border-width: 1px;
        drop-shadow-blur-radius: 8px;
        drop-shadow-color: #0000001A;

        VerticalBox {
            padding: DesignTokens.token-space-2;
            spacing: DesignTokens.token-space-1;

            // 禁用项：邮箱 + 套餐
            VerticalBox {
                spacing: DesignTokens.token-space-1;
                padding: DesignTokens.token-space-3;
                background: DesignTokens.token-surface-3;
                border-radius: DesignTokens.token-radius-md;
                Text {
                    text: root.account-email;
                    font-size: DesignTokens.token-font-size-md;
                    font-weight: DesignTokens.token-font-weight-semibold;
                    color: DesignTokens.token-text-primary;
                }
                Text {
                    text: (ui-locale == "en" ? I18n.account-current-plan : I18n.account-current-plan-zh) + ": " + root.account-tier;
                    font-size: DesignTokens.token-font-size-base;
                    color: DesignTokens.token-text-muted;
                }
            }

            // 查看详情
            _MenuItem {
                icon: "👤";
                label: ui-locale == "en" ? I18n.account-view-profile : I18n.account-view-profile-zh;
                clicked => {
                    root.view-profile-clicked();
                    root._show-menu = false;
                }
            }

            // 修改密码
            _MenuItem {
                icon: "🔒";
                label: ui-locale == "en" ? I18n.account-change-password : I18n.account-change-password-zh;
                clicked => {
                    root._show-change-password = true;
                    root._show-menu = false;
                }
            }

            // 分隔线
            Rectangle { height: 1px; background: DesignTokens.token-border; }

            // 退出登录
            _MenuItem {
                icon: "🚪";
                label: ui-locale == "en" ? I18n.account-logout : I18n.account-logout-zh;
                text-color: DesignTokens.token-danger;
                clicked => {
                    root.logout();
                    root._show-menu = false;
                }
            }
        }
    }

    // 修改密码弹窗
    if root._show-change-password: ChangePasswordDialog {
        change-password-clicked(old, new1, new2) => {
            root.change-password-clicked(old, new1, new2);
            root._show-change-password = false;
        }
        cancelled => { root._show-change-password = false; }
    }

    // 查看详情弹窗
    if root._show-profile: ProfileDialog {
        account-email: root.account-email;
        account-tier: root.account-tier;
        account-display-name: root.account-display-name;
        account-created-at: root.account-created-at;
        closed => { root._show-profile = false; }
    }
}

component _MenuItem inherits Rectangle {
    in property <string> icon: "";
    in property <string> label: "";
    in property <color> text-color: DesignTokens.token-text-primary;
    callback clicked();

    min-height: 36px;
    border-radius: DesignTokens.token-radius-md;
    padding: DesignTokens.token-space-2 DesignTokens.token-space-3;
    cursor: pointer;

    HorizontalBox {
        spacing: DesignTokens.token-space-3;
        alignment: center;
        Text { text: root.icon; font-size: DesignTokens.token-font-size-base; }
        Text {
            text: root.label;
            font-size: DesignTokens.token-font-size-md;
            color: root.text-color;
        }
    }

    TouchArea { clicked => { root.clicked(); } }
    states [
        hovered: { background: DesignTokens.token-surface-3; }
    ]
}

function _initial(name: string) -> string {
    if (name == "" || name == "Guest") return "?";
    return name[0].to_uppercase();
}
```

### 5.8 `ProfileDialog` — 账号详情弹窗

```slint
// crates/desktop-slint/src/ui/components/profile-dialog.slint

import { VerticalBox, HorizontalBox, Button, Rectangle } from "std-widgets.slint";
import { DesignTokens } from "../tokens.slint";
import { I18n } from "../i18n-strings.slint";

export component ProfileDialog inherits Rectangle {
    in property <string> account-email: "";
    in property <string> account-tier: "free";
    in property <string> account-display-name: "Guest";
    in property <string> account-created-at: "";
    callback closed();

    background: DesignTokens.token-surface-overlay;
    width: parent.width;
    height: parent.height;

    // 弹窗主体
    Rectangle {
        background: DesignTokens.token-surface-1;
        border-color: DesignTokens.token-border;
        border-radius: DesignTokens.token-radius-xl;
        border-width: 1px;
        width: 400px;
        height: 380px;
        x: (parent.width - 400px) / 2;
        y: (parent.height - 380px) / 2;

        VerticalBox {
            padding: DesignTokens.token-space-8;
            spacing: DesignTokens.token-space-4;

            // 标题
            Text {
                text: ui-locale == "en" ? I18n.account-profile-title : I18n.account-profile-title-zh;
                font-size: DesignTokens.token-font-size-xl;
                font-weight: DesignTokens.token-font-weight-bold;
                color: DesignTokens.token-text-primary;
            }

            // 圆形头像 (80px)
            Rectangle {
                width: 80px;
                height: 80px;
                border-radius: 40px;
                background: DesignTokens.token-accent;
                horizontal-alignment: center;

                Text {
                    text: _initial(root.account-display-name);
                    font-size: 36px;
                    font-weight: DesignTokens.token-font-weight-bold;
                    color: DesignTokens.token-text-inverse;
                    horizontal-alignment: center;
                    vertical-alignment: center;
                }
            }

            // 信息行
            VerticalBox {
                spacing: DesignTokens.token-space-2;
                _InfoRow {
                    label: ui-locale == "en" ? I18n.account-email : I18n.account-email-zh;
                    value: root.account-email;
                }
                _InfoRow {
                    label: ui-locale == "en" ? I18n.account-current-plan : I18n.account-current-plan-zh;
                    value: root.account-tier;
                }
                if root.account-display-name != "" && root.account-display-name != "Guest": _InfoRow {
                    label: ui-locale == "en" ? I18n.account-display-name : I18n.account-display-name-zh;
                    value: root.account-display-name;
                }
            }

            // 关闭按钮
            Button {
                text: ui-locale == "en" ? I18n.common-close : I18n.common-close-zh;
                primary: true;
                clicked => { root.closed(); }
            }
        }
    }
}

component _InfoRow inherits VerticalBox {
    in property <string> label: "";
    in property <string> value: "";
    spacing: DesignTokens.token-space-1;
    HorizontalBox {
        spacing: DesignTokens.token-space-4;
        Text {
            text: root.label;
            font-size: DesignTokens.token-font-size-base;
            color: DesignTokens.token-text-muted;
            width: 120px;
        }
        Text {
            text: root.value;
            font-size: DesignTokens.token-font-size-md;
            color: DesignTokens.token-text-primary;
        }
    }
}
```

### 5.9 `ChangePasswordDialog` — 修改密码弹窗

```slint
// crates/desktop-slint/src/ui/components/change-password-dialog.slint

import { VerticalBox, Button, Rectangle, LineEdit, TextEdit } from "std-widgets.slint";
import { DesignTokens } from "../tokens.slint";
import { I18n } from "../i18n-strings.slint";

export component ChangePasswordDialog inherits Rectangle {
    in property <string> error-message: "";
    in property <string> success-message: "";
    callback change-password-clicked(string, string, string);
    callback cancelled();

    background: DesignTokens.token-surface-overlay;
    width: parent.width;
    height: parent.height;

    Rectangle {
        background: DesignTokens.token-surface-1;
        border-color: DesignTokens.token-border;
        border-radius: DesignTokens.token-radius-xl;
        border-width: 1px;
        width: 400px;
        height: 480px;
        x: (parent.width - 400px) / 2;
        y: (parent.height - 480px) / 2;

        VerticalBox {
            padding: DesignTokens.token-space-8;
            spacing: DesignTokens.token-space-4;

            Text {
                text: ui-locale == "en" ? I18n.account-change-password-title : I18n.account-change-password-title-zh;
                font-size: DesignTokens.token-font-size-xl;
                font-weight: DesignTokens.token-font-weight-bold;
                color: DesignTokens.token-text-primary;
            }

            LineEdit {
                placeholder-text: ui-locale == "en" ? I18n.account-old-password : I18n.account-old-password-zh;
                input-type: password;
                text <=> _old-pwd;
            }
            LineEdit {
                placeholder-text: ui-locale == "en" ? I18n.account-new-password : I18n.account-new-password-zh;
                input-type: password;
                text <=> _new-pwd-1;
            }
            LineEdit {
                placeholder-text: ui-locale == "en" ? I18n.account-confirm-new-password : I18n.account-confirm-new-password-zh;
                input-type: password;
                text <=> _new-pwd-2;
            }

            // 错误提示
            if root.error-message != "": Text {
                text: root.error-message;
                font-size: DesignTokens.token-font-size-base;
                color: DesignTokens.token-danger;
                wrap: word-wrap;
            }

            // 成功提示
            if root.success-message != "": Text {
                text: root.success-message;
                font-size: DesignTokens.token-font-size-base;
                color: DesignTokens.token-success;
            }

            HorizontalBox {
                spacing: DesignTokens.token-space-3;
                Button {
                    text: ui-locale == "en" ? I18n.common-cancel : I18n.common-cancel-zh;
                    clicked => { root.cancelled(); }
                }
                Button {
                    text: ui-locale == "en" ? I18n.common-confirm : I18n.common-confirm-zh;
                    primary: true;
                    clicked => {
                        if _new-pwd-1 != _new-pwd-2 {
                            root.error-message = (ui-locale == "en" ? I18n.account-password-mismatch : I18n.account-password-mismatch-zh);
                            return;
                        }
                        if _new-pwd-1.length < 6 {
                            root.error-message = (ui-locale == "en" ? I18n.account-password-too-short : I18n.account-password-too-short-zh);
                            return;
                        }
                        root.change-password-clicked(_old-pwd, _new-pwd-1, _new-pwd-2);
                    }
                }
            }
        }
    }
}
```

### 5.10 `ConvertRecordsPanel` — 转换记录页面（DataTable）

```slint
// crates/desktop-slint/src/ui/pages/convert-records.slint

import { VerticalBox, HorizontalBox, Button, Rectangle, ListView } from "std-widgets.slint";
import { DesignTokens } from "../tokens.slint";
import { I18n } from "../i18n-strings.slint";
import { StatusChip } from "../components/status-chip.slint";
import { EmptyState } from "../components/empty-state.slint";
import { LoadingIndicator } from "../components/loading-indicator.slint";

export component ConvertRecordsPanel inherits Rectangle {
    in property <[ConversionRecord]> records: [];
    in property <bool> is-busy: false;
    in property <string> error-message: "";
    callback query-records();
    callback refresh();

    background: DesignTokens.token-window-bg;

    VerticalBox {
        padding: DesignTokens.token-space-8;
        spacing: DesignTokens.token-space-6;

        // 标题栏
        HorizontalBox {
            spacing: DesignTokens.token-space-4;
            VerticalBox {
                spacing: DesignTokens.token-space-1;
                Text {
                    text: ui-locale == "en" ? I18n.nav-convert-records : I18n.nav-convert-records-zh;
                    font-size: DesignTokens.token-font-size-xl;
                    font-weight: DesignTokens.token-font-weight-bold;
                    color: DesignTokens.token-text-primary;
                }
                Text {
                    text: ui-locale == "en" ? I18n.convert-records : I18n.convert-records-zh;
                    font-size: DesignTokens.token-font-size-base;
                    color: DesignTokens.token-text-secondary;
                }
            }

            Rectangle { horizontal-stretch: 1; }

            Button {
                text: ui-locale == "en" ? I18n.common-refresh : I18n.common-refresh-zh;
                enabled: !root.is-busy;
                clicked => { root.refresh(); }
            }
        }

        // 加载态
        if root.is-busy: LoadingIndicator {
            label: ui-locale == "en" ? I18n.common-loading : I18n.common-loading-zh;
        }

        // 错误态
        if root.error-message != "" && !root.is-busy: AlertBanner {
            message: root.error-message;
            level: "danger";
        }

        // 空态
        if root.records.length == 0 && !root.is-busy && root.error-message == "": EmptyState {
            title: ui-locale == "en" ? I18n.empty-no-data : I18n.empty-no-data-zh;
            message: ui-locale == "en" ? I18n.empty-no-data : I18n.empty-no-data-zh;
        }

        // DataTable
        if root.records.length > 0: _RecordsTable {
            records: root.records;
        }
    }
}

component _RecordsTable inherits Rectangle {
    in property <[ConversionRecord]> records: [];

    background: DesignTokens.token-surface-1;
    border-color: DesignTokens.token-border;
    border-radius: DesignTokens.token-radius-lg;
    border-width: 1px;

    VerticalBox {
        padding: DesignTokens.token-space-4;

        // 表头
        HorizontalBox {
            spacing: 0px;
            padding: DesignTokens.token-space-3;
            background: DesignTokens.token-surface-3;
            border-radius: DesignTokens.token-radius-md;

            _Col { text: "Job ID"; flex: 2; }
            _Col { text: "Main File"; flex: 2; }
            _Col { text: "Profile"; flex: 1; }
            _Col { text: "Quality"; flex: 1; }
            _Col { text: "Status"; flex: 1; }
            _Col { text: "Created"; flex: 2; }
        }

        // 数据行
        ListView {
            for record[idx] in root.records: HorizontalBox {
                spacing: 0px;
                padding: DesignTokens.token-space-3;
                background: Math.mod(idx, 2) == 0
                    ? DesignTokens.token-surface-1
                    : DesignTokens.token-surface-4;
                border-radius: DesignTokens.token-radius-md;

                _Cell { text: record.job-id; flex: 2; overflow: elide; }
                _Cell { text: record.main-tex != "" ? record.main-tex : "-"; flex: 2; overflow: elide; }
                _Cell { text: record.profile != "" ? record.profile : "-"; flex: 1; }
                _Cell { text: record.quality != "" ? record.quality : "-"; flex: 1; }
                StatusChip { text: record.status; status: record.status; flex: 1; }
                _Cell { text: _format-date(record.created-at); flex: 2; }
            }
        }
    }
}

component _Col inherits Text {
    in property <string> text: "";
    in property <float> flex: 1;
    font-size: DesignTokens.token-font-size-md;
    font-weight: DesignTokens.token-font-weight-semibold;
    color: DesignTokens.token-text-secondary;
    horizontal-alignment: center;
}

component _Cell inherits Text {
    in property <string> text: "";
    font-size: DesignTokens.token-font-size-base;
    color: DesignTokens.token-text-primary;
    horizontal-alignment: center;
    overflow: elide;
}
```

### 5.11 `RechargeRecordsPanel` — 充值记录页面（DataTable）

与 `ConvertRecordsPanel` 结构相同，仅数据源为 `RechargeRecord`，列不同：

| 列 | 映射 |
|----|------|
| ID | `recharge.id` |
| Type | `recharge.type` (按次/按日期) |
| Package | `recharge.package-name` + `recharge.count` |
| Amount | `recharge.amount` (绿色正数) |
| Provider | `recharge.provider` |
| Status | `recharge.status` (StatusChip) |
| Created | `recharge.created-at` |

### 5.12 `RechargePanel` — 充值面板（Mock 支付，与 Flutter `_RechargePanel` 对齐）

```slint
// crates/desktop-slint/src/ui/pages/recharge-panel.slint

import { VerticalBox, HorizontalBox, Button, Rectangle } from "std-widgets.slint";
import { DesignTokens } from "../tokens.slint";
import { I18n } from "../i18n-strings.slint";
import { StatusPill } from "../components/status-pill.slint";

export component RechargePanel inherits Rectangle {
    in property <bool> is-busy: false;
    in property <string> status-message: "";
    in property <string> currency: "CNY";
    callback recharge-clicked(string, string);
    callback query-records();

    background: DesignTokens.token-window-bg;

    VerticalBox {
        padding: DesignTokens.token-space-8;
        spacing: DesignTokens.token-space-6;

        // 标题
        VerticalBox {
            spacing: DesignTokens.token-space-1;
            Text {
                text: ui-locale == "en" ? I18n.recharge-title : I18n.recharge-title-zh;
                font-size: DesignTokens.token-font-size-xl;
                font-weight: DesignTokens.token-font-weight-bold;
                color: DesignTokens.token-text-primary;
            }
            Text {
                text: ui-locale == "en" ? I18n.recharge-description : I18n.recharge-description-zh;
                font-size: DesignTokens.token-font-size-base;
                color: DesignTokens.token-text-secondary;
            }
        }

        // 操作行
        HorizontalBox {
            spacing: DesignTokens.token-space-3;
            Button {
                text: ui-locale == "en" ? I18n.recharge-query-records : I18n.recharge-query-records-zh;
                enabled: !root.is-busy;
                clicked => { root.query-records(); }
            }
            StatusPill {
                icon: "💰";
                label: ui-locale == "en" ? I18n.recharge-mock-provider : I18n.recharge-mock-provider-zh;
                color: DesignTokens.token-accent;
            }
        }

        // 按次充值
        Text {
            text: ui-locale == "en" ? I18n.recharge-count-title : I18n.recharge-count-title-zh;
            font-size: DesignTokens.token-font-size-lg;
            font-weight: DesignTokens.token-font-weight-semibold;
            color: DesignTokens.token-text-primary;
        }

        HorizontalBox {
            spacing: DesignTokens.token-space-3;
            _RechargeButton { pkg-id: "count_3"; label: "3 次 / ¥3"; enabled: !root.is-busy; recharge-clicked => { root.recharge-clicked("count", "count_3"); } }
            _RechargeButton { pkg-id: "count_10"; label: "10 次 / ¥10"; enabled: !root.is-busy; recharge-clicked => { root.recharge-clicked("count", "count_10"); } }
            _RechargeButton { pkg-id: "count_30"; label: "30 次 / ¥30"; enabled: !root.is-busy; recharge-clicked => { root.recharge-clicked("count", "count_30"); } }
        }

        // 日期充值
        Text {
            text: ui-locale == "en" ? I18n.recharge-date-title : I18n.recharge-date-title-zh;
            font-size: DesignTokens.token-font-size-lg;
            font-weight: DesignTokens.token-font-weight-semibold;
            color: DesignTokens.token-text-primary;
        }

        HorizontalBox {
            spacing: DesignTokens.token-space-3;
            _RechargeButton { pkg-id: "day"; label: "日卡 / ¥5"; enabled: !root.is-busy; recharge-clicked => { root.recharge-clicked("date", "day"); } }
            _RechargeButton { pkg-id: "week"; label: "周卡 / ¥14"; enabled: !root.is-busy; recharge-clicked => { root.recharge-clicked("date", "week"); } }
            _RechargeButton { pkg-id: "month"; label: "月卡 / ¥30"; enabled: !root.is-busy; recharge-clicked => { root.recharge-clicked("date", "month"); } }
            _RechargeButton { pkg-id: "year"; label: "年卡 / ¥120"; enabled: !root.is-busy; recharge-clicked => { root.recharge-clicked("date", "year"); } }
        }

        // 状态消息
        if root.status-message != "": StatusPill {
            icon: "✅";
            label: root.status-message;
            color: DesignTokens.token-success;
        }
    }
}

component _RechargeButton inherits Button {
    in property <string> pkg-id: "";
    in property <string> label: "";
    callback recharge-clicked();
    text: root.label;
    enabled: root.enabled;
    primary: false;
    clicked => { root.recharge-clicked(); }
}
```

### 5.13 `StatusChip` — 状态徽章（与 Flutter `_StatusChip` 对齐）

```slint
// crates/desktop-slint/src/ui/components/status-chip.slint

import { DesignTokens } from "../tokens.slint";

export component StatusChip inherits Rectangle {
    in property <string> text: "";
    in property <string> status: ""; // pending | running | completed | failed | paid | etc.

    background: _bg-color(root.status);
    border-radius: DesignTokens.token-radius-sm;
    min-width: 64px;
    height: 24px;
    padding: 0px DesignTokens.token-space-2;

    Text {
        text: root.text;
        font-size: DesignTokens.token-font-size-sm;
        font-weight: DesignTokens.token-font-weight-semibold;
        color: _text-color(root.status);
        horizontal-alignment: center;
        vertical-alignment: center;
    }
}

function _bg-color(status: string) -> color {
    if (status == "succeeded" || status == "completed" || status == "paid" || status == "paid_mock")
        return DesignTokens.token-success-subtle;
    if (status == "failed" || status == "expired")
        return DesignTokens.token-danger-subtle;
    if (status == "running" || status == "processing" || status == "uploading" || status == "pending")
        return DesignTokens.token-info-subtle;
    return DesignTokens.token-surface-3;
}

function _text-color(status: string) -> color {
    if (status == "succeeded" || status == "completed" || status == "paid" || status == "paid_mock")
        return DesignTokens.token-success;
    if (status == "failed" || status == "expired")
        return DesignTokens.token-danger;
    if (status == "running" || status == "processing" || status == "uploading" || status == "pending")
        return DesignTokens.token-info;
    return DesignTokens.token-text-muted;
}
```

### 5.14 `ToastNotify` — 轻量通知（右上角浮窗）

```slint
// crates/desktop-slint/src/ui/components/toast-notify.slint

import { VerticalBox, HorizontalBox, Button, Rectangle, Text } from "std-widgets.slint";
import { DesignTokens } from "../tokens.slint";
import { I18n } from "../i18n-strings.slint";

export component ToastNotify inherits Rectangle {
    in property <string> message: "";
    in property <string> level: "info"; // success | error | warning | info
    in property <bool> visible: false;

    callback dismissed();

    visible: root.visible;
    x: parent.width - self.width - DesignTokens.token-space-4;
    y: DesignTokens.token-space-4;
    width: 320px;
    background: DesignTokens.token-surface-1;
    border-radius: DesignTokens.token-radius-lg;
    border-width: 1px;
    border-color: DesignTokens.token-border;
    drop-shadow-blur-radius: 8px;
    drop-shadow-color: #0000001A;

    // 左侧色条
    Rectangle {
        width: 4px;
        height: parent.height;
        background: _accent-color(root.level);
        border-radius: DesignTokens.token-radius-lg;
    }

    HorizontalBox {
        padding: DesignTokens.token-space-3 DesignTokens.token-space-4;
        spacing: DesignTokens.token-space-3;

        Text {
            text: _icon(root.level);
            font-size: DesignTokens.token-font-size-lg;
            vertical-alignment: center;
        }

        Text {
            text: root.message;
            font-size: DesignTokens.token-font-size-md;
            color: DesignTokens.token-text-primary;
            wrap: word-wrap;
            horizontal-stretch: 1;
            vertical-alignment: center;
        }

        Button {
            text: "✕";
            max-width: 24px;
            min-width: 24px;
            clicked => { root.dismissed(); }
        }
    }
}

function _accent-color(level: string) -> color {
    if (level == "success") return DesignTokens.token-success;
    if (level == "error") return DesignTokens.token-danger;
    if (level == "warning") return DesignTokens.token-warning;
    return DesignTokens.token-accent;
}

function _icon(level: string) -> string {
    if (level == "success") return "✅";
    if (level == "error") return "❌";
    if (level == "warning") return "⚠️";
    return "ℹ️";
}
```

### 5.15 `LoadingIndicator` — 加载状态

```slint
// crates/desktop-slint/src/ui/components/loading-indicator.slint

import { HorizontalBox, ProgressIndicator, Rectangle } from "std-widgets.slint";
import { DesignTokens } from "../tokens.slint";

export component LoadingIndicator inherits Rectangle {
    in property <string> label: "Loading...";

    background: transparent;
    min-height: 48px;

    HorizontalBox {
        spacing: DesignTokens.token-space-3;
        alignment: center;

        ProgressIndicator {
            progress: -1;  // indeterminate
        }

        Text {
            text: root.label;
            font-size: DesignTokens.token-font-size-md;
            color: DesignTokens.token-text-secondary;
            vertical-alignment: center;
        }
    }
}
```

### 5.16 `EmptyState` — 空态组件（增强版）

```slint
// crates/desktop-slint/src/ui/components/empty-state.slint

import { VerticalBox, Rectangle } from "std-widgets.slint";
import { DesignTokens } from "../tokens.slint";

export component EmptyState inherits Rectangle {
    in property <string> title: "";
    in property <string> message: "";
    in property <string> icon: "📭";

    background: DesignTokens.token-surface-3;
    border-color: DesignTokens.token-border;
    border-radius: DesignTokens.token-radius-lg;
    border-width: 1px;
    min-height: 120px;

    VerticalBox {
        padding: DesignTokens.token-space-8;
        spacing: DesignTokens.token-space-3;
        alignment: center;

        Text {
            text: root.icon;
            font-size: 32px;
            horizontal-alignment: center;
        }

        Text {
            text: root.title;
            font-size: DesignTokens.token-font-size-lg;
            font-weight: DesignTokens.token-font-weight-semibold;
            color: DesignTokens.token-text-secondary;
            horizontal-alignment: center;
        }

        Text {
            text: root.message;
            font-size: DesignTokens.token-font-size-base;
            color: DesignTokens.token-text-muted;
            horizontal-alignment: center;
            wrap: word-wrap;
            max-width: 400px;
        }
    }
}
```

---

## 六、数据模型扩展

### 6.1 `types.slint` 扩展

```slint
// crates/desktop-slint/src/ui/types.slint

// 已有
export struct PlanEntry { id, name, price, quota, features }
export struct JobRow { id, kind, input, output, status, opened-at, error, html-report }
export enum ConversionMode { local, cloud }

// Flutter 新增结构
export struct ConversionRecord {
    job-id: string,
    remote-job-id: string,
    main-tex: string,
    profile: string,
    quality: string,
    status: string,   // pending | running | completed | failed | expired
    created-at: string,
    error: string,
}

export struct RechargeRecord {
    id: string,
    type: string,      // "count" | "date"
    package-id: string,
    package-name: string,
    quantity: int,
    amount: string,
    currency: string,
    provider: string,
    status: string,    // pending | paid | paid_mock | failed
    created-at: string,
}

export struct UsageSummary {
    cloud-conversions-used: int,
    cloud-conversions-limit: int,
    count-balance: int,
    date-valid-until: string,
    entitlement: string,
}

// Toast
export enum ToastLevel { success, error, warning, info }

export struct ToastMessage {
    id: string,
    level: ToastLevel,
    message: string,
}

// App 状态
export enum AppPage {
    account,
    recharge,
    convert,
    convert-records,
    recharge-records,
}

// User Tier
export enum UserTier {
    free,
    pro,
    enterprise,
}
```

---

## 七、Rust 侧扩展

### 7.1 `theme.rs` — DesignToken 同步函数

```rust
// crates/desktop-slint/src/theme.rs

use slint::Color;

pub fn apply_theme_to_tokens(ui: &MainWindow, theme: &str) {
    let tokens = ui.global::<DesignTokens>();
    let p = palette(theme);

    // Surface / Background
    tokens.set_token_window_bg(parse_color(p.window_bg));
    tokens.set_token_surface_1(parse_color(p.surface));
    tokens.set_token_surface_2(parse_color(p.surface));
    tokens.set_token_surface_3(parse_color(p.surface_alt));
    tokens.set_token_surface_4(parse_color_adjusted(p.surface_alt, 0.95));

    // Border
    tokens.set_token_border(parse_color(p.border));
    tokens.set_token_border_strong(parse_color_adjusted(p.border, 1.1));
    tokens.set_token_border_focus(parse_color(p.accent));

    // Text
    tokens.set_token_text_primary(parse_color(p.text_primary));
    tokens.set_token_text_secondary(parse_color(p.text_secondary));
    tokens.set_token_text_muted(parse_color(p.text_muted));
    tokens.set_token_text_inverse(if is_dark(theme) {
        parse_color("#0B1120")
    } else {
        parse_color("#FFFFFF")
    });
    tokens.set_token_text_link(parse_color(p.accent));
    tokens.set_token_text_disabled(parse_color(p.text_muted));

    // Accent (per theme)
    tokens.set_token_accent(parse_color(p.accent));
    tokens.set_token_accent_hover(parse_color_adjusted(p.accent, 0.9));
    tokens.set_token_accent_pressed(parse_color_adjusted(p.accent, 0.8));
    tokens.set_token_accent_subtle(parse_color_subtle(p.accent));

    // Semantic
    tokens.set_token_success(parse_color(p.success));
    tokens.set_token_success_subtle(parse_color_subtle(p.success));
    tokens.set_token_warning(parse_color(p.warning));
    tokens.set_token_warning_subtle(parse_color_subtle(p.warning));
    tokens.set_token_danger(parse_color(p.danger));
    tokens.set_token_danger_subtle(parse_color_subtle(p.danger));
    tokens.set_token_info(parse_color(p.accent));
    tokens.set_token_info_subtle(parse_color_subtle(p.accent));

    // Disabled
    if is_dark(theme) {
        tokens.set_token_disabled_bg(parse_color("#1F2937"));
        tokens.set_token_disabled_text(parse_color("#475569"));
        tokens.set_token_disabled_border(parse_color("#374151"));
    } else {
        tokens.set_token_disabled_bg(parse_color("#F1F5F9"));
        tokens.set_token_disabled_text(parse_color("#9CA3AF"));
        tokens.set_token_disabled_border(parse_color("#E5E7EB"));
    }
}

fn parse_color(hex: &str) -> Color {
    let t = hex.trim_start_matches('#');
    let v = u32::from_str_radix(t, 16).unwrap_or(0);
    Color::from_argb_u8(
        255,
        ((v >> 16) & 0xff) as u8,
        ((v >> 8) & 0xff) as u8,
        (v & 0xff) as u8,
    )
}

fn parse_color_adjusted(hex: &str, factor: f32) -> Color {
    let c = parse_color(hex);
    Color::from_argb_u8(
        c.alpha(),
        ((c.red() as f32 * factor) as u8).min(255),
        ((c.green() as f32 * factor) as u8).min(255),
        ((c.blue() as f32 * factor) as u8).min(255),
    )
}

fn parse_color_subtle(hex: &str) -> Color {
    // Convert solid color to very subtle tint (10-12% opacity effect via lightening)
    let c = parse_color(hex);
    Color::from_argb_u8(
        c.alpha(),
        (220u8.saturating_add((c.red() - 220) / 3)),
        (248u8.saturating_add((c.green() - 248) / 3)),
        (250u8.saturating_add((c.blue() - 250) / 3)),
    )
}

fn is_dark(theme: &str) -> bool {
    matches!(theme, "dark")
}
```

### 7.2 `i18n.rs` — Flutter i18n key 追加

在 `i18n.rs` 的 `text_for()` 中追加所有 Flutter key，zh-Hans/zh-Hant/ja-JP 返回翻译（绕过 ICU4X）：

```rust
// crates/desktop-slint/src/i18n.rs

fn text_for(locale: &str, key: &str) -> Option<&'static str> {
    match locale {
        "zh-Hans" => zh_hans_full(key),
        "zh-Hant" => zh_hant_full(key),
        "ja-JP" => ja_full(key),
        "fr" => fr_full(key).or_else(|| en_full(key)),
        "de" => de_full(key).or_else(|| en_full(key)),
        _ => en_full(key),
    }
}

// 所有 key 的英文/中文翻译表（包含 Flutter 新增 key）
fn en_full(key: &str) -> Option<&'static str> {
    match key {
        // Flutter app.*
        "app.title" => Some("Tex2Doc"),
        "app.subtitle" => Some("Commercial LaTeX to DOCX conversion workspace"),
        "topbar.platform" => Some("Platform"),
        // nav.*
        "nav.account" => Some("Account"),
        "nav.recharge" => Some("Recharge"),
        "nav.convert" => Some("Convert"),
        "nav.convert-records" => Some("Conversion Records"),
        "nav.recharge-records" => Some("Recharge Records"),
        // auth.*
        "auth.login-tab" => Some("Login"),
        "auth.register-tab" => Some("Register"),
        "auth.sign-in-first" => Some("Please sign in to continue."),
        // account.*
        "account.email" => Some("Email"),
        "account.password" => Some("Password"),
        "account.confirm-password" => Some("Confirm Password"),
        "account.change-password" => Some("Change Password"),
        "account.view-profile" => Some("View Profile"),
        "account.profile-title" => Some("Account Details"),
        "account.current-plan" => Some("Current Plan"),
        "account.logout" => Some("Sign Out"),
        "account.display-name" => Some("Display Name"),
        "account.password-mismatch" => Some("Passwords do not match"),
        "account.password-too-short" => Some("Password must be at least 6 characters"),
        // recharge.*
        "recharge.title" => Some("Recharge"),
        "recharge.count-title" => Some("By count"),
        "recharge.date-title" => Some("By duration"),
        "recharge.mock-paid" => Some("Mock payment settled CNY {amount} through {provider}."),
        // convert.*
        "convert.title" => Some("Document conversion"),
        "convert.step-upload" => Some("1. Package the full LaTeX project as a ZIP."),
        "convert.step-main-tex" => Some("2. Enter the main TeX path inside the ZIP."),
        "convert.step-convert" => Some("3. Run the cloud semantic engine and download DOCX."),
        // common.*
        "common.register" => Some("Register"),
        "common.login" => Some("Login"),
        "common.refresh" => Some("Refresh"),
        "common.confirm" => Some("OK"),
        "common.cancel" => Some("Cancel"),
        "common.loading" => Some("Working..."),
        "common.empty" => Some("No data"),
        "common.error" => Some("Error"),
        "empty.no-data" => Some("No data yet. Results will appear here after an operation."),
        "error.network" => Some("Network or service error. Check the API base URL."),
        // toast.*
        "toast.sign-in-success" => Some("Signed in successfully."),
        "toast.password-changed" => Some("Password changed successfully."),
        "toast.recharge-success" => Some("Recharge successful."),
        // ... (其余所有 Flutter key)
        _ => None,
    }
}

fn zh_hans_full(key: &str) -> Option<&'static str> {
    match key {
        "app.title" => Some("Tex2Doc"),
        "app.subtitle" => Some("LaTeX 到 DOCX 的商业级转换工作台"),
        "topbar.platform" => Some("平台"),
        "nav.account" => Some("账号"),
        "nav.recharge" => Some("充值"),
        "nav.convert" => Some("转换"),
        "nav.convert-records" => Some("转换记录"),
        "nav.recharge-records" => Some("充值记录"),
        "auth.login-tab" => Some("登录"),
        "auth.register-tab" => Some("注册"),
        "auth.sign-in-first" => Some("请先登录以继续操作。"),
        "account.email" => Some("邮箱"),
        "account.password" => Some("密码"),
        "account.confirm-password" => Some("确认密码"),
        "account.change-password" => Some("修改密码"),
        "account.view-profile" => Some("查看详情"),
        "account.profile-title" => Some("账号详情"),
        "account.current-plan" => Some("当前套餐"),
        "account.logout" => Some("退出登录"),
        "account.display-name" => Some("显示名称"),
        "account.password-mismatch" => Some("两次输入的密码不一致"),
        "account.password-too-short" => Some("密码长度不能少于 6 位"),
        "recharge.title" => Some("充值"),
        "recharge.count-title" => Some("按次充值"),
        "recharge.date-title" => Some("日期充值"),
        "recharge.mock-paid" => Some("mock 支付完成，到账 ¥{amount}，渠道 {provider}。"),
        "convert.title" => Some("文档转换"),
        "convert.step-upload" => Some("1. 将完整 LaTeX 项目打包为 ZIP 后上传。"),
        "convert.step-main-tex" => Some("2. 填写 ZIP 内主 TeX 文件相对路径。"),
        "convert.step-convert" => Some("3. 启动云端语义引擎并下载 DOCX。"),
        "common.register" => Some("注册"),
        "common.login" => Some("登录"),
        "common.refresh" => Some("刷新"),
        "common.confirm" => Some("确定"),
        "common.cancel" => Some("取消"),
        "common.loading" => Some("处理中..."),
        "common.empty" => Some("暂无数据"),
        "common.error" => Some("出错"),
        "empty.no-data" => Some("暂无数据。完成一次操作后这里会显示结果。"),
        "error.network" => Some("网络或服务异常，请检查 API 地址。"),
        "toast.sign-in-success" => Some("登录成功。"),
        "toast.password-changed" => Some("密码修改成功。"),
        "toast.recharge-success" => Some("充值成功。"),
        // ... (其余所有 Flutter key)
        _ => None,
    }
}
```

### 7.3 `main.rs` — 新增 Callback 绑定

在 `main.rs` 中追加以下 callback 绑定（`apply_i18n` 和 `apply_theme` 保持兼容）：

```rust
// crates/desktop-slint/src/main.rs

// === Auth callbacks ===
let app_state_clone = Arc::clone(&app_state);
let ui_weak = ui.as_weak();
ui.on_auth_login(move |email, password, base_url| {
    let base_url = base_url.to_string();
    let email = email.to_string();
    let password = password.to_string();
    let app = Arc::clone(&app_state_clone);
    let ui_weak = ui_weak.clone();
    std::thread::spawn(move || {
        let result = cloud_account::login_and_fetch_usage_blocking(&base_url, &email, &password);
        slint::invoke_from_event_loop(move || {
            if let Some(ui) = ui_weak.upgrade() {
                match result {
                    Ok(session) => {
                        apply_account_session(&app, &ui, &base_url, session);
                        ui.set_is_signed_in(true);
                        // Show toast
                        ui.set_toast_message(tr("toast.sign-in-success").into());
                        ui.set_toast_level("success".into());
                        ui.set_toast_visible(true);
                    }
                    Err(e) => {
                        ui.set_toast_message(format!("Login failed: {}", e).into());
                        ui.set_toast_level("error".into());
                        ui.set_toast_visible(true);
                    }
                }
            }
        });
    });
});

// === Recharge callbacks ===
ui.on_recharge_clicked(move |recharge_type, package_id| {
    let base_url = ui.get_api_base_url().to_string();
    let token = app_state_clone.auth_token();
    let ui_weak = ui_weak.clone();
    std::thread::spawn(move || {
        // Mock 充值逻辑（直接标记 paid_mock）
        let result = cloud_account::mock_recharge_blocking(&base_url, token, recharge_type.as_str(), package_id.as_str());
        slint::invoke_from_event_loop(move || {
            if let Some(ui) = ui_weak.upgrade() {
                match result {
                    Ok(record) => {
                        ui.set_recharge_status(format!(
                            "Mock payment settled CNY {} through {}.",
                            record.amount, record.provider
                        ).into());
                        ui.set_toast_message(tr("toast.recharge-success").into());
                        ui.set_toast_level("success".into());
                        ui.set_toast_visible(true);
                    }
                    Err(e) => {
                        ui.set_toast_message(format!("Recharge failed: {}", e).into());
                        ui.set_toast_level("error".into());
                        ui.set_toast_visible(true);
                    }
                }
            }
        });
    });
});

// === Conversion records ===
ui.on_query_conversion_records(move || {
    let base_url = ui.get_api_base_url().to_string();
    let token = app_state_clone.auth_token();
    let ui_weak = ui_weak.clone();
    std::thread::spawn(move || {
        let result = cloud_account::fetch_conversions_blocking(&base_url, token);
        slint::invoke_from_event_loop(move || {
            if let Some(ui) = ui_weak.upgrade() {
                match result {
                    Ok(records) => {
                        let slint_records: Vec<ConversionRecord> = records.into_iter().map(|r| ConversionRecord { ... }).collect();
                        ui.set_conversion_records(slint_records.into());
                    }
                    Err(e) => {
                        ui.set_toast_message(format!("Failed to load records: {}", e).into());
                        ui.set_toast_level("error".into());
                        ui.set_toast_visible(true);
                    }
                }
            }
        });
    });
});

// === Recharge records ===
ui.on_query_recharge_records(move || {
    // 类似 conversion records
});

// === Theme/Locale callbacks ===
ui.on_theme_changed(move |theme| {
    let theme = theme::normalize_theme(theme.as_str());
    if let Some(ui) = ui_weak.upgrade() {
        ui.set_ui_theme(theme.clone().into());
        apply_theme_to_tokens(&ui, &theme);
        persist_appearance(&ui.get_ui_locale().to_string(), &theme);
    }
});

ui.on_locale_changed(move |locale| {
    let locale = i18n::normalize_locale(locale.as_str());
    if let Some(ui) = ui_weak.upgrade() {
        ui.set_ui_locale(locale.clone().into());
        apply_i18n_full(&ui, &locale);
        persist_appearance(&locale, &ui.get_ui_theme().to_string());
    }
});

// === Toast dismiss ===
ui.on_toast_dismissed(move || {
    if let Some(ui) = ui_weak.upgrade() {
        ui.set_toast_visible(false);
    }
});

// === Change password ===
ui.on_change_password_clicked(move |old_pwd, new_pwd, confirm_pwd| {
    // 客户端校验已在 Slint 端完成，此处调用 API
    // 当前为 mock 实现
    let ui_weak = ui_weak.clone();
    std::thread::spawn(move || {
        // 调用 POST /auth/password (预留)
        slint::invoke_from_event_loop(move || {
            if let Some(ui) = ui_weak.upgrade() {
                // Mock 成功
                ui.set_toast_message(tr("toast.password-changed").into());
                ui.set_toast_level("success".into());
                ui.set_toast_visible(true);
            }
        });
    });
});
```

### 7.4 `app_state.rs` — 新增状态

```rust
// crates/desktop-slint/src/app_state.rs

#[derive(Clone, Debug)]
pub struct ConversionRecordState {
    pub job_id: String,
    pub remote_job_id: Option<String>,
    pub main_tex: Option<String>,
    pub profile: String,
    pub quality: String,
    pub status: String,
    pub created_at: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RechargeRecordState {
    pub id: String,
    pub recharge_type: String,
    pub package_id: String,
    pub package_name: String,
    pub quantity: i32,
    pub amount: String,
    pub currency: String,
    pub provider: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Clone, Debug)]
pub struct ToastState {
    pub message: String,
    pub level: String,
    pub visible: bool,
}

pub enum AppPageState {
    Account,
    Recharge,
    Convert,
    ConvertRecords,
    RechargeRecords,
}
```

---

## 八、布局与尺寸参数对照

| Flutter | 值 | Slint | 值 |
|---------|---|-------|---|
| Sidebar width | 248px | `token-sidebar-width` | 248px |
| TopBar height | 64px | `token-header-height` | 64px |
| Content max-width | 1180px | `token-content-max-width` | 1180px |
| Card padding | 24px | `token-space-6` | 24px |
| NavItem padding | 12px / 8px | `token-space-3` / `token-space-2` | 12px / 8px |
| Auth card max-width | 420px | 固定 max-width | 420px |
| Metric card min-height | 86px | MetricCard min-height | 86px |
| Dialog width | 400px | Dialog width | 400px |

---

## 九、状态覆盖完整矩阵

| 页面/组件 | Normal | Loading | Empty | Error | Disabled | Offline | Quota-Expired | Token-Expired |
|---------|--------|---------|-------|-------|---------|--------|-------------|--------------|
| `AuthPage` | 登录/注册表单 | LoginButton spinner | - | 错误 Banner | 按钮禁用 | API 未配置 Banner | - | - |
| `Sidebar` | 5 个 NavItem | - | - | - | - | - | - | - |
| `TopBar` | 主题/语言/头像 | - | - | - | - | - | - | - |
| `AccountAvatarButton` | CircleAvatar + 菜单 | - | - | - | - | - | - | - |
| `ProfileDialog` | 头像 + 信息 | - | - | - | - | - | - | - |
| `ChangePasswordDialog` | 3 个密码输入框 | 确认按钮 spinner | - | 错误文案 | 按钮禁用 | - | - | - |
| `AccountPanel` | 账号卡片 + 指标 | Skeleton | - | API 错误 Banner | - | - | AlertBanner | AlertBanner |
| `RechargePanel` | 按次/按日期按钮 | Recharging spinner | - | 错误 Banner | 按钮禁用 | 未登录 Banner | - | - |
| `ConvertPanel` | 上传+主文件+转换 | 转换中 ProgressIndicator | 状态提示 | 错误 Banner | 按钮禁用 | 未登录 Banner | AlertBanner | - |
| `ConvertRecordsPanel` | DataTable | LoadingIndicator | EmptyState | Error Banner | - | 未登录 Banner | - | - |
| `RechargeRecordsPanel` | DataTable | LoadingIndicator | EmptyState | Error Banner | - | 未登录 Banner | - | - |
| `ToastNotify` | 右上角浮窗 | - | - | - | - | - | - | - |

---

## 十、主题切换机制

### 10.1 6+1 主题体系

| 主题 | Flutter seed | Slint accent | 说明 |
|------|------------|-------------|------|
| `default` | `#2563EB` | `#2563EB` | 蓝色主色 |
| `blue` | `#1D4ED8` | `#1D4ED8` | 深蓝 |
| `green` | `#047857` | `#047857` | 绿色 |
| `purple` | `#7C3AED` | `#7C3AED` | 紫色 |
| `orange` | `#EA580C` | `#EA580C` | 橙色 |
| `dark` | `#60A5FA` | `#60A5FA` | 暗色（色值完全切换） |

### 10.2 切换流程

```
用户选择主题
  ↓
Rust: theme::normalize_theme() 验证
  ↓
Rust: apply_theme_to_tokens(ui, theme) → 写入 Slint DesignTokens global
  ↓
Slint: 所有引用 DesignTokens.token-* 的组件自动重渲染
  ↓
同时更新 settings.locale / settings.theme 持久化
```

---

## 十一、实施计划

### Phase 1: 基础设施（预计 3 天）

| 任务 | 文件 | 说明 |
|------|------|------|
| 扩展 `tokens.slint` DesignToken（对齐 Flutter） | `ui/tokens.slint` | 完整 token 定义 |
| 扩展 `motion.slint` | `ui/motion.slint` | 动画 token |
| 扩展 `i18n-strings.slint` | `ui/i18n-strings.slint` | Flutter 全部 key |
| 扩展 `types.slint` | `ui/types.slint` | 新增结构体 |
| 实现 `theme.rs` 的 `apply_theme_to_tokens()` | `src/theme.rs` | DesignTokens 同步 |
| 实现 `i18n.rs` 的 `zh_hans_full()` / `zh_hant_full()` | `src/i18n.rs` | 绕过 ICU4X |
| 验证：6 主题切换正常 | - | - |
| 验证：zh-Hans/zh-Hant/ja 渲染正常 | - | - |

### Phase 2: 组件库构建（预计 2 天）

| 任务 | 文件 |
|------|------|
| `sidebar-nav-item.slint` | 导航条目 |
| `status-chip.slint` | 状态徽章 |
| `empty-state.slint` | 空态（增强版） |
| `loading-indicator.slint` | 加载指示器 |
| `toast-notify.slint` | Toast 通知 |
| `theme-dropdown.slint` | 主题下拉 |
| `locale-dropdown.slint` | 语言下拉 |
| `account-avatar-button.slint` | 账号头像 |
| `profile-dialog.slint` | 账号详情弹窗 |
| `change-password-dialog.slint` | 修改密码弹窗 |

### Phase 3: 页面重构（预计 3 天）

| 任务 | 文件 |
|------|------|
| `auth.slint`（独立登录/注册窗口） | `pages/auth.slint` |
| `sidebar.slint`（248px 侧边栏） | `pages/sidebar.slint` |
| `topbar.slint`（64px 顶栏） | `pages/topbar.slint` |
| `workspace-shell.slint`（布局骨架） | `pages/workspace-shell.slint` |
| `account-panel.slint` | `pages/account-panel.slint` |
| `recharge-panel.slint`（mock 充值） | `pages/recharge-panel.slint` |
| `convert-panel.slint` | `pages/convert-panel.slint` |
| `convert-records.slint`（DataTable） | `pages/convert-records.slint` |
| `recharge-records.slint`（DataTable） | `pages/recharge-records.slint` |
| `main.slint`（精简为 ~180 行） | `main.slint` |

### Phase 4: Rust 回调绑定（预计 2 天）

| 任务 | 文件 |
|------|------|
| 新增 AppState 字段 | `src/app_state.rs` |
| 新增 theme/toast/recharge callback | `src/main.rs` |
| Mock 充值实现 | `cloud_account.rs` |
| 转换/充值记录查询 | `cloud_account.rs` |
| Rust → Slint DesignTokens 同步 | `src/theme.rs` |
| Rust → Slint I18n 全量同步 | `src/i18n.rs` |

### Phase 5: 集成验证（预计 1 天）

| 验收项 | 验证方式 |
|--------|---------|
| `main.slint` 行数 ≤ 200 | 代码行数统计 |
| 所有颜色字面量替换为 `DesignTokens.token-*` | grep 验证 |
| 所有用户可见文本通过 `I18n.*` 引用 | grep 验证 |
| 6 主题 + 暗色主题切换正常 | UI 测试 |
| zh-Hans/zh-Hant/ja 正常渲染 | UI 测试 |
| 登录/注册流程正常 | UI 测试 |
| 侧边栏 5 个导航项正常切换 | UI 测试 |
| 账号头像 PopupMenu 正常 | UI 测试 |
| 修改密码弹窗正常 | UI 测试 |
| 充值 mock 流程正常 | UI 测试 |
| 转换记录 DataTable 正常 | UI 测试 |
| 充值记录 DataTable 正常 | UI 测试 |
| Toast 通知正常弹出/消失 | UI 测试 |
| `cargo build -p desktop-slint` 编译通过 | CI/本地 |

---

## 十二、Flutter 与 Slint 组件对照表

| Flutter Widget | Flutter 文件 | Slint 组件 | Slint 文件 |
|---------------|-------------|-----------|-----------|
| `AuthWindow` | `auth_window.dart` | `AuthPage` | `pages/auth.slint` |
| `_WorkspaceShell` | `workspace_app.dart` | `WorkspaceShell` | `pages/workspace-shell.slint` |
| `_Sidebar` | `workspace_app.dart` | `Sidebar` | `pages/sidebar.slint` |
| `_NavItem` | `workspace_app.dart` | `SidebarNavItem` | `components/sidebar-nav-item.slint` |
| `_TopBar` | `workspace_app.dart` | `TopBar` | `pages/topbar.slint` |
| `_ThemeDropdown` | `workspace_app.dart` | `ThemeDropdown` | `components/theme-dropdown.slint` |
| `_LocaleDropdown` | `workspace_app.dart` | `LocaleDropdown` | `components/locale-dropdown.slint` |
| `_AccountAvatarButton` | `workspace_app.dart` | `AccountAvatarButton` | `components/account-avatar-button.slint` |
| `_ProfileDialog` | `workspace_app.dart` | `ProfileDialog` | `components/profile-dialog.slint` |
| `_ChangePasswordDialog` | `workspace_app.dart` | `ChangePasswordDialog` | `components/change-password-dialog.slint` |
| `_AccountPanel` | `workspace_app.dart` | `AccountPanel` | `pages/account-panel.slint` |
| `_RechargePanel` | `workspace_app.dart` | `RechargePanel` | `pages/recharge-panel.slint` |
| `_ConvertPanel` | `workspace_app.dart` | `ConvertPanel` | `pages/convert-panel.slint` |
| `ConvertRecordsPanel` | `convert_records_panel.dart` | `ConvertRecordsPanel` | `pages/convert-records.slint` |
| `_RecordsTable` | `convert_records_panel.dart` | `RecordsTable` | `components/records-table.slint` |
| `_StatusChip` | `convert_records_panel.dart` | `StatusChip` | `components/status-chip.slint` |
| `RechargeRecordsPanel` | `recharge_records_panel.dart` | `RechargeRecordsPanel` | `pages/recharge-records.slint` |
| `AppCard` | `app_components.dart` | 使用 `Rectangle` + token | 通用 |
| `AppSectionHeader` | `app_components.dart` | 使用 `VerticalBox` + Text | 页面标题 |
| `PageContainer` | `app_components.dart` | 内容区布局 | 页面容器 |
| `MetricTile` | `app_components.dart` | `MetricCard` (已有) | `components/metric-card.slint` |
| `LoadingState` | `app_components.dart` | `LoadingIndicator` | `components/loading-indicator.slint` |
| `EmptyState` | `app_components.dart` | `EmptyState` (增强) | `components/empty-state.slint` |
| `ErrorState` | `app_components.dart` | `AlertBanner` | `components/alert-banner.slint` |
| `StatusPill` | `app_components.dart` | `StatusPill` | `components/status-pill.slint` |
| N/A | - | `ToastNotify` | `components/toast-notify.slint` |
| N/A | - | `ChangePasswordDialog` | `components/change-password-dialog.slint` |

---

## 十三、与 Flutter 的核心差异及处理策略

由于 Slint 与 Flutter 在桌面端的能力差异，以下功能需要调整或降级处理：

| Flutter 功能 | Slint 等价方案 | 差异说明 |
|------------|--------------|---------|
| Material `TabController` + `TabBarView` | Slint `TabWidget` | 行为一致 |
| `PopupMenuButton` | 内联 Rectangle + TouchArea 模拟 | Slint 无原生 PopupMenu |
| `CircleAvatar` | 圆形 Rectangle + Text | Slint 无 CircleAvatar 原生组件 |
| `CircularProgressIndicator` | `ProgressIndicator { progress: -1 }` | 不确定进度条，需动画 |
| `SegmentedButton` | 多个 NavItem 高亮 | Slint 无 SegmentedButton |
| `AnimatedContainer` | Slint `states` 语法 | Slint 用状态机替代动画 |
| `AnimatedSwitcher` | `if` 条件切换 | 行为一致 |
| `DropdownButton` | `ComboBox` | Slint 原生 ComboBox |
| `showDialog()` | `if dialog-visible: Dialog` 或内联 Rectangle | Slint 无全局 Dialog API |
| `InkWell` | `TouchArea` + background | Slint 无 Material InkWell |
| `TextField` + suffix icon | `LineEdit` + 右侧 Button | Slint 无复合 TextField |
| Emoji icon | Text 渲染 emoji | Slint 无 IconFont，使用 emoji fallback |

---

*文档由 Cursor AI 基于 `commercial-ui-design` 技能自动生成，基于 Flutter 已实现方案对齐 Slint 桌面端*
