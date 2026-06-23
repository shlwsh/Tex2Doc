# Semantic TeX Engine P5-P7 开发进展报告

**时间戳**：20260622-010501  
**基准计划**：`docs-zh/plan-0621.md`  
**本轮目标**：继续推进 P5-P9 后续开发任务；优先核实 P6/P7 现状，并补强 P5 Slint Desktop MVP 的本地使用闭环。

---

## 一、当前结论

本轮确认 P6/P7 preview API 和 semantic worker 当前回归通过，并对 P5 桌面端新增 settings 持久化能力。

阶段状态保持如下：

| 阶段 | 当前状态 | 本轮变化 |
|---|---|---|
| P5 Slint Desktop MVP | in_progress | 新增转换/检测 profile 后的设置持久化；`cargo check -p doc-desktop-slint` 通过 |
| P6 账号、订阅、用量 API | in_progress | 完整 API 集成测试 9/9 通过，包含 Bearer token 门禁和 usage 扣减验证 |
| P7 云端转换 SaaS Worker | in_progress | `p7_cloud_worker_converts_uploaded_zip` 通过，semantic worker 能产出真实 DOCX/report |
| P8 真实样本回归与质量指标 | in_progress | 本轮未新增实现，沿用已有 nightly regression 能力 |
| P9 自动升级与三平台分发 | in_progress | 本轮未新增实现，updater 仍是 manifest/SHA256 骨架 |

注意：P5-P9 仍不能标记为 completed。当前只是从 preview 能力继续向商业化闭环推进。

---

## 二、P6/P7 当前核实结果

### 2.1 服务端商业 API 测试

已执行：

```bash
cargo test -p doc-server --test api -- --nocapture
```

结果：

```text
PASS: 9 passed, 0 failed
```

覆盖测试包括：

- `health_returns_ok`
- `version_returns_semver`
- `convert_missing_file_returns_400`
- `convert_zip_header_only_returns_400`
- `convert_main_tex_mismatch_returns_400`
- `p6_commercial_user_endpoints_require_bearer_token`
- `p6_commercial_contract_endpoints_return_json`
- `p7_cloud_worker_converts_uploaded_zip`

本轮验证到的关键事实：

- `/v1/usage`、`/v1/uploads`、`/v1/conversions` 未携带 Bearer token 时返回 401。
- demo auth 返回的 token 可访问受保护端点。
- `/v1/conversions` 会触发 preview 用量扣减。
- P7 worker 通过 `SemanticTexEngine::compile_zip_to_docx()` 处理 `examples/paper3/upload.zip`。
- 成功任务可以返回真实 DOCX 和 report。

### 2.2 客户端 SDK 编译

已执行：

```bash
cargo check -p doc-commercial-api-client
```

结果：

```text
PASS
```

说明：

- `auth / usage / billing / uploads / conversions / releases` 模块仍保持可编译。
- 默认 `ApiClient` 继续使用 Bearer token 请求模型，与 server preview auth gate 对齐。

### 2.3 桌面端编译

已执行：

```bash
cargo check -p doc-desktop-slint
```

结果：

```text
PASS
```

仍存在的 warning：

- `AppState.auth_token/user_name/quota_remaining` 尚未接 UI。
- `updater` 模块尚未接 UI 和真实发布包下载。
- `CommandError` 中部分错误变体尚未使用。

这些 warning 反映 P5/P9 后续工作仍未完成。

---

## 三、P5 本轮实现内容

### 3.1 设置持久化接入

修改文件：

```text
crates/desktop-slint/src/main.rs
```

新增函数：

```rust
fn persist_settings(
    project_path: Option<&str>,
    output_path: Option<&str>,
    profile: Option<&str>,
    quality: Option<&str>,
)
```

实现行为：

- 点击 `Convert` 时保存：
  - project path
  - output path
  - profile
  - quality
- 点击 `Detect Profile` 且检测成功时保存：
  - project path
  - detected profile
- 保存失败时只记录 warning，不阻断转换。

### 3.2 用户体验影响

当前桌面端启动时已从 `Settings::load()` 初始化 UI：

- `output_path`
- `quality_level`
- `detected_profile`
- `project_path`

本轮改造后，用户上一次输入的项目路径、输出路径、profile 和 quality 可以在下次启动时恢复。这补齐了 P5 MVP 中“本地转换闭环”的一个实际可用性缺口。

### 3.3 影响范围

GitNexus impact 分析：

```text
target: crates/desktop-slint/src/main.rs::main
risk: LOW
direct callers: 0
affected processes: 0
```

本轮只修改桌面端入口逻辑，不影响：

- 旧 Rust rule-based 转换引擎
- SemanticTexEngine 核心
- server worker
- commercial API client 合约

---

## 四、当前仍未完成内容

### 4.1 P5 剩余

- 真实 GUI 操作验收。
- 文件/目录选择器。
- 拖拽项目或 zip。
- 登录/用量/订阅入口。
- 云端转换 UI：
  - login
  - upload
  - create conversion
  - poll
  - download docx
  - show report
- 设置项需要进一步区分“默认输出目录”和“最近输出 DOCX 文件”。

### 4.2 P6 剩余

- demo token 替换为 JWT access/refresh token。
- 密码哈希、邮箱验证、refresh token 轮换。
- usage 从内存 `HashMap` 升级为持久化 event ledger。
- billing checkout/portal 接入真实 provider。
- webhook 签名验证和幂等处理。

### 4.3 P7 剩余

- uploads/jobs/docx/report 从内存态迁移到数据库和对象存储。
- conversion queue 从 `tokio::sync::mpsc` 升级为可恢复队列。
- worker sandbox：
  - 禁网
  - 禁 shell escape
  - CPU/memory/disk/time/process 限制
  - 每 job 隔离目录
- worker 崩溃恢复、重试、取消、过期清理。

### 4.4 P8 剩余

- 每 profile realistic fixture 从 3+ 扩展到 10+。
- `semantic-verify` 和 nightly regression 共享更完整质量指标：
  - OMML 公式覆盖率
  - 表格 span 覆盖率
  - 图片关系完整率
  - style coverage
  - bookmark/hyperlink 检查
- LibreOffice/Word-open 验证在 CI 中启用 required 模式。

### 4.5 P9 剩余

- updater 接入 Slint UI。
- release manifest 按 platform/channel/version 读取真实发布记录。
- 真实 artifact 下载。
- Ed25519/minisign 或平台签名验签。
- Windows MSI、macOS DMG、Linux AppImage。
- 代码签名、公证、灰度升级和回滚。

---

## 五、建议下一步

下一轮建议优先做 P5-P6 串联：

```text
Slint Desktop
  -> login/register
  -> usage display
  -> upload project zip
  -> create cloud conversion
  -> poll status
  -> download DOCX/report
```

原因：

- P6/P7 server preview 已通过集成测试，具备可调用基础。
- `doc-commercial-api-client` 已有对应方法。
- 桌面端已有 `AppState.auth_token/user_name/quota_remaining` 字段但尚未使用。
- 这一块完成后，P5 和 P6/P7 能形成第一条完整商业产品路径。

建议同步保留本地转换入口，云端转换作为独立 mode，不影响旧 rule-based 引擎和新 semantic engine 的独立验证。
