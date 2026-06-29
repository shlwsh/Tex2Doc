# Tex2Doc 后端服务技术栈评估：Rust vs Bun/Node 对比及改进建议

> 日期：2026-06-29
> 目标：为 Rust Service（`apps/rust-service`）提供技术选型对比与渐进式演进路线
> 前置参考：`docs-zh/api/technical-architecture.md`、`docs-zh/api/rust-service-commercial-scale-optimization-plan.md`

---

## 1. Rust Service 当前画像

### 1.1 服务职责

`apps/rust-service` 是一个 Rust 单体 API 服务，定位为商业化后端的 MVP 交付单元：

| 职责 | 技术实现 |
|---|---|
| HTTP API | Axum 0.7 + Tower 中间件 |
| 异步运行时 | Tokio 多线程 runtime |
| 持久化 | PostgreSQL（sqlx 0.8），4 张核心 schema |
| 认证 | JWT Bearer Token，access + refresh 双 token |
| 文件存储 | 本地 `sessions/` 目录（生产需迁移对象存储）|
| 转换队列 | Tokio `mpsc::channel(32)` → 单 worker loop 串行执行 |
| 转换引擎 | 直接调用 `doc-core`、`doc-compiler-engine`、`doc-quality`（零序列化开销）|
| 静态托管 | 同一 Axum 服务托管 home/user/admin 三端 Web 产物 |
| 监控 | tracing + 日志，暂无结构化指标暴露 |

### 1.2 当前暴露面

```
Routes（~55 个端点）：
  /api/v1/*          用户 API（auth / usage / conversions / feedback）
  /admin/v1/*       管理 API（users / billing / redeem / releases / feedback / automation）
  /v1/*             API 别名路由（与上重复）
  /                  首页
  /app               Flutter 用户端
  /admin             Flutter 管理端
```

### 1.3 核心依赖关系

```
doc-server（apps/rust-service）
├── doc-core（crates/core）          ← 纯函数，无状态，WASM 兼容
├── doc-compiler-engine（crates/compiler-engine）  ← 语义引擎核心
├── doc-quality（crates/quality）    ← 多维质量评分
└── PostgreSQL + 本地文件系统
```

> **关键发现**：`doc-core`、`doc-compiler-engine`、`doc-quality` 是 Rust crates，同时被 WebAssembly（`crates/wasm`）和浏览器端共享。服务层调用这些 crates 不存在跨语言进程通信，是 Rust 的核心优势。

---

## 2. Rust vs Bun/Node 对比评估

### 2.1 总体结论

| 维度 | Rust（当前） | Bun | Node.js |
|---|---|---|---|
| 转换引擎集成 | **直接调用**，零开销 | FFI 或 WASM 桥接 | WASM 或子进程调用 |
| 内存效率 | **极优**（~10 MB 运行时） | 良好（基于 JS 引擎）| 良好（V8 优化）|
| 并发吞吐 | **Tokio 多线程**，无 GC 暂停 | 单线程事件循环 + Worker | 单线程事件循环 + Worker |
| 类型安全 | **编译期强类型**，无运行时类型错 | TypeScript 编译期 | TypeScript 编译期 |
| 冷启动延迟 | **无 JIT 编译延迟** | 无 JIT 编译延迟 | JIT 编译延迟 |
| 数据库驱动 | sqlx（零开销预处理语句）| Drizzle ORM / Bun:sqlite | Prisma / Drizzle / TypeORM |
| 生态成熟度 | Axum生态成熟但crate多 | 生态年轻（2023 GA）| 生态极为成熟 |
| 前端技术栈一致性 | 低（Rust vs React/TS）| **高**（TS 全栈同构）| **高**（TS 全栈同构）|
| 团队学习曲线 | **高**（Rust 所有权模型）| 低（TypeScript 原生）| 低（TypeScript 原生）|
| 编译产物部署 | **单一二进制**，无运行时依赖 | Node.js 运行时 | Node.js 运行时 |
| 生产级运维 | 需 Rust 工具链 | 需 Node.js 运行时 | 成熟容器化方案 |

### 2.2 各维度详细分析

#### 2.2.1 转换引擎集成（最关键维度）

Rust Service 直接 import 并调用 `doc-core::convert_zip()`，数据流如下：

```
HTTP Request → Axum Handler → doc-core::convert_zip() → Vec<u8>
                                  ↓
                         CPU 密集计算（LaTeX 解析 + DOCX 生成）
                                  ↓
                            返回 docx bytes
```

如果迁移到 Bun/Node：
- **路径 A（推荐）**：通过 `wasm-pack` 将 `doc-core` 编译为 WASM，Bun/Node 通过 WASM 接口调用。需处理 WASM 内存模型与 JS 侧的 bytes 序列化。
- **路径 B（不推荐）**：通过 child_process/spawn 调用 Rust CLI 子进程。每兆输入/输出数据需跨进程序列化 IPC，延迟高且复杂。

**评估**：Rust 直接调用转换引擎是该架构的核心优势，切换语言会引入不可忽视的序列化/桥接开销。

#### 2.2.2 前端技术栈一致性

| 层级 | 当前技术栈 |
|---|---|
| 前端用户端 | React + TypeScript（`apps/react-web`） |
| 前端管理端 | React + TypeScript（`apps/react-web`） |
| 后端服务 | Rust + Axum |

如果后端也迁移到 Bun/Node + TypeScript，则形成 **全栈 TypeScript 同构**：
- API 类型（DTD/Interface）可在前后端共享，编译期保证契约一致性。
- JSON Schema / OpenAPI 可从 TypeScript 类型自动生成。
- 开发体验统一，AI 辅助编程覆盖率更高。

#### 2.2.3 性能对比

| 场景 | Rust | Bun | Node.js |
|---|---|---|---|
| 空路由 GET | ~0.1 ms | ~0.5 ms | ~1 ms |
| 数据库读写 | ~5 ms（sqlx 预处理）| ~8 ms（Drizzle）| ~8 ms（Prisma）|
| 大文件上传（50 MB）| ~200 ms | ~200 ms | ~200 ms |
| 转换引擎调用（CPU 密集）| ~2-10 s（Rust CPU 优势）| ~2-10 s（通过 WASM 同速）| ~2-10 s（通过 WASM 同速）|
| 并发吞吐（RPS/核）| **极高** | 高 | 高 |

**结论**：Rust 在 CPU 密集转换上有优势，但通过 WASM，Bun/Node 可达到相同性能。Rust 的真正优势在于 **无 GC 暂停的确定性延迟**，对延迟敏感的 API 路由有轻微优势。

#### 2.2.4 运维与部署

| 维度 | Rust | Bun | Node.js |
|---|---|---|---|
| 部署包大小 | ~10-20 MB（静态链接二进制）| ~30-50 MB（含 JS 引擎）| ~30-50 MB（含 V8）|
| 容器镜像 | `FROM scratch` 或 `rust:bookworm` | `oven/bun:debian` | `node:alpine` |
| 运行时依赖 | **零**（静态链接）| glibc | Node.js 版本 |
| 内存基线 | ~8-15 MB | ~30-50 MB | ~50-80 MB |
| 多架构构建 | 需交叉编译 | 多平台预编译 | 多平台预编译 |

---

## 3. 改进建议：分阶段演进路线

### 3.0 前提：当前是否值得迁移？

**结论：当前阶段不建议整体迁移 Rust Service 到 Bun/Node。**

理由：
1. `doc-core` / `doc-compiler-engine` / `doc-quality` 是 Rust crates，云端和 WASM 两端共享。
2. Rust 直接调用转换引擎是该架构的核心竞争力，引入 WASM 桥接会损失性能优势。
3. 当前 `apps/rust-service` 已具备商业 API MVP 完整能力。
4. React 前端改造已优先进行，团队工作量应聚焦前端而非重复造后端轮子。

### 3.1 短期（0-2 个月）：深化 Rust Service 商业化能力

在保持 Rust 技术栈的前提下，推进以下改进，与 React 前端同步上线：

#### 建议 1：API 层拆解——服务可观测性

**问题**：当前主要依赖日志，缺少结构化指标。

**方案**：
- 引入 `axum-prometheus` 或自定义 `tracing::metrics`，暴露关键指标：
  - `http_requests_total{path, method, status}`
  - `conversion_jobs_active`, `conversion_jobs_completed`, `conversion_jobs_failed`
  - `conversion_duration_seconds{engine}`
  - `db_pool_connections_active`, `db_pool_connections_idle`
  - `queue_depth`
- 对接 Prometheus + Grafana，建立容量看板。

#### 建议 2：Worker 并发增强

**问题**：单 worker loop 串行执行，高并发时转换任务排队。

**方案**：
- 将 `tokio::task::spawn_blocking` 中的 CPU 密集计算保持阻塞执行（避免阻塞 async 调度器）。
- 通过环境变量 `CONVERSION_WORKERS` 控制并行 worker 数量（默认 `num_cpus::get()`）。
- 每个 worker 持有独立数据库连接，避免跨 worker 竞争。
- `mpsc::channel(32)` 改为 `mpsc::channel(256)` 以应对突发提交。

#### 建议 3：数据库连接池优化

**问题**：`max_connections(10)` 对并发请求不足。

**方案**：
- `max_connections` 改为 `num_cpus::get() * 4` 或通过 `DATABASE_POOL_SIZE` 环境变量配置。
- 用户端 API 路由和后台 worker 分组使用不同池配置。
- 引入 `deadpool-postgres` 替代 `sqlx` 内置池（预热连接，减少冷启动延迟）。

#### 建议 4：上传流式处理

**问题**：`to_bytes(body, MAX_BODY)` 全量读入内存，5 个 50 MB 并发上传时需 250 MB 缓冲。

**方案**：
- `POST /v1/uploads` 路由改用流式写入：将 body 流 pipe 到临时文件，上传完成后再读入 `FileStorage`。
- 配合 `tower-http::limit`（已在用）和超时控制，防止慢客户端攻击。
- 临时文件使用 `tempfile` crate 自动清理。

#### 建议 5：限流与熔断

**问题**：未见全局 / IP / 用户级限流。

**方案**：
- 引入 `axumGovernor`（基于 Governor）或 `tower-limit` 实现：
  - 每 IP 限速：100 req/min
  - 每用户（auth）：200 req/min
  - 转换任务提交：10 job/min/user
- 设置全局背压阈值，超限时返回 `429 Too Many Requests`。

### 3.2 中期（2-6 个月）：架构解耦与服务化

#### 建议 6：API 与 Worker 解耦部署

**问题**：当前 API 和 Worker 共进程，无法独立扩缩。

**方案**：
- 将 `apps/rust-service` 拆分为两个 crate：
  - `apps/api-server`：只负责 HTTP API，不含 worker loop。
  - `apps/conversion-worker`：只负责转换任务消费，通过 `apps/shared-queue` crate 共享数据库队列实现。
- 两者共享 `crates/` 中的所有 crates（doc-core、db_store 等）。
- 迁移后：
  - API Server 可无状态水平扩展。
  - Worker 可按队列深度独立扩容。

```
迁移后架构：
┌─────────────────────────────────────────────────────┐
│                      Frontend                        │
│           (Flutter Web / React Web)                  │
└────────────────────────┬────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────┐
│              API Gateway / LB                        │
│         (Nginx / Cloudflare / AWS ALB)              │
└──────┬──────────────────────────────────┬───────────┘
       │                                  │
┌──────▼────────┐              ┌─────────▼──────────┐
│  API Server 1 │  ...  ...    │  API Server N     │
│  (无状态)     │              │  (无状态)         │
└──────┬────────┘              └─────────┬──────────┘
       │                                  │
┌──────▼──────────────────────────────────▼───────────┐
│              PostgreSQL + Redis                     │
│   (DB-backed job queue, 共享连接池)                  │
└───────────────────────────┬─────────────────────────┘
                             │
              ┌──────────────▼──────────────┐
              │     Conversion Worker N     │
              │   (tokio multi-thread)     │
              └────────────────────────────┘
```

#### 建议 7：文件存储迁移对象存储

**问题**：本地 `sessions/` 目录无法跨实例共享。

**方案**：
- 引入 `rust-s3` 或 `aws-sdk-s3`，将上传 ZIP、结果 DOCX、日志写入 S3/MinIO。
- `FileStorage` trait 抽象化：
  - `local: FileStorage`（开发 / 单机）
  - `s3: S3Storage`（生产 / 多实例）
- 迁移脚本：将现有 `sessions/` 文件异步上传到 S3，写入新记录时直接写 S3。

#### 建议 8：引入 Redis 辅助

**问题**：任务队列依赖 PostgreSQL（`claim_next_job` 数据库锁竞争）。

**方案**：
- 引入 Redis 列表作为轻量队列：`LPUSH job_id`，`BRPOP` 消费。
- PostgreSQL 保留为持久化存储（任务状态、用户数据、计费）。
- 好处：
  - Redis 操作是微秒级，PostgreSQL 是毫秒级。
  - Worker 扩缩时无需担心数据库连接竞争。
  - 支持 `BRPOP` 超时等待，降低轮询开销。

### 3.3 长期（6 个月+）：按需评估 Bun/Node 迁移

在以下信号出现时，再认真评估迁移后端到 Bun/Node 的收益/成本比：

**触发信号（满足任一即评估）**：
1. 团队中有 2 名以上工程师明确掌握 Bun/Node 全栈开发。
2. WASM 版的 `doc-core` 性能、稳定性和功能覆盖率达到 Rust 原生的 95% 以上。
3. Rust 招聘或维护成本显著高于预期。
4. 有强烈的 SSR（服务端渲染）需求，Bun/Node SSR 与 React 前端同构更自然。

**如果决定迁移，推荐路径**：

```
Phase 1: 将 doc-core/doc-compiler-engine 完善 WASM 支持
          - 确保所有核心 API 在 WASM 环境下通过测试
          - 构建产物: doc-engine.wasm

Phase 2: 搭建 Node.js/Bun API 服务原型（独立项目 apps/api-node）
          - Express.js 或 Hono 或 Bun 原生 HTTP
          - 通过 @aspect/wasm 调用 doc-engine.wasm
          - 保持 rust-service 正常运行，作为对照

Phase 3: 功能对照测试
          - 两套服务 API 行为 100% 对齐
          - 性能基准：P50/P95/P99 对比
          - 内存 / 并发吞吐对比

Phase 4: 灰度切流
          - 10% / 50% / 100% 逐步切流
          - 保留 rust-service 为降级路径

Phase 5: 下线 rust-service（可选）
          - 确认稳定后归档
          - 或作为特殊场景备用（极高并发 / 特殊硬件加速）
```

---

## 4. 面向 React 前端的协作优化

### 4.1 共享类型包

**问题**：当前 React SDK（`apps/react-web/src/api/types.ts`）手工维护 DTO 类型，与 Rust 侧 `serde` 结构体无自动同步机制。

**建议**：创建 `packages/api-types`（TypeScript），Rust 端通过 `cargo generate-rust-types-from-openapi` 从 OpenAPI spec 生成 TS 类型包。工程结构变为：

```
Tex2Doc/
├── packages/
│   └── api-types/          # 共享 TS 类型，publish 到内部 registry
│       ├── src/index.ts    # ApiUser, ConversionJob, RedeemCode, ...
│       └── package.json
├── apps/
│   ├── react-web/          # import from @tex2doc/api-types
│   └── rust-service/       # 运行时生成 OpenAPI spec
└── crates/                 # Rust crates
```

### 4.2 API SDK 增强

**建议**：
- React SDK 增加请求拦截器：自动注入 `Authorization: Bearer {token}`。
- 增加重试逻辑（指数退避），应对暂时性网络抖动。
- 增加请求取消（`AbortController` 集成），用户导航离开时取消 pending 请求。

---

## 5. 总结与优先级

| 优先级 | 改进项 | 工作量 | 价值 | 推荐行动 |
|---|---|---|---|---|
| **P0** | Worker 并发增强（多 worker）| 小 | 高 | 立即实现 |
| **P0** | 数据库连接池动态配置 | 小 | 高 | 立即实现 |
| **P0** | 上传流式处理 | 中 | 高 | 立即实现 |
| **P1** | API 层可观测性（Prometheus metrics）| 中 | 高 | 下个迭代 |
| **P1** | 限流与熔断 | 中 | 高 | 下个迭代 |
| **P1** | API + Worker 解耦部署 | 中 | 中 | 下个迭代 |
| **P2** | 文件存储迁移 S3 | 中 | 中 | Beta 上线后 |
| **P2** | 引入 Redis 辅助队列 | 中 | 中 | Beta 上线后 |
| **P2** | 共享类型包（packages/api-types）| 中 | 中 | React 联调期间 |
| **P3** | Bun/Node 迁移评估 | 大 | 待定 | 触发信号出现时 |
| **P3** | SSR 同构（如果有）| 大 | 待定 | 有需求时评估 |

### 核心结论

**Rust Service 当前技术选型是合理的，不应因 React 前端已改造为 TypeScript 就冲动迁移后端。** Rust 的核心优势（直接调用转换引擎、无 GC 停顿的确定性性能、编译期类型安全）在当前阶段是真实且重要的。

真正的改进空间在于：
1. **提高并发能力**（多 worker、连接池、流式上传、限流）。
2. **提升可观测性**（Prometheus metrics + Grafana 看板）。
3. **架构解耦**（API/Worker 分离，文件存储外置）。
4. **前后端类型共享**（packages/api-types）。

Bun/Node 迁移作为**长期选项**保留，但应作为有明确业务信号驱动的主动决策，而非追赶技术潮流的冲动选择。
