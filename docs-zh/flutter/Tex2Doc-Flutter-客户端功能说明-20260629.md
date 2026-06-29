# Tex2Doc Flutter 客户端功能说明

> 更新日期：2026-06-29
> 适用模块：`flutter_app` 用户端 Web / Desktop
> 入口文件：`flutter_app/lib/main_user.dart`、`flutter_app/lib/user/user_app.dart`、`flutter_app/lib/shared/workspace_app.dart`

## 1. 客户端定位

Flutter 客户端面向普通用户，承载两类使用路径：

| 模式 | 登录要求 | 主要用途 |
|---|---|---|
| 快捷助手 | 不要求用户手工注册登录；输入兑换码后自动创建/登录影子账号 | 用兑换码快速激活额度，选择 ZIP 项目，本地或云端转换 DOCX。 |
| 会员中心 | 需要邮箱账号登录或注册 | 查看账号与额度，兑换充值码，发起云端转换，查看转换/充值记录，提交反馈。 |

启动链路：

```text
main_user.dart
  -> UserApp(isWeb)
  -> DocEngineApp(mode: user)
  -> _WorkspaceShell
  -> 快捷助手 / 会员中心
```

用户端默认进入“快捷助手”。切换到“会员中心”后，如果还没有登录，会显示 `AuthWindow` 登录/注册窗口；登录成功后才显示左侧导航或紧凑 Tab。

## 2. 顶层界面与通用操作

### 2.1 模式切换

页面顶部使用 `SegmentedButton` 切换：

| 选项 | 说明 |
|---|---|
| 快捷助手 | 默认页，适合已有兑换码的用户快速转换。 |
| 会员中心 | 登录后进入完整工作台，包含账号、充值、转换、记录、反馈、关于。 |

### 2.2 主题与语言

登录后的 TopBar 提供：

1. 主题下拉：默认、深色等色调。
2. 语言下拉：`zh-CN` / `en-US`。
3. 账号头像菜单：查看详情、修改密码、退出登录。

主题与语言会通过 `AppPreferences` 保存，刷新或重启后恢复。

### 2.3 API 地址

登录/注册表单允许填写 API Base URL。默认规则：

| 运行环境 | 默认地址 |
|---|---|
| Web | 当前站点根路径解析出的 `/v1/`，例如 `https://example.com/v1/`。 |
| 桌面/本地 | `http://127.0.0.1:2624/v1/`。 |

## 3. 快捷助手

### 3.1 功能介绍

快捷助手由 `QuickAssistantPanel`、`QuickActivationService`、`QuickSession` 组成，提供：

1. 兑换码激活。
2. 影子账号登录/注册。
3. 额度摘要展示。
4. ZIP 项目选择。
5. 快捷版本地转换。
6. 专业版云端转换。
7. 转换日志和 DOCX 下载。

激活后的会话包含 `apiBaseUrl`、`accessToken`、用户资料、兑换码和用量摘要。用户端会把兑换码保存到 `quick.redeemCode`，下次启动时自动尝试恢复激活。

### 3.2 激活流程

操作步骤：

1. 打开用户端，停留在“快捷助手”。
2. 在兑换码输入框输入兑换码。
3. 点击“激活当前模式”。
4. 激活成功后，页面显示可用次数和有效期。

内部流程：

```text
输入兑换码
  -> 使用 code 作为 email/password 调用 login
  -> 如果 login 返回 401/404，则调用 register 创建影子账号
  -> 调用 redeem-codes/redeem 兑换额度
  -> 如果已兑换返回 409，登录成功场景视为可恢复
  -> 调用 usage 获取额度
  -> 生成 QuickSession 并保存 quick.redeemCode
```

状态说明：

| 状态 | 含义 |
|---|---|
| `idle` | 没有可恢复的兑换码，等待用户输入。 |
| `restoring` | 启动时正在使用已保存兑换码恢复会话。 |
| `activating` | 正在登录/注册/兑换/读取用量。 |
| `activated` | 已激活，可以转换。 |
| `error` | 激活失败，页面显示错误信息。 |

### 3.3 快捷版本地转换

适合希望在浏览器 WASM 或桌面 native 引擎中快速转换的用户。

操作步骤：

1. 激活快捷助手。
2. 转换模式选择“快捷版”。
3. 点击“上传”，选择 `.zip` 项目包。
4. 选择 Profile：`JOS` 或 `Standard`。
5. 选择 Quality：`High` 或 `Medium`。
6. 填写主 TeX 文件名，默认 `main.tex`。
7. 点击“转换”。
8. 转换成功后自动触发 DOCX 下载，并可点击下载按钮再次下载。

内部流程：

```text
检查本地转换额度 local-conversions/check
  -> 额度允许
  -> DocEngineFacade.convertZipToDocx(zip, mainTex)
  -> 扣减额度 local-conversions/consume
  -> downloadBlob 下载 DOCX
  -> usage 刷新额度摘要
```

限制与策略：

| 项目 | 说明 |
|---|---|
| 文件大小 | ZIP 大于等于 10 MB 会被前端拒绝。 |
| 未激活 | 上传和转换按钮禁用。 |
| 额度不足 | 不启动转换，提示购买或更换兑换码。 |
| 转换失败 | 不扣减额度。 |
| 扣减失败 | 不交付 DOCX，提示重试。 |

### 3.4 专业版云端转换

适合需要服务端队列、质量流程和转换记录追踪的用户。

操作步骤：

1. 激活快捷助手。
2. 转换模式选择“专业版”。
3. 上传 ZIP 项目包。
4. 填写主 TeX、Profile 和 Quality。
5. 点击“转换”。
6. 等待云端任务完成后下载 DOCX。

内部流程：

```text
uploads 上传 ZIP
  -> conversions 创建任务
  -> 每 1 秒轮询 conversions/{jobId}
  -> completed 后下载 conversions/{jobId}/download/docx
  -> usage 刷新云端转换用量
```

轮询最多 120 次。若任务状态为 `failed` 或 `expired`，页面显示后端返回的错误码或错误信息。

### 3.5 日志

快捷助手下方显示操作日志，包括：

1. 文件选择结果。
2. 额度检查结果。
3. 本地转换耗时。
4. 云端上传、任务创建、轮询状态。
5. 下载与用量刷新结果。
6. 异常错误。

日志最多保留 100 条，可点击 Clear 清空。

## 4. 会员中心

会员中心通过 `AuthWindow` 登录后显示完整工作台。用户端可用导航包括：

| 导航 | 功能 |
|---|---|
| 账号 | 查看邮箱、套餐、云端转换额度、按次余额、日期有效期。 |
| 充值 | 打开购买链接、兑换充值码、查看兑换记录和套餐说明。 |
| 转换 | 上传 ZIP 发起云端转换并下载 DOCX。 |
| 转换记录 | 查看历史转换任务、状态和相关文件入口。 |
| 充值记录 | 查看历史充值记录。 |
| 反馈 | 提交问题或需求，查看会话并继续回复。 |
| 关于 | 查看产品说明、目标和核心能力。 |

### 4.1 登录与注册

操作步骤：

1. 切换到“会员中心”。
2. 在登录窗口填写 API Base URL、邮箱、密码。
3. 点击登录。
4. 如没有账号，切换到注册 Tab，填写邮箱、密码、确认密码。

注册规则：

| 校验 | 说明 |
|---|---|
| 密码长度 | 至少 6 位。 |
| 确认密码 | 必须与密码一致。 |
| 账号已存在 | 返回冲突时切回登录页并提示使用已有账号。 |

登录成功后 `_WorkspaceShell` 会保存内存态 `_auth`，包含 API 地址、access token 和用户资料。当前普通登录 token 未做持久化，刷新 Web 页面后需要重新登录。

### 4.2 账号页

账号页展示：

1. 邮箱头像和邮箱。
2. 当前套餐 `planId`。
3. 可选显示名 `displayName`。
4. 云端转换用量：`cloudConversionsUsed / cloudConversionsLimit`。
5. 按次余额 `countBalance`。
6. 日期有效期 `dateValidUntil`。

右上角账号菜单：

| 操作 | 说明 |
|---|---|
| 查看详情 | 弹窗显示邮箱、当前套餐、显示名。 |
| 修改密码 | 弹窗校验旧密码、新密码和确认密码；当前实现为前端模拟成功。 |
| 退出登录 | 清空当前内存登录态并回到会员中心登录窗口。 |

### 4.3 充值页

功能：

1. 展示购买卡片，点击后打开 `https://pay.ldxp.cn/item/ns8i2g`。
2. 加载兑换码选项和套餐列表。
3. 输入兑换码并提交。
4. 展示兑换成功后的套餐、数量和余额。
5. 展示用户兑换记录。

操作步骤：

1. 进入“充值”。
2. 可点击“购买卡片”跳转外部购买页。
3. 在兑换码输入框输入卡密。
4. 点击输入框右侧确认按钮或“提交兑换码”。
5. 成功后页面刷新兑换记录和余额提示。

### 4.4 转换页

会员中心的“转换”页只走云端转换。

操作步骤：

1. 进入“转换”。
2. 点击上传，选择 ZIP 项目包。
3. 确认主 TeX 文件名，默认 `main-jos.tex`。
4. 点击“转换”。
5. 页面显示上传、创建任务、轮询日志。
6. 成功后点击下载按钮下载 DOCX。

固定参数：

| 参数 | 当前值 |
|---|---|
| Profile | `jos` |
| Quality | `high` |
| 轮询间隔 | 1 秒 |
| 最大轮询 | 120 次 |
| 文件大小限制 | ZIP 小于 10 MB |

### 4.5 转换记录

转换记录页启动时调用 `GET /conversions`，并支持刷新。

表格字段：

| 字段 | 说明 |
|---|---|
| Job ID | 转换任务 ID。 |
| Main File | 主 TeX 文件。 |
| Profile | 编译配置。 |
| Quality | 质量档位。 |
| Status | 任务状态，完成为绿色，失败为红色，处理中为蓝色。 |
| Created | 创建时间。 |
| Files | DOCX、ZIP、LOG 文件入口。 |

注意：当前文件按钮会请求后端下载接口并显示 SnackBar，但没有统一调用 `downloadBlob` 保存文件。

### 4.6 充值记录

充值记录页调用 `GET /recharges`，展示：

| 字段 | 说明 |
|---|---|
| ID | 充值记录 ID。 |
| Type | 充值类型。 |
| Package | 套餐 ID 和数量。 |
| Amount | 金额，按元展示。 |
| Provider | 支付或兑换渠道。 |
| Status | 状态。 |
| Created | 创建日期。 |

### 4.7 反馈

反馈模块提供用户与客服/运营之间的会话。

反馈列表：

1. 自动加载 `GET /feedback/threads`。
2. 支持刷新。
3. 显示类型、优先级、状态、标题、消息数、创建时间、关联转换任务。

创建反馈：

1. 点击 New Feedback。
2. 填写标题，最多 100 字符。
3. 选择类型：`issue` 或 `requirement`。
4. 选择优先级：`low`、`normal`、`high`、`urgent`。
5. 可选择关联转换任务。
6. 填写描述并提交。

会话详情：

1. 点击反馈卡片进入详情页。
2. 查看消息气泡、系统消息和时间。
3. 如果线程未关闭，可在底部输入回复并发送。
4. 下拉刷新或点击刷新可重新加载。

## 5. 平台差异

| 能力 | Web | Desktop |
|---|---|---|
| 文件选择 | JS 事件触发隐藏 `<input type=file>`，回传 base64。 | `file_picker` 打开系统文件选择器。 |
| DOCX 下载 | 创建 Blob URL，触发浏览器下载。 | 写入系统临时目录并用默认程序打开。 |
| 本地转换 | WASM 桥接。 | Native bridge，调用本地 Rust 动态库。 |
| 外部链接 | `window.open(url, '_blank')`。 | Windows 使用 `cmd /c start`，macOS 使用 `open`，Linux 使用 `xdg-open`。 |

## 6. API 对照

| 功能 | API 封装 |
|---|---|
| 登录/注册 | `login`、`register` |
| 用量 | `usage` |
| 充值记录 | `recharges` |
| 兑换码选项/兑换/记录 | `redeemCodeOptions`、`redeemCode`、`redeemCodeRecords` |
| 上传与云端转换 | `uploadProjectZip`、`createConversion`、`getConversion`、`downloadConversionDocx` |
| 本地转换额度 | `checkLocalConversion`、`consumeLocalConversion` |
| 转换记录与文件 | `conversions`、`downloadConversionDocx`、`downloadConversionZip`、`downloadConversionLog` |
| 反馈 | `feedbackThreads`、`feedbackThread`、`createFeedbackThread`、`addFeedbackMessage` |

## 7. 使用建议

1. 仅想快速转换时，优先使用“快捷助手”并准备有效兑换码。
2. 需要查看历史任务、充值记录或提交反馈时，切换到“会员中心”并登录真实账号。
3. ZIP 包建议保持在 10 MB 以下，主 TeX 文件名需要与压缩包内路径一致。
4. 云端转换失败时，优先到“转换记录”中查任务状态，再通过“反馈”关联该任务提交问题。
5. Web 部署时确保同源 `/v1/` 能代理到后端 API，否则需要在登录窗口手动填写正确 API 地址。
