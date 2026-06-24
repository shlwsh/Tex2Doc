# Flutter Web PWA 使用

> 本节描述 **Flutter Web PWA** 形态的使用方式。最适合：浏览器内转换、无需安装。

---

## 1. 启动 PWA（开发模式）

### 1.1 构建 WASM 产物

```bash
cd <仓库根>
npm run build:wasm
# 等价于：wasm-pack build crates/wasm --target web --out-dir ../flutter_app/wasm/pkg --out-name doc_engine --dev
```

产物：
* `flutter_app/wasm/pkg/doc_engine.js`（~14 KB）
* `flutter_app/wasm/pkg/doc_engine_bg.wasm`（~3.5 MB）

### 1.2 复制到 web/ 目录

`flutter build web` 会把 `wasm/pkg/` 内容复制到 `build/web/wasm/`，无需手工复制（默认会嵌入）。

### 1.3 构建 Flutter Web

```bash
npm run build:web
# 等价于：cd flutter_app && flutter build web --no-source-maps --no-tree-shake-icons
```

* `--no-source-maps`：减小产物（~1.5 MB）。
* `--no-tree-shake-icons`：保留所有 icon font。

### 1.4 启静态服务器

```bash
node scripts/serve_flutter_web.mjs
# 默认端口 2627
# 访问 http://127.0.0.1:2627/
```

或用任意静态服务器：

```bash
cd flutter_app/build/web
python -m http.server 2627
```

---

## 2. 浏览器端使用

### 2.1 打开页面

访问 `http://127.0.0.1:2627/`，应看到 Material 3 风格的 Flutter App。

### 2.2 状态卡

* **绿色对勾** + "引擎已就绪" + `Version: doc-native/0.1.0` 或 `0.1.0`。
* 若失败，显示红色错误图标 + 错误消息。

### 2.3 转换流程

* 当前 Flutter Web PWA 主要是**状态展示**（状态卡 + 转换卡）。
* 实际文件选择 + 转换：调用 `DocEngineFacade.convertZipToDocx(Uint8List, String)`。
* 文件输入：需要扩展 `workspace_app.dart` 加文件选择器（当前 V1 是状态卡 + 转换按钮 + 握手）。

> 实际产品级 Web PWA 通常用 `file_picker` 插件；当前仓库是核心展示。

---

## 3. PWA 部署（生产）

### 3.1 静态资源

把 `flutter_app/build/web/` 全部内容部署到任意静态服务器（Nginx / Caddy / Cloudflare Pages / Vercel）。

```bash
# 示例：拷贝到 /var/www/doc-engine
sudo cp -r flutter_app/build/web/* /var/www/doc-engine/
sudo nginx -s reload
```

### 3.2 Nginx 配置示例

```nginx
server {
    listen 80;
    server_name doc-engine.example.com;
    root /var/www/doc-engine;
    index index.html;

    # SPA 路由 fallback
    location / {
        try_files $uri $uri/ /index.html;
    }

    # WASM MIME
    types {
        application/wasm wasm;
    }

    # 缓存策略
    location ~* \.(js|css|wasm)$ {
        expires 7d;
        add_header Cache-Control "public, max-age=604800, immutable";
    }
    location ~* \.(png|svg|ico)$ {
        expires 30d;
    }
}
```

### 3.3 HTTPS

WASM 加载需要 HTTPS（浏览器安全策略）。可用：
* Let's Encrypt（certbot）
* Cloudflare 代理
* 自签名证书（仅内网）

### 3.4 CORS / COOP / COEP

WASM 共享内存 + `WebAssembly.Memory` 共享需要 cross-origin isolation：

```nginx
add_header Cross-Origin-Opener-Policy "same-origin" always;
add_header Cross-Origin-Embedder-Policy "require-corp" always;
```

> 我们的 WASM 是 `wasm-pack --target web`，默认无共享内存；COOP/COEP 可选。

### 3.5 PWA 安装

* Chrome 桌面 / 移动：自动提示「安装 Doc-engine」。
* iOS Safari：「分享 → 添加到主屏幕」。

---

## 4. 限制

| 限制 | 影响 | 建议 |
|------|------|------|
| 单文件 ≤ 50 MiB | 大工程被截断 | 拆 zip / 用桌面端 |
| 浏览器内存 ≤ 4 GB | 极大工程可能 OOM | 拆 zip |
| 首次加载 ~5 MB（canvaskit） | 移动端慢 | 评估 `--web-renderer html`（功能受限） |
| 不能直接读本地文件夹 | 需手工选择 zip | 评估 File System Access API |

---

## 5. 调试

### 5.1 浏览器 DevTools

* **Console**：`window.docEngine.version()` 验证 WASM 加载。
* **Network**：`doc_engine_bg.wasm` 加载状态。
* **Application → Service Workers**：PWA 注册情况。
* **Sources → Overrides**：临时改前端代码。

### 5.2 Flutter Web 调试

```bash
flutter run -d chrome --web-port 2626
```

* 支持 hot reload。
* `flutter inspect` 启用 DevTools。

### 5.3 端到端验证

```bash
node scripts/e2e_paper3.mjs
```

输出 `examples/paper3/output/playwright-report.html`，含截图 + 断言。

---

## 6. 进一步阅读

* [01-cli-and-script.md](./01-cli-and-script.md) — CLI / 脚本
* [04-chrome-extension.md](./04-chrome-extension.md) — Chrome 扩展
* [07-deployment/03-wasm-publish.md](../07-deployment/03-wasm-publish.md) — WASM 部署
