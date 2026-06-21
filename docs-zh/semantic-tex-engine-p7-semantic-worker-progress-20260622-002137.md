# Semantic TeX Engine P7 Semantic Worker 开发进展报告

**时间戳**：20260622-002137  
**基准计划**：`docs-zh/plan-0621.md`  
**关联旧报告**：`docs-zh/semantic-tex-engine-p7-worker-progress-20260621-233323.md`  
**阶段定位**：P7 从“内存态旧引擎 worker”推进为“内存态 semantic-engine worker preview”

---

## 一、当前结论

本轮已将 P7 云端转换 worker 的主路径从旧 `doc_core::convert_zip()` 升级为 `doc_compiler_engine::SemanticTexEngine::compile_zip_to_docx()`。

当前已经具备：

- `/v1/uploads` 保存上传 zip 到 `ServerState`。
- `/v1/conversions` 创建 queued job 并投递到 `tokio::sync::mpsc` 队列。
- job 状态新增 `analyzing`，状态流转更贴近语义引擎阶段。
- worker 默认执行 `semantic-engine`。
- `legacy-rule` / `doc-core` 保留为显式 fallback 路径。
- semantic worker report 输出 executor、backend、profile、quality_status、compatibility_score、warnings。
- paper3 上传 zip 能通过 P7 API 产出真实 DOCX 和转换报告。

当前仍不是生产级 SaaS Worker，因为还缺：

- PostgreSQL 任务表。
- 对象存储。
- JWT 鉴权。
- 用量扣减。
- Docker/namespace sandbox。
- CPU/memory/disk/time 限制。
- worker 崩溃恢复。
- 多 worker 横向扩展。

---

## 二、核心实现变化

### 2.1 Worker 主路径升级

修改文件：

```text
crates/server/Cargo.toml
crates/server/src/worker_service.rs
```

新增依赖：

```text
doc-compiler-engine = { path = "../compiler-engine" }
```

执行分支：

```text
execute_conversion
  ├─ semantic-engine / semantic / auto -> execute_semantic
  └─ legacy-rule / doc-core             -> execute_legacy
```

`execute_semantic` 调用：

```text
SemanticTexEngine::compile_zip_to_docx(zip_bytes, main_tex, options)
```

核心选项：

```text
profile_ref: ProfileRef
semantic_backend: Auto
allow_backend_fallback: true
min_compatibility_score_override: quality -> score
```

### 2.2 保留旧引擎 fallback

semantic worker 失败时，如果请求 engine 为默认 semantic 路径，会回退旧 `doc_core::convert_zip()`，并在 warnings 中写入：

```text
semantic-engine fallback to legacy-rule: <error>
```

这样可以满足两点：

1. 云端主路径开始验证新语义引擎。
2. 旧 Rust 规则引擎仍然独立存在，不被语义引擎改造污染。

### 2.3 Job 状态扩展

修改文件：

```text
crates/server/src/state.rs
crates/commercial-api-client/src/models.rs
```

新增状态：

```text
analyzing
```

当前状态流：

```text
queued
  -> normalizing
  -> detecting
  -> analyzing
  -> compiling
  -> rendering
  -> verifying
  -> completed / failed / expired
```

### 2.4 API 请求与报告字段扩展

修改文件：

```text
crates/server/src/routes.rs
crates/commercial-api-client/src/models.rs
```

`POST /v1/conversions` 请求新增：

```json
{
  "engine": "semantic-engine"
}
```

兼容字段：

```text
backend
```

优先级：

```text
engine -> backend -> semantic-engine
```

job 查询响应新增：

```text
main_tex
profile
quality
engine
```

report 响应新增：

```text
executor
backend
quality_status
compatibility_score
docx_bytes
warnings
```

---

## 三、当前 API 行为

### 3.1 创建 semantic conversion

请求：

```http
POST /v1/conversions
```

```json
{
  "upload_id": "upload_0000000000000001",
  "main_tex": "main-jos.tex",
  "profile": "jos-paper",
  "quality": "standard",
  "engine": "semantic-engine"
}
```

返回：

```json
{
  "job_id": "conv_0000000000000002",
  "upload_id": "upload_0000000000000001",
  "status": "queued",
  "profile": "jos-paper",
  "quality": "standard",
  "engine": "semantic-engine",
  "docx_ready": false,
  "report_ready": false
}
```

### 3.2 查询报告

成功后：

```http
GET /v1/conversions/:id/report
```

返回字段示例：

```json
{
  "job_id": "conv_0000000000000002",
  "status": "completed",
  "quality_score": 73,
  "profile": "jos-paper-toml",
  "main_tex": "main-jos.tex",
  "executor": "semantic-engine",
  "backend": "xelatex-hook",
  "quality_status": "passed_with_warnings",
  "compatibility_score": 90,
  "docx_bytes": 3057630,
  "warnings": []
}
```

实际 profile 可能为 `jos-paper` 或 TOML 加载后的 `jos-paper-toml`，测试中已兼容该别名。

---

## 四、验证结果

本轮已执行：

```bash
cargo fmt -p doc-server -p doc-commercial-api-client
cargo check -p doc-commercial-api-client
cargo check -p doc-server
cargo test -p doc-server --test api p7_cloud_worker_converts_uploaded_zip -- --nocapture
cargo test -p doc-server --test api -- --nocapture
```

结果：

```text
PASS
```

说明：

- `cargo fmt` 仍会输出项目历史 rustfmt nightly-only 配置 warning，不影响格式化结果。
- `doc-server` 集成测试需要绑定 `127.0.0.1:0`，受限沙箱可能需要放行本地监听。
- `p7_cloud_worker_converts_uploaded_zip` 使用 `examples/paper3/upload.zip`，确认 semantic worker 能产出真实 DOCX 和 report。

---

## 五、GitNexus 影响范围

本轮修改前已对关键符号做 impact 分析：

| 符号 | 风险 | 说明 |
|---|---|---|
| `process_job` | LOW | 直接上游为 `worker_loop`，间接入口为 `spawn_worker_state` / `router` |
| `p7_cloud_worker_converts_uploaded_zip` | LOW | 测试路径影响 |
| `ConversionReport` / `JobStatus` | LOW | commercial-api-client 响应模型扩展 |

合并 P5-P9 当前所有未提交变更后，`gitnexus detect_changes` 报告整体风险为 HIGH，原因是本轮工作覆盖了 server routes、worker、commercial-api-client、desktop-slint 多条执行流。该 HIGH 风险与当前阶段“仍为 Preview，不可直接 GA 发布”的判断一致。

---

## 六、剩余 P7 工作

### 6.1 生产级任务表

- PostgreSQL `conversion_jobs` 表。
- job 状态持久化。
- 幂等创建。
- job 取消和超时。
- worker 重启恢复。

### 6.2 Artifact 存储

- 上传 zip 存对象存储。
- 输出 DOCX 存对象存储。
- report/log 存对象存储。
- 支持过期清理和用户删除。

### 6.3 安全 sandbox

- 每 job 独立临时目录。
- 禁止 shell escape。
- 禁止网络访问。
- 限制 CPU/memory/disk/time。
- rootless container 或 namespace 隔离。
- 日志脱敏。

### 6.4 账号和用量衔接

- 创建 conversion 前检查 JWT。
- 创建 job 前检查额度。
- 成功/失败后写 usage ledger。
- 失败退款策略。
- 套餐限制：文件大小、超时、并发数。

### 6.5 横向扩展

- 队列从 `mpsc` 替换为 Redis Stream、PostgreSQL queue 或专用 MQ。
- 多 worker 并发消费。
- worker heartbeat。
- stuck job recovery。

---

## 七、商业化判断

P7 当前已经满足 Preview 演示：

```text
上传 paper3 zip
  -> 创建 semantic-engine job
  -> 状态轮询
  -> 下载 DOCX
  -> 查询 report
```

但尚未满足 Beta/GA：

```text
生产账号
生产计费
生产持久化
生产 sandbox
生产监控
生产 SLA
```

下一步建议优先进入：

```text
P11: 生产账号/用量/计费基础
P12: 生产级 worker + sandbox
P13: Desktop 云端转换闭环
```
