# GitHub 自动部署 Flutter Web 与 Rust 服务端实施方案

## 目标

将 Tex2Doc 的生产部署收敛为一条 GitHub Actions 自动流水线：

- GitHub 侧完成代码检查、Flutter Web 构建、Rust 服务端 release 构建。
- 构建产物通过 SSH/SCP 发布到腾讯云生产服务器。
- 服务器使用 `systemd` 托管 Rust 服务端，使用 `nginx` 暴露 Flutter Web 静态站点并反代 API。
- 数据库仍使用生产服务器现有 PostgreSQL，数据库端口不暴露到公网。

生产服务器连接信息：

```sshconfig
Host my-server
    HostName 82.156.234.59
    User ubuntu
    IdentityFile ~/.ssh/orcaterm_key
    ServerAliveInterval 60
```

> 不要把服务器密码、私钥、数据库密码提交到仓库。所有敏感值统一放入 GitHub Secrets 或服务器本地环境文件。

## 推荐部署形态

### 当前发布范围

当前生产发布优先保证：

- Linux 生产服务端部署。
- Windows 与 Linux 的 CI 检查和原生包构建。
- GitHub CI 暂时只执行格式与静态检查；Rust/Flutter 测试和数据库集成测试集中在本地 `npm run ci:preflight` 开发预检中执行。

macOS intel / macOS arm 打包暂时从必过发布要求中移除，避免 GitHub macOS runner 长时间排队阻塞 PR 合并与生产部署。后续 runner 稳定后，再恢复 `macos-13` 与 `macos-14` 矩阵。

生产服务器当前是 Ubuntu 22.04，因此生产部署和 Linux release 包固定使用 `ubuntu-22.04` runner 构建，避免在 `ubuntu-latest` 上生成依赖更高 glibc 版本的二进制。

### 端口规划

延续项目此前端口约束：除数据库端口外，项目服务端口统一使用 `2624-2634`。

| 服务 | 监听地址 | 端口 | 说明 |
| --- | --- | --- | --- |
| Rust API `doc-server` | `127.0.0.1` | `2624` | 只允许本机访问，由 nginx 反代 |
| Nginx HTTP | `0.0.0.0` | `80` | 对外访问入口 |
| Nginx HTTPS | `0.0.0.0` | `443` | 配置证书后启用 |
| PostgreSQL | `127.0.0.1` 或内网 | `5432` | 数据库端口不纳入项目服务端口范围 |

### 目录规划

```text
/opt/tex2doc/
  releases/
    20260625-120000/
      server/doc-server
      static/home/
      static/user/
      static/admin/
  current -> /opt/tex2doc/releases/20260625-120000
  shared/
    env/doc-server.env
    sessions/
    logs/
```

建议使用 `current` 软链接指向当前版本，发布失败时可以快速回滚到上一个 release。

### 对外路径

| URL | 目标 |
| --- | --- |
| `/` | Flutter Home Web |
| `/user/` | Flutter User Web |
| `/admin/` | Flutter Admin Web |
| `/api/` | 反代到 `http://127.0.0.1:2624` |
| `/v1/` | 反代到 `http://127.0.0.1:2624` |
| `/admin/v1/` | 反代到 `http://127.0.0.1:2624` |

## 一次性服务器初始化

以下命令在生产服务器执行。

### 1. 安装依赖

```bash
sudo apt-get update
sudo apt-get install -y nginx postgresql-client tar unzip ca-certificates
```

如服务器本机也运行 PostgreSQL：

```bash
sudo apt-get install -y postgresql postgresql-contrib
```

### 2. 创建部署目录

```bash
sudo mkdir -p /opt/tex2doc/releases /opt/tex2doc/shared/env /opt/tex2doc/shared/sessions /opt/tex2doc/shared/logs
sudo chown -R ubuntu:ubuntu /opt/tex2doc
```

### 3. 配置服务端环境文件

创建 `/opt/tex2doc/shared/env/doc-server.env`：

```env
DOC_SERVER_ADDR=127.0.0.1:2624
DATABASE_URL=postgres://postgres:REPLACE_ME@127.0.0.1:5432/docdb
RUST_LOG=info
TEX2DOC_BOOTSTRAP_ADMIN_EMAIL=admin@example.com
TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD=REPLACE_ME
```

权限：

```bash
chmod 600 /opt/tex2doc/shared/env/doc-server.env
```

### 4. 配置 systemd

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
NoNewPrivileges=true
PrivateTmp=true
ReadWritePaths=/opt/tex2doc/shared

[Install]
WantedBy=multi-user.target
```

启用服务：

```bash
sudo systemctl daemon-reload
sudo systemctl enable tex2doc-server
```

### 5. 配置部署用户 sudo 权限

GitHub Actions 通过 SSH 执行部署时无法输入 sudo 密码，因此需要允许部署用户免密执行限定命令。

创建 `/etc/sudoers.d/tex2doc-deploy`：

```sudoers
ubuntu ALL=(root) NOPASSWD: /bin/systemctl restart tex2doc-server, /bin/systemctl reload nginx, /usr/sbin/nginx -t
```

检查 sudoers 语法：

```bash
sudo visudo -cf /etc/sudoers.d/tex2doc-deploy
```

如果 `systemctl` 或 `nginx` 的路径不同，使用以下命令确认：

```bash
command -v systemctl
command -v nginx
```

### 6. 配置 nginx

创建 `/etc/nginx/sites-available/tex2doc`：

```nginx
server {
    listen 80;
    server_name _;

    client_max_body_size 60m;

    root /opt/tex2doc/current/static/home;
    index index.html;

    location / {
        try_files $uri $uri/ /index.html;
    }

    location /user/ {
        alias /opt/tex2doc/current/static/user/;
        try_files $uri $uri/ /user/index.html;
    }

    location /admin/ {
        alias /opt/tex2doc/current/static/admin/;
        try_files $uri $uri/ /admin/index.html;
    }

    location /api/ {
        proxy_pass http://127.0.0.1:2624;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location /v1/ {
        proxy_pass http://127.0.0.1:2624;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location /admin/v1/ {
        proxy_pass http://127.0.0.1:2624;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

启用 nginx 站点：

```bash
sudo ln -sf /etc/nginx/sites-available/tex2doc /etc/nginx/sites-enabled/tex2doc
sudo nginx -t
sudo systemctl reload nginx
```

## GitHub Secrets

在 GitHub 仓库 `Settings -> Secrets and variables -> Actions` 中配置：

| Secret | 示例 | 说明 |
| --- | --- | --- |
| `PROD_SSH_HOST` | `82.156.234.59` | 生产服务器 IP |
| `PROD_SSH_USER` | `ubuntu` | SSH 用户 |
| `PROD_SSH_KEY` | 私钥全文 | 与服务器 `authorized_keys` 匹配的部署私钥 |
| `PROD_SSH_PORT` | `22` | 可选，默认 22 |
| `PROD_DEPLOY_DIR` | `/opt/tex2doc` | 部署根目录 |

推荐使用独立部署密钥，不要直接使用个人日常登录私钥。

## GitHub Actions 自动部署工作流

建议新增 `.github/workflows/deploy-production.yml`：

```yaml
name: Deploy Production

on:
  workflow_dispatch:
  push:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: -D warnings

jobs:
  build:
    name: Build production bundle
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
        with:
          fetch-depth: 1

      - name: Install Linux native dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libfontconfig1-dev \
            libxkbcommon-dev \
            libwayland-dev \
            libx11-xcb-dev \
            libxcb1-dev \
            libxcb-render0-dev \
            libxcb-shape0-dev \
            libxcb-xfixes0-dev \
            libegl1-mesa-dev \
            libgl1-mesa-dev \
            libudev-dev \
            libinput-dev

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: .

      - name: Build doc-server
        run: cargo build -p doc-server --release

      - name: Install Flutter
        uses: subosito/flutter-action@v2
        with:
          channel: stable
          cache: true

      - name: Build Flutter home
        working-directory: flutter_app
        run: flutter build web --release --target lib/main.dart --base-href /

      - name: Stage Flutter home
        run: |
          mkdir -p dist/static/home
          cp -R flutter_app/build/web/. dist/static/home/

      - name: Build Flutter user
        working-directory: flutter_app
        run: flutter build web --release --target lib/main_user.dart --base-href /user/

      - name: Stage Flutter user
        run: |
          mkdir -p dist/static/user
          cp -R flutter_app/build/web/. dist/static/user/

      - name: Build Flutter admin
        working-directory: flutter_app
        run: flutter build web --release --target lib/main_admin.dart --base-href /admin/

      - name: Stage bundle
        run: |
          mkdir -p dist/server dist/static/admin
          cp -R flutter_app/build/web/. dist/static/admin/
          cp target/release/doc-server dist/server/doc-server
          chmod +x dist/server/doc-server
          tar -czf tex2doc-production.tar.gz -C dist .

      - name: Upload production bundle
        uses: actions/upload-artifact@v4
        with:
          name: tex2doc-production
          path: tex2doc-production.tar.gz
          if-no-files-found: error

  deploy:
    name: Deploy to Tencent Cloud
    runs-on: ubuntu-latest
    needs: build
    environment: production
    steps:
      - name: Download production bundle
        uses: actions/download-artifact@v4
        with:
          name: tex2doc-production

      - name: Configure SSH
        run: |
          mkdir -p ~/.ssh
          printf '%s\n' "${{ secrets.PROD_SSH_KEY }}" > ~/.ssh/prod_deploy_key
          chmod 600 ~/.ssh/prod_deploy_key
          ssh-keyscan -p "${{ secrets.PROD_SSH_PORT || 22 }}" "${{ secrets.PROD_SSH_HOST }}" >> ~/.ssh/known_hosts
          cat > ~/.ssh/config <<EOF
          Host tex2doc-production
            HostName ${{ secrets.PROD_SSH_HOST }}
            User ${{ secrets.PROD_SSH_USER }}
            Port ${{ secrets.PROD_SSH_PORT || 22 }}
            IdentityFile ~/.ssh/prod_deploy_key
            KexAlgorithms curve25519-sha256
            StrictHostKeyChecking yes
          EOF

      - name: Upload bundle
        run: |
          scp tex2doc-production.tar.gz tex2doc-production:/tmp/tex2doc-production.tar.gz

      - name: Activate release
        run: |
          DEPLOY_DIR="${{ secrets.PROD_DEPLOY_DIR }}"
          if [ -z "$DEPLOY_DIR" ]; then
            DEPLOY_DIR="/opt/tex2doc"
          fi
          ssh tex2doc-production "DEPLOY_DIR='$DEPLOY_DIR' bash -s" <<'REMOTE'
          set -euo pipefail
          RELEASE_ID="$(date +%Y%m%d-%H%M%S)"
          RELEASE_DIR="$DEPLOY_DIR/releases/$RELEASE_ID"
          mkdir -p "$RELEASE_DIR"
          tar -xzf /tmp/tex2doc-production.tar.gz -C "$RELEASE_DIR"
          chmod +x "$RELEASE_DIR/server/doc-server"
          ln -sfn "$RELEASE_DIR" "$DEPLOY_DIR/current"
          sudo systemctl restart tex2doc-server
          sudo nginx -t
          sudo systemctl reload nginx
          curl -fsS http://127.0.0.1:2624/api/v1/health >/dev/null
          rm -f /tmp/tex2doc-production.tar.gz
          ls -1dt "$DEPLOY_DIR"/releases/* | tail -n +6 | xargs -r rm -rf
          REMOTE
```

## 发布验证

GitHub Actions 成功后，在服务器执行：

```bash
systemctl status tex2doc-server --no-pager
curl -fsS http://127.0.0.1:2624/api/v1/health
curl -I http://127.0.0.1/
curl -I http://127.0.0.1/user/
curl -I http://127.0.0.1/admin/
```

外网验证：

```bash
curl -I http://82.156.234.59/
curl -fsS http://82.156.234.59/api/v1/health
```

## 回滚

查看历史版本：

```bash
ls -1dt /opt/tex2doc/releases/*
```

回滚到上一个版本：

```bash
PREV_RELEASE=/opt/tex2doc/releases/REPLACE_ME
ln -sfn "$PREV_RELEASE" /opt/tex2doc/current
sudo systemctl restart tex2doc-server
sudo systemctl reload nginx
```

## 实施顺序

1. 在服务器完成目录、环境文件、systemd、nginx 初始化。
2. 在 GitHub 配置生产环境 Secrets。
3. 新增 `deploy-production.yml`，先使用 `workflow_dispatch` 手动发布一次。
4. 验证 `/`、`/user/`、`/admin/`、`/api/v1/health`。
5. 手动发布稳定后，再保留 `push main` 自动发布。
6. 后续接入域名和 HTTPS，用 certbot 或腾讯云证书完成 TLS 配置。

## 安全注意事项

- 不要在仓库、Actions 日志、文档中写入服务器密码或数据库密码。
- 生产数据库只允许本机或内网访问，不开放公网入站。
- GitHub `production` environment 建议开启 required reviewers，避免误推 `main` 直接上线。
- 部署 SSH key 建议只授权当前服务器当前用户，必要时限制 command 或单独创建 `deploy` 用户。
