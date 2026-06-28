import 'package:flutter/material.dart';

import '../../../commercial_api.dart';
import '../../../ui/app_components.dart';
import '../../../ui/app_i18n.dart';
import '../../../ui/app_tokens.dart';
import 'automation_request_detail.dart';
import 'automation_status.dart';

class AutomationRequestList extends StatelessWidget {
  final List<AutomationRequest> requests;
  final String accessToken;
  final CommercialApiClient api;
  final String statusFilter;
  final String riskFilter;
  final String sourceFilter;
  final String searchQuery;
  final ValueChanged<String> onStatusFilterChanged;
  final ValueChanged<String> onRiskFilterChanged;
  final ValueChanged<String> onSourceFilterChanged;
  final ValueChanged<String> onSearchChanged;
  final VoidCallback onActionCompleted;

  const AutomationRequestList({
    super.key,
    required this.requests,
    required this.accessToken,
    required this.api,
    required this.statusFilter,
    required this.riskFilter,
    required this.sourceFilter,
    required this.searchQuery,
    required this.onStatusFilterChanged,
    required this.onRiskFilterChanged,
    required this.onSourceFilterChanged,
    required this.onSearchChanged,
    required this.onActionCompleted,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // Filters
        _buildFilters(context),
        const SizedBox(height: AppSpacing.md),

        // List
        Expanded(
          child: requests.isEmpty
              ? const EmptyState(
                  label: 'No requests',
                )
              : ListView.separated(
                  itemCount: requests.length,
                  separatorBuilder: (_, __) => const SizedBox(height: AppSpacing.sm),
                  itemBuilder: (context, index) {
                    final request = requests[index];
                    return _RequestCard(
                      request: request,
                      api: api,
                      accessToken: accessToken,
                      onActionCompleted: onActionCompleted,
                    );
                  },
                ),
        ),
      ],
    );
  }

  Widget _buildFilters(BuildContext context) {
    return Wrap(
      spacing: AppSpacing.md,
      runSpacing: AppSpacing.sm,
      crossAxisAlignment: WrapCrossAlignment.center,
      children: [
        // Status filter
        SizedBox(
          width: 160,
          child: DropdownButtonFormField<String>(
            value: statusFilter,
            decoration: const InputDecoration(
              labelText: 'Status',
              isDense: true,
              contentPadding: EdgeInsets.symmetric(
                horizontal: AppSpacing.sm,
                vertical: AppSpacing.xs,
              ),
            ),
            items: const [
              DropdownMenuItem(value: 'all', child: Text('All')),
              DropdownMenuItem(value: 'triaged', child: Text('Triaged')),
              DropdownMenuItem(value: 'needs_approval', child: Text('Needs Approval')),
              DropdownMenuItem(value: 'queued_for_dev', child: Text('Queued')),
              DropdownMenuItem(value: 'claimed', child: Text('Claimed')),
              DropdownMenuItem(value: 'coding', child: Text('Coding')),
              DropdownMenuItem(value: 'local_failed', child: Text('Local Failed')),
              DropdownMenuItem(value: 'ci_failed', child: Text('CI Failed')),
              DropdownMenuItem(value: 'production_deployed', child: Text('Deployed')),
              DropdownMenuItem(value: 'rejected', child: Text('Rejected')),
            ],
            onChanged: (v) => onStatusFilterChanged(v ?? 'all'),
          ),
        ),

        // Risk filter
        SizedBox(
          width: 140,
          child: DropdownButtonFormField<String>(
            value: riskFilter,
            decoration: const InputDecoration(
              labelText: 'Risk',
              isDense: true,
              contentPadding: EdgeInsets.symmetric(
                horizontal: AppSpacing.sm,
                vertical: AppSpacing.xs,
              ),
            ),
            items: const [
              DropdownMenuItem(value: 'all', child: Text('All')),
              DropdownMenuItem(value: 'low', child: Text('Low')),
              DropdownMenuItem(value: 'medium', child: Text('Medium')),
              DropdownMenuItem(value: 'high', child: Text('High')),
              DropdownMenuItem(value: 'critical', child: Text('Critical')),
            ],
            onChanged: (v) => onRiskFilterChanged(v ?? 'all'),
          ),
        ),

        // Source filter
        SizedBox(
          width: 160,
          child: DropdownButtonFormField<String>(
            value: sourceFilter,
            decoration: const InputDecoration(
              labelText: 'Source',
              isDense: true,
              contentPadding: EdgeInsets.symmetric(
                horizontal: AppSpacing.sm,
                vertical: AppSpacing.xs,
              ),
            ),
            items: const [
              DropdownMenuItem(value: 'all', child: Text('All')),
              DropdownMenuItem(value: 'feedback', child: Text('Feedback')),
              DropdownMenuItem(value: 'github_issue', child: Text('GitHub')),
              DropdownMenuItem(value: 'admin_manual', child: Text('Manual')),
              DropdownMenuItem(value: 'ci_failure', child: Text('CI Failure')),
            ],
            onChanged: (v) => onSourceFilterChanged(v ?? 'all'),
          ),
        ),

        // Search
        Expanded(
          child: TextField(
            decoration: InputDecoration(
              labelText: 'Search',
              hintText: 'Title, ID, or source...',
              isDense: true,
              prefixIcon: const Icon(Icons.search, size: 20),
              contentPadding: const EdgeInsets.symmetric(
                horizontal: AppSpacing.sm,
                vertical: AppSpacing.xs,
              ),
            ),
            onChanged: onSearchChanged,
          ),
        ),
      ],
    );
  }
}

class _RequestCard extends StatelessWidget {
  final AutomationRequest request;
  final CommercialApiClient api;
  final String accessToken;
  final VoidCallback onActionCompleted;

  const _RequestCard({
    required this.request,
    required this.api,
    required this.accessToken,
    required this.onActionCompleted,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final statusInfo = getAutomationStatusInfo(request.status);

    return AppCard(
      child: InkWell(
        onTap: () => _showDetail(context),
        borderRadius: BorderRadius.circular(AppRadius.md),
        child: Padding(
          padding: const EdgeInsets.all(AppSpacing.md),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Header row
              Row(
                children: [
                  // ID
                  Text(
                    request.shortId,
                    style: theme.textTheme.labelMedium?.copyWith(
                      fontFamily: 'monospace',
                      color: theme.colorScheme.primary,
                    ),
                  ),
                  const SizedBox(width: AppSpacing.sm),

                  // Status pill
                  Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: AppSpacing.xs,
                      vertical: 2,
                    ),
                    decoration: BoxDecoration(
                      color: statusInfo.color.withValues(alpha: 0.1),
                      borderRadius: BorderRadius.circular(AppRadius.sm),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(statusInfo.icon, size: 14, color: statusInfo.color),
                        const SizedBox(width: 4),
                        Text(
                          request.statusLabel,
                          style: theme.textTheme.labelSmall?.copyWith(
                            color: statusInfo.color,
                            fontWeight: FontWeight.w500,
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(width: AppSpacing.sm),

                  // Risk
                  _RiskBadge(risk: request.riskLevel),
                  const SizedBox(width: AppSpacing.sm),

                  // Type
                  Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: AppSpacing.xs,
                      vertical: 2,
                    ),
                    decoration: BoxDecoration(
                      color: theme.colorScheme.surfaceContainerHighest,
                      borderRadius: BorderRadius.circular(AppRadius.sm),
                    ),
                    child: Text(
                      request.typeLabel,
                      style: theme.textTheme.labelSmall,
                    ),
                  ),

                  const Spacer(),

                  // PR link if available
                  if (request.prUrl != null && request.prUrl!.isNotEmpty)
                    IconButton(
                      icon: const Icon(Icons.link, size: 18),
                      tooltip: 'View PR',
                      onPressed: () {
                        // TODO: Open PR URL
                      },
                    ),
                ],
              ),
              const SizedBox(height: AppSpacing.sm),

              // Title
              Text(
                request.title,
                style: theme.textTheme.titleSmall,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
              ),
              const SizedBox(height: AppSpacing.sm),

              // Footer
              Row(
                children: [
                  // Source
                  Icon(
                    Icons.source_outlined,
                    size: 14,
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                  const SizedBox(width: 4),
                  Text(
                    request.sourceLabel,
                    style: theme.textTheme.labelSmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                  const SizedBox(width: AppSpacing.md),

                  // Agent
                  if (request.claimedBy != null) ...[
                    Icon(
                      Icons.terminal,
                      size: 14,
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                    const SizedBox(width: 4),
                    Text(
                      request.claimedBy!,
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                    const SizedBox(width: AppSpacing.md),
                  ],

                  const Spacer(),

                  // Updated time
                  Text(
                    _formatTime(request.updatedAt),
                    style: theme.textTheme.labelSmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  void _showDetail(BuildContext context) {
    showDialog(
      context: context,
      builder: (context) => AutomationRequestDetailDialog(
        requestId: request.id,
        api: api,
        accessToken: accessToken,
        onActionCompleted: onActionCompleted,
      ),
    );
  }

  String _formatTime(String isoTime) {
    try {
      final dt = DateTime.parse(isoTime);
      final now = DateTime.now();
      final diff = now.difference(dt);

      if (diff.inMinutes < 1) return 'just now';
      if (diff.inMinutes < 60) return '${diff.inMinutes}m ago';
      if (diff.inHours < 24) return '${diff.inHours}h ago';
      if (diff.inDays < 7) return '${diff.inDays}d ago';

      return '${dt.month}/${dt.day} ${dt.hour}:${dt.minute.toString().padLeft(2, '0')}';
    } catch (_) {
      return isoTime;
    }
  }
}

class _RiskBadge extends StatelessWidget {
  final String risk;

  const _RiskBadge({required this.risk});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final color = switch (risk) {
      'critical' => Colors.red,
      'high' => Colors.orange,
      'medium' => Colors.amber,
      _ => Colors.green,
    };

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: AppSpacing.xs, vertical: 2),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.1),
        borderRadius: BorderRadius.circular(AppRadius.sm),
        border: Border.all(color: color.withValues(alpha: 0.3)),
      ),
      child: Text(
        risk.toUpperCase(),
        style: theme.textTheme.labelSmall?.copyWith(
          color: color,
          fontWeight: FontWeight.w600,
          fontSize: 10,
        ),
      ),
    );
  }
}
