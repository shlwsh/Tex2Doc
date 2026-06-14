import 'package:flutter/material.dart';

class HomePage extends StatelessWidget {
  const HomePage({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: const Text('首页'),
      ),
      body: ListView(
        padding: const EdgeInsets.all(24),
        children: [
          Text(
            '欢迎来到 Flutter Web 学习示例 👋',
            style: theme.textTheme.headlineSmall,
          ),
          const SizedBox(height: 8),
          Text(
            '这是一个从零开始的极简 Flutter Web 项目，'
            '包含路由、状态管理、响应式布局、列表与主题切换等常见知识点。',
            style: theme.textTheme.bodyLarge,
          ),
          const SizedBox(height: 32),

          // 特性卡片 - 响应式布局
          LayoutBuilder(
            builder: (context, constraints) {
              final crossCount = constraints.maxWidth >= 900
                  ? 3
                  : constraints.maxWidth >= 600
                      ? 2
                      : 1;
              return GridView.count(
                crossAxisCount: crossCount,
                shrinkWrap: true,
                physics: const NeverScrollableScrollPhysics(),
                mainAxisSpacing: 12,
                crossAxisSpacing: 12,
                children: const [
                  _FeatureCard(
                    icon: Icons.route,
                    title: '路由导航',
                    description: 'Named routes + Drawer/NavBar 响应式导航',
                  ),
                  _FeatureCard(
                    icon: Icons.bolt,
                    title: 'StatefulWidget',
                    description: '使用 setState 管理本地状态',
                  ),
                  _FeatureCard(
                    icon: Icons.list_alt,
                    title: '列表管理',
                    description: 'ListView.builder + 增删改查',
                  ),
                  _FeatureCard(
                    icon: Icons.contrast,
                    title: '主题切换',
                    description: '支持亮色/暗色/跟随系统',
                  ),
                  _FeatureCard(
                    icon: Icons.photo_size_select_large,
                    title: '响应式布局',
                    description: 'LayoutBuilder / MediaQuery',
                  ),
                  _FeatureCard(
                    icon: Icons.code,
                    title: '纯 Dart',
                    description: '没有额外依赖，易于阅读',
                  ),
                ],
              );
            },
          ),

          const SizedBox(height: 32),

          Card.outlined(
            child: Padding(
              padding: const EdgeInsets.all(20),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('下一步建议', style: theme.textTheme.titleMedium),
                  const SizedBox(height: 12),
                  const _StepItem(step: 1, text: '打开侧边栏，体验"计数器"页面'),
                  const _StepItem(step: 2, text: '打开"待办清单"，练习列表操作'),
                  const _StepItem(step: 3, text: '阅读 lib/ 下的源码，修改样式、文字、主题'),
                  const _StepItem(step: 4, text: '尝试新增一个页面（如"设置"）并接入路由'),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _FeatureCard extends StatelessWidget {
  const _FeatureCard({
    required this.icon,
    required this.title,
    required this.description,
  });

  final IconData icon;
  final String title;
  final String description;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Card.filled(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(icon, color: theme.colorScheme.primary, size: 28),
            const SizedBox(height: 12),
            Text(title, style: theme.textTheme.titleSmall),
            const SizedBox(height: 6),
            Text(
              description,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _StepItem extends StatelessWidget {
  const _StepItem({required this.step, required this.text});

  final int step;
  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            width: 22,
            height: 22,
            margin: const EdgeInsets.only(right: 10, top: 2),
            decoration: BoxDecoration(
              color: Theme.of(context).colorScheme.primary,
              shape: BoxShape.circle,
            ),
            alignment: Alignment.center,
            child: Text(
              '$step',
              style: const TextStyle(color: Colors.white, fontSize: 12),
            ),
          ),
          Expanded(child: Text(text)),
        ],
      ),
    );
  }
}
