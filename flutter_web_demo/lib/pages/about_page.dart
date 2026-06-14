import 'package:flutter/material.dart';

class AboutPage extends StatelessWidget {
  const AboutPage({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isDark = theme.brightness == Brightness.dark;

    return Scaffold(
      appBar: AppBar(title: const Text('关于')),
      body: ListView(
        padding: const EdgeInsets.all(24),
        children: [
          Card.filled(
            child: Padding(
              padding: const EdgeInsets.all(20),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Container(
                        width: 48,
                        height: 48,
                        decoration: BoxDecoration(
                          color: theme.colorScheme.primary,
                          borderRadius: BorderRadius.circular(12),
                        ),
                        child: const Icon(
                          Icons.flutter_dash,
                          color: Colors.white,
                        ),
                      ),
                      const SizedBox(width: 14),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              'Flutter Web Demo',
                              style: theme.textTheme.titleLarge,
                            ),
                            Text(
                              '版本 1.0.0',
                              style: theme.textTheme.bodyMedium?.copyWith(
                                    color: theme.colorScheme.onSurfaceVariant,
                                  ),
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 16),
                  const Text(
                    '这是一个专为学习 Flutter Web 开发而设计的最小化示例项目。'
                    '没有引入任何第三方状态管理库，所有功能都使用 Flutter 内置 API 实现，'
                    '便于理解底层原理。',
                  ),
                ],
              ),
            ),
          ),

          const SizedBox(height: 20),

          // 主题切换
          Card.outlined(
            child: SwitchListTile(
              secondary: Icon(
                isDark ? Icons.dark_mode : Icons.light_mode,
                color: theme.colorScheme.primary,
              ),
              title: const Text('使用深色模式（跟随系统）'),
              subtitle: Text(isDark ? '当前：深色' : '当前：浅色'),
              value: isDark,
              onChanged: (_) {
                // 本示例采用 ThemeMode.system，这里仅给出提示
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(
                    content: Text('本示例跟随系统主题，切换系统主题试试～'),
                    duration: Duration(seconds: 2),
                  ),
                );
              },
            ),
          ),

          const SizedBox(height: 20),

          // 目录结构
          Text('项目结构', style: theme.textTheme.titleMedium),
          const SizedBox(height: 8),
          const Card.outlined(
            child: Padding(
              padding: EdgeInsets.all(16),
              child: Text(
                'flutter_web_demo/\n'
                '├── lib/\n'
                '│   ├── main.dart          (应用入口、主题、路由)\n'
                '│   └── pages/\n'
                '│       ├── home_page.dart      (首页 - 响应式卡片)\n'
                '│       ├── counter_page.dart   (计数器 - 状态管理)\n'
                '│       ├── todo_page.dart      (待办清单 - 列表操作)\n'
                '│       └── about_page.dart     (关于页 - 静态展示)\n'
                '├── web/\n'
                '│   ├── index.html         (Web 宿主页面)\n'
                '│   └── manifest.json      (PWA 元信息)\n'
                '└── pubspec.yaml           (依赖与资源配置)',
                style: TextStyle(fontFamily: 'monospace', fontSize: 13, height: 1.6),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
