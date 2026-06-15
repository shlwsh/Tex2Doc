# Chrome MV3 扩展使用

> 本节描述 **Chrome MV3 扩展** 形态的使用方式。最适合：在 Overleaf / arXiv 页面快速转换、无需离开浏览器。

---

## 1. 安装扩展

### 1.1 开发模式加载

1. 打开 Chrome 访问 `chrome://extensions/`。
2. 右上角开启「开发者模式」。
3. 点击「加载已解压的扩展程序」。
4. 选择仓库根的 `extension/` 目录。

### 1.2 验证加载

* 扩展列表出现 "Doc-engine" + 版本 `0.1.0`。
* 浏览器右上角工具栏出现 Doc-engine 图标。
* 点击图标 → 弹窗显示 360×240 Material 3 风格 UI。

### 1.3 重新加载

修改 popup / background / content 脚本后，在扩展列表点击刷新按钮 🔄。

---

## 2. 浏览器端使用

### 2.1 弹窗界面

* 标题栏：📖 Doc-engine + 版本徽章。
* 文件选择："选择 .zip 文件" 按钮 + 文件名显示。
* 主 tex 路径：默认 `main-jos.tex`，可改。
* 状态栏：loading / ready / error。
* 转换按钮：「开始转换」（未就绪时禁用）。
* 结果区：产物大小 + 耗时 + 下载按钮。
* 错误区：错误消息（红字）。

### 2.2 典型流程

1. 点击「选择 .zip 文件」→ 选你的 LaTeX 项目 zip（≤ 5 MB）。
2. 确认主 tex 路径（默认 `main-jos.tex`）。
3. 点击「开始转换」。
4. 状态栏显示「完成 XXX KB（YYms）」。
5. 点击「下载 .docx」→ 浏览器下载 docx 文件。

### 2.3 大小限制

文件 ≥ 5 MB 时：
* 显示错误「文件过大」。
* 弹 Chrome 通知（`chrome.notifications.create`）提示「请使用桌面 App 或 PWA」。
* 清空文件选择。

### 2.4 错误处理

| 错误 | 表现 |
|------|------|
| WASM 加载失败 | 状态栏「WASM 加载失败：...」+ 红字 |
| 文件读取失败 | 错误区「文件读取失败」 |
| 转换失败 | 状态栏「转换失败」+ 错误区详情 |
| docx 过小 / 魔数错 | 错误区「docx 过小：...」/「docx 头部非 ZIP」 |

---

## 3. 浏览器集成

### 3.1 右键菜单

* 打开任意网页（如 Overleaf），选中一段文本。
* 右键 → 「使用 Doc-engine 转换」。
* Chrome service worker 收到 `OPEN_POPUP` 消息。
* 弹窗打开（需先点扩展图标，再触发）。

### 3.2 内容脚本（Overleaf / arXiv）

* 在 `*.overleaf.com` / `*.arxiv.org` 页面自动注入。
* 监听 `selectionchange`：把选中文本存到 `chrome.storage.session`。
* 弹窗（V2 路线）可读 `selectedText` 并自动填充到主文件路径或注释。

> 当前 popup 未读取 `selectedText`（V2 计划）。

---

## 4. 测试扩展

### 4.1 端到端（Playwright）

```bash
node scripts/e2e_extension.mjs
```

* 启 Playwright Chromium。
* 加载 `extension/` 目录为 unpacked extension。
* 触发右键菜单 / popup / 转换流程。
* 截图 + 内容断言。

### 4.2 手动测试清单

* [ ] 安装扩展
* [ ] 弹窗能打开
* [ ] 选小文件（< 5 MB）
* [ ] 转换成功
* [ ] 下载 .docx 成功
* [ ] 选大文件（≥ 5 MB）→ 弹通知
* [ ] 关闭/重开浏览器 → 扩展仍加载
* [ ] 访问 Overleaf → content script 注入（DevTools → Elements → 检查 `<script>`）

---

## 5. 调试

### 5.1 弹窗调试

* 右键弹窗 → 「审查弹出内容」→ DevTools 打开。
* Console / Network / Sources 面板可用。

### 5.2 Service Worker 调试

* `chrome://extensions/` → Doc-engine → 「Service Worker」链接。
* Console / Network 面板可用。

### 5.3 内容脚本调试

* 打开 Overleaf 页面 → DevTools → Console。
* 输入 `chrome.runtime.sendMessage({ type: 'PING' })` → 应回 `{ ok: true, version: '0.1.0' }`。

### 5.4 存储查看

* `chrome://extensions/` → Doc-engine → 「检查视图：background page」或「service worker」。
* Console 输入 `chrome.storage.local.get(console.log)` / `chrome.storage.session.get(console.log)`。

---

## 6. 限制

| 限制 | 影响 | 建议 |
|------|------|------|
| popup 单文件 ≤ 5 MB | 大工程失败 | 用桌面 App / PWA |
| popup WASM 加载慢（~3.5 MB） | 首次慢 | 评估缓存策略 |
| content script 仅 Overleaf / arXiv | 其它站点不工作 | 评估更多站点 |
| 不支持 File System Access API | 不能直接选目录 | V2 评估 |
| 不读 `selectedText` 到 popup | 手动填主 tex | V2 计划 |

---

## 7. 打包分发

### 7.1 生成 .crx

```bash
# 打包为 .crx（自托管）
chrome --pack-extension=extension/ --pack-extension-key=key.pem
```

### 7.2 Chrome Web Store

需要：
* 注册 Chrome Web Store Developer 账号（一次性 $5）。
* 上传 zip 形式的 `extension/` 目录。
* 填写商品详情、截图、隐私政策。
* 等待审核（通常 1-3 天）。

### 7.3 自托管 CRX

```bash
# 生成私钥（首次）
openssl genrsa -out key.pem 2048

# 打包
chrome --pack-extension=extension/ --pack-extension-key=key.pem
# 产物：extension.crx + extension.pem
```

部署到自托管服务器，添加 CSP / CORS 允许 `application/x-chrome-extension`。

---

## 8. 进一步阅读

* [01-cli-and-script.md](./01-cli-and-script.md) — CLI / 脚本
* [02-pwa-web.md](./02-pwa-web.md) — Flutter Web PWA
* [07-deployment/05-extension-pack.md](../07-deployment/05-extension-pack.md) — 扩展打包
