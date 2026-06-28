# Tex2Doc 浏览器插件测试验证说明

> 文档版本：v1.0  
> 更新日期：2026-06-28

---

## 1. 测试套件概览

### 1.1 测试金字塔

```
┌─────────────────────────────────────────────────────────────────────┐
│                          测试金字塔                                  │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│                           ▲                                         │
│                          ╱ ╲                                        │
│                         ╱   ╲                                       │
│                        ╱ E2E ╲         ┌─────────────────────────┐ │
│                       ╱────────╲        │ npm run verify:e2e      │ │
│                      ╱          ╲       │ (完整集成测试)           │ │
│                     ╱  集成测试  ╲      └─────────────────────────┘ │
│                    ╱──────────────╲     ┌─────────────────────────┐ │
│                   ╱                ╲    │ e2e_extension.mjs       │ │
│                  ╱   UI 测试        ╲   │ (MV3 静态+动态冒烟)     │ │
│                 ╱──────────────────╲   └─────────────────────────┘ │
│                ╱                      ╲  ┌─────────────────────────┐ │
│               ╱     单元测试           ╲ │ vitest (project-zip)   │ │
│              ╱────────────────────────╲│ (ZIP 验证)              │ │
│             ╱                            │                        │ │
│            ╱     构建验证                 ╲└─────────────────────────┘ │
│           ╱────────────────────────────────                            │
│          ╱  manifest + 语法检查                                        ╲│
│         ╱─────────────────────────────────────────────────────────────╲│
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.2 测试命令速查表

| 层级 | 命令 | 脚本文件 | 执行时间 | 用途 |
|-----|------|---------|---------|------|
| **构建验证** | `npm run build:chrome` | WXT | ~2s | 验证扩展构建成功 |
| **单元测试** | `cd apps/browser-extension && npm test` | vitest | ~5s | ZIP 验证逻辑 |
| **UI 冒烟** | `npm run e2e:extension` | e2e_extension.mjs | ~10s | MV3 静态+动态验证 |
| **WASM 验证** | `node scripts/e2e_wasm_convert.mjs` | e2e_wasm_convert.mjs | ~30s | 本地转换链路 |
| **完整 E2E** | `npm run verify:e2e` | 组合 | ~5min | 全链路集成测试 |

---

## 2. 单元测试

### 2.1 执行命令

```bash
cd apps/browser-extension
npm test
```

或直接运行 vitest：

```bash
cd apps/browser-extension
npx vitest run
```

### 2.2 测试范围

| 测试文件 | 测试内容 | 覆盖模块 |
|---------|---------|---------|
| `tests/unit/project-zip.test.ts` | ZIP 文件验证 | `@/conversion/project-zip` |

**project-zip.test.ts 测试用例：**

```typescript
// 1. 验证有效 ZIP 文件
it('should validate a valid ZIP file', async () => {
  const zipBytes = new Uint8Array([0x50, 0x4b, 0x03, 0x04]);
  const file = new File([blob], 'test.zip');
  const result = await validateZipFile(file);
  expect(result.valid).toBe(true);
});

// 2. 拒绝非 ZIP 文件
it('should reject non-ZIP files', async () => {
  const textBytes = new Uint8Array([0x74, 0x65, 0x73, 0x74]); // "test"
  const file = new File([blob], 'test.txt');
  const result = await validateZipFile(file);
  expect(result.valid).toBe(false);
});
```

### 2.3 Vitest 配置

```typescript
// vitest.config.ts
export default defineConfig({
  test: {
    globals: true,
    environment: 'jsdom',        // 模拟浏览器 DOM
    setupFiles: ['./tests/setup.ts'],
  },
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
      // ... 其他路径别名
    },
  },
});
```

### 2.4 预期输出

```
✓ tests/unit/project-zip.test.ts (2 tests, 2 passed)
```

---

## 3. UI 冒烟测试 (e2e_extension.mjs)

### 3.1 执行命令

```bash
npm run e2e:extension
```

### 3.2 测试架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                    e2e_extension.mjs 三层验证                       │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐              │
│  │  静态验证    │   │  动态验证    │   │  DOM 验证    │              │
│  ├─────────────┤   ├─────────────┤   ├─────────────┤              │
│  │ manifest.json│   │ Playwright  │   │ popup.html  │              │
│  │ background.js│   │ Chromium    │   │ 独立加载     │              │
│  │ popup.js    │   │ 浏览器测试   │   │ 元素检查     │              │
│  │ popup.html  │   │ chrome API  │   │             │              │
│  │ WASM 文件   │   │             │   │             │              │
│  └─────────────┘   └─────────────┘   └─────────────┘              │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.3 静态验证层

**验证项目：**

| 检查项 | 说明 | 失败原因 |
|-------|------|---------|
| `manifest.json` 有效 JSON | 解析 manifest 文件 | JSON 语法错误 |
| `manifest_version: 3` | 确认 MV3 | 配置文件错误 |
| `background.service_worker` | SW 入口存在 | 构建配置缺失 |
| `action.default_popup` | popup 入口存在 | WXT 配置错误 |
| `background.js` 语法 | `new Function()` 验证 | JS 语法错误 |
| `popup.js` 语法 | `new Function()` 验证 | JS 语法错误 |
| `popup.html` 关键元素 | `id="status-bar"`, `id="convert-btn"` | HTML 结构变更 |
| `WASM bundle` 大小 | > 100KB | WASM 未正确打包 |

**示例输出：**

```
[e2e-extension] === Static checks ===
[e2e-extension] manifest.json: valid JSON, MV3
[e2e-extension] manifest.background.service_worker: "background.js"
[e2e-extension] manifest.action.default_popup: "popup.html"
[e2e-extension] background.js: valid JS syntax
[e2e-extension] popup.js: valid JS syntax
[e2e-extension] popup.html: structure OK
[e2e-extension] WASM bundle: 1228 KB
```

### 3.4 动态验证层

**验证流程：**

1. 启动 Chromium (`headless: true`)
2. 加载扩展为 unpacked extension
3. 访问 `https://example.com`
4. 验证 `chrome` 全局对象存在

```javascript
const chromeExists = await page.evaluate(() => typeof chrome !== 'undefined');
console.log(`[e2e-extension] chrome global in page: ${chromeExists}`);
```

### 3.5 DOM 验证层

**检查的 DOM 元素：**

| 元素 ID | 说明 |
|--------|------|
| `header h1` | 标题 |
| `version-badge` | 版本号显示 |
| `status-bar` | 状态栏 |
| `status-text` | 状态文本 |
| `zip-input` | ZIP 文件选择 |
| `pick-label` | 文件选择标签 |
| `main-tex-input` | 主文件输入 |
| `convert-btn` | 转换按钮 |
| `result-section` | 结果区域 |
| `error-section` | 错误区域 |
| `download-btn` | 下载按钮 |

**示例输出：**

```
[e2e-extension] === Popup DOM check ===
[e2e-extension] popup DOM: ALL PRESENT ✓
[e2e-extension] h1 text: "Tex2Doc"
[e2e-extension] status: "Ready"
[e2e-extension] error hidden: true
[e2e-extension] result hidden: true
```

### 3.6 完整输出示例

```
[e2e-extension] starting
[e2e-extension] === Static checks ===
[e2e-extension] manifest.json: valid JSON, MV3
[e2e-extension] manifest.background.service_worker: "background.js"
[e2e-extension] manifest.action.default_popup: "popup.html"
[e2e-extension] background.js: valid JS syntax
[e2e-extension] popup.js: valid JS syntax
[e2e-extension] popup.html: structure OK
[e2e-extension] WASM bundle: 1228 KB
[e2e-extension] === Dynamic checks ===
[e2e-extension] background pages: 1
[e2e-extension] chrome global in page: true
[e2e-extension] === Popup DOM check ===
[e2e-extension] popup DOM: ALL PRESENT ✓
[e2e-extension] h1 text: "Tex2Doc"
[e2e-extension] status: "Ready"
[e2e-extension] error hidden: true
[e2e-extension] result hidden: true
[e2e-extension] exit code: 0
```

---

## 4. WASM 转换测试 (e2e_wasm_convert.mjs)

### 4.1 执行命令

```bash
# 默认参数
node scripts/e2e_wasm_convert.mjs

# 指定输入输出
node scripts/e2e_wasm_convert.mjs "D:\papers\upload.zip" "D:\temp\output.docx"
```

### 4.2 前提条件

1. **扩展已构建：**
   ```bash
   npm run extension:build:chrome
   ```

2. **测试 ZIP 文件存在：**
   - 默认路径：`D:\temp\upload.zip`
   - 必须包含有效的 LaTeX 源文件
   - 主文件命名为 `main-jos.tex`（或通过参数指定）

### 4.3 验证链路

```
┌─────────────────────────────────────────────────────────────────────┐
│                      WASM 转换验证链路                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Playwright (Node.js)                                              │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │ 1. 读取 upload.zip → number[]                               │  │
│  │ 2. 启动 Chromium + 加载扩展                                │  │
│  │ 3. 等待 Service Worker 就绪                                │  │
│  │ 4. 调用 globalThis.__tex2docConvertZip()                   │  │
│  │ 5. 接收 docxBytes (number[]) 返回                          │  │
│  │ 6. 写文件 + 验证 magic bytes                               │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                              ↓                                     │
│  Service Worker (background.js)                                    │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │ handleStartWasmConversion(payload)                         │  │
│  │   ├── convertLocal(bytes, options)                         │  │
│  │   │   ├── initWasm()                                       │  │
│  │   │   │   └── WebAssembly.instantiate()                   │  │
│  │   │   └── convertZipToDocxBytes(bytes, mainTex, opts)     │  │
│  │   │       └── __tex2docApi.convert_zip_to_docx()          │  │
│  │   └── downloadBytes(docxBytes, filename)                  │  │
│  │       └── data: URL fallback                              │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                              ↓                                     │
│  WebAssembly Engine (doc_engine.wasm)                              │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │ LaTeX → DOCX 转换引擎                                       │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 4.4 验证项目

| 验证点 | 说明 | 失败症状 |
|-------|------|---------|
| Service Worker 就绪 | 等待 SW 启动 | `Timeout waiting for serviceworker` |
| WASM 初始化 | `__wbindgen_*` 导入 | `__wbindgen_object_drop_ref is not a function` |
| 转换成功 | `convert_zip_to_docx` 返回 | `Conversion failed` |
| DOCX 有效 | magic bytes = `PK\x03\x04` | `docx magic bytes wrong` |
| 下载模块 | `downloadBytes` 不抛 `window` 错误 | `window is not defined` |

### 4.5 输出示例

**成功：**
```
[e2e-wasm] extension dir: E:\work\Tex2Doc\apps\browser-extension\.output\chrome-mv3
[e2e-wasm] input zip:     D:\temp\upload.zip
[e2e-wasm] output docx:   E:\work\Tex2Doc\target\e2e-out.docx
[e2e-wasm] zip size: 2.35 MiB
[e2e-wasm] context 创建成功
[e2e-wasm] 当前 service workers: 1
[e2e-wasm] ✓ service worker: chrome-extension://...
[e2e-wasm] 触发 WASM 初始化 + 转换...
[e2e-wasm] ✓ 转换成功
   jobId: 550e8400-e29b-41d4-a716-446655440000
   docx:  E:\work\Tex2Doc\target\e2e-out.docx (123456 bytes)
   ✓ docx magic bytes OK (PK\x03\x04)
[e2e-wasm] downloadBytes smoke test:
{"ok":true,"chromeDownloadsError":"Download cannot be performed in headless"}
[e2e-wasm] ✓ downloadBytes fallback 到 data: URL 工作
[e2e-wasm] exit code: 0
```

**失败（WASM 导入错误）：**
```
[e2e-wasm] ✗ WASM 转换失败
   __wbindgen_object_drop_ref is not a function
```

---

## 5. 完整 E2E 测试 (verify:e2e)

### 5.1 执行命令

```bash
npm run verify:e2e
```

### 5.2 测试流程

```
┌─────────────────────────────────────────────────────────────────────┐
│                     npm run verify:e2e 执行流程                      │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  1. build:paper3-zip                                               │
│     └── 生成测试用 upload.zip                                        │
│                                                                     │
│  2. e2e:paper3                                                     │
│     └── 验证 server-side 转换（完整论文流程）                         │
│                                                                     │
│  3. e2e:server                                                      │
│     └── 验证 API 服务器端点                                          │
│                                                                     │
│  4. e2e:desktop                                                     │
│     └── 验证 Flutter 桌面应用                                       │
│                                                                     │
│  5. e2e:extension                                                    │
│     └── 验证浏览器扩展 UI                                            │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 5.3 各阶段说明

| 阶段 | 脚本 | 主要验证 |
|-----|------|---------|
| 1 | `build_paper3_zip.mjs` | 生成测试用 ZIP 文件 |
| 2 | `e2e_paper3.mjs` | 服务器端 LaTeX→DOCX 转换 |
| 3 | `e2e_server.mjs` | REST API 可用性 |
| 4 | `native_smoke.dart` | Flutter 桌面应用 |
| 5 | `e2e_extension.mjs` | 浏览器扩展 UI |

---

## 6. 开发工作流

### 6.1 TDD 开发循环

```bash
# 1. 写测试
cd apps/browser-extension
npx vitest write tests/unit/my-feature.test.ts

# 2. 运行测试（watch 模式）
npm test -- --watch

# 3. 开发功能
# ... 修改代码 ...

# 4. 验证
npm test
```

### 6.2 提交前验证

```bash
# 1. 单元测试
cd apps/browser-extension && npm test

# 2. 构建扩展
npm run extension:build:chrome

# 3. UI 冒烟测试
npm run e2e:extension

# 4. WASM 转换测试（如果有测试 ZIP）
node scripts/e2e_wasm_convert.mjs
```

### 6.3 CI/CD 流水线

```yaml
# .github/workflows/extension-test.yml
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
      - run: npm ci
      - run: cd apps/browser-extension && npm test
      - run: npm run extension:build:chrome
      - run: npm run e2e:extension
      - run: node scripts/e2e_wasm_convert.mjs
```

---

## 7. 测试覆盖范围

### 7.1 模块覆盖矩阵

| 模块 | 单元测试 | UI 冒烟 | WASM E2E |
|------|:--------:|:-------:|:--------:|
| `@/conversion/project-zip` | ✓ | - | - |
| `@/conversion/local-wasm` | - | - | ✓ |
| `@/workers/wasm-worker` | - | - | ✓ |
| `@/browser/downloads` | - | - | ✓ |
| `@/browser/messaging` | - | ✓ | - |
| `@/state/job-store` | - | ✓ | - |
| `@/state/session-store` | - | ✓ | - |
| UI Components | - | ✓ | - |

### 7.2 未覆盖区域

以下区域需要手动测试或补充测试：

| 区域 | 说明 | 建议 |
|------|------|------|
| Content Scripts | Overleaf/arXiv 集成 | Playwright 页面测试 |
| Sidepanel | 侧边栏 UI | 手动测试 |
| Chrome.downloads API | 真实下载 | 集成测试 |
| 网络错误处理 | 离线场景 | 补充网络模拟 |

---

## 8. 故障排查

### 8.1 常见问题

| 问题 | 命令 | 解决方案 |
|------|------|---------|
| vitest 找不到模块 | `npm test` | 检查 `vitest.config.ts` 的 alias |
| WASM 文件未找到 | e2e_wasm_convert | 运行 `npm run extension:build:chrome` |
| Service Worker 超时 | e2e_wasm_convert | 检查 manifest.json 的 background 配置 |
| DOM 元素缺失 | e2e_extension | 确认 HTML 结构未变更 |
| Playwright 启动失败 | 所有 e2e 脚本 | 安装浏览器：`npx playwright install chromium` |

### 8.2 调试技巧

**Vitest 调试：**
```bash
# 带 UI 的调试模式
cd apps/browser-extension
npm test -- --ui

# 指定文件
npx vitest run tests/unit/project-zip.test.ts

# 保留控制台输出
npx vitest run --reporter=verbose
```

**Playwright 调试：**
```bash
# 非 headless 模式运行
# 修改脚本中的 headless: true → headless: false

# 开启 Playwright debug
DEBUG=pw:browser* node scripts/e2e_extension.mjs
```

---

## 9. 相关文档

| 文档 | 位置 | 说明 |
|------|------|------|
| WASM E2E 脚本详解 | `docs-zh/extension/e2e-wasm-convert-script-guide-20260628.md` | e2e_wasm_convert.mjs 深度分析 |
| 本地转换联调方案 | `docs-zh/extension/Tex2Doc-浏览器插件本地转换联调风险与解决方案-20260628.md` | MV3 WASM 技术方案 |
| 自动化研发面板 | `docs-zh/cicd/Tex2Doc自动化研发面板操作手册-v1.0.md` | CI/CD 集成 |

---

## 10. 附录：测试文件索引

```
apps/browser-extension/
├── tests/
│   ├── setup.ts                 # Vitest 全局配置
│   └── unit/
│       └── project-zip.test.ts  # ZIP 验证单元测试
├── vitest.config.ts             # Vitest 配置
└── package.json                # 测试脚本定义

scripts/
├── e2e_extension.mjs            # 扩展 UI 冒烟测试
├── e2e_wasm_convert.mjs         # WASM 转换 E2E 测试
├── e2e_paper3.mjs               # 服务器端转换测试
├── e2e_server.mjs               # API 服务器测试
└── e2e_flutter_commercial_web.mjs  # Web UI 测试

apps/browser-extension/scripts/
└── fix-chrome-manifest.mjs      # Chrome manifest 后处理
```
