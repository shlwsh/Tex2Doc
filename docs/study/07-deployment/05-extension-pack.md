# Chrome 扩展打包

> 本节描述 `extension/` 目录的打包与发布流程：开发模式加载、CRX 自托管、Chrome Web Store 上架。

---

## 1. 准备

### 1.1 WASM 产物

扩展依赖 `extension/popup/wasm/doc_engine_bg.wasm` + `doc_engine.js`。

构建：

```bash
# 仓库根
npm run build:wasm
# 产物：flutter_app/wasm/pkg/

# 复制到扩展
cp flutter_app/wasm/pkg/doc_engine.js extension/popup/wasm/
cp flutter_app/wasm/pkg/doc_engine_bg.wasm extension/popup/wasm/
```

或自动化（PowerShell）：

```powershell
# scripts/copy_wasm_to_extension.ps1
$src = "flutter_app/wasm/pkg"
$dst = "extension/popup/wasm"
New-Item -ItemType Directory -Force -Path $dst | Out-Null
Copy-Item "$src/doc_engine.js" $dst -Force
Copy-Item "$src/doc_engine_bg.wasm" $dst -Force
Write-Host "✅ WASM copied to extension"
```

### 1.2 图标

确认 `extension/icons/icon{16,48,128}.png` 存在（已入仓）。

### 1.3 manifest 版本

`extension/manifest.json`：

```json
{
  "version": "0.1.0",
  ...
}
```

* 递增规则：修复补丁 +0.0.1；新功能 +0.1.0；破坏性 +1.0.0。

---

## 2. 开发模式加载

### 2.1 加载步骤

1. 打开 Chrome `chrome://extensions/`。
2. 右上角开启「开发者模式」。
3. 点击「加载已解压的扩展程序」。
4. 选择 `extension/` 目录。
5. 出现 "Doc-engine" + 工具栏图标。

### 2.2 修改后重载

* 修改 popup.html / popup.js / popup.css 后：点击扩展卡片刷新按钮 🔄。
* 修改 manifest.json 后：必须重载扩展。
* 修改 background.js 后：点击「Service Worker」链接旁的「终止」按钮，重启。

---

## 3. 端到端测试

### 3.1 Playwright（自动）

```bash
# 一次性
npx playwright install chromium

# 跑 e2e
node scripts/e2e_extension.mjs
```

行为：
* 启 Playwright Chromium。
* 加载 unpacked extension。
* 触发弹窗 / 转换流程。
* 内容断言 + 截图。

### 3.2 手动清单

- [ ] 加载扩展成功
- [ ] 弹窗能打开（点工具栏图标）
- [ ] 状态栏显示「就绪」+ 版本
- [ ] 选小文件（< 5 MB）
- [ ] 转换成功，下载 .docx
- [ ] Word 能打开 docx
- [ ] 选大文件（≥ 5 MB）→ 弹通知
- [ ] 在 Overleaf 页面注入 content script

---

## 4. Chrome Web Store 上架

### 4.1 准备

* **注册开发者账号**：[Chrome Web Store Developer Dashboard](https://chrome.google.com/webstore/devconsole/)
* **一次性注册费**：$5。
* **合规要求**：
  * 隐私政策 URL（必须 hosted on your domain）
  * 单用途说明
  * 截图（1280×800 或 640×400）
  * 图标（128×128 PNG）

### 4.2 打包

```bash
# 1. 进入 extension 目录
cd extension

# 2. 打包为 zip（不包含 .git 等隐藏文件）
zip -r ../doc-engine-v0.1.0.zip . -x "*.DS_Store" "*.git*"
```

### 4.3 上传

1. 访问 [Chrome Web Store Developer Dashboard](https://chrome.google.com/webstore/devconsole/)。
2. 点击「新增项」。
3. 上传 zip。
4. 填写：
   * **商品详情**：名称、摘要、详细描述、截图。
   * **图形资源**：图标、宣传图。
   * **类别**：生产力 / 开发者工具。
   * **隐私**：单用途说明 + 隐私政策。
5. 提交审核。

### 4.4 审核要求

* 详细权限说明。
* 隐私政策（不收集个人数据时也要说明）。
* 单用途：「把 LaTeX 项目转换为 docx」。
* 截图清晰展示核心功能。

### 4.5 审核时长

* 通常 1-3 天。
* 拒绝原因：权限过度、未声明数据收集、图标不规范。

---

## 5. 自托管 CRX

### 5.1 生成私钥

```bash
# 仅首次
openssl genrsa -out /path/to/doc-engine-extension.pem 2048
```

### 5.2 打包

```bash
# 在仓库根
google-chrome --pack-extension=extension/ --pack-extension-key=/path/to/doc-engine-extension.pem

# 产物：
#   extension.crx        — 扩展包
#   extension.pem        — 私钥备份
```

### 5.3 部署

```bash
# 上传到你的服务器
scp extension.crx user@server:/var/www/downloads/

# 用户安装：
# 1. 访问 https://example.com/downloads/extension.crx
# 2. Chrome 弹「添加扩展？」→ 确认
```

需要：
* HTTPS（CRX 必须 HTTPS）。
* `Content-Type: application/x-chrome-extension`。
* 服务器 CSP 允许 `chrome-extension://`。

### 5.4 自动更新

CRX 支持自动更新，需在服务器上提供 `update.xml`：

```xml
<?xml version='1.0' encoding='UTF-8'?>
<gupdate xmlns='http://www.google.com/update2/response' protocol='2.0'>
  <app appid='<extension-id>'>
    <updatecheck codebase='https://example.com/downloads/extension.crx' version='0.1.1' />
  </app>
</gupdate>
```

* `appid`：扩展的 Chrome Web Store ID（首次发布后才有；自托管则用打包生成的 ID）。

> Tex2Doc 当前**未实现**自动更新（V2 路线图）。

---

## 6. 企业分发

### 6.1 Chrome Enterprise Policy

```json
// Windows: registry / macOS: plist / Linux: JSON
{
  "ExtensionSettings": {
    "doc-engine-extension-id": {
      "installation_mode": "force_installed",
      "update_url": "https://example.com/downloads/extension/updates.xml"
    }
  }
}
```

### 6.2 MSI / PKG

用 [Chrome Enterprise Bundle](https://support.google.com/chrome/a/answer/7572896) 把 CRX 打包为 MSI（Windows）/ PKG（macOS）。

---

## 7. CI 集成

### 7.1 自动构建 CRX

```yaml
# .github/workflows/release-extension.yml
name: Release Extension

on:
  push:
    tags: ['extension-v*']

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Build WASM
        run: |
          cargo install wasm-pack
          rustup target add wasm32-unknown-unknown
          npm run build:wasm

      - name: Copy WASM to extension
        run: |
          mkdir -p extension/popup/wasm
          cp flutter_app/wasm/pkg/doc_engine.js extension/popup/wasm/
          cp flutter_app/wasm/pkg/doc_engine_bg.wasm extension/popup/wasm/

      - name: Package extension
        run: |
          cd extension
          zip -r ../doc-engine.zip . -x "*.DS_Store"

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: doc-engine-extension
          path: doc-engine.zip
```

### 7.2 自动发布到 Web Store

用 [`chrome-webstore-upload-cli`](https://www.npmjs.com/package/chrome-webstore-upload-cli)：

```bash
npm install -g chrome-webstore-upload-cli

# 配置凭据（环境变量）
export CHROME_EXTENSION_ID=<id>
export CHROME_CLIENT_ID=<client_id>
export CHROME_CLIENT_SECRET=<client_secret>
export CHROME_REFRESH_TOKEN=<refresh_token>

# 上传
chrome-webstore-upload --source doc-engine.zip --auto-publish
```

---

## 8. 故障排查

### 8.1 扩展加载失败

| 错误 | 解决 |
|------|------|
| `manifest_version must be 3` | Chrome 升级到 88+ |
| `minimum_chrome_version too low` | 升级 Chrome |
| `Invalid manifest` | 用 `chrome://extensions/` 的「错误」链接看详情 |
| `Could not load background script` | 检查 `background.js` 语法 |

### 8.2 Service Worker 不启动

* 原因：MV3 Service Worker 会因 idle 而被杀死。
* 解决：用 `chrome.alarms` 保持活跃，或事件触发。

### 8.3 Content script 不注入

* 原因：URL 匹配规则不命中。
* 解决：检查 `manifest.json` 的 `matches` 数组。

### 8.4 WASM 加载失败

* 原因：路径 / CORS / Content-Type。
* 解决：检查 `chrome.runtime.getURL('popup/wasm/doc_engine_bg.wasm')` 返回的 URL。

### 8.5 大文件 popup 崩溃

* 原因：单文件 > 5 MB。
* 解决：限制 5 MB（已实现），引导用桌面 App。

---

## 9. 进一步阅读

* [02-flutter-build.md](./02-flutter-build.md) — Flutter 构建
* [03-wasm-publish.md](./03-wasm-publish.md) — WASM 产物
* [06-ci-and-hooks.md](./06-ci-and-hooks.md) — CI / 钩子
* [06-user-guide/04-chrome-extension.md](../06-user-guide/04-chrome-extension.md) — 使用方式
