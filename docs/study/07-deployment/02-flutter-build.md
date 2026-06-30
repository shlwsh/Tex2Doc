# Flutter 多端构建
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



> 本节描述 Flutter App 在 Web / Windows / macOS / Linux 四端的完整构建流程。

---

## 1. 环境要求

### 1.1 Flutter SDK

* **3.12+**（`pubspec.yaml` 写 `^3.12.2`）
* [下载](https://flutter.dev/docs/get-started/install)

### 1.2 平台工具

| 平台 | 工具 |
|------|------|
| **Web** | Chrome 114+（测试用） |
| **Windows** | Visual Studio 2022 + Flutter Windows 工具链 |
| **macOS** | Xcode 14+ + CocoaPods |
| **Linux** | clang + cmake + ninja + GTK 3 |

### 1.3 验证

```bash
flutter doctor -v
```

预期全部 ✅。

---

## 2. 拉取依赖

```bash
cd flutter_app
flutter pub get
```

产物：
* `flutter_app/.dart_tool/`（Dart 工具缓存，gitignore）
* `flutter_app/pubspec.lock`（锁定包版本，**已入仓**）

---

## 3. Web 构建

### 3.1 标准命令

```bash
flutter build web --no-source-maps --no-tree-shake-icons
```

* `--no-source-maps`：~1.5 MB 减小。
* `--no-tree-shake-icons`：保留所有 icon font。
* 产物：`flutter_app/build/web/`

### 3.2 评估 canvaskit vs skwasm vs html renderer

```bash
# 默认（canvaskit ~22 MB）
flutter build web

# 关闭 canvaskit（仅 HTML 渲染 ~2 MB，但功能受限）
flutter build web --web-renderer html
```

> Tex2Doc 表格 + OMML + 复杂排版需要 CanvasKit/Skwasm，HTML 渲染不推荐。

### 3.3 部署产物

```bash
# 静态资源全部
ls -la flutter_app/build/web/
# 关键：
#   index.html
#   main.dart.js（~2.3 MB）
#   manifest.json
#   canvaskit/（~22 MB）
#   wasm/（~3.5 MB）
#   icons/
#   assets/
```

> 部署见 [02-pwa-web.md](../06-user-guide/02-pwa-web.md)。

---

## 4. Windows 桌面构建

### 4.1 标准命令

```bash
# 仓库根
npm run build:desktop
# 等价于：
#   cargo build -p doc-native
#   cd flutter_app && flutter build windows --debug
```

> CMake 会自动调 `cargo build -p doc-native` 并把 `doc_native.dll` 拷贝到 `bin/`。

### 4.2 手动步骤

```bash
# 1. 构建 native cdylib
cd <仓库根>
cargo build -p doc-native --release
# 产物：target/release/doc_native.dll

# 2. 拷贝
cp target/release/doc_native.dll flutter_app/build/windows/x64/runner/Release/doc_engine.dll

# 3. 构建 Flutter
cd flutter_app
flutter build windows --debug       # 或 --release
```

### 4.3 产物

```
flutter_app/build/windows/x64/runner/Release/
├── doc_engine.exe
├── doc_engine.dll                    # Rust cdylib
├── flutter_windows.dll
├── icudtl.dat
├── *.dll
└── data/
    ├── flutter_assets/
    └── icu/
```

### 4.4 打包分发

用 **Inno Setup** / **MSIX** / **NSIS**：

```iss
; Inno Setup 脚本（节选）
[Files]
Source: "build\windows\x64\runner\Release\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autodesktop}\Doc-engine"; Filename: "{app}\doc_engine.exe"
```

---

## 5. macOS 桌面构建

### 5.1 命令

```bash
cd flutter_app
flutter build macos --debug       # 或 --release
```

### 5.2 手动 native 集成

macOS 当前**没有**等价的 CMake 集成（Windows 专属）。手动步骤：

```bash
# 1. 构建 native cdylib
cd <仓库根>
cargo build -p doc-native --release
# 产物：target/release/libdoc_native.dylib

# 2. 拷贝到 app bundle 的 Frameworks
cp target/release/libdoc_native.dylib \
   flutter_app/build/macos/Build/Products/Release/doc_engine.app/Contents/Frameworks/

# 3. 构建
cd flutter_app
flutter build macos --release
```

### 5.3 产物

```
flutter_app/build/macos/Build/Products/Release/doc_engine.app/
├── Contents/
│   ├── Info.plist
│   ├── MacOS/
│   │   └── doc_engine
│   ├── Frameworks/
│   │   ├── libdoc_native.dylib   # Rust cdylib
│   │   ├── AppKit.framework
│   │   └── ...
│   └── Resources/
│       └── flutter_assets/
```

### 5.4 打包为 DMG

```bash
hdiutil create -volname "Doc-engine" \
  -srcfolder flutter_app/build/macos/Build/Products/Release/doc_engine.app \
  -ov -format UDZO Doc-engine.dmg
```

### 5.5 公证（Gatekeeper）

macOS Catalina+ 需要公证：

```bash
# 1. 申请 Apple Developer ID
# 2. 签名
codesign --deep --force --options runtime \
  --sign "Developer ID Application: <your name>" \
  flutter_app/build/macos/Build/Products/Release/doc_engine.app

# 3. 公证
xcrun notarytool submit Doc-engine.dmg \
  --keychain-profile <profile> \
  --wait

# 4. Staple
xcrun stapler staple Doc-engine.dmg
```

---

## 6. Linux 桌面构建

### 6.1 命令

```bash
cd flutter_app
flutter build linux --debug       # 或 --release
```

### 6.2 手动 native 集成

Linux 当前**没有**等价的 CMake 集成。手动步骤：

```bash
# 1. 构建 native cdylib
cd <仓库根>
cargo build -p doc-native --release
# 产物：target/release/libdoc_native.so

# 2. 拷贝
cp target/release/libdoc_native.so \
   flutter_app/build/linux/x64/release/bundle/lib/

# 3. 构建
cd flutter_app
flutter build linux --release
```

### 6.3 产物

```
flutter_app/build/linux/x64/release/bundle/
├── doc_engine
├── lib/
│   ├── libdoc_native.so
│   ├── libflutter_linux_gtk.so
│   └── ...
├── data/
│   ├── flutter_assets/
│   └── icudtl.dat
```

### 6.4 打包为 AppImage

用 [`appimage-builder`](https://appimage-builder.readthedocs.io/)：

```yaml
# AppImageBuilder.yml
version: 1
AppDir:
  path: ./AppDir
  app_info:
    name: Doc-engine
    icon: doc-engine
    exec: usr/bin/doc_engine
    exec_args: $@
  files:
    include: [flutter_app/build/linux/x64/release/bundle/*]
  apt:
    include: [libgtk-3-0, libstdc++6]
```

```bash
appimage-builder
```

### 6.5 打包为 Snap

```yaml
# snap/snapcraft.yaml
name: doc-engine
base: core22
version: '0.1.0'
summary: LaTeX to DOCX converter
description: |
  Pure Rust core + Flutter UI for high-fidelity LaTeX → DOCX conversion.
grade: stable
confinement: strict
apps:
  doc-engine:
    command: usr/bin/doc_engine
    extensions: [gnome]
    plugs: [home, network]
parts:
  doc-engine:
    plugin: nil
    source: flutter_app/build/linux/x64/release/bundle/
    override-build: |
      set -eux
      cp -R $CRAFT_PART_SRC/* $CRAFT_PART_INSTALL/
    stage-packages: [libgtk-3-0, libstdc++6]
```

```bash
snapcraft
```

---

## 7. 跨平台一致性保证

### 7.1 平台分发（Dart）

`lib/main.dart`：

```dart
import 'package:flutter/foundation.dart' show kIsWeb;
import 'workspace_app.dart';

void main() {
  runApp(DocEngineApp(isWeb: kIsWeb));
}
```

### 7.2 桥接层条件 import

`lib/bridge.dart`：

```dart
import 'bridge_stub.dart' if (dart.library.js_interop) 'bridge_web.dart';
```

* Web：`bridge_web.dart`（WASM）。
* 其它：`bridge_stub.dart` → `native_bridge.dart`（FFI）。

### 7.3 跨平台测试

```bash
# 1. 单元测试
flutter test

# 2. 集成（仅桌面）
cd flutter_app
dart run bin/native_smoke.dart

# 3. 端到端（仅 Web）
node scripts/e2e_paper3.mjs
```

---

## 8. 性能优化

### 8.1 减小 Web 产物

```bash
# --release 默认
flutter build web --release \
  --no-source-maps \
  --no-tree-shake-icons \
  --dart-define=FLUTTER_WEB_USE_SKIA=true \
  --pwa-strategy=offline-first
```

### 8.2 桌面端 binary 减小

* `strip` Rust binary。
* `lto = "thin"` 跨 crate 优化。
* UPX 压缩（仅 Linux）：

  ```bash
  upx --best --lzma doc_engine
  ```

### 8.3 启动加速

* 延迟加载（deferred loading）：V2 计划。
* AOT：Flutter release 默认开启。

---

## 9. 故障排查

### 9.1 平台工具链缺失

| 错误 | 解决 |
|------|------|
| `cmake not found` | Linux：`sudo apt install cmake` |
| `gtk+-3.0 not found` | Linux：`sudo apt install libgtk-3-dev` |
| `Visual Studio not found` | Windows：装 Visual Studio 2022 + Desktop development with C++ |
| `Xcode not found` | macOS：`xcode-select --install` |
| `CocoaPods not found` | macOS：`sudo gem install cocoapods` |

### 9.2 native dll 加载失败

* Windows：确认 `doc_engine.dll` 在 `bin/` 旁 / `PATH`。
* macOS：确认 `libdoc_native.dylib` 在 `Contents/Frameworks/`，且 `install_name_tool` 已设置 `@rpath`。
* Linux：确认 `LD_LIBRARY_PATH` 含 `bundle/lib/`。

### 9.3 WASM 加载失败

* 确认 `wasm/doc_engine.js` 路径正确。
* 确认 Content-Type：`application/wasm`。
* 确认 HTTPS（WASM 不允许 HTTP）。

---

## 10. 进一步阅读

* [01-rust-build.md](./01-rust-build.md) — Rust 核心构建
* [03-wasm-publish.md](./03-wasm-publish.md) — WASM 产物发布
* [06-ci-and-hooks.md](./06-ci-and-hooks.md) — CI / 钩子
* [06-user-guide/03-desktop.md](../06-user-guide/03-desktop.md) — 桌面端使用
