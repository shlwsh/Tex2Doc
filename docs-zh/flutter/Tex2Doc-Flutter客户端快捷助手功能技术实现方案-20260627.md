# Tex2Doc Flutter 客户端快捷助手功能技术实现方案

> 更新日期：2026-06-27  
> 适用范围：`flutter_app` 用户端 Web / Desktop  
> 参考对象：`apps/slint-user` 快捷助手模块与 `docs-zh/slint/Tex2Doc-Slint桌面端快捷助手与本地转换额度管理设计方案-20260626.md`

## 1. 背景与目标

Slint 用户端已经提供“快捷助手”入口：应用默认进入免登录模式，用户输入兑换码后自动完成临时账号登录/注册、兑换码激活、额度同步，然后可在同一界面发起本地快捷转换或云端专业转换。Flutter 用户端当前仍是未登录即进入 `AuthWindow` 的硬门禁，转换页以云端 ZIP 上传为主，缺少免登录快捷入口、影子账号激活、本地转换额度校验与快捷/会员双模式切换。

本方案目标是在 Flutter 客户端补齐与 Slint 客户端同等的快捷助手能力，并尽量复用现有商业 API、兑换码、转换桥接和 UI 组件：

1. 用户端默认进入“快捷助手”，不要求先注册或登录。
2. 快捷助手支持兑换码激活、购买卡片跳转、激活状态恢复。
3. 兑换码采用影子账号方案：`email = code`、`password = code`，登录失败则注册，再兑换并同步用量。
4. 快捷助手内提供 ZIP、本地主 TeX、profile、quality、快捷/专业模式和转换按钮。
5. 本地快捷转换接入 `/v1/local-conversions/check` 与 `/v1/local-conversions/consume`，确保生成 DOCX 前后有额度控制。
6. 会员中心保持现有账号、充值、转换记录、反馈等完整工作台。

## 2. 现状核查

### 2.1 Slint 端可参考能力

Slint 端相关实现位置：

| 能力 | 位置 | 说明 |
|---|---|---|
| 快捷/会员双模式状态 | `apps/slint-user/src/ui/main.slint` | `is-quick-mode` 默认 `true`，顶部 Tab 切换“快捷助手 / 会员中心”。 |
| 快捷助手 UI | `apps/slint-user/src/ui/main.slint` | 包含标题、兑换码激活、购买卡片、文件输入、profile/quality、快捷/专业模式、日志入口。 |
| 激活回调 | `apps/slint-user/src/main.rs` | `quick-activate-clicked` 中执行 code 登录、失败注册、兑换、同步 usage。 |
| 兑换码持久化 | `apps/slint-user/src/settings.rs`、`src/main.rs` | 保存 `last_redeem_code`，启动后恢复输入与激活态。 |
| 购买卡片 URL | `apps/slint-user/src/main.rs` | `https://pay.ldxp.cn/item/ns8i2g`。 |

需要注意：`apps/slint-user/src/cloud_convert.rs` 当前代码注释与行为仍是“本地转换不需要云端配额”，而 2026-06-26 的设计文档要求接入本地转换额度 check/consume。Flutter 实现建议按产品目标接入额度管理，同时后续让 Slint 端也收敛到同一计费口径。

### 2.2 Flutter 端现状

Flutter 端已有基础：

| 能力 | 位置 | 当前状态 |
|---|---|---|
| 用户端壳层 | `flutter_app/lib/shared/workspace_app.dart` | `_WorkspaceShell` 未登录时直接显示 `AuthWindow`。 |
| 登录/注册 | `flutter_app/lib/ui/auth_window.dart`、`commercial_api.dart` | 已有 `login` / `register`。 |
| 兑换码 | `commercial_api.dart`、`_RechargePanel` | 已有 `redeemCode`、`redeemCodeRecords`、购买链接跳转。 |
| 用量 | `commercial_api.dart`、`_AccountPanel` | 已有 `usage` 与 `UsageSummary`。 |
| 转换桥接 | `bridge.dart`、`bridge_web.dart`、`bridge_stub.dart` | 已可通过 WASM/native 返回 DOCX 字节。 |
| 云端转换 | `_ConvertPanel` | 已有 ZIP 上传、创建 job、轮询、下载 DOCX。 |
| 偏好持久化 | `ui/app_preferences*.dart` | 支持桌面 JSON 文件与 Web localStorage。 |
| i18n | `ui/app_i18n.dart` | 中英文 key-value 表。 |

主要缺口：

1. 缺少快捷助手默认首页与双模式入口。
2. 缺少影子账号激活服务与持久化恢复。
3. `CommercialApiClient` 缺少 `checkLocalConversion` / `consumeLocalConversion` Dart 方法和响应模型。
4. `_ConvertPanel` 内的转换表单与云端逻辑耦合，尚未抽成快捷助手可复用控件。
5. 桌面端 `downloadBlob` 当前写系统临时目录并打开，快捷助手若要“保存到指定输出路径”，需新增保存文件/目录选择能力；Web 端继续走浏览器下载。

## 3. GitNexus 影响分析

本方案尚不修改代码，但核心实现会触达 `_WorkspaceShell`，已对该符号执行 GitNexus upstream impact：

| 目标符号 | 风险 | 直接影响 | 二级影响 | 受影响流程 |
|---|---:|---:|---:|---|
| `_WorkspaceShell` | MEDIUM | 5 | 3 | 0 |

直接影响文件包括 `flutter_app/lib/workspace_app.dart`、`flutter_app/lib/user/user_app.dart`、`flutter_app/lib/product/product_home_app.dart`、`flutter_app/lib/admin/admin_app.dart`、`flutter_app/lib/admin/pages/redeem/admin_redeem_pages.dart`；二级入口包括 `main_user.dart`、`main_admin.dart`、`main.dart`。实现时应保持 admin 模式不进入快捷助手门禁，并为 user 模式单独放开默认快捷入口。

GitNexus 索引状态：索引提交为 `2017eaf`，当前提交为 `193d673`，状态 stale；正式编码前建议执行 `node .gitnexus/run.cjs analyze` 刷新索引。

## 4. 产品交互设计

### 4.1 顶层模式

Flutter 用户端 `DocEngineAppMode.user` 建议采用双模式：

| 模式 | 默认 | 登录要求 | 主要内容 |
|---|---|---|---|
| 快捷助手 | 是 | 不要求手工登录；兑换码激活后后台创建/登录影子账号 | 激活卡片、本地快捷转换、云端专业转换入口、额度摘要。 |
| 会员中心 | 否 | 需要真实账号登录 | 账号、充值、转换、转换记录、充值记录、反馈、关于。 |

桌面宽屏建议保留左侧品牌栏，主区域顶部使用 `SegmentedButton` 或 Tab 切换“快捷助手 / 会员中心”。移动/窄屏时把模式切换放在 TopBar 下方。

### 4.2 快捷助手首屏布局

建议新增 `QuickAssistantPanel`，首屏结构：

1. 标题区：Tex2Doc 快捷助手、免登录模式、本地安全转换。
2. 激活区：兑换码输入框、激活按钮、购买卡片按钮、激活状态。
3. 额度摘要：按次余额、有效期、预览额度/云端额度、当前账号类型。
4. 转换区：选择 ZIP、主 TeX、profile、quality、快捷/专业模式、开始转换、下载/保存结果、日志。
5. 异常提示：未激活、额度不足、API 不可用、转换失败。

### 4.3 快捷/专业转换

| 模式 | 引擎 | 消耗 | 输出 |
|---|---|---|---|
| 快捷版 | `DocEngineFacade.convertZipToDocx` 本地 WASM/native | 调用 local check/consume | 桌面保存到用户路径或打开临时文件；Web 下载 DOCX。 |
| 专业版 | 复用 `_ConvertPanel._convertCloud` 的商业 API job | 云端 conversion quota | 下载 DOCX，并可进入转换记录查看详情。 |

专业版应在未激活或未登录时禁用。快捷版也应要求激活成功，因为本地转换要使用影子账号 token 做额度 check/consume。

## 5. 技术方案

### 5.1 状态模型

建议新增：

```dart
enum _WorkspaceMode { quick, member }

class _QuickSession {
  final String apiBaseUrl;
  final String accessToken;
  final UserProfile profile;
  final String redeemCode;
  final UsageSummary? usage;
}
```

`_WorkspaceShellState` 增加：

```dart
_WorkspaceMode _workspaceMode = _WorkspaceMode.quick;
_QuickSession? _quickSession;
String? _quickActivationStatus;
bool _quickBusy = false;
```

`_auth` 继续代表会员中心真实账号；`_quickSession` 代表兑换码影子账号。两者可以共存，避免用户切换会员中心后覆盖快捷助手激活态。

### 5.2 持久化 Key

复用 `AppPreferences`，新增以下 key：

| Key | 值 | 用途 |
|---|---|---|
| `quick.redeemCode` | 兑换码 | 启动时尝试恢复激活。 |
| `quick.apiBaseUrl` | API 地址 | 默认 `defaultCommercialApiBaseUrl`。 |
| `quick.accessToken` | access token，可选 | 短期恢复；过期后用 code 重新登录。 |
| `quick.profileEmail` | 影子账号邮箱 | 展示与排错。 |
| `quick.lastMainTex` | 主 TeX | 提升重复转换体验。 |
| `quick.lastProfile` | profile | 同上。 |
| `quick.lastQuality` | quality | 同上。 |

安全性建议：当前 Flutter 偏好文件是普通 JSON/localStorage，不适合长期保存 refresh token；快捷助手第一期可只保存兑换码并在启动时重新登录。后续桌面端可引入 `flutter_secure_storage` 或平台 keychain。

### 5.3 API 客户端扩展

在 `flutter_app/lib/commercial_api.dart` 增加：

```dart
Future<LocalQuotaCheckResponse> checkLocalConversion(String accessToken) async {
  return LocalQuotaCheckResponse.fromJson(
    await _postJson('local-conversions/check', const {}, accessToken: accessToken),
  );
}

Future<LocalQuotaConsumeResponse> consumeLocalConversion(String accessToken) async {
  return LocalQuotaConsumeResponse.fromJson(
    await _postJson('local-conversions/consume', const {}, accessToken: accessToken),
  );
}
```

并补充模型：

```dart
class LocalQuotaCheckResponse {
  final bool allowed;
  final bool validUntilActive;
  final int countBalance;
  final int used;
  final int limit;
}

class LocalQuotaConsumeResponse {
  final bool consumed;
  final int balance;
}
```

服务端端点当前已存在：`/v1/local-conversions/check` 与 `/v1/local-conversions/consume`。

### 5.4 快捷激活服务

建议新增 `flutter_app/lib/ui/quick_activation.dart` 或放入 `shared/workspace_app.dart` 第一阶段内联，后续再拆：

流程：

1. 校验 code 非空，标准化大小写与空格。
2. `CommercialApiClient(apiBaseUrl).login(email: code, password: code)`。
3. 若登录返回 401/404，则调用 `register(email: code, password: code, displayName: 'Quick $code')`。
4. 登录/注册成功后调用 `redeemCode(accessToken, code)`。
5. 若兑换接口返回“已兑换/冲突”，但登录成功且 usage 可读取，则视为恢复成功；具体按 `CommercialApiException.statusCode == 409` 或服务端错误码判断。
6. 调用 `usage(accessToken)`，生成 `_QuickSession`。
7. 写入 `quick.redeemCode`、`quick.apiBaseUrl`，刷新 UI。

启动恢复：

1. `_WorkspaceShellState.initState` 读取 `quick.redeemCode`。
2. 有 code 则后台执行同一激活流程，但 UI 状态显示“正在恢复激活状态”。
3. 恢复失败不阻塞快捷助手，只显示未激活和错误信息。

### 5.5 转换表单抽取

建议从 `_ConvertPanel` 抽出一个通用控件：

```dart
class ConversionForm extends StatefulWidget {
  final bool cloudEnabled;
  final bool localEnabled;
  final Future<Uint8List> Function(Uint8List bytes, String fileName, String mainTex, String profile, String quality) onLocalConvert;
  final Future<Uint8List> Function(Uint8List bytes, String fileName, String mainTex, String profile, String quality)? onCloudConvert;
}
```

复用内容：

1. ZIP 选择与 10 MB 限制。
2. 主 TeX 输入。
3. profile 与 quality 下拉。
4. 日志列表。
5. DOCX 下载。

会员中心的 `_ConvertPanel` 使用 `cloudEnabled: true`，快捷助手使用 `localEnabled: true` 和可选 `cloudEnabled: true`。这样可避免同一转换 UI 在两个页面内重复维护。

### 5.6 本地快捷转换额度链路

快捷版转换必须使用如下顺序：

1. 读取 `_quickSession.accessToken`，无 session 则提示先激活。
2. 调用 `checkLocalConversion(accessToken)`。
3. 若 `allowed == false`，显示额度不足并引导购买卡片。
4. 调用 `DocEngineFacade.convertZipToDocx(zipBytes, mainTex)` 生成 DOCX 字节，暂存于内存。
5. 调用 `consumeLocalConversion(accessToken)` 扣减额度。
6. 扣减成功后才允许 `downloadBlob(docx, filename)` 或桌面保存到用户选择路径。
7. 调用 `usage(accessToken)` 刷新额度摘要。

关键异常策略：

| 场景 | 行为 |
|---|---|
| check 失败 | 不启动转换，提示 API/网络异常。 |
| check 不允许 | 不启动转换，提示额度不足，展示购买卡片按钮。 |
| 本地转换失败 | 不 consume，展示转换错误。 |
| consume 失败 | 不交付 DOCX；Web 端不触发下载，桌面端不保存到用户路径。 |
| consume 成功但刷新 usage 失败 | 仍交付 DOCX，状态提示“已扣费，额度刷新失败”。 |

当前 `downloadBlob` 在桌面端会写系统临时目录并打开。为严格满足“扣费成功才交付成品”，必须确保 `downloadBlob` 只在 consume 成功后调用；若后续支持“选择输出路径”，建议新增 `saveDocxFile(bytes, suggestedName)`。

### 5.7 顶层壳层改造

`_WorkspaceShell.build` 当前在 `_auth == null` 时直接返回 `AuthWindow`。改造建议：

1. 仅 `DocEngineAppMode.admin` 保留硬登录门禁。
2. `DocEngineAppMode.user` 始终进入主壳层。
3. 主壳层根据 `_workspaceMode` 显示：
   - quick：`QuickAssistantPanel`。
   - member：如果 `_auth == null` 显示嵌入式 `AuthWindow` 或 `MemberSignInPanel`；登录后显示现有 sidebar + `_NavContent`。
4. `_TopBar` 的 profile 参数改为可空，未登录时显示“游客/未登录”，并提供进入会员中心登录的按钮。

### 5.8 i18n Key

新增中英文 key：

| Key | 中文 |
|---|---|
| `quick.title` | Tex2Doc 快捷助手 |
| `quick.subtitle` | 免登录模式，本地安全文档转换 |
| `quick.activateTitle` | 激活此模块以开始使用 |
| `quick.codeHint` | 请输入激活兑换码 |
| `quick.activate` | 激活当前模式 |
| `quick.buyCode` | 购买卡片 |
| `quick.notActivated` | 请先输入兑换码激活快捷助手 |
| `quick.activated` | 已激活，可用额度 {remaining} |
| `quick.restoring` | 正在恢复激活状态... |
| `quick.localMode` | 快捷版 |
| `quick.cloudMode` | 专业版 |
| `quick.quotaExhausted` | 额度不足，请购买或更换兑换码 |
| `nav.quickAssistant` | 快捷助手 |
| `nav.memberCenter` | 会员中心 |

### 5.9 文件拆分建议

| 文件 | 动作 | 内容 |
|---|---|---|
| `flutter_app/lib/shared/workspace_app.dart` | 修改 | 顶层双模式、session 状态、路由编排。 |
| `flutter_app/lib/ui/quick_assistant_panel.dart` | 新增 | 快捷助手 UI。 |
| `flutter_app/lib/ui/conversion_form.dart` | 新增 | 可复用 ZIP 转换表单。 |
| `flutter_app/lib/ui/quick_activation.dart` | 新增 | 影子账号激活流程。 |
| `flutter_app/lib/commercial_api.dart` | 修改 | local quota API 与模型。 |
| `flutter_app/lib/ui/app_i18n.dart` | 修改 | 快捷助手中英文文案。 |
| `flutter_app/lib/file_web_utils_io.dart` | 可选修改 | 桌面保存 DOCX 到用户指定路径。 |

## 6. 实施计划

### Phase 1：最小可用快捷助手

1. 为 `commercial_api.dart` 增加 local quota 方法与模型。
2. 新增 `QuickActivationService`，实现 code 登录/注册/兑换/usage。
3. 改造 user 模式壳层，默认显示 `QuickAssistantPanel`，保留会员中心入口。
4. `QuickAssistantPanel` 内复用现有 ZIP 选择和 `DocEngineFacade.convertZipToDocx`，先完成本地快捷转换。
5. 接入 check/consume，成功后下载 DOCX。

### Phase 2：复用化与会员中心一致性

1. 抽取 `ConversionForm`，让快捷助手和 `_ConvertPanel` 共用。
2. 快捷助手增加专业版云端转换。
3. 展示 usage 摘要、兑换记录入口、转换日志。
4. 启动时自动恢复上次兑换码激活态。

### Phase 3：桌面体验完善

1. 桌面端新增输出路径选择和安全保存。
2. 本地转换报告 JSON 下载/保存。
3. 引入安全存储保存 refresh token 或短期 session。
4. 与 Slint 端统一“本地转换是否消耗额度”的最终产品口径。

## 7. 测试与验收

### 单元测试

1. `CommercialApiClient.checkLocalConversion` / `consumeLocalConversion` JSON 解析。
2. `QuickActivationService`：
   - 登录成功后兑换。
   - 登录失败后注册再兑换。
   - 兑换码已使用但 usage 可读时恢复成功。
   - 网络失败时不写入激活态。
3. 本地转换流程：
   - check 不允许时不调用 engine。
   - engine 失败时不 consume。
   - consume 失败时不 download。

### Widget 测试

1. 未登录启动默认显示快捷助手。
2. 会员中心未登录时只显示登录面板，不影响快捷助手。
3. 激活成功后转换按钮可用，额度摘要刷新。
4. 额度不足时显示购买卡片按钮。

### 集成验收

1. Web 用户端：选择 ZIP -> 激活 -> 快捷转换 -> 浏览器下载 DOCX。
2. Desktop 用户端：选择 ZIP -> 激活 -> 快捷转换 -> 打开/保存 DOCX。
3. 切到会员中心登录真实账号，账号、充值、转换记录功能保持可用。
4. 刷新页面或重启 App 后，快捷助手自动恢复上次 code。

## 8. 风险与注意事项

| 风险 | 等级 | 处理 |
|---|---|---|
| `_WorkspaceShell` 改造影响 user/admin 共用壳层 | 中 | admin 模式保留原登录门禁；user 模式单独分支；补 widget 测试。 |
| 兑换码作为账号密码保存安全性弱 | 中 | 第一期只保存 code；后续引入安全存储。 |
| Slint 与 Flutter 本地额度口径不一致 | 中 | Flutter 按 check/consume 实现；同步修正 Slint 端行为或产品说明。 |
| Web 本地转换文件较大时性能不足 | 中 | 保持 10 MB 限制，后续做 worker/streaming。 |
| consume 成功后下载失败 | 低 | 提供重新下载当前内存 DOCX 的按钮，并保留操作日志。 |

## 9. 验收标准

1. Flutter 用户端无需登录即可看到快捷助手。
2. 输入有效兑换码后可以自动激活，且重启/刷新后可恢复。
3. 激活后可完成至少一次本地快捷转换，并在扣费成功后下载 DOCX。
4. 额度不足时不会启动本地转换，也不会交付 DOCX。
5. 会员中心原有登录、充值、云端转换、记录、反馈能力不回退。
6. `flutter analyze` 与核心 widget/unit 测试通过。
