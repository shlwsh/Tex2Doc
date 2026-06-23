# Semantic TeX Engine P7 云端转换 Worker 开发进展报告

**时间戳**：20260621-233323  
**基准计划**：`docs-zh/plan-0621.md`  
**阶段定位**：P7 从 pending 推进到 in_progress  

## 一、当前结论

本轮已将 P6 的模拟 conversion 合约升级为 P7 的内存态异步转换 Worker 骨架。

当前已经具备：

- `/v1/uploads` 保存上传 zip 到 `ServerState`。
- `/v1/conversions` 创建 queued job 并投递到 `tokio::sync::mpsc` 队列。
- `worker_service` 后台消费任务并调用 `doc_core::convert_zip()`。
- job 状态可通过 `/v1/conversions/:id` 查询。
- 成功后 `/v1/conversions/:id/download/docx` 返回真实 DOCX。
- 成功后 `/v1/conversions/:id/report` 返回真实转换报告。
- 旧 `/api/v1/convert` 同步转换路径保持不变。

当前仍不是生产级 SaaS Worker，因为还缺：

- 持久化任务表。
- 对象存储。
- JWT 鉴权。
- 用量扣减。
- Docker/namespace sandbox。
- CPU/memory/disk/time 限制。
- worker 崩溃恢复。
- 多 worker 横向扩展。

## 二、新增代码结构

### 2.1 ServerState

新增文件：

```text
crates/server/src/state.rs
```

核心结构：

```text
ServerState
├── uploads: RwLock<HashMap<String, UploadRecord>>
├── jobs: RwLock<HashMap<String, ConversionJobRecord>>
├── queue: mpsc::Sender<WorkerCommand>
└── seq: AtomicU64
```

已支持：

- `store_upload()`
- `get_upload()`
- `create_job()`
- `enqueue_job()`
- `get_job()`
- `update_status()`
- `complete_job()`
- `fail_job()`

任务状态：

```text
queued
normalizing
detecting
compiling
rendering
verifying
completed
failed
expired
```

### 2.2 Worker Service

新增文件：

```text
crates/server/src/worker_service.rs
```

核心流程：

```text
spawn_worker_state()
  -> ServerState
  -> tokio::spawn(worker_loop)

worker_loop
  -> recv WorkerCommand
  -> process_job

process_job
  -> normalizing
  -> detecting
  -> compiling
  -> doc_core::convert_zip()
  -> rendering
  -> verifying
  -> completed / failed
```

当前 worker 使用：

```text
tokio::task::spawn_blocking
```

避免同步转换阻塞 async runtime。

### 2.3 Routes 接入

修改文件：

```text
crates/server/src/routes.rs
crates/server/src/lib.rs
crates/server/src/main.rs
crates/server/src/error.rs
```

新增/变更点：

- `router()` 内部创建带 worker 的 `ServerState`。
- 新增 `router_with_state(state)`，方便测试或后续嵌入式部署。
- P6/P7 商业端点接入 Axum `State<ServerState>`。
- 新增 `not_found` 和 `conflict` 错误码。

## 三、接口行为变化

### 3.1 上传项目

```text
POST /v1/uploads
```

返回：

```json
{
  "upload_id": "upload_0000000000000001",
  "status": "stored",
  "bytes": 123456,
  "file_name": "project.zip",
  "created_at": "1782056000"
}
```

### 3.2 创建转换任务

```text
POST /v1/conversions
```

请求：

```json
{
  "upload_id": "upload_0000000000000001",
  "main_tex": "main-jos.tex",
  "profile": "jos-paper",
  "quality": "standard"
}
```

返回 queued job：

```json
{
  "job_id": "conv_0000000000000002",
  "upload_id": "upload_0000000000000001",
  "status": "queued",
  "docx_ready": false,
  "report_ready": false
}
```

### 3.3 查询任务

```text
GET /v1/conversions/:id
```

返回实时状态：

```json
{
  "job_id": "conv_0000000000000002",
  "status": "compiling",
  "docx_ready": false,
  "report_ready": false
}
```

### 3.4 下载 DOCX

```text
GET /v1/conversions/:id/download/docx
```

行为：

- job 未完成：返回 `409 conflict`。
- job 不存在：返回 `404 not_found`。
- job 完成：返回真实 DOCX 字节流。

### 3.5 查询报告

```text
GET /v1/conversions/:id/report
```

行为：

- job 未完成：返回 `409 conflict`。
- job 不存在：返回 `404 not_found`。
- job 完成：返回真实报告。

## 四、验证结果

已执行：

```bash
cargo check -p doc-server
cargo check -p doc-commercial-api-client
cargo test -p doc-server --test api -- --nocapture
```

结果：

```text
PASS
```

`api` 集成测试共 8 项通过：

- health
- version
- P6 commercial contract
- P7 cloud worker converts uploaded zip
- legacy `/api/v1/convert`
- missing file
- main tex mismatch
- bad zip

注意：

- 集成测试需要绑定 `127.0.0.1:0`，必须在允许本地监听的环境运行。
- `p7_cloud_worker_converts_uploaded_zip` 使用 `examples/paper3/upload.zip`，确认 worker 产出真实 DOCX 和报告。

## 五、剩余 P7 工作

### 5.1 生产级任务状态

- 将内存 `HashMap` 替换为数据库任务表。
- 支持 job 过期清理。
- 支持 worker 重启后的任务恢复。
- 支持失败重试策略。

### 5.2 对象存储

- 上传 zip 写入对象存储。
- DOCX/report 写入对象存储。
- 下载接口改为签名 URL 或流式读取。

### 5.3 Sandbox

首期建议：

```text
Docker sandbox
├── readonly runtime image
├── per-job temp directory
├── network disabled
├── cpu/memory limit
├── disk quota
└── timeout kill
```

后续可演进为：

```text
containerd / firecracker / gVisor
```

### 5.4 鉴权与用量

- `/v1/conversions` 前检查用户额度。
- job 完成后扣减额度。
- 失败任务不扣减或按策略部分扣减。
- 将 P6 demo token 替换为 JWT middleware。

## 六、下一步建议

1. 补 P7 `conversion_service.rs`，把 route handler 中的业务逻辑下沉。
2. 补 `upload_service.rs` 和对象存储 trait。
3. 补 `auth_service.rs` JWT middleware。
4. 补 sandbox 执行器抽象：

```rust
trait ConversionExecutor {
    async fn execute(&self, job: ConversionJobInput) -> ConversionJobOutput;
}
```

5. 在 P8 中把 P7 worker 纳入 nightly regression。
