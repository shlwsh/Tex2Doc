# Semantic TeX Engine P5-P6 开发进展报告
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



**时间戳**：20260621-232121  
**基准计划**：`docs-zh/plan-0621.md`  
**本轮目标**：核实 P5 当前实现，继续推进 P5-P9 后续商业化开发任务  

## 一、当前结论

本轮完成了 P5 Slint Desktop MVP 的关键可编译闭环，并启动 P6 商业 API 合约开发。

当前状态：

| 阶段 | 计划状态 | 本轮后状态 | 说明 |
|---|---|---|---|
| P5 Slint Desktop MVP | in_progress | in_progress | 已通过 Linux `cargo check`，本地转换、profile/quality、报告摘要、任务历史已接入；尚未做 GUI 运行验收、Windows/macOS 构建与安装包 |
| P6 账号/订阅/用量 API | pending | in_progress | 已新增客户端 SDK 模块、服务端内存合约端点、集成测试；仍缺真实账号、JWT、支付、额度扣减、持久化 |
| P7 云端转换 Worker | pending | pending | 仍待在 P6 合约基础上接入真实异步队列、sandbox 和转换产物存储 |
| P8 真实样本回归 | pending | pending | 仍待补 realistic fixtures、nightly regression 和质量统计 |
| P9 自动升级与三平台分发 | pending | pending | 仍待 updater、签名校验、MSI/DMG/AppImage 打包 |

## 二、P5 已完成内容

### 2.1 Slint UI 编译修复

修复文件：

```text
crates/desktop-slint/src/ui/main.slint
```

修复点：

- 当前 Slint 1.16 不支持原 `GridBox` 写法，已改为 `VerticalBox`/`HorizontalBox` 布局。
- 保留项目路径、profile、quality、输出路径、进度、报告摘要、状态和最近任务区域。

### 2.2 Rust 状态与设置修复

修复文件：

```text
crates/desktop-slint/src/app_state.rs
crates/desktop-slint/src/settings.rs
```

修复点：

- 去掉 `AppState` 对 `RwLock<Vec<JobEntry>>` 不成立的 `Clone/Default` 派生，保留手写 `Default`。
- `Settings::save()` 将 `serde_json::Error` 显式转换为 `std::io::Error`。
- 默认输出目录改用 `ProjectDirs::data_dir().join("output")`，避免调用不存在的 `document_dir()`。

### 2.3 本地转换链路接入 UI

修复文件：

```text
crates/desktop-slint/src/main.rs
crates/desktop-slint/src/commands.rs
crates/desktop-slint/src/job.rs
crates/desktop-slint/src/report.rs
```

实现点：

- `Detect Profile` 后台执行 `commands::detect_profile()`，通过 `slint::invoke_from_event_loop` 回写 UI。
- `Convert` 通过 `job::start_job()` 进入任务模块，后台执行本地转换，不阻塞 UI。
- UI 选择的 `profile` 和 `quality` 会传入 `local_convert::convert()`。
- 转换完成后回写：
  - detected profile
  - compatibility score
  - quality status/score
  - profile confidence
  - report summary
  - recent jobs
- `Open Output` 已按平台调用 `cmd /C start`、`open` 或 `xdg-open` 打开输出目录。

### 2.4 P5 验证结果

已执行：

```bash
cargo check -p doc-desktop-slint
```

结果：

```text
PASS
```

说明：

- 已验证 Linux 构建期和 Rust/Slint 类型链路。
- 未在当前环境启动 GUI，也未完成 Windows/macOS 构建验收。

## 三、P6 已完成内容

### 3.1 commercial-api-client SDK 扩展

新增文件：

```text
crates/commercial-api-client/src/auth.rs
crates/commercial-api-client/src/usage.rs
crates/commercial-api-client/src/billing.rs
crates/commercial-api-client/src/uploads.rs
crates/commercial-api-client/src/conversions.rs
crates/commercial-api-client/src/releases.rs
```

扩展文件：

```text
crates/commercial-api-client/src/client.rs
crates/commercial-api-client/src/lib.rs
crates/commercial-api-client/src/models.rs
```

新增能力：

- `register`
- `login`
- `refresh`
- `me`
- `usage`
- `plans`
- `create_checkout`
- `create_billing_portal`
- `upload_project_zip`
- `create_conversion`
- `get_conversion`
- `download_conversion_docx`
- `get_conversion_report`
- `release_manifest`

同时修复：

- 默认 `base_url` 改为 `https://api.tex2doc.cn/v1/`。
- 新增 `endpoint()`，避免 URL join 时丢失 `/v1/` 前缀。
- 新增 `get_bytes()` 和 `post_multipart()`，供 DOCX 下载和项目上传使用。

### 3.2 服务端商业 API 合约端点

扩展文件：

```text
crates/server/src/routes.rs
```

新增端点：

```text
POST /v1/auth/register
POST /v1/auth/login
POST /v1/auth/refresh
GET  /v1/me
GET  /v1/usage
GET  /v1/plans
POST /v1/billing/checkout
POST /v1/billing/portal
POST /v1/uploads
POST /v1/conversions
GET  /v1/conversions/:id
GET  /v1/conversions/:id/download/docx
GET  /v1/conversions/:id/report
GET  /v1/releases/:channel
```

兼容别名：

```text
/api/v1/*
```

说明：

- 当前服务端为内存模拟合约，不是生产级账号系统。
- auth 返回 demo token。
- usage/plans 返回固定套餐数据。
- uploads 解析 multipart file 并返回 upload_id。
- conversions 返回 demo conversion job。
- download/docx 返回带 DOCX magic header 的占位字节，P7 将替换为真实 worker 产物。
- releases 返回 P9 所需 manifest 结构，但签名和 sha256 仍是占位值。

### 3.3 P6 验证结果

已执行：

```bash
cargo check -p doc-commercial-api-client
cargo check -p doc-server
cargo test -p doc-server p6_commercial_contract_endpoints_return_json -- --nocapture
```

结果：

```text
PASS
```

注意：

- `doc-server` 集成测试需要绑定 `127.0.0.1:0`，在受限沙箱中会出现 `PermissionDenied`，已在非沙箱环境重跑通过。

## 四、剩余开发清单

### 4.1 P5 剩余

- 使用真实 `examples/journals/generic` 或 `paper3` 通过 GUI 完成一次转换验收。
- 保存 last project/output/profile/quality 到 `Settings`。
- 增加登录/用量/订阅入口，为 P6 客户端接入 UI 做准备。
- Windows/macOS/Linux 三平台构建冒烟。

### 4.2 P6 剩余

- 真实用户模型和密码哈希。
- JWT access/refresh token。
- API 鉴权 middleware。
- 用量扣减和额度不足阻断。
- Stripe 或等价 billing provider 接入。
- 上传文件持久化和配额校验。
- conversions 与 P7 worker 的真实任务表衔接。

### 4.3 P7 下一步设计实施

建议下一轮从服务端内部状态开始：

```text
ServerState
├── uploads: HashMap<UploadId, UploadRecord>
├── jobs: HashMap<JobId, ConversionJobRecord>
└── queue: tokio::sync::mpsc::Sender<WorkerCommand>
```

第一步实现：

- `upload_project` 保存 zip bytes 到内存状态。
- `create_conversion` 创建 queued job。
- `worker_service` 后台消费队列。
- 状态流转：

```text
queued -> normalizing -> detecting -> compiling -> rendering -> verifying -> succeeded/failed
```

第二步实现：

- worker 调用现有 `doc-core::convert_zip()` 或后续 `doc-compiler-engine` 云端编译入口。
- 成功时保存 DOCX bytes 和 report。
- `download/docx` 返回真实 DOCX。
- `report` 返回真实质量报告。

第三步实现：

- 每 job 独立临时目录。
- 超时控制。
- 文件大小限制。
- 后续接入 Docker/namespace sandbox。

## 五、当前风险

| 风险 | 级别 | 说明 |
|---|---|---|
| P5 UI 未运行验收 | 中 | 已编译通过，但未在真实桌面会话中操作验证 |
| P6 为模拟合约 | 高 | 可支撑前后端开发联调，但不能直接商业上线 |
| P7 未接队列和 sandbox | 高 | 云端转换还不具备生产隔离能力 |
| P9 release manifest 为占位 | 中 | 已有接口形态，但没有签名、sha256 和真实安装包 |

## 六、下一步执行顺序

1. 完成 P7 `ServerState + worker_service + job queue`。
2. 将 P6 conversion endpoints 改为读取真实 job 状态。
3. 将 Slint 客户端接入 `ApiClient` 登录、用量和云端转换入口。
4. 补 P8 realistic fixtures 和 nightly regression。
5. 补 P9 updater、release manifest 签名校验和三平台打包脚本。
