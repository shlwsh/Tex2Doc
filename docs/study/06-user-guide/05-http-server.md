# HTTP 服务端使用

> 本节描述 **HTTP 服务端** 形态的使用方式。最适合：企业内部集成、跨域 API、批处理。

---

## 1. 启动服务

### 1.1 开发模式

```bash
# 仓库根
cargo run -p doc-server
# 默认监听 0.0.0.0:2624
```

### 1.2 生产模式（release）

```bash
cargo build -p doc-server --release
./target/release/doc-server
```

### 1.3 自定义监听地址

```bash
DOC_SERVER_ADDR=127.0.0.1:9000 cargo run -p doc-server --release
```

或环境变量：

```bash
export DOC_SERVER_ADDR=0.0.0.0:9000
./target/release/doc-server
```

### 1.4 日志级别

`tracing-subscriber` 用 `RUST_LOG` 控制：

```bash
RUST_LOG=info,doc_server=debug cargo run -p doc-server --release
# 或：RUST_LOG=warn cargo run -p doc-server --release
```

---

## 2. API

### 2.1 `GET /api/v1/health`

健康检查。

```bash
curl http://127.0.0.1:2624/api/v1/health
```

响应：
```json
{"status":"ok"}
```

### 2.2 `GET /api/v1/version`

版本信息。

```bash
curl http://127.0.0.1:2624/api/v1/version
```

响应：
```json
{"name":"doc-server","version":"0.1.0"}
```

### 2.3 `POST /api/v1/convert`

* **Content-Type**: `multipart/form-data`
* **Fields**:
  * `file`（必填）：项目 zip 字节
  * `main_tex`（可选）：主 .tex 路径，缺省 `main-jos.tex`

#### curl

```bash
curl -X POST http://127.0.0.1:2624/api/v1/convert \
  -F "file=@examples/paper3/upload.zip" \
  -F "main_tex=main-jos.tex" \
  -o out.docx

file out.docx
# Microsoft Word 2007+
```

#### Node.js (axios)

```javascript
const axios = require('axios');
const FormData = require('form-data');
const fs = require('fs');

const form = new FormData();
form.append('file', fs.createReadStream('project.zip'));
form.append('main_tex', 'main.tex');

axios.post('http://127.0.0.1:2624/api/v1/convert', form, {
  headers: form.getHeaders(),
  responseType: 'arraybuffer',
  maxContentLength: 50 * 1024 * 1024,
}).then(res => {
  fs.writeFileSync('out.docx', res.data);
  console.log('✅', res.data.length, 'bytes');
}).catch(err => {
  console.error('❌', err.response?.status, err.message);
});
```

#### Python (requests)

```python
import requests
with open('project.zip', 'rb') as f:
    r = requests.post(
        'http://127.0.0.1:2624/api/v1/convert',
        files={'file': ('project.zip', f, 'application/zip')},
        data={'main_tex': 'main.tex'},
    )
if r.status_code == 200:
    with open('out.docx', 'wb') as g:
        g.write(r.content)
    print(f'✅ {len(r.content)} bytes')
else:
    print(f'❌ {r.status_code} {r.text}')
```

#### Go (net/http)

```go
import (
    "bytes"
    "io"
    "mime/multipart"
    "net/http"
    "os"
)

func main() {
    body := &bytes.Buffer{}
    writer := multipart.NewWriter(body)
    file, _ := os.Open("project.zip")
    part, _ := writer.CreateFormFile("file", "project.zip")
    io.Copy(part, file)
    file.Close()
    writer.WriteField("main_tex", "main.tex")
    writer.Close()

    r, _ := http.Post("http://127.0.0.1:2624/api/v1/convert", writer.FormDataContentType(), body)
    defer r.Body.Close()
    out, _ := os.Create("out.docx")
    io.Copy(out, r.Body)
}
```

### 2.4 错误响应

```http
HTTP/1.1 400 Bad Request
Content-Type: application/json

{"error":"parse","message":"解析错误：主文件非 UTF-8：..."}
```

错误码：

| HTTP Status | `error` 字段 | 触发条件 |
|-------------|-------------|----------|
| 400 | `io` / `missing_field` / `parse` | IO / multipart 字段缺失 / 解析错误 |
| 422 | `unsupported` | 图片格式不支持等 |
| 500 | `internal` | 序列化错误（极少） |

### 2.5 限制

* **请求体 ≤ 50 MiB**（`tower_http::limit::RequestBodyLimitLayer` + `axum::body::to_bytes(_, MAX_BODY)`）。
* **docx 至少 4 KiB**（routes.rs 内部断言，< 4 KiB 返 400）。
* **docx 头必须是 `PK\x03\x04`**（routes.rs 内部断言，否 400）。

---

## 3. 集成示例

### 3.1 期刊投稿系统集成

```python
# 提交论文 → 转换 → 提交
def submit_paper(tex_zip_path: str, journal_id: str) -> str:
    r = requests.post(
        f'https://journal-api.example.com/papers/{journal_id}/submit-docx',
        files={'file': open(tex_zip_path, 'rb')},
        data={'main_tex': 'main.tex'},
        timeout=60,
    )
    r.raise_for_status()
    return r.content  # docx 字节
```

### 3.2 Web 前端集成

```javascript
async function convertInBrowser(zipFile: File, mainTex: string) {
  const form = new FormData();
  form.append('file', zipFile);
  form.append('main_tex', mainTex);
  const r = await fetch('https://doc-engine.example.com/api/v1/convert', {
    method: 'POST',
    body: form,
  });
  if (!r.ok) throw new Error(`HTTP ${r.status}`);
  return await r.blob();
}
```

### 3.3 CI 验证

```bash
# GitHub Actions
- name: Verify docx via doc-server
  run: |
    cargo build -p doc-server --release
    ./target/release/doc-server &
    SERVER_PID=$!
    sleep 2
    curl -X POST http://127.0.0.1:2624/api/v1/convert \
      -F "file=@examples/paper3/upload.zip" \
      -F "main_tex=main-jos.tex" \
      -o out.docx
    file out.docx | grep -q "Microsoft Word" || (kill $SERVER_PID && exit 1)
    kill $SERVER_PID
```

---

## 4. 部署

### 4.1 systemd（Linux）

```ini
# /etc/systemd/system/doc-server.service
[Unit]
Description=Doc-engine HTTP server
After=network.target

[Service]
Type=simple
User=doc-engine
Environment=DOC_SERVER_ADDR=0.0.0.0:2624
Environment=RUST_LOG=info
ExecStart=/opt/doc-engine/doc-server
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now doc-server
sudo systemctl status doc-server
```

### 4.2 Docker

```dockerfile
# Dockerfile
FROM rust:1.82 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p doc-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/doc-server /usr/local/bin/doc-server
ENV DOC_SERVER_ADDR=0.0.0.0:2624
EXPOSE 2624
CMD ["/usr/local/bin/doc-server"]
```

```bash
docker build -t doc-engine/server:0.1.0 .
docker run -d -p 2624:2624 --name doc-server doc-engine/server:0.1.0
```

### 4.3 反向代理（Nginx）

```nginx
upstream doc_server {
    server 127.0.0.1:2624;
    keepalive 32;
}

server {
    listen 443 ssl;
    server_name doc-engine.example.com;
    ssl_certificate /etc/letsencrypt/live/doc-engine.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/doc-engine.example.com/privkey.pem;

    client_max_body_size 60M;  # 大于 doc-server 的 50 MiB 限制

    location / {
        proxy_pass http://doc_server;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_read_timeout 120s;  # 大文件转换可能慢
        proxy_send_timeout 120s;
    }
}
```

### 4.4 Kubernetes

```yaml
# doc-server-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: doc-server
spec:
  replicas: 2
  selector:
    matchLabels: { app: doc-server }
  template:
    metadata:
      labels: { app: doc-server }
    spec:
      containers:
        - name: doc-server
          image: doc-engine/server:0.1.0
          ports:
            - containerPort: 2624
          env:
            - name: DOC_SERVER_ADDR
              value: "0.0.0.0:2624"
            - name: RUST_LOG
              value: "info"
          resources:
            requests:
              cpu: "500m"
              memory: "256Mi"
            limits:
              cpu: "2"
              memory: "1Gi"
---
apiVersion: v1
kind: Service
metadata:
  name: doc-server
spec:
  selector: { app: doc-server }
  ports:
    - port: 80
      targetPort: 2624
```

---

## 5. 监控

### 5.1 健康检查

```bash
# Kubernetes liveness probe
livenessProbe:
  httpGet: { path: /api/v1/health, port: 2624 }
  initialDelaySeconds: 5
  periodSeconds: 10
```

### 5.2 日志

`tracing-subscriber` 输出到 stderr：

```
2026-06-14T12:00:00Z  INFO doc_server: doc-server listening on http://0.0.0.0:2624
2026-06-14T12:00:01Z  INFO request{method=POST path=/api/v1/convert}: doc_server::routes: 200 OK
```

JSON 输出（生产推荐）：

```bash
RUST_LOG=info cargo run -p doc-server 2>&1 | tee -a /var/log/doc-server.json
```

### 5.3 Prometheus（V2 路线）

* 集成 `axum-prometheus` 暴露 `/metrics`。
* 监控：请求数 / 延迟直方图 / 错误率 / docx 字节数。

---

## 6. 测试

### 6.1 集成测试（`crates/server/tests/api.rs`）

用 `reqwest` + `rustls-tls` 测三接口。运行：

```bash
cargo test -p doc-server --test api
```

### 6.2 端到端（Node）

```bash
node scripts/e2e_server.mjs
```

* 启 doc-server，curl 三个接口，断言。

---

## 7. 安全考虑

### 7.1 输入限制

* 单请求体 ≤ 50 MiB（多层防护：Nginx `client_max_body_size` + doc-server `RequestBodyLimitLayer`）。
* docx 大小 ≥ 4 KiB（routes.rs）。
* docx 头 `PK\x03\x04`（routes.rs）。
* zip `..` 路径拒绝（convert_zip）。

### 7.2 鉴权（V2 计划）

当前**无鉴权**。生产建议：

* API Key：`X-API-Key` header。
* OAuth2 / JWT。
* mTLS（双向 TLS）。

### 7.3 TLS

* 必须 HTTPS（生产）。
* 用 Let's Encrypt / cert-manager。

### 7.4 速率限制

V2 计划：用 `tower-governor` 加限流。

---

## 8. 进一步阅读

* [01-cli-and-script.md](./01-cli-and-script.md) — CLI / 脚本
* [04-architecture/03-frontend-bridges.md](../04-architecture/03-frontend-bridges.md) — HTTP 集成详情
* [07-deployment/04-server-deploy.md](../07-deployment/04-server-deploy.md) — 部署细节
