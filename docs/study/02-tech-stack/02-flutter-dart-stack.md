# Flutter / Dart 技术栈

> Tex2Doc 的多端 UI 全部基于 Flutter 3.12+，通过 `dart:ffi`（桌面）和 `dart:js_interop`（Web）调 Rust 核心。本节列出 Flutter 侧依赖、桥接方式与平台配置。

---

## 1.1 工具链要求

| 工具 | 最低版本 | 用途 |
|------|----------|------|
| Flutter SDK | 3.12+ | 多端 UI 构建 |
| Dart SDK | 3.0+ | `^3.12.2`（pubspec.yaml） |
| 桌面平台依赖 | Windows / macOS / Linux 工具链 | Windows: MSVC / macOS: Xcode CLT / Linux: clang + GTK |

---

## 1.2 Flutter 侧依赖（`pubspec.yaml`）

```yaml
name: doc_engine
description: "A new Flutter project."
publish_to: 'none'
version: 1.0.0+1
environment:
  sdk: ^3.12.2

dependencies:
  flutter:
    sdk: flutter
  cupertino_icons: ^1.0.8    # iOS 风格图标
  web: ^1.1.0                # Web 平台 JS interop 包装
  ffi: ^2.1.3                # 桌面端 dart:ffi 调本地 Rust 库

dev_dependencies:
  flutter_test:
    sdk: flutter
  flutter_lints: ^6.0.0

flutter:
  uses-material-design: true
```

> 故意保持极简：核心 UI 走 Material 3，业务逻辑放在 Dart 桥接层。

---

## 1.3 Dart 桥接层架构

```
lib/
├── main.dart              # App 入口（kIsWeb 分发）
├── workspace_app.dart     # 共享 Material 3 UI
├── bridge.dart            # 条件 import 聚合
├── bridge_stub.dart       # 桌面端桥接（导出 DocEngineBridge）
├── bridge_web.dart        # Web 端桥接（导出 DocEngineBridge）
├── native_bridge.dart     # dart:ffi 实现（Desktop）
└── wasm_bridge.dart       # dart:js_interop 实现（Web）
```

### 1.3.1 条件 import 机制

`bridge.dart`：

```dart
import 'bridge_stub.dart' if (dart.library.js_interop) 'bridge_web.dart';
```

* 编译 Web 时，Dart 自动选 `bridge_web.dart`（绑定到 `window.docEngine`）。
* 编译桌面时，Dart 选 `bridge_stub.dart`（转发到 `native_bridge.dart`）。
* 两侧都暴露同名同 API 的 `DocEngineBridge` 类。

### 1.3.2 桌面桥接（`native_bridge.dart`）

* 平台分发：`Platform.isWindows` / `isMacOS` / `isLinux` → `doc_engine`（同一动态库名）。
* 环境变量覆盖：`DOC_ENGINE_LIB` 允许显式指定 `.dll` / `.dylib` / `.so` 路径。
* FFI 函数：
  * `doc_engine_version() -> *const c_char`
  * `doc_engine_last_error() -> *const c_char`
  * `doc_engine_convert_zip(...) -> c_int`（返回 docx + warnings）
  * `doc_engine_free(ptr)` 释放 Rust 分配
* 内存模型：
  * Dart `Uint8List` 拷贝到 C 堆（`pkg_ffi.calloc`）
  * 主 `tex` 路径字符串 `utf8.encode` 后 `calloc`
  * 输出参数（`out_docx_ptr` / `out_docx_len` / `out_warnings_ptr` / `out_warnings_len`）在 C 堆分配
  * Dart 端 `asTypedList(len)` 零拷贝读出
  * 读完后调 `doc_engine_free` 释放

### 1.3.3 Web 桥接（`wasm_bridge.dart`）

* API：`dart:js_interop` 1.x 官方 API（`Uint8ListToJSUint8Array` / `JSUint8ArrayToUint8Array`）。
* 加载机制：
  1. 等待 `window.docEngine` 全局对象（由 `extension/popup/popup.js` 或 `flutter_app/web/index.html` 预加载）。
  2. 默认 timeout 30 秒；可调。
  3. 自定义事件 `doc-engine-ready` 触发就绪。
* 字节转换：Dart `Uint8List` ↔ JS `Uint8Array` 零拷贝。
* 版本握手：`window.docEngine.version()` 返回字符串。

### 1.3.4 UI 框架

* Material 3 + `useMaterial3: true`
* ColorScheme.fromSeed：seed = `Color(0xFF1565C0)`（蓝）
* 亮 / 暗双主题
* 共享 UI（`workspace_app.dart`）：状态卡 + 转换卡 + 版本握手

---

## 1.4 桌面端平台配置

### 1.4.1 Windows

`flutter_app/windows/CMakeLists.txt`：
* 主体：标准 Flutter Windows runner。
* **关键定制**（`doc_native_runtime` 自定义 target）：
  ```cmake
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
* 行为：CMake 配置时自动跑 `cargo build -p doc-native`，把 `doc_native.dll` 拷贝到 `bin/` 旁（dev 模式 dart:ffi 可直接找到）。
* install 阶段：把 DLL 一并打包到目标目录。

### 1.4.2 macOS / Linux

> 当前工作流：手动 `cargo build -p doc-native` + 复制 `.dylib` / `.so` 到 `Contents/Frameworks/` 或应用根目录。
> 自动化（V2）：计划在 `macos/CMakeLists.txt` 同步加 `add_custom_target`（与 Windows 一致）。

### 1.4.3 Web

`flutter_app/web/` 目录：
* `index.html`：包含 `<script src="wasm/doc_engine.js" defer></script>`。
* `wasm/doc_engine.js` / `wasm/doc_engine_bg.wasm`：由 `wasm-pack build` 输出到 `flutter_app/wasm/pkg/`，再由脚本复制到 `web/wasm/`。
* `manifest.json`：PWA manifest。
* `icons/Icon-{192,512,…}.png`：PWA 图标。

---

## 1.5 入口与启动流程

`lib/main.dart`：

```dart
import 'package:flutter/foundation.dart' show kIsWeb;
import 'workspace_app.dart';

void main() {
  runApp(DocEngineApp(isWeb: kIsWeb));
}
```

`DocEngineApp`（`workspace_app.dart`）：
* `kIsWeb == true` → `Platform: Web`（WASM 桥接）
* `kIsWeb == false` → `Platform: Desktop`（FFI 桥接）
* Material 3 主题 + 状态卡（握手 / 错误）+ 转换卡（按钮触发）

---

## 1.6 桌面端端到端冒烟

`flutter_app/bin/native_smoke.dart`：
* 读 `examples/paper3/upload.zip`
* 调 `NativeBridge.instance.convertZipToDocx`
* 写到 `examples/paper3/output/desktop-main-jos.docx`
* 断言：≥ 4 KiB + `PK\x03\x04` 头
* 退出码 0=通过

调用：

```bash
cd flutter_app
dart run bin/native_smoke.dart
# 或：dart run bin/native_smoke.dart main-zh.tex
```

---

## 1.7 测试

`flutter_app/test/`：

| 文件 | 作用 |
|------|------|
| `widget_test.dart` | 基础 widget smoke |
| `bridge_smoke_test.dart` | 桥接层冒烟（仅 stub，不调真实引擎） |

```bash
flutter test
```

> 真实 e2e 验证放在 `scripts/e2e_paper3.mjs`（Playwright 驱动 Flutter Web）。

---

## 1.8 关键配置文件

| 路径 | 作用 |
|------|------|
| `pubspec.yaml` | Dart 依赖 |
| `pubspec.lock` | 锁定包版本（已入仓） |
| `analysis_options.yaml` | 静态分析规则 |
| `doc_engine.iml` | IDEA 工程文件（自动生成） |
| `.flutter-plugins` / `.flutter-plugins-dependencies` | 插件元信息（运行时生成，已 .gitignore） |
| `.dart_tool/` | Dart 工具缓存（.gitignore） |
| `web/index.html` | Web 入口 |
| `web/manifest.json` | PWA 描述 |
| `windows/CMakeLists.txt` | Windows 桌面构建 |
| `windows/runner/` | 桌面应用模板代码 |
| `windows/flutter/ephemeral/` | Flutter 引擎 dll + ICU（不提交） |

---

## 1.9 构建命令速查

```bash
# Web
cd flutter_app
flutter build web --no-source-maps --no-tree-shake-icons

# Windows 桌面（自动触发 cargo build -p doc-native）
flutter build windows --debug

# macOS 桌面
flutter build macos --debug

# Linux 桌面
flutter build linux --debug

# 跑 App
flutter run -d chrome              # Web
flutter run -d windows             # Windows

# 跑端到端冒烟
dart run bin/native_smoke.dart

# 跑 widget 测试
flutter test
```

> `npm` 别名（仓库根 `package.json`）：
> ```json
> "build:wasm": "wasm-pack build crates/wasm --target web --out-dir ../flutter_app/wasm/pkg --out-name doc_engine --dev",
> "build:web": "cd flutter_app && flutter build web --no-source-maps --no-tree-shake-icons",
> "build:native": "cargo build -p doc-native",
> "build:windows": "cd flutter_app && flutter build windows --debug",
> "build:all": "npm run build:wasm && npm run build:web",
> "build:desktop": "npm run build:native && npm run build:windows",
> ```
