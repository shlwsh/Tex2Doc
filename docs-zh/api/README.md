# Tex2Doc API 服务文档

本文档目录说明 `apps/rust-service` 提供的 Tex2Doc 后端 API 服务。

当前结论：

- API 服务端实现位于 `apps/rust-service`。
- 服务端技术栈为 Rust + Axum + Tokio + PostgreSQL/sqlx。
- `flutter_app`、`apps/slint-user`、`apps/browser-extension` 是客户端或前端发布单元，不包含独立 API 服务端。
- `apps/rust-service` 同时提供静态文件托管能力，用于发布产品首页、Flutter 用户端和 Flutter 管理端。

## 文档索引

| 文档 | 内容 |
| --- | --- |
| [技术架构](./technical-architecture.md) | 服务边界、模块职责、运行链路、数据与文件存储 |
| [接口清单](./api-reference.md) | 用户端、管理端、转换、反馈、自动化等 REST API 分组 |
| [部署手册](./deployment-guide.md) | 本地、预览和生产部署配置、数据库、静态资源、健康检查 |
| [使用手册](./usage-guide.md) | 前端接入、认证、上传转换、反馈、管理端操作示例 |
| [商业化扩展优化方案](./rust-service-commercial-scale-optimization-plan.md) | 并发、队列、存储、数据库、限流、监控和部署扩容改造方案 |

## 服务入口

- 源码目录：`apps/rust-service`
- Rust 包名：`doc-server`
- 默认监听地址：`127.0.0.1:2624`
- 默认 API 前缀：`/v1` 与兼容前缀 `/api/v1`
- 管理端 API 前缀：`/admin/v1`
- 健康检查：`GET /api/v1/health`

## 关联客户端

| 客户端 | API 调用方式 |
| --- | --- |
| `flutter_app` | Dart `CommercialApiClient`，默认 `http://127.0.0.1:2624/v1/`，Web 环境按当前站点解析 `/v1/` |
| `apps/slint-user` | Rust `doc-commercial-api-client` 调用商业 API |
| `apps/browser-extension` | TypeScript/WXT 前端，可通过配置接入服务端 API |
