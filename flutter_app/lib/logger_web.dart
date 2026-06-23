// lib/logger_web.dart
// ------------------------------------------------------------
// Web 平台日志服务
//
// - 控制台：debugPrint（DevTools Console）
// - 应用内查看：通过 Dart 状态暴露，无持久化文件（Web 安全限制）
// ------------------------------------------------------------
import 'dart:async';
import 'dart:js_interop';

import 'package:flutter/foundation.dart';
import 'package:web/web.dart' as web;

/// 日志级别
enum LogLevel { debug, info, warn, error }

/// 一条日志记录
class LogEntry {
  final DateTime time;
  final LogLevel level;
  final String tag;
  final String message;

  const LogEntry({
    required this.time,
    required this.level,
    required this.tag,
    required this.message,
  });

  String get _levelStr => switch (level) {
        LogLevel.debug => 'D',
        LogLevel.info  => 'I',
        LogLevel.warn  => 'W',
        LogLevel.error => 'E',
      };

  String get _timeStr =>
      '${time.hour.toString().padLeft(2, '0')}:'
      '${time.minute.toString().padLeft(2, '0')}:'
      '${time.second.toString().padLeft(2, '0')}';

  @override
  String toString() =>
      '\x1B[90m[$_timeStr $_levelStr/$tag]\x1B[0m $message';

  Map<String, dynamic> toJson() => {
        'time': time.toIso8601String(),
        'level': level.name,
        'tag': tag,
        'message': message,
      };
}

/// 日志状态（暴露给 UI）
class LogState {
  final List<LogEntry> entries;
  const LogState({this.entries = const []});

  LogState append(LogEntry e) => LogState(entries: [...entries, e]);
}

/// 全局日志服务
class DocLogger {
  DocLogger._();
  static final DocLogger instance = DocLogger._();

  final List<LogEntry> _entries = [];
  final _controller = StreamController<LogEntry>.broadcast();
  static const int _maxEntries = 500;

  Stream<LogEntry> get stream => _controller.stream;
  List<LogEntry> get entries => List.unmodifiable(_entries);

  void _log(LogLevel level, String tag, String message) {
    final entry = LogEntry(
      time: DateTime.now(),
      level: level,
      tag: tag,
      message: message,
    );

    _entries.add(entry);
    if (_entries.length > _maxEntries) {
      _entries.removeAt(0);
    }
    _controller.add(entry);

    // 控制台输出（Flutter DevTools Console）
    debugPrint(entry.toString());

    // 同步写浏览器 Console（不同运行时可见）
    _jsConsole(level, tag, message);
  }

  void _jsConsole(LogLevel level, String tag, String message) {
    final full = '[$tag] $message';
    switch (level) {
      case LogLevel.debug:
      case LogLevel.info:
        web.console.info(full.toJS);
      case LogLevel.warn:
        web.console.warn(full.toJS);
      case LogLevel.error:
        web.console.error(full.toJS);
    }
  }

  void d(String tag, String message) => _log(LogLevel.debug, tag, message);
  void i(String tag, String message) => _log(LogLevel.info,  tag, message);
  void w(String tag, String message) => _log(LogLevel.warn,  tag, message);
  void e(String tag, String message) => _log(LogLevel.error, tag, message);

  void dispose() => _controller.close();
}

/// 便捷前缀常量
class LogTags {
  static const String app     = 'App';
  static const String engine  = 'Engine';
  static const String convert = 'Convert';
  static const String file    = 'File';
  static const String ui      = 'UI';
}
