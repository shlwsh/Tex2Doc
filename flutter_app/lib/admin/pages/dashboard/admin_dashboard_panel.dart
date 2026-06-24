import 'package:flutter/material.dart';

import '../../../commercial_api.dart';
import '../../../ui/app_components.dart';
import '../../../ui/app_tokens.dart';

class AdminDashboardPanel extends StatefulWidget {
  final String apiBaseUrl;
  final String accessToken;

  const AdminDashboardPanel({
    super.key,
    required this.apiBaseUrl,
    required this.accessToken,
  });

  @override
  State<AdminDashboardPanel> createState() => _AdminDashboardPanelState();
}

class _AdminDashboardPanelState extends State<AdminDashboardPanel> {
  AdminDashboardSummary? _summary;
  String? _error;
  bool _loading = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    if (_loading) return;
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final summary = await CommercialApiClient(
        widget.apiBaseUrl,
      ).adminDashboard(widget.accessToken);
      if (!mounted) return;
      setState(() => _summary = summary);
    } on Object catch (error) {
      if (!mounted) return;
      setState(() => _error = error.toString());
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final summary = _summary;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Expanded(
              child: AppSectionHeader(
                title: '管理端仪表盘',
                description: '聚合兑换码、反馈、发布与审计模块的运行概览。',
              ),
            ),
            IconButton(
              tooltip: '刷新',
              onPressed: _loading ? null : _load,
              icon: const Icon(Icons.refresh),
            ),
          ],
        ),
        const SizedBox(height: AppSpacing.lg),
        if (_loading && summary == null) const LinearProgressIndicator(),
        if (_error != null) ...[
          Text(_error!, style: theme.textTheme.bodyMedium),
          const SizedBox(height: AppSpacing.md),
        ],
        if (summary != null)
          Wrap(
            spacing: AppSpacing.md,
            runSpacing: AppSpacing.md,
            children: [
              _MetricTile(
                icon: Icons.confirmation_number_outlined,
                label: '兑换码批次',
                value: summary.redeemBatches.toString(),
              ),
              _MetricTile(
                icon: Icons.feedback_outlined,
                label: '待处理反馈',
                value: summary.openFeedback.toString(),
              ),
              _MetricTile(
                icon: Icons.inventory_2_outlined,
                label: '套餐数量',
                value: summary.billingPlans.toString(),
              ),
              _MetricTile(
                icon: Icons.verified_outlined,
                label: '发布通道',
                value: summary.releaseChannels.join(', '),
              ),
            ],
          ),
        const SizedBox(height: AppSpacing.lg),
        if (summary != null)
          _ModuleList(
            modules: summary.modules,
            generatedAt: summary.generatedAt,
          ),
      ],
    );
  }
}

class _MetricTile extends StatelessWidget {
  final IconData icon;
  final String label;
  final String value;

  const _MetricTile({
    required this.icon,
    required this.label,
    required this.value,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return SizedBox(
      width: 220,
      child: Material(
        color: theme.colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(AppRadius.md),
        child: Padding(
          padding: const EdgeInsets.all(AppSpacing.md),
          child: Row(
            children: [
              Icon(icon, color: theme.colorScheme.primary),
              const SizedBox(width: AppSpacing.sm),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(label, style: theme.textTheme.labelMedium),
                    const SizedBox(height: AppSpacing.xs),
                    Text(value, style: theme.textTheme.titleMedium),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _ModuleList extends StatelessWidget {
  final List<String> modules;
  final String generatedAt;

  const _ModuleList({required this.modules, required this.generatedAt});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('管理模块', style: theme.textTheme.titleSmall),
        const SizedBox(height: AppSpacing.sm),
        Wrap(
          spacing: AppSpacing.sm,
          runSpacing: AppSpacing.sm,
          children: modules
              .map((module) => Chip(label: Text(module)))
              .toList(growable: false),
        ),
        const SizedBox(height: AppSpacing.sm),
        Text('更新时间：$generatedAt', style: theme.textTheme.bodySmall),
      ],
    );
  }
}
