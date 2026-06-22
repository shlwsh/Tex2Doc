// lib/file_web_utils_web.dart
// ------------------------------------------------------------
// Web 文件选择器 + Blob 下载（纯 dart:js_interop，无额外依赖）
//
// 文件选择流程（标准 Flutter Web 模式）：
//   1. Dart 发 'flutter-file-trigger' 事件  →  JS 点击隐藏 <input type=file>
//   2. 用户选文件后 JS 派发 'flutter-file-selected'（含 base64 数据）
//   3. Dart 监听接收，解码 base64 → Uint8List
// ------------------------------------------------------------
import 'dart:async';
import 'dart:convert';
import 'dart:js_interop';
import 'dart:js_interop_unsafe';
import 'dart:typed_data';

import 'package:web/web.dart' as web;

/// 触发浏览器文件选择器（通过 JS 事件桥接 DOM），返回文件字节。
Future<(Uint8List bytes, String fileName)?> pickZipFile() async {
  final completer = Completer<(Uint8List, String)?>();

  late final web.EventListener onSelected;
  late final web.EventListener onError;

  onSelected = ((web.Event event) {
    web.window.removeEventListener('flutter-file-selected', onSelected);
    web.window.removeEventListener('flutter-file-error', onError);
    try {
      final detail = (event as web.CustomEvent).detail as JSObject;
      final name = detail.getProperty<JSString>('name'.toJS).toDart;
      final base64 = detail.getProperty<JSString>('data'.toJS).toDart;
      final bytes = base64Decode(base64);
      // ignore: avoid_print
      print('[doc-engine] 文件已选: $name, ${(bytes.length / 1024).toStringAsFixed(1)} KB');
      completer.complete((Uint8List.fromList(bytes), name));
    } catch (e) {
      // ignore: avoid_print
      print('[doc-engine] 文件数据解析失败: $e');
      completer.complete(null);
    }
  }).toJS;

  onError = ((web.Event event) {
    web.window.removeEventListener('flutter-file-selected', onSelected);
    web.window.removeEventListener('flutter-file-error', onError);
    // ignore: avoid_print
    print('[doc-engine] 文件选择出错');
    completer.complete(null);
  }).toJS;

  web.window.addEventListener('flutter-file-selected', onSelected);
  web.window.addEventListener('flutter-file-error', onError);

  // 发事件让 JS 去点击隐藏 input
  // ignore: avoid_print
  print('[doc-engine] 文件选择器触发');
  web.window.dispatchEvent(web.CustomEvent('flutter-file-trigger'));

  return completer.future;
}

/// 触发浏览器文件下载。
void downloadBlob(Uint8List bytes, String filename) {
  final blob = web.Blob(
    [bytes.toJS].toJS,
    web.BlobPropertyBag(type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'),
  );
  final url = web.URL.createObjectURL(blob);
  final web.HTMLAnchorElement anchor = web.HTMLAnchorElement();
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  web.URL.revokeObjectURL(url);
  // ignore: avoid_print
  print('[doc-engine] 下载触发: $filename (${(bytes.length / 1024).toStringAsFixed(1)} KB)');
}
