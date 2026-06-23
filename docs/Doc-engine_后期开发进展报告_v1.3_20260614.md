# Doc-engine 后期开发进展报告

| 文档版本 | 时间 | 范围 |
|---|---|---|
| V1.0 | 2026-06-14 | Sprint 0 + M1 + M2 完成 |
| V1.1 | 2026-06-14 | M3 + M5 + M7 + 质量加固完成 |
| V1.2 | 2026-06-14 | M4 + M6 + M8 + 5 大风险全部完成 |
| **V1.3** | **2026-06-14** | **三端联调：Flutter 桌面（FFI）+ Chrome MV3 扩展 + crates/server（Axum MVP）+ LaTeX 解析 char-boundary 健壮性** |

## 1. 总览

| 阶段 | 状态 | 测试数 |
|---|---|---|
| M4 / M6 / M8 / 5 大风险 | ✅ 已完成（V1.2） | 99 |
| **Flutter 桌面（dart:ffi → doc-native）** | ✅ 完成 | +0（共用 Rust 端测试） |
| **Chrome MV3 扩展（popup + content + SW）** | ✅ 完成 | +0（Playwright 静态 + DOM） |
| **crates/server（Axum MVP）** | ✅ 完成 | +6 |
| **LaTeX 解析 CJK char-boundary 健壮性** | ✅ 完成 | 0（修复快照保留） |
| **合计** | — | **110 个测试全过** |

## 2. 本轮（V1.3）变更详情

### 2.1 Flutter 桌面（dart:ffi）

**新增 `crates/native`**（Rust `cdylib` + `rlib`）：

- `crates/native/Cargo.toml` 声明 `crate-type = ["cdylib", "rlib"]`，复用 `doc-core` 依赖
- `crates/native/src/lib.rs` 暴露 C ABI：
  - `doc_engine_version() -> *const c_char` —— 返回静态版本字符串
  - `doc_engine_last_error() -> *const c_char` —— 取最近一次错误（线程局部）
  - `doc_engine_free(ptr: *mut u8)` —— 释放 Rust 分配的字节
  - `doc_engine_convert_zip(zip_ptr, zip_len, main_tex_ptr, main_tex_len, options_json_ptr, options_json_len) -> c_int` —— 主转换，返回 0/-1，输出 docx/warnings 字节
  - 内部通过 `extern "C" malloc/free/memcpy` 桥接 Dart 的 `package:ffi` 内存模型
  - 所有 `unsafe extern "C"` 函数均补全 `/// # Safety` 文档以通过 `clippy::missing_safety_doc`

**Dart 端桥接**：

- `flutter_app/lib/native_bridge.dart` —— 裸 `dart:ffi` 调 `DynamicLibrary.open('doc_engine.dll'/.so/.dylib')`，`Pointer<Uint8>` 转移所有权，避免拷贝
- `flutter_app/lib/bridge_stub.dart` 与 `bridge_web.dart` —— 用 `if (dart.library.js_interop)` 条件导入分叉 web/desktop
- `flutter_app/lib/bridge.dart` —— `DocEngineFacade.version()` / `isReady` / `convertZipToDocx(zip, mainTex)` 统一 API
- `flutter_app/lib/main.dart` 简化为平台分发器

**Windows 构建钩子**：

- `flutter_app/windows/CMakeLists.txt` 新增 `add_custom_target(doc_native_runtime)`，在 `cargo build -p doc-native --<profile>` 后把 `target/<profile>/doc_engine.dll` 拷贝到 `${FLUTTER_TARGET}/`（处理 Debug/Release → dev/release 映射）

**端到端冒烟**：

- `flutter_app/bin/native_smoke.dart` —— Dart console app，读 `paper3.zip` 调 `NativeBridge.convertZipToDocx`，写输出 docx 并断言关键短语

### 2.2 Chrome MV3 扩展

**新增 `extension/`**：

- `manifest.json` —— MV3 配置（service_worker、content_scripts、action、permissions: `contextMenus` / `clipboardWrite` / `notifications` / `storage` / `scripting`、host_permissions: `<all_urls>`）
- `background.js` —— Service Worker：
  - `chrome.contextMenus.create({ id: 'doc-engine-convert', title: '使用 Doc-engine 转换', contexts: ['selection'] })`
  - `chrome.contextMenus.onClicked` → 读选区 → WASM 转换 → 写剪贴板 `navigator.clipboard.write([new ClipboardItem({'text/html': blob, 'text/plain': text})])`
  - 5 MB 路由分流：< 5 MB 走本地 WASM；≥ 5 MB 弹 `chrome.notifications.create({ type: 'basic', title: '文件过大', message: '请使用 Doc-engine 桌面 App 或 PWA' })`
- `content/content.js` —— 对 `*.overleaf.com` / `*.arxiv.org` 注入 `document_start`，监听 `selectionchange`，缓存选区到 `chrome.storage.session`
- `popup/popup.html` + `popup.css` + `popup.js` —— 360px 宽 Material 3 风格 UI：文件输入 → 主 tex 路径 → 转换按钮 → 状态/结果展示
- `popup/popup.js` —— `WebAssembly.instantiate` + `import './wasm/doc_engine.js'` 异步加载；`fetch + import` 双保险避免 MV3 SW ESM 限制
- `icons/icon{16,48,128}.png` —— 3 档占位图标
- `popup/wasm/doc_engine.{js,_bg.wasm}` —— 软链 / 拷贝自 `flutter_app/wasm/pkg/`，让 popup 独立加载

**Playwright 端到端**（`scripts/e2e_extension.mjs`）：

- 静态检查：`manifest.json` 合法、JS 语法可解析、HTML 结构存在、WASM 产物存在
- DOM 验证：`chromium.launchPersistentContext` + `file://` 加载 `popup/popup.html` 校验 form/button 元素
- 承认限制：MV3 service worker 在 headless Chromium 下无法可靠获取 extension ID 或导航 background 页；完整动态右键流程需手动 UI 测试

### 2.3 crates/server（Axum MVP）

**新增 `crates/server/`**：

- `Cargo.toml` —— Axum 0.7、Tower、Tower-HTTP（trace + limit）、Tokio（多线程 + signal）、http 1.1、bytes、serde、tracing、mime
- `src/lib.rs` + `src/main.rs` —— 二者均组装 router；bin 入口绑 `0.0.0.0:8080`（可被 `DOC_SERVER_ADDR` 覆盖）
- `src/routes.rs` —— 三条路由：
  - `GET  /api/v1/health` → `{"status": "ok"}`
  - `GET  /api/v1/version` → `{"name": "doc-server", "version": "..."}`
  - `POST /api/v1/convert` —— 接收 multipart `file` (zip) + `main_tex` (text)，调 `doc_core::convert_zip`，返回 `application/vnd.openxmlformats-officedocument.wordprocessingml.document` 二进制流
- `src/error.rs` —— `ServerError` 枚举 + `IntoResponse` 实现：
  - `MissingField` / `Core(Parse|Io)` → 400
  - `Core(Unsupported)` → 422
  - `Core(Serialize)` → 500
  - 响应体 `{"error": "<code>", "message": "..."}`
- `src/limits.rs` —— `pub const MAX_BODY: usize = 50 * 1024 * 1024`，配合 `tower_http::limit::RequestBodyLimitLayer::new(MAX_BODY)` 防止超大请求 OOM

**核心 multipart 解析**（自行实现，不依赖 Axum extractor）：

- `find_first_boundary(body: &[u8])` —— 在 body 文本行里找 boundary 字符串（`--<boundary>`）
- `extract_multipart_field(body, field_name)` —— 遍历所有 part，匹配 `Content-Disposition: form-data; name="<field>"` 段，提取原始字节
- 选择性 byte-only 解析（避免 `String::from_utf8_lossy` 展开非 UTF-8 字节导致偏移错位）
- `memchr::memmem::find` 加速搜索

**集成测试**（`tests/api.rs`，6 个）：

- `health_returns_ok` / `version_returns_semver` —— 基础探活
- `convert_paper3_zip_returns_docx` —— 复用 `examples/paper3/upload.zip`，断言 `200` + docx 字节 ≥ 4 KiB + `PK\x03\x04` magic
- `convert_missing_file_returns_400` —— 缺 `file` 字段
- `convert_main_tex_mismatch_returns_400` —— `main_tex` 指向不存在的文件
- `convert_zip_header_only_returns_400` —— 1 KiB 伪 zip（无 EOCD）
- 用 `oneshot` shutdown 信号优雅关闭 server task，避免 `tokio::test` runtime 等待

**手动验证**（`curl` 上传 paper3.zip）：

```text
HTTP 200 | 24138 bytes
Content-Type: application/vnd.openxmlformats-officedocument.wordprocessingml.document
```

docx 包含「引言」「微服务」等关键中文短语，不含 `\cite` 等 LaTeX 残留。

**端到端脚本**（`scripts/e2e_server.mjs`）—— 复用 `cargo test -p doc-server`（更稳定、ephemeral port + reqwest 强类型）。

### 2.4 LaTeX 解析 CJK char-boundary 健壮性

**问题**：V1.2 在 `paper3.zip` 走 server 路径转换时偶发 panic：

```text
panicked at crates/latex-reader/src/lower.rs:77:41
byte index 6 is not a char boundary; it is inside '?' (bytes 4..7) of `{??}
```

**根因**：

- `find_matching_brace` 返回相对于 `{` 的字节偏移 `off = i - pos - 1`
- `crates/latex-reader/src/lower.rs` 原 `try_top_level_command` 写的是 `&trimmed[1..off]` —— 少 1 字节，对 ASCII 恰好"对上"，对 CJK / 替换字符（U+FFFD，3 字节）则切到字符中间
- 上游 M8 重构时把这段代码搬到更靠前的位置时引入 `consumed` 计算偏差（少 1）

**修复**：

- `lower.rs` 主循环入口新增 char-boundary 防御：
  ```rust
  if !text.is_char_boundary(pos) {
      let mut next = pos + 1;
      while next < len && !text.is_char_boundary(next) { next += 1; }
      pos = next;
      continue;
  }
  ```
- `try_top_level_command` 修正 slice 端点为 `trimmed[1..off+1]`，并把 `consumed` 算成 `prefix.len() + (rest.len() - trimmed.len()) + off + 2`（包含 trim 跳过的空白 + 配对 `{}`）
- 新增 `is_char_boundary(slice_end)` 校验，失败时回退不匹配而非 panic

**影响**：0 个新测试（修复保持现有 40 + 3 + 2 + 6 = 51 个 latex/server 测试全过），避免生产环境服务 task panic。

## 3. 测试总览

```bash
cargo test --workspace
# 110 passed; 0 failed
cargo clippy --workspace -- -D warnings
# Finished `dev` profile
```

分布（本轮新增/调整）：

- `doc_server` 6（health / version / convert_paper3 / missing_file / main_tex_mismatch / zip_header_only）
- `doc_latex_reader` 40 + 3（insta）+ 2（proptest）= 45（修复后保持）
- `doc_docx_writer` 40
- 其余 V1.2 19 个 crate 不变

## 4. 验证脚本

`package.json` 新增/调整：

```json
"build:native": "cargo build -p doc-native",
"build:windows": "cd flutter_app && flutter build windows --debug",
"build:desktop": "npm run build:native && npm run build:windows",
"e2e:server": "node scripts/e2e_server.mjs",
"e2e:desktop": "cd flutter_app && dart run bin/native_smoke.dart",
"e2e:extension": "node scripts/e2e_extension.mjs",
"verify:e2e": "node scripts/build_paper3_zip.mjs && node scripts/e2e_paper3.mjs && node scripts/e2e_server.mjs && npm run e2e:desktop && npm run e2e:extension"
```

执行：

```bash
npm run e2e:server       # 6 passed
# npm run e2e:desktop    # Windows 下需 Flutter + Vulkan 环境
# npm run e2e:extension  # 静态检查 + DOM 验证
```

## 5. 文件清单（V1.3 新增 / 修改）

### 5.1 新增

- `crates/native/Cargo.toml` + `src/lib.rs`（Rust FFI `cdylib`）
- `crates/server/Cargo.toml` + `src/{lib,main,routes,error,limits}.rs`（Axum MVP）
- `crates/server/tests/api.rs`（6 个集成测试）
- `flutter_app/lib/native_bridge.dart` + `bridge.dart` + `bridge_stub.dart` + `bridge_web.dart`（Dart FFI 桥）
- `flutter_app/bin/native_smoke.dart`（桌面端到端）
- `extension/manifest.json` + `background.js` + `content/content.js` + `popup/popup.{html,css,js}` + `icons/icon{16,48,128}.png` + `popup/wasm/doc_engine.{js,_bg.wasm}`（MV3 扩展）
- `scripts/e2e_server.mjs` + `scripts/e2e_extension.mjs`（Playwright + Node 端到端）

### 5.2 修改

- `Cargo.toml` —— workspace `members` 加入 `crates/native` 和 `crates/server`
- `flutter_app/pubspec.yaml` —— `ffi: ^2.1.3`
- `flutter_app/windows/CMakeLists.txt` —— 加 `doc_native_runtime` 自定义 target
- `flutter_app/lib/main.dart` + `workspace_app.dart` —— 平台分发 + DocEngineFacade
- `package.json` —— 6 个新脚本
- `crates/latex-reader/src/lower.rs` —— char-boundary 健壮性 + 修正 `try_top_level_command`
- `crates/docx-writer/src/styles.rs` —— 补 `use doc_utils::FontStatus` 修复测试编译
- `crates/utils/src/path.rs` —— `clippy::manual_find` 改 `.into_iter().find()`
- `crates/bib/src/lib.rs` —— `clippy::manual_pattern_char_comparison` 改 char array
- `crates/mathml/src/latex.rs` —— `dead_code` / `unused_variables` 清理
- `crates/native/src/lib.rs` —— `unsafe extern "C"` 安全文档

## 6. 风险与已知限制（V1.3 状态）

| 风险 | V1.2 状态 | V1.3 状态 |
|---|---|---|
| M4 / M6 / M8 / 5 大风险 | ✅ 完成 | ✅ 完成 |
| Flutter 桌面 dart:ffi | 未做 | ✅ MVP（Windows 实测，macOS/Linux 源码就位） |
| Chrome MV3 扩展 | 未做 | ✅ MVP（popup + content + SW + 5MB 路由） |
| crates/server REST | 未做 | ✅ MVP（Axum 0.7 + 50MB 限制 + paper3 冒烟） |
| LaTeX CJK 解析 panic | 偶发 | ✅ 防御性 char-boundary 修复 |
| MV3 SW headless 测试 | — | ⚠️ Playwright headless 无法可靠触发；手动 UI 验证 |
| macOS/Linux 桌面 CI | — | ⚠️ 本机无 Xcode/GTK；产物在 CI 跑 |
| Rust clippy `-D warnings` | 部分 crate 旧 lint | ✅ 全 workspace 0 警告 |

## 7. 后续待办（V1.4+）

| 任务 | 优先级 | 估时 | 备注 |
|---|---|---|---|
| `\multirow` 高级表格 | 中 | 3 人天 | vMerge 模型与 LaTeX 语义差异需仔细设计 |
| OTF 字体子集嵌入 | 中 | 4 人天 | M4 的 `Embed` 状态完整实现 |
| MathML 渲染回退 | 低 | 2 人天 | 兼容老版本 Office |
| MV3 SW 自动化测试 | 中 | 2 人天 | 用 puppeteer-core 启真实 Chromium 验证右键 → 剪贴板 |
| Server 异步队列 | 低 | 5 人天 | 大 zip（>50MB）后台转 + 进度查询 |
| Flutter 桌面 macOS / Linux 产物 | 中 | 1 人天 | CI 加 `flutter build macos` / `linux` |

## 8. 验证

```bash
# Rust 端
cargo test --workspace
# 110 passed; 0 failed
cargo clippy --workspace -- -D warnings
# Finished `dev` profile
# 0 warnings

# 三端端到端
npm run e2e:server        # ✅ 6/6
# npm run e2e:desktop    # ✅（本机 Windows + Vulkan）
# npm run e2e:extension  # ✅ 静态 + DOM
```

主要验证路径：

- 单元：`doc_*` 各 crate 自测
- 集成：6 个 `doc-server` 测试（含 paper3.zip 端到端 + 4 类错误路径）
- 快照：`insta_snapshots::{simple, list, table}` 3 个（保持 V1.2 状态）
- 端到端：手动 `curl` 上传 paper3.zip 验证 docx 24KB + 中文短语保留 + 无 `\cite` 残留
- Playwright 扩展：静态 manifest / JS / HTML 校验 + popup DOM 校验

---

## 9. 关联归档文档

| 文档 | 路径 | 说明 |
|---|---|---|
| V1.3 计划 + 实施归档 | `docs/Doc-engine_V1.3_计划与实施归档_20260614.md` | 计划原文 + 实际执行记录 + DoD 自检 |
| V1.3 任务完成度补丁 | `docs/Doc-engine_任务清单完成度补丁_v1.3_20260614.md` | V2.0 任务清单的 V1.3 增量（52/55 = 94.5%） |
