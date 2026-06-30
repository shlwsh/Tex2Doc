// Quick Assistant Panel - login-free conversion with redeem code activation.

import 'dart:typed_data';

import 'package:flutter/material.dart';

import '../commercial_api.dart';
import '../file_web_stub.dart'
    if (dart.library.js_interop) '../file_web_utils_web.dart'
    if (dart.library.io) '../file_web_utils_io.dart';
import '../bridge.dart';
import 'app_i18n.dart';
import 'quick_session.dart';

enum _ConvertMode { quick, professional }

enum _ConvertState { idle, checking, converting, success, error }

class QuickAssistantPanel extends StatefulWidget {
  final QuickSession? session;
  final String? activationStatus;
  final String? errorText;
  final bool busy;
  final String apiBaseUrl;
  final ValueChanged<String> onActivate;
  final ValueChanged<QuickSession> onSessionUpdated;

  const QuickAssistantPanel({
    super.key,
    required this.session,
    required this.activationStatus,
    required this.errorText,
    required this.busy,
    required this.apiBaseUrl,
    required this.onActivate,
    required this.onSessionUpdated,
  });

  @override
  State<QuickAssistantPanel> createState() => _QuickAssistantPanelState();
}

class _QuickAssistantPanelState extends State<QuickAssistantPanel> {
  final _codeController = TextEditingController();
  final _mainTexController = TextEditingController(text: 'main.tex');
  Uint8List? _zipBytes;
  String? _zipFileName;
  Uint8List? _docxBytes;
  int? _elapsedMs;
  final List<String> _logs = [];
  _ConvertMode _convertMode = _ConvertMode.quick;
  _ConvertState _convertState = _ConvertState.idle;
  String? _statusText;
  String? _convertErrorText;
  bool _canDownload = false;
  String _profile = 'jos';
  String _quality = 'high';

  bool get _isActivated => widget.activationStatus == 'activated' && widget.session != null;

  @override
  void dispose() {
    _codeController.dispose();
    _mainTexController.dispose();
    super.dispose();
  }

  Future<void> _pickFile() async {
    final result = await pickZipFile();
    if (result == null) return;

    final (bytes, fileName) = result;
    final sizeMB = bytes.length / (1024 * 1024);
    if (sizeMB >= 10) {
      _addLog('File exceeds 10 MB limit: ${sizeMB.toStringAsFixed(2)} MB');
      setState(() => _convertErrorText = 'File too large (max 10 MB)');
      return;
    }

    setState(() {
      _zipBytes = bytes;
      _zipFileName = fileName;
      _docxBytes = null;
      _convertState = _ConvertState.idle;
      _canDownload = false;
      _convertErrorText = null;
    });
    _addLog('Selected: $fileName (${sizeMB.toStringAsFixed(2)} MB)');
  }

  Future<void> _doConvert() async {
    if (!_isActivated) return;
    if (_zipBytes == null) {
      setState(() => _convertErrorText = 'Please select a ZIP file first.');
      return;
    }

    final mainTex = _mainTexController.text.trim().isEmpty
        ? 'main.tex'
        : _mainTexController.text.trim();

    if (_convertMode == _ConvertMode.quick) {
      await _doQuickConvert(mainTex);
    } else {
      await _doCloudConvert(mainTex);
    }
  }

  Future<void> _doQuickConvert(String mainTex) async {
    final session = widget.session!;
    final client = CommercialApiClient(session.apiBaseUrl);

    try {
      // Step 1: Check local conversion quota
      setState(() {
        _convertState = _ConvertState.checking;
        _statusText = 'Checking quota...';
        _convertErrorText = null;
      });
      _addLog('Checking local conversion quota...');

      final check = await client.checkLocalConversion(session.accessToken);
      if (!check.allowed) {
        setState(() {
          _convertState = _ConvertState.error;
          _convertErrorText = 'Quota exhausted. Please buy a new code.';
        });
        _addLog('Quota check failed: not allowed');
        return;
      }
      _addLog('Quota check passed: ${check.countBalance} remaining');

      // Step 2: Local conversion
      setState(() {
        _convertState = _ConvertState.converting;
        _statusText = 'Converting (local)...';
      });

      final stopwatch = Stopwatch()..start();
      final docx = await DocEngineFacade.convertZipToDocx(_zipBytes!, mainTex);
      stopwatch.stop();
      _elapsedMs = stopwatch.elapsedMilliseconds;
      _addLog('Local conversion completed in ${_elapsedMs}ms');

      // Step 3: Consume quota
      final consume = await client.consumeLocalConversion(session.accessToken);
      if (!consume.consumed) {
        setState(() {
          _convertState = _ConvertState.error;
          _convertErrorText = 'Quota consumption failed. Please retry.';
        });
        _addLog('Quota consumption failed');
        return;
      }
      _addLog('Quota consumed, new balance: ${consume.balance}');

      // Step 4: Download DOCX
      setState(() {
        _docxBytes = docx;
        _convertState = _ConvertState.success;
        _canDownload = true;
        _statusText = 'Completed in ${_elapsedMs}ms';
      });

      downloadBlob(docx, '${_zipFileName?.replaceAll('.zip', '') ?? 'output'}.docx');
      _addLog('DOCX downloaded');

      // Step 5: Refresh quota
      final usage = await client.usage(session.accessToken);
      widget.onSessionUpdated(session.copyWith(usage: usage));
      _addLog('Quota refreshed: ${usage.countBalance} remaining');
    } catch (e) {
      setState(() {
        _convertState = _ConvertState.error;
        _convertErrorText = e.toString();
      });
      _addLog('Error: $e');
    }
  }

  Future<void> _doCloudConvert(String mainTex) async {
    final session = widget.session!;
    final client = CommercialApiClient(session.apiBaseUrl);

    try {
      setState(() {
        _convertState = _ConvertState.converting;
        _statusText = 'Converting (cloud)...';
        _convertErrorText = null;
      });
      _addLog('Starting cloud conversion: $mainTex');

      // Step 1: Upload ZIP
      _addLog('Uploading ZIP...');
      final upload = await client.uploadProjectZip(
        accessToken: session.accessToken,
        bytes: _zipBytes!,
        fileName: _zipFileName ?? 'project.zip',
      );
      _addLog('Upload complete: ${upload.uploadId}');

      // Step 2: Create conversion job
      _addLog('Creating cloud conversion job...');
      final created = await client.createConversion(
        accessToken: session.accessToken,
        uploadId: upload.uploadId,
        mainTex: mainTex,
        profile: _profile,
        quality: _quality,
      );
      _addLog('Job created: ${created.jobId}');

      // Step 3: Poll for completion
      var job = created;
      for (var attempt = 0; attempt < 120; attempt += 1) {
        if (job.status == ConversionStatus.completed) {
          _addLog('Conversion completed');
          break;
        }
        if (job.status == ConversionStatus.failed || job.status == ConversionStatus.expired) {
          throw Exception('Cloud conversion failed: ${job.error ?? job.errorCode ?? job.status.name}');
        }
        await Future<void>.delayed(const Duration(seconds: 1));
        job = await client.getConversion(
          accessToken: session.accessToken,
          jobId: job.jobId,
        );
        _addLog('Polling: ${job.status.name}');
      }

      if (job.status != ConversionStatus.completed) {
        throw Exception('Cloud conversion timeout');
      }

      // Step 4: Download DOCX
      _addLog('Downloading DOCX...');
      final docx = await client.downloadConversionDocx(
        accessToken: session.accessToken,
        jobId: job.jobId,
      );
      final docxBytes = Uint8List.fromList(docx);

      setState(() {
        _docxBytes = docxBytes;
        _convertState = _ConvertState.success;
        _canDownload = true;
        _statusText = 'Cloud conversion completed (${(docxBytes.length / 1024).toStringAsFixed(1)} KB)';
      });

      downloadBlob(docxBytes, '${_zipFileName?.replaceAll('.zip', '') ?? 'output'}.docx');
      _addLog('DOCX downloaded');

      // Step 5: Refresh usage
      final usage = await client.usage(session.accessToken);
      widget.onSessionUpdated(session.copyWith(usage: usage));
      _addLog('Usage refreshed: cloud conversions used=${usage.cloudConversionsUsed}');
    } catch (e) {
      setState(() {
        _convertState = _ConvertState.error;
        _convertErrorText = e.toString();
      });
      _addLog('Error: $e');
    }
  }

  void _addLog(String message) {
    setState(() {
      _logs.add('[${DateTime.now().toIso8601String().substring(11, 19)}] $message');
      if (_logs.length > 100) _logs.removeAt(0);
    });
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final theme = Theme.of(context);

    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Title
          Text(
            strings.t('quick.title'),
            style: theme.textTheme.headlineSmall?.copyWith(fontWeight: FontWeight.bold),
          ),
          const SizedBox(height: 4),
          Text(strings.t('quick.subtitle'), style: theme.textTheme.bodyMedium),
          const SizedBox(height: 24),

          // Activation Card
          _buildActivationCard(context, strings, theme),
          const SizedBox(height: 24),

          // Quota Summary (if activated)
          if (_isActivated) ...[
            _buildQuotaCard(context, strings, theme),
            const SizedBox(height: 24),
          ],

          // Conversion Card
          _buildConversionCard(context, strings, theme),
          const SizedBox(height: 24),

          // Logs
          _buildLogsCard(context, strings, theme),
        ],
      ),
    );
  }

  Widget _buildActivationCard(BuildContext context, AppStrings strings, ThemeData theme) {
    final isBusy = widget.busy || widget.activationStatus == 'restoring' || widget.activationStatus == 'activating';

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(strings.t('quick.activateTitle'), style: theme.textTheme.titleMedium),
            const SizedBox(height: 16),
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _codeController,
                    decoration: InputDecoration(
                      hintText: strings.t('quick.codeHint'),
                      border: const OutlineInputBorder(),
                      isDense: true,
                    ),
                    enabled: !isBusy,
                    onSubmitted: (_) {
                      if (!isBusy && _codeController.text.isNotEmpty) {
                        widget.onActivate(_codeController.text.trim());
                      }
                    },
                  ),
                ),
                const SizedBox(width: 8),
                FilledButton(
                  onPressed: isBusy
                      ? null
                      : () {
                          if (_codeController.text.isNotEmpty) {
                            widget.onActivate(_codeController.text.trim());
                          }
                        },
                  child: Text(strings.t('quick.activate')),
                ),
              ],
            ),
            const SizedBox(height: 8),
            Row(
              children: [
                TextButton.icon(
                  onPressed: () {
                    // Open purchase URL
                  },
                  icon: const Icon(Icons.shopping_cart, size: 16),
                  label: Text(strings.t('quick.buyCode')),
                ),
                const Spacer(),
                if (widget.activationStatus == 'restoring')
                  Row(
                    children: [
                      const SizedBox(
                        width: 16,
                        height: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      ),
                      const SizedBox(width: 8),
                      Text(strings.t('quick.restoring')),
                    ],
                  )
                else if (widget.activationStatus == 'activating')
                  const Row(
                    children: [
                      SizedBox(
                        width: 16,
                        height: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      ),
                      SizedBox(width: 8),
                      Text('Activating...'),
                    ],
                  )
                else if (_isActivated)
                  Row(
                    children: [
                      Icon(Icons.check_circle, color: Colors.green.shade600, size: 20),
                      const SizedBox(width: 8),
                      Text(
                        strings.t('quick.activated').replaceAll('{remaining}', '${widget.session!.usage.countBalance}'),
                        style: TextStyle(color: Colors.green.shade700),
                      ),
                    ],
                  )
                else if (widget.activationStatus == 'error')
                  Row(
                    children: [
                      Icon(Icons.error, color: Colors.red.shade600, size: 20),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          strings.t('quick.activationError').replaceAll('{error}', widget.errorText ?? 'Unknown error'),
                          style: TextStyle(color: Colors.red.shade700),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ],
                  )
                else
                  Text(strings.t('quick.notActivated'), style: theme.textTheme.bodySmall),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildQuotaCard(BuildContext context, AppStrings strings, ThemeData theme) {
    final usage = widget.session!.usage;
    return Card(
      color: theme.colorScheme.primaryContainer,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          children: [
            Icon(Icons.account_balance_wallet, color: theme.colorScheme.onPrimaryContainer),
            const SizedBox(width: 12),
            Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  strings.t('quick.countCredit').replaceAll('{count}', '${usage.countBalance}'),
                  style: theme.textTheme.titleMedium?.copyWith(
                    color: theme.colorScheme.onPrimaryContainer,
                    fontWeight: FontWeight.bold,
                  ),
                ),
                if (usage.dateValidUntil != null)
                  Text(
                    strings.t('metrics.dateValidUntil').replaceAll('{time}', usage.dateValidUntil!),
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onPrimaryContainer.withValues(alpha: 0.8),
                    ),
                  ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildConversionCard(BuildContext context, AppStrings strings, ThemeData theme) {
    final canConvert = _isActivated && _zipBytes != null && _convertState != _ConvertState.converting && _convertState != _ConvertState.checking;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Mode selector
            SegmentedButton<_ConvertMode>(
              segments: [
                ButtonSegment(value: _ConvertMode.quick, label: Text(strings.t('quick.localMode'))),
                ButtonSegment(value: _ConvertMode.professional, label: Text(strings.t('quick.cloudMode'))),
              ],
              selected: {_convertMode},
              onSelectionChanged: (selection) => setState(() => _convertMode = selection.first),
            ),
            const SizedBox(height: 16),

            // ZIP selection
            Row(
              children: [
                FilledButton.tonalIcon(
                  onPressed: _isActivated ? _pickFile : null,
                  icon: const Icon(Icons.folder_open),
                  label: Text(strings.t('common.upload')),
                ),
                const SizedBox(width: 12),
                if (_zipFileName != null)
                  Expanded(
                    child: Text(
                      _zipFileName!,
                      style: theme.textTheme.bodyMedium,
                      overflow: TextOverflow.ellipsis,
                    ),
                  )
                else
                  Text(strings.t('convert.noFile'), style: theme.textTheme.bodySmall),
              ],
            ),
            const SizedBox(height: 16),

            // Profile & Quality
            Row(
              children: [
                Expanded(
                  child: DropdownButtonFormField<String>(
                    initialValue: _profile,
                    decoration: const InputDecoration(
                      labelText: 'Profile',
                      border: OutlineInputBorder(),
                      isDense: true,
                    ),
                    items: const [
                      DropdownMenuItem(value: 'jos', child: Text('JOS')),
                      DropdownMenuItem(value: 'standard', child: Text('Standard')),
                    ],
                    onChanged: (v) => setState(() => _profile = v ?? 'jos'),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: DropdownButtonFormField<String>(
                    initialValue: _quality,
                    decoration: const InputDecoration(
                      labelText: 'Quality',
                      border: OutlineInputBorder(),
                      isDense: true,
                    ),
                    items: const [
                      DropdownMenuItem(value: 'high', child: Text('High')),
                      DropdownMenuItem(value: 'medium', child: Text('Medium')),
                    ],
                    onChanged: (v) => setState(() => _quality = v ?? 'high'),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),

            // Main TeX input
            TextField(
              controller: _mainTexController,
              decoration: InputDecoration(
                labelText: strings.t('convert.mainTex'),
                hintText: 'main.tex',
                border: const OutlineInputBorder(),
                isDense: true,
              ),
            ),
            const SizedBox(height: 16),

            // Status / Error
            if (_convertErrorText != null)
              Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: Text(_convertErrorText!, style: TextStyle(color: theme.colorScheme.error)),
              ),
            if (_statusText != null && _convertState != _ConvertState.idle)
              Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: Text(_statusText!, style: theme.textTheme.bodySmall),
              ),

            // Convert button
            SizedBox(
              width: double.infinity,
              child: FilledButton.icon(
                onPressed: canConvert ? _doConvert : null,
                icon: _convertState == _ConvertState.converting || _convertState == _ConvertState.checking
                    ? const SizedBox(
                        width: 16,
                        height: 16,
                        child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white),
                      )
                    : const Icon(Icons.play_arrow),
                label: Text(
                  _convertState == _ConvertState.converting
                      ? strings.t('convert.converting')
                      : strings.t('common.convert'),
                ),
              ),
            ),

            // Download button (if success)
            if (_canDownload && _docxBytes != null) ...[
              const SizedBox(height: 8),
              SizedBox(
                width: double.infinity,
                child: OutlinedButton.icon(
                  onPressed: () {
                    downloadBlob(_docxBytes!, '${_zipFileName?.replaceAll('.zip', '') ?? 'output'}.docx');
                  },
                  icon: const Icon(Icons.download),
                  label: Text(strings.t('common.download')),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  Widget _buildLogsCard(BuildContext context, AppStrings strings, ThemeData theme) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Text(strings.t('convert.logs'), style: theme.textTheme.titleMedium),
                const Spacer(),
                TextButton(
                  onPressed: () => setState(() => _logs.clear()),
                  child: const Text('Clear'),
                ),
              ],
            ),
            const Divider(),
            Container(
              height: 200,
              decoration: BoxDecoration(
                color: theme.colorScheme.surfaceContainerHighest,
                borderRadius: BorderRadius.circular(8),
              ),
              child: _logs.isEmpty
                  ? Center(child: Text(strings.t('empty.noData'), style: theme.textTheme.bodySmall))
                  : ListView.builder(
                      padding: const EdgeInsets.all(8),
                      itemCount: _logs.length,
                      itemBuilder: (context, index) {
                        return Text(
                          _logs[index],
                          style: theme.textTheme.bodySmall?.copyWith(
                            fontFamily: 'monospace',
                            fontSize: 11,
                          ),
                        );
                      },
                    ),
            ),
          ],
        ),
      ),
    );
  }
}
