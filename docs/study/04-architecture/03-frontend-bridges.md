# 三种前端集成模式

> 本节详细描述 Tex2Doc 的三种集成模式：**WASM（Web/Chrome 扩展）**、**Native FFI（桌面端）**、**HTTP（服务端）**。读完应能独立把 `doc-core` 集成到任何前端。

---

## 1. 模式 A — WASM 集成（Flutter Web + Chrome MV3）

### 1.1 适用场景

* **Flutter Web PWA**：浏览器内运行。
* **Chrome MV3 扩展**：popup 内运行。
* 其它任意 JS / Web 平台。

### 1.2 必备组件

| 组件 | 路径 | 大小 |
|------|------|------|
| `doc_engine.js`（ESM 包装） | `flutter_app/wasm/pkg/doc_engine.js` | ~14 KB |
| `doc_engine_bg.wasm`（WASM 字节） | `flutter_app/wasm/pkg/doc_engine_bg.wasm` | ~3.5 MB |
| `doc_engine.d.ts`（TypeScript） | `flutter_app/wasm/pkg/doc_engine.d.ts` | ~3 KB |

### 1.3 加载流程

```javascript
// 浏览器端
const mod = await import('./wasm/doc_engine.js');
const wasmUrl = chrome.runtime.getURL('popup/wasm/doc_engine_bg.wasm');
const resp = await fetch(wasmUrl);
const wasmBuffer = await resp.arrayBuffer();
await mod.default({ wasmBinary: wasmBuffer });  // 触发 wasm-bindgen init
const docEngine = mod;
const version = docEngine.version();           // "0.1.0"
const docxU8 = docEngine.convert_zip_to_docx(zipBytes, mainTex, '');
```

### 1.4 公开 API（`#[wasm_bindgen]`）

```rust
pub fn convert_zip(
    zip_bytes: &[u8],
    main_tex_path: &str,
    options_js: Option<String>,
) -> Result<JsValue, JsValue>;
// 成功：JsValue = { docx: Vec<u8>, docx_len: number, warnings: string[] }
// 失败：JsValue = string（错误消息）

pub fn convert_zip_to_docx(
    zip_bytes: &[u8],
    main_tex_path: &str,
    options_js: Option<String>,
) -> Result<js_sys::Uint8Array, JsValue>;
// 成功：Uint8Array（docx 字节流）
// 失败：JsValue = string

pub fn version() -> String;
```

### 1.5 options JSON 协议

V1 简化为：

```json
{ "bib_style": "numeric" | "author-year" | "authoryear" }
```

`bib_style` 缺省 = `"numeric"`；未识别值抛 `WasmError`。

### 1.6 Dart 桥接（`flutter_app/lib/wasm_bridge.dart`）

```dart
// 关键：dart:js_interop 1.x 官方 API
import 'dart:js_interop';
import 'dart:js_interop_unsafe';
import 'package:web/web.dart' as web;

class WasmBridge {
  static final WasmBridge instance = WasmBridge._();

  Future<void> ensureReady({Duration timeout = const Duration(seconds: 30)}) async {
    if (_ready) return;
    if (_hasGlobal('docEngine')) { _ready = true; return; }
    if (_hasGlobal('docEngineError')) throw WasmBridgeException('WASM init 失败');
    await _waitForEvent('doc-engine-ready', timeout: timeout);
    _ready = true;
  }

  Future<Uint8List> convertZipToDocx(Uint8List zipBytes, String mainTexPath,
      {String? optionsJson}) async {
    await ensureReady();
    final JSAny jsInput = zipBytes.toJS;  // 零拷贝
    final JSObject ns = globalContext.getProperty<JSObject>('docEngine'.toJS);
    final JSFunction fn = ns.getProperty<JSFunction>('convert_zip_to_docx'.toJS);
    final out = fn.callAsFunction(ns, jsInput, mainTexPath.toJS, (optionsJson ?? '').toJS);
    return (out as JSUint8Array).toDart;  // 零拷贝
  }
}
```

### 1.7 Chrome 扩展 popup（`extension/popup/popup.js`）

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
const docxBytes = new Uint8Array(result);
// 验证 + 下载
const blob = new Blob([docxBytes], { type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document' });
const url = URL.createObjectURL(blob);
const a = document.createElement('a');
a.href = url;
a.download = (zipFileName || 'output').replace(/\.[^.]+$/, '') + '.docx';
a.click();
```

### 1.8 复制 WASM 产物

| 目标位置 | 用途 |
|----------|------|
| `flutter_app/wasm/pkg/` | wasm-pack 直接输出 |
| `flutter_app/web/wasm/` | Flutter Web PWA（`index.html` 引用） |
| `extension/popup/wasm/` | Chrome 扩展 popup |

> 推荐用脚本（`scripts/copy_wasm_to_extension.{ps1,sh}` 之类）做同步。当前手动 cp。

### 1.9 性能特征

* WASM 编译 / 实例化：~200-500 ms（首次加载）。
* 8 KB LaTeX + 6 include：~1 s 转换。
* 内存峰值：与原生 Rust 相近。
* 二进制大小：~3.5 MB（dev build），release 可减小 30%+。

### 1.10 限制

* 单文件 < 5 MB（Chrome 扩展 popup 限制）；超过时引导用户用桌面 App。
* 大工程（>50 MiB zip）受 WASM 内存限制（4 GB 浏览器）。
* 无文件系统访问（必须在 VFS 内）。
* 浏览器单线程（除 Web Worker）。

---

## 2. 模式 B — Native FFI 集成（Flutter Desktop）

### 2.1 适用场景

* Flutter Windows / macOS / Linux 桌面端。
* 其它任意支持 C FFI 的运行时（Go / C# / Java JNI / Swift）。

### 2.2 必备组件

| 组件 | 路径 | 大小 |
|------|------|------|
| `doc_native.dll`（Windows） | `target/debug/doc_native.dll` | ~几 MB |
| `doc_native.dylib`（macOS） | `target/debug/libdoc_native.dylib` | ~几 MB |
| `libdoc_native.so`（Linux） | `target/debug/libdoc_native.so` | ~几 MB |

> Windows CMake 自动从 `target/{debug,release}/doc_native.dll` 拷贝到 `runner/bin/doc_engine.dll`（见 `flutter_app/windows/CMakeLists.txt`）。

### 2.3 FFI 签名

```rust
// crates/native/src/lib.rs
#[no_mangle] pub unsafe extern "C" fn doc_engine_version() -> *const c_char;
#[no_mangle] pub unsafe extern "C" fn doc_engine_last_error() -> *const c_char;
#[no_mangle] pub unsafe extern "C" fn doc_engine_free(ptr: *mut u8);

#[no_mangle] pub unsafe extern "C" fn doc_engine_convert_zip(
    zip_ptr: *const u8, zip_len: usize,
    main_tex_ptr: *const u8, main_tex_len: usize,
    out_docx_ptr: *mut *mut u8, out_docx_len: *mut usize,
    out_warnings_ptr: *mut *mut u8, out_warnings_len: *mut usize,
) -> c_int;  // 0=成功, 1=失败
```

### 2.4 内存契约

| 方向 | 内存 | 由谁分配 | 由谁释放 |
|------|------|----------|----------|
| 入参 zip | C 堆 | Dart（`pkg_ffi.calloc`） | Dart（`pkg_ffi.calloc.free`） |
| 入参 main_tex | C 堆 | Dart | Dart |
| 出参 docx | C 堆 | Rust（`libc::malloc`） | Dart（`doc_engine_free`） |
| 出参 warnings | C 堆 | Rust | Dart（`doc_engine_free`） |
| 出参错误字符串 | thread-local `CString` | Rust | 静态（永不释放） |

### 2.5 调用流程（Dart）

```dart
import 'dart:ffi' as ffi;
import 'package:ffi/ffi.dart' as pkg_ffi;

Future<NativeConvertResult> convertZipToDocx(Uint8List zipBytes, String mainTexPath) async {
  await ensureReady();
  final fn = _convertFn!;

  final zipBuf = pkg_ffi.calloc<ffi.Uint8>(zipBytes.length);
  try {
    final zipPtr = zipBuf.cast<ffi.Uint8>();
    zipPtr.asTypedList(zipBytes.length).setAll(0, zipBytes);

    final mainBytes = utf8.encode(mainTexPath);
    final mainBuf = pkg_ffi.calloc<ffi.Uint8>(mainBytes.length);
    try {
      mainBuf.asTypedList(mainBytes.length).setAll(0, mainBytes);

      final outDocxPtr = pkg_ffi.calloc<ffi.Pointer<ffi.Uint8>>();
      final outDocxLen = pkg_ffi.calloc<ffi.IntPtr>();
      final outWarnPtr = pkg_ffi.calloc<ffi.Pointer<ffi.Uint8>>();
      final outWarnLen = pkg_ffi.calloc<ffi.IntPtr>();
      try {
        final rc = fn(zipPtr, zipBytes.length, mainBuf, mainBytes.length,
            outDocxPtr, outDocxLen, outWarnPtr, outWarnLen);
        if (rc != 0) {
          final errPtr = _lastErrorFn!();
          final err = errPtr == ffi.nullptr ? 'unknown' : _readCString(errPtr);
          throw NativeBridgeException('convert_zip 失败 (rc=$rc): $err');
        }

        final docxPtr = outDocxPtr.value;
        final docxLen = outDocxLen.value;
        final warnPtr = outWarnPtr.value;
        final warnLen = outWarnLen.value;
        try {
          final docx = Uint8List.fromList(docxPtr.asTypedList(docxLen));
          final warnings = warnLen > 0
              ? List<String>.from(jsonDecode(utf8.decode(warnPtr.asTypedList(warnLen))) as List)
              : <String>[];
          return NativeConvertResult(docx: docx, warnings: warnings);
        } finally {
          _freeFn!(docxPtr);
          if (warnLen > 0) _freeFn!(warnPtr);
        }
      } finally {
        pkg_ffi.calloc.free(outDocxPtr);
        pkg_ffi.calloc.free(outDocxLen);
        pkg_ffi.calloc.free(outWarnPtr);
        pkg_ffi.calloc.free(outWarnLen);
      }
    } finally {
      pkg_ffi.calloc.free(mainBuf);
    }
  } finally {
    pkg_ffi.calloc.free(zipBuf);
  }
}
```

### 2.6 平台分发

```dart
String _libName() {
  final override = Platform.environment['DOC_ENGINE_LIB'];
  if (override != null && override.isNotEmpty) return override;
  if (Platform.isWindows) return 'doc_engine';
  if (Platform.isMacOS) return 'doc_engine';
  if (Platform.isLinux) return 'doc_engine';
  throw NativeBridgeException('不支持的平台：${Platform.operatingSystem}');
}
```

> Windows / macOS / Linux 都叫 `doc_engine`（macOS / Linux 自动加 `lib` 前缀与 `.dylib` / `.so` 后缀）。

### 2.7 CMake 自动构建（Windows）

`flutter_app/windows/CMakeLists.txt`：

```cmake
find_program(CARGO_BIN cargo HINTS $ENV{CARGO_HOME} REQUIRED)
set(DOC_NATIVE_CRATE "${CMAKE_CURRENT_SOURCE_DIR}/../../crates/native")

# 选 profile
if(CMAKE_BUILD_TYPE STREQUAL "Release" OR CMAKE_BUILD_TYPE STREQUAL "Profile")
  set(DOC_NATIVE_RUST_PROFILE_FLAG "--release")
  set(DOC_NATIVE_DLL_DIR_SUFFIX "release")
else()
  set(DOC_NATIVE_RUST_PROFILE_FLAG "")
  set(DOC_NATIVE_DLL_DIR_SUFFIX "debug")
endif()
set(DOC_NATIVE_DLL_SRC "${CMAKE_CURRENT_SOURCE_DIR}/../../target/${DOC_NATIVE_DLL_DIR_SUFFIX}/doc_native.dll")

add_custom_target(doc_native_runtime ALL
  COMMAND ${CARGO_BIN} build -p doc-native ${DOC_NATIVE_RUST_PROFILE_FLAG}
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}/../.."
)
add_custom_command(TARGET doc_native_runtime POST_BUILD
  COMMAND ${CMAKE_COMMAND} -E copy_if_different
    "${DOC_NATIVE_DLL_SRC}"
    "$<TARGET_FILE_DIR:${BINARY_NAME}>/doc_engine.dll"
)
```

> macOS / Linux 当前**没有**等价的 CMake 集成，需手动 `cargo build -p doc-native` + 拷贝 dylib/so。V2 计划同步。

### 2.8 端到端冒烟（`flutter_app/bin/native_smoke.dart`）

```dart
// 用法：dart run flutter_app/bin/native_smoke.dart [main_tex]
import 'dart:io';
import 'package:doc_engine/native_bridge.dart';

Future<int> main(List<String> args) async {
  final zipBytes = await File('examples/paper3/upload.zip').readAsBytes();
  await NativeBridge.instance.ensureReady();
  final result = await NativeBridge.instance.convertZipToDocx(zipBytes, 'main-jos.tex');
  await File('examples/paper3/output/desktop-main-jos.docx').writeAsBytes(result.docx);
  return 0;
}
```

### 2.9 性能特征

* 首次 dll open + 函数查找：< 10 ms。
* 8 KB LaTeX 转换：~800 ms。
* 内存峰值：受 C 堆控制（无 GC 抖动）。
* 跨平台 binary：单 cdylib；同一份 Rust 代码编译到三平台。

### 2.10 限制

* 必须把 `doc_engine.dll` 放到可执行文件旁 / `PATH` 中。
* 不支持 WASM 平台（iOS / Android 待 V2 启用 native 编译）。
* 线程局部错误：跨线程调用 `doc_engine_last_error` 可能拿到其它线程的错误（当前单线程使用，无影响）。

---

## 3. 模式 C — HTTP 集成（服务端 / 跨域）

### 3.1 适用场景

* 集成到企业内部系统。
* 浏览器/桌面端不能集成 WASM/FFI 的场景。
* 跨域 API 转换。

### 3.2 必备组件

| 组件 | 路径 |
|------|------|
| `doc-server` 二进制 | `target/release/doc-server` |

### 3.3 启动

```bash
DOC_SERVER_ADDR=0.0.0.0:8080 cargo run --release -p doc-server
```

默认监听 `0.0.0.0:8080`。日志由 `tracing-subscriber` 控制，env filter 默认 `info`。

### 3.4 API

#### `GET /api/v1/health`

```http
HTTP/1.1 200 OK
Content-Type: application/json

{"status":"ok"}
```

#### `GET /api/v1/version`

```http
HTTP/1.1 200 OK
Content-Type: application/json

{"name":"doc-server","version":"0.1.0"}
```

#### `POST /api/v1/convert`

* **Content-Type**: `multipart/form-data`
* **Fields**:
  * `file`（必填）：项目 zip 字节（≤ 50 MiB）
  * `main_tex`（可选）：主 .tex 相对路径，缺省 `main-jos.tex`
* **成功响应**:
  ```http
  HTTP/1.1 200 OK
  Content-Type: application/vnd.openxmlformats-officedocument.wordprocessingml.document
  Content-Disposition: attachment; filename="<sanitized>.docx"
  Content-Length: <docx_len>
  
  <docx bytes>
  ```
* **失败响应**:
  ```http
  HTTP/1.1 400 Bad Request
  Content-Type: application/json
  
  {"error":"parse","message":"解析错误：..."}
  ```

### 3.5 curl 示例

```bash
# 1) 健康检查
curl http://127.0.0.1:8080/api/v1/health

# 2) 转换
curl -X POST http://127.0.0.1:8080/api/v1/convert \
  -F "file=@examples/paper3/upload.zip" \
  -F "main_tex=main-jos.tex" \
  -o out.docx

# 3) 验证
file out.docx  # → Microsoft Word 2007+
```

### 3.6 关键约束

* **请求体 ≤ 50 MiB**（`tower_http::limit::RequestBodyLimitLayer` + `axum::body::to_bytes(_, MAX_BODY)`）。
* **docx ≥ 4 KiB**（routes.rs 内部断言）。
* **docx 头**：`PK\x03\x04`（routes.rs 内部断言）。
* **错误状态码**：
  * `400 Bad Request`：Io / Parse / MissingField
  * `422 Unprocessable Entity`：Unsupported
  * `500 Internal Server Error`：Serialize

### 3.7 关键实现（`crates/server/src/routes.rs`）

```rust
async fn convert(request: Request) -> Result<Response, ApiError> {
    let (_parts, body) = request.into_parts();
    let full_body = axum::body::to_bytes(body, MAX_BODY)
        .await
        .map_err(|e| ApiError::Io(format!("body read error: {e}")))?;

    let file_part = extract_multipart_field(&full_body, "file")?
        .ok_or(ApiError::MissingField("file"))?;
    if file_part.is_empty() {
        return Err(ApiError::MissingField("file"));
    }

    let main_tex = extract_multipart_field(&full_body, "main_tex")?
        .and_then(|v| String::from_utf8(v).ok())
        .unwrap_or_else(|| "main-jos.tex".to_string());

    let result = doc_core::convert_zip(&file_part, &main_tex, &ConvertOptions::default())?;

    if result.docx.len() < 4 * 1024 {
        return Err(ApiError::Io(format!("docx 字节数异常：{}", result.docx.len())));
    }
    if &result.docx[..4] != b"PK\x03\x04" {
        return Err(ApiError::Io("docx 头部非 PK\\x03\\x04".into()));
    }

    let mime: Mime = "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        .parse().expect("static mime is valid");
    let docx_len = result.docx.len();
    let resp = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime.as_ref())
        .header(header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}.docx\"", sanitize(&main_tex)))
        .header(header::CONTENT_LENGTH, docx_len)
        .body(axum::body::Body::from(result.docx))
        .map_err(|e| ApiError::Io(e.to_string()))?;
    Ok(resp.into_response())
}
```

### 3.8 multipart 自定义解析

* 不使用 `axum::extract::Multipart`（性能更好，控制更细）。
* `find_first_boundary` 解析首行 boundary。
* `extract_multipart_field(name)` 找到名为 `name="<field>"` 的 part。

### 3.9 性能特征

* tokio multi-thread runtime。
* 单请求 < 50 MiB：tokio worker 内存允许即可。
* 8 KB LaTeX 转换：~800 ms。
* 无持久化（stateless）。

### 3.10 部署建议

* 反向代理（Nginx / Caddy）：开启 gzip / TLS。
* systemd unit：
  ```ini
  [Service]
  ExecStart=/opt/doc-engine/doc-server
  Environment=DOC_SERVER_ADDR=0.0.0.0:8080
  Restart=on-failure
  ```
* Docker：参见 [07-deployment/04-server-deploy.md](../07-deployment/04-server-deploy.md)。

---

## 4. 模式对比

| 维度 | WASM | Native FFI | HTTP |
|------|------|------------|------|
| 平台 | Web / Chrome 扩展 | Win / macOS / Linux | 任意 |
| 部署 | 静态资源 | 动态库 | 二进制 |
| 性能 | ~1x native | 1x native | 1x native + 网络 |
| 网络 | 不需要 | 不需要 | 需要 |
| 内存峰值 | 4 GB 浏览器 | 系统 RAM | 系统 RAM |
| 单请求大小 | 受 fetch 限制 | 系统 RAM | 50 MiB |
| 安全边界 | browser sandbox | OS 进程隔离 | TLS / auth |
| 集成难度 | 低 | 中 | 极低 |
| 适用产品 | PWA / 扩展 | 桌面端 App | 企业服务 / 跨域 API |

---

## 5. 进一步阅读

* [02-layered-architecture.md](./02-layered-architecture.md) — 依赖关系
* [05-key-tech/](../05-key-tech/) — 各 crate 深入技术
* [06-user-guide/](../06-user-guide/) — 各类使用方式
* [07-deployment/](../07-deployment/) — 各类构建/部署
