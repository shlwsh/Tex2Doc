# `extension/` / `scripts/` / `tests/` / `examples/` 详尽说明

---

## 1. `extension/` — Chrome MV3 扩展

### 1.1 目录树

```
extension/
├── manifest.json                # MV3 清单
├── background.js                # Service Worker
├── README.md
│
├── content/
│   └── content.js               # Overleaf / arXiv 选区监听
│
├── popup/
│   ├── popup.html               # 360×240 弹窗 UI
│   ├── popup.css                # Material 3 轻量化风格
│   ├── popup.js                 # WASM 加载 + 文件选择 + 转换 + 下载
│   └── wasm/
│       ├── doc_engine.js
│       └── doc_engine_bg.wasm
│
└── icons/
    ├── icon16.png
    ├── icon48.png
    └── icon128.png
```

### 1.2 `manifest.json`（关键能力）

* `manifest_version: 3`
* `minimum_chrome_version: 114`
* `permissions: ["contextMenus", "clipboardWrite", "storage"]`
* `host_permissions: ["<all_urls>"]`
* `background.service_worker: "background.js"`
* `content_scripts` 域名白名单：`*.overleaf.com` / `*.arxiv.org`，`run_at: document_start`
* `action.default_popup: "popup/popup.html"`

### 1.3 `background.js`（Service Worker）

* `install`：创建右键菜单 "使用 Doc-engine 转换"，context `selection`。
* `activate`：日志。
* `contextMenus.onClicked`：转发 `OPEN_POPUP` 消息给 popup。
* `runtime.onMessage`：
  * `PING` → `{ ok: true, version: '0.1.0' }`
  * `WRITE_CLIPBOARD` → `navigator.clipboard.writeText`

### 1.4 `content/content.js`

* 仅在 `*.overleaf.com` / `*.arxiv.org` 注入。
* 监听 `selectionchange`：把选中文本存到 `chrome.storage.session`。
* 通知 `CONTENT_SCRIPT_READY`。

### 1.5 `popup/`

* 360 宽弹窗；Material 3 风格。
* 文件选择（accept `.zip` / `.tex`，限 5 MB）。
* 主 tex 路径输入（默认 `main-jos.tex`）。
* 状态栏（loading / ready / error）。
* 转换按钮（loading 时禁用）。
* 结果区：产物大小 + 耗时 + 下载按钮。
* 错误区：错误信息。

### 1.6 `popup.js`（WASM 加载 + 转换流程）

```javascript
async function initWasm() {
  const mod = await import('./wasm/doc_engine.js');
  const wasmUrl = chrome.runtime.getURL('popup/wasm/doc_engine_bg.wasm');
  const resp = await fetch(wasmUrl);
  const wasmBuffer = await resp.arrayBuffer();
  await mod.default({ wasmBinary: wasmBuffer });
  docEngine = mod;
  wasmReady = true;
}

// 转换
const result = docEngine.convert_zip_to_docx(zipBytes, mainTex, '');
// 验证 docx 头
if (docxBytes[0] !== 0x50 || docxBytes[1] !== 0x4B) throw new Error('docx 头部非 ZIP');
// 下载
const blob = new Blob([docxBytes], { type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document' });
const url = URL.createObjectURL(blob);
const a = document.createElement('a');
a.href = url;
a.download = (zipFileName || 'output').replace(/\.[^.]+$/, '') + '.docx';
a.click();
```

### 1.7 5 MB 大小限制

超过时弹 `chrome.notifications` 引导用户用桌面 App；禁用转换按钮；清空文件选择。

---

## 2. `scripts/` — 工具脚本

### 2.1 完整清单

| 脚本 | 平台 | 行数 | 作用 |
|------|------|------|------|
| `build_paper3_zip.mjs` | Node | 79 | 把 `examples/paper3/latex/` → `upload.zip` |
| `commit_push.ps1` | PowerShell | 144 | 自动 add / commit / push |
| `e2e_extension.mjs` | Node | 200+ | Playwright 验证 Chrome 扩展 |
| `e2e_paper3.mjs` | Node | 328 | Playwright 验证 Flutter Web |
| `e2e_server.mjs` | Node | 30+ | 验证 doc-server HTTP |
| `install_commit_push_hook.ps1` | PowerShell | 100+ | 启用 post-commit 钩子 |
| `link_cursor_skills.sh` | Bash | 30+ | 链接 .cursor/skills 到 .agent/skills |
| `mygit.ps1` | PowerShell | 200+ | 通用 mygit 工具 |
| `mygit.sh` | Bash | 50+ | 通用 mygit 工具 |
| `mygit.py` | Python | 800+ | 通用 mygit 工具 |
| `serve_flutter_web.mjs` | Node | 100+ | 静态服务器（端口 4173） |
| `test_proxy.py` | Python | 30+ | 测试代理 |
| `verify_install.mjs` | Node | 20+ | 环境自检 |
| `verify_paper3.mjs` | Node | 272 | 旧版 verify（保留） |
| `verify_paper3.ps1` | PowerShell | 130+ | PowerShell 版 verify |

### 2.2 关键脚本详解

#### `build_paper3_zip.mjs`

```javascript
// 1) 读 examples/paper3/latex/ 全部 .tex / .cls / .bib（递归）
// 2) 用 fflate 打包成 zip
// 3) 写 examples/paper3/upload.zip
```

#### `e2e_paper3.mjs`

```javascript
// 1) 启 Playwright + Chromium
// 2) 访问 $BASE_URL (默认 http://127.0.0.1:4173/)
// 3) 等 Flutter 容器挂载 + 2s 渲染 → 截图 flutter-app.png
// 4) 等 window.docEngine 就绪 → 读 version()
// 5) 上传 paper3/upload.zip → 调 window.docEngine.convert_zip_to_docx
// 6) 解压 docx → 抽 document.xml → 关键短语断言（5 个）
// 7) 杂质命令断言（21 个）
// 8) 写 playwright-report.html（HTML 报告）
// 9) 退出码：0=通过，1=失败
```

#### `verify_paper3.mjs`

```javascript
// 1) 调 cargo test -p doc-core --test paper3_e2e 生成 docx
// 2) fflate 解压 docx
// 3) 抽 <w:p> + <w:t> 段落
// 4) 关键短语 + 杂质命令断言
// 5) 用 Playwright 渲染 HTML 报告 + 截图
// 6) 退出码：0/1
```

#### `verify_paper3.ps1`

* PowerShell 入口，行为同 `.mjs`。
* 适合 Windows 上手工调（CI 仍跑 `.mjs`）。

#### `serve_flutter_web.mjs`

```javascript
// 启 http.createServer，serve flutter_app/build/web/
// 端口默认 4173
// MIME 自定
```

#### `commit_push.ps1`

```powershell
# 1) 检查 git 仓库
# 2) 若工作区干净，退出 0
# 3) 解析 -Message（第一行作标题）+ 可选 -Scope + -Body
# 4) git add -A
# 5) git commit --no-verify -m $commitMsg
# 6) git push origin <branch>（首次自动 --set-upstream；可选 -ForcePush）
```

#### `install_commit_push_hook.ps1`

```powershell
# git config core.hooksPath .githooks
# 或 -Uninstall 恢复
```

---

## 3. `tests/` — 跨 crate 共享夹具

```
tests/
└── fixtures/
    └── ieee/
        ├── ieee_simple.tex      # 577 字节
        └── ieee_nested.tex      # 540 字节
```

### 3.1 `ieee_simple.tex`

最小 IEEE 风格：
* `\documentclass{article}` + `amsmath` / `graphicx`
* `\section{First Section}`
* 段落 + `\textbf` / `\textit`
* `itemize` 列表
* `tabular{c|c}` 简单表格
* `figure` + `\includegraphics`
* `equation` 块

### 3.2 `ieee_nested.tex`

嵌套结构：
* `\section{Nested}`
* 嵌套 `itemize`（两层）+ `enumerate`
* 段落 + `\ref`
* `equation` + `pmatrix`（LaTeX 矩阵语法）

> 这两个夹具供 `crates/core/tests/ieee_fixtures.rs` 等使用。

---

## 4. `examples/` — 示例项目

### 4.1 `examples/paper3/`

主示例。完整学术论文（rjthesis 模板）。

```
examples/paper3/
├── upload.zip                   # ~42 KB（由 build_paper3_zip.mjs 生成）
└── latex/
    ├── .latexmkrc               # latexmk 配置
    ├── rjthesis.cls             # 期刊 class 文件
    ├── main-jos.tex             # 主源（英文）—— e2e 入口
    ├── main-zh.tex              # 主源（中文）
    ├── references.bib           # BibTeX 数据库（19 KB）
    ├── chk.{tex,aux,log,pdf}    # 校验用
    ├── main-jos.{aux,bbl,blg,log,out,pdf}    # LaTeX 编译产物
    ├── main-zh.{aux,bbl,blg,log,out,pdf}     # 同上
    ├── test_spacing.{aux,log,pdf}            # 间距测试
    └── sections/zh/             # 各章节子文件
        ├── 00_abstract.tex
        ├── 01_intro.tex
        ├── 02_related.tex
        ├── 03_system.tex
        ├── 04_algorithms.tex
        ├── 05_implementation.tex
        ├── 06_experiments.tex
        └── 07_conclusion.tex
```

### 4.2 关键统计

* `main-jos.tex` ~ 8 KB
* 6 个 `\input` 子文件 + `references.bib`
* 5 个关键短语（断言目标）："微服务架构下" / "网关" / "Grafana Loki" / "石洪雷" / "赵涓涓"
* 21 个杂质命令（必须剥离）：`\AbstractContentZh` / `\AbstractContentEn` / `\KeywordsZh` / `\KeywordsEn` / `\documentclass` / `\usepackage` / `\PassOptionsToClass` / `\geometry` / `\begin{CJK}` / `\hypersetup` / `\newcommand` / `\fancyhead` / `\rjtitle` / `\rjauthor` / `\rjinfor` / `\rjkeywords` / `\rjcategory` / `\rjmaketitle` / `\bibliographystyle` / `{ctexart}` / `{rjthesis}`

### 4.3 端到端验证产物

* `examples/paper3/output/main-jos.docx`（Rust 集成测试产物）
* `examples/paper3/output/desktop-main-jos.docx`（Dart 桌面端冒烟产物）
* `examples/paper3/output/preview.png`（Playwright 报告截图）
* `examples/paper3/output/report.html`（Playwright 报告）
* `examples/paper3/output/flutter-app.png`（Flutter Web 渲染截图）
* `examples/paper3/output/playwright-report.html`（Playwright e2e 报告）

### 4.4 复用建议

* 添加新示例：保持 `latex/main-xxx.tex` + 多个 `sections/xxx/*.tex` 结构。
* 验证脚本需相应更新 `REQUIRED_PHRASES` / `FORBIDDEN` 列表。
* CI 默认跑 `examples/paper3`（已在 `.github/workflows/ci.yml` 中）。

---

## 5. `docs/` — 项目文档

> 已在 [01-top-level.md](./01-top-level.md) §9 详述。

新增文档建议放 `docs/study/` 体系（当前 study 目录已建 7 个子目录）；设计/方案/计划类仍放 `docs/` 根。

---

## 6. 进一步阅读

* [05-key-tech/](../05-key-tech/) — 深入解析
* [06-user-guide/](../06-user-guide/) — 各类使用方式
* [07-deployment/](../07-deployment/) — 构建/部署
