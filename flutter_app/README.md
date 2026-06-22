# Doc-engine Flutter 多端应用

> 状态：M1 预留骨架（M1-M2 任务 F-001/F-002/F-003 占位）。

## 计划

- M1 末：Material 3 骨架 + `flutter_rust_bridge_codegen` 接入。
- M3-M4：中央工作台 + 拖拽 + Riverpod 状态。
- M5-M6：进度总线 + 高级选项侧边栏 + 模板下拉。
- M7-M8：日志抽屉。
- M11-M12：PWA 离线 WASM。

## 待办

- 安装 Flutter SDK 后执行：
  ```bash
  flutter create --platforms=windows,macos,linux,android,ios,web --org com.docengine flutter_app
  ```
- 当前阶段无 Flutter 二进制 / `pubspec.yaml`；待人工或脚本执行 `flutter create`。

详细规范见 `docs/Doc-engine_LaTeX-to-DOCX_技术方案_v2.0_20260614.md` §5.1。
