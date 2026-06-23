// lib/bridge_stub.dart
// ------------------------------------------------------------
// 非 web 平台桥接：委托给 native_bridge.dart（doc_engine.dll/.dylib/.so）
//
// 暴露统一外观 `DocEngineBridge`，与 web 端同名同签名。
// ------------------------------------------------------------
import 'dart:typed_data';

import 'native_bridge.dart';

export 'native_bridge.dart' show NativeBridgeException;

class DocEngineBridge {
  DocEngineBridge._();

  static bool get isReady {
    if (NativeBridge.instance.isReady) return true;
    return false;
  }

  static Future<String> version() async {
    if (NativeBridge.instance.isReady) {
      return NativeBridge.instance.version ?? 'unknown';
    }
    await NativeBridge.instance.ensureReady();
    return NativeBridge.instance.version ?? 'unknown';
  }

  static Future<Uint8List> convertZipToDocx(Uint8List zip, String mainTex) async {
    final r = await NativeBridge.instance.convertZipToDocx(zip, mainTex);
    return r.docx;
  }
}
