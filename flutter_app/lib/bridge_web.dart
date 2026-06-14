// lib/bridge_web.dart
// ------------------------------------------------------------
// Web 平台桥接：WASM via dart:js_interop
// ------------------------------------------------------------
import 'dart:typed_data';

import 'wasm_bridge.dart';

export 'wasm_bridge.dart' show WasmBridgeException;

class DocEngineBridge {
  DocEngineBridge._();

  static bool get isReady => WasmBridge.instance.isReady;

  static Future<String> version() async {
    if (WasmBridge.instance.isReady) {
      return WasmBridge.instance.version ?? (await WasmBridge.instance.getVersion());
    }
    await WasmBridge.instance.ensureReady();
    return WasmBridge.instance.getVersion();
  }

  static Future<Uint8List> convertZipToDocx(Uint8List zip, String mainTex) async {
    return WasmBridge.instance.convertZipToDocx(zip, mainTex);
  }
}
