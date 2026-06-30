# Tex2Doc API 服务部署手册

## 1. 部署对象

部署对象为 Rust 包：

```text
apps/rust-service
```

Cargo 包名和二进制名：

```text
doc-server
```

服务职责：

- HTTP API。
- PostgreSQL 持久化。
- 转换 worker。
- 本地文件存储。
- 静态资源托管。

## 2. 环境要求

| 组件 | 要求 |
| --- | --- |
| Rust | workspace 当前 `rust-toolchain.toml` 指定版本 |
| PostgreSQL | 建议 14+，当前默认连接本机 `docdb` |
| 网络端口 | 默认 `2624` |
| 文件系统 | 需要可写 `sessions` 目录 |
| 静态资源 | 可选，放置到 `apps/rust-service/static` 或 `TEX2DOC_STATIC_DIR` |

## 3. 环境变量

| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `DOC_SERVER_ADDR` | `127.0.0.1:2624` | API 服务监听地址 |
| `DATABASE_URL` | `postgres://postgres:postgres@127.0.0.1:5432/docdb` | PostgreSQL 连接串 |
| `TEX2DOC_STATIC_DIR` | `apps/rust-service/static` | 静态资源根目录 |
| `TEX2DOC_BOOTSTRAP_ADMIN_EMAIL` | 无 | 启动时创建或更新管理员账号 |
| `TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD` | 无 | 管理员初始密码 |
| `REDEEM_CODE_PEPPER` | `tex2doc-preview-pepper` | 兑换码 hash pepper |
| `REDEEM_CODE_MASTER_KEY` | `tex2doc-preview-master-key` | 兑换码简易加密 key |
| `RUST_LOG` | 代码默认 `info` | tracing 日志过滤 |

生产环境必须显式设置：

- `DATABASE_URL`
- `TEX2DOC_BOOTSTRAP_ADMIN_EMAIL`
- `TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD`
- `REDEEM_CODE_PEPPER`
- `REDEEM_CODE_MASTER_KEY`

## 4. 本地开发启动

1. 启动 PostgreSQL，并准备数据库：

```powershell
createdb docdb
```

如果本机账号不是默认 `postgres/postgres`，设置：

```powershell
$env:DATABASE_URL="postgres://user:password@127.0.0.1:5432/docdb"
```

2. 启动服务：

```powershell
cargo run -p doc-server
```

3. 健康检查：

```powershell
Invoke-RestMethod http://127.0.0.1:2624/api/v1/health
```

预期响应：

```json
{
  "status": "ok"
}
```

## 5. 指定端口启动

```powershell
$env:DOC_SERVER_ADDR="127.0.0.1:2624"
cargo run -p doc-server
```

对外部署时可监听：

```powershell
$env:DOC_SERVER_ADDR="0.0.0.0:2624"
cargo run -p doc-server --release
```

## 6. 构建发布二进制

```powershell
cargo build -p doc-server --release
```

产物通常位于：

```text
target/release/doc-server.exe
```

运行：

```powershell
$env:DATABASE_URL="postgres://user:password@db-host:5432/docdb"
$env:DOC_SERVER_ADDR="0.0.0.0:2624"
.\target\release\doc-server.exe
```

## 7. 数据库初始化

服务启动时自动执行以下 SQL：

```text
docs-zh/money/001_docdb_business_schema.sql
docs-zh/money/002_redeem_codes_stock_status.sql
docs-zh/money/003_feedback_and_session_storage.sql
docs-zh/money/004_automation_rnd.sql
```

开发环境可依赖自动初始化。生产环境建议：

1. 先在变更窗口手动执行 SQL。
2. 备份数据库。
3. 再启动新版本服务。

## 8. 管理员账号引导

设置以下环境变量后，服务启动时会创建或更新管理员账号：

```powershell
$env:TEX2DOC_BOOTSTRAP_ADMIN_EMAIL="admin@example.com"
$env:TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD="change-me"
```

启动后可调用：

```powershell
$body = @{
  email = "admin@example.com"
  password = "change-me"
} | ConvertTo-Json

Invoke-RestMethod `
  -Method Post `
  -Uri http://127.0.0.1:2624/v1/auth/login `
  -ContentType "application/json" `
  -Body $body
```

## 9. 静态资源部署

默认静态目录：

```text
apps/rust-service/static/home
apps/rust-service/static/user
apps/rust-service/static/admin
apps/rust-service/static/assets
```

访问映射：

| URL | 文件 |
| --- | --- |
| `/` | `home/index.html` |
| `/app` | `user/index.html` |
| `/admin` | `admin/index.html` |
| `/assets/*` | `assets/*` |

自定义目录：

```powershell
$env:TEX2DOC_STATIC_DIR="D:\deploy\tex2doc-static"
```

建议将 Flutter Web 构建产物分别放入：

```text
{TEX2DOC_STATIC_DIR}/user
{TEX2DOC_STATIC_DIR}/admin
```

## 10. 文件存储

服务默认在工作目录下创建：

```text
sessions/
```

其中保存：

- 上传源文件：`source.zip`
- 转换结果：`result.docx`
- 转换日志：`conversion.log`

单机部署无需额外配置。多实例部署必须保证所有实例都能访问同一份文件：

- 共享卷。
- 网络文件系统。
- 或后续改造为对象存储。

## 11. 反向代理建议

生产环境建议在 Nginx/Caddy/Traefik 后运行 `doc-server`：

```text
https://tex2doc.example.com/      -> doc-server /
https://tex2doc.example.com/v1    -> doc-server /v1
https://tex2doc.example.com/admin -> doc-server /admin
```

反向代理需要注意：

- 上传接口允许较大的 request body。
- `/v1/uploads`、`/api/v1/convert` 的 body size 需要与服务端限制匹配。
- 如果前后端同域部署，Flutter Web 默认会把 API base URL 解析为当前 origin 的 `/v1/`。

## 12. 健康检查与烟测

健康检查：

```powershell
Invoke-RestMethod http://127.0.0.1:2624/api/v1/health
```

版本：

```powershell
Invoke-RestMethod http://127.0.0.1:2624/api/v1/version
```

注册登录：

```powershell
$body = @{ email = "demo@example.com"; password = "secret" } | ConvertTo-Json
Invoke-RestMethod -Method Post -Uri http://127.0.0.1:2624/v1/auth/register -ContentType "application/json" -Body $body
Invoke-RestMethod -Method Post -Uri http://127.0.0.1:2624/v1/auth/login -ContentType "application/json" -Body $body
```

## 13. 日志与排障

开启详细日志：

```powershell
$env:RUST_LOG="debug,tower_http=debug"
cargo run -p doc-server
```

常见问题：

| 现象 | 排查方向 |
| --- | --- |
| 启动时报数据库连接失败 | 检查 `DATABASE_URL`、PostgreSQL 是否启动、账号权限 |
| 健康检查无法访问 | 检查 `DOC_SERVER_ADDR`、端口占用、防火墙 |
| 上传失败 | 检查请求体大小、ZIP 是否包含危险路径、文件数量和解压体积 |
| 转换任务一直 queued | 检查 worker 日志、数据库 job 状态、是否有 panic |
| 下载 DOCX 404 | 检查 `sessions` 文件是否存在、数据库 object key 是否正确 |
| 管理接口 401 | 检查 token 是否有效、用户 role 是否为 `admin` |

## 14. 生产安全建议

上线前建议完成：

- 将 CORS 从 `Any` 收敛到正式域名。
- 使用专用密码哈希算法替代普通 SHA-256。
- 将兑换码 key 和 pepper 改为高强度随机值。
- 对 `DATABASE_URL` 和管理员密码使用密钥管理。
- 将自动建表迁移改为受控迁移。
- 增加 HTTPS、访问日志、备份和监控。
- 对上传和转换增加速率限制。

