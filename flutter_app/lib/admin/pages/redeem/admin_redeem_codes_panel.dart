import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';

import '../../../commercial_api.dart';
import '../../../file_web_stub.dart'
    if (dart.library.js_interop) '../../../file_web_utils_web.dart'
    if (dart.library.io) '../../../file_web_utils_io.dart';
import '../../../ui/app_components.dart';
import '../../../ui/app_i18n.dart';
import '../../../ui/app_tokens.dart';

class AdminRedeemCodesPage extends StatelessWidget {
  final String apiBaseUrl;
  final String adminToken;

  const AdminRedeemCodesPage({
    super.key,
    required this.apiBaseUrl,
    required this.adminToken,
  });

  @override
  Widget build(BuildContext context) {
    return AdminRedeemCodesPanel(
      apiBaseUrl: apiBaseUrl,
      adminToken: adminToken,
    );
  }
}

class AdminRedeemCodesPanel extends StatefulWidget {
  final String apiBaseUrl;
  final String adminToken;

  const AdminRedeemCodesPanel({
    super.key,
    required this.apiBaseUrl,
    required this.adminToken,
  });

  @override
  State<AdminRedeemCodesPanel> createState() => _RedeemCodesPanelState();
}

class _RedeemCodesPanelState extends State<AdminRedeemCodesPanel> {
  final _searchController = TextEditingController();
  final _importController = TextEditingController();

  String? _filterStatus;
  String? _filterBatchId;
  String? _filterPackageId;
  String _searchText = '';

  int _page = 1;
  int _pageSize = 50;
  int _total = 0;
  List<RedeemCodeRecord> _records = [];
  Set<String> _selected = {};

  String? _status;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    unawaited(_load());
  }

  @override
  void dispose() {
    _searchController.dispose();
    _importController.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    if (_busy) return;
    setState(() {
      _busy = true;
      _status = AppStrings.of(context).t('status.working');
    });
    try {
      final client = CommercialApiClient(widget.apiBaseUrl);
      final result = await client.adminListRedeemCodes(
        adminToken: widget.adminToken,
        stockStatus: _filterStatus,
        batchId: _filterBatchId,
        packageId: _filterPackageId,
        search: _searchText.isEmpty ? null : _searchText,
        page: _page,
        pageSize: _pageSize,
      );
      if (!mounted) return;
      setState(() {
        _records = result.records;
        _total = result.total;
        _page = result.page;
        _pageSize = result.pageSize;
        _selected = {};
        _status = null;
      });
    } on Object catch (e) {
      if (!mounted) return;
      setState(() => _status = e.toString());
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _bulkStock() async {
    if (_selected.isEmpty) return;
    final strings = AppStrings.of(context);
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(strings.t('redeemCodes.bulkStock')),
        content: Text(strings.t('redeemCodes.bulkStockConfirm').fill({
          'count': _selected.length.toString(),
        })),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: Text(strings.t('common.cancel')),
          ),
          FilledButton(
            onPressed: () => Navigator.of(ctx).pop(true),
            child: Text(strings.t('common.confirm')),
          ),
        ],
      ),
    );
    if (confirmed != true) return;

    setState(() {
      _busy = true;
      _status = strings.t('status.working');
    });
    try {
      final client = CommercialApiClient(widget.apiBaseUrl);
      final affected = await client.adminBulkStockRedeemCodes(
        adminToken: widget.adminToken,
        codeIds: _selected.toList(),
      );
      if (!mounted) return;
      setState(() {
        _selected = {};
        _status = strings.t('redeemCodes.stockedOk').fill({'count': affected.toString()});
      });
      await _load();
    } on Object catch (e) {
      if (!mounted) return;
      setState(() => _status = e.toString());
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _importRestock() async {
    final strings = AppStrings.of(context);
    final result = await showDialog<String>(
      context: context,
      builder: (ctx) => _RestockImportDialog(
        strings: strings,
        controller: _importController,
      ),
    );
    if (result == null || result.isEmpty) return;

    setState(() {
      _busy = true;
      _status = strings.t('status.working');
    });
    try {
      final client = CommercialApiClient(widget.apiBaseUrl);
      final affected = await client.adminRestockRedeemCodes(
        adminToken: widget.adminToken,
        codes: result,
      );
      if (!mounted) return;
      setState(() {
        _importController.clear();
        _status = strings.t('redeemCodes.restockOk').fill({'count': affected.toString()});
      });
      await _load();
    } on Object catch (e) {
      if (!mounted) return;
      setState(() => _status = e.toString());
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _exportExcel() async {
    setState(() => _status = AppStrings.of(context).t('status.working'));
    try {
      final client = CommercialApiClient(widget.apiBaseUrl);
      final bytes = await client.adminExportRedeemCodesExcel(
        adminToken: widget.adminToken,
        stockStatus: _filterStatus,
        batchId: _filterBatchId,
        packageId: _filterPackageId,
        search: _searchText.isEmpty ? null : _searchText,
      );
      if (!mounted) return;
      downloadBlob(Uint8List.fromList(bytes), 'redeem-codes-list.xlsx');
      setState(() => _status = AppStrings.of(context).t('redeemCodes.exported'));
    } on Object catch (e) {
      if (!mounted) return;
      setState(() => _status = e.toString());
    }
  }

  void _toggleSelectAll() {
    if (_selected.length == _records.length) {
      setState(() => _selected = {});
    } else {
      setState(() => _selected = _records.map((r) => r.redeemId).toSet());
    }
  }

  void _toggleSelect(String id) {
    setState(() {
      if (_selected.contains(id)) {
        _selected.remove(id);
      } else {
        _selected.add(id);
      }
    });
  }

  void _setFilter(String? status) {
    setState(() {
      _filterStatus = status;
      _page = 1;
    });
    unawaited(_load());
  }

  void _goPage(int page) {
    setState(() => _page = page);
    unawaited(_load());
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final theme = Theme.of(context);
    final totalPages = (_total / _pageSize).ceil();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        AppSectionHeader(
          title: strings.t('nav.redeemCodes'),
          description: strings.t('redeemCodes.description'),
        ),
        SizedBox(height: AppSpacing.lg),

        // ─── Filters & actions ───────────────────────────────────────
        _FilterBar(
          searchController: _searchController,
          filterStatus: _filterStatus,
          selectedCount: _selected.length,
          totalCount: _records.length,
          onSearch: (v) {
            _searchText = v;
            _page = 1;
            unawaited(_load());
          },
          onStatusChanged: _setFilter,
          onToggleSelectAll: _toggleSelectAll,
          onBulkStock: _busy || _selected.isEmpty ? null : _bulkStock,
          onImportRestock: _busy ? null : _importRestock,
          onExportExcel: _busy ? null : _exportExcel,
          onRefresh: _busy ? null : () => unawaited(_load()),
        ),

        if (_status != null) ...[
          SizedBox(height: AppSpacing.sm),
          Text(_status!, style: theme.textTheme.bodySmall),
        ],

        SizedBox(height: AppSpacing.lg),

        // ─── Table ───────────────────────────────────────────────
        if (_busy && _records.isEmpty)
          const Expanded(child: Center(child: CircularProgressIndicator()))
        else if (_records.isEmpty)
          Expanded(
            child: Center(
              child: Text(strings.t('empty.noData')),
            ),
          )
        else ...[
          Expanded(
            child: SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: SingleChildScrollView(
                child: DataTable(
                  showCheckboxColumn: true,
                  headingRowColor: WidgetStateProperty.all(
                    theme.colorScheme.primaryContainer.withValues(alpha: 0.3),
                  ),
                  columns: [
                    DataColumn(label: Text(strings.t('redeemCodes.colBatchNo'))),
                    DataColumn(label: Text(strings.t('redeemCodes.colCodePreview'))),
                    DataColumn(label: Text(strings.t('redeemCodes.colPackage'))),
                    DataColumn(label: Text(strings.t('redeemCodes.colStatus'))),
                    DataColumn(label: Text(strings.t('redeemCodes.colStockedAt'))),
                    DataColumn(label: Text(strings.t('redeemCodes.colRedeemedAt'))),
                    DataColumn(label: Text(strings.t('redeemCodes.colRestockedAt'))),
                    DataColumn(label: Text(strings.t('redeemCodes.colCreatedAt'))),
                  ],
                  rows: _records.map((r) {
                    final selected = _selected.contains(r.redeemId);
                    return DataRow(
                      selected: selected,
                      onSelectChanged: (_) => _toggleSelect(r.redeemId),
                      cells: [
                        DataCell(ConstrainedBox(
                          constraints: const BoxConstraints(maxWidth: 140),
                          child: Text(r.batchNo, overflow: TextOverflow.ellipsis),
                        )),
                        DataCell(ConstrainedBox(
                          constraints: const BoxConstraints(maxWidth: 160),
                          child: Text(r.codePreview, overflow: TextOverflow.ellipsis),
                        )),
                        DataCell(ConstrainedBox(
                          constraints: const BoxConstraints(maxWidth: 160),
                          child: Text('${r.packageName} (x${r.quantity})', overflow: TextOverflow.ellipsis),
                        )),
                        DataCell(_StockStatusBadge(stockStatus: r.stockStatus)),
                        DataCell(ConstrainedBox(
                          constraints: const BoxConstraints(maxWidth: 150),
                          child: Text(r.stockedAt ?? '-', overflow: TextOverflow.ellipsis),
                        )),
                        DataCell(ConstrainedBox(
                          constraints: const BoxConstraints(maxWidth: 150),
                          child: Text(r.redeemedAt ?? '-', overflow: TextOverflow.ellipsis),
                        )),
                        DataCell(ConstrainedBox(
                          constraints: const BoxConstraints(maxWidth: 150),
                          child: Text(r.restockedAt ?? '-', overflow: TextOverflow.ellipsis),
                        )),
                        DataCell(ConstrainedBox(
                          constraints: const BoxConstraints(maxWidth: 150),
                          child: Text(r.createdAt, overflow: TextOverflow.ellipsis),
                        )),
                      ],
                    );
                  }).toList(),
                ),
              ),
            ),
          ),

          // ─── Pagination ──────────────────────────────────────────
          SizedBox(height: AppSpacing.md),
          _PaginationBar(
            page: _page,
            totalPages: totalPages,
            total: _total,
            pageSize: _pageSize,
            onPage: _goPage,
            onPageSizeChanged: (size) {
              setState(() {
                _pageSize = size;
                _page = 1;
              });
              unawaited(_load());
            },
          ),
        ],
      ],
    );
  }
}

// ─── Filter bar ────────────────────────────────────────────────────────

class _FilterBar extends StatelessWidget {
  final TextEditingController searchController;
  final String? filterStatus;
  final int selectedCount;
  final int totalCount;
  final ValueChanged<String> onSearch;
  final ValueChanged<String?> onStatusChanged;
  final VoidCallback onToggleSelectAll;
  final VoidCallback? onBulkStock;
  final VoidCallback? onImportRestock;
  final VoidCallback? onExportExcel;
  final VoidCallback? onRefresh;

  const _FilterBar({
    required this.searchController,
    required this.filterStatus,
    required this.selectedCount,
    required this.totalCount,
    required this.onSearch,
    required this.onStatusChanged,
    required this.onToggleSelectAll,
    this.onBulkStock,
    this.onImportRestock,
    this.onExportExcel,
    this.onRefresh,
  });

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final allSelected = selectedCount == totalCount && totalCount > 0;

    return AppCard(
      padding: const EdgeInsets.all(AppSpacing.md),
      child: Wrap(
        spacing: AppSpacing.sm,
        runSpacing: AppSpacing.sm,
        crossAxisAlignment: WrapCrossAlignment.center,
        children: [
          _FilterChip(
            label: strings.t('redeemCodes.statusNew'),
            value: 'new',
            active: filterStatus == 'new',
            onTap: () => onStatusChanged(filterStatus == 'new' ? null : 'new'),
          ),
          _FilterChip(
            label: strings.t('redeemCodes.statusStocked'),
            value: 'stocked',
            active: filterStatus == 'stocked',
            onTap: () => onStatusChanged(filterStatus == 'stocked' ? null : 'stocked'),
            color: Colors.orange,
          ),
          _FilterChip(
            label: strings.t('redeemCodes.statusRedeemed'),
            value: 'redeemed',
            active: filterStatus == 'redeemed',
            onTap: () => onStatusChanged(filterStatus == 'redeemed' ? null : 'redeemed'),
            color: Colors.green,
          ),
          _FilterChip(
            label: strings.t('redeemCodes.statusRestocked'),
            value: 'restocked',
            active: filterStatus == 'restocked',
            onTap: () => onStatusChanged(filterStatus == 'restocked' ? null : 'restocked'),
            color: Colors.purple,
          ),
          SizedBox(width: AppSpacing.md),
          SizedBox(
            width: 220,
            child: TextField(
              controller: searchController,
              decoration: InputDecoration(
                hintText: strings.t('redeemCodes.searchHint'),
                prefixIcon: const Icon(Icons.search, size: 18),
                isDense: true,
                contentPadding: const EdgeInsets.symmetric(vertical: 8),
                border: const OutlineInputBorder(),
              ),
              onSubmitted: onSearch,
            ),
          ),
          if (selectedCount > 0) ...[
            FilledButton.icon(
              onPressed: onBulkStock,
              icon: const Icon(Icons.sell_outlined, size: 16),
              label: Text(strings.t('redeemCodes.bulkStockShort').fill({'count': selectedCount})),
            ),
          ],
          TextButton.icon(
            onPressed: onToggleSelectAll,
            icon: Icon(
              allSelected ? Icons.deselect : Icons.select_all,
              size: 16,
            ),
            label: Text(allSelected
                ? strings.t('redeemCodes.deselectAll')
                : strings.t('redeemCodes.selectAll')),
          ),
          const Spacer(),
          OutlinedButton.icon(
            onPressed: onImportRestock,
            icon: const Icon(Icons.upload_file, size: 16),
            label: Text(strings.t('redeemCodes.importRestock')),
          ),
          FilledButton.tonalIcon(
            onPressed: onExportExcel,
            icon: const Icon(Icons.download, size: 16),
            label: Text(strings.t('redeemCodes.exportExcel')),
          ),
          IconButton(
            onPressed: onRefresh,
            icon: const Icon(Icons.refresh),
            tooltip: strings.t('common.refresh'),
          ),
        ],
      ),
    );
  }
}

// ─── Filter chip (status filter button) ──────────────────────────────

class _FilterChip extends StatelessWidget {
  final String label;
  final String value;
  final bool active;
  final Color? color;
  final VoidCallback? onTap;

  const _FilterChip({
    required this.label,
    required this.value,
    this.active = false,
    this.color,
    this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final c = color ?? Theme.of(context).colorScheme.primary;
    return FilterChip(
      label: Text(label),
      selected: active,
      onSelected: (_) => onTap?.call(),
      selectedColor: c.withValues(alpha: 0.2),
      checkmarkColor: c,
    );
  }
}

// ─── Stock status badge in table ───────────────────────────────────

class _StockStatusBadge extends StatelessWidget {
  final String stockStatus;

  const _StockStatusBadge({required this.stockStatus});

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final (displayLabel, color) = switch (stockStatus) {
      'new' => (strings.t('redeemCodes.statusNew'), Colors.grey),
      'stocked' => (strings.t('redeemCodes.statusStocked'), Colors.orange),
      'redeemed' => (strings.t('redeemCodes.statusRedeemed'), Colors.green),
      'restocked' => (strings.t('redeemCodes.statusRestocked'), Colors.purple),
      _ => (stockStatus, Colors.grey),
    };

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.12),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: color.withValues(alpha: 0.4)),
      ),
      child: Text(
        displayLabel,
        style: TextStyle(color: color, fontSize: 12, fontWeight: FontWeight.w500),
      ),
    );
  }
}

// ─── Pagination bar ────────────────────────────────────────────────

class _PaginationBar extends StatelessWidget {
  final int page;
  final int totalPages;
  final int total;
  final int pageSize;
  final ValueChanged<int> onPage;
  final ValueChanged<int> onPageSizeChanged;

  const _PaginationBar({
    required this.page,
    required this.totalPages,
    required this.total,
    required this.pageSize,
    required this.onPage,
    required this.onPageSizeChanged,
  });

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final theme = Theme.of(context);
    final from = (page - 1) * pageSize + 1;
    final to = page * pageSize < total ? page * pageSize : total;

    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        Text(
          strings.t('redeemCodes.paginationInfo').fill({'from': from.toString(), 'to': to.toString(), 'total': total.toString()}),
          style: theme.textTheme.bodySmall,
        ),
        SizedBox(width: AppSpacing.lg),
        DropdownButton<int>(
          value: pageSize,
          underline: const SizedBox.shrink(),
          items: [20, 50, 100, 200].map((s) {
            return DropdownMenuItem(value: s, child: Text('$s /页'));
          }).toList(),
          onChanged: (v) {
            if (v != null) onPageSizeChanged(v);
          },
        ),
        SizedBox(width: AppSpacing.lg),
        IconButton(
          icon: const Icon(Icons.first_page),
          tooltip: strings.t('redeemCodes.firstPage'),
          onPressed: page > 1 ? () => onPage(1) : null,
        ),
        IconButton(
          icon: const Icon(Icons.chevron_left),
          tooltip: strings.t('redeemCodes.prevPage'),
          onPressed: page > 1 ? () => onPage(page - 1) : null,
        ),
        Text(
          strings.t('redeemCodes.pageOf').fill({'page': page.toString(), 'total': totalPages.toString()}),
          style: theme.textTheme.bodyMedium,
        ),
        IconButton(
          icon: const Icon(Icons.chevron_right),
          tooltip: strings.t('redeemCodes.nextPage'),
          onPressed: page < totalPages ? () => onPage(page + 1) : null,
        ),
        IconButton(
          icon: const Icon(Icons.last_page),
          tooltip: strings.t('redeemCodes.lastPage'),
          onPressed: page < totalPages ? () => onPage(totalPages) : null,
        ),
      ],
    );
  }
}

// ─── Restock import dialog ────────────────────────────────────

class _RestockImportDialog extends StatefulWidget {
  final AppStrings strings;
  final TextEditingController controller;

  const _RestockImportDialog({
    required this.strings,
    required this.controller,
  });

  @override
  State<_RestockImportDialog> createState() => _RestockImportDialogState();
}

class _RestockImportDialogState extends State<_RestockImportDialog> {
  @override
  Widget build(BuildContext context) {
    final strings = widget.strings;

    return AlertDialog(
      title: Text(strings.t('redeemCodes.importRestock')),
      content: SizedBox(
        width: 500,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              strings.t('redeemCodes.importRestockHint'),
              style: Theme.of(context).textTheme.bodySmall,
            ),
            SizedBox(height: AppSpacing.md),
            TextField(
              controller: widget.controller,
              maxLines: 12,
              minLines: 6,
              style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
              decoration: InputDecoration(
                hintText: 'T2DX-XXXX-XXXX-XXXX\nT2DY-XXXX-XXXX-XXXX\n...',
                border: const OutlineInputBorder(),
              ),
            ),
            SizedBox(height: AppSpacing.sm),
            Text(
              strings.t('redeemCodes.importRestockNote'),
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: Theme.of(context).hintColor,
                  ),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(strings.t('common.cancel')),
        ),
        FilledButton(
          onPressed: () {
            Navigator.of(context).pop(widget.controller.text.trim());
          },
          child: Text(strings.t('redeemCodes.restockConfirm')),
        ),
      ],
    );
  }
}
