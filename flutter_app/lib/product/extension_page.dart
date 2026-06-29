import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';

import '../ui/app_i18n.dart';
import '../ui/app_theme.dart';
import '../ui/app_tokens.dart';
import '../ui/app_components.dart';
import '../file_web_stub.dart'
    if (dart.library.js_interop) '../file_web_utils_web.dart'
    if (dart.library.io) '../file_web_utils_io.dart';

const _appIconAsset = 'assets/app_icon.png';

// Store URLs (these are placeholders; update once the extension is published)
const _chromeWebStoreUrl = 'https://chromewebstore.google.com/category/extensions';
const _edgeAddonsUrl = 'https://microsoftedge.microsoft.com/addons';
const _firefoxAddonsUrl = 'https://addons.mozilla.org';
const _safariAppStoreUrl = 'https://apps.apple.com';

class ExtensionPage extends StatelessWidget {
  const ExtensionPage({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Tex2Doc Browser Extension',
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
      home: const _ExtensionHomePage(),
    );
  }
}

class _ExtensionHomePage extends StatelessWidget {
  const _ExtensionHomePage();

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
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
                        _ExtensionNav(onBack: () => _backToHome(context)),
                        const SizedBox(height: AppSpacing.xxl),
                        _ExtensionHero(
                          strings: strings,
                          onChrome: () => openExternalUrl(_chromeWebStoreUrl),
                          onEdge: () => openExternalUrl(_edgeAddonsUrl),
                          onFirefox: () => openExternalUrl(_firefoxAddonsUrl),
                          onSafari: () => openExternalUrl(_safariAppStoreUrl),
                        ),
                        const SizedBox(height: AppSpacing.xxl),
                        Text(
                          strings.t('extension.featuresTitle'),
                          style: theme.textTheme.headlineSmall,
                        ),
                        const SizedBox(height: AppSpacing.md),
                        const _FeatureGrid(),
                        const SizedBox(height: AppSpacing.xxl),
                        Text(
                          strings.t('extension.installGuide.title'),
                          style: theme.textTheme.headlineSmall,
                        ),
                        const SizedBox(height: AppSpacing.md),
                        const _InstallGuideGrid(),
                        const SizedBox(height: AppSpacing.xxl),
                        const _UsageGuide(),
                        const SizedBox(height: AppSpacing.xxl),
                        const _ExtensionFooterBand(),
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

  void _backToHome(BuildContext context) {
    // SPA fallback: rely on parent navigator / browser history.
    Navigator.of(context).maybePop();
  }
}

// ─── Nav bar (with back) ───────────────────────────────────────────────────────

class _ExtensionNav extends StatelessWidget {
  final VoidCallback onBack;
  const _ExtensionNav({required this.onBack});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Row(
      children: [
        IconButton(
          tooltip: 'Back',
          onPressed: onBack,
          icon: const Icon(Icons.arrow_back),
        ),
        ClipRRect(
          borderRadius: BorderRadius.circular(AppRadius.sm),
          child: Image.asset(_appIconAsset, width: 42, height: 42),
        ),
        const SizedBox(width: AppSpacing.sm),
        Flexible(
          child: Text(
            'Tex2Doc Browser Extension',
            overflow: TextOverflow.ellipsis,
            style: theme.textTheme.titleLarge,
          ),
        ),
        const Spacer(),
        Wrap(
          spacing: AppSpacing.xs,
          runSpacing: AppSpacing.xs,
          children: [
            TextButton.icon(
              onPressed: onBack,
              icon: const Icon(Icons.home_outlined),
              label: const Text('主页'),
            ),
          ],
        ),
      ],
    );
  }
}

// ─── Hero ─────────────────────────────────────────────────────────────────────

class _ExtensionHero extends StatelessWidget {
  final AppStrings strings;
  final VoidCallback onChrome;
  final VoidCallback onEdge;
  final VoidCallback onFirefox;
  final VoidCallback onSafari;

  const _ExtensionHero({
    required this.strings,
    required this.onChrome,
    required this.onEdge,
    required this.onFirefox,
    required this.onSafari,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return LayoutBuilder(
      builder: (context, constraints) {
        final compact = constraints.maxWidth < AppBreakpoints.tablet;
        final titleBlock = Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Container(
                  padding: const EdgeInsets.all(AppSpacing.sm),
                  decoration: BoxDecoration(
                    color: theme.colorScheme.primaryContainer,
                    borderRadius: BorderRadius.circular(AppRadius.lg),
                  ),
                  child: Icon(
                    Icons.extension,
                    size: 32,
                    color: theme.colorScheme.primary,
                  ),
                ),
                const SizedBox(width: AppSpacing.md),
                Flexible(
                  child: Text(
                    strings.t('extension.title'),
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.displayMedium?.copyWith(
                      fontWeight: FontWeight.w800,
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: AppSpacing.md),
            Text(
              strings.t('extension.subtitle'),
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
                  onPressed: onChrome,
                  icon: const Icon(Icons.shop_outlined),
                  label: Text(strings.t('extension.install.chrome')),
                ),
                FilledButton.tonalIcon(
                  onPressed: onEdge,
                  icon: const Icon(Icons.shop_outlined),
                  label: Text(strings.t('extension.install.edge')),
                ),
                OutlinedButton.icon(
                  onPressed: onFirefox,
                  icon: const Icon(Icons.shop_outlined),
                  label: Text(strings.t('extension.install.firefox')),
                ),
                OutlinedButton.icon(
                  onPressed: onSafari,
                  icon: const Icon(Icons.shop_outlined),
                  label: Text(strings.t('extension.install.safari')),
                ),
              ],
            ),
          ],
        );
        final heroPanel = AppCard(
          padding: const EdgeInsets.all(AppSpacing.lg),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              MetricTile(
                icon: Icons.cloud_sync_outlined,
                label: strings.t('extension.feature.cloud.title'),
                value: strings.t('extension.feature.cloud.body'),
              ),
              const SizedBox(height: AppSpacing.sm),
              MetricTile(
                icon: Icons.memory,
                label: strings.t('extension.feature.local.title'),
                value: strings.t('extension.feature.local.body'),
              ),
              const SizedBox(height: AppSpacing.sm),
              MetricTile(
                icon: Icons.web_stories_outlined,
                label: strings.t('extension.feature.overleaf.title'),
                value: strings.t('extension.feature.overleaf.body'),
              ),
              const SizedBox(height: AppSpacing.sm),
              MetricTile(
                icon: Icons.article_outlined,
                label: strings.t('extension.feature.arxiv.title'),
                value: strings.t('extension.feature.arxiv.body'),
              ),
              const SizedBox(height: AppSpacing.sm),
              MetricTile(
                icon: Icons.account_circle_outlined,
                label: strings.t('extension.feature.account.title'),
                value: strings.t('extension.feature.account.body'),
              ),
            ],
          ),
        );
        if (compact) {
          return Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              titleBlock,
              const SizedBox(height: AppSpacing.xl),
              heroPanel,
            ],
          );
        }
        return Row(
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            Expanded(flex: 6, child: titleBlock),
            const SizedBox(width: AppSpacing.xxl),
            Expanded(flex: 5, child: heroPanel),
          ],
        );
      },
    );
  }
}

// ─── Features grid ────────────────────────────────────────────────────────────

class _FeatureGrid extends StatelessWidget {
  const _FeatureGrid();

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final features = [
      _FeatureTile(
        icon: Icons.memory,
        title: strings.t('extension.feature.local.title'),
        body: strings.t('extension.feature.local.body'),
      ),
      _FeatureTile(
        icon: Icons.cloud_sync_outlined,
        title: strings.t('extension.feature.cloud.title'),
        body: strings.t('extension.feature.cloud.body'),
      ),
      _FeatureTile(
        icon: Icons.web_stories_outlined,
        title: strings.t('extension.feature.overleaf.title'),
        body: strings.t('extension.feature.overleaf.body'),
      ),
      _FeatureTile(
        icon: Icons.article_outlined,
        title: strings.t('extension.feature.arxiv.title'),
        body: strings.t('extension.feature.arxiv.body'),
      ),
      _FeatureTile(
        icon: Icons.account_circle_outlined,
        title: strings.t('extension.feature.account.title'),
        body: strings.t('extension.feature.account.body'),
      ),
      _FeatureTile(
        icon: Icons.space_dashboard_outlined,
        title: strings.t('extension.feature.sidepanel.title'),
        body: strings.t('extension.feature.sidepanel.body'),
      ),
    ];
    return LayoutBuilder(
      builder: (context, constraints) {
        final columns = constraints.maxWidth < AppBreakpoints.tablet
            ? 1
            : (constraints.maxWidth < 900 ? 2 : 3);
        return GridView.count(
          crossAxisCount: columns,
          shrinkWrap: true,
          physics: const NeverScrollableScrollPhysics(),
          crossAxisSpacing: AppSpacing.md,
          mainAxisSpacing: AppSpacing.md,
          childAspectRatio: columns == 1 ? 3.4 : (columns == 2 ? 2.0 : 1.45),
          children: features,
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

// ─── Install guide ────────────────────────────────────────────────────────────

class _InstallGuideGrid extends StatelessWidget {
  const _InstallGuideGrid();

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final cards = [
      _InstallCard(
        icon: Icons.shop_outlined,
        title: strings.t('extension.install.chrome'),
        steps: strings.t('extension.installGuide.chromeSteps'),
        onTap: () => openExternalUrl(_chromeWebStoreUrl),
      ),
      _InstallCard(
        icon: Icons.shop_outlined,
        title: strings.t('extension.install.edge'),
        steps: strings.t('extension.installGuide.edgeSteps'),
        onTap: () => openExternalUrl(_edgeAddonsUrl),
      ),
      _InstallCard(
        icon: Icons.shop_outlined,
        title: strings.t('extension.install.firefox'),
        steps: strings.t('extension.installGuide.firefoxSteps'),
        onTap: () => openExternalUrl(_firefoxAddonsUrl),
      ),
      _InstallCard(
        icon: Icons.shop_outlined,
        title: strings.t('extension.install.safari'),
        steps: strings.t('extension.installGuide.safariSteps'),
        onTap: () => openExternalUrl(_safariAppStoreUrl),
      ),
    ];
    return LayoutBuilder(
      builder: (context, constraints) {
        final columns = constraints.maxWidth < AppBreakpoints.tablet
            ? 1
            : (constraints.maxWidth < 900 ? 2 : 4);
        return GridView.count(
          crossAxisCount: columns,
          shrinkWrap: true,
          physics: const NeverScrollableScrollPhysics(),
          crossAxisSpacing: AppSpacing.md,
          mainAxisSpacing: AppSpacing.md,
          childAspectRatio: columns == 1
              ? 4.0
              : (columns == 2
                  ? 2.4
                  : 1.6),
          children: cards,
        );
      },
    );
  }
}

class _InstallCard extends StatelessWidget {
  final IconData icon;
  final String title;
  final String steps;
  final VoidCallback onTap;

  const _InstallCard({
    required this.icon,
    required this.title,
    required this.steps,
    required this.onTap,
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
          const SizedBox(height: AppSpacing.sm),
          Text(title, style: theme.textTheme.titleMedium),
          const SizedBox(height: AppSpacing.xs),
          Expanded(
            child: Text(
              steps,
              style: theme.textTheme.bodyMedium,
              softWrap: true,
            ),
          ),
          const SizedBox(height: AppSpacing.xs),
          Align(
            alignment: Alignment.centerLeft,
            child: FilledButton.tonalIcon(
              onPressed: onTap,
              icon: const Icon(Icons.open_in_new, size: 18),
              label: const Text('前往'),
            ),
          ),
        ],
      ),
    );
  }
}

// ─── Usage guide ─────────────────────────────────────────────────────────────

class _UsageGuide extends StatelessWidget {
  const _UsageGuide();

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(
          '使用说明',
          style: theme.textTheme.headlineSmall,
        ),
        const SizedBox(height: AppSpacing.md),
        LayoutBuilder(
          builder: (context, constraints) {
            final wide = constraints.maxWidth > 900;
            final items = [
              _UsageItem(
                icon: Icons.open_in_new,
                title: '弹出窗口',
                body: '点击工具栏图标打开弹出窗口，选择文件后立即转换。',
              ),
              _UsageItem(
                icon: Icons.space_dashboard_outlined,
                title: '侧边面板（Chrome/Edge）',
                body: '通过侧边面板管理 Jobs / Billing / Feedback / Account。',
              ),
              _UsageItem(
                icon: Icons.account_tree_outlined,
                title: '网站集成',
                body: 'Overleaf 项目页右下角、arXiv 摘要页会出现转换按钮。',
              ),
            ];
            if (wide) {
              return Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  for (var i = 0; i < items.length; i++) ...[
                    Expanded(child: items[i]),
                    if (i < items.length - 1)
                      const SizedBox(width: AppSpacing.md),
                  ],
                ],
              );
            }
            return Column(
              children: [
                for (var i = 0; i < items.length; i++) ...[
                  items[i],
                  if (i < items.length - 1) const SizedBox(height: AppSpacing.md),
                ],
              ],
            );
          },
        ),
      ],
    );
  }
}

class _UsageItem extends StatelessWidget {
  final IconData icon;
  final String title;
  final String body;

  const _UsageItem({
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
          const SizedBox(height: AppSpacing.sm),
          Text(title, style: theme.textTheme.titleMedium),
          const SizedBox(height: AppSpacing.xs),
          Text(body, style: theme.textTheme.bodyMedium),
        ],
      ),
    );
  }
}

// ─── Footer band ─────────────────────────────────────────────────────────────

class _ExtensionFooterBand extends StatelessWidget {
  const _ExtensionFooterBand();

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
            'Tex2Doc 浏览器插件由启哲科技出品',
            style: theme.textTheme.titleMedium,
          ),
          FilledButton.tonalIcon(
            onPressed: () => openExternalUrl(_chromeWebStoreUrl),
            icon: const Icon(Icons.extension),
            label: const Text('安装 Chrome 扩展'),
          ),
        ],
      ),
    );
  }
}
