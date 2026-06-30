# Tex2Doc Web 商业化工作台开发进展报告
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



日期：2026-06-23

## 一、当前目标

围绕 Web 端商业化推广，完成登录注册、充值、转换、账号四个核心模块的闭环设计与开发：

1. 未登录时，除登录/注册外的业务功能不可用。
2. 充值支持按次与按日期套餐，当前采用 mock 支付到账。
3. 转换模块保留现有云端语义引擎调用，并补充日志、打包提示、操作步骤和转换记录查询。
4. 账号模块可查看当前账号、套餐/额度、充值记录、转换记录。

## 二、已完成开发内容

### 1. 服务端商业 API

涉及文件：

- `crates/server/src/state.rs`
- `crates/server/src/routes.rs`

已完成：

- 新增充值记录内存态存储 `RechargeRecord`。
- 转换任务增加 `user_id`，支持按当前登录用户查询转换记录。
- 新增充值套餐接口：
  - `GET /v1/recharge/options`
  - `GET /api/v1/recharge/options`
- 新增充值记录接口：
  - `POST /v1/recharges`
  - `GET /v1/recharges`
  - `POST /api/v1/recharges`
  - `GET /api/v1/recharges`
- 转换接口增加列表查询：
  - `GET /v1/conversions`
  - `GET /api/v1/conversions`
- mock 充值规则：
  - 按次：1 元/次，最低 3 次，预置 3 次、10 次、30 次。
  - 日期：日卡 5 元、周卡 14 元、月卡 30 元、年卡 120 元。
  - 当前返回 `paid_mock`，provider 为 `mock-pay`，保留后续三方支付接入字段。

### 2. Flutter Web API 客户端

涉及文件：

- `flutter_app/lib/commercial_api.dart`

已完成：

- 新增 `RechargeOptions`、`RechargePackage`、`RechargeRecord` 模型。
- 新增 API 调用：
  - `rechargeOptions()`
  - `createRecharge(...)`
  - `recharges(accessToken)`
  - `conversions(accessToken)`
- 价格展示支持 CNY 显示为 `¥`。

### 3. Flutter Web UI

涉及文件：

- `flutter_app/lib/workspace_app.dart`
- `flutter_app/lib/ui/app_i18n.dart`

已完成：

- 左侧导航新增“充值”模块。
- 工作台页聚合账号、充值、转换入口。
- 账号页由“登录/注册”与“账号总览”组成：
  - 可查询账号信息。
  - 可查询用量。
  - 可查询充值记录。
  - 可查询转换记录。
- 充值页新增：
  - 按次套餐按钮。
  - 日期套餐按钮。
  - mock 支付状态提示。
  - 充值记录查询与展示。
- 转换页增强：
  - 未登录时上传与转换按钮禁用。
  - 增加操作步骤说明。
  - 增加 ZIP 打包方法提示。
  - 增加转换日志输出。
  - 增加转换记录查询。
- 中英文文案已补充，避免界面出现未翻译 key。

### 4. 设计文档

已输出：

- `docs-zh/ui/web-commercial-account-recharge-convert-design-20260623.md`

内容覆盖：

- 数据库设计。
- 系统架构设计。
- 功能设计。
- UI 设计。
- 后续三方支付与数据库落地计划。

## 三、验证进展

已完成并通过：

- `cargo test -p doc-server --test api -- --nocapture`
  - 结果：10 个服务端 API 测试全部通过。
- `flutter test`
  - 结果：4 个 Flutter 测试全部通过。
- `flutter analyze`
  - 结果：无问题。
- `flutter build web --debug`
  - 结果：Web 产物构建成功。
- `node --check scripts/e2e_flutter_commercial_web.mjs`
  - 结果：脚本语法检查通过。

未完成：

- Playwright 全链路 E2E 已扩展到注册、登录、充值、账号记录、转换、下载 DOCX，但最后一次运行被中断，尚未形成完整通过结果。

当前本地服务端口状态：

- `8080`：doc-server 正在监听。
- `5173`：当前 Web 页面服务正在监听。
- `4174`：Playwright 测试 Web 服务正在监听。

## 四、当前工作区状态说明

本轮 Web 商业化相关主要改动：

- `crates/server/src/state.rs`
- `crates/server/src/routes.rs`
- `flutter_app/lib/commercial_api.dart`
- `flutter_app/lib/workspace_app.dart`
- `flutter_app/lib/ui/app_i18n.dart`
- `scripts/e2e_flutter_commercial_web.mjs`
- `docs-zh/ui/web-commercial-account-recharge-convert-design-20260623.md`
- `docs-zh/ui/web-commercial-workbench-progress-report-20260623.md`

工作区还存在其他历史/并行改动，例如：

- `AGENTS.md`
- `CLAUDE.md`
- `crates/desktop-slint/*`
- `flutter_app/assets/*`
- `flutter_app/web/*`

这些文件未在本报告中归入本轮 Web 商业化功能开发范围，提交前需要再次确认分组。

## 五、风险与待处理问题

1. 当前服务端仍使用内存态 mock 数据，重启后充值记录、转换记录会丢失。
2. 充值到账尚未真实扣减/增加权益，当前只记录订单，转换仍沿用 preview 云转换额度。
3. 转换日志当前主要为前端操作日志，尚未持久化到服务端 `conversion_logs`。
4. Playwright 全链路测试需要重新跑完并根据实际 UI 语义定位做最后修正。
5. 未登录门禁已在 UI 层实现，正式商业化还需要服务端按权益校验上传、转换、下载等接口。

## 六、下一步计划

1. 完成 Playwright 全链路 E2E，并固定为可重复运行的商业化验收脚本。
2. 将 mock 充值结果写入权益模型，实现按次余额和日期有效期。
3. 转换前增加权益校验与扣减逻辑。
4. 将用户、充值订单、权益、转换任务、转换日志迁移到数据库。
5. 接入三方支付 webhook，完成 pending -> paid -> entitlement 生效流程。
6. 优化账号页记录展示，增加任务号、订单号复制、失败原因与客服排障字段。
