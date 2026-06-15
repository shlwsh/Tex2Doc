// lib/native_bridge.dart
// ------------------------------------------------------------
// Doc-engine 原生桥接（Flutter 桌面端：Windows / macOS / Linux）
//
// 用 dart:ffi 调 crates/native 生成的 cdylib（doc_engine.dll/.dylib/.so）。
// ------------------------------------------------------------
import 'dart:async';
import 'dart:convert';
import 'dart:ffi' as ffi;
import 'dart:io' show Platform;
import 'dart:typed_data';

import 'package:ffi/ffi.dart' as pkg_ffi;

// ---- C 侧函数签名 ----

typedef _VersionNative = ffi.Pointer<ffi.Uint8> Function();
typedef _VersionDart = ffi.Pointer<ffi.Uint8> Function();

typedef _LastErrorNative = ffi.Pointer<ffi.Uint8> Function();
typedef _LastErrorDart = ffi.Pointer<ffi.Uint8> Function();

typedef _FreeNative = ffi.Void Function(ffi.Pointer<ffi.Uint8>);
typedef _FreeDart = void Function(ffi.Pointer<ffi.Uint8>);

typedef _ConvertZipNative = ffi.Int32 Function(
  ffi.Pointer<ffi.Uint8>,
  ffi.IntPtr,
  ffi.Pointer<ffi.Uint8>,
  ffi.IntPtr,
  ffi.Pointer<ffi.Pointer<ffi.Uint8>>,
  ffi.Pointer<ffi.IntPtr>,
  ffi.Pointer<ffi.Pointer<ffi.Uint8>>,
  ffi.Pointer<ffi.IntPtr>,
);

typedef _ConvertZipDart = int Function(
  ffi.Pointer<ffi.Uint8>,
  int,
  ffi.Pointer<ffi.Uint8>,
  int,
  ffi.Pointer<ffi.Pointer<ffi.Uint8>>,
  ffi.Pointer<ffi.IntPtr>,
  ffi.Pointer<ffi.Pointer<ffi.Uint8>>,
  ffi.Pointer<ffi.IntPtr>,
);

class NativeBridgeException implements Exception {
  final String message;
  NativeBridgeException(this.message);
  @override
  String toString() => 'NativeBridgeException: $message';
}

class NativeBridge {
  NativeBridge._();
  static final NativeBridge instance = NativeBridge._();

  ffi.DynamicLibrary? _lib;
  _VersionDart? _versionFn;
  _LastErrorDart? _lastErrorFn;
  _FreeDart? _freeFn;
  _ConvertZipDart? _convertFn;

  bool _ready = false;
  String? _cachedVersion;

  String? get version => _cachedVersion;
  bool get isReady => _ready;

  /// 按平台选择动态库名；优先用 `DOC_ENGINE_LIB` 环境变量覆盖。
  String _libName() {
    final override = Platform.environment['DOC_ENGINE_LIB'];
    if (override != null && override.isNotEmpty) return override;
    if (Platform.isWindows) return 'doc_engine';
    if (Platform.isMacOS) return 'doc_engine';
    if (Platform.isLinux) return 'doc_engine';
    throw NativeBridgeException('不支持的平台：${Platform.operatingSystem}');
  }

  Future<void> ensureReady({String? libPath}) async {
    if (_ready) return;
    final name = libPath ?? _libName();
    try {
      _lib = ffi.DynamicLibrary.open(name);
    } on Object catch (e) {
      throw NativeBridgeException('打开 $name 失败：$e');
    }
    _versionFn = _lib!.lookupFunction<_VersionNative, _VersionDart>('doc_engine_version');
    _lastErrorFn =
        _lib!.lookupFunction<_LastErrorNative, _LastErrorDart>('doc_engine_last_error');
    _freeFn = _lib!.lookupFunction<_FreeNative, _FreeDart>('doc_engine_free');
    _convertFn =
        _lib!.lookupFunction<_ConvertZipNative, _ConvertZipDart>('doc_engine_convert_zip');

    try {
      _cachedVersion = _readCString(_versionFn!());
    } on Object catch (e) {
      throw NativeBridgeException('doc_engine_version 失败：$e');
    }
    _ready = true;
  }

  Future<NativeConvertResult> convertZipToDocx(
    Uint8List zipBytes,
    String mainTexPath,
  ) async {
    await ensureReady();
    final fn = _convertFn!;

    // 1) zip 字节拷到 C 堆
    final zipBuf = pkg_ffi.calloc<ffi.Uint8>(zipBytes.length);
    try {
      final zipPtr = zipBuf.cast<ffi.Uint8>();
      zipPtr.asTypedList(zipBytes.length).setAll(0, zipBytes);

      // 2) main_tex 路径字符串
      final mainBytes = utf8.encode(mainTexPath);
      final mainBuf = pkg_ffi.calloc<ffi.Uint8>(mainBytes.length);
      try {
        mainBuf.asTypedList(mainBytes.length).setAll(0, mainBytes);

        // 3) 输出参数占位（全部用 IntPtr 保持 Native 边界一致）
        final outDocxPtr = pkg_ffi.calloc<ffi.Pointer<ffi.Uint8>>();
        final outDocxLen = pkg_ffi.calloc<ffi.IntPtr>();
        final outWarnPtr = pkg_ffi.calloc<ffi.Pointer<ffi.Uint8>>();
        final outWarnLen = pkg_ffi.calloc<ffi.IntPtr>();
        try {
          final rc = fn(
            zipPtr,
            zipBytes.length,
            mainBuf,
            mainBytes.length,
            outDocxPtr,
            outDocxLen,
            outWarnPtr,
            outWarnLen,
          );
          if (rc != 0) {
            final errPtr = _lastErrorFn!();
            final err = errPtr == ffi.nullptr ? 'unknown' : _readCString(errPtr);
            throw NativeBridgeException('convert_zip 失败 (rc=$rc): $err');
          }

          final docxPtr = outDocxPtr.value;
          final docxLen = outDocxLen.value;
          final warnPtr = outWarnPtr.value;
          final warnLen = outWarnLen.value;

          try {
            final docx = Uint8List.fromList(docxPtr.asTypedList(docxLen));
            final warnings = warnLen > 0
                ? List<String>.from(
                    jsonDecode(utf8.decode(warnPtr.asTypedList(warnLen))) as List)
                : <String>[];
            return NativeConvertResult(docx: docx, warnings: warnings);
          } finally {
            _freeFn!(docxPtr);
            if (warnLen > 0) _freeFn!(warnPtr);
          }
        } finally {
          pkg_ffi.calloc.free(outDocxPtr);
          pkg_ffi.calloc.free(outDocxLen);
          pkg_ffi.calloc.free(outWarnPtr);
          pkg_ffi.calloc.free(outWarnLen);
        }
      } finally {
        pkg_ffi.calloc.free(mainBuf);
      }
    } finally {
      pkg_ffi.calloc.free(zipBuf);
    }
  }

  String _readCString(ffi.Pointer<ffi.Uint8> p) {
    if (p == ffi.nullptr) return '';
    final units = p.cast<ffi.Uint8>();
    final bytes = <int>[];
    for (var i = 0; i < 1024; i++) {
      final b = units.elementAt(i).value;
      if (b == 0) break;
      bytes.add(b);
    }
    return utf8.decode(bytes);
  }
}

class NativeConvertResult {
  final Uint8List docx;
  final List<String> warnings;
  const NativeConvertResult({required this.docx, required this.warnings});
}
