# Flutter Desktop 使用
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



> 本节描述 **Flutter Desktop**（Windows / macOS / Linux）的使用方式。最适合：本地化离线转换、批量任务、与系统集成。

---

## 1. 准备环境

| 平台 | 必需 |
|------|------|
| **Windows** | Rust + Visual Studio 2022 Build Tools（C++ 工作负载） + Flutter SDK |
| **macOS** | Rust + Xcode Command Line Tools + Flutter SDK |
| **Linux** | Rust + clang + GTK 开发包 + Flutter SDK |

```bash
# Linux 示例
sudo apt install -y libgtk-3-dev libstdc++6 libstdc++6-dev
```

---

## 2. 构建桌面 App

### 2.1 自动构建（Windows 首选）

```bash
# 仓库根
npm run build:desktop
# 等价于：
#   cargo build -p doc-native
#   cd flutter_app && flutter build windows --debug
```

CMake 在 `flutter build windows` 时自动：
1. 调 `cargo build -p doc-native --release`（profile 与 CMake 一致）。
2. 把 `target/<profile>/doc_native.dll` 拷贝到 `runner/<BINARY_NAME>/doc_engine.dll`。

### 2.2 手动构建

```bash
# 1. 构建 native cdylib
cargo build -p doc-native --release
# 产物：target/release/doc_native.dll (Windows) / libdoc_native.dylib (macOS) / libdoc_native.so (Linux)

# 2. 拷贝到可执行文件目录
# Windows
cp target/release/doc_native.dll flutter_app/build/windows/x64/runner/Release/doc_engine.dll
# macOS
cp target/release/libdoc_native.dylib flutter_app/build/macos/Build/Products/Release/doc_engine.app/Contents/Frameworks/
# Linux
cp target/release/libdoc_native.so flutter_app/build/linux/x64/release/bundle/lib/

# 3. 构建 Flutter App
cd flutter_app
flutter build windows --debug    # 或 --release
flutter build macos --debug
flutter build linux --debug
```

### 2.3 平台注意事项

* **macOS / Linux**：当前**没有**等价的 CMake 集成（Windows 专属）。V2 计划同步添加。
* **iOS / Android**：当前**未在 V1 范围**（待 V2 启用 native 编译）。

---

## 3. 启动 App

### 3.1 Windows

```bash
flutter_app/build/windows/x64/runner/Release/doc_engine.exe
```

或开发模式：

```bash
cd flutter_app
flutter run -d windows
```

### 3.2 macOS

```bash
open flutter_app/build/macos/Build/Products/Release/doc_engine.app
```

或开发模式：

```bash
cd flutter_app
flutter run -d macos
```

### 3.3 Linux

```bash
flutter_app/build/linux/x64/release/bundle/doc_engine
```

或开发模式：

```bash
cd flutter_app
flutter run -d linux
```

---

## 4. 端到端冒烟

### 4.1 dart 脚本

```bash
cd flutter_app
dart run bin/native_smoke.dart
# 或：dart run bin/native_smoke.dart main-zh.tex
```

行为：
1. 读 `examples/paper3/upload.zip`。
2. 调 `NativeBridge.instance.convertZipToDocx`。
3. 写 `examples/paper3/output/desktop-main-jos.docx`。
4. 断言：≥ 4 KiB + `PK\x03\x04` 头。

### 4.2 自定义 zip 路径

```dart
// 编辑 bin/native_smoke.dart
final zipPath = '/path/to/your/project.zip';
final mainTex = 'main.tex';
```

或通过环境变量 `DOC_ENGINE_LIB` 覆盖动态库名。

### 4.3 集成到 CI

```yaml
- name: Desktop smoke
  run: |
    cd flutter_app
    dart run bin/native_smoke.dart
```

---

## 5. App UI

`flutter_app/lib/workspace_app.dart`：

* **状态卡**：
  * 绿色对勾 + "引擎已就绪" + `Version: doc-native/0.1.0`
  * 失败时红色错误图标
* **转换卡**：
  * "核心引擎（Web=WASM / Desktop=Native FFI）"
  * "握手 / 状态检查" 按钮
  * docx 字节数显示

* 当前 UI 是**演示**级别；生产应加：
  * 文件选择器（`file_picker` 插件）
  * 进度条（`DocEngineFacade.convertZipToDocx` 接受 `onProgress` 回调）
  * 历史记录
  * 主题切换

---

## 6. 性能特征

| 操作 | 桌面端耗时 |
|------|-----------|
| 库加载（dll open + 函数查找） | < 10 ms |
| `version()` 握手 | < 1 ms |
| 8 KB LaTeX 转换（paper3） | ~800 ms |
| 内存峰值 | ~50 MB |
| 冷启动到 UI | < 1 s（Windows / macOS） |

---

## 7. 调试

### 7.1 Flutter

```bash
cd flutter_app
flutter run -d windows --debug      # 启用 DevTools
flutter logs
```

### 7.2 Rust 侧

* `cargo build -p doc-native` 输出在 `target/<profile>/build/doc_native/...`。
* 加日志：
  ```rust
  eprintln!("[doc-native] convert_zip called");
  ```
* 在 Dart 端读：
  ```dart
  // 用 Process 启动可执行文件，捕获 stderr
  ```

### 7.3 内存问题

* 内存泄漏检查：`valgrind --leak-check=full ./doc_engine`（Linux）
* 段错误：`gdb ./doc_engine core`（Linux）

---

## 8. 打包分发

### 8.1 Windows

```bash
cd flutter_app
flutter build windows --release
# 产物：flutter_app/build/windows/x64/runner/Release/
#   doc_engine.exe
#   doc_engine.dll
#   *.dll（Flutter 引擎 + 应用）
#   data/icu_dat
```

打包为安装包：用 **Inno Setup** / **MSIX** / **NSIS**。

### 8.2 macOS

```bash
cd flutter_app
flutter build macos --release
# 产物：flutter_app/build/macos/Build/Products/Release/doc_engine.app
```

打包为 DMG：

```bash
hdiutil create -volname Doc-engine -srcfolder doc_engine.app -ov -format UDZO doc-engine.dmg
```

### 8.3 Linux

```bash
cd flutter_app
flutter build linux --release
# 产物：flutter_app/build/linux/x64/release/bundle/
#   doc_engine
#   lib/*.so
```

打包为 AppImage / Snap / Flatpak。

---

## 9. 进一步阅读

* [01-cli-and-script.md](./01-cli-and-script.md) — CLI / 脚本
* [07-deployment/02-flutter-build.md](../07-deployment/02-flutter-build.md) — Flutter 构建
* [04-architecture/03-frontend-bridges.md](../04-architecture/03-frontend-bridges.md) — FFI 集成详情
