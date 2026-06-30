# Tex2Doc Flutter 模块文档索引

> 更新日期：2026-06-29
> 适用范围：`flutter_app` Flutter Web / Desktop 用户端、Flutter Web 管理端
> 文档依据：当前仓库实现、GitNexus 索引、`flutter_app/lib/**` 源码

## 1. 模块入口

当前 Flutter 仍是一个工程承载多入口，而不是物理拆分成两个独立 App：

| 入口 | 目标模块 | 说明 |
|---|---|---|
| `flutter_app/lib/main.dart` | Web 总入口 | Web 下按 URL path 分流：`/admin` 进入管理端，`/app` 进入用户端，`/extension` 进入插件页，其余进入产品首页；非 Web 默认进入用户端。 |
| `flutter_app/lib/main_user.dart` | 客户端 | 直接启动 `UserApp(isWeb: kIsWeb)`，进入用户端快捷助手/会员中心。 |
| `flutter_app/lib/main_admin.dart` | Web 管理端 | 直接启动 `AdminApp(isWeb: true)`，进入管理端登录门禁。 |
| `flutter_app/lib/shared/workspace_app.dart` | 共享应用壳 | `DocEngineApp`、模式切换、登录状态、导航、账号、充值、转换等核心工作台。 |
| `flutter_app/lib/commercial_api.dart` | 商业 API 客户端 | 登录、用量、充值、兑换码、转换、反馈、发布、审计、自动化等 REST API 封装。 |

默认 API 地址由 `defaultCommercialApiBaseUrl` 决定：Web 在 HTTP/HTTPS 环境下使用当前站点的 `/v1/`，桌面/本地默认使用 `http://127.0.0.1:2624/v1/`。

## 2. 文档清单

| 文档 | 内容 |
|---|---|
| [Tex2Doc-Flutter-客户端功能说明-20260629.md](./Tex2Doc-Flutter-客户端功能说明-20260629.md) | 用户端快捷助手、会员中心、账号、充值、转换、记录、反馈、平台差异和使用说明。 |
| [Tex2Doc-Flutter-WEB管理端功能说明-20260629.md](./Tex2Doc-Flutter-WEB管理端功能说明-20260629.md) | Web 管理端登录门禁、仪表盘、兑换码运营、反馈、发布、审计、自动化研发面板和操作流程。 |
| [Tex2Doc-Flutter客户端快捷助手功能技术实现方案-20260627.md](./Tex2Doc-Flutter客户端快捷助手功能技术实现方案-20260627.md) | 快捷助手的设计/技术方案文档。 |
| [flutter-desktop-ui-design-20260623.md](./flutter-desktop-ui-design-20260623.md) | 早期 Flutter 桌面商业化 UI 设计说明。 |

## 3. 本地运行与构建

开发调试：

```powershell
cd flutter_app
flutter pub get

# 用户端
flutter run -d chrome --target lib/main_user.dart

# Web 管理端
flutter run -d chrome --target lib/main_admin.dart

# 总入口，按 URL path 分流
flutter run -d chrome --target lib/main.dart
```

静态发布构建：

```powershell
# 构建 home / user / admin 三套静态资源到 apps/rust-service/static
pwsh -NoProfile -File scripts/build_flutter_static_release.ps1
```

项目脚本：

```powershell
npm run build:wasm
npm run build:web
npm run build:all
npm run serve:web
npm run e2e:flutter-web
```

## 4. 公共能力

两端共享以下基础能力：

| 能力 | 当前实现 |
|---|---|
| 主题与语言 | `AppPreferences` 持久化 `ui.theme` / `ui.locale`，支持浅色、深色、默认色调和 `zh-CN` / `en-US`。 |
| 认证 | `AuthWindow` 提供登录/注册 Tab；注册校验密码长度和确认密码；登录后持有内存态 access token。 |
| 文件选择 | Web 通过 JS 事件桥接隐藏 input；桌面通过 `file_picker`。 |
| 文件下载 | Web 触发浏览器 Blob 下载；桌面写入系统临时目录并调用系统默认程序打开。 |
| 本地转换桥接 | Web 通过 WASM，非 Web 通过 native bridge；统一由 `DocEngineFacade` 对外暴露。 |

## 5. 当前注意事项

1. `AuthWindow` 登录态目前主要保存在内存，刷新 Web 页面后需要重新登录；快捷助手会保存兑换码并尝试恢复激活。
2. 用户端“修改密码”弹窗当前是前端模拟成功，尚未真正调用密码修改 API。
3. 快捷助手的“购买卡片”按钮当前是预留按钮；会员中心充值页的购买卡片已调用外部支付链接。
4. 转换记录页的 DOCX/ZIP/LOG 按钮当前会请求后端字节并显示提示，但未统一调用 `downloadBlob` 保存到浏览器/本地文件。
5. 管理端自动化页的 Auto-refresh 按钮当前只切换状态图标，未实现定时刷新任务。
