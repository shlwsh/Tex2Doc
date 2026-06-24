import 'package:flutter/material.dart';

import '../../../ui/app_components.dart';
import '../../../ui/app_tokens.dart';

class AdminReleasesPanel extends StatelessWidget {
  final String apiBaseUrl;
  final String accessToken;

  const AdminReleasesPanel({
    super.key,
    required this.apiBaseUrl,
    required this.accessToken,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const AppSectionHeader(
          title: '发布管理',
          description: '集中查看服务端 Web、Flutter 用户端、Flutter 管理端与 Slint 用户端的发布通道。',
        ),
        const SizedBox(height: AppSpacing.lg),
        _ReleaseRow(
          channel: 'stable',
          endpoint: '${apiBaseUrl}releases/stable',
        ),
        _ReleaseRow(channel: 'beta', endpoint: '${apiBaseUrl}releases/beta'),
        const SizedBox(height: AppSpacing.md),
        Text(
          '当前阶段以发布清单读取为主，写入、灰度和签名上传能力预留在管理端模块中。',
          style: theme.textTheme.bodySmall,
        ),
      ],
    );
  }
}

class _ReleaseRow extends StatelessWidget {
  final String channel;
  final String endpoint;

  const _ReleaseRow({required this.channel, required this.endpoint});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return ListTile(
      contentPadding: EdgeInsets.zero,
      leading: const Icon(Icons.system_update_alt_outlined),
      title: Text(channel),
      subtitle: Text(endpoint),
      trailing: Icon(Icons.chevron_right, color: theme.colorScheme.outline),
    );
  }
}
