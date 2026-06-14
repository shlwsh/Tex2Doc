// lib/bridge.dart
// ------------------------------------------------------------
// Doc-engine 统一桥接外观（条件 import 入口）
//
// 平台分发：
//   web   → bridge_web.dart   (WASM via dart:js_interop)
//   其他  → bridge_stub.dart  (走 native_bridge.dart，由 crates/native 提供)
//
// 两侧都 export 一个 `DocEngineBridge` 类（同名同 API），由本文件聚合。
// ------------------------------------------------------------
import 'dart:typed_data';

import 'bridge_stub.dart' if (dart.library.js_interop) 'bridge_web.dart';

class DocEngineFacade {
  DocEngineFacade._();

  /// 握手：返回底层引擎版本字符串
  static Future<String> version() async {
    try {
      return await DocEngineBridge.version();
    } on Object catch (e) {
      throw StateError('无法初始化 Doc-engine 引擎：$e');
    }
  }

  static bool get isReady => DocEngineBridge.isReady;

  static Future<Uint8List> convertZipToDocx(Uint8List zip, String mainTex) async {
    return DocEngineBridge.convertZipToDocx(zip, mainTex);
  }
}
