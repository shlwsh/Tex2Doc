import 'package:flutter/material.dart';

import '../../../ui/app_components.dart';
import '../../../ui/app_tokens.dart';

class AdminAuditPanel extends StatelessWidget {
  final String apiBaseUrl;
  final String accessToken;

  const AdminAuditPanel({
    super.key,
    required this.apiBaseUrl,
    required this.accessToken,
  });

  @override
  Widget build(BuildContext context) {
    return const Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        AppSectionHeader(
          title: '审计中心',
          description: '汇总登录、兑换码、发布、反馈处理等关键操作的审计入口。',
        ),
        SizedBox(height: AppSpacing.lg),
        _AuditPlaceholder(),
      ],
    );
  }
}

class _AuditPlaceholder extends StatelessWidget {
  const _AuditPlaceholder();

  @override
  Widget build(BuildContext context) {
    return const ListTile(
      contentPadding: EdgeInsets.zero,
      leading: Icon(Icons.manage_search_outlined),
      title: Text('审计日志接口预留'),
      subtitle: Text('数据库审计表与筛选导出能力将在下一阶段接入。'),
    );
  }
}
