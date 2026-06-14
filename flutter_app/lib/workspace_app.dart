// lib/workspace_app.dart
// ------------------------------------------------------------
// Doc-engine App 框架（Web + Desktop 共享）
// ------------------------------------------------------------
import 'package:flutter/material.dart';

import 'bridge.dart';

class DocEngineApp extends StatelessWidget {
  final bool isWeb;
  const DocEngineApp({super.key, required this.isWeb});

  @override
  Widget build(BuildContext context) {
    final platform = isWeb ? 'Web' : 'Desktop';
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
      setState(() => _version = v);
    } on Object catch (e) {
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

class _ConvertCard extends StatefulWidget {
  @override
  State<_ConvertCard> createState() => _ConvertCardState();
}

class _ConvertCardState extends State<_ConvertCard> {
  String? _status;
  int? _docxBytes;

  Future<void> _onSmoke() async {
    setState(() {
      _status = '正在调用底层引擎…';
      _docxBytes = null;
    });
    try {
      final v = await DocEngineFacade.version();
      setState(() {
        _status = 'OK：$v';
        _docxBytes = 0;
      });
    } on Object catch (e) {
      setState(() => _status = '失败：$e');
    }
  }

  @override
  Widget build(BuildContext context) {
    return Card(
      key: const ValueKey('convert-card'),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text(
              '核心引擎（Web=WASM / Desktop=Native FFI）',
              style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600),
            ),
            const SizedBox(height: 8),
            Text(_status ?? '点击下方按钮触发版本握手', style: Theme.of(context).textTheme.bodySmall),
            const SizedBox(height: 12),
            ElevatedButton.icon(
              key: const ValueKey('smoke-btn'),
              onPressed: _onSmoke,
              icon: const Icon(Icons.power_settings_new),
              label: const Text('握手 / 状态检查'),
            ),
            if (_docxBytes != null && _docxBytes! > 0) ...[
              const SizedBox(height: 8),
              Text('docx bytes: $_docxBytes'),
            ],
          ],
        ),
      ),
    );
  }
}
