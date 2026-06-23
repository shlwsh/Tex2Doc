// lib/file_web_utils_io.dart
// ------------------------------------------------------------
// 桌面端文件选择器 + Blob 下载（使用 file_picker 包 + dart:io）
//
// pickZipFile  → 使用 file_picker 弹出系统文件选择框
// downloadBlob → 使用 dart:io 写入临时文件后用系统默认程序打开
// ------------------------------------------------------------
import 'dart:io';
import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';

// Opens the native OS file picker filtered to ZIP files.
Future<(Uint8List bytes, String fileName)?> pickZipFile() async {
  try {
    final result = await FilePicker.platform.pickFiles(
      type: FileType.custom,
      allowedExtensions: ['zip'],
      allowMultiple: false,
      withData: true,
    );

    if (result == null || result.files.isEmpty) return null;

    final file = result.files.first;
    final bytes = file.bytes;
    if (bytes == null) {
      // Fallback: read from path (for desktop paths)
      if (file.path != null) {
        final f = File(file.path!);
        final data = await f.readAsBytes();
        return (data, file.name);
      }
      return null;
    }

    return (bytes, file.name);
  } on Object catch (e) {
    // ignore: avoid_print
    print('[doc-engine] 文件选择失败: $e');
    return null;
  }
}

// Writes DOCX bytes to a temp file and opens it with the OS default handler.
void downloadBlob(Uint8List bytes, String filename) {
  try {
    final tempDir = Directory.systemTemp;
    final file = File('${tempDir.path}/$filename');
    file.writeAsBytesSync(bytes);
    // ignore: avoid_print
    print('[doc-engine] 下载: ${file.path}');
    Process.runSync('cmd', ['/c', 'start', '', file.path]);
  } on Object catch (e) {
    // ignore: avoid_print
    print('[doc-engine] 下载失败: $e');
  }
}
