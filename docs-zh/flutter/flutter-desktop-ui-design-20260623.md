# Flutter 桌面应用界面设计方案

> 文档版本：v1.0
> 日期：2026-06-23
> 状态：**已实现**

---

## 1. 概述

本方案描述 Tex2Doc Flutter 桌面应用的商业化界面重构，涵盖登录/注册流程、导航结构、账号管理及独立记录模块的设计决策和实现细节。

### 1.1 需求回顾

| 需求 | 状态 |
|------|------|
| 登录、注册模块各自独立，不混在工作台中 | ✅ 已实现 |
| 登录和注册时密码输入掩码，防网络攻击 | ✅ 已实现 |
| 应用打开时进入登录窗口 | ✅ 已实现 |
| 登录成功后账号图标显示在窗口右上角 | ✅ 已实现 |
| 右上角支持退出、修改密码、显示详细信息 | ✅ 已实现 |
| 登录后方能操作其它模块 | ✅ 已实现 |
| 工作台模块从左侧导航移除 | ✅ 已实现 |
| 转换记录、充值记录作为独立模块放入左侧导航 | ✅ 已实现 |

### 1.2 技术栈

- **Flutter**: 3.44.2 / Dart 3.12.2
- **状态管理**: `setState` + 组件树 props 传递（轻量 SaaS 工具）
- **设计系统**: `app_theme.dart`, `app_tokens.dart`, `app_components.dart`, `app_i18n.dart`
- **国际化**: 手写 `AppStrings` 委托，支持 `zh-CN` / `en-US`
- **桌面桥接**: `dart:ffi` → Rust `doc_native.dll`（CDylib）
- **HTTP 客户端**: `commercial_api.dart` — REST + Bearer Token

---

## 2. 架构变更

### 2.1 文件变更概览

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `lib/workspace_app.dart` | 重构 | 核心应用壳；顶级 auth 状态；侧边栏重构 |
| `lib/ui/auth_window.dart` | 新增 | 独立登录/注册窗口 |
| `lib/ui/convert_records_panel.dart` | 新增 | 转换记录独立页面（DataTable） |
| `lib/ui/recharge_records_panel.dart` | 新增 | 充值记录独立页面（DataTable） |
| `lib/ui/app_i18n.dart` | 扩展 | 新增 auth、profile、nav 键 |
| `lib/main.dart` | 不变 | 入口点无需改动 |

### 2.2 应用状态流

```
DocEngineApp
└── _WorkspaceShell (StatefulWidget, 顶级状态)
    ├── _auth: _AuthState?  ← 登录后持有 apiBaseUrl + accessToken + UserProfile
    ├── _selectedSection: _NavSection
    │
    ├── [未登录] → AuthWindow (独立登录/注册 Tab 页)
    │              ├── Tab 0: 登录表单（邮箱 + 密码掩码 + API地址）
    │              └── Tab 1: 注册表单（邮箱 + 密码 + 确认密码 + API地址）
    │
    └── [已登录] → Scaffold
                   ├── Sidebar (248px，左侧导航)
                   │   ├── 账号
                   │   ├── 充值
                   │   ├── 转换
                   │   ├── 转换记录
                   │   └── 充值记录
                   └── Column
                       ├── TopBar (含右上角账号头像 PopupMenuButton)
                       │   ├── Logo + 平台标签
                       │   ├── 主题切换下拉
                       │   ├── 语言切换下拉
                       │   └── 账号头像 (CircleAvatar + 弹出菜单)
                       │       ├── 查看详情 → _ProfileDialog
                       │       ├── 修改密码 → _ChangePasswordDialog
                       │       └── 退出登录
                       └── _NavContent (根据 _selectedSection 渲染内容)
```

### 2.3 认证状态管理

认证状态提升到 `_WorkspaceShell`，通过 `_AuthState`（包含 `apiBaseUrl`、`accessToken`、`UserProfile`）管理。登录成功后：

```dart
void _handleSignedIn(String apiBaseUrl, String accessToken, UserProfile profile) {
  setState(() {
    _apiBaseUrl = apiBaseUrl;
    _auth = _AuthState(apiBaseUrl: apiBaseUrl, accessToken: accessToken, profile: profile);
  });
}
```

未登录时直接渲染 `AuthWindow`，登录成功后切换到 `Scaffold`。

---

## 3. 登录/注册窗口设计

### 3.1 布局

- 单卡片居中设计，最大宽度 420px
- 顶部：Logo + 应用名称 + 副标题
- Tab 栏：登录 / 注册切换
- 表单内容区高度固定 320px，避免 Tab 切换时抖动

### 3.2 密码掩码

使用 `obscureText: true` 的 `TextField`，配合右侧眼睛图标切换显示/隐藏：

```dart
class _PasswordField extends StatefulWidget {
  bool _obscured = true;
  // ...
  suffixIcon: IconButton(
    icon: Icon(_obscured ? Icons.visibility_off : Icons.visibility),
    onPressed: () => setState(() => _obscured = !_obscured),
  )
}
```

### 3.3 密码安全校验

- 注册表单：密码长度 ≥ 6 位
- 确认密码不一致时提示 `account.passwordMismatch`
- 注册时自动触发登录流程，无需二次登录

---

## 4. 导航结构

### 4.1 左侧导航 (`_Sidebar`)

| 模块 | 图标 | 说明 |
|------|------|------|
| 账号 | `Icons.person_outline` | 账号信息 + 额度总览 |
| 充值 | `Icons.payments_outlined` | 充值套餐选择 |
| 转换 | `Icons.sync_alt` | 上传 ZIP → 转换 DOCX |
| 转换记录 | `Icons.history` | DataTable 展示所有转换任务 |
| 充值记录 | `Icons.receipt_long` | DataTable 展示所有充值记录 |

**工作台（Dashboard）已移除**，符合需求。

### 4.2 紧凑模式（移动/平板）

在 `< 1040px` 宽度下，左侧导航隐藏，使用 `SegmentedButton` 在内容区顶部切换模块。

---

## 5. 右上角账号模块

### 5.1 组件结构

```dart
_AccountAvatarButton → PopupMenuButton<String>
├── CircleAvatar(initial: email[0].toUpperCase())
└── PopupMenuItem[]
    ├── enabled: false → 显示邮箱 + 套餐
    ├── 'profile' → _ProfileDialog
    ├── 'password' → _ChangePasswordDialog
    ├── divider
    └── 'logout' → onSignedOut()
```

### 5.2 账号详情弹窗 (`_ProfileDialog`)

- 居中大头像（72px CircleAvatar）
- 显示：邮箱、当前套餐、注册时间（`UserProfile.createdAt`）
- 确认按钮关闭

### 5.3 修改密码弹窗 (`_ChangePasswordDialog`)

- 三个密码输入框（当前密码、新密码、确认新密码），均带掩码切换
- 客户端校验：长度 ≥ 6 位、两次输入一致
- API 端点预留（`POST /auth/password`），当前为模拟成功响应

---

## 6. 独立记录模块

### 6.1 转换记录面板 (`ConvertRecordsPanel`)

- 启动时自动调用 `GET /conversions` 获取所有转换任务
- 刷新按钮手动重载
- **DataTable 展示**（相对于原来 `_RecordPreview` 的简单列表，信息密度更高）：

| 列 | 说明 |
|----|------|
| Job ID | 任务唯一标识 |
| Main File | 主 TeX 文件名 |
| Profile | 编译配置（jos 等） |
| Quality | 质量档位 |
| Status | 状态颜色芯片（completed=绿，failed=红，processing=蓝） |
| Created | 格式化时间 `YYYY-MM-DD HH:mm` |

### 6.2 充值记录面板 (`RechargeRecordsPanel`)

- 启动时自动调用 `GET /recharges` 获取所有充值记录
- DataTable 展示：

| 列 | 说明 |
|----|------|
| ID | 充值记录 ID |
| Type | 按次 / 按日期 |
| Package | 套餐名称 + 数量 |
| Amount | 金额（绿色正数） |
| Provider | 支付渠道 |
| Status | 状态 |
| Created | 创建时间 |

---

## 7. 国际化

### 7.1 新增键值（zh-CN / en-US）

```yaml
# 认证
auth.loginTab / auth.registerTab
auth.loginTitle / auth.registerTitle
auth.signInFirst

# 账号
account.confirmPassword
account.logout / account.changePassword / account.viewProfile
account.profileTitle / account.currentPlan / account.memberSince
account.quotaUsed / account.quotaRemaining
account.changePasswordTitle / account.oldPassword / account.newPassword / account.confirmNewPassword
account.passwordMismatch / account.passwordChanged / account.passwordChangeFailed / account.passwordTooShort
account.displayName

# 导航
nav.convertRecords / nav.rechargeRecords

# 通用
common.confirm / common.cancel
```

### 7.2 设计原则

- 所有用户可见文本提取到 `app_i18n.dart` 的 `_localized` 表中
- `AppStrings.t()` 方法在 key 不存在时回退到 `en-US`，永不崩溃
- 模板填充使用 `.fill({'key': value})` 方法

---

## 8. 设计令牌（Design Tokens）

继承现有 `app_tokens.dart` 和 `app_theme.dart` 的设计系统，无新增冲突：

| 令牌 | 值 | 用途 |
|------|-----|------|
| `AppSpacing.lg` | 24px | 卡片内边距、元素间距 |
| `AppRadius.md` | 8px | 输入框、按钮圆角 |
| `AppMotion.fast` | 120ms | 导航高亮过渡 |
| `AppBreakpoints.tablet` | 1040px | 紧凑模式断点 |

---

## 9. 组件清单

| 组件 | 文件 | 说明 |
|------|------|------|
| `AuthWindow` | `auth_window.dart` | 登录/注册 Tab 页容器 |
| `_LoginForm` | `auth_window.dart` | 登录表单 |
| `_RegisterForm` | `auth_window.dart` | 注册表单 |
| `_PasswordField` | `auth_window.dart` | 带掩码切换的密码输入框 |
| `ConvertRecordsPanel` | `convert_records_panel.dart` | 转换记录独立页面 |
| `_RecordsTable` | `convert_records_panel.dart` | 转换记录 DataTable |
| `_StatusChip` | `convert_records_panel.dart` | 状态颜色芯片 |
| `RechargeRecordsPanel` | `recharge_records_panel.dart` | 充值记录独立页面 |
| `_RechargeRecordsTable` | `recharge_records_panel.dart` | 充值记录 DataTable |
| `_AccountAvatarButton` | `workspace_app.dart` | 右上角账号头像按钮 |
| `_ProfileDialog` | `workspace_app.dart` | 账号详情弹窗 |
| `_ChangePasswordDialog` | `workspace_app.dart` | 修改密码弹窗 |
| `_ObscuredTextField` | `workspace_app.dart` | 带掩码切换的 TextField |
| `_AccountPanel` | `workspace_app.dart` | 账号总览面板 |
| `_AccountCard` | `workspace_app.dart` | 账号头像卡片 |
| `_MetricsRow` | `workspace_app.dart` | 额度指标行 |

---

## 10. 状态覆盖

每个模块均覆盖以下状态：

| 状态 | 展示方式 |
|------|----------|
| 正常 | 数据内容 |
| 加载中 | `LoadingState` + `CircularProgressIndicator` |
| 空数据 | `EmptyState` |
| 错误 | `ErrorState` + 红色提示 |
| 未登录 | 路由到 `AuthWindow`（全局守卫） |

---

## 11. 构建与运行

```powershell
# 方式一：使用脚本（推荐）
.\scripts\run_flutter_desk.ps1          # 构建 + 运行
.\scripts\run_flutter_desk.ps1 -SkipBuild  # 仅运行已有构建

# 方式二：手动
cd flutter_app
flutter pub get
flutter build windows --release
# exe 位于: build/windows/x64/runner/Release/doc_engine.exe
```

---

## 12. 已知限制与后续优化

1. **修改密码 API** — 当前为模拟成功响应，需对接 `POST /auth/password` 端点
2. **Token 持久化** — auth token 目前存于内存，刷新页面会丢失；建议后续存入 `AppPreferences`（支持 `localStorage` / JSON 文件）
3. **退出登录清除状态** — 目前仅清内存状态，未清除持久化 token（待 Token 持久化实现后补全）
4. **Slint 版本** — 同步参考 `docs-zh/slint/` 中的 Slint 设计方案，后续可迁移到原生桌面 UI

---

*文档由 Cursor AI 基于 `commercial-ui-design` 技能自动生成*
