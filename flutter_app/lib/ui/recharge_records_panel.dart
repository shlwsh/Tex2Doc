import 'dart:async';

import 'package:flutter/material.dart';

import '../commercial_api.dart';
import 'app_components.dart';
import 'app_i18n.dart';
import 'app_tokens.dart';

class RechargeRecordsPanel extends StatefulWidget {
  final String apiBaseUrl;
  final String accessToken;

  const RechargeRecordsPanel({
    super.key,
    required this.apiBaseUrl,
    required this.accessToken,
  });

  @override
  State<RechargeRecordsPanel> createState() => _RechargeRecordsPanelState();
}

class _RechargeRecordsPanelState extends State<RechargeRecordsPanel> {
  List<RechargeRecord> _records = const [];
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
      final records = await client.recharges(widget.accessToken);
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
                title: strings.t('nav.rechargeRecords'),
                description: strings.t('recharge.records'),
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
          _RechargeRecordsTable(records: _records),
      ],
    );
  }
}

class _RechargeRecordsTable extends StatelessWidget {
  final List<RechargeRecord> records;

  const _RechargeRecordsTable({required this.records});

  Color _amountColor(RechargeRecord record) {
    if (record.amountCents > 0) return Colors.green;
    return Colors.orange;
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
            DataColumn(label: Text('ID', style: theme.textTheme.labelLarge)),
            DataColumn(label: Text('Type', style: theme.textTheme.labelLarge)),
            DataColumn(label: Text('Package', style: theme.textTheme.labelLarge)),
            DataColumn(label: Text('Amount', style: theme.textTheme.labelLarge)),
            DataColumn(label: Text('Provider', style: theme.textTheme.labelLarge)),
            DataColumn(label: Text('Status', style: theme.textTheme.labelLarge)),
            DataColumn(label: Text('Created', style: theme.textTheme.labelLarge)),
          ],
          rows: [
            for (final record in records)
              DataRow(cells: [
                DataCell(
                  ConstrainedBox(
                    constraints: const BoxConstraints(maxWidth: 140),
                    child: Text(
                      record.rechargeId,
                      style: theme.textTheme.bodySmall,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                ),
                DataCell(Text(
                  record.rechargeType,
                  style: theme.textTheme.bodySmall,
                )),
                DataCell(Text(
                  '${record.packageId} ×${record.quantity}',
                  style: theme.textTheme.bodySmall,
                )),
                DataCell(Text(
                  _formatAmount(record.amountCents),
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: _amountColor(record),
                  ),
                )),
                DataCell(Text(
                  record.provider,
                  style: theme.textTheme.bodySmall,
                )),
                DataCell(Text(
                  record.status,
                  style: theme.textTheme.bodySmall,
                )),
                DataCell(Text(
                  _formatDate(record.createdAt),
                  style: theme.textTheme.bodySmall,
                )),
              ]),
          ],
        ),
      ),
    );
  }

  String _formatAmount(int cents) {
    final yuan = cents / 100.0;
    return '¥${yuan.toStringAsFixed(2)}';
  }

  String _formatDate(String? iso) {
    if (iso == null) return '-';
    try {
      final dt = DateTime.parse(iso);
      return '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')}';
    } catch (_) {
      return iso;
    }
  }
}
