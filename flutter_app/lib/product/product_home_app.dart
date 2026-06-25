import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';

import '../ui/app_i18n.dart';
import '../ui/app_theme.dart';
import '../ui/app_tokens.dart';
import '../admin/admin_app.dart';
import '../shared/workspace_app.dart';
import '../user/user_app.dart';

const _appIconAsset = 'assets/app_icon.jpg';

class ProductHomeApp extends StatelessWidget {
  const ProductHomeApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Tex2Doc',
      debugShowCheckedModeBanner: false,
      locale: AppLocale.zhCn.locale,
      supportedLocales: AppLocale.values.map((locale) => locale.locale),
      localizationsDelegates: const [
        AppStringsDelegate(),
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ],
      theme: AppTheme.light(AppThemeTone.defaultTone),
      darkTheme: AppTheme.dark(AppThemeTone.defaultTone),
      home: const ProductHomePage(),
    );
  }
}

class ProductHomePage extends StatelessWidget {
  const ProductHomePage({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Scaffold(
      body: SafeArea(
        child: CustomScrollView(
          slivers: [
            SliverToBoxAdapter(
              child: Padding(
                padding: const EdgeInsets.fromLTRB(
                  AppSpacing.xl,
                  AppSpacing.lg,
                  AppSpacing.xl,
                  AppSpacing.xl,
                ),
                child: Center(
                  child: ConstrainedBox(
                    constraints: const BoxConstraints(maxWidth: 1180),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        const _HomeNav(),
                        const SizedBox(height: AppSpacing.xxl),
                        LayoutBuilder(
                          builder: (context, constraints) {
                            final compact =
                                constraints.maxWidth < AppBreakpoints.tablet;
                            return compact
                                ? const Column(
                                    crossAxisAlignment:
                                        CrossAxisAlignment.stretch,
                                    children: [
                                      _HeroCopy(),
                                      SizedBox(height: AppSpacing.xl),
                                      _HeroPanel(),
                                    ],
                                  )
                                : const Row(
                                    crossAxisAlignment:
                                        CrossAxisAlignment.center,
                                    children: [
                                      Expanded(flex: 6, child: _HeroCopy()),
                                      SizedBox(width: AppSpacing.xxl),
                                      Expanded(flex: 5, child: _HeroPanel()),
                                    ],
                                  );
                          },
                        ),
                        const SizedBox(height: AppSpacing.xxl),
                        Text('核心能力', style: theme.textTheme.headlineSmall),
                        const SizedBox(height: AppSpacing.md),
                        const _FeatureGrid(),
                        const SizedBox(height: AppSpacing.xxl),
                        const _ReleaseBand(),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _HomeNav extends StatelessWidget {
  const _HomeNav();

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Row(
      children: [
        ClipRRect(
          borderRadius: BorderRadius.circular(AppRadius.sm),
          child: Image.asset(_appIconAsset, width: 42, height: 42),
        ),
        const SizedBox(width: AppSpacing.sm),
        Text('Tex2Doc', style: theme.textTheme.titleLarge),
        const Spacer(),
        TextButton(
          onPressed: () => _openWorkspace(context, DocEngineAppMode.admin),
          child: const Text('管理端'),
        ),
        const SizedBox(width: AppSpacing.xs),
        FilledButton(
          onPressed: () => _openWorkspace(context, DocEngineAppMode.user),
          child: const Text('用户登录'),
        ),
      ],
    );
  }
}

class _HeroCopy extends StatelessWidget {
  const _HeroCopy();

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Tex2Doc',
          style: theme.textTheme.displayMedium?.copyWith(
            fontWeight: FontWeight.w800,
          ),
        ),
        const SizedBox(height: AppSpacing.md),
        Text(
          '面向论文、期刊投稿和机构文档流转的 TeX/LaTeX 到 Word 转换平台。',
          style: theme.textTheme.headlineSmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: AppSpacing.lg),
        Wrap(
          spacing: AppSpacing.sm,
          runSpacing: AppSpacing.sm,
          children: [
            FilledButton.icon(
              onPressed: () => _openWorkspace(context, DocEngineAppMode.user),
              icon: const Icon(Icons.open_in_new),
              label: const Text('进入用户端'),
            ),
            OutlinedButton.icon(
              onPressed: () => _openWorkspace(context, DocEngineAppMode.user),
              icon: const Icon(Icons.download_outlined),
              label: const Text('下载桌面端'),
            ),
          ],
        ),
      ],
    );
  }
}

class _HeroPanel extends StatelessWidget {
  const _HeroPanel();

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.all(AppSpacing.lg),
      decoration: BoxDecoration(
        border: Border.all(color: theme.colorScheme.outline),
        borderRadius: BorderRadius.circular(AppRadius.md),
        color: theme.colorScheme.surface,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: const [
          _MetricLine(label: '云端转换', value: '任务队列与历史记录可追溯'),
          _MetricLine(label: '本地桌面端', value: 'Slint 跨平台用户端'),
          _MetricLine(label: '商业化', value: '套餐、兑换码、用量与反馈闭环'),
          _MetricLine(label: '管理端', value: '运营与客服独立后台入口'),
        ],
      ),
    );
  }
}

class _MetricLine extends StatelessWidget {
  final String label;
  final String value;

  const _MetricLine({required this.label, required this.value});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: AppSpacing.md),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 96,
            child: Text(label, style: theme.textTheme.labelLarge),
          ),
          Expanded(child: Text(value, style: theme.textTheme.bodyMedium)),
        ],
      ),
    );
  }
}

class _FeatureGrid extends StatelessWidget {
  const _FeatureGrid();

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final columns = constraints.maxWidth < AppBreakpoints.tablet ? 1 : 3;
        return GridView.count(
          crossAxisCount: columns,
          shrinkWrap: true,
          physics: const NeverScrollableScrollPhysics(),
          crossAxisSpacing: AppSpacing.md,
          mainAxisSpacing: AppSpacing.md,
          childAspectRatio: columns == 1 ? 3.4 : 1.45,
          children: const [
            _FeatureTile(
              icon: Icons.sync_alt,
              title: '转换工作台',
              body: '上传 TeX 项目，生成 DOCX、日志和结构化报告。',
            ),
            _FeatureTile(
              icon: Icons.history,
              title: '记录与追溯',
              body: '任务状态、下载文件、失败原因和用量账本统一保存。',
            ),
            _FeatureTile(
              icon: Icons.admin_panel_settings_outlined,
              title: '独立管理端',
              body: '兑换码、反馈、发布版本和运营数据从用户端分离。',
            ),
          ],
        );
      },
    );
  }
}

class _FeatureTile extends StatelessWidget {
  final IconData icon;
  final String title;
  final String body;

  const _FeatureTile({
    required this.icon,
    required this.title,
    required this.body,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.all(AppSpacing.lg),
      decoration: BoxDecoration(
        border: Border.all(color: theme.colorScheme.outline),
        borderRadius: BorderRadius.circular(AppRadius.md),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(icon, color: theme.colorScheme.primary),
          const SizedBox(height: AppSpacing.md),
          Text(title, style: theme.textTheme.titleMedium),
          const SizedBox(height: AppSpacing.xs),
          Text(body, style: theme.textTheme.bodyMedium),
        ],
      ),
    );
  }
}

class _ReleaseBand extends StatelessWidget {
  const _ReleaseBand();

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.all(AppSpacing.lg),
      decoration: BoxDecoration(
        color: theme.colorScheme.primaryContainer,
        borderRadius: BorderRadius.circular(AppRadius.md),
      ),
      child: Wrap(
        alignment: WrapAlignment.spaceBetween,
        crossAxisAlignment: WrapCrossAlignment.center,
        spacing: AppSpacing.md,
        runSpacing: AppSpacing.md,
        children: [
          Text(
            '服务端 Web、Flutter 管理端、Flutter 用户端、Slint 桌面端四部分发布。',
            style: theme.textTheme.titleMedium,
          ),
          OutlinedButton.icon(
            onPressed: () => _openWorkspace(context, DocEngineAppMode.admin),
            icon: const Icon(Icons.shield_outlined),
            label: const Text('进入管理端'),
          ),
        ],
      ),
    );
  }
}

void _openWorkspace(BuildContext context, DocEngineAppMode mode) {
  Navigator.of(context).pushReplacement(
    MaterialPageRoute<void>(
      builder: (_) => mode == DocEngineAppMode.admin
          ? const AdminApp(isWeb: true)
          : const UserApp(isWeb: true),
    ),
  );
}
