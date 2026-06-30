// lib/file_web_stub.dart
// ------------------------------------------------------------
// 桌面端存根
// ------------------------------------------------------------
import 'dart:typed_data';

Future<(Uint8List bytes, String fileName)?> pickZipFile() async => null;
void downloadBlob(Uint8List bytes, String filename) {}
void openExternalUrl(String url) {}
String saveDocxFile(Uint8List bytes, String suggestedName) => suggestedName;
