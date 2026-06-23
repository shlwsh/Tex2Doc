// lib/logger_stub.dart
// ------------------------------------------------------------
// 桌面端日志存根（Web 由 logger_web.dart 覆盖）
// ------------------------------------------------------------
import 'dart:async';

class LogEntry {
  final DateTime time;
  final String level;
  final String tag;
  final String message;
  const LogEntry({
    required this.time,
    required this.level,
    required this.tag,
    required this.message,
  });
}

class DocLogger {
  DocLogger._();
  static final DocLogger instance = DocLogger._();
  Stream<LogEntry> get stream => const Stream.empty();
  List<LogEntry> get entries => const [];
  void d(String tag, String message) {}
  void i(String tag, String message) {}
  void w(String tag, String message) {}
  void e(String tag, String message) {}
  void dispose() {}
}

class LogTags {
  static const String app     = 'App';
  static const String engine  = 'Engine';
  static const String convert = 'Convert';
  static const String file    = 'File';
  static const String ui      = 'UI';
}
