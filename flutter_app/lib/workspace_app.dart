// lib/workspace_app.dart
// ------------------------------------------------------------
// Doc-engine App 框架（Web + Desktop 共享）
// ------------------------------------------------------------
import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';

import 'bridge.dart';
import 'file_web_stub.dart'
    if (dart.library.js_interop) 'file_web_utils_web.dart';
import 'logger.dart';

class DocEngineApp extends StatelessWidget {
  final bool isWeb;
  const DocEngineApp({super.key, required this.isWeb});

  @override
  Widget build(BuildContext context) {
    final platform = isWeb ? 'Web' : 'Desktop';
    DocLogger.instance.i(LogTags.app, 'DocEngineApp build, platform=$platform');
    return MaterialApp(
      title: 'Doc-engine · LaTeX → DOCX',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        useMaterial3: true,
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xFF1565C0),
          brightness: Brightness.light,
        ),
      ),
      darkTheme: ThemeData(
        useMaterial3: true,
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xFF1565C0),
          brightness: Brightness.dark,
        ),
      ),
      home: Scaffold(
        appBar: AppBar(
          title: const Text('Doc-engine · LaTeX → DOCX'),
          actions: [
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              child: Center(
                child: Text('Platform: $platform', style: Theme.of(context).textTheme.bodySmall),
              ),
            ),
          ],
        ),
        body: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 720),
            child: SingleChildScrollView(
              padding: const EdgeInsets.all(24),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  _StatusCard(),
                  const SizedBox(height: 16),
                  _ConvertCard(),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _StatusCard extends StatefulWidget {
  @override
  State<_StatusCard> createState() => _StatusCardState();
}

class _StatusCardState extends State<_StatusCard> {
  String? _version;
  String? _error;

  @override
  void initState() {
    super.initState();
    _loadVersion();
  }

  Future<void> _loadVersion() async {
    try {
      final v = await DocEngineFacade.version();
      if (!mounted) return;
      DocLogger.instance.i(LogTags.engine, '引擎版本: $v');
      setState(() => _version = v);
    } on Object catch (e) {
      DocLogger.instance.e(LogTags.engine, '引擎初始化失败: $e');
      if (!mounted) return;
      setState(() => _error = e.toString());
    }
  }

  @override
  Widget build(BuildContext context) {
    return Card(
      key: const ValueKey('status-card'),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          children: [
            Icon(
              _error != null ? Icons.error_outline : Icons.check_circle,
              color: _error != null ? Colors.red : Colors.green,
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    _error != null ? '引擎初始化失败' : '引擎已就绪',
                    style: const TextStyle(fontWeight: FontWeight.w600),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    _error ?? 'Version: ${_version ?? "loading…"}',
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// ------------------------------------------------------------
// _ConvertCard — 完整文件上传 + 转换 UI
//
// 布局参照 Chrome 扩展 popup（extension/popup/）设计：
//   1. 文件选择区（按钮 + 文件名 + 大小）
//   2. 主 .tex 路径输入框
//   3. 状态条（就绪 / 加载中 / 错误，色码区分）
//   4. 转换按钮（disabled 直到文件就绪）
//   5. 结果卡片（下载按钮 + 耗时）
//   6. 错误卡片
// ------------------------------------------------------------

enum _ConvertState { idle, converting, success, error }

class _ConvertCard extends StatefulWidget {
  @override
  State<_ConvertCard> createState() => _ConvertCardState();
}

class _ConvertCardState extends State<_ConvertCard> {
  // 文件选择
  Uint8List? _zipBytes;
  String? _zipFileName;
  int? _zipSizeBytes;

  // 转换结果
  Uint8List? _docxBytes;
  int? _elapsedMs;

  // UI 状态
  _ConvertState _state = _ConvertState.idle;
  String? _statusText;
  String? _errorText;

  // 主 .tex 路径
  final _mainTexController = TextEditingController(text: 'main-jos.tex');

  @override
  void dispose() {
    _mainTexController.dispose();
    super.dispose();
  }

  // ---------- 文件选择 ----------
  Future<void> _pickFile() async {
    DocLogger.instance.i(LogTags.file, '点击文件选择按钮');
    final result = await pickZipFile();
    if (result == null) {
      DocLogger.instance.w(LogTags.file, '文件选择取消或失败');
      return;
    }

    final (bytes, fileName) = result;
    DocLogger.instance.i(LogTags.file, '文件已选: $fileName, ${(bytes.length / 1024).toStringAsFixed(1)} KB');

    // 5 MB 上限（与 Chrome 扩展一致）
    final sizeMB = bytes.length / (1024 * 1024);
    if (sizeMB >= 5) {
      DocLogger.instance.w(LogTags.file, '文件过大: ${sizeMB.toStringAsFixed(1)} MB >= 5 MB');
      setState(() {
        _state = _ConvertState.error;
        _errorText = '文件 ${sizeMB.toStringAsFixed(1)} MB，超过 5 MB 上限。\n请使用 Doc-engine 桌面 App。';
        _zipBytes = null;
        _zipFileName = null;
        _zipSizeBytes = null;
      });
      return;
    }

    DocLogger.instance.i(LogTags.file, '文件大小合法: ${sizeMB.toStringAsFixed(2)} MB');
    setState(() {
      _zipBytes = bytes;
      _zipFileName = fileName;
      _zipSizeBytes = bytes.length;
      _state = _ConvertState.idle;
      _statusText = null;
      _errorText = null;
    });
  }

  // ---------- 转换 ----------
  Future<void> _startConvert() async {
    if (_zipBytes == null) return;

    final mainTex = _mainTexController.text.trim().isEmpty
        ? 'main-jos.tex'
        : _mainTexController.text.trim();

    DocLogger.instance.i(LogTags.convert, '开始转换: 文件=$_zipFileName, 主文件=$mainTex');
    setState(() {
      _state = _ConvertState.converting;
      _statusText = '正在转换…';
      _errorText = null;
    });

    try {
      final t0 = DateTime.now();
      final docx = await DocEngineFacade.convertZipToDocx(_zipBytes!, mainTex);
      if (!mounted) return;

      _elapsedMs = DateTime.now().difference(t0).inMilliseconds;
      _docxBytes = docx;

      // 验证 DOCX 魔数
      if (docx.length < 4 * 1024) {
        throw Exception('docx 过小：${docx.length} bytes');
      }
      if (docx[0] != 0x50 || docx[1] != 0x4B) {
        throw Exception('docx 头部非 ZIP（PK\\x03\\x04）');
      }

      DocLogger.instance.i(LogTags.convert,
          '转换成功: ${(docx.length / 1024).toStringAsFixed(1)} KB, 耗时 ${_elapsedMs}ms');
      setState(() {
        _state = _ConvertState.success;
        _statusText = '完成 ${(docx.length / 1024).toStringAsFixed(1)} KB（${_elapsedMs}ms）';
      });
    } on Object catch (e) {
      DocLogger.instance.e(LogTags.convert, '转换失败: $e');
      if (!mounted) return;
      setState(() {
        _state = _ConvertState.error;
        _errorText = e.toString();
      });
    }
  }

  // ---------- 下载 ----------
  void _downloadDocx() {
    if (_docxBytes == null) return;
    final base = _zipFileName?.replaceAll(RegExp(r'\.[^.]+$'), '') ?? 'output';
    DocLogger.instance.i(LogTags.file, '触发下载: $base.docx');
    downloadBlob(_docxBytes!, '$base.docx');
  }

  // ---------- UI ----------
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Card(
      key: const ValueKey('convert-card'),
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // 标题
            Text(
              'LaTeX → DOCX 转换',
              style: theme.textTheme.titleMedium?.copyWith(
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: 16),

            // ---- 文件选择 ----
            _FilePickSection(
              fileName: _zipFileName,
              sizeBytes: _zipSizeBytes,
              onPick: _pickFile,
            ),
            const SizedBox(height: 16),

            // ---- 主 .tex 路径 ----
            TextField(
              controller: _mainTexController,
              decoration: InputDecoration(
                labelText: '主 .tex 路径（zip 内）',
                hintText: 'main-jos.tex',
                border: const OutlineInputBorder(),
                isDense: true,
              ),
              style: theme.textTheme.bodyMedium,
            ),
            const SizedBox(height: 16),

            // ---- 状态条 ----
            _StatusBar(
              state: _state,
              text: _statusText ?? _statusText,
            ),

            // ---- 错误卡片 ----
            if (_state == _ConvertState.error && _errorText != null) ...[
              const SizedBox(height: 12),
              _ErrorCard(message: _errorText!),
            ],

            // ---- 转换按钮 ----
            const SizedBox(height: 12),
            FilledButton.icon(
              key: const ValueKey('convert-btn'),
              onPressed: _state == _ConvertState.converting || _zipBytes == null
                  ? null
                  : _startConvert,
              icon: _state == _ConvertState.converting
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(
                        strokeWidth: 2,
                        color: Colors.white,
                      ),
                    )
                  : const Icon(Icons.play_arrow),
              label: Text(
                _state == _ConvertState.converting ? '转换中…' : '开始转换',
              ),
            ),

            // ---- 结果卡片 ----
            if (_state == _ConvertState.success && _docxBytes != null) ...[
              const SizedBox(height: 12),
              _ResultCard(
                docxBytes: _docxBytes!.length,
                elapsedMs: _elapsedMs ?? 0,
                onDownload: _downloadDocx,
              ),
            ],
          ],
        ),
      ),
    );
  }
}

// ---- 子组件 ----

class _FilePickSection extends StatelessWidget {
  final String? fileName;
  final int? sizeBytes;
  final VoidCallback onPick;

  const _FilePickSection({
    required this.fileName,
    required this.sizeBytes,
    required this.onPick,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final hasFile = fileName != null;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        OutlinedButton.icon(
          onPressed: onPick,
          icon: const Icon(Icons.upload_file),
          label: const Text('选择 .zip 文件'),
          style: OutlinedButton.styleFrom(
            padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 14),
          ),
        ),
        const SizedBox(height: 6),
        Text(
          hasFile
              ? '$fileName  (${(sizeBytes! / (1024 * 1024)).toStringAsFixed(2)} MB)'
              : '未选择文件',
          style: theme.textTheme.bodySmall?.copyWith(
            color: hasFile ? colorScheme.primary : theme.textTheme.bodySmall?.color,
            fontStyle: hasFile ? null : FontStyle.italic,
          ),
        ),
      ],
    );
  }
}

class _StatusBar extends StatelessWidget {
  final _ConvertState state;
  final String? text;

  const _StatusBar({required this.state, this.text});

  @override
  Widget build(BuildContext context) {
    final (bgColor, fgColor, icon, label) = switch (state) {
      _ConvertState.idle     => (Colors.grey.shade100, Colors.grey.shade700,  Icons.hourglass_empty, text ?? '就绪'),
      _ConvertState.converting=> (Colors.blue.shade50,   Colors.blue.shade700,  Icons.sync,            text ?? '转换中…'),
      _ConvertState.success   => (Colors.green.shade50, Colors.green.shade700, Icons.check_circle,    text ?? '完成'),
      _ConvertState.error     => (Colors.red.shade50,   Colors.red.shade700,   Icons.error,           text ?? '出错'),
    };

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: bgColor,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        children: [
          Icon(icon, size: 16, color: fgColor),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              label,
              style: TextStyle(fontSize: 13, color: fgColor, fontWeight: FontWeight.w500),
            ),
          ),
        ],
      ),
    );
  }
}

class _ResultCard extends StatelessWidget {
  final int docxBytes;
  final int elapsedMs;
  final VoidCallback onDownload;

  const _ResultCard({
    required this.docxBytes,
    required this.elapsedMs,
    required this.onDownload,
  });

  @override
  Widget build(BuildContext context) {
    final successBg = Colors.green.shade50;
    final successFg = Colors.green.shade700;

    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: successBg,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            '产物：${(docxBytes / 1024).toStringAsFixed(1)} KB  耗时 ${elapsedMs}ms',
            style: TextStyle(fontSize: 13, color: successFg, fontWeight: FontWeight.w500),
          ),
          const SizedBox(height: 10),
          FilledButton.tonalIcon(
            onPressed: onDownload,
            icon: const Icon(Icons.download, size: 18),
            label: const Text('下载 .docx'),
          ),
        ],
      ),
    );
  }
}

class _ErrorCard extends StatelessWidget {
  final String message;

  const _ErrorCard({required this.message});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: Colors.red.shade50,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(Icons.warning_amber, size: 16, color: Colors.red.shade700),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              message,
              style: TextStyle(fontSize: 13, color: Colors.red.shade700),
            ),
          ),
        ],
      ),
    );
  }
}
