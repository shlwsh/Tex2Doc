// lib/wasm_bridge.dart
// ------------------------------------------------------------
// Doc-engine WASM 桥接（Flutter Web 端）
//
// 用 dart:js_interop 官方 API（Uint8List ↔ JSUint8Array，零拷贝 / 智能 unwrap）
// 调用 index.html 中预加载并 init 的 window.docEngine.convert_zip_to_docx。
//
// 字节转换路径：
//   Dart Uint8List  →  Uint8ListToJSUint8Array  →  JSUint8Array （零拷贝）
//   JSUint8Array  →  JSUint8ArrayToUint8List  →  Dart Uint8List （零拷贝）
// ------------------------------------------------------------
import 'dart:async';
import 'dart:js_interop';
import 'dart:js_interop_unsafe';
import 'dart:typed_data';

import 'package:web/web.dart' as web;

class WasmBridgeException implements Exception {
  final String message;
  WasmBridgeException(this.message);
  @override
  String toString() => 'WasmBridgeException: $message';
}

class WasmBridge {
  WasmBridge._();
  static final WasmBridge instance = WasmBridge._();

  bool _ready = false;
  String? _cachedVersion;

  String? get version => _cachedVersion;
  bool get isReady => _ready;

  /// 等待 window.docEngine 就绪。
  Future<void> ensureReady({Duration timeout = const Duration(seconds: 30)}) async {
    if (_ready) return;
    if (_hasGlobal('docEngine')) {
      _ready = true;
      return;
    }
    if (_hasGlobal('docEngineError')) {
      throw WasmBridgeException('WASM init 失败：${_readGlobalString('docEngineError')}');
    }
    await _waitForEvent('doc-engine-ready', timeout: timeout);
    _ready = true;
  }

  /// zip 字节流 + 主 .tex 路径 → docx 字节流。
  Future<Uint8List> convertZipToDocx(
    Uint8List zipBytes,
    String mainTexPath, {
    String? optionsJson,
  }) async {
    await ensureReady();
    try {
      // Dart Uint8List → JS Uint8Array（零拷贝）
      final JSAny jsInput = zipBytes.toJS;
      // 拿 window.docEngine
      final JSObject ns;
      try {
        ns = globalContext.getProperty<JSObject>('docEngine'.toJS);
      } on Object {
        throw WasmBridgeException('window.docEngine 不存在');
      }
      // 拿 convert_zip_to_docx 函数
      final JSFunction fn;
      try {
        fn = ns.getProperty<JSFunction>('convert_zip_to_docx'.toJS);
      } on Object {
        throw WasmBridgeException('window.docEngine.convert_zip_to_docx 不存在');
      }
      // 调 fn(this=ns, args)
      final out = fn.callAsFunction(
        ns,
        jsInput,
        mainTexPath.toJS,
        (optionsJson ?? '').toJS,
      );
      if (out == null) {
        throw WasmBridgeException('WASM 返回 null');
      }
      // JS Uint8Array → Dart Uint8List
      return (out as JSUint8Array).toDart;
    } on WasmBridgeException {
      rethrow;
    } on Object catch (e) {
      throw WasmBridgeException('WASM 转换失败：${_formatError(e)}');
    }
  }

  Future<String> getVersion() async {
    await ensureReady();
    final JSObject ns;
    try {
      ns = globalContext.getProperty<JSObject>('docEngine'.toJS);
    } on Object {
      throw WasmBridgeException('window.docEngine 不存在');
    }
    final JSFunction fn;
    try {
      fn = ns.getProperty<JSFunction>('version'.toJS);
    } on Object {
      throw WasmBridgeException('window.docEngine.version 不存在');
    }
    final out = fn.callAsFunction(ns);
    if (out == null) throw WasmBridgeException('version 返回 null');
    return (out as JSString).toDart;
  }

  // ---- helpers ----

  bool _hasGlobal(String key) =>
      globalContext.getProperty<JSAny?>(key.toJS) != null;

  String _readGlobalString(String key) {
    final v = globalContext.getProperty<JSAny?>(key.toJS);
    if (v == null) return '<empty>';
    try {
      return (v as JSString).toDart;
    } catch (_) {
      return v.toString();
    }
  }

  Future<void> _waitForEvent(
    String name, {
    Duration timeout = const Duration(seconds: 30),
  }) async {
    final completer = Completer<void>();
    late web.EventListener listener;
    listener = ((web.Event _) {
      web.window.removeEventListener(name, listener);
      if (!completer.isCompleted) completer.complete();
    }).toJS;
    web.window.addEventListener(name, listener);
    Timer(timeout, () {
      web.window.removeEventListener(name, listener);
      if (!completer.isCompleted) {
        completer.completeError(
          WasmBridgeException('WASM $name 等待超时 (${timeout.inSeconds}s)'),
        );
      }
    });
    return completer.future;
  }

  String _formatError(Object e) {
    try {
      return e.toString();
    } catch (_) {
      return '${e.runtimeType}';
    }
  }
}
