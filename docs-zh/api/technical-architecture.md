# Tex2Doc API 服务技术架构

## 1. 服务定位

`apps/rust-service` 是 Tex2Doc 的后端 API 发布单元，负责：

- 用户端和管理端 HTTP API。
- PostgreSQL 持久化与 schema 初始化。
- 上传 ZIP、转换任务、DOCX 结果和转换日志的存储。
- 转换任务队列与后台 worker。
- 调用共享 Rust 文档转换引擎。
- 静态托管产品首页、Flutter 用户端、Flutter 管理端。

该服务是一个 Rust 单体 API 服务。当前目录内没有 Node.js、Python、Dart、Java 等第二套 API 服务端实现。

## 2. 技术栈

| 层级 | 技术 |
| --- | --- |
| 语言 | Rust 2021 |
| HTTP 框架 | Axum 0.7 |
| 异步运行时 | Tokio，多线程 runtime |
| 中间件 | Tower、tower-http |
| 数据库 | PostgreSQL |
| 数据库访问 | sqlx 0.8，runtime-tokio-rustls，postgres，uuid，chrono，json |
| 序列化 | serde、serde_json |
| 日志 | tracing、tracing-subscriber |
| 上传与压缩 | multipart、zip、bytes |
| 文件存储 | 本地 session 目录 |
| 转换引擎 | `doc-core`、`doc-compiler-engine`、`doc-quality` |
| 测试客户端 | reqwest，仅作为 dev-dependency |

关键依赖定义在 `apps/rust-service/Cargo.toml`。

## 3. 目录与模块职责

| 文件 | 职责 |
| --- | --- |
| `src/main.rs` | 二进制入口，初始化 tracing，构建 router，绑定 TCP 地址并启动 Axum |
| `src/lib.rs` | 库入口，导出模块和 `build_router`，供集成测试复用 |
| `src/routes.rs` | REST 路由与 HTTP handler |
| `src/state.rs` | `ServerState` 聚合数据库、队列、文件存储、反馈存储 |
| `src/db_store.rs` | PostgreSQL 访问、schema 初始化、用户/订单/转换/反馈/发布等持久化逻辑 |
| `src/worker_service.rs` | 转换 worker 队列、任务领取、转换执行、失败/完成状态更新 |
| `src/file_storage.rs` | 上传 ZIP、结果 DOCX、转换日志的本地文件存储 |
| `src/feedback_service.rs` | 反馈领域服务封装 |
| `src/automation_service.rs` | 自动化研发请求、事件、agent 状态管理 |
| `src/error.rs` | API 错误类型与 Axum 响应转换 |
| `src/error_code.rs` | 转换错误码、HTTP 状态、用户提示、是否可重试/退款 |
| `src/excel_export.rs` | 管理端 Excel 导出 |
| `src/limits.rs` | 上传体积、文件数量、解压后体积等限制 |

## 4. 启动链路

1. `main.rs` 读取 `DOC_SERVER_ADDR`，默认 `127.0.0.1:2624`。
2. 初始化 `tracing_subscriber`。
3. 调用 `routes::router().await`。
4. `routes::router()` 调用 `worker_service::spawn_worker_state().await`。
5. `spawn_worker_state()` 创建 `mpsc` 队列，初始化 `ServerState`，并启动后台 worker。
6. `ServerState::new()` 连接 PostgreSQL，初始化 `DbStore`、`FileStorage`、`FeedbackStore`。
7. Router 挂载业务路由、静态文件路由、CORS、请求体限制。
8. `axum::serve(listener, app).await` 开始处理请求。

## 5. 请求处理架构

```mermaid
flowchart LR
  Client["Flutter / Slint / Extension / curl"] --> Axum["Axum Router"]
  Axum --> Routes["routes.rs handlers"]
  Routes --> State["ServerState"]
  State --> Db["DbStore / PostgreSQL"]
  State --> Storage["FileStorage / sessions"]
  State --> Queue["Tokio mpsc queue"]
  Queue --> Worker["worker_service"]
  Worker --> Engine["doc-compiler-engine / doc-core"]
  Worker --> Db
  Worker --> Storage
```

## 6. 转换任务链路

推荐云端转换链路：

1. 客户端登录获取 `access_token`。
2. `POST /v1/uploads` 上传项目 ZIP。
3. 服务端校验认证、文件大小和 multipart 内容。
4. 服务端保存 `source.zip` 到 session 文件目录，并记录 `uploads`。
5. `POST /v1/conversions` 创建转换任务。
6. 服务端创建 `conversion_jobs`，保存源 ZIP 引用，并向 worker 队列发送任务。
7. worker 从数据库 claim queued job。
8. worker 校验 ZIP 安全限制，调用语义引擎或 legacy 引擎转换。
9. worker 保存 `result.docx` 和 `conversion.log`。
10. worker 更新任务状态为 `completed` 或 `failed`。
11. 客户端轮询 `GET /v1/conversions/{id}` 或下载 `GET /v1/conversions/{id}/download/docx`。

兼容同步转换链路：

- `POST /api/v1/convert` 接收 multipart 项目 ZIP 与 `main_tex`，同步返回 DOCX 字节流。
- 该接口适合 MVP、烟测或旧调用方；商业化主链路建议使用上传加异步转换 API。

## 7. 数据库架构

服务使用 PostgreSQL，连接串由 `DATABASE_URL` 指定。默认值：

```text
postgres://postgres:postgres@127.0.0.1:5432/docdb
```

启动时 `DbStore::connect_from_env()` 会执行内置 SQL：

- `docs-zh/money/001_docdb_business_schema.sql`
- `docs-zh/money/002_redeem_codes_stock_status.sql`
- `docs-zh/money/003_feedback_and_session_storage.sql`
- `docs-zh/money/004_automation_rnd.sql`

初始化逻辑是幂等导向的，适合开发和预览环境自动建表。生产环境仍建议将 SQL 纳入受控迁移流程。

## 8. 文件存储

默认文件根目录为 `sessions`，实际对象路径形如：

```text
sessions/sessions/YYYY/MM/DD/{job_id}/source.zip
sessions/sessions/YYYY/MM/DD/{job_id}/result.docx
sessions/sessions/YYYY/MM/DD/{job_id}/conversion.log
```

说明：

- `FileStorage::new(PathBuf::from("sessions"))` 将根目录设为当前工作目录下的 `sessions`。
- `session_dir()` 会再拼接一层 `sessions/YYYY/MM/DD/{job_id}`。
- 数据库中保存的是相对 object key，下载时由 `FileStorage` 读取本地文件。

## 9. 静态资源托管

API 服务同时托管静态页面：

| 路径 | 静态目录 |
| --- | --- |
| `/` | `${TEX2DOC_STATIC_DIR}/home/index.html` |
| `/app` | `${TEX2DOC_STATIC_DIR}/user` |
| `/admin` | `${TEX2DOC_STATIC_DIR}/admin` |
| `/assets` | `${TEX2DOC_STATIC_DIR}/assets` |

默认 `TEX2DOC_STATIC_DIR=apps/rust-service/static`。

## 10. 安全与限制

现有安全控制：

- Bearer token 认证。
- 管理接口要求 `role=admin`。
- CORS 当前允许任意 origin、method、header。
- 全局请求体大小限制由 `MAX_BODY` 控制。
- ZIP 安全校验：文件数量、单文件大小、总解压大小、路径穿越检查。
- 兑换码以 hash、nonce、ciphertext 方式落库。
- token 以 hash 形式保存。

注意事项：

- 当前预览实现中密码使用 SHA-256 哈希，不是专用密码哈希算法。生产环境建议升级为 Argon2/bcrypt/scrypt。
- CORS 生产环境建议收敛为可信域名。
- 本地文件存储适合单机部署；多实例部署需切换为共享对象存储或挂载共享卷。

