import 'package:flutter/material.dart';

import '../commercial_api.dart';
import '../ui/app_components.dart';
import '../ui/app_tokens.dart';

class AdminFeedbackPanel extends StatefulWidget {
  final String apiBaseUrl;
  final String accessToken;

  const AdminFeedbackPanel({
    super.key,
    required this.apiBaseUrl,
    required this.accessToken,
  });

  @override
  State<AdminFeedbackPanel> createState() => _AdminFeedbackPanelState();
}

class _AdminFeedbackPanelState extends State<AdminFeedbackPanel> {
  late final CommercialApiClient _api;
  List<FeedbackThread> _threads = const [];
  bool _loading = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _api = CommercialApiClient(widget.apiBaseUrl);
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final threads = await _api.adminFeedbackThreads(widget.accessToken);
      if (!mounted) return;
      setState(() {
        _threads = threads;
        _loading = false;
      });
    } on Object catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _loading = false;
      });
    }
  }

  Future<void> _setStatus(FeedbackThread thread, String status) async {
    await _api.adminUpdateFeedbackThread(
      adminToken: widget.accessToken,
      threadId: thread.threadId,
      status: status,
    );
    await _load();
  }

  Future<void> _reply(FeedbackThread thread) async {
    final controller = TextEditingController();
    final content = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Reply: ${thread.title}'),
        content: TextField(
          controller: controller,
          minLines: 4,
          maxLines: 8,
          decoration: const InputDecoration(
            labelText: 'Reply content',
            border: OutlineInputBorder(),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(controller.text.trim()),
            child: const Text('Send'),
          ),
        ],
      ),
    );
    controller.dispose();
    if (content == null || content.isEmpty) return;
    await _api.adminReplyFeedbackThread(
      adminToken: widget.accessToken,
      threadId: thread.threadId,
      content: content,
    );
    await _load();
  }

  @override
  Widget build(BuildContext context) {
    if (_loading && _threads.isEmpty) {
      return const AppCard(child: LoadingState(label: 'Loading feedback...'));
    }
    if (_error != null && _threads.isEmpty) {
      return AppCard(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(_error!, style: TextStyle(color: Theme.of(context).colorScheme.error)),
            const SizedBox(height: AppSpacing.md),
            OutlinedButton.icon(
              onPressed: _load,
              icon: const Icon(Icons.refresh),
              label: const Text('Retry'),
            ),
          ],
        ),
      );
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            const AppSectionHeader(
              title: 'Feedback management',
              description: 'Review user feedback, update status, and send replies.',
            ),
            const Spacer(),
            IconButton(
              onPressed: _loading ? null : _load,
              icon: const Icon(Icons.refresh),
              tooltip: 'Refresh',
            ),
          ],
        ),
        const SizedBox(height: AppSpacing.md),
        if (_threads.isEmpty)
          const AppCard(child: EmptyState(label: 'No feedback threads.'))
        else
          for (final thread in _threads) ...[
            _AdminFeedbackThreadCard(
              thread: thread,
              onReply: () => _reply(thread),
              onStatusChanged: (status) => _setStatus(thread, status),
            ),
            const SizedBox(height: AppSpacing.sm),
          ],
      ],
    );
  }
}

class _AdminFeedbackThreadCard extends StatelessWidget {
  final FeedbackThread thread;
  final VoidCallback onReply;
  final ValueChanged<String> onStatusChanged;

  const _AdminFeedbackThreadCard({
    required this.thread,
    required this.onReply,
    required this.onStatusChanged,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return AppCard(
      padding: const EdgeInsets.all(AppSpacing.md),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Wrap(
                      spacing: AppSpacing.xs,
                      runSpacing: AppSpacing.xs,
                      children: [
                        StatusPill(
                          icon: Icons.feedback_outlined,
                          label: thread.typeLabel,
                          color: theme.colorScheme.primary,
                        ),
                        StatusPill(
                          icon: Icons.flag_outlined,
                          label: thread.priorityLabel,
                          color: _priorityColor(thread.priority, theme),
                        ),
                        StatusPill(
                          icon: Icons.radio_button_checked,
                          label: thread.statusLabel,
                          color: _statusColor(thread.status, theme),
                        ),
                      ],
                    ),
                    const SizedBox(height: AppSpacing.sm),
                    Text(thread.title, style: theme.textTheme.titleMedium),
                    const SizedBox(height: AppSpacing.xs),
                    Text(
                      '${thread.messageCount} messages · created ${thread.createdAt}',
                      style: theme.textTheme.bodySmall,
                    ),
                    if (thread.conversionJobId != null) ...[
                      const SizedBox(height: AppSpacing.xs),
                      Text(
                        'Conversion: ${thread.conversionJobId}',
                        style: theme.textTheme.bodySmall,
                      ),
                    ],
                  ],
                ),
              ),
              const SizedBox(width: AppSpacing.md),
              Wrap(
                spacing: AppSpacing.xs,
                runSpacing: AppSpacing.xs,
                alignment: WrapAlignment.end,
                children: [
                  DropdownButton<String>(
                    value: thread.status,
                    items: const [
                      DropdownMenuItem(value: 'open', child: Text('Open')),
                      DropdownMenuItem(
                        value: 'in_progress',
                        child: Text('In progress'),
                      ),
                      DropdownMenuItem(value: 'resolved', child: Text('Resolved')),
                      DropdownMenuItem(value: 'closed', child: Text('Closed')),
                    ],
                    onChanged: (value) {
                      if (value != null && value != thread.status) {
                        onStatusChanged(value);
                      }
                    },
                  ),
                  FilledButton.icon(
                    onPressed: onReply,
                    icon: const Icon(Icons.reply),
                    label: const Text('Reply'),
                  ),
                ],
              ),
            ],
          ),
        ],
      ),
    );
  }
}

Color _priorityColor(String priority, ThemeData theme) {
  return switch (priority) {
    'urgent' || 'high' => theme.colorScheme.error,
    'low' => theme.colorScheme.tertiary,
    _ => theme.colorScheme.primary,
  };
}

Color _statusColor(String status, ThemeData theme) {
  return switch (status) {
    'resolved' || 'closed' => Colors.green,
    'in_progress' => Colors.orange,
    _ => theme.colorScheme.primary,
  };
}
