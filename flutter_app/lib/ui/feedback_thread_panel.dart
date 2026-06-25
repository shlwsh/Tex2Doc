import 'package:flutter/material.dart';
import '../commercial_api.dart';

class FeedbackThreadPanel extends StatefulWidget {
  final String apiBaseUrl;
  final String accessToken;
  final String threadId;

  const FeedbackThreadPanel({
    super.key,
    required this.apiBaseUrl,
    required this.accessToken,
    required this.threadId,
  });

  @override
  State<FeedbackThreadPanel> createState() => _FeedbackThreadPanelState();
}

class _FeedbackThreadPanelState extends State<FeedbackThreadPanel> {
  late final CommercialApiClient _api;
  FeedbackThreadDetail? _detail;
  bool _loading = false;
  String? _error;
  final _replyController = TextEditingController();
  final _scrollController = ScrollController();

  @override
  void initState() {
    super.initState();
    _api = CommercialApiClient(widget.apiBaseUrl);
    _loadThread();
  }

  @override
  void dispose() {
    _replyController.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  Future<void> _loadThread() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final detail = await _api.feedbackThread(widget.accessToken, widget.threadId);
      setState(() {
        _detail = detail;
        _loading = false;
      });
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (_scrollController.hasClients) {
          _scrollController.animateTo(
            _scrollController.position.maxScrollExtent,
            duration: const Duration(milliseconds: 300),
            curve: Curves.easeOut,
          );
        }
      });
    } catch (e) {
      setState(() {
        _loading = false;
        _error = e.toString();
      });
    }
  }

  Future<void> _sendReply() async {
    final content = _replyController.text.trim();
    if (content.isEmpty) return;

    setState(() => _loading = true);
    try {
      await _api.addFeedbackMessage(
        accessToken: widget.accessToken,
        threadId: widget.threadId,
        content: content,
      );
      _replyController.clear();
      await _loadThread();
    } catch (e) {
      setState(() {
        _loading = false;
        _error = e.toString();
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Scaffold(
      appBar: AppBar(
        title: Text(_detail?.thread.title ?? 'Feedback'),
        actions: [
          IconButton(icon: const Icon(Icons.refresh), onPressed: _loadThread, tooltip: 'Refresh'),
        ],
      ),
      body: _buildBody(theme),
      bottomNavigationBar:
          _detail != null && _detail!.thread.status != 'closed' ? _buildReplyBar(theme) : null,
    );
  }

  Widget _buildBody(ThemeData theme) {
    if (_loading && _detail == null) {
      return const Center(child: CircularProgressIndicator());
    }
    if (_error != null && _detail == null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.error_outline, size: 48, color: theme.colorScheme.error),
            const SizedBox(height: 16),
            Text(_error!, style: TextStyle(color: theme.colorScheme.error)),
            const SizedBox(height: 16),
            ElevatedButton.icon(onPressed: _loadThread, icon: const Icon(Icons.refresh), label: const Text('Retry')),
          ],
        ),
      );
    }

    final detail = _detail!;
    return Column(
      children: [
        Container(
          padding: const EdgeInsets.all(16),
          decoration: BoxDecoration(
            color: theme.colorScheme.surfaceContainerHighest.withAlpha(128),
            border: Border(bottom: BorderSide(color: theme.dividerColor)),
          ),
          child: Row(
            children: [
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                  color: detail.thread.feedbackType == 'issue' ? Colors.red.shade50 : Colors.blue.shade50,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  detail.thread.typeLabel,
                  style: TextStyle(
                    fontSize: 12,
                    fontWeight: FontWeight.w600,
                    color: detail.thread.feedbackType == 'issue' ? Colors.red.shade700 : Colors.blue.shade700,
                  ),
                ),
              ),
              const SizedBox(width: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                  color: _statusColor(detail.thread.status).withAlpha(25),
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  detail.thread.statusLabel,
                  style: TextStyle(fontSize: 12, fontWeight: FontWeight.w600, color: _statusColor(detail.thread.status)),
                ),
              ),
              const Spacer(),
              Text('Priority: ${detail.thread.priorityLabel}', style: theme.textTheme.bodySmall),
            ],
          ),
        ),
        Expanded(
          child: RefreshIndicator(
            onRefresh: _loadThread,
            child: ListView.builder(
              controller: _scrollController,
              padding: const EdgeInsets.all(16),
              itemCount: detail.messages.length,
              itemBuilder: (context, index) {
                final msg = detail.messages[index];
                return _MessageBubble(message: msg);
              },
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildReplyBar(ThemeData theme) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        border: Border(top: BorderSide(color: theme.dividerColor)),
      ),
      child: SafeArea(
        child: Row(
          children: [
            Expanded(
              child: TextField(
                controller: _replyController,
                decoration: InputDecoration(
                  hintText: 'Type your reply...',
                  border: OutlineInputBorder(borderRadius: BorderRadius.circular(24)),
                  contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                  filled: true,
                  fillColor: theme.colorScheme.surfaceContainerHighest.withAlpha(128),
                ),
                maxLines: 4,
                minLines: 1,
                textInputAction: TextInputAction.send,
                onSubmitted: (_) => _sendReply(),
              ),
            ),
            const SizedBox(width: 8),
            IconButton.filled(
              onPressed: _loading ? null : _sendReply,
              icon: _loading
                  ? const SizedBox(width: 20, height: 20, child: CircularProgressIndicator(strokeWidth: 2))
                  : const Icon(Icons.send),
              tooltip: 'Send',
            ),
          ],
        ),
      ),
    );
  }

  Color _statusColor(String status) {
    switch (status) {
      case 'open':
        return Colors.green.shade700;
      case 'in_progress':
        return Colors.amber.shade700;
      case 'resolved':
        return Colors.purple.shade700;
      case 'closed':
        return Colors.grey.shade600;
      default:
        return Colors.grey.shade600;
    }
  }
}

class _MessageBubble extends StatelessWidget {
  final FeedbackMessage message;

  const _MessageBubble({required this.message});

  @override
  Widget build(BuildContext context) {
    final isUser = message.senderType == 'user';
    final isSystem = message.senderType == 'system';
    final theme = Theme.of(context);

    if (isSystem) {
      return Container(
        margin: const EdgeInsets.symmetric(vertical: 8),
        child: Center(
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
            decoration: BoxDecoration(color: Colors.grey.shade100, borderRadius: BorderRadius.circular(16)),
            child: Text(message.content, style: theme.textTheme.bodySmall?.copyWith(color: Colors.grey[600])),
          ),
        ),
      );
    }

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        mainAxisAlignment: isUser ? MainAxisAlignment.end : MainAxisAlignment.start,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (!isUser) ...[
            CircleAvatar(
              radius: 16,
              backgroundColor: isUser ? theme.colorScheme.primary : theme.colorScheme.secondary,
              child: Icon(isUser ? Icons.person : Icons.support_agent, size: 16, color: Colors.white),
            ),
            const SizedBox(width: 8),
          ],
          Flexible(
            child: Column(
              crossAxisAlignment: isUser ? CrossAxisAlignment.end : CrossAxisAlignment.start,
              children: [
                Container(
                  constraints: BoxConstraints(maxWidth: MediaQuery.of(context).size.width * 0.65),
                  padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                  decoration: BoxDecoration(
                    color: isUser ? theme.colorScheme.primary : theme.colorScheme.surfaceContainerHighest,
                    borderRadius: BorderRadius.only(
                      topLeft: const Radius.circular(16),
                      topRight: const Radius.circular(16),
                      bottomLeft: isUser ? const Radius.circular(16) : const Radius.circular(4),
                      bottomRight: isUser ? const Radius.circular(4) : const Radius.circular(16),
                    ),
                  ),
                  child: Text(
                    message.content,
                    style: TextStyle(
                      color: isUser ? theme.colorScheme.onPrimary : theme.colorScheme.onSurface,
                    ),
                  ),
                ),
                const SizedBox(height: 4),
                Text(
                  _formatDateTime(message.createdAt),
                  style: theme.textTheme.bodySmall?.copyWith(color: Colors.grey[500], fontSize: 10),
                ),
              ],
            ),
          ),
          if (isUser) ...[
            const SizedBox(width: 8),
            CircleAvatar(
              radius: 16,
              backgroundColor: theme.colorScheme.primary,
              child: const Icon(Icons.person, size: 16, color: Colors.white),
            ),
          ],
        ],
      ),
    );
  }

  String _formatDateTime(String iso) {
    try {
      final dt = DateTime.parse(iso);
      return '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}  '
          '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')}';
    } catch (_) {
      return iso;
    }
  }
}
