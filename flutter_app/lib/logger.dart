// lib/logger.dart
// ------------------------------------------------------------
// 条件导入入口（Web = logger_web, Desktop = logger_stub）
// ------------------------------------------------------------
export 'logger_stub.dart'
    if (dart.library.js_interop) 'logger_web.dart';
