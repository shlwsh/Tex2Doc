import 'package:flutter/material.dart';

import '../../../commercial_api.dart';
import '../../../ui/app_tokens.dart';
import 'automation_status.dart';

class AutomationRequestDetailDialog extends StatefulWidget {
  final String requestId;
  final CommercialApiClient api;
  final String accessToken;
  final VoidCallback onActionCompleted;

  const AutomationRequestDetailDialog({
    super.key,
    required this.requestId,
    required this.api,
    required this.accessToken,
    required this.onActionCompleted,
  });

  @override
  State<AutomationRequestDetailDialog> createState() =>
      _AutomationRequestDetailDialogState();
}

class _AutomationRequestDetailDialogState
    extends State<AutomationRequestDetailDialog> {
  AutomationRequest? _request;
  List<AutomationEvent> _events = [];
  bool _loading = true;
  String? _error;
  bool _actionInProgress = false;

  @override
  void initState() {
    super.initState();
    _loadData();
  }

  Future<void> _loadData() async {
    setState(() {
      _loading = true;
      _error = null;
    });

    try {
      final results = await Future.wait([
        widget.api.adminAutomationRequest(widget.accessToken, widget.requestId),
        widget.api.adminAutomationEvents(widget.accessToken, widget.requestId),
      ]);

      if (!mounted) return;
      setState(() {
        _request = results[0] as AutomationRequest;
        _events = results[1] as List<AutomationEvent>;
        _loading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _loading = false;
        _error = e.toString();
      });
    }
  }

  Future<void> _approve() async {
    if (!canAutoApprove(_request!.riskLevel)) {
      _showError('High/Critical risk requests cannot be auto-approved');
      return;
    }

    final confirmed = await _showConfirmDialog(
      'Approve Request',
      'This will queue the request for automated development. Continue?',
    );

    if (confirmed != true) return;

    await _doAction(() async {
      await widget.api.adminAutomationApprove(
        widget.accessToken,
        widget.requestId,
      );
    });
  }

  Future<void> _reject() async {
    final reason = await _showReasonDialog();
    if (reason == null || reason.isEmpty) return;

    await _doAction(() async {
      await widget.api.adminAutomationReject(
        widget.accessToken,
        widget.requestId,
        reason,
      );
    });
  }

  Future<void> _retry() async {
    final confirmed = await _showConfirmDialog(
      'Retry Request',
      'This will retry the failed step. Continue?',
    );

    if (confirmed != true) return;

    await _doAction(() async {
      await widget.api.adminAutomationRetry(
        widget.accessToken,
        widget.requestId,
      );
    });
  }

  Future<void> _escalate() async {
    final assignee = await _showAssigneeDialog();
    if (assignee == null || assignee.isEmpty) return;

    await _doAction(() async {
      await widget.api.adminAutomationEscalate(
        widget.accessToken,
        widget.requestId,
        assignee,
      );
    });
  }

  Future<bool?> _showConfirmDialog(String title, String message) {
    return showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(title),
        content: Text(message),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('Confirm'),
          ),
        ],
      ),
    );
  }

  Future<String?> _showReasonDialog() {
    final controller = TextEditingController();
    return showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Reject Request'),
        content: TextField(
          controller: controller,
          decoration: const InputDecoration(
            labelText: 'Reason (required)',
            hintText: 'Enter rejection reason...',
          ),
          maxLines: 3,
          autofocus: true,
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, controller.text),
            child: const Text('Reject'),
          ),
        ],
      ),
    );
  }

  Future<String?> _showAssigneeDialog() {
    final controller = TextEditingController();
    return showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Escalate to Human'),
        content: TextField(
          controller: controller,
          decoration: const InputDecoration(
            labelText: 'Assignee (required)',
            hintText: 'Enter assignee name or ID...',
          ),
          autofocus: true,
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, controller.text),
            child: const Text('Escalate'),
          ),
        ],
      ),
    );
  }

  void _showError(String message) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(message), backgroundColor: Colors.red),
    );
  }

  Future<void> _doAction(Future<void> Function() action) async {
    setState(() => _actionInProgress = true);

    try {
      await action();
      widget.onActionCompleted();
      _loadData();
    } catch (e) {
      if (mounted) {
        _showError(e.toString());
      }
    } finally {
      if (mounted) {
        setState(() => _actionInProgress = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Dialog(
      child: Container(
        constraints: const BoxConstraints(maxWidth: 800, maxHeight: 700),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // Header
            Container(
              padding: const EdgeInsets.all(AppSpacing.md),
              decoration: BoxDecoration(
                color: theme.colorScheme.surfaceContainerHighest,
                borderRadius: const BorderRadius.vertical(
                  top: Radius.circular(12),
                ),
              ),
              child: Row(
                children: [
                  Text('Request Details', style: theme.textTheme.titleMedium),
                  const Spacer(),
                  IconButton(
                    icon: const Icon(Icons.close),
                    onPressed: () => Navigator.pop(context),
                  ),
                ],
              ),
            ),

            // Content
            Flexible(
              child: _loading
                  ? const Center(child: CircularProgressIndicator())
                  : _error != null
                  ? Center(
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Text(_error!),
                          const SizedBox(height: AppSpacing.md),
                          FilledButton(
                            onPressed: _loadData,
                            child: const Text('Retry'),
                          ),
                        ],
                      ),
                    )
                  : SingleChildScrollView(
                      padding: const EdgeInsets.all(AppSpacing.md),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          // Summary
                          _buildSummary(),
                          const SizedBox(height: AppSpacing.lg),

                          // Actions
                          _buildActions(),
                          const SizedBox(height: AppSpacing.lg),

                          // Timeline
                          _buildTimeline(),
                        ],
                      ),
                    ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildSummary() {
    final theme = Theme.of(context);
    final request = _request!;
    final statusInfo = getAutomationStatusInfo(request.status);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // ID and Status
        Row(
          children: [
            Text(
              request.shortId,
              style: theme.textTheme.labelLarge?.copyWith(
                fontFamily: 'monospace',
                color: theme.colorScheme.primary,
              ),
            ),
            const SizedBox(width: AppSpacing.md),
            Container(
              padding: const EdgeInsets.symmetric(
                horizontal: AppSpacing.sm,
                vertical: AppSpacing.xs,
              ),
              decoration: BoxDecoration(
                color: statusInfo.color.withValues(alpha: 0.1),
                borderRadius: BorderRadius.circular(AppRadius.sm),
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(statusInfo.icon, size: 16, color: statusInfo.color),
                  const SizedBox(width: 4),
                  Text(
                    request.statusLabel,
                    style: theme.textTheme.labelMedium?.copyWith(
                      color: statusInfo.color,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
        const SizedBox(height: AppSpacing.md),

        // Title
        Text(request.title, style: theme.textTheme.titleLarge),
        const SizedBox(height: AppSpacing.md),

        // Meta info grid
        Wrap(
          spacing: AppSpacing.lg,
          runSpacing: AppSpacing.sm,
          children: [
            _MetaItem(label: 'Type', value: request.typeLabel),
            _MetaItem(label: 'Risk', value: request.riskLevel.toUpperCase()),
            _MetaItem(label: 'Priority', value: request.priority),
            _MetaItem(label: 'Source', value: request.sourceLabel),
            if (request.claimedBy != null)
              _MetaItem(label: 'Agent', value: request.claimedBy!),
            if (request.branchName != null)
              _MetaItem(label: 'Branch', value: request.branchName!),
          ],
        ),

        // AI Summary
        if (request.aiSummary != null && request.aiSummary!.isNotEmpty) ...[
          const SizedBox(height: AppSpacing.md),
          Container(
            padding: const EdgeInsets.all(AppSpacing.md),
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerHighest,
              borderRadius: BorderRadius.circular(AppRadius.md),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'AI Summary',
                  style: theme.textTheme.labelMedium?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: AppSpacing.xs),
                Text(request.aiSummary!, style: theme.textTheme.bodySmall),
              ],
            ),
          ),
        ],

        // PR link
        if (request.prUrl != null && request.prUrl!.isNotEmpty) ...[
          const SizedBox(height: AppSpacing.md),
          Row(
            children: [
              Icon(Icons.link, size: 16, color: theme.colorScheme.primary),
              const SizedBox(width: AppSpacing.xs),
              Flexible(
                child: Text(
                  request.prUrl!,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.primary,
                    decoration: TextDecoration.underline,
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
        ],
      ],
    );
  }

  Widget _buildActions() {
    final theme = Theme.of(context);
    final request = _request!;
    final canApproveAction = canApprove(request.status);
    final canRetryAction = canRetry(request.status);
    final canRejectAction = canReject(request.status);

    if (!canApproveAction && !canRetryAction && !canRejectAction) {
      return const SizedBox.shrink();
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('Actions', style: theme.textTheme.titleSmall),
        const SizedBox(height: AppSpacing.sm),
        Wrap(
          spacing: AppSpacing.sm,
          runSpacing: AppSpacing.sm,
          children: [
            if (canApproveAction && canAutoApprove(request.riskLevel))
              FilledButton.icon(
                onPressed: _actionInProgress ? null : _approve,
                icon: _actionInProgress
                    ? const SizedBox(
                        width: 16,
                        height: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.check),
                label: const Text('Approve'),
              ),
            if (canRejectAction)
              OutlinedButton.icon(
                onPressed: _actionInProgress ? null : _reject,
                icon: const Icon(Icons.close),
                label: const Text('Reject'),
              ),
            if (canApproveAction)
              OutlinedButton.icon(
                onPressed: _actionInProgress ? null : _escalate,
                icon: const Icon(Icons.person),
                label: const Text('Escalate'),
              ),
            if (canRetryAction)
              OutlinedButton.icon(
                onPressed: _actionInProgress ? null : _retry,
                icon: const Icon(Icons.refresh),
                label: const Text('Retry'),
              ),
          ],
        ),
      ],
    );
  }

  Widget _buildTimeline() {
    final theme = Theme.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('Timeline', style: theme.textTheme.titleSmall),
        const SizedBox(height: AppSpacing.md),
        if (_events.isEmpty)
          Center(
            child: Text(
              'No events yet',
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          )
        else
          ListView.builder(
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            itemCount: _events.length,
            itemBuilder: (context, index) {
              final event = _events[index];
              final isLast = index == _events.length - 1;
              return _TimelineItem(event: event, isLast: isLast);
            },
          ),
      ],
    );
  }
}

class _MetaItem extends StatelessWidget {
  final String label;
  final String value;

  const _MetaItem({required this.label, required this.value});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          label,
          style: theme.textTheme.labelSmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
        Text(
          value,
          style: theme.textTheme.bodyMedium?.copyWith(
            fontWeight: FontWeight.w500,
          ),
        ),
      ],
    );
  }
}

class _TimelineItem extends StatelessWidget {
  final AutomationEvent event;
  final bool isLast;

  const _TimelineItem({required this.event, required this.isLast});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return IntrinsicHeight(
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Timeline line and dot
          SizedBox(
            width: 24,
            child: Column(
              children: [
                Container(
                  width: 10,
                  height: 10,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    color: theme.colorScheme.primary,
                  ),
                ),
                if (!isLast)
                  Expanded(
                    child: Container(
                      width: 2,
                      color: theme.colorScheme.outlineVariant,
                    ),
                  ),
              ],
            ),
          ),
          const SizedBox(width: AppSpacing.sm),

          // Event content
          Expanded(
            child: Padding(
              padding: const EdgeInsets.only(bottom: AppSpacing.md),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Text(
                        event.message,
                        style: theme.textTheme.bodySmall?.copyWith(
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 2),
                  Row(
                    children: [
                      Text(
                        event.actorType.toUpperCase(),
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: theme.colorScheme.primary,
                        ),
                      ),
                      const SizedBox(width: AppSpacing.sm),
                      Text(
                        _formatTime(event.createdAt),
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
        ],
      ),
    );
  }

  String _formatTime(String isoTime) {
    try {
      final dt = DateTime.parse(isoTime);
      return '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')} '
          '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
    } catch (_) {
      return isoTime;
    }
  }
}
