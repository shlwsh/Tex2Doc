import 'package:flutter/material.dart';

import '../../../commercial_api.dart';
import '../../../ui/app_components.dart';
import '../../../ui/app_tokens.dart';

class AdminAuditPanel extends StatefulWidget {
  final String apiBaseUrl;
  final String accessToken;

  const AdminAuditPanel({
    super.key,
    required this.apiBaseUrl,
    required this.accessToken,
  });

  @override
  State<AdminAuditPanel> createState() => _AdminAuditPanelState();
}

class _AdminAuditPanelState extends State<AdminAuditPanel> {
  late Future<List<Map<String, dynamic>>> _future;

  @override
  void initState() {
    super.initState();
    _future = _load();
  }

  @override
  void didUpdateWidget(covariant AdminAuditPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.apiBaseUrl != widget.apiBaseUrl ||
        oldWidget.accessToken != widget.accessToken) {
      _refresh();
    }
  }

  Future<List<Map<String, dynamic>>> _load() {
    return CommercialApiClient(
      widget.apiBaseUrl,
    ).adminReleaseAudit(widget.accessToken);
  }

  void _refresh() {
    setState(() => _future = _load());
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const AppSectionHeader(
          title: '审计中心',
          description: '查看发布清单新增、回滚和灰度策略变更记录。',
        ),
        const SizedBox(height: AppSpacing.md),
        Align(
          alignment: Alignment.centerLeft,
          child: OutlinedButton.icon(
            onPressed: _refresh,
            icon: const Icon(Icons.refresh, size: 18),
            label: const Text('刷新'),
          ),
        ),
        const SizedBox(height: AppSpacing.lg),
        FutureBuilder<List<Map<String, dynamic>>>(
          future: _future,
          builder: (context, snapshot) {
            if (snapshot.connectionState == ConnectionState.waiting) {
              return const Center(child: CircularProgressIndicator());
            }
            if (snapshot.hasError) {
              return ListTile(
                contentPadding: EdgeInsets.zero,
                leading: const Icon(Icons.error_outline),
                title: const Text('审计日志加载失败'),
                subtitle: Text(snapshot.error.toString()),
              );
            }
            final items = snapshot.data ?? const [];
            if (items.isEmpty) {
              return const ListTile(
                contentPadding: EdgeInsets.zero,
                leading: Icon(Icons.manage_search_outlined),
                title: Text('暂无审计日志'),
                subtitle: Text('发布或回滚清单后会在这里出现记录。'),
              );
            }
            return Column(
              children: items
                  .map((item) => _AuditRow(item: item))
                  .toList(growable: false),
            );
          },
        ),
      ],
    );
  }
}

class _AuditRow extends StatelessWidget {
  final Map<String, dynamic> item;

  const _AuditRow({required this.item});

  @override
  Widget build(BuildContext context) {
    final action = item['action']?.toString() ?? '-';
    final releaseId = item['release_id']?.toString() ?? '-';
    final createdAt = item['created_at']?.toString() ?? '';
    final actor = item['actor_user_id']?.toString();
    final note = item['note']?.toString();
    return ListTile(
      contentPadding: EdgeInsets.zero,
      leading: const Icon(Icons.manage_search_outlined),
      title: Text('$action / $releaseId'),
      subtitle: Text(
        [
          if (createdAt.isNotEmpty) createdAt,
          if (actor != null && actor.isNotEmpty) 'actor: $actor',
          if (note != null && note.isNotEmpty) note,
        ].join('\n'),
      ),
    );
  }
}
