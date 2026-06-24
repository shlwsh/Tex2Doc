import 'dart:async';

import 'package:flutter/material.dart';

import '../commercial_api.dart';
import 'app_components.dart';
import 'app_i18n.dart';
import 'app_tokens.dart';

class ConvertRecordsPanel extends StatefulWidget {
  final String apiBaseUrl;
  final String accessToken;

  const ConvertRecordsPanel({
    super.key,
    required this.apiBaseUrl,
    required this.accessToken,
  });

  @override
  State<ConvertRecordsPanel> createState() => _ConvertRecordsPanelState();
}

class _ConvertRecordsPanelState extends State<ConvertRecordsPanel> {
  List<ConversionJob> _records = const [];
  bool _busy = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    unawaited(_load());
  }

  Future<void> _load() async {
    if (_busy) return;
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final client = CommercialApiClient(widget.apiBaseUrl);
      final records = await client.conversions(widget.accessToken);
      if (!mounted) return;
      setState(() {
        _records = records;
        _busy = false;
      });
    } on Object catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _busy = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(
              child: AppSectionHeader(
                title: strings.t('nav.convertRecords'),
                description: strings.t('convert.records'),
              ),
            ),
            OutlinedButton.icon(
              onPressed: _busy ? null : _load,
              icon: const Icon(Icons.refresh, size: 18),
              label: Text(strings.t('common.refresh')),
            ),
          ],
        ),
        const SizedBox(height: AppSpacing.lg),
        if (_busy)
          LoadingState(label: strings.t('common.loading'))
        else if (_error != null)
          ErrorState(message: _error!)
        else if (_records.isEmpty)
          EmptyState(label: strings.t('empty.noData'))
        else
          _RecordsTable(records: _records, accessToken: widget.accessToken),
      ],
    );
  }
}

class _RecordsTable extends StatelessWidget {
  final List<ConversionJob> records;
  final String accessToken;

  const _RecordsTable({required this.records, required this.accessToken});

  Color _statusColor(ConversionStatus status) {
    return switch (status) {
      ConversionStatus.completed => Colors.green,
      ConversionStatus.failed => Colors.red,
      ConversionStatus.expired => Colors.grey,
      ConversionStatus.queued ||
      ConversionStatus.pending ||
      ConversionStatus.normalizing ||
      ConversionStatus.detecting ||
      ConversionStatus.analyzing ||
      ConversionStatus.compiling ||
      ConversionStatus.rendering ||
      ConversionStatus.verifying ||
      ConversionStatus.processing =>
        Colors.blue,
    };
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return AppCard(
      padding: const EdgeInsets.all(AppSpacing.md),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: DataTable(
          headingRowColor: WidgetStatePropertyAll(
            theme.colorScheme.surfaceContainerHighest,
          ),
          columns: [
            DataColumn(label: Text('Job ID', style: theme.textTheme.labelLarge)),
            DataColumn(label: Text('Main File', style: theme.textTheme.labelLarge)),
            DataColumn(label: Text('Profile', style: theme.textTheme.labelLarge)),
            DataColumn(label: Text('Quality', style: theme.textTheme.labelLarge)),
            DataColumn(label: Text('Status', style: theme.textTheme.labelLarge)),
            DataColumn(label: Text('Created', style: theme.textTheme.labelLarge)),
            DataColumn(label: Text('Files', style: theme.textTheme.labelLarge)),
          ],
          rows: [
            for (final job in records)
              DataRow(cells: [
                DataCell(Text(job.jobId, style: theme.textTheme.bodySmall)),
                DataCell(Text(job.mainTex ?? '-', style: theme.textTheme.bodySmall)),
                DataCell(Text(job.profile ?? '-', style: theme.textTheme.bodySmall)),
                DataCell(Text(job.quality ?? '-', style: theme.textTheme.bodySmall)),
                DataCell(_StatusChip(
                  status: job.status.name,
                  color: _statusColor(job.status),
                )),
                DataCell(Text(
                  _formatDate(job.createdAt),
                  style: theme.textTheme.bodySmall,
                )),
                DataCell(_FileButtons(job: job, accessToken: accessToken)),
              ]),
          ],
        ),
      ),
    );
  }

  String _formatDate(String? iso) {
    if (iso == null) return '-';
    try {
      final dt = DateTime.parse(iso);
      return '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')} '
          '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
    } catch (_) {
      return iso;
    }
  }
}

class _StatusChip extends StatelessWidget {
  final String status;
  final Color color;

  const _StatusChip({required this.status, required this.color});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(
        horizontal: AppSpacing.sm,
        vertical: 2,
      ),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.12),
        borderRadius: BorderRadius.circular(AppRadius.sm),
      ),
      child: Text(
        status,
        style: TextStyle(color: color, fontSize: 12),
      ),
    );
  }
}

class _FileButtons extends StatelessWidget {
  final ConversionJob job;
  final String accessToken;

  const _FileButtons({required this.job, required this.accessToken});

  @override
  Widget build(BuildContext context) {
    final storage = job.storageInfo;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        if (storage?.hasDocx ?? false)
          Tooltip(
            message: 'Download DOCX',
            child: IconButton(
              icon: const Icon(Icons.description, size: 18),
              onPressed: () => _download(context, 'docx'),
              color: Colors.blue,
              visualDensity: VisualDensity.compact,
            ),
          ),
        if (storage?.hasZip ?? false)
          Tooltip(
            message: 'Download ZIP',
            child: IconButton(
              icon: const Icon(Icons.archive, size: 18),
              onPressed: () => _download(context, 'zip'),
              color: Colors.orange,
              visualDensity: VisualDensity.compact,
            ),
          ),
        if (storage?.hasLog ?? false)
          Tooltip(
            message: 'Download Log',
            child: IconButton(
              icon: const Icon(Icons.article, size: 18),
              onPressed: () => _download(context, 'log'),
              color: Colors.green,
              visualDensity: VisualDensity.compact,
            ),
          ),
        if (storage == null || !storage.hasAny)
          Text('-', style: Theme.of(context).textTheme.bodySmall?.copyWith(color: Colors.grey[400])),
      ],
    );
  }

  void _download(BuildContext context, String type) async {
    final panel = context.findAncestorWidgetOfExactType<ConvertRecordsPanel>();
    final api = CommercialApiClient(
      panel?.apiBaseUrl ?? '',
    );
    try {
      List<int> bytes;
      String filename;
      switch (type) {
        case 'docx':
          bytes = await api.downloadConversionDocx(accessToken: accessToken, jobId: job.jobId);
          filename = '${job.jobId}.docx';
          break;
        case 'zip':
          bytes = await api.downloadConversionZip(accessToken: accessToken, jobId: job.jobId);
          filename = '${job.jobId}.zip';
          break;
        case 'log':
          bytes = await api.downloadConversionLog(accessToken: accessToken, jobId: job.jobId);
          filename = '${job.jobId}.log';
          break;
        default:
          return;
      }
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Downloaded $filename (${bytes.length} bytes)')),
        );
      }
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Download failed: $e')),
        );
      }
    }
  }
}
