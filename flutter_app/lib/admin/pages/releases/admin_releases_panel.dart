import 'package:flutter/material.dart';

import '../../../commercial_api.dart';
import '../../../ui/app_components.dart';
import '../../../ui/app_tokens.dart';

class AdminReleasesPanel extends StatefulWidget {
  final String apiBaseUrl;
  final String accessToken;

  const AdminReleasesPanel({
    super.key,
    required this.apiBaseUrl,
    required this.accessToken,
  });

  @override
  State<AdminReleasesPanel> createState() => _AdminReleasesPanelState();
}

class _AdminReleasesPanelState extends State<AdminReleasesPanel> {
  final _channelController = TextEditingController(text: 'beta');
  final _platformController = TextEditingController(text: 'windows');
  final _archController = TextEditingController(text: 'x64');
  final _versionController = TextEditingController();
  final _urlController = TextEditingController();
  final _shaController = TextEditingController();
  final _titleController = TextEditingController();

  late Future<List<Map<String, dynamic>>> _future;
  bool _publishing = false;

  @override
  void initState() {
    super.initState();
    _future = _load();
  }

  @override
  void didUpdateWidget(covariant AdminReleasesPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.apiBaseUrl != widget.apiBaseUrl ||
        oldWidget.accessToken != widget.accessToken) {
      _refresh();
    }
  }

  @override
  void dispose() {
    _channelController.dispose();
    _platformController.dispose();
    _archController.dispose();
    _versionController.dispose();
    _urlController.dispose();
    _shaController.dispose();
    _titleController.dispose();
    super.dispose();
  }

  Future<List<Map<String, dynamic>>> _load() {
    return CommercialApiClient(
      widget.apiBaseUrl,
    ).adminReleases(widget.accessToken);
  }

  void _refresh() {
    setState(() => _future = _load());
  }

  Future<void> _publish() async {
    if (_publishing) return;
    final version = _versionController.text.trim();
    final url = _urlController.text.trim();
    final sha = _shaController.text.trim();
    if (version.isEmpty || url.isEmpty || sha.isEmpty) {
      _showMessage('版本、下载地址和 SHA-256 必填。');
      return;
    }
    setState(() => _publishing = true);
    try {
      await CommercialApiClient(widget.apiBaseUrl).adminPublishRelease(
        adminToken: widget.accessToken,
        channel: _channelController.text.trim().isEmpty
            ? 'beta'
            : _channelController.text.trim(),
        platform: _platformController.text.trim().isEmpty
            ? 'windows'
            : _platformController.text.trim(),
        arch: _archController.text.trim().isEmpty
            ? 'x64'
            : _archController.text.trim(),
        version: version,
        downloadUrl: url,
        sha256: sha,
        releaseTitle: _titleController.text.trim(),
        isPrerelease: _channelController.text.trim() != 'stable',
        strategy: const {'rollout_percent': 100, 'audience': 'invite_beta'},
      );
      _showMessage('发布清单已写入。');
      _refresh();
    } on Object catch (e) {
      _showMessage(e.toString());
    } finally {
      if (mounted) setState(() => _publishing = false);
    }
  }

  Future<void> _rollback(Map<String, dynamic> release) async {
    final id = release['release_id']?.toString();
    if (id == null || id.isEmpty) return;
    try {
      await CommercialApiClient(widget.apiBaseUrl).adminRollbackRelease(
        adminToken: widget.accessToken,
        releaseId: id,
        reason: 'admin panel rollback',
      );
      _showMessage('已标记回滚。');
      _refresh();
    } on Object catch (e) {
      _showMessage(e.toString());
    }
  }

  void _showMessage(String message) {
    if (!mounted) return;
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(message)));
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const AppSectionHeader(
          title: '发布管理',
          description: '维护邀请制 Beta 的发布清单，支持灰度策略记录和回滚审计。',
        ),
        const SizedBox(height: AppSpacing.lg),
        _PublishForm(
          channelController: _channelController,
          platformController: _platformController,
          archController: _archController,
          versionController: _versionController,
          urlController: _urlController,
          shaController: _shaController,
          titleController: _titleController,
          busy: _publishing,
          onPublish: _publish,
          onRefresh: _refresh,
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
                title: const Text('发布清单加载失败'),
                subtitle: Text(snapshot.error.toString()),
              );
            }
            final releases = snapshot.data ?? const [];
            if (releases.isEmpty) {
              return const ListTile(
                contentPadding: EdgeInsets.zero,
                leading: Icon(Icons.inventory_2_outlined),
                title: Text('暂无发布清单'),
                subtitle: Text('填写上方表单后可发布第一个 Beta 清单。'),
              );
            }
            return Column(
              children: releases
                  .map(
                    (release) => _ReleaseRow(
                      release: release,
                      onRollback: () => _rollback(release),
                    ),
                  )
                  .toList(growable: false),
            );
          },
        ),
      ],
    );
  }
}

class _PublishForm extends StatelessWidget {
  final TextEditingController channelController;
  final TextEditingController platformController;
  final TextEditingController archController;
  final TextEditingController versionController;
  final TextEditingController urlController;
  final TextEditingController shaController;
  final TextEditingController titleController;
  final bool busy;
  final VoidCallback onPublish;
  final VoidCallback onRefresh;

  const _PublishForm({
    required this.channelController,
    required this.platformController,
    required this.archController,
    required this.versionController,
    required this.urlController,
    required this.shaController,
    required this.titleController,
    required this.busy,
    required this.onPublish,
    required this.onRefresh,
  });

  @override
  Widget build(BuildContext context) {
    return AppCard(
      child: Column(
        children: [
          Wrap(
            spacing: AppSpacing.md,
            runSpacing: AppSpacing.md,
            children: [
              _SmallField(label: '通道', controller: channelController),
              _SmallField(label: '平台', controller: platformController),
              _SmallField(label: '架构', controller: archController),
              _SmallField(label: '版本', controller: versionController),
              _SmallField(label: '标题', controller: titleController),
            ],
          ),
          const SizedBox(height: AppSpacing.md),
          TextField(
            controller: urlController,
            decoration: const InputDecoration(
              labelText: '下载地址',
              prefixIcon: Icon(Icons.link_outlined),
            ),
          ),
          const SizedBox(height: AppSpacing.md),
          TextField(
            controller: shaController,
            decoration: const InputDecoration(
              labelText: 'SHA-256',
              prefixIcon: Icon(Icons.fingerprint),
            ),
          ),
          const SizedBox(height: AppSpacing.md),
          Row(
            children: [
              FilledButton.icon(
                onPressed: busy ? null : onPublish,
                icon: const Icon(Icons.rocket_launch_outlined),
                label: const Text('发布清单'),
              ),
              const SizedBox(width: AppSpacing.sm),
              OutlinedButton.icon(
                onPressed: busy ? null : onRefresh,
                icon: const Icon(Icons.refresh),
                label: const Text('刷新'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _SmallField extends StatelessWidget {
  final String label;
  final TextEditingController controller;

  const _SmallField({required this.label, required this.controller});

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 180,
      child: TextField(
        controller: controller,
        decoration: InputDecoration(labelText: label),
      ),
    );
  }
}

class _ReleaseRow extends StatelessWidget {
  final Map<String, dynamic> release;
  final VoidCallback onRollback;

  const _ReleaseRow({required this.release, required this.onRollback});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final channel = release['channel']?.toString() ?? '-';
    final platform = release['platform']?.toString() ?? '-';
    final version = release['version']?.toString() ?? '-';
    final deprecatedAt = release['deprecated_at']?.toString();
    final title = release['release_title']?.toString();
    return ListTile(
      contentPadding: EdgeInsets.zero,
      leading: Icon(
        deprecatedAt == null
            ? Icons.system_update_alt_outlined
            : Icons.undo_outlined,
      ),
      title: Text('$channel / $platform / $version'),
      subtitle: Text(
        [
          if (title != null && title.isNotEmpty) title,
          release['download_url']?.toString() ?? '',
          if (deprecatedAt != null && deprecatedAt.isNotEmpty)
            '已回滚: $deprecatedAt',
        ].where((item) => item.isNotEmpty).join('\n'),
      ),
      trailing: deprecatedAt == null
          ? TextButton.icon(
              onPressed: onRollback,
              icon: const Icon(Icons.undo, size: 18),
              label: const Text('回滚'),
            )
          : Icon(Icons.check_circle_outline, color: theme.colorScheme.outline),
    );
  }
}
