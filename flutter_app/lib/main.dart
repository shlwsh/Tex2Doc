// lib/main.dart
// ------------------------------------------------------------
// Doc-engine Flutter App 入口
//
// Web 端：lib/workspace_web.dart（wasm_bridge.dart + package:web）
// 桌面端：lib/workspace_desktop.dart（native_bridge.dart + dart:io）
//
// 桥接层：lib/bridge.dart（条件 import：wasm_bridge vs native_bridge）
// ------------------------------------------------------------
import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/material.dart';

import 'workspace_app.dart';

void main() {
  runApp(DocEngineApp(isWeb: kIsWeb));
}
