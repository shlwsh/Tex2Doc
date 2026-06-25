# 服务端部署
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



> 本节描述 `doc-server` 的生产部署流程：二进制部署、Docker 容器、Kubernetes。

---

## 1. 准备

### 1.1 系统要求

| 资源 | 最低 | 推荐 |
|------|------|------|
| CPU | 1 vCPU | 2+ vCPU |
| 内存 | 256 MiB | 512 MiB – 1 GiB |
| 磁盘 | 100 MB（仅 binary） | 1 GB（含日志） |
| 操作系统 | Linux / macOS / Windows | Linux（生产） |

### 1.2 依赖

* 运行环境：C 标准库 + 操作系统 glibc。
* 数据库：PostgreSQL (v15+)，用于计费和充值功能持久化（`doc-server` 默认使用 `sqlx` 驱动）。
* 无外部 runtime 依赖（无需安装 Node.js / Python / JVM 在服务器）。

---

## 2. 部署方式

### 2.1 直接运行（二进制）

```bash
# 1. 上传 target/release/doc-server（Linux）/ doc-server.exe（Windows）到服务器
scp target/release/doc-server user@server:/opt/doc-engine/

# 2. 登录服务器
ssh user@server
cd /opt/doc-engine
chmod +x doc-server

# 3. 启动
DOC_SERVER_ADDR=0.0.0.0:2624 RUST_LOG=info ./doc-server
```

### 2.2 systemd 服务

创建 `/etc/systemd/system/tex2doc-server.service`：

```ini
[Unit]
Description=Tex2Doc Rust API server
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=ubuntu
Group=ubuntu
WorkingDirectory=/opt/tex2doc/current
EnvironmentFile=/opt/tex2doc/shared/env/doc-server.env
ExecStart=/opt/tex2doc/current/server/doc-server
Restart=always
RestartSec=3

# 安全加固
NoNewPrivileges=true
PrivateTmp=true
ReadWritePaths=/opt/tex2doc/shared

[Install]
WantedBy=multi-user.target
```

```bash
# 1. 准备配置与共享目录
sudo mkdir -p /opt/tex2doc/shared/env /opt/tex2doc/shared/sessions /opt/tex2doc/shared/logs
sudo chown -R ubuntu:ubuntu /opt/tex2doc

# 2. 配置环境文件（/opt/tex2doc/shared/env/doc-server.env）
# 内容参照 docs-zh/install/github-auto-deploy-flutter-web-and-server.md 中定义

# 3. 启用服务
sudo systemctl daemon-reload
sudo systemctl enable --now tex2doc-server
sudo systemctl status tex2doc-server

# 4. 查看日志
sudo journalctl -u tex2doc-server -f
```

### 2.3 Docker

#### Dockerfile

```dockerfile
# 多阶段构建
FROM rust:1.82-slim AS builder
WORKDIR /build

# 安装构建依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

# 复制源码
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# 编译（仅 doc-server）
RUN cargo build --release -p doc-server

# 运行镜像（极小）
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates && rm -rf /var/lib/apt/lists/* \
    && useradd -r -s /bin/false -u 1001 doc-engine

COPY --from=builder /build/target/release/doc-server /usr/local/bin/doc-server

USER doc-engine
EXPOSE 2624
ENV DOC_SERVER_ADDR=0.0.0.0:2624
ENV RUST_LOG=info

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD wget -qO- http://127.0.0.1:2624/api/v1/health || exit 1

CMD ["/usr/local/bin/doc-server"]
```

#### 构建与运行

```bash
docker build -t doc-engine/server:0.1.0 -f Dockerfile.server .

# 测试
docker run --rm -p 2624:2624 doc-engine/server:0.1.0
curl http://127.0.0.1:2624/api/v1/health

# 后台运行
docker run -d --name doc-server -p 2624:2624 \
  --restart unless-stopped \
  -e RUST_LOG=info \
  -v /var/log/doc-server:/var/log/doc-server \
  doc-engine/server:0.1.0
```

#### docker-compose

```yaml
# docker-compose.yml
version: '3.8'
services:
  doc-server:
    image: doc-engine/server:0.1.0
    restart: unless-stopped
    ports:
      - "2624:2624"
    environment:
      DOC_SERVER_ADDR: "0.0.0.0:2624"
      RUST_LOG: "info"
    healthcheck:
      test: ["CMD", "wget", "-qO-", "http://127.0.0.1:2624/api/v1/health"]
      interval: 30s
      timeout: 5s
      retries: 3
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 1G
```

```bash
docker compose up -d
```

### 2.4 Kubernetes

#### 部署清单

```yaml
# doc-server-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: doc-server
  namespace: doc-engine
  labels: { app: doc-server }
spec:
  replicas: 2
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  selector:
    matchLabels: { app: doc-server }
  template:
    metadata:
      labels: { app: doc-server }
    spec:
      containers:
        - name: doc-server
          image: doc-engine/server:0.1.0
          imagePullPolicy: IfNotPresent
          ports:
            - name: http
              containerPort: 2624
          env:
            - name: DOC_SERVER_ADDR
              value: "0.0.0.0:2624"
            - name: RUST_LOG
              value: "info"
          resources:
            requests:
              cpu: 100m
              memory: 128Mi
            limits:
              cpu: "2"
              memory: 1Gi
          livenessProbe:
            httpGet: { path: /api/v1/health, port: http }
            initialDelaySeconds: 10
            periodSeconds: 30
            timeoutSeconds: 5
            failureThreshold: 3
          readinessProbe:
            httpGet: { path: /api/v1/health, port: http }
            initialDelaySeconds: 5
            periodSeconds: 10
            timeoutSeconds: 3
            failureThreshold: 2
          securityContext:
            runAsNonRoot: true
            runAsUser: 1001
            allowPrivilegeEscalation: false
            readOnlyRootFilesystem: true
            capabilities:
              drop: ["ALL"]

---
# doc-server-service.yaml
apiVersion: v1
kind: Service
metadata:
  name: doc-server
  namespace: doc-engine
  labels: { app: doc-server }
spec:
  type: ClusterIP
  selector: { app: doc-server }
  ports:
    - name: http
      port: 80
      targetPort: http
      protocol: TCP
  sessionAffinity: None
```

#### HPA（自动伸缩）

```yaml
# doc-server-hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: doc-server
  namespace: doc-engine
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: doc-server
  minReplicas: 2
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
```

#### Ingress

```yaml
# doc-server-ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: doc-server
  namespace: doc-engine
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/proxy-body-size: "60m"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "120"
    nginx.ingress.kubernetes.io/proxy-send-timeout: "120"
spec:
  ingressClassName: nginx
  tls:
    - hosts: [doc-engine.example.com]
      secretName: doc-engine-tls
  rules:
    - host: doc-engine.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: doc-server
                port: { number: 80 }
```

部署：

```bash
kubectl apply -f doc-server-deployment.yaml
kubectl apply -f doc-server-service.yaml
kubectl apply -f doc-server-ingress.yaml
kubectl apply -f doc-server-hpa.yaml

# 验证
kubectl get pods -n doc-engine
kubectl get svc -n doc-engine
curl https://doc-engine.example.com/api/v1/health
```

### 2.5 其它平台

| 平台 | 部署方式 |
|------|----------|
| **Heroku** | Container build + Procfile |
| **Fly.io** | `fly launch` + Dockerfile |
| **Railway** | GitHub 自动部署 |
| **Cloud Run** | Docker + Cloud Build |
| **AWS Fargate** | ECS Task Definition + ECR |
| **Azure Container Apps** | `az containerapp up` |

---

## 3. 反向代理

### 3.1 Nginx

配置 `/etc/nginx/sites-available/tex2doc` 以承载 Flutter Home、User、Admin 多段 Web，并反代 Rust 服务端 API：

```nginx
upstream doc_server {
    server 127.0.0.1:2624;
    keepalive 32;
}

server {
    listen 80;
    server_name doc-engine.example.com; # 替换为实际域名或 IP

    client_max_body_size 60m;

    # 1. Flutter 主端 / Home 路由
    root /opt/tex2doc/current/static/home;
    index index.html;

    location / {
        try_files $uri $uri/ /index.html;
    }

    # 2. Flutter 用户端 / User 路由
    location /user/ {
        alias /opt/tex2doc/current/static/user/;
        try_files $uri $uri/ /user/index.html;
    }

    # 3. Flutter 管理端 / Admin 路由
    location /admin/ {
        alias /opt/tex2doc/current/static/admin/;
        try_files $uri $uri/ /admin/index.html;
    }

    # 4. API 反代 (doc-server)
    location /api/ {
        proxy_pass http://doc_server;
        proxy_http_version 1.1;
        proxy_set_header Connection "";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 120s;
        proxy_send_timeout 120s;
        proxy_buffering off;
        proxy_request_buffering off;  # 允许大文件流式上传
    }

    location /v1/ {
        proxy_pass http://doc_server;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location /admin/v1/ {
        proxy_pass http://doc_server;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### 3.2 Caddy

```caddyfile
doc-engine.example.com {
    reverse_proxy 127.0.0.1:2624 {
        transport http {
            dial_timeout 5s
            response_header_timeout 120s
        }
    }
    encode zstd gzip
    request_body {
        max_size 60MB
    }
}
```

### 3.3 Traefik

```yaml
# docker-compose.traefik.yml
services:
  traefik:
    image: traefik:v3.0
    command:
      - --providers.docker=true
      - --entrypoints.websecure.address=:443
      - --certificatesresolvers.letsencrypt.acme.tlschallenge=true
      - [email protected]
      - --certificatesresolvers.letsencrypt.acme.storage=/letsencrypt.json
    ports: ["443:443"]
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - letsencrypt:/letsencrypt

  doc-server:
    image: doc-engine/server:0.1.0
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.doc-server.rule=Host(`doc-engine.example.com`)"
      - "traefik.http.routers.doc-server.tls.certresolver=letsencrypt"
      - "traefik.http.services.doc-server.loadbalancer.server.port=2624"

volumes:
  letsencrypt:
```

---

## 4. TLS / HTTPS

### 4.1 Let's Encrypt（certbot）

```bash
sudo apt install certbot python3-certbot-nginx
sudo certbot --nginx -d doc-engine.example.com

# 自动续期（certbot 包默认启用 systemd timer）
sudo systemctl status certbot.timer
```

### 4.2 cert-manager（Kubernetes）

```bash
kubectl apply -f https://github.com/jetstack/cert-manager/releases/download/v1.14.0/cert-manager.yaml

# ClusterIssuer
cat <<EOF | kubectl apply -f -
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: [email protected]
    privateKeySecretRef:
      name: letsencrypt-prod
    solvers:
      - http01:
          ingress:
            class: nginx
EOF
```

### 4.3 自签名（仅内网）

```bash
openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
  -keyout /etc/ssl/private/doc-engine.key \
  -out /etc/ssl/certs/doc-engine.crt \
  -subj "/CN=doc-engine.internal"

# Nginx 中
ssl_certificate /etc/ssl/certs/doc-engine.crt;
ssl_certificate_key /etc/ssl/private/doc-engine.key;
```

---

## 5. 监控

### 5.1 基础健康检查

```bash
curl -f http://127.0.0.1:2624/api/v1/health
```

### 5.2 日志

`tracing-subscriber` 输出到 stderr。生产推荐 JSON 格式：

```bash
RUST_LOG=info ./doc-server 2>&1 | tee -a /var/log/doc-server/app.json
```

或重定向到 systemd journal：

```ini
[Service]
StandardOutput=journal
StandardError=journal
```

`journalctl -u doc-server -f`。

### 5.3 Prometheus（V2 路线）

集成 `axum-prometheus`：

```rust
// crates/server/src/main.rs
use axum_prometheus::PrometheusLayer;

let app = Router::new()
    .route("/api/v1/health", get(health))
    .route("/api/v1/convert", post(convert))
    .route("/metrics", get(metrics))   // 新增
    .layer(PrometheusLayer::new());
```

Grafana Dashboard：
* 请求速率：`rate(http_requests_total[5m])`
* 错误率：`rate(http_requests_total{status=~"5.."}[5m]) / rate(http_requests_total[5m])`
* P99 延迟：`histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m]))`

### 5.4 Sentry（错误追踪）

V2 路线：用 `sentry::init` 集成。

---

## 6. 备份与恢复

### 6.1 备份内容

* **配置文件**：`/opt/tex2doc/shared/env/doc-server.env` 环境变量文件。
* **TLS 证书**：`/etc/letsencrypt/`。
* **日志**：`/opt/tex2doc/shared/logs/` 目录以及 systemd journal 日志。
* **数据库持久数据**：PostgreSQL 生产数据库 `docdb`（包含用户与账目信息）。

#### 备份数据库命令 (pg_dump)

```bash
pg_dump -U postgres -d docdb -F c -b -v -f /opt/tex2doc/shared/backups/docdb_$(date +%Y%m%d).backup
```

### 6.2 灾难恢复

```bash
# 1. 还原数据库
createdb -U postgres docdb
pg_restore -U postgres -d docdb -v /opt/tex2doc/shared/backups/docdb_xxxx.backup

# 2. 还原配置文件与证书
# 3. 重新激活应用和静态资源（通过 GitHub CD 触发或手动部署）
sudo systemctl restart tex2doc-server
sudo nginx -t
sudo systemctl reload nginx
```

RTO（恢复时间目标）：< 30 分钟。
RPO（恢复点目标）：根据 pg_dump 定时任务的频率而定（推荐每 12 小时备份一次）。

---

## 7. 安全加固

### 7.1 防火墙

```bash
sudo ufw allow 22/tcp          # SSH
sudo ufw allow 80/tcp          # HTTP
sudo ufw allow 443/tcp         # HTTPS
sudo ufw enable
```

### 7.2 fail2ban

```bash
sudo apt install fail2ban
sudo systemctl enable fail2ban
```

### 7.3 自动更新

```bash
sudo apt install unattended-upgrades
sudo dpkg-reconfigure -plow unattended-upgrades
```

### 7.4 用户与权限

```bash
# 限制 doc-server binary 权限
sudo chown root:ubuntu /opt/tex2doc/current/server/doc-server
sudo chmod 750 /opt/tex2doc/current/server/doc-server
```

### 7.5 容器安全

* 基础镜像用 `debian:bookworm-slim`（最小化）。
* 用户 `doc-engine`（UID 1001，非 root）。
* `readOnlyRootFilesystem: true`。
* 禁用所有 capabilities。

---

## 8. 容量规划

### 8.1 单实例性能

| 操作 | 耗时 | CPU | 内存峰值 |
|------|------|-----|----------|
| 8 KB LaTeX 转换 | 800 ms | 50% | 50 MB |
| 50 MB LaTeX 转换 | ~5 s | 80% | 200 MB |

### 8.2 并发

* tokio multi-thread：默认 1 worker / CPU 核。
* 单 worker 可串行处理转换（CPU 密集）。
* 8 vCPU 实例可并行处理 ~8 个转换。

### 8.3 流量估算

假设每用户每 10 分钟 1 次转换，平均 5 秒：

| 用户数 | 每小时请求 | QPS | 推荐实例 |
|--------|------------|-----|----------|
| 100 | 600 | 0.17 | 1 × 1 vCPU |
| 1000 | 6000 | 1.7 | 2 × 2 vCPU |
| 10000 | 60000 | 17 | 5 × 4 vCPU + HPA |

---

## 9. GitHub Actions 自动化 CD 部署

在 GitHub 配置生产环境的 Secrets 之后，每次 push 或 merge 到 `main` 分支时，会触发自动部署工作流 `.github/workflows/deploy-production.yml`。

### 9.1 GitHub Secrets 配置

在 GitHub 仓库 `Settings -> Secrets and variables -> Actions` 中配置以下 secrets：

| Secret | 示例 | 说明 |
| --- | --- | --- |
| `PROD_SSH_HOST` | `82.156.234.59` | 生产服务器 IP |
| `PROD_SSH_USER` | `ubuntu` | SSH 登录用户 |
| `PROD_SSH_KEY` | 私钥全文 | 与服务器 `authorized_keys` 匹配的部署私钥 |
| `PROD_SSH_PORT` | `22` | 可选，默认 22 |
| `PROD_DEPLOY_DIR` | `/opt/tex2doc` | 部署目标根目录 |

### 9.2 服务器免密 sudo 权限

为了让 GitHub Actions 能够无交互地重启服务和重载 Nginx，需在 `/etc/sudoers.d/tex2doc-deploy` 配置文件中添加免密 sudo 规则：
```sudoers
ubuntu ALL=(root) NOPASSWD: /bin/systemctl restart tex2doc-server, /bin/systemctl reload nginx, /usr/sbin/nginx -t
```

---

## 10. 进一步阅读

* [01-rust-build.md](./01-rust-build.md) — Rust 构建
* [06-ci-and-hooks.md](./06-ci-and-hooks.md) — CI / 钩子
* [06-user-guide/05-http-server.md](../06-user-guide/05-http-server.md) — 使用方式
