# e2e_wasm_convert.mjs 端到端测试脚本说明

> 文档版本：v1.0  
> 更新日期：2026-06-28

## 1. 脚本概述

`scripts/e2e_wasm_convert.mjs` 是 Tex2Doc 浏览器插件的核心端到端测试脚本，专门用于验证 **Chrome MV3 扩展内 WASM 本地转换**链路的正确性。

### 核心验证目标

| 问题类型 | 错误示例 | 验证内容 |
|---------|---------|---------|
| WASM 导入错误 | `__wbindgen_object_drop_ref` | WASM 模块是否正确初始化 |
| ESM 动态导入 | `import()` MV3 禁止 | 胶水代码是否静态打包进 background.js |
| Service Worker API | `window is not defined` | `URL.createObjectURL` fallback 是否生效 |

---

## 2. 架构图

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Playwright Node.js 进程                        │
│                                                                      │
│   e2e_wasm_convert.mjs                                              │
│   ┌─────────────────┐   ┌──────────────────┐                       │
│   │ 读取 upload.zip  │──▶│ Playwright       │                       │
│   │ (number[])      │   │ launchPersistent │                       │
│   └─────────────────┘   │ Context         │                       │
│                         └────────┬─────────┘                       │
│                                  │ 加载 chrome-mv3 扩展             │
└──────────────────────────────────┼──────────────────────────────────┘
                                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     Chromium Browser (headless)                       │
│                                                                      │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │ Service Worker (background.js)                                │  │
│  │                                                               │  │
│  │  入口点                                                        │  │
│  │  ┌───────────────────┐  ┌─────────────────────────────┐      │  │
│  │  │ globalThis.__     │  │ chrome.runtime.onMessage   │      │  │
│  │  │ tex2docConvertZip │  │ .addListener(handleMsg)   │      │  │
│  │  │ (e2e 专用钩子)     │  │                             │      │  │
│  │  └─────────┬─────────┘  └──────────────┬──────────────┘      │  │
│  │            │                             │                      │  │
│  │            ▼                             ▼                      │  │
│  │  ┌─────────────────────────────────────────────────────────┐   │  │
│  │  │          handleStartWasmConversion()                   │   │  │
│  │  │  ┌─────────────────────────────────────────────────┐   │   │  │
│  │  │  │  1. bytes = new Uint8Array(zipBytes)            │   │   │  │
│  │  │  │  2. result = await convertLocal(bytes, opts)    │   │   │  │
│  │  │  │  3. return { docxBytes: Array.from(result) }    │   │   │  │
│  │  │  └─────────────────────────────────────────────────┘   │   │  │
│  │  └─────────────────────────────────────────────────────────┘   │  │
│  │                           │                                    │  │
│  │                           ▼                                    │  │
│  │  ┌─────────────────────────────────────────────────────────┐   │  │
│  │  │              convertLocal()                              │   │  │
│  │  │  @/conversion/local-wasm.ts                             │   │  │
│  │  │                                                         │   │  │
│  │  │  ├── isWasmReady() → false?                             │   │  │
│  │  │  │   └── initWasm()                                     │   │  │
│  │  │  │                                                      │   │  │
│  │  │  └── convertZipToDocxBytes(bytes, mainTex, options)    │   │  │
│  │  │       @/workers/wasm-worker.ts                         │   │  │
│  │  └─────────────────────────────────────────────────────────┘   │  │
│  │                           │                                    │  │
│  │                           ▼                                    │  │
│  │  ┌─────────────────────────────────────────────────────────┐   │  │
│  │  │         WebAssembly.instantiate()                        │   │  │
│  │  │                                                         │   │  │
│  │  │  胶水来源 (post-build-wasm.mjs 处理后):                   │   │  │
│  │  │  ├── ESM export {} → globalThis.__tex2docApi           │   │  │
│  │  │  ├── ESM import.meta.url → globalThis.__tex2docBaseUrl │   │  │
│  │  │  └── __wbg_init → globalThis.__tex2docWbg              │   │  │
│  │  │                                                         │   │  │
│  │  │  WASM 二进制:                                           │   │  │
│  │  │  browser.runtime.getURL('/wasm/doc_engine_bg.wasm')    │   │  │
│  │  └─────────────────────────────────────────────────────────┘   │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                      │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │ Extension Page (触发 SW 事件循环)                               │  │
│  │   await page.goto('https://example.com')                      │  │
│  └───────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                                   │
                                   ▼ (转换成功)
┌─────────────────────────────────────────────────────────────────────┐
│                        Node.js 回调处理                               │
│                                                                      │
│   e2eResult = {                                                      │
│     ok: true,                                                        │
│     reply: {                                                          │
│       success: true,                                                  │
│       jobId: "uuid",                                                  │
│       docxBytes: number[]  ←── e2e 模式返回字节                        │
│     }                                                                │
│   }                                                                  │
│                                                                      │
│   ┌────────────────┐                                                │
│   │ 写文件验证      │                                                │
│   │ target/e2e-    │                                                │
│   │ out.docx       │                                                │
│   └───────┬────────┘                                                │
│           ▼                                                          │
│   ┌────────────────┐   ┌────────────────────┐                      │
│   │ 验证 magic     │──▶│ 0x50 0x4B (PK)     │                      │
│   │ bytes          │   │ DOCX = ZIP 格式     │                      │
│   └────────────────┘   └────────────────────┘                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 3. 执行流程详解

### 3.1 参数解析

```javascript
const zipPath = process.argv[2] || 'D:\\temp\\upload.zip';
const outDocx = process.argv[3] || path.join(repoRoot, 'target', 'e2e-out.docx');
```

| 参数 | 默认值 | 说明 |
|-----|-------|------|
| `argv[2]` | `D:\temp\upload.zip` | 输入的 paper3 zip 文件 |
| `argv[3]` | `target/e2e-out.docx` | 输出的 docx 文件路径 |

**完整命令示例：**

```bash
node scripts/e2e_wasm_convert.mjs "D:\papers\upload.zip" "D:\temp\output.docx"
```

### 3.2 环境准备

```javascript
// 清空 user-data-dir 防止老 SW 注册缓存干扰
const userDataDir = path.join(repoRoot, 'target', 'e2e-userdata');
await fs.rm(userDataDir, { recursive: true, force: true });
```

**作用：** 清除 Playwright 使用的 Chromium 用户数据目录，确保每次测试都从干净的扩展状态开始。

### 3.3 启动 Chromium 并加载扩展

```javascript
const context = await chromium.launchPersistentContext(userDataDir, {
  channel: 'chromium',        // 必须用系统安装的 Chromium，不是 Playwright 内置
  headless: true,
  args: [
    `--disable-extensions-except=${extPath}`,
    `--load-extension=${extPath}`,
    '--no-sandbox',
  ],
  timeout: 120_000,
});
```

**关键技术点：**

| 配置 | 说明 |
|-----|------|
| `channel: 'chromium'` | Playwright headless 默认禁用扩展，必须指定系统 Chromium |
| `--disable-extensions-except` | 只启用目标扩展，禁用其他扩展 |
| `--load-extension` | 指定扩展目录路径 |

### 3.4 Service Worker 就绪等待

```javascript
if (context.serviceWorkers().length === 0) {
  console.log('[e2e-wasm] 等待 service worker 就绪（最多 60s）...');
  serviceWorker = await context.waitForEvent('serviceworker', { timeout: 60_000 });
} else {
  serviceWorker = context.serviceWorkers()[0];
}
```

**为什么需要等待？**  
MV3 Service Worker 是懒加载的，只有在收到事件时才会启动。

### 3.5 WASM 转换调用

```javascript
const e2eResult = await serviceWorker.evaluate(async (args) => {
  // 关键：绕过 message channel，直接调用全局函数
  const fn = globalThis.__tex2docConvertZip;
  const reply = await fn({
    zipBytes: Array.from(new Uint8Array(args.zipArr)),
    fileName: 'upload.zip',
    mainTex: 'main-jos.tex',
    _e2eReturnBytes: true,  // 关键：要求返回 docx 字节
  });
  return { ok: true, reply };
}, { zipArr });
```

**设计亮点：**

1. **绕过 message channel**：生产代码走 `chrome.runtime.sendMessage`，但 e2e 测试直接调用 `globalThis` 上的函数，避免消息协议复杂性。

2. **`_e2eReturnBytes` 参数**：告诉 service worker 把 docx 字节以 `number[]` 形式返回（因为 SW 无法直接写文件系统）。

### 3.6 下载模块 Smoke Test

```javascript
const downloadResult = await serviceWorker.evaluate(async () => {
  const mod = globalThis.__tex2docDownloads;
  const data = new Uint8Array(5 * 1024);
  // 测试 data URL fallback
  const result = await mod.downloadBytes(data, 'smoke-test.bin', 'mime');
  return result;
});
```

**验证目标：** 确认 `downloadBytes` 在 service worker 内不会抛出 `window is not defined` 错误。

---

## 4. 核心代码路径

### 4.1 WASM 初始化链路

```
initWasm()
  │
  ├── fetch('/wasm/doc_engine_bg.wasm')
  │
  ├── 获取胶水对象 globalThis.__tex2docWbg
  │     (post-build-wasm.mjs 静态打包时挂载)
  │
  ├── 设置 globalThis.__tex2docBaseUrl
  │
  └── await __wbg_init(wasmResponse)
        │
        └── WebAssembly.instantiate() ←── 这里可能抛出 __wbindgen_object_drop_ref 错误
```

### 4.2 转换链路

```
convertLocal(zipBytes, options)
  │
  ├── isWithinSizeLimit(10MB)
  │
  ├── initWasm() (如果尚未初始化)
  │
  └── convertZipToDocxBytes(bytes, mainTex, options)
        │
        └── __tex2docApi.convert_zip_to_docx(bytes, path, '{}')
              │
              └── WASM 引擎执行 LaTeX → DOCX 转换
```

### 4.3 下载链路

```
downloadBytes(data, filename, mimeType)
  │
  ├── createBlobUrl(data, mimeType)
  │     │
  │     ├── URL.createObjectURL(blob) ←── popup/sidepanel 上下文
  │     │
  │     └── base64 data: URL ←── service worker fallback
  │
  └── chrome.downloads.download({ url, filename })
```

---

## 5. 输出结果解读

### 5.1 成功示例

```json
{
  "ok": true,
  "reply": {
    "success": true,
    "jobId": "550e8400-e29b-41d4-a716-446655440000",
    "docxBytes": [80, 75, 3, 4, ...],  // PK\x03\x04 ZIP magic bytes
    "docxFilename": "upload.docx"
  }
}
```

**验证通过标识：**

```
[e2e-wasm] ✓ 转换成功
   jobId: 550e8400-e29b-41d4-a716-446655440000
   docx:  target/e2e-out.docx (123456 bytes)
   ✓ docx magic bytes OK (PK\x03\x04)
[e2e-wasm] ✓ downloadBytes fallback 到 data: URL 工作
```

### 5.2 失败场景

| 错误类型 | 错误信息 | 原因分析 |
|---------|---------|---------|
| WASM 初始化失败 | `__wbindgen_object_drop_ref is not a function` | 胶水与 WASM 版本不匹配 |
| ESM 导入失败 | `import() is not allowed in service worker` | 胶水未静态打包 |
| 文件太大 | `File too large. Maximum size is 10.0 MB.` | ZIP 超过限制 |
| 转换失败 | `Conversion produced invalid DOCX file` | 引擎处理出错 |

---

## 6. 与其他测试的关系

```
┌─────────────────────────────────────────────────────────────┐
│                      完整测试套件                            │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  verify:e2e (package.json)                                  │
│  ├── build:paper3-zip  → 生成测试用的 upload.zip            │
│  ├── e2e:paper3       → 验证 server-side 转换               │
│  ├── e2e:server       → 验证 API 服务器                     │
│  ├── e2e:desktop      → 验证 Flutter 桌面应用               │
│  └── e2e:extension    → 验证浏览器扩展 UI                   │
│                                                             │
│  e2e_wasm_convert.mjs (独立运行)                            │
│  └── 专门验证 WASM 本地转换（不依赖服务器）                  │
│      ├── 服务端故障时可定位问题                              │
│      └── CI 快速回归测试                                     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 7. 运行前提条件

### 7.1 必须先构建扩展

```bash
# 方式一：完整构建（推荐）
npm run extension:build:chrome

# 方式二：仅构建 WASM + 扩展
npm run build:wasm:extension
cd apps/browser-extension && npm run build
```

### 7.2 必须有测试 ZIP 文件

准备一个 paper3 格式的 ZIP 文件（包含 `main.tex` 等文件），放置到默认路径或作为参数传入。

### 7.3 Chromium 路径（Windows）

脚本使用 Playwright 的 `channel: 'chromium'`，需要系统已安装 Chrome/Chromium。

---

## 8. 扩展钩子机制说明

### 8.1 为什么要暴露 globalThis？

MV3 Service Worker 无法使用 `import()` 动态导入，测试脚本也无法通过 message channel 传递二进制数据（`ArrayBuffer`/`Uint8Array` 会被结构化克隆算法拒绝）。

**解决方案：** 在 background.ts 启动时暴露全局函数：

```typescript
// @/entrypoints/background.ts
(globalThis as unknown as { __tex2docConvertZip?: unknown }).__tex2docConvertZip =
  handleStartWasmConversion;
(globalThis as unknown as { __tex2docDownloads?: unknown }).__tex2docDownloads = {
  downloadBytes,
};
```

### 8.2 钩子函数签名

```typescript
// 转换入口
interface Tex2DocConvertZip {
  (payload: {
    zipBytes: number[];      // 可序列化的字节数组
    fileName: string;
    mainTex: string;
    _e2eReturnBytes?: boolean; // e2e 模式：返回字节而非下载
  }): Promise<{
    success: boolean;
    jobId?: string;
    docxBytes?: number[];      // _e2eReturnBytes=true 时返回
    error?: string;
  }>;
}

// 下载模块
interface Tex2DocDownloads {
  downloadBytes(
    data: Uint8Array,
    filename: string,
    mimeType?: string
  ): Promise<{ id: number; filename?: string }>;
}
```

---

## 9. 故障排查

### 9.1 Service Worker 未启动

```
[e2e-wasm] 等待 service worker 就绪（最多 60s）...
[e2e-wasm] ✗ 测试运行失败: Timeout waiting for serviceworker event
```

**排查步骤：**
1. 确认扩展已正确构建到 `apps/browser-extension/.output/chrome-mv3/`
2. 检查 `manifest.json` 的 `background.service_worker` 配置
3. 确认 `wxt.config.ts` 设置了 `type: 'module'`

### 9.2 WASM 导入错误

```
Error: Import #0 "./doc_engine_bg.js" "__wbindgen_object_drop_ref":
function import requires a callable
```

**原因：** `scripts/post-build-wasm.mjs` 未正确执行，或胶水文件与 WASM 版本不匹配。

**排查步骤：**
1. 确认 `npm run build:wasm:extension` 成功执行
2. 检查 `apps/browser-extension/public/wasm/doc_engine.js` 是否存在
3. 确认胶水中的 `__wbindgen_*` 函数与 `.wasm` 文件中的 import 匹配

### 9.3 DOCX Magic Bytes 验证失败

```
   ✗ docx magic bytes wrong
```

**原因：** WASM 转换返回的不是有效的 DOCX 文件。

**排查步骤：**
1. 检查输入 ZIP 是否包含有效的 LaTeX 源文件
2. 检查 `mainTex` 参数是否正确指向主文件
3. 查看 service worker console 日志中的警告信息

---

## 10. 相关文件索引

| 文件 | 说明 |
|------|------|
| `scripts/e2e_wasm_convert.mjs` | 本文档所述测试脚本 |
| `apps/browser-extension/src/entrypoints/background.ts` | Service Worker 入口，暴露全局钩子 |
| `apps/browser-extension/src/conversion/local-wasm.ts` | 本地转换业务逻辑 |
| `apps/browser-extension/src/workers/wasm-worker.ts` | WASM 引擎封装 |
| `apps/browser-extension/src/browser/downloads.ts` | 下载工具（含 data URL fallback） |
| `scripts/post-build-wasm.mjs` | WASM 胶水代码转写脚本 |
| `apps/browser-extension/wxt.config.ts` | WXT 构建配置 |
