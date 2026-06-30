# Tex2Doc 生产自动化部署与使用手册

## 1. 当前结论

截至 2026-06-25，Tex2Doc 已完成 GitHub Actions 到腾讯云生产服务器的自动化部署闭环。

当前生产入口：

| 项目 | 地址 |
| --- | --- |
| Home Web | `http://82.156.234.59/` |
| User Web | `http://82.156.234.59/user/` |
| Admin Web | `http://82.156.234.59/admin/` |
| API 健康检查 | `http://82.156.234.59/api/v1/health` |

当前生产服务：

| 服务 | 状态 |
| --- | --- |
| `nginx` | active |
| `postgresql` | active |
| `tex2doc-server` | active |
| 服务端监听 | `127.0.0.1:2624` |
| 当前 release | `/opt/tex2doc/releases/20260625-094640` |

已验证：

```bash
curl -I http://82.156.234.59/
curl -fsS http://82.156.234.59/api/v1/health
```

期望健康检查响应：

```json
{"status":"ok"}
```

## 2. 发布触发方式

生产部署 workflow：

```text
.github/workflows/deploy-production.yml
```

触发方式：

| 触发方式 | 说明 |
| --- | --- |
| push 到 `main` | 自动构建并部署到生产服务器 |
| GitHub Actions 手动 Run workflow | 用于手动重新发布或验证部署链路 |

手动触发命令：

```bash
gh workflow run deploy-production.yml --repo shlwsh/Tex2Doc --ref main
```

查看部署 run：

```bash
gh run list --repo shlwsh/Tex2Doc --workflow "Deploy Production" --limit 5
gh run view <RUN_ID> --repo shlwsh/Tex2Doc
```

最近一次成功自动部署：

```text
Run ID: 28140703778
Build production bundle: success
Deploy to Tencent Cloud: success
```

## 3. GitHub CI 当前策略

为避免 GitHub runner 队列和集成测试耗时阻塞生产发布，GitHub CI 当前只保留快速门禁。

当前 GitHub CI 执行：

- Rust `cargo fmt --all -- --check`
- Rust `cargo clippy --workspace --all-targets -- -D warnings`
- Flutter `flutter pub get`
- Flutter `flutter analyze`

当前 GitHub CI 暂时不执行：

- Rust `cargo test`
- `doc-server` API 数据库集成测试
- Flutter `flutter test`
- macOS runner 检查
- macOS intel / macOS arm 打包

完整测试集中在开发过程执行：

```bash
npm run ci:preflight
```

如需运行数据库 API 集成测试，先准备可用数据库并设置：

```bash
export DATABASE_URL=postgres://USER:PASSWORD@127.0.0.1:5432/docdb
npm run ci:preflight
```

Windows PowerShell 示例：

```powershell
$env:DATABASE_URL="postgres://USER:PASSWORD@127.0.0.1:5432/docdb"
npm run ci:preflight
```

## 4. 当前发布范围

当前优先保证：

- Linux 生产服务端部署。
- Windows 与 Linux 的 CI 快速检查。
- Windows 与 Linux 的原生包构建。
- Flutter Web 三入口部署。

暂时移除：

- macOS CI 检查。
- macOS intel 打包。
- macOS arm 打包。

恢复 macOS 时，需要重新加入：

```yaml
macos-13
macos-14
```

恢复前建议先观察 GitHub macOS runner 是否仍长时间 queued。

## 5. 服务器信息

SSH 配置：

```sshconfig
Host my-server
    HostName 82.156.234.59
    User ubuntu
    IdentityFile ~/.ssh/orcaterm_key
    ServerAliveInterval 60
```

生产部署实际使用 GitHub Actions 专用 SSH key，保存在 GitHub Secret `PROD_SSH_KEY` 中。不要将私钥、服务器密码、数据库密码提交到仓库。

服务器系统：

```text
Ubuntu 22.04
glibc 2.35
```

因此生产构建固定使用：

```yaml
runs-on: ubuntu-22.04
```

原因：如果用 `ubuntu-latest` 构建，GitHub runner 可能生成依赖 `GLIBC_2.39` 的二进制，部署到 Ubuntu 22.04 后会启动失败。

## 6. 端口与路由

项目服务端口统一落在 `2624-2634` 范围内；数据库端口除外。

| 服务 | 监听地址 | 端口 | 说明 |
| --- | --- | --- | --- |
| Rust API `doc-server` | `127.0.0.1` | `2624` | 只允许本机访问 |
| Nginx HTTP | `0.0.0.0` | `80` | 对外入口 |
| Nginx HTTPS | `0.0.0.0` | `443` | 后续配置证书 |
| PostgreSQL | `127.0.0.1` | `5432` | 不对公网开放 |

Nginx 路由：

| URL | 目标 |
| --- | --- |
| `/` | `/opt/tex2doc/current/static/home` |
| `/user/` | `/opt/tex2doc/current/static/user` |
| `/admin/` | `/opt/tex2doc/current/static/admin` |
| `/api/` | `http://127.0.0.1:2624` |
| `/v1/` | `http://127.0.0.1:2624` |
| `/admin/v1/` | `http://127.0.0.1:2624` |

## 7. 服务器目录结构

```text
/opt/tex2doc/
  current -> /opt/tex2doc/releases/<release-id>
  releases/
    <release-id>/
      server/doc-server
      static/home/
      static/user/
      static/admin/
  shared/
    env/doc-server.env
    sessions/
    logs/
```

重要文件：

| 路径 | 用途 |
| --- | --- |
| `/opt/tex2doc/current` | 当前线上版本软链接 |
| `/opt/tex2doc/releases` | 历史发布版本 |
| `/opt/tex2doc/shared/env/doc-server.env` | 服务端环境变量 |
| `/etc/systemd/system/tex2doc-server.service` | systemd 服务 |
| `/etc/nginx/sites-available/tex2doc` | nginx 站点配置 |
| `/etc/sudoers.d/tex2doc-deploy` | GitHub 部署用户免密执行限定命令 |

## 8. GitHub Secrets

仓库 Secrets：

| Secret | 用途 |
| --- | --- |
| `PROD_SSH_HOST` | 生产服务器 IP |
| `PROD_SSH_USER` | SSH 用户，当前为 `ubuntu` |
| `PROD_SSH_PORT` | SSH 端口，当前为 `22` |
| `PROD_DEPLOY_DIR` | 部署根目录，当前为 `/opt/tex2doc` |
| `PROD_SSH_KEY` | GitHub Actions 部署私钥 |

查看 Secrets 是否存在：

```bash
gh secret list --repo shlwsh/Tex2Doc
```

更新私钥 Secret 时建议使用原始文件流，避免 PowerShell 管道破坏 OpenSSH 私钥格式：

```bash
cat ~/.ssh/tex2doc_prod_deploy | gh secret set PROD_SSH_KEY --repo shlwsh/Tex2Doc
```

在 Git Bash for Windows 中：

```bash
cat /c/Users/Administrator/.ssh/tex2doc_prod_deploy \
  | "/c/Program Files/GitHub CLI/gh.exe" secret set PROD_SSH_KEY --repo shlwsh/Tex2Doc
```

## 9. 自动部署流程

自动部署分两个 job。

### 9.1 Build production bundle

执行环境：

```yaml
runs-on: ubuntu-22.04
```

步骤：

1. Checkout 代码。
1. 安装 Linux 原生依赖。
1. 安装 Rust stable。
1. 缓存 cargo。
1. 构建 Rust 服务端：

```bash
cargo build -p doc-server --release
```

1. 安装 Flutter stable。
1. 构建 Home Web：

```bash
flutter build web --release --target lib/main.dart --base-href /
```

1. 构建 User Web：

```bash
flutter build web --release --target lib/main_user.dart --base-href /user/
```

1. 构建 Admin Web：

```bash
flutter build web --release --target lib/main_admin.dart --base-href /admin/
```

1. 归档产物：

```text
tex2doc-production.tar.gz
```

### 9.2 Deploy to Tencent Cloud

步骤：

1. 下载 `tex2doc-production.tar.gz`。
1. 写入临时 SSH key。
1. 配置 SSH。
1. 上传产物。
1. 解压到新 release 目录。
1. 更新 `/opt/tex2doc/current` 软链接。
1. 重启 `tex2doc-server`。
1. 校验 nginx 配置并 reload。
1. 调用本机健康检查：

```bash
curl -fsS http://127.0.0.1:2624/api/v1/health
```

1. 清理旧 release，仅保留最近 5 个版本。

## 10. SSH 上传实现说明

部署中遇到过 `scp` 长时间卡住、服务器留下半截压缩包的问题。因此当前 workflow 改为 SSH 流式上传到临时文件：

```bash
ssh tex2doc-production \
  'cat > /tmp/tex2doc-production.tar.gz.tmp && mv /tmp/tex2doc-production.tar.gz.tmp /tmp/tex2doc-production.tar.gz' \
  < tex2doc-production.tar.gz
```

好处：

- 上传未完成时只存在 `.tmp` 文件。
- 上传完整后才原子替换成正式包。
- 避免半截包被激活脚本误用。

SSH 配置包含：

```sshconfig
KexAlgorithms curve25519-sha256
ServerAliveInterval 30
ServerAliveCountMax 6
```

原因：

- 部分客户端默认 KEX 会导致服务器在密钥交换阶段 reset。
- 上传产物约 50MB，GitHub 到腾讯云链路可能较慢，需要 keepalive。

## 11. 服务器初始化手册

以下命令仅在新服务器或重装系统后执行。

安装依赖：

```bash
sudo apt-get update
sudo apt-get install -y nginx postgresql postgresql-contrib postgresql-client tar unzip ca-certificates openssl \
  texlive-xetex texlive-luatex texlive-latex-recommended texlive-latex-extra \
  texlive-lang-chinese texlive-bibtex-extra latexmk fontconfig fonts-noto-cjk fonts-noto-cjk-extra

sudo fc-cache -fv
```

创建目录：

```bash
sudo mkdir -p /opt/tex2doc/releases /opt/tex2doc/shared/env /opt/tex2doc/shared/sessions /opt/tex2doc/shared/logs
sudo chown -R ubuntu:ubuntu /opt/tex2doc
```

创建数据库用户和数据库，密码应自行生成并保存到环境文件：

```bash
sudo systemctl enable --now postgresql
sudo -u postgres psql -c "CREATE USER tex2doc_app WITH PASSWORD 'REPLACE_ME';"
sudo -u postgres createdb -O tex2doc_app docdb
sudo -u postgres psql -c "ALTER DATABASE docdb OWNER TO tex2doc_app;"
```

创建 `/opt/tex2doc/shared/env/doc-server.env`：

```env
DOC_SERVER_ADDR=127.0.0.1:2624
DATABASE_URL=postgres://tex2doc_app:REPLACE_ME@127.0.0.1:5432/docdb
RUST_LOG=info
TEX2DOC_STATIC_DIR=/opt/tex2doc/current/static
TEX2DOC_BOOTSTRAP_ADMIN_EMAIL=admin@example.com
TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD=REPLACE_ME
```

设置权限：

```bash
chmod 600 /opt/tex2doc/shared/env/doc-server.env
```

创建 systemd 服务 `/etc/systemd/system/tex2doc-server.service`：

```ini
[Unit]
Description=Tex2Doc Rust API server
After=network-online.target postgresql.service
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

配置部署用户 sudoers `/etc/sudoers.d/tex2doc-deploy`：

```sudoers
ubuntu ALL=(root) NOPASSWD: /bin/systemctl restart tex2doc-server, /bin/systemctl reload nginx, /usr/sbin/nginx -t
```

校验：

```bash
sudo visudo -cf /etc/sudoers.d/tex2doc-deploy
```

## 12. Nginx 配置

站点文件：

```text
/etc/nginx/sites-available/tex2doc
```

核心配置：

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

启用：

```bash
sudo ln -sf /etc/nginx/sites-available/tex2doc /etc/nginx/sites-enabled/tex2doc
sudo rm -f /etc/nginx/sites-enabled/default
sudo nginx -t
sudo systemctl reload nginx
```

## 13. 日常发布操作

### 13.1 自动发布

合并或 push 到 `main`：

```bash
git push origin main
```

GitHub 自动执行：

```text
Deploy Production
```

### 13.2 手动重新发布

```bash
gh workflow run deploy-production.yml --repo shlwsh/Tex2Doc --ref main
```

查看：

```bash
gh run list --repo shlwsh/Tex2Doc --workflow "Deploy Production" --limit 5
```

### 13.3 查看生产服务状态

```bash
ssh -i ~/.ssh/tex2doc_prod_deploy ubuntu@82.156.234.59 \
  'systemctl status tex2doc-server --no-pager'
```

### 13.4 查看服务日志

```bash
ssh -i ~/.ssh/tex2doc_prod_deploy ubuntu@82.156.234.59 \
  'journalctl -u tex2doc-server -n 120 --no-pager'
```

### 13.5 查看当前版本

```bash
ssh -i ~/.ssh/tex2doc_prod_deploy ubuntu@82.156.234.59 \
  'readlink -f /opt/tex2doc/current'
```

## 14. 发布验证清单

GitHub Actions 侧：

```bash
gh run view <RUN_ID> --repo shlwsh/Tex2Doc
```

服务器本机：

```bash
curl -fsS http://127.0.0.1:2624/api/v1/health
curl -I http://127.0.0.1/
curl -I http://127.0.0.1/user/
curl -I http://127.0.0.1/admin/
```

外网：

```bash
curl -I http://82.156.234.59/
curl -fsS http://82.156.234.59/api/v1/health
```

预期：

- Home Web 返回 `HTTP/1.1 200 OK`。
- API health 返回 `{"status":"ok"}`。

## 15. 回滚手册

列出历史版本：

```bash
ssh -i ~/.ssh/tex2doc_prod_deploy ubuntu@82.156.234.59 \
  'ls -1dt /opt/tex2doc/releases/*'
```

回滚：

```bash
ssh -i ~/.ssh/tex2doc_prod_deploy ubuntu@82.156.234.59 '
set -euo pipefail
PREV_RELEASE=/opt/tex2doc/releases/REPLACE_ME
ln -sfn "$PREV_RELEASE" /opt/tex2doc/current
sudo systemctl restart tex2doc-server
sudo nginx -t
sudo systemctl reload nginx
curl -fsS http://127.0.0.1:2624/api/v1/health
'
```

## 16. 故障排查

### 16.1 `GLIBC_2.39 not found`

现象：

```text
/opt/tex2doc/current/server/doc-server: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.39' not found
```

原因：

- GitHub `ubuntu-latest` runner 的 glibc 高于生产服务器。
- 产物在新系统构建，部署到 Ubuntu 22.04 后无法运行。

处理：

- 生产部署和 Linux release 包固定 `ubuntu-22.04`。

### 16.2 `Load key ... error in libcrypto`

现象：

```text
Load key "/home/runner/.ssh/prod_deploy_key": error in libcrypto
Permission denied (publickey,password)
```

原因：

- GitHub Secret 中的 OpenSSH 私钥格式被破坏，多见于 Windows PowerShell 管道处理多行私钥。

处理：

```bash
cat ~/.ssh/tex2doc_prod_deploy | gh secret set PROD_SSH_KEY --repo shlwsh/Tex2Doc
```

使用 Git Bash 时：

```bash
cat /c/Users/Administrator/.ssh/tex2doc_prod_deploy \
  | "/c/Program Files/GitHub CLI/gh.exe" secret set PROD_SSH_KEY --repo shlwsh/Tex2Doc
```

### 16.3 SSH 连接 reset

现象：

```text
Connection reset by 82.156.234.59 port 22
```

原因：

- 某些 OpenSSH 客户端默认 KEX 和服务器链路不兼容。

处理：

```sshconfig
KexAlgorithms curve25519-sha256
```

### 16.4 上传包半截导致 `tar Unexpected EOF`

现象：

```text
gzip: stdin: invalid compressed data--format violated
tar: Unexpected EOF in archive
```

原因：

- scp 或网络中断留下半截 `/tmp/tex2doc-production.tar.gz`。

处理：

- 当前 workflow 已改为先上传 `.tmp`，成功后再 `mv`。
- 如果服务器上存在半截文件，删除后重新部署：

```bash
ssh -i ~/.ssh/tex2doc_prod_deploy ubuntu@82.156.234.59 \
  'rm -f /tmp/tex2doc-production.tar.gz /tmp/tex2doc-production.tar.gz.tmp'
```

### 16.5 健康检查 connection refused

现象：

```text
curl: (7) Failed to connect to 127.0.0.1 port 2624
```

排查：

```bash
systemctl status tex2doc-server --no-pager
journalctl -u tex2doc-server -n 120 --no-pager
readlink -f /opt/tex2doc/current
ls -lh /opt/tex2doc/current/server/doc-server
```

常见原因：

- 二进制 glibc 不兼容。
- `DATABASE_URL` 不可用。
- `/opt/tex2doc/current` 指向无效 release。
- `doc-server` 没有执行权限。

### 16.6 nginx 配置失败

检查：

```bash
sudo nginx -t
```

查看日志：

```bash
sudo tail -n 120 /var/log/nginx/error.log
```

## 17. 安全要求

- 不要把服务器密码、数据库密码、私钥写入仓库。
- `doc-server.env` 权限保持 `600`。
- PostgreSQL 不开放公网访问。
- GitHub `production` environment 后续建议加 required reviewers。
- 部署 key 建议只用于当前服务器部署，不复用个人常用私钥。

## 18. 后续建议

- 接入正式域名。
- 配置 HTTPS。
- 将 GitHub 上传产物链路改为对象存储中转，以提升大文件上传速度。
- 恢复 macOS 矩阵前先单独建非阻塞 workflow 观察 runner 排队情况。
- 为生产数据库建立定时备份和恢复演练。
