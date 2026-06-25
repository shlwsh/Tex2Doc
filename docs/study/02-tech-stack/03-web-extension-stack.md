# 浏览器 / 扩展 / Node 工具栈
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



> Tex2Doc 在浏览器端有两条独立路径：**Chrome MV3 扩展**和**Flutter Web PWA**（共享同一 WASM 产物）。Node.js 侧负责构建、端到端测试、CI 验证。

---

## 1.1 Chrome MV3 扩展

### 1.1.1 清单（`extension/manifest.json`）

```json
{
  "manifest_version": 3,
  "name": "Doc-engine",
  "version": "0.1.0",
  "description": "LaTeX → DOCX locally in the browser via WASM. (M11-M12 完整实现)",
  "minimum_chrome_version": "114",
  "action": {
    "default_popup": "popup/popup.html",
    "default_title": "Doc-engine"
  },
  "permissions": ["contextMenus", "clipboardWrite", "storage"],
  "host_permissions": ["<all_urls>"],
  "background": {
    "service_worker": "background.js"
  },
  "content_scripts": [
    {
      "matches": ["*://*.overleaf.com/*", "*://*.arxiv.org/*"],
      "js": ["content/content.js"],
      "run_at": "document_start"
    }
  ],
  "icons": {
    "16": "icons/icon16.png",
    "48": "icons/icon48.png",
    "128": "icons/icon128.png"
  }
}
```

### 1.1.2 关键能力

* **MV3 Service Worker**（`background.js`）：创建右键菜单、转发 PING / WRITE_CLIPBOARD 消息。
* **content script**（`content/content.js`）：在 Overleaf / arXiv 页面注入；监听 `selectionchange`，把选中文本存到 `chrome.storage.session`。
* **popup**（`popup/popup.html` + `popup.js` + `popup.css`）：
  * 文件选择（accept `.zip` / `.tex`）
  * 主 tex 路径输入
  * 状态栏（loading / ready / error）
  * 转换按钮
  * 结果下载
  * 大小分流：≥ 5 MB 弹 `chrome.notifications` 引导用户用桌面 App

### 1.1.3 WASM 加载

```javascript
// popup.js
const mod = await import('./wasm/doc_engine.js');
const wasmUrl = chrome.runtime.getURL('popup/wasm/doc_engine_bg.wasm');
const resp = await fetch(wasmUrl);
const wasmBuffer = await resp.arrayBuffer();
await mod.default({ wasmBinary: wasmBuffer });
docEngine = mod;
```

* `extension/popup/wasm/` 与 `flutter_app/wasm/pkg/` 内容**一致**（同一 `wasm-pack` 产物拷贝）。
* 加载方式：`chrome.runtime.getURL` 解析扩展内部 URL，`fetch` + `arrayBuffer` + `mod.default({wasmBinary})`。
* 调用：`docEngine.convert_zip_to_docx(zipBytes, mainTex, '')`。

### 1.1.4 安全边界

* `host_permissions: ["<all_urls>"]`：仅用于在所有页面注入 content script；不主动跨域。
* content script 域名白名单：仅 `*.overleaf.com` / `*.arxiv.org`。
* 单文件 5 MB 上限（避免 popup OOM）。

---

## 1.2 Flutter Web PWA

### 1.2.1 入口

`flutter_app/web/index.html`：

```html
<!doctype html>
<html>
<head>
  <link rel="manifest" href="manifest.json">
</head>
<body>
  <script src="wasm/doc_engine.js" defer></script>
  <script src="flutter_bootstrap.js" defer></script>
</body>
</html>
```

### 1.2.2 PWA manifest

`flutter_app/web/manifest.json`：

```json
{
  "name": "Doc-engine",
  "short_name": "Doc-engine",
  "start_url": ".",
  "display": "standalone",
  "background_color": "#ffffff",
  "theme_color": "#1565C0",
  "description": "LaTeX → DOCX locally in the browser.",
  "icons": [
    { "src": "icons/Icon-192.png", "sizes": "192x192" },
    { "src": "icons/Icon-512.png", "sizes": "512x512" }
  ]
}
```

### 1.2.3 路由

* `/` → `DocEngineApp`（Material 3 SPA）
* 无服务端路由（纯静态）。

---

## 1.3 Node.js / npm 工具栈

`package.json`（仓库根）：

```json
{
  "name": "tex2doc-verify",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "verify": "node scripts/verify_paper3.mjs",
    "verify:install": "node scripts/verify_install.mjs",
    "build:paper3-zip": "node scripts/build_paper3_zip.mjs",
    "build:wasm": "wasm-pack build crates/wasm --target web --out-dir ../flutter_app/wasm/pkg --out-name doc_engine --dev",
    "build:web": "cd flutter_app && flutter build web --no-source-maps --no-tree-shake-icons",
    "build:native": "cargo build -p doc-native",
    "build:windows": "cd flutter_app && flutter build windows --debug",
    "build:all": "npm run build:wasm && npm run build:web",
    "build:desktop": "npm run build:native && npm run build:windows",
    "serve:web": "node scripts/serve_flutter_web.mjs",
    "e2e:paper3": "node scripts/e2e_paper3.mjs",
    "e2e:server": "node scripts/e2e_server.mjs",
    "e2e:desktop": "cd flutter_app && dart run bin/native_smoke.dart",
    "e2e:extension": "node scripts/e2e_extension.mjs",
    "verify:e2e": "node scripts/build_paper3_zip.mjs && node scripts/e2e_paper3.mjs && node scripts/e2e_server.mjs && npm run e2e:desktop && npm run e2e:extension"
  },
  "dependencies": {
    "fflate": "^0.8.2"
  },
  "devDependencies": {
    "@playwright/test": "^1.49.0",
    "playwright": "^1.49.0"
  }
}
```

### 1.3.1 关键依赖

* **fflate** ^0.8.2：纯 JS 的 zip 读写。验证脚本用它解压 docx 抽 `word/document.xml`。
* **@playwright/test** ^1.49.0：浏览器端到端测试。
* **playwright** ^1.49.0：底层驱动。

### 1.3.2 端到端脚本

| 脚本 | 作用 |
|------|------|
| `scripts/build_paper3_zip.mjs` | 把 `examples/paper3/latex/` 打包成 `upload.zip` |
| `scripts/e2e_paper3.mjs` | 启 Playwright，打开 Flutter Web，上传 zip，断言内容 |
| `scripts/e2e_server.mjs` | 启 doc-server，curl `/api/v1/convert` 验证 |
| `scripts/e2e_extension.mjs` | 启 Playwright，加载 Chrome 扩展 popup，验证转换 |
| `scripts/verify_paper3.mjs` | 旧版验证脚本（保留）；调 cargo test 生成 docx，做内容断言 |
| `scripts/verify_install.mjs` | 环境自检 |
| `scripts/serve_flutter_web.mjs` | 启静态服务器（默认 2627），serve `flutter_app/build/web/` |
| `scripts/verify_paper3.ps1` | PowerShell 版 verify 入口（与 `verify_paper3.mjs` 对应） |

### 1.3.3 验证脚本关键技术

* **fflate 解压 docx**：
  ```javascript
  import { unzipSync, strFromU8 } from "fflate";
  const unzipped = unzipSync(new Uint8Array(buf));
  const xml = strFromU8(unzipped["word/document.xml"]);
  ```
* **段落抽取**：正则 `<w:p>...</w:p>` + `<w:t>...</w:t>` 嵌套匹配。
* **XML 实体解码**：`&amp;` `&lt;` `&gt;` `&quot;` `&apos;` `&#NNN;`。
* **关键短语断言**：与 `paper3_e2e.rs` 同步漂移（"微服务架构下" / "网关" / "Grafana Loki" / "石洪雷" / "赵涓涓"）。
* **杂质剥离断言**：21 个 LaTeX 装饰命令必须不存在。
* **Playwright 截图**：viewport 1280x1024，全页 PNG，写到 `examples/paper3/output/`。
* **HTML 报告**：内嵌 base64 PNG / 关键短语命中表 / 失败项 / 控制台前 20 行。

---

## 1.4 WASM 工具链

### 1.4.1 wasm-pack

* 工具：`wasm-pack` ≥ 0.12
* 目标：`--target web`（生成 ESM + import.meta.url 解析）
* 输出：`flutter_app/wasm/pkg/{doc_engine.js, doc_engine.d.ts, doc_engine_bg.wasm, package.json}`
* 副产物：手写 `flutter_app/web/index.html` 通过 `<script src="wasm/doc_engine.js" defer>` 加载；扩展 popup 走 `import('./wasm/doc_engine.js')`。

### 1.4.2 WASM 体积优化

* `--dev`：跳过 LTO / release 优化，编译快（开发用）。
* `--release`：生产用，未在 npm 脚本中默认启用。
* `wasm-opt`（wasm-pack 自动调用，默认 -Oz）。

> 当前 `doc_engine_bg.wasm` ≈ 3.5 MB（dev build）。

---

## 1.5 PowerShell 工具栈（Windows 提交）

`scripts/` 下：

* `commit_push.ps1`：自动 `git add -A` → `git commit --no-verify` → `git push origin <branch>`。
* `install_commit_push_hook.ps1`：设置 `core.hooksPath = .githooks`。
* `mygit.ps1` / `mygit.sh` / `mygit.py`：通用 mygit 脚本（来自 `.agent/skills/mygit`，与项目弱耦合）。

`scripts/verify_paper3.ps1`：PowerShell 版 verify 入口（与 `.mjs` 版本等价）。

---

## 1.6 Git 钩子

`.githooks/post-commit`：

```sh
#!/bin/sh
set -e
BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")
if [ -z "$BRANCH" ] || [ "$BRANCH" = "HEAD" ]; then
    exit 0
fi
if ! git rev-parse --abbrev-ref "$BRANCH@{u}" >/dev/null 2>&1; then
    echo "[post-commit] 首次推送，设置 upstream origin/$BRANCH"
    git push --set-upstream origin "$BRANCH" || echo "[post-commit] 推送失败"
else
    echo "[post-commit] 自动推送 origin/$BRANCH"
    git push origin "$BRANCH" || echo "[post-commit] 推送失败"
fi
```

* 仅在 `git commit` 之后触发，幂等。
* push 失败不阻断 commit 本体。
* 启用：仓库根 `git config core.hooksPath .githooks` 或 `scripts/install_commit_push_hook.ps1`。

---

## 1.7 CI 工具链（GitHub Actions）

`.github/workflows/ci.yml`：

```yaml
name: CI
on:
  push: { branches: [main] }
  pull_request: { branches: [main] }
env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: -D warnings

jobs:
  rust:
    name: Rust · ${{ matrix.os }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: [rustfmt, clippy] }
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: >-
            .
            crates/core crates/utils crates/semantic-ast
            crates/latex-reader crates/docx-writer crates/bib
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace --all-targets
      - if: matrix.os == 'ubuntu-latest'
        run: mkdir -p artifacts && cp crates/core/tests/output/hello.docx artifacts/hello.docx
      - uses: actions/upload-artifact@v4
        if: matrix.os == 'ubuntu-latest'
        with: { name: hello-docx, path: artifacts/hello.docx, if-no-files-found: ignore }
```

* **三平台矩阵**：`ubuntu-latest` / `windows-latest` / `macos-latest`。
* **强约束**：`RUSTFLAGS=-D warnings` + `cargo clippy -D warnings`。
* **缓存**：`Swatinem/rust-cache@v2` 缓存 7 个核心 crate 的 cargo 目录。
* **产物**：Ubuntu 平台跑通后上传 `hello.docx` 作为 CI artifact。

---

## 1.8 配套 gitnexus 索引

* 索引器：`.gitnexus/run.cjs`（自动 runner）
* 数据：2419 符号 / 5035 关系 / 203 执行流
* 工具：通过 `gitnexus` MCP 服务调用
* 文档：`AGENTS.md` / `CLAUDE.md` 中列出 always-do / never-do 规则
* 重新分析：`node .gitnexus/run.cjs analyze` 或 `npx gitnexus analyze`

> 修改函数前必跑 `impact`；提交前必跑 `detect_changes`。
