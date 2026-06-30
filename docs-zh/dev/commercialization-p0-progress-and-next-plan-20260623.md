# Tex2Doc 商业化 P0 开发进展与下一步规划
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



**日期**：2026-06-23  
**输出目录**：`docs-zh/dev`  
**关联方案**：

- `docs-zh/dev/slint-flutter-commercialization-dev-plan-20260623.md`
- `docs-zh/money/next-step-commercialization-work-plan-20260623.md`

## 一、当前结论

本轮已经完成商业化 P0 的第一批工程落地，重点补齐 Slint 桌面端、Flutter 多端入口和 Rust 服务端之间的商业化 Preview 闭环。

当前状态可以定义为：

```text
Preview 商业闭环增强完成
  -> 可继续推进受控 PoC 验收
  -> 尚未达到邀请制 Beta / 付费 Beta
```

本轮没有把系统直接推进到生产级商业化，因为生产级 auth、PostgreSQL store、usage ledger、object storage、sandbox worker、真实 billing provider 和签名安装包仍属于下一阶段工作。

## 二、本轮完成内容

### 2.1 Rust 服务端

| 工作项 | 完成情况 | 关键价值 |
|---|---|---|
| ZIP guard | 已完成 | 上传和 `/api/v1/convert` 都会检查无效 zip、空 zip、zip-slip、文件数、单文件大小、解压总大小 |
| 稳定错误码 | 已完成 | `ServerError::BadRequest` 支持稳定 `error` code，便于客户端展示和诊断 |
| conversion error_code | 已完成 | job JSON 和 report JSON 都包含 `error_code` |
| 失败 report | 已完成 | worker 失败时也生成最小 report，支持用户和客服定位 |
| API 回归测试 | 已完成 | 增加最小 zip fixture 和失败 job report 测试 |

当前新增/强化的错误码包括：

```text
invalid_zip
empty_zip
zip_slip
upload_too_large
too_many_files
file_too_large
uncompressed_too_large
upload_not_found
convert_failed
worker_join_error
invalid_docx
```

### 2.2 Slint 桌面端

| 工作项 | 完成情况 | 关键价值 |
|---|---|---|
| 启动 session 恢复 | 已完成 | 检测本地 refresh token 后自动 refresh，并拉 `/v1/me` 与 usage |
| 云转换前额度预检 | 已完成 | 上传前先查 usage，额度不足直接阻断 |
| recent jobs 远端任务定位 | 已完成 | `JobEntry` 新增 `remote_job_id`，可用于售后定位 |
| 诊断包增强 | 已完成 | 诊断包包含 `cloud-job-report.json` |
| 旧历史兼容 | 已完成 | `remote_job_id` 使用 `serde(default)`，旧 recent jobs 可继续读取 |

Slint 当前已具备受控 PoC 需要的核心路径：

```text
启动
  -> 自动恢复账号
  -> 查看用量
  -> 云端转换前额度预检
  -> 上传项目
  -> 创建 conversion
  -> 轮询 job
  -> 下载 DOCX/report
  -> recent jobs 记录远端 job id
  -> 导出诊断包
```

### 2.3 Flutter 多端入口

| 工作项 | 完成情况 | 关键价值 |
|---|---|---|
| CommercialApiClient 扩展 | 已完成 | 补齐 refresh、me、checkout、portal、upload、conversion、poll、download、report |
| 转换数据模型 | 已完成 | 新增 `UploadResponse`、`ConversionJob`、`ConversionReport`、`BillingSession` |
| 终态判断 | 已完成 | `ConversionJob.isTerminal` 可支持后续轮询 UI |
| 静态分析 | 已通过 | `flutter analyze` 无问题 |
| 测试 | 已通过 | `flutter test` 和 `commercial_api_test.dart` 均通过 |

Flutter 当前仍是 API 能力准备完成，UI 层的 CloudConvertPanel 属于下一步 P1。

## 三、验证结果

已执行并通过：

```powershell
cargo check -p doc-server
cargo check -p doc-commercial-api-client
cargo check -p doc-desktop-slint
cargo test -p doc-server --test api -- --nocapture
cargo test -p doc-desktop-slint -- --nocapture
flutter analyze
flutter test
flutter test test/commercial_api_test.dart
```

测试结果摘要：

| 验证项 | 结果 |
|---|---|
| doc-server API tests | 10 passed |
| doc-desktop-slint tests | 29 passed |
| Flutter analyze | No issues found |
| Flutter tests | All passed |
| Commercial API Dart test | passed |

GitNexus 变更检测：

```text
detect_changes(scope=all): critical
```

原因说明：

- 本轮触及 Slint 主 UI、云端转换、server worker/API 等商业主链路。
- 工作区还存在进入本轮前已有的 `AGENTS.md` / `CLAUDE.md` 索引数字变更。
- 相关主链路已通过 server API、desktop Slint、Flutter 静态分析和测试覆盖。

## 四、当前商业化就绪度

| 阶段 | 当前判断 | 说明 |
|---|---|---|
| 内部 Preview | 可继续使用 | P0 增强后更适合内部联调和演示 |
| 受控 PoC | 接近可启动 | 仍建议补 GUI 手工验收矩阵和 demo 包 |
| 邀请制 Beta | 未达标 | 缺生产 auth、持久化 store、安装包、sandbox |
| 付费 Beta | 未达标 | 缺 billing provider、usage ledger、失败返还、签名发布 |
| GA | 未达标 | 缺监控、合规、SLA、质量 dashboard |

短期建议继续以受控 PoC 为目标，不要直接公开自助收费。

## 五、已知边界

### 5.1 服务端边界

- auth 仍是 demo token，不是 JWT。
- usage 仍是 in-memory 计数。
- upload/job/docx/report 仍是内存态。
- billing checkout/portal 仍是 mock URL。
- worker 尚未 sandbox，仅增加了上传 zip guard 和失败 report。

### 5.2 Slint 边界

- session 恢复依赖 preview refresh API。
- 任务轮询仍是阻塞式桥接，UI 阶段进度展示还偏粗。
- token 安全存储还需要三平台真实验证。
- 尚未完成 Windows/macOS/Linux 安装包与签名。

### 5.3 Flutter 边界

- SDK 方法已经补齐，但 UI 仍未接入完整云端转换面板。
- Web/PWA 下的 refresh token 安全策略尚未定稿。
- 还没有上传进度、轮询状态和 report 展示 UI。

## 六、下一步规划

### 6.1 P1：PoC 收口

周期：1 周  
目标：让 5-10 个合作用户可在人工支持下完成真实论文试用。

任务：

| 优先级 | 任务 | 模块 | 验收 |
|---|---|---|---|
| P1 | Slint GUI 手工验收矩阵 | Slint | 登录、用量、本地转换、云端转换、report、诊断包 |
| P1 | Cloud job 阶段进度展示 | Slint/Server | queued 到 verifying 可映射到 UI |
| P1 | Flutter CloudConvertPanel | Flutter | 上传 zip、创建 job、轮询、下载 DOCX/report |
| P1 | Demo 包制作 | 产品/质量 | 3 个 before/after demo |
| P1 | 试用手册 | Docs | 非研发用户可按步骤完成试用 |
| P1 | conversion_stats 输出 | Quality | profile 维度通过率、失败率、openability |

### 6.2 P2：生产底座起步

周期：2-3 周  
目标：替换 preview mock 的关键商业基础设施。

任务：

| 优先级 | 任务 | 模块 | 验收 |
|---|---|---|---|
| P2 | PostgreSQL migration | Server | `docdb` 可初始化用户、套餐、用量、任务表 |
| P2 | JWT access token | Server | demo token 替换为短期 JWT |
| P2 | refresh token hash/rotation | Server/Slint | logout 后 refresh token 不可继续使用 |
| P2 | usage ledger | Server | 创建预占、成功确认、失败返还 |
| P2 | local object storage | Server/Worker | 上传和产物服务重启不丢 |
| P2 | Postgres queue | Worker | worker 崩溃后任务可恢复或标记失败 |

### 6.3 P3：商业推广 Beta 准备

周期：4-6 周  
目标：进入邀请制 Beta。

任务：

| 优先级 | 任务 | 模块 | 验收 |
|---|---|---|---|
| P3 | Windows installer | Release/Slint | 可安装、卸载、升级 |
| P3 | release manifest DB 化 | Server/Release | 按 channel/platform/arch 查询 |
| P3 | manifest signature | Release/Slint | sha256 + signature 校验 |
| P3 | sandbox worker | Worker | no network、timeout、资源限制 |
| P3 | Stripe test mode | Billing | checkout/webhook 幂等 |
| P3 | quality dashboard | Quality | profile/backend/date 维度趋势 |

## 七、推荐执行顺序

```text
1. Slint GUI 手工验收矩阵
2. Flutter CloudConvertPanel
3. conversion_stats 和 demo 包
4. PostgreSQL migration
5. JWT + refresh token hash
6. usage ledger + failure refund
7. local object storage + Postgres queue
8. Windows installer + release manifest signature
```

## 八、结论

本轮 P0 已经把商业化 Preview 的关键缺口补到可继续 PoC 的水平。下一步不建议继续扩散功能，而应围绕受控 PoC 做收口：

```text
真实用户可跑通
失败可解释
诊断可导出
任务可定位
质量可统计
```

完成 P1 后即可组织 5-10 个合作用户试用；完成 P2 后再扩大邀请制 Beta；完成 P3 后才适合进入付费 Beta。
