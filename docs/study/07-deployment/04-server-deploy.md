# 服务端部署

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

* 仅需 C 标准库 + 操作系统 glibc。
* 无外部 runtime 依赖（无 Node.js / Python / JVM）。

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

```ini
# /etc/systemd/system/doc-server.service
[Unit]
Description=Doc-engine HTTP server
Documentation=https://example.com/doc-engine
After=network.target

[Service]
Type=simple
User=doc-engine
Group=doc-engine
Environment=DOC_SERVER_ADDR=0.0.0.0:2624
Environment=RUST_LOG=info,doc_server=info
ExecStart=/opt/doc-engine/doc-server
Restart=on-failure
RestartSec=5
LimitNOFILE=65536

# 安全加固
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/doc-engine

[Install]
WantedBy=multi-user.target
```

```bash
# 1. 创建用户
sudo useradd -r -s /bin/false doc-engine

# 2. 准备日志目录
sudo mkdir -p /var/log/doc-engine
sudo chown doc-engine:doc-engine /var/log/doc-engine

# 3. 启用服务
sudo systemctl daemon-reload
sudo systemctl enable --now doc-server
sudo systemctl status doc-server

# 4. 查看日志
sudo journalctl -u doc-server -f
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

```nginx
upstream doc_server {
    server 127.0.0.1:2624;
    keepalive 32;
}

server {
    listen 80;
    server_name doc-engine.example.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name doc-engine.example.com;

    ssl_certificate /etc/letsencrypt/live/doc-engine.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/doc-engine.example.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;

    client_max_body_size 60M;
    client_body_timeout 120s;

    access_log /var/log/nginx/doc-server-access.log;
    error_log /var/log/nginx/doc-server-error.log;

    location / {
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
        proxy_request_buffering off;  # 大文件流式
    }

    location /api/v1/health {
        proxy_pass http://doc_server;
        access_log off;
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

* **配置**：环境变量 / `.env` 文件。
* **TLS 证书**：`/etc/letsencrypt/`。
* **日志**：`/var/log/doc-server/`。

> 服务**无状态**——无需备份数据库。

### 6.2 灾难恢复

```bash
# 在新服务器上
1. 复制 doc-server binary
2. 配置 systemd / Docker
3. 复制环境变量
4. 启动
5. 配置 Nginx / 反代
6. 申请证书
```

RTO（恢复时间目标）：< 30 分钟。
RPO（恢复点目标）：N/A（无状态）。

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
# doc-engine 用户（无 shell、无 home）
sudo useradd -r -s /usr/sbin/nologin -M doc-engine

# 限制 doc-server binary
sudo chown root:doc-engine /opt/doc-engine/doc-server
sudo chmod 750 /opt/doc-engine/doc-server
```

### 7.5 容器安全

* 基础镜像用 `debian:bookworm-slim`（最小化）。
* 用户 `doc-engine`（UID 1001，非 root）。
* `readOnlyRootFilesystem: true`。
* 禁用所有 capabilities。
* 定期 `docker scan` / `trivy`。

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

## 9. 进一步阅读

* [01-rust-build.md](./01-rust-build.md) — Rust 构建
* [06-ci-and-hooks.md](./06-ci-and-hooks.md) — CI / 钩子
* [06-user-guide/05-http-server.md](../06-user-guide/05-http-server.md) — 使用方式
