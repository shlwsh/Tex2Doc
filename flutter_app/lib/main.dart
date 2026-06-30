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

import 'admin/admin_app.dart';
import 'product/extension_page.dart';
import 'product/product_home_app.dart';
import 'user/user_app.dart';

void main() {
  final path = Uri.base.path;
  if (kIsWeb) {
    if (path.startsWith('/admin')) {
      runApp(const AdminApp(isWeb: true));
    } else if (path.startsWith('/app')) {
      runApp(UserApp(isWeb: true));
    } else if (path.startsWith('/extension')) {
      runApp(const ExtensionPage());
    } else {
      runApp(const ProductHomeApp());
    }
  } else {
    runApp(UserApp(isWeb: false));
  }
}
