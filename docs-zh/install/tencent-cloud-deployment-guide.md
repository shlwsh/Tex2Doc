# Tex2Doc 腾讯云生产服务器部署方案

> 创建时间：2026-06-29
> 服务器：腾讯云 Ubuntu 22.04
> 服务器 IP：82.156.234.59

## 1. 当前生产环境概览

### 1.1 服务架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                         用户请求 (HTTP/HTTPS)                         │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         Nginx (端口 80/443)                          │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────────────┐    │
│  │ /       │  │ /user/  │  │ /admin/ │  │ /api/ /v1/          │    │
│  │ Home    │  │ User    │  │ Admin   │  │ doc-server          │    │
│  └─────────┘  └─────────┘  └─────────┘  └─────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Rust API Server (127.0.0.1:2624)                   │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────┐ │
│  │ 用户认证    │  │ 转换任务    │  │ 文件存储    │  │ PostgreSQL │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.2 当前生产入口

| 应用 | URL | 说明 |
|------|-----|------|
| Home Web | http://82.156.234.59/ | 产品首页 |
| User Web | http://82.156.234.59/user/ | 用户转换界面 |
| Admin Web | http://82.156.234.59/admin/ | 管理后台 |
| API 健康检查 | http://82.156.234.59/api/v1/health | 返回 `{"status":"ok"}` |

### 1.3 服务器当前状态

| 服务 | 状态 | 说明 |
|------|------|------|
| nginx | active | 反向代理 |
| postgresql | active | 数据库 |
| tex2doc-server | active | Rust API 服务 |
| 服务端端口 | 127.0.0.1:2624 | 仅本机访问 |
| 当前版本 | /opt/tex2doc/releases/20260625-094640 | 最新发布日期 |

## 2. 项目产出组件

### 2.1 Rust 服务端 (doc-server)

- **位置**: `apps/rust-service/`
- **二进制**: `doc-server`
- **功能**: LaTeX → DOCX 转换 API 服务
- **依赖**: PostgreSQL 数据库
- **构建命令**: `cargo build -p doc-server --release`

### 2.2 Flutter Web 应用

| 应用 | 入口文件 | 部署路径 | base-href |
|------|----------|----------|-----------|
| Home | `lib/main.dart` | /opt/tex2doc/current/static/home | / |
| User | `lib/main_user.dart` | /opt/tex2doc/current/static/user | /user/ |
| Admin | `lib/main_admin.dart` | /opt/tex2doc/current/static/admin | /admin/ |

### 2.3 浏览器插件 (Browser Extension)

- **位置**: `apps/browser-extension/`
- **框架**: WXT (基于 Vite)
- **支持浏览器**: Chrome, Edge, Firefox, Safari
- **构建工具**: Node.js 18+
- **WASM 引擎**: `crates/wasm/` (Rust → WebAssembly)

### 2.4 部署包结构

```
tex2doc-production.tar.gz
├── server/
│   └── doc-server          # Rust API 二进制
├── static/
│   ├── home/              # Home Web 静态文件
│   ├── user/              # User Web 静态文件
│   └── admin/             # Admin Web 静态文件
```

浏览器插件 ZIP 包（独立发布）:

```
tex2doc-browser-extension-chrome.zip
tex2doc-browser-extension-firefox.zip
tex2doc-browser-extension-safari.zip
```

## 3. 服务器目录结构

```
/opt/tex2doc/
├── current -> /opt/tex2doc/releases/<release-id>    # 当前版本软链接
├── releases/
│   └── <release-id>/
│       ├── server/
│       │   └── doc-server                              # Rust 二进制
│       └── static/
│           ├── home/                                   # Home Web
│           ├── user/                                  # User Web
│           └── admin/                                 # Admin Web
└── shared/
    ├── env/
    │   └── doc-server.env                             # 环境变量
    ├── sessions/                                      # 会话数据
    └── logs/                                          # 日志目录
```

## 4. 发布触发方式

### 4.1 自动发布（推荐）

推送到 `main` 分支自动触发：

```bash
git push origin main
```

GitHub Actions 自动执行：

1. **Build production bundle**: 在 `ubuntu-22.04` 构建所有组件
2. **Deploy to Tencent Cloud**: 通过 SSH 部署到生产服务器

### 4.2 手动发布

通过 GitHub CLI 手动触发：

```bash
# 触发部署
gh workflow run deploy-production.yml --repo shlwsh/Tex2Doc --ref main

# 查看最近部署
gh run list --repo shlwsh/Tex2Doc --workflow "Deploy Production" --limit 5

# 查看具体部署详情
gh run view <RUN_ID> --repo shlwsh/Tex2Doc
```

### 4.3 本地构建后手动部署

```powershell
# 1. 构建所有组件
.\scripts\release\build-rust-service.ps1
.\scripts\release\build-flutter-home.ps1
.\scripts\release\build-flutter-user.ps1
.\scripts\release\build-flutter-admin.ps1

# 2. 打包
tar -czf tex2doc-production.tar.gz -C apps/rust-service static

# 3. 上传到服务器
scp -i ~/.ssh/orcaterm_key tex2doc-production.tar.gz ubuntu@82.156.234.59:/tmp/

# 4. SSH 到服务器执行部署
ssh -i ~/.ssh/orcaterm_key ubuntu@82.156.234.59
```

## 5. GitHub Actions 部署流程详解

### 5.1 构建阶段 (Build production bundle)

```yaml
runs-on: ubuntu-22.04  # 固定版本，避免 glibc 不兼容
```

执行步骤：

1. Checkout 代码
2. 安装 Linux 原生依赖
3. 安装 Rust stable
4. 构建 `doc-server`: `cargo build -p doc-server --release`
5. 安装 Flutter stable
6. 构建 Home Web: `flutter build web --release --target lib/main.dart --base-href /`
7. 构建 User Web: `flutter build web --release --target lib/main_user.dart --base-href /user/`
8. 构建 Admin Web: `flutter build web --release --target lib/main_admin.dart --base-href /admin/`
9. 打包: `tar -czf tex2doc-production.tar.gz`
10. 上传 artifact

### 5.2 部署阶段 (Deploy to Tencent Cloud)

执行步骤：

1. 下载 artifact
2. 配置 SSH 连接（使用 GitHub Secrets 中的私钥）
3. 流式上传压缩包到服务器 `/tmp/`
4. 创建新 release 目录
5. 解压到 `/opt/tex2doc/releases/<release-id>/`
6. 更新软链接 `current` → 新 release
7. 重启服务: `sudo systemctl restart tex2doc-server`
8. 重载 nginx: `sudo systemctl reload nginx`
9. 健康检查
10. 清理旧 release（保留最近 5 个）

### 5.3 GitHub Secrets 配置

| Secret | 用途 | 当前值 |
|--------|------|--------|
| `PROD_SSH_HOST` | 服务器 IP | 82.156.234.59 |
| `PROD_SSH_USER` | SSH 用户 | ubuntu |
| `PROD_SSH_PORT` | SSH 端口 | 22 |
| `PROD_SSH_KEY` | SSH 部署私钥 | (已配置) |
| `PROD_DEPLOY_DIR` | 部署目录 | /opt/tex2doc |

更新 SSH 私钥（Windows PowerShell）：

```bash
cat ~/.ssh/tex2doc_prod_deploy | gh secret set PROD_SSH_KEY --repo shlwsh/Tex2Doc
```

## 6. 日常运维命令

### 6.1 查看服务状态

```bash
ssh -i ~/.ssh/orcaterm_key ubuntu@82.156.234.59 \
  'systemctl status tex2doc-server --no-pager'
```

### 6.2 查看服务日志

```bash
ssh -i ~/.ssh/orcaterm_key ubuntu@82.156.234.59 \
  'journalctl -u tex2doc-server -n 120 --no-pager'
```

### 6.3 查看当前版本

```bash
ssh -i ~/.ssh/orcaterm_key ubuntu@82.156.234.59 \
  'readlink -f /opt/tex2doc/current'
```

### 6.4 查看历史版本

```bash
ssh -i ~/.ssh/orcaterm_key ubuntu@82.156.234.59 \
  'ls -1dt /opt/tex2doc/releases/*'
```

### 6.5 健康检查

```bash
# 服务器本机
curl -fsS http://127.0.0.1:2624/api/v1/health

# 外网
curl -fsS http://82.156.234.59/api/v1/health
```

预期响应：`{"status":"ok"}`

## 7. 回滚操作

### 7.1 列出可用版本

```bash
ssh -i ~/.ssh/orcaterm_key ubuntu@82.156.234.59 \
  'ls -1dt /opt/tex2doc/releases/*'
```

### 7.2 执行回滚

```bash
ssh -i ~/.ssh/orcaterm_key ubuntu@82.156.234.59 '
set -euo pipefail
RELEASE_ID=20260624-120000  # 替换为要回滚的版本
ln -sfn "/opt/tex2doc/releases/$RELEASE_ID" /opt/tex2doc/current
sudo systemctl restart tex2doc-server
sudo nginx -t
sudo systemctl reload nginx
curl -fsS http://127.0.0.1:2624/api/v1/health
'
```

## 8. 浏览器插件发布与部署

### 8.1 插件概述

浏览器插件是 Tex2Doc 的独立产出，支持以下浏览器：

| 浏览器 | 清单版本 | 产出文件 | 发布平台 |
|--------|----------|----------|----------|
| Chrome | MV3 | `tex2doc-browser-extension-chrome.zip` | Chrome Web Store |
| Edge | MV3 | 同 Chrome | Microsoft Edge Add-ons |
| Firefox | MV2 | `tex2doc-browser-extension-firefox.zip` | Firefox Add-ons |
| Safari | MV2 | `tex2doc-browser-extension-safari.zip` | Safari App Store |

### 8.2 本地构建

```bash
# 进入插件目录
cd apps/browser-extension

# 安装依赖
npm install

# 构建所有浏览器版本
npm run build:all

# 或单独构建
npm run build:chrome    # Chrome/Edge
npm run build:firefox   # Firefox
npm run build:safari    # Safari

# 打包 ZIP
npm run zip:all
```

构建产物位于 `.output/` 目录：

```
.output/
├── chrome-mv3/           # Chrome 构建产物
├── chrome-mv3-edge/     # Edge 构建产物
├── firefox-mv2/        # Firefox 构建产物
└── safari-mv2/         # Safari 构建产物
```

### 8.3 GitHub Actions 自动构建

推送代码到 `main` 分支时，GitHub Actions 会自动执行：

1. **Build**: 并行构建 Chrome 和 Firefox 版本
2. **Type Check**: TypeScript 类型检查
3. **Lint**: ESLint 代码检查
4. **Test**: 单元测试
5. **E2E**: Playwright E2E 测试
6. **WASM Size Check**: WASM 文件大小检查
7. **Manifest Check**: manifest.json 权限验证

查看 GitHub Actions 运行状态：

```bash
# 查看最近运行
gh run list --repo shlwsh/Tex2Doc --workflow "Browser Extension CI" --limit 10

# 查看具体运行详情
gh run view <RUN_ID> --repo shlwsh/Tex2Doc
```

### 8.4 手动触发构建

```bash
# 通过 GitHub CLI 触发
gh workflow run browser-extension.yml --repo shlwsh/Tex2Doc --ref main

# 下载构建产物
gh run download <RUN_ID> --repo shlwsh/Tex2Doc --name chrome-extension
```

### 8.5 手动加载扩展（开发者模式）

#### Chrome / Edge

1. 打开 `chrome://extensions/` (Edge: `edge://extensions/`)
2. 开启右上角的 **开发者模式**
3. 点击 **加载已解压的扩展程序**
4. 选择对应目录（如 `.output/chrome-mv3/`）

#### Firefox

1. 打开 `about:debugging#/runtime/this-firefox`
2. 点击 **临时加载扩展程序**
3. 选择 `.output/firefox-mv2/manifest.json`

### 8.6 各平台发布指南

#### Chrome Web Store

1. 访问 [Chrome Web Store Developer Dashboard](https://chrome.google.com/webstore/devconsole)
2. 创建开发者账户（如需要，收取一次性注册费）
3. 上传 `.zip` 文件
4. 填写应用信息（名称、描述、截图）
5. 设置分发范围（公开/受限）
6. 提交审核（通常 1-3 个工作日）

#### Microsoft Edge Add-ons

1. 访问 [Microsoft Partner Center](https://partner.microsoft.com/dashboard/microsoft_edge/)
2. 注册为开发者
3. 创建新扩展
4. 上传 `.zip` 文件
5. 填写扩展信息
6. 提交审核

#### Firefox Browser Extensions

1. 访问 [Firefox Add-ons](https://addons.mozilla.org/developers/)
2. 创建账户
3. 创建新扩展
4. 上传 `.zip` 文件或提交源代码
5. 填写扩展信息
6. 提交审核（通常自动审核，几小时内完成）

#### Safari App Store

1. 加入 Apple Developer Program（年费 $99）
2. 在 Xcode 中打开 `apps/browser-extension`
3. 配置签名证书和 App ID
4. 构建 Archive
5. 通过 Xcode 提交到 App Store Connect
6. 在 App Store Connect 完善应用信息
7. 提交审核

### 8.7 发布检查清单

发布前确认以下事项：

- [ ] 所有构建命令执行成功
- [ ] 各浏览器扩展加载测试通过
- [ ] 单元测试全部通过
- [ ] E2E 测试全部通过
- [ ] WASM 大小检查通过（< 10MB）
- [ ] manifest.json 权限配置正确
- [ ] 版本号已更新（`package.json` → `version`）
- [ ] 更新日志已记录
- [ ] README 和文档已更新

### 8.8 版本号管理

版本号定义位置：`apps/browser-extension/package.json` → `version` 字段

遵循语义化版本 `major.minor.patch`：

- `major`: 主版本变更（不兼容的 API 修改）
- `minor`: 次版本变更（向后兼容的功能新增）
- `patch`: 修订版本（向后兼容的问题修复）

### 8.9 插件配置说明

#### API 端点配置

生产环境插件连接 `https://api.tex2doc.cn`，在 `wxt.config.ts` 中配置：

```typescript
manifest: ({ browser, mode }) => ({
  host_permissions: ['https://api.tex2doc.cn/*'],
  // ...
}),
```

#### 权限说明

| 权限 | 用途 |
|------|------|
| `storage` | 存储用户设置和缓存 |
| `downloads` | 下载转换后的 DOCX 文件 |
| `contextMenus` | 右键菜单 |
| `notifications` | 转换完成通知 |
| `alarms` | 定时任务 |

#### 可选主机权限

| 主机 | 用途 |
|------|------|
| `https://*.overleaf.com/*` | Overleaf 网站集成 |
| `https://*.arxiv.org/*` | arXiv 论文下载 |

### 8.10 插件与服务器集成

浏览器插件调用后端 API：

| API 端点 | 用途 |
|----------|------|
| `POST /api/v1/conversions` | 提交转换任务 |
| `GET /api/v1/conversions/{id}` | 查询转换状态 |
| `POST /api/v1/uploads` | 上传 LaTeX 文件 |
| `GET /api/v1/usage` | 查询用户配额 |
| `POST /api/v1/auth/register` | 用户注册 |
| `POST /api/v1/auth/login` | 用户登录 |
| `POST /api/v1/redeem` | 兑换码兑换 |

服务器需确保插件域名的 CORS 允许。

## 9. 故障排查

### 9.1 GLIBC 版本不兼容

**现象**：
```
/opt/tex2doc/current/server/doc-server: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.39' not found
```

**原因**：GitHub runner glibc 版本高于服务器

**解决**：确保 workflow 使用 `ubuntu-22.04`

### 9.2 SSH 认证失败

**现象**：
```
Load key "... error in libcrypto"
Permission denied (publickey)
```

**原因**：OpenSSH 私钥格式被破坏

**解决**：
```bash
cat ~/.ssh/tex2doc_prod_deploy | gh secret set PROD_SSH_KEY --repo shlwsh/Tex2Doc
```

### 9.3 健康检查 Connection Refused

**排查**：
```bash
systemctl status tex2doc-server --no-pager
journalctl -u tex2doc-server -n 50 --no-pager
ls -lh /opt/tex2doc/current/server/doc-server
```

**常见原因**：
- glibc 不兼容
- DATABASE_URL 配置错误
- 二进制缺少执行权限

### 9.4 Nginx 配置失败

```bash
sudo nginx -t
sudo tail -n 50 /var/log/nginx/error.log
```

## 10. 安全要求

1. **敏感信息不提交**：服务器密码、数据库密码、私钥不得写入仓库
2. **环境文件权限**：`doc-server.env` 权限保持 `600`
3. **数据库隔离**：PostgreSQL 不开放公网访问
4. **部署密钥专用**：使用专用部署私钥，不复用个人密钥
5. **插件发布**：Chrome Web Store 密钥妥善保管
6. **建议后续**：为 GitHub production environment 添加 required reviewers

## 11. 新服务器初始化

如需在新服务器部署，执行以下步骤：

### 11.1 安装系统依赖

```bash
sudo apt-get update
sudo apt-get install -y \
  nginx postgresql postgresql-contrib postgresql-client \
  tar unzip ca-certificates openssl \
  texlive-xetex texlive-luatex texlive-latex-recommended \
  texlive-latex-extra texlive-lang-chinese \
  texlive-bibtex-extra latexmk \
  fontconfig fonts-noto-cjk fonts-noto-cjk-extra

sudo fc-cache -fv
```

### 11.2 创建目录结构

```bash
sudo mkdir -p /opt/tex2doc/releases /opt/tex2doc/shared/env
sudo mkdir -p /opt/tex2doc/shared/sessions /opt/tex2doc/shared/logs
sudo chown -R ubuntu:ubuntu /opt/tex2doc
```

### 11.3 配置数据库

```bash
sudo systemctl enable --now postgresql
sudo -u postgres psql -c "CREATE USER tex2doc_app WITH PASSWORD 'YOUR_PASSWORD';"
sudo -u postgres createdb -O tex2doc_app docdb
sudo -u postgres psql -c "ALTER DATABASE docdb OWNER TO tex2doc_app;"
```

### 11.4 配置环境变量

创建 `/opt/tex2doc/shared/env/doc-server.env`：

```env
DOC_SERVER_ADDR=127.0.0.1:2624
DATABASE_URL=postgres://tex2doc_app:YOUR_PASSWORD@127.0.0.1:5432/docdb
RUST_LOG=info
TEX2DOC_STATIC_DIR=/opt/tex2doc/current/static
TEX2DOC_BOOTSTRAP_ADMIN_EMAIL=admin@example.com
TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD=YOUR_PASSWORD
```

```bash
chmod 600 /opt/tex2doc/shared/env/doc-server.env
```

### 11.5 配置 systemd 服务

创建 `/etc/systemd/system/tex2doc-server.service`：

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

```bash
sudo systemctl daemon-reload
sudo systemctl enable tex2doc-server
```

### 11.6 配置 sudoers

创建 `/etc/sudoers.d/tex2doc-deploy`：

```sudoers
ubuntu ALL=(root) NOPASSWD: /bin/systemctl restart tex2doc-server, /bin/systemctl reload nginx, /usr/sbin/nginx -t
```

```bash
sudo visudo -cf /etc/sudoers.d/tex2doc-deploy
```

### 11.7 配置 Nginx

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

```bash
sudo ln -sf /etc/nginx/sites-available/tex2doc /etc/nginx/sites-enabled/tex2doc
sudo rm -f /etc/nginx/sites-enabled/default
sudo nginx -t
sudo systemctl reload nginx
```

## 12. 后续优化建议

1. **HTTPS 配置**：申请 SSL 证书，启用 HTTPS
2. **对象存储**：大文件上传改用 COS 中转
3. **数据库备份**：建立定时备份和恢复演练机制
4. **macOS 构建**：恢复 macOS CI 矩阵（当前已移除以加快 CI）
5. **域名接入**：配置正式域名替代 IP 访问
6. **插件自动发布**：配置 GitHub Actions 自动发布到各商店
