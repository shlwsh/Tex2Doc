import 'package:flutter/material.dart';
import '../commercial_api.dart';
import 'app_theme.dart';
import 'app_components.dart';

class FeedbackPanel extends StatefulWidget {
  final String apiBaseUrl;
  final String accessToken;
  final List<ConversionJob>? recentJobs;

  const FeedbackPanel({
    super.key,
    required this.apiBaseUrl,
    required this.accessToken,
    this.recentJobs,
  });

  @override
  State<FeedbackPanel> createState() => _FeedbackPanelState();
}

class _FeedbackPanelState extends State<FeedbackPanel> {
  late final CommercialApiClient _api;
  List<FeedbackThread> _threads = [];
  bool _loading = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _api = CommercialApiClient(widget.apiBaseUrl);
    _loadThreads();
  }

  Future<void> _loadThreads() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final threads = await _api.feedbackThreads(widget.accessToken);
      setState(() {
        _threads = threads;
        _loading = false;
      });
    } catch (e) {
      setState(() {
        _loading = false;
        _error = e.toString();
      });
    }
  }

  void _showCreateDialog() {
    showDialog(
      context: context,
      builder: (ctx) => _CreateFeedbackDialog(
        apiBaseUrl: widget.apiBaseUrl,
        accessToken: widget.accessToken,
        recentJobs: widget.recentJobs,
        onCreated: (threadId) {
          Navigator.of(ctx).pop();
          _loadThreads();
          _openThread(threadId);
        },
      ),
    );
  }

  void _openThread(String threadId) {
    Navigator.of(context).push(
      MaterialPageRoute(
        builder: (_) => FeedbackThreadPanel(
          apiBaseUrl: widget.apiBaseUrl,
          accessToken: widget.accessToken,
          threadId: threadId,
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Feedback'),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: _loadThreads,
            tooltip: 'Refresh',
          ),
        ],
      ),
      body: _buildBody(),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: _showCreateDialog,
        icon: const Icon(Icons.add),
        label: const Text('New Feedback'),
      ),
    );
  }

  Widget _buildBody() {
    if (_loading && _threads.isEmpty) {
      return const Center(child: CircularProgressIndicator());
    }
    if (_error != null && _threads.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.error_outline, size: 48, color: Theme.of(context).colorScheme.error),
            const SizedBox(height: 16),
            Text(_error!, style: TextStyle(color: Theme.of(context).colorScheme.error)),
            const SizedBox(height: 16),
            ElevatedButton.icon(
              onPressed: _loadThreads,
              icon: const Icon(Icons.refresh),
              label: const Text('Retry'),
            ),
          ],
        ),
      );
    }
    if (_threads.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.feedback_outlined, size: 64, color: Colors.grey[400]),
            const SizedBox(height: 16),
            Text(
              'No feedback yet',
              style: Theme.of(context).textTheme.titleMedium?.copyWith(color: Colors.grey[600]),
            ),
            const SizedBox(height: 8),
            Text(
              'Submit issues or feature requests to get help from our team.',
              style: Theme.of(context).textTheme.bodySmall?.copyWith(color: Colors.grey[500]),
              textAlign: TextAlign.center,
            ),
          ],
        ),
      );
    }
    return RefreshIndicator(
      onRefresh: _loadThreads,
      child: ListView.builder(
        padding: const EdgeInsets.all(16),
        itemCount: _threads.length,
        itemBuilder: (context, index) {
          final thread = _threads[index];
          return _FeedbackThreadCard(
            thread: thread,
            onTap: () => _openThread(thread.threadId),
          );
        },
      ),
    );
  }
}

class _FeedbackThreadCard extends StatelessWidget {
  final FeedbackThread thread;
  final VoidCallback onTap;

  const _FeedbackThreadCard({required this.thread, required this.onTap});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isIssue = thread.feedbackType == 'issue';
    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                    decoration: BoxDecoration(
                      color: isIssue ? Colors.red.shade50 : Colors.blue.shade50,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      thread.typeLabel,
                      style: TextStyle(
                        fontSize: 11,
                        fontWeight: FontWeight.w600,
                        color: isIssue ? Colors.red.shade700 : Colors.blue.shade700,
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  _PriorityBadge(priority: thread.priority),
                  const Spacer(),
                  _StatusBadge(status: thread.status),
                ],
              ),
              const SizedBox(height: 10),
              Text(
                thread.title,
                style: theme.textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w600),
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
              ),
              const SizedBox(height: 8),
              Row(
                children: [
                  Icon(Icons.chat_bubble_outline, size: 14, color: Colors.grey[500]),
                  const SizedBox(width: 4),
                  Text(
                    '${thread.messageCount} message${thread.messageCount == 1 ? '' : 's'}',
                    style: theme.textTheme.bodySmall?.copyWith(color: Colors.grey[500]),
                  ),
                  const SizedBox(width: 16),
                  Icon(Icons.access_time, size: 14, color: Colors.grey[500]),
                  const SizedBox(width: 4),
                  Text(
                    _formatDate(thread.createdAt),
                    style: theme.textTheme.bodySmall?.copyWith(color: Colors.grey[500]),
                  ),
                  if (thread.conversionJobId != null) ...[
                    const SizedBox(width: 16),
                    Icon(Icons.link, size: 14, color: Colors.grey[500]),
                    const SizedBox(width: 4),
                    Text(
                      'Job #${thread.conversionJobId!.substring(0, 8)}...',
                      style: theme.textTheme.bodySmall?.copyWith(color: Colors.grey[500]),
                    ),
                  ],
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  String _formatDate(String iso) {
    try {
      final dt = DateTime.parse(iso);
      return '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')}';
    } catch (_) {
      return iso.substring(0, 10);
    }
  }
}

class _PriorityBadge extends StatelessWidget {
  final String priority;

  const _PriorityBadge({required this.priority});

  @override
  Widget build(BuildContext context) {
    Color color;
    switch (priority) {
      case 'urgent':
        color = Colors.red;
        break;
      case 'high':
        color = Colors.orange;
        break;
      case 'normal':
        color = Colors.blue;
        break;
      default:
        color = Colors.grey;
    }
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: color.withAlpha(25),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: color.withAlpha(76)),
      ),
      child: Text(
        priority.toUpperCase(),
        style: TextStyle(fontSize: 10, fontWeight: FontWeight.w600, color: color),
      ),
    );
  }
}

class _StatusBadge extends StatelessWidget {
  final String status;

  const _StatusBadge({required this.status});

  @override
  Widget build(BuildContext context) {
    Color bg;
    Color fg;
    switch (status) {
      case 'open':
        bg = Colors.green.shade50;
        fg = Colors.green.shade700;
        break;
      case 'in_progress':
        bg = Colors.amber.shade50;
        fg = Colors.amber.shade700;
        break;
      case 'resolved':
        bg = Colors.purple.shade50;
        fg = Colors.purple.shade700;
        break;
      case 'closed':
        bg = Colors.grey.shade100;
        fg = Colors.grey.shade600;
        break;
      default:
        bg = Colors.grey.shade100;
        fg = Colors.grey.shade600;
    }
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
      decoration: BoxDecoration(
        color: bg,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        status.replaceAll('_', ' ').toUpperCase(),
        style: TextStyle(fontSize: 11, fontWeight: FontWeight.w600, color: fg),
      ),
    );
  }
}

class _CreateFeedbackDialog extends StatefulWidget {
  final String apiBaseUrl;
  final String accessToken;
  final List<ConversionJob>? recentJobs;
  final void Function(String threadId) onCreated;

  const _CreateFeedbackDialog({
    super.key,
    required this.apiBaseUrl,
    required this.accessToken,
    this.recentJobs,
    required this.onCreated,
  });

  @override
  State<_CreateFeedbackDialog> createState() => _CreateFeedbackDialogState();
}

class _CreateFeedbackDialogState extends State<_CreateFeedbackDialog> {
  late final CommercialApiClient _api;
  final _titleController = TextEditingController();
  final _contentController = TextEditingController();
  String _feedbackType = 'issue';
  String _priority = 'normal';
  String? _selectedJobId;
  bool _loading = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _api = CommercialApiClient(widget.apiBaseUrl);
  }

  @override
  void dispose() {
    _titleController.dispose();
    _contentController.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    if (_titleController.text.trim().isEmpty) {
      setState(() => _error = 'Title is required');
      return;
    }
    if (_contentController.text.trim().isEmpty) {
      setState(() => _error = 'Content is required');
      return;
    }
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final result = await _api.createFeedbackThread(
        accessToken: widget.accessToken,
        title: _titleController.text.trim(),
        feedbackType: _feedbackType,
        content: _contentController.text.trim(),
        conversionJobId: _selectedJobId,
        priority: _priority,
      );
      widget.onCreated(result.threadId);
    } catch (e) {
      setState(() {
        _loading = false;
        _error = e.toString();
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Submit Feedback'),
      content: SizedBox(
        width: 500,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              TextField(
                controller: _titleController,
                decoration: const InputDecoration(
                  labelText: 'Title',
                  hintText: 'Brief summary of your issue or request',
                  border: OutlineInputBorder(),
                ),
                maxLength: 100,
              ),
              const SizedBox(height: 16),
              Row(
                children: [
                  Expanded(
                    child: DropdownButtonFormField<String>(
                      value: _feedbackType,
                      decoration: const InputDecoration(
                        labelText: 'Type',
                        border: OutlineInputBorder(),
                      ),
                      items: const [
                        DropdownMenuItem(value: 'issue', child: Text('Issue')),
                        DropdownMenuItem(value: 'requirement', child: Text('Requirement')),
                      ],
                      onChanged: (v) => setState(() => _feedbackType = v!),
                    ),
                  ),
                  const SizedBox(width: 16),
                  Expanded(
                    child: DropdownButtonFormField<String>(
                      value: _priority,
                      decoration: const InputDecoration(
                        labelText: 'Priority',
                        border: OutlineInputBorder(),
                      ),
                      items: const [
                        DropdownMenuItem(value: 'low', child: Text('Low')),
                        DropdownMenuItem(value: 'normal', child: Text('Normal')),
                        DropdownMenuItem(value: 'high', child: Text('High')),
                        DropdownMenuItem(value: 'urgent', child: Text('Urgent')),
                      ],
                      onChanged: (v) => setState(() => _priority = v!),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 16),
              if (widget.recentJobs != null && widget.recentJobs!.isNotEmpty) ...[
                DropdownButtonFormField<String?>(
                  value: _selectedJobId,
                  decoration: const InputDecoration(
                    labelText: 'Related Conversion (optional)',
                    border: OutlineInputBorder(),
                  ),
                  items: [
                    const DropdownMenuItem(value: null, child: Text('None')),
                    ...widget.recentJobs!.map(
                      (j) => DropdownMenuItem(
                        value: j.jobId,
                        child: Text('${j.jobId.substring(0, 8)}... ${j.status.name}'),
                      ),
                    ),
                  ],
                  onChanged: (v) => setState(() => _selectedJobId = v),
                ),
                const SizedBox(height: 16),
              ],
              TextField(
                controller: _contentController,
                decoration: const InputDecoration(
                  labelText: 'Description',
                  hintText: 'Describe the issue or request in detail...',
                  border: OutlineInputBorder(),
                ),
                maxLines: 6,
                minLines: 4,
              ),
              if (_error != null) ...[
                const SizedBox(height: 8),
                Text(
                  _error!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error, fontSize: 12),
                ),
              ],
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: _loading ? null : () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        ElevatedButton(
          onPressed: _loading ? null : _submit,
          child: _loading
              ? const SizedBox(width: 20, height: 20, child: CircularProgressIndicator(strokeWidth: 2))
              : const Text('Submit'),
        ),
      ],
    );
  }
}
