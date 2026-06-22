# `flutter_app/` 详尽说明

> 本节列出 Flutter 多端工程的全部文件、作用、配置约定。

---

## 1. 顶层文件

| 文件 | 作用 |
|------|------|
| `.metadata` | Flutter 工程元信息（自动生成） |
| `analysis_options.yaml` | Dart 静态分析规则（基于 `package:flutter_lints/flutter.yaml`） |
| `doc_engine.iml` | IDEA / Android Studio 模块文件 |
| `pubspec.yaml` | Dart 依赖 + Flutter 配置 |
| `pubspec.lock` | 锁文件（已入仓） |
| `README.md` | Flutter 子工程说明（项目根 README 的镜像） |

### 1.1 `analysis_options.yaml`（节选）

```yaml
include: package:flutter_lints/flutter.yaml
linter:
  rules:
    avoid_print: true        # 测试外禁用 print
    prefer_single_quotes: true
analyzer:
  exclude:
    - "**/*.g.dart"
    - "**/*.freezed.dart"
  errors:
    invalid_annotation_target: ignore
```

---

## 2. `bin/` — 桌面端端到端

### `bin/native_smoke.dart`

```dart
// 用法（仓库根）：dart run flutter_app/bin/native_smoke.dart [main_tex]
//
// 流程：
// 1. 读 examples/paper3/upload.zip
// 2. 调 NativeBridge.convertZipToDocx
// 3. 写 examples/paper3/output/desktop-main-jos.docx
// 4. 断言：>= 4 KiB + "PK\x03\x04" 头
//
// 退出码：0=通过，1=参数错，2=zip 缺失，3=init 失败，4=过小，5=魔数错
```

* **关键调用**：`NativeBridge.instance.convertZipToDocx(zipBytes, mainTex)`
* **环境变量**：`DOC_ENGINE_LIB` 覆盖默认动态库名
* **不需要 Flutter runtime**（纯 dart 脚本）

---

## 3. `lib/` — Dart 源代码

### 3.1 文件清单

| 文件 | 行数 | 作用 |
|------|------|------|
| `main.dart` | 17 | 入口；调 `runApp(DocEngineApp(isWeb: kIsWeb))` |
| `workspace_app.dart` | 185 | 共享 Material 3 UI（状态卡 + 转换卡） |
| `bridge.dart` | 32 | 条件 import 聚合；暴露 `DocEngineFacade` |
| `bridge_stub.dart` | 33 | 桌面端桥接（导出 `DocEngineBridge`） |
| `bridge_web.dart` | 27 | Web 端桥接（导出 `DocEngineBridge`） |
| `native_bridge.dart` | 192 | `NativeBridge`（dart:ffi）+ `NativeConvertResult` + `NativeBridgeException` |
| `wasm_bridge.dart` | 156 | `WasmBridge`（dart:js_interop）+ `WasmBridgeException` |

### 3.2 详细说明

#### `main.dart`
```dart
import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/material.dart';
import 'workspace_app.dart';

void main() {
  runApp(DocEngineApp(isWeb: kIsWeb));
}
```

#### `bridge.dart`（条件 import）
```dart
import 'bridge_stub.dart' if (dart.library.js_interop) 'bridge_web.dart';

class DocEngineFacade {
  DocEngineFacade._();
  static Future<String> version() async { ... }
  static bool get isReady => DocEngineBridge.isReady;
  static Future<Uint8List> convertZipToDocx(Uint8List zip, String mainTex) async {
    return DocEngineBridge.convertZipToDocx(zip, mainTex);
  }
}
```

#### `bridge_stub.dart`（桌面）
```dart
import 'native_bridge.dart';
class DocEngineBridge {
  static bool get isReady => NativeBridge.instance.isReady;
  static Future<String> version() async { ... }
  static Future<Uint8List> convertZipToDocx(Uint8List zip, String mainTex) async {
    final r = await NativeBridge.instance.convertZipToDocx(zip, mainTex);
    return r.docx;
  }
}
```

#### `bridge_web.dart`（Web）
```dart
import 'wasm_bridge.dart';
class DocEngineBridge {
  static bool get isReady => WasmBridge.instance.isReady;
  static Future<String> version() async { ... }
  static Future<Uint8List> convertZipToDocx(Uint8List zip, String mainTex) async {
    return WasmBridge.instance.convertZipToDocx(zip, mainTex);
  }
}
```

#### `native_bridge.dart`（FFI）
* 类型定义：`_VersionNative` / `_VersionDart` / `_LastErrorNative` / `_LastErrorDart` / `_FreeNative` / `_FreeDart` / `_ConvertZipNative` / `_ConvertZipDart`。
* `NativeBridge.instance`（单例）：
  * `ensureReady({String? libPath})`：用 `Platform.isWindows` / `isMacOS` / `isLinux` 选 `doc_engine`（可 `DOC_ENGINE_LIB` 覆盖），调 `ffi.DynamicLibrary.open`，注册 4 个函数。
  * `convertZipToDocx(Uint8List zipBytes, String mainTexPath)`：用 `pkg_ffi.calloc` 分配 C 堆，`memcpy` 写入 zip + main_tex 字符串；调 `doc_engine_convert_zip`；读出 `out_docx_ptr/len` + `out_warnings_ptr/len`；`asTypedList` 零拷贝读出；调 `doc_engine_free` 释放。
* `NativeConvertResult { docx, warnings }`
* `NativeBridgeException`

#### `wasm_bridge.dart`（JS interop）
* `WasmBridge.instance`（单例）：
  * `ensureReady({Duration timeout = 30s})`：等 `window.docEngine` 全局对象；或等 `doc-engine-ready` 自定义事件；超时抛 `WasmBridgeException`。
  * `convertZipToDocx(Uint8List zipBytes, String mainTexPath, {String? optionsJson})`：把 Dart `Uint8List` 转 `JSUint8Array`（零拷贝），调 `window.docEngine.convert_zip_to_docx`；把返回的 `JSUint8Array` 转 Dart `Uint8List`。
  * `getVersion()`：调 `window.docEngine.version()`。

#### `workspace_app.dart`（UI）
* `DocEngineApp`（StatelessWidget）：
  * `MaterialApp` Material 3 + 双主题（light/dark）
  * 标题：`Doc-engine · LaTeX → DOCX`
  * `AppBar` 显示当前平台
  * body：`_StatusCard`（握手 / 错误）+ `_ConvertCard`（按钮触发 `DocEngineFacade.version()`）
* `_StatusCard`（StatefulWidget）：
  * `initState` 异步调 `DocEngineFacade.version()` 显示版本
  * 错误显示红色 `error_outline` 图标
* `_ConvertCard`（StatefulWidget）：
  * 按钮触发 `_onSmoke`：调 `DocEngineFacade.version()` 验证桥接

---

## 4. `test/` — 单元测试

| 文件 | 作用 |
|------|------|
| `widget_test.dart` | 基础 widget smoke（占位） |
| `bridge_smoke_test.dart` | 桥接层冒烟（仅测试类存在性，不调真实引擎） |

运行：

```bash
flutter test
```

---

## 5. `wasm/` — WASM 产物目录

由 `wasm-pack build crates/wasm --target web --out-dir ../flutter_app/wasm/pkg` 生成。

```
wasm/
└── pkg/
    ├── doc_engine.js           # ESM 入口（含 init 函数）
    ├── doc_engine.d.ts         # TypeScript 类型
    ├── doc_engine_bg.wasm      # WASM 字节流（~3.5 MB）
    ├── doc_engine_bg.wasm.d.ts # TypeScript 声明
    └── package.json            # npm 元信息
```

> 重新构建：`npm run build:wasm`

---

## 6. `web/` — Web 入口

| 文件 | 作用 |
|------|------|
| `favicon.png` | 浏览器标签图标 |
| `index.html` | Web 入口（含 `<script src="wasm/doc_engine.js" defer>`） |
| `manifest.json` | PWA manifest（name / icons / theme_color） |
| `icons/Icon-192.png` | PWA 图标 192×192 |
| `icons/Icon-512.png` | PWA 图标 512×512 |
| `icons/Icon-maskable-192.png` | 适配启动屏 192×192 |
| `icons/Icon-maskable-512.png` | 适配启动屏 512×512 |
| `wasm/doc_engine.js` | 与 `wasm/pkg/` 一致（脚本复制） |
| `wasm/doc_engine_bg.wasm` | 同上 |

---

## 7. `windows/` — Windows 桌面端

### 7.1 `CMakeLists.txt`（关键定制）

* 标准 Flutter Windows runner（`add_subdirectory(runner)`）。
* **doc-native 集成**（V1 关键）：

  ```cmake
  find_program(CARGO_BIN cargo HINTS $ENV{CARGO_HOME} REQUIRED)
  set(DOC_NATIVE_CRATE "${CMAKE_CURRENT_SOURCE_DIR}/../../crates/native")

  # 选 profile
  if(CMAKE_BUILD_TYPE STREQUAL "Release" OR CMAKE_BUILD_TYPE STREQUAL "Profile")
    set(DOC_NATIVE_RUST_PROFILE "release")
    set(DOC_NATIVE_RUST_PROFILE_FLAG "--release")
    set(DOC_NATIVE_DLL_DIR_SUFFIX "release")
  else()
    set(DOC_NATIVE_RUST_PROFILE "dev")
    set(DOC_NATIVE_RUST_PROFILE_FLAG "")
    set(DOC_NATIVE_DLL_DIR_SUFFIX "debug")
  endif()
  set(DOC_NATIVE_DLL_SRC "${CMAKE_CURRENT_SOURCE_DIR}/../../target/${DOC_NATIVE_DLL_DIR_SUFFIX}/doc_native.dll")

  # 编译期：cargo build + 拷贝 DLL
  add_custom_target(doc_native_runtime ALL
    COMMAND ${CARGO_BIN} build -p doc-native ${DOC_NATIVE_RUST_PROFILE_FLAG}
    WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}/../.."
    COMMENT "[doc-native] cargo build -p doc-native ${DOC_NATIVE_RUST_PROFILE_FLAG}"
  )
  add_custom_command(TARGET doc_native_runtime POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
      "${DOC_NATIVE_DLL_SRC}"
      "$<TARGET_FILE_DIR:${BINARY_NAME}>/doc_engine.dll"
  )

  # install 阶段也带上
  install(FILES "${DOC_NATIVE_DLL_SRC}"
    DESTINATION "${INSTALL_BUNDLE_LIB_DIR}"
    COMPONENT Runtime
    OPTIONAL
    RENAME "doc_engine.dll")
  ```

### 7.2 `flutter/`

* `CMakeLists.txt`：Flutter 引擎库引入。
* `generated_plugins.cmake`：插件元信息。
* `generated_plugin_registrant.{cc,h}`：插件注册代码。
* `ephemeral/`：Flutter 引擎 dll + ICU 数据（**gitignore 候选**）。

### 7.3 `runner/`

* `CMakeLists.txt`：桌面 app 可执行目标。
* `Runner.rc`：Windows 资源（图标、版本信息）。
* `flutter_window.{cpp,h}`：Flutter 窗口类。
* `main.cpp`：入口。
* `win32_window.{cpp,h}`：Win32 窗口底层。
* `utils.{cpp,h}`：工具函数。
* `runner.exe.manifest`：UAC / DPI 清单。
* `resources/app_icon.ico`：应用图标。

---

## 8. `build/` — 编译产物（不入仓）

```
build/
├── .last_build_id
├── flutter_assets/        # Web 编译源（fonts / shaders / NativeAssets）
├── web/                   # `flutter build web` 输出
│   ├── index.html
│   ├── main.dart.js
│   ├── manifest.json
│   ├── canvaskit/         # CanvasKit 引擎（~22 MB）
│   ├── icons/
│   ├── assets/
│   ├── fonts/
│   ├── shaders/
│   └── wasm/              # 与 web/wasm/ 一致
├── native_assets/windows/
├── test_cache/
├── unit_test_assets/
└── windows/x64/           # CMake 编译产物（.vcxproj / .sln / .cmake）
```

---

## 9. 关键约定

### 9.1 平台分发

`kIsWeb` 由 `flutter/foundation.dart` 提供；其余平台按 `Platform.isXxx` 分发。

### 9.2 桥接层暴露 API

| 方法 | Web | Desktop |
|------|-----|---------|
| `version()` | `WasmBridge.getVersion()` | `NativeBridge.version` |
| `isReady` | `WasmBridge.isReady` | `NativeBridge.isReady` |
| `convertZipToDocx(zip, mainTex)` | `WasmBridge.convertZipToDocx` | `NativeBridge.convertZipToDocx` |

### 9.3 错误处理

* `WasmBridgeException` / `NativeBridgeException` 透传到 UI 层（`setState(_error = e.toString())`）。
* `DocEngineFacade.version()` 把异常包装为 `StateError('无法初始化 Doc-engine 引擎：$e')`。

### 9.4 性能注意

* Web：`Uint8List` ↔ `JSUint8Array` 用 `toJS.toDart`，零拷贝。
* Desktop：所有 C 堆分配都在 `try/finally` 块中显式 `calloc.free` + `doc_engine_free`，避免泄漏。

---

## 10. 进一步阅读

* [05-key-tech/05-vfs-and-fonts.md](../05-key-tech/05-vfs-and-fonts.md) — VFS / 字体技术细节
* [06-user-guide/03-desktop.md](../06-user-guide/03-desktop.md) — 桌面端使用
* [07-deployment/02-flutter-build.md](../07-deployment/02-flutter-build.md) — Flutter 构建
