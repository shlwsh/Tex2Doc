import 'package:flutter/material.dart';

import '../../../commercial_api.dart';
import '../../../ui/app_tokens.dart';

class AutomationSummaryCards extends StatelessWidget {
  final AutomationSummary summary;

  const AutomationSummaryCards({super.key, required this.summary});

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        return Wrap(
          spacing: AppSpacing.md,
          runSpacing: AppSpacing.md,
          children: [
            _MetricCard(
              label: 'Pending Approval',
              value: summary.pendingApproval,
              icon: Icons.pending_actions,
              color: Colors.orange,
            ),
            _MetricCard(
              label: 'Waiting Dev',
              value: summary.waitingDev,
              icon: Icons.schedule,
              color: Colors.grey,
            ),
            _MetricCard(
              label: 'In Development',
              value: summary.inDevelopment,
              icon: Icons.code,
              color: Colors.blue,
            ),
            _MetricCard(
              label: 'Local Failed',
              value: summary.localFailed,
              icon: Icons.error_outline,
              color: Colors.red,
            ),
            _MetricCard(
              label: 'CI Failed',
              value: summary.ciFailed,
              icon: Icons.cancel,
              color: Colors.deepOrange,
            ),
            _MetricCard(
              label: 'Deployed',
              value: summary.deployed,
              icon: Icons.check_circle_outline,
              color: Colors.green,
            ),
            _MetricCard(
              label: 'Total',
              value: summary.total,
              icon: Icons.analytics,
              color: Colors.purple,
            ),
          ],
        );
      },
    );
  }
}

class _MetricCard extends StatelessWidget {
  final String label;
  final int value;
  final IconData icon;
  final Color color;

  const _MetricCard({
    required this.label,
    required this.value,
    required this.icon,
    required this.color,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Container(
      constraints: const BoxConstraints(minWidth: 140),
      padding: const EdgeInsets.all(AppSpacing.md),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(AppRadius.md),
        border: Border.all(color: theme.colorScheme.outlineVariant, width: 1),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(icon, size: 18, color: color),
              const SizedBox(width: AppSpacing.xs),
              Text(
                label,
                style: theme.textTheme.labelMedium?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
          const SizedBox(height: AppSpacing.xs),
          Text(
            value.toString(),
            style: theme.textTheme.headlineMedium?.copyWith(
              fontWeight: FontWeight.bold,
              color: color,
            ),
          ),
        ],
      ),
    );
  }
}
