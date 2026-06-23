import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';

import 'commercial_api.dart';
import 'file_web_stub.dart'
    if (dart.library.js_interop) 'file_web_utils_web.dart';
import 'logger.dart';
import 'ui/app_components.dart';
import 'ui/app_i18n.dart';
import 'ui/app_preferences.dart';
import 'ui/app_theme.dart';
import 'ui/app_tokens.dart';

const _appIconAsset = 'assets/app_icon.jpg';

class DocEngineApp extends StatefulWidget {
  final bool isWeb;

  const DocEngineApp({super.key, required this.isWeb});

  @override
  State<DocEngineApp> createState() => _DocEngineAppState();
}

class _DocEngineAppState extends State<DocEngineApp> {
  AppThemeTone _themeTone = AppThemeTone.defaultTone;
  AppLocale _locale = AppLocale.zhCn;

  @override
  void initState() {
    super.initState();
    _loadPreferences();
  }

  Future<void> _loadPreferences() async {
    final storedTheme = await AppPreferences.read('ui.theme');
    final storedLocale = await AppPreferences.read('ui.locale');
    if (!mounted) return;
    setState(() {
      _themeTone = AppThemeToneLabel.fromStorageKey(storedTheme);
      _locale = AppLocaleMeta.fromStorageKey(storedLocale);
    });
  }

  Future<void> _setTheme(AppThemeTone tone) async {
    await AppPreferences.write('ui.theme', tone.storageKey);
    if (mounted) setState(() => _themeTone = tone);
  }

  Future<void> _setLocale(AppLocale locale) async {
    await AppPreferences.write('ui.locale', locale.storageKey);
    if (mounted) setState(() => _locale = locale);
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Tex2Doc',
      debugShowCheckedModeBanner: false,
      locale: _locale.locale,
      supportedLocales: AppLocale.values.map((locale) => locale.locale),
      localizationsDelegates: const [
        AppStringsDelegate(),
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ],
      theme: AppTheme.light(_themeTone),
      darkTheme: AppTheme.dark(_themeTone),
      themeMode: _themeTone == AppThemeTone.dark
          ? ThemeMode.dark
          : ThemeMode.light,
      home: _WorkspaceShell(
        isWeb: widget.isWeb,
        themeTone: _themeTone,
        locale: _locale,
        onThemeChanged: _setTheme,
        onLocaleChanged: _setLocale,
      ),
    );
  }
}

enum _WorkspaceSection { dashboard, account, recharge, convert }

extension _WorkspaceSectionMeta on _WorkspaceSection {
  IconData get icon => switch (this) {
    _WorkspaceSection.dashboard => Icons.dashboard_outlined,
    _WorkspaceSection.account => Icons.person_outline,
    _WorkspaceSection.recharge => Icons.payments_outlined,
    _WorkspaceSection.convert => Icons.sync_alt,
  };

  String label(AppStrings strings) => switch (this) {
    _WorkspaceSection.dashboard => strings.t('nav.dashboard'),
    _WorkspaceSection.account => strings.t('nav.account'),
    _WorkspaceSection.recharge => strings.t('nav.recharge'),
    _WorkspaceSection.convert => strings.t('nav.convert'),
  };
}

class _WorkspaceShell extends StatefulWidget {
  final bool isWeb;
  final AppThemeTone themeTone;
  final AppLocale locale;
  final ValueChanged<AppThemeTone> onThemeChanged;
  final ValueChanged<AppLocale> onLocaleChanged;

  const _WorkspaceShell({
    required this.isWeb,
    required this.themeTone,
    required this.locale,
    required this.onThemeChanged,
    required this.onLocaleChanged,
  });

  @override
  State<_WorkspaceShell> createState() => _WorkspaceShellState();
}

class _WorkspaceShellState extends State<_WorkspaceShell> {
  _WorkspaceSection _selectedSection = _WorkspaceSection.dashboard;
  String _apiBaseUrl = 'http://127.0.0.1:8080/v1/';
  String? _accessToken;

  void _selectSection(_WorkspaceSection section) {
    setState(() => _selectedSection = section);
  }

  void _handleSignedIn(String apiBaseUrl, String accessToken) {
    setState(() {
      _apiBaseUrl = apiBaseUrl;
      _accessToken = accessToken;
    });
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final platform = widget.isWeb ? 'Web' : 'Desktop';

    DocLogger.instance.i(LogTags.app, 'DocEngineApp build, platform=$platform');

    return Scaffold(
      body: SafeArea(
        child: LayoutBuilder(
          builder: (context, constraints) {
            final compact = constraints.maxWidth < AppBreakpoints.tablet;
            final content = _WorkspaceContent(
              selectedSection: _selectedSection,
              compact: compact,
              apiBaseUrl: _apiBaseUrl,
              accessToken: _accessToken,
              onSignedIn: _handleSignedIn,
              onSectionChanged: _selectSection,
            );

            if (compact) {
              return Column(
                children: [
                  _TopBar(
                    platform: platform,
                    themeTone: widget.themeTone,
                    locale: widget.locale,
                    onThemeChanged: widget.onThemeChanged,
                    onLocaleChanged: widget.onLocaleChanged,
                  ),
                  Expanded(child: content),
                ],
              );
            }

            return Row(
              children: [
                _Sidebar(
                  strings: strings,
                  selectedSection: _selectedSection,
                  onSectionChanged: _selectSection,
                ),
                Expanded(
                  child: Column(
                    children: [
                      _TopBar(
                        platform: platform,
                        themeTone: widget.themeTone,
                        locale: widget.locale,
                        onThemeChanged: widget.onThemeChanged,
                        onLocaleChanged: widget.onLocaleChanged,
                      ),
                      Expanded(child: content),
                    ],
                  ),
                ),
              ],
            );
          },
        ),
      ),
    );
  }
}

class _Sidebar extends StatelessWidget {
  final AppStrings strings;
  final _WorkspaceSection selectedSection;
  final ValueChanged<_WorkspaceSection> onSectionChanged;

  const _Sidebar({
    required this.strings,
    required this.selectedSection,
    required this.onSectionChanged,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      width: 248,
      padding: const EdgeInsets.all(AppSpacing.lg),
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        border: Border(right: BorderSide(color: theme.colorScheme.outline)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              ClipRRect(
                borderRadius: BorderRadius.circular(AppRadius.sm),
                child: Image.asset(
                  _appIconAsset,
                  width: 44,
                  height: 44,
                  fit: BoxFit.cover,
                ),
              ),
              const SizedBox(width: AppSpacing.sm),
              Expanded(
                child: Text(
                  strings.t('app.title'),
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.titleLarge,
                ),
              ),
            ],
          ),
          const SizedBox(height: AppSpacing.xs),
          Text(strings.t('app.subtitle'), style: theme.textTheme.bodySmall),
          const SizedBox(height: AppSpacing.xl),
          for (final section in _WorkspaceSection.values)
            _NavItem(
              icon: section.icon,
              label: section.label(strings),
              selected: selectedSection == section,
              onTap: () => onSectionChanged(section),
            ),
          const Spacer(),
          StatusPill(
            icon: Icons.lock_outline,
            label: strings.t('common.permissionDenied'),
            color: theme.disabledColor,
          ),
        ],
      ),
    );
  }
}

class _NavItem extends StatelessWidget {
  final IconData icon;
  final String label;
  final bool selected;
  final VoidCallback onTap;

  const _NavItem({
    required this.icon,
    required this.label,
    required this.onTap,
    this.selected = false,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Semantics(
      button: true,
      selected: selected,
      label: label,
      child: Padding(
        padding: const EdgeInsets.only(bottom: AppSpacing.xs),
        child: Material(
          color: Colors.transparent,
          child: InkWell(
            borderRadius: BorderRadius.circular(AppRadius.md),
            onTap: onTap,
            child: AnimatedContainer(
              duration: AppMotion.fast,
              curve: AppMotion.curve,
              padding: const EdgeInsets.symmetric(
                horizontal: AppSpacing.md,
                vertical: AppSpacing.sm,
              ),
              decoration: BoxDecoration(
                color: selected
                    ? theme.colorScheme.primary.withValues(alpha: 0.10)
                    : Colors.transparent,
                borderRadius: BorderRadius.circular(AppRadius.md),
              ),
              child: Row(
                children: [
                  Icon(
                    icon,
                    size: 18,
                    color: selected
                        ? theme.colorScheme.primary
                        : theme.hintColor,
                  ),
                  const SizedBox(width: AppSpacing.sm),
                  Expanded(
                    child: Text(
                      label,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.labelLarge?.copyWith(
                        color: selected ? theme.colorScheme.primary : null,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _TopBar extends StatelessWidget {
  final String platform;
  final AppThemeTone themeTone;
  final AppLocale locale;
  final ValueChanged<AppThemeTone> onThemeChanged;
  final ValueChanged<AppLocale> onLocaleChanged;

  const _TopBar({
    required this.platform,
    required this.themeTone,
    required this.locale,
    required this.onThemeChanged,
    required this.onLocaleChanged,
  });

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.symmetric(
        horizontal: AppSpacing.lg,
        vertical: AppSpacing.md,
      ),
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        border: Border(bottom: BorderSide(color: theme.colorScheme.outline)),
      ),
      child: Wrap(
        spacing: AppSpacing.md,
        runSpacing: AppSpacing.sm,
        crossAxisAlignment: WrapCrossAlignment.center,
        children: [
          ClipRRect(
            borderRadius: BorderRadius.circular(AppRadius.sm),
            child: Image.asset(
              _appIconAsset,
              width: 40,
              height: 40,
              fit: BoxFit.cover,
            ),
          ),
          ConstrainedBox(
            constraints: const BoxConstraints(minWidth: 220),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  strings.t('app.title'),
                  style: theme.textTheme.titleMedium,
                ),
                Text(
                  '${strings.t('topbar.platform')}: $platform',
                  style: theme.textTheme.bodySmall,
                ),
              ],
            ),
          ),
          _ThemeDropdown(value: themeTone, onChanged: onThemeChanged),
          _LocaleDropdown(value: locale, onChanged: onLocaleChanged),
        ],
      ),
    );
  }
}

class _ThemeDropdown extends StatelessWidget {
  final AppThemeTone value;
  final ValueChanged<AppThemeTone> onChanged;

  const _ThemeDropdown({required this.value, required this.onChanged});

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    return DropdownButton<AppThemeTone>(
      value: value,
      underline: const SizedBox.shrink(),
      borderRadius: BorderRadius.circular(AppRadius.md),
      items: AppThemeTone.values.map((tone) {
        return DropdownMenuItem(
          value: tone,
          child: Text(strings.t('theme.${tone.storageKey}')),
        );
      }).toList(),
      onChanged: (tone) {
        if (tone != null) onChanged(tone);
      },
    );
  }
}

class _LocaleDropdown extends StatelessWidget {
  final AppLocale value;
  final ValueChanged<AppLocale> onChanged;

  const _LocaleDropdown({required this.value, required this.onChanged});

  @override
  Widget build(BuildContext context) {
    return DropdownButton<AppLocale>(
      value: value,
      underline: const SizedBox.shrink(),
      borderRadius: BorderRadius.circular(AppRadius.md),
      items: AppLocale.values.map((locale) {
        return DropdownMenuItem(value: locale, child: Text(locale.label));
      }).toList(),
      onChanged: (locale) {
        if (locale != null) onChanged(locale);
      },
    );
  }
}

class _WorkspaceContent extends StatelessWidget {
  final _WorkspaceSection selectedSection;
  final bool compact;
  final String apiBaseUrl;
  final String? accessToken;
  final void Function(String apiBaseUrl, String accessToken) onSignedIn;
  final ValueChanged<_WorkspaceSection> onSectionChanged;

  const _WorkspaceContent({
    required this.selectedSection,
    required this.compact,
    required this.apiBaseUrl,
    required this.accessToken,
    required this.onSignedIn,
    required this.onSectionChanged,
  });

  @override
  Widget build(BuildContext context) {
    final body = switch (selectedSection) {
      _WorkspaceSection.dashboard => <Widget>[
        const _MetricsRow(),
        const SizedBox(height: AppSpacing.lg),
        _ResponsiveCards(
          apiBaseUrl: apiBaseUrl,
          accessToken: accessToken,
          onSignedIn: onSignedIn,
        ),
      ],
      _WorkspaceSection.account => <Widget>[
        _CommercialApiPanel(onSignedIn: onSignedIn),
        const SizedBox(height: AppSpacing.lg),
        _AccountOverviewPanel(apiBaseUrl: apiBaseUrl, accessToken: accessToken),
      ],
      _WorkspaceSection.recharge => <Widget>[
        _RechargePanel(apiBaseUrl: apiBaseUrl, accessToken: accessToken),
      ],
      _WorkspaceSection.convert => <Widget>[
        _ConvertPanel(apiBaseUrl: apiBaseUrl, accessToken: accessToken),
      ],
    };

    return PageContainer(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (compact) ...[
            _SectionTabs(
              selectedSection: selectedSection,
              onSectionChanged: onSectionChanged,
            ),
            const SizedBox(height: AppSpacing.lg),
          ],
          ...body,
        ],
      ),
    );
  }
}

class _SectionTabs extends StatelessWidget {
  final _WorkspaceSection selectedSection;
  final ValueChanged<_WorkspaceSection> onSectionChanged;

  const _SectionTabs({
    required this.selectedSection,
    required this.onSectionChanged,
  });

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    return SegmentedButton<_WorkspaceSection>(
      showSelectedIcon: false,
      segments: [
        for (final section in _WorkspaceSection.values)
          ButtonSegment<_WorkspaceSection>(
            value: section,
            icon: Icon(section.icon),
            label: Text(section.label(strings)),
          ),
      ],
      selected: {selectedSection},
      onSelectionChanged: (sections) {
        if (sections.isNotEmpty) onSectionChanged(sections.first);
      },
    );
  }
}

class _MetricsRow extends StatelessWidget {
  const _MetricsRow();

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    return LayoutBuilder(
      builder: (context, constraints) {
        final columns = constraints.maxWidth < AppBreakpoints.mobile ? 1 : 3;
        return GridView.count(
          shrinkWrap: true,
          physics: const NeverScrollableScrollPhysics(),
          crossAxisCount: columns,
          crossAxisSpacing: AppSpacing.md,
          mainAxisSpacing: AppSpacing.md,
          childAspectRatio: columns == 1 ? 4.8 : 2.4,
          children: [
            MetricTile(
              label: strings.t('metrics.engine'),
              value: strings.t('common.ready'),
              icon: Icons.memory,
            ),
            MetricTile(
              label: strings.t('metrics.quota'),
              value: '0 / 0',
              icon: Icons.speed,
            ),
            MetricTile(
              label: strings.t('metrics.document'),
              value: strings.t('common.empty'),
              icon: Icons.description_outlined,
            ),
          ],
        );
      },
    );
  }
}

class _ResponsiveCards extends StatelessWidget {
  final String apiBaseUrl;
  final String? accessToken;
  final void Function(String apiBaseUrl, String accessToken) onSignedIn;

  const _ResponsiveCards({
    required this.apiBaseUrl,
    required this.accessToken,
    required this.onSignedIn,
  });

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final stack = constraints.maxWidth < AppBreakpoints.tablet;
        final account = _CommercialApiPanel(onSignedIn: onSignedIn);
        final convert = _ConvertPanel(
          apiBaseUrl: apiBaseUrl,
          accessToken: accessToken,
        );
        final recharge = _RechargePanel(
          apiBaseUrl: apiBaseUrl,
          accessToken: accessToken,
        );
        if (stack) {
          return Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              account,
              const SizedBox(height: AppSpacing.lg),
              recharge,
              const SizedBox(height: AppSpacing.lg),
              convert,
            ],
          );
        }
        return Column(
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Expanded(child: account),
                const SizedBox(width: AppSpacing.lg),
                Expanded(child: recharge),
              ],
            ),
            const SizedBox(height: AppSpacing.lg),
            convert,
          ],
        );
      },
    );
  }
}

class _CommercialApiPanel extends StatefulWidget {
  final void Function(String apiBaseUrl, String accessToken) onSignedIn;

  const _CommercialApiPanel({required this.onSignedIn});

  @override
  State<_CommercialApiPanel> createState() => _CommercialApiPanelState();
}

class _CommercialApiPanelState extends State<_CommercialApiPanel> {
  final _baseUrlController = TextEditingController(
    text: 'http://127.0.0.1:8080/v1/',
  );
  final _emailController = TextEditingController(text: 'demo@example.com');
  final _passwordController = TextEditingController(text: 'demo');

  String? _accessToken;
  String? _status;
  bool _busy = false;

  @override
  void dispose() {
    _baseUrlController.dispose();
    _emailController.dispose();
    _passwordController.dispose();
    super.dispose();
  }

  CommercialApiClient _client() => CommercialApiClient(_baseUrlController.text);

  Future<void> _run(
    Future<void> Function(CommercialApiClient client, AppStrings strings)
    action,
  ) async {
    if (_busy) return;
    final strings = AppStrings.of(context);
    setState(() {
      _busy = true;
      _status = strings.t('status.working');
    });
    try {
      await action(_client(), strings);
    } on Object catch (e) {
      if (!mounted) return;
      setState(() => _status = e.toString());
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(strings.t('error.network'))));
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _register() async {
    await _run((client, strings) async {
      final auth = await client.register(
        email: _emailController.text.trim(),
        password: _passwordController.text,
      );
      if (!mounted) return;
      setState(() {
        _accessToken = auth.accessToken;
        _status = strings.t('account.registered').fill({
          'email': auth.user.email,
          'plan': auth.user.planId,
        });
      });
      widget.onSignedIn(_baseUrlController.text, auth.accessToken);
    });
  }

  Future<void> _login() async {
    await _run((client, strings) async {
      final auth = await client.login(
        email: _emailController.text.trim(),
        password: _passwordController.text,
      );
      if (!mounted) return;
      setState(() {
        _accessToken = auth.accessToken;
        _status = strings.t('account.signedIn').fill({
          'email': auth.user.email,
          'plan': auth.user.planId,
        });
      });
      widget.onSignedIn(_baseUrlController.text, auth.accessToken);
    });
  }

  Future<void> _usage() async {
    final token = _accessToken;
    final strings = AppStrings.of(context);
    if (token == null) {
      setState(() => _status = strings.t('status.signInFirst'));
      return;
    }
    await _run((client, strings) async {
      final usage = await client.usage(token);
      if (!mounted) return;
      setState(() {
        _status = strings.t('account.usage').fill({
          'plan': usage.planId,
          'used': usage.cloudConversionsUsed,
          'limit': usage.cloudConversionsLimit,
          'remaining': usage.cloudConversionsRemaining,
          'entitlement': _formatEntitlement(strings, usage),
        });
      });
    });
  }

  Future<void> _plans() async {
    await _run((client, strings) async {
      final plans = await client.plans();
      if (!mounted) return;
      setState(() {
        _status = plans.isEmpty
            ? strings.t('empty.noData')
            : plans.map((plan) => plan.label).join('\n');
      });
    });
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final theme = Theme.of(context);

    return AppCard(
      key: const ValueKey('commercial-api-card'),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          AppSectionHeader(
            title: strings.t('account.title'),
            description: strings.t('account.description'),
          ),
          const SizedBox(height: AppSpacing.lg),
          AppTextField(
            controller: _baseUrlController,
            label: strings.t('account.apiBaseUrl'),
          ),
          const SizedBox(height: AppSpacing.sm),
          AppTextField(
            controller: _emailController,
            label: strings.t('account.email'),
          ),
          const SizedBox(height: AppSpacing.sm),
          AppTextField(
            controller: _passwordController,
            label: strings.t('account.password'),
            obscureText: true,
          ),
          const SizedBox(height: AppSpacing.md),
          Wrap(
            spacing: AppSpacing.sm,
            runSpacing: AppSpacing.sm,
            children: [
              FilledButton.icon(
                onPressed: _busy ? null : _register,
                icon: const Icon(Icons.person_add),
                label: Text(strings.t('common.register')),
              ),
              FilledButton.tonalIcon(
                onPressed: _busy ? null : _login,
                icon: const Icon(Icons.login),
                label: Text(strings.t('common.login')),
              ),
              OutlinedButton.icon(
                onPressed: _busy ? null : _usage,
                icon: const Icon(Icons.speed),
                label: Text(strings.t('common.refresh')),
              ),
              OutlinedButton.icon(
                onPressed: _busy ? null : _plans,
                icon: const Icon(Icons.receipt_long),
                label: Text(strings.t('common.plans')),
              ),
            ],
          ),
          const SizedBox(height: AppSpacing.md),
          AnimatedSwitcher(
            duration: AppMotion.normal,
            child: _busy
                ? LoadingState(
                    key: const ValueKey('account-loading'),
                    label: strings.t('common.loading'),
                  )
                : (_status == null
                      ? EmptyState(
                          key: const ValueKey('account-empty'),
                          label: strings.t('empty.noData'),
                        )
                      : Text(
                          _status!,
                          key: ValueKey(_status),
                          style: theme.textTheme.bodySmall,
                        )),
          ),
        ],
      ),
    );
  }
}

String _formatEntitlement(AppStrings strings, UsageSummary usage) {
  final parts = <String>[];
  if (usage.countBalance > 0) {
    parts.add(
      strings.t('metrics.countBalance').fill({'count': usage.countBalance}),
    );
  }
  final validUntil = usage.dateValidUntil;
  if (validUntil != null && validUntil.isNotEmpty) {
    parts.add(strings.t('metrics.dateValidUntil').fill({'time': validUntil}));
  }
  return parts.isEmpty ? strings.t('metrics.previewQuota') : parts.join(', ');
}

class _AccountOverviewPanel extends StatefulWidget {
  final String apiBaseUrl;
  final String? accessToken;

  const _AccountOverviewPanel({
    required this.apiBaseUrl,
    required this.accessToken,
  });

  @override
  State<_AccountOverviewPanel> createState() => _AccountOverviewPanelState();
}

class _AccountOverviewPanelState extends State<_AccountOverviewPanel> {
  UserProfile? _profile;
  UsageSummary? _usage;
  List<RechargeRecord> _recharges = const [];
  List<ConversionJob> _conversions = const [];
  String? _status;
  bool _busy = false;

  Future<void> _refresh() async {
    final strings = AppStrings.of(context);
    final token = widget.accessToken;
    if (token == null) {
      setState(() => _status = strings.t('account.signInGate'));
      return;
    }
    if (_busy) return;
    setState(() {
      _busy = true;
      _status = strings.t('status.working');
    });
    try {
      final client = CommercialApiClient(widget.apiBaseUrl);
      final profile = await client.me(token);
      final usage = await client.usage(token);
      final recharges = await client.recharges(token);
      final conversions = await client.conversions(token);
      if (!mounted) return;
      setState(() {
        _profile = profile;
        _usage = usage;
        _recharges = recharges;
        _conversions = conversions;
        _status = strings.t('account.overviewLoaded').fill({
          'recharges': recharges.length,
          'conversions': conversions.length,
        });
      });
    } on Object catch (e) {
      if (mounted) setState(() => _status = e.toString());
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final theme = Theme.of(context);
    final signedIn = widget.accessToken != null;
    return AppCard(
      key: const ValueKey('account-overview-card'),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          AppSectionHeader(
            title: strings.t('account.overviewTitle'),
            description: strings.t('account.overviewDescription'),
          ),
          const SizedBox(height: AppSpacing.md),
          Wrap(
            spacing: AppSpacing.sm,
            runSpacing: AppSpacing.sm,
            children: [
              OutlinedButton.icon(
                onPressed: signedIn && !_busy ? _refresh : null,
                icon: const Icon(Icons.manage_search),
                label: Text(strings.t('account.queryRecords')),
              ),
              StatusPill(
                icon: signedIn ? Icons.verified_user_outlined : Icons.lock,
                label: signedIn
                    ? strings.t('account.signedInShort')
                    : strings.t('account.signInGate'),
                color: signedIn
                    ? theme.colorScheme.primary
                    : theme.disabledColor,
              ),
            ],
          ),
          const SizedBox(height: AppSpacing.md),
          if (_busy)
            LoadingState(label: strings.t('common.loading'))
          else ...[
            if (_profile != null || _usage != null)
              _KeyValueList(
                entries: [
                  if (_profile != null)
                    '${strings.t('account.email')}: ${_profile!.email}',
                  if (_profile != null)
                    '${strings.t('account.plan')}: ${_profile!.planId}',
                  if (_usage != null)
                    '${strings.t('metrics.quota')}: ${_usage!.cloudConversionsUsed}/${_usage!.cloudConversionsLimit}',
                  if (_usage != null)
                    '${strings.t('metrics.entitlement')}: ${_formatEntitlement(strings, _usage!)}',
                ],
              ),
            if (_status != null) ...[
              const SizedBox(height: AppSpacing.sm),
              Text(_status!, style: theme.textTheme.bodySmall),
            ],
            const SizedBox(height: AppSpacing.md),
            _RecordPreview(
              title: strings.t('recharge.records'),
              emptyLabel: strings.t('empty.noData'),
              items: _recharges.map((record) => record.label).toList(),
            ),
            const SizedBox(height: AppSpacing.md),
            _RecordPreview(
              title: strings.t('convert.records'),
              emptyLabel: strings.t('empty.noData'),
              items: _conversions
                  .map((job) => '${job.jobId} / ${job.status.name}')
                  .toList(),
            ),
          ],
        ],
      ),
    );
  }
}

class _RechargePanel extends StatefulWidget {
  final String apiBaseUrl;
  final String? accessToken;

  const _RechargePanel({required this.apiBaseUrl, required this.accessToken});

  @override
  State<_RechargePanel> createState() => _RechargePanelState();
}

class _RechargePanelState extends State<_RechargePanel> {
  RechargeOptions? _options;
  List<RechargeRecord> _records = const [];
  String? _status;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    if (widget.accessToken != null) {
      unawaited(_loadRecords());
    }
  }

  @override
  void didUpdateWidget(covariant _RechargePanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.accessToken != widget.accessToken ||
        oldWidget.apiBaseUrl != widget.apiBaseUrl) {
      _loadRecords();
    }
  }

  Future<void> _loadRecords() async {
    final token = widget.accessToken;
    if (token == null) return;
    try {
      final client = CommercialApiClient(widget.apiBaseUrl);
      final options = await client.rechargeOptions();
      final records = await client.recharges(token);
      if (!mounted) return;
      setState(() {
        _options = options;
        _records = records;
      });
    } on Object catch (e) {
      if (mounted) setState(() => _status = e.toString());
    }
  }

  Future<void> _recharge(String rechargeType, RechargePackage package) async {
    final strings = AppStrings.of(context);
    final token = widget.accessToken;
    if (token == null) {
      setState(() => _status = strings.t('recharge.signInRequired'));
      return;
    }
    if (_busy) return;
    setState(() {
      _busy = true;
      _status = strings.t('status.working');
    });
    try {
      final client = CommercialApiClient(widget.apiBaseUrl);
      final record = await client.createRecharge(
        accessToken: token,
        rechargeType: rechargeType,
        packageId: package.id,
        quantity: rechargeType == 'count' ? package.quantity : null,
      );
      final records = await client.recharges(token);
      if (!mounted) return;
      setState(() {
        _records = records;
        _status = strings.t('recharge.mockPaid').fill({
          'amount': (record.amountCents / 100).toStringAsFixed(0),
          'provider': record.provider,
        });
      });
    } on Object catch (e) {
      if (mounted) setState(() => _status = e.toString());
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final theme = Theme.of(context);
    final signedIn = widget.accessToken != null;
    final fallbackOptions = RechargeOptions(
      currency: 'CNY',
      provider: 'mock-pay',
      countPackages: [
        RechargePackage(
          id: 'count_3',
          name: '3 次',
          quantity: 3,
          amountCents: 300,
        ),
        RechargePackage(
          id: 'count_10',
          name: '10 次',
          quantity: 10,
          amountCents: 1000,
        ),
        RechargePackage(
          id: 'count_30',
          name: '30 次',
          quantity: 30,
          amountCents: 3000,
        ),
      ],
      datePackages: [
        RechargePackage(id: 'day', name: '日卡', quantity: 1, amountCents: 500),
        RechargePackage(id: 'week', name: '周卡', quantity: 7, amountCents: 1400),
        RechargePackage(
          id: 'month',
          name: '月卡',
          quantity: 30,
          amountCents: 3000,
        ),
        RechargePackage(
          id: 'year',
          name: '年卡',
          quantity: 365,
          amountCents: 12000,
        ),
      ],
    );
    final options = _options ?? fallbackOptions;
    return AppCard(
      key: const ValueKey('recharge-card'),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          AppSectionHeader(
            title: strings.t('recharge.title'),
            description: strings.t('recharge.description'),
          ),
          const SizedBox(height: AppSpacing.md),
          Wrap(
            spacing: AppSpacing.sm,
            runSpacing: AppSpacing.sm,
            children: [
              OutlinedButton.icon(
                onPressed: signedIn && !_busy ? _loadRecords : null,
                icon: const Icon(Icons.history),
                label: Text(strings.t('recharge.queryRecords')),
              ),
              StatusPill(
                icon: signedIn ? Icons.payments_outlined : Icons.lock,
                label: signedIn
                    ? strings.t('recharge.mockProvider')
                    : strings.t('recharge.signInRequired'),
                color: signedIn
                    ? theme.colorScheme.primary
                    : theme.disabledColor,
              ),
            ],
          ),
          const SizedBox(height: AppSpacing.md),
          Text(
            strings.t('recharge.countTitle'),
            style: theme.textTheme.titleSmall,
          ),
          const SizedBox(height: AppSpacing.sm),
          _RechargeButtons(
            enabled: signedIn && !_busy,
            currency: options.currency,
            packages: options.countPackages,
            onRecharge: (package) => _recharge('count', package),
          ),
          const SizedBox(height: AppSpacing.md),
          Text(
            strings.t('recharge.dateTitle'),
            style: theme.textTheme.titleSmall,
          ),
          const SizedBox(height: AppSpacing.sm),
          _RechargeButtons(
            enabled: signedIn && !_busy,
            currency: options.currency,
            packages: options.datePackages,
            onRecharge: (package) => _recharge('date', package),
          ),
          const SizedBox(height: AppSpacing.md),
          if (_busy)
            LoadingState(label: strings.t('common.loading'))
          else if (_status != null)
            Text(_status!, style: theme.textTheme.bodySmall),
          const SizedBox(height: AppSpacing.md),
          _RecordPreview(
            title: strings.t('recharge.records'),
            emptyLabel: strings.t('empty.noData'),
            items: _records.map((record) => record.label).toList(),
          ),
        ],
      ),
    );
  }
}

class _RechargeButtons extends StatelessWidget {
  final bool enabled;
  final String currency;
  final List<RechargePackage> packages;
  final ValueChanged<RechargePackage> onRecharge;

  const _RechargeButtons({
    required this.enabled,
    required this.currency,
    required this.packages,
    required this.onRecharge,
  });

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: AppSpacing.sm,
      runSpacing: AppSpacing.sm,
      children: [
        for (final package in packages)
          FilledButton.tonalIcon(
            onPressed: enabled ? () => onRecharge(package) : null,
            icon: const Icon(Icons.add_card),
            label: Text(package.priceLabel(currency)),
          ),
      ],
    );
  }
}

class _KeyValueList extends StatelessWidget {
  final List<String> entries;

  const _KeyValueList({required this.entries});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final entry in entries)
          Padding(
            padding: const EdgeInsets.only(bottom: AppSpacing.xs),
            child: Text(entry, style: theme.textTheme.bodySmall),
          ),
      ],
    );
  }
}

class _RecordPreview extends StatelessWidget {
  final String title;
  final String emptyLabel;
  final List<String> items;

  const _RecordPreview({
    required this.title,
    required this.emptyLabel,
    required this.items,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final visibleItems = items.take(6).toList(growable: false);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(title, style: theme.textTheme.titleSmall),
        const SizedBox(height: AppSpacing.xs),
        if (visibleItems.isEmpty)
          Text(emptyLabel, style: theme.textTheme.bodySmall)
        else
          for (final item in visibleItems)
            Padding(
              padding: const EdgeInsets.only(bottom: AppSpacing.xs),
              child: Text(item, style: theme.textTheme.bodySmall),
            ),
      ],
    );
  }
}

enum _ConvertState { idle, converting, success, error }

class _ConvertPanel extends StatefulWidget {
  final String apiBaseUrl;
  final String? accessToken;

  const _ConvertPanel({required this.apiBaseUrl, required this.accessToken});

  @override
  State<_ConvertPanel> createState() => _ConvertPanelState();
}

class _ConvertPanelState extends State<_ConvertPanel> {
  Uint8List? _zipBytes;
  String? _zipFileName;
  int? _zipSizeBytes;
  Uint8List? _docxBytes;
  int? _elapsedMs;
  List<String> _logs = const [];
  List<ConversionJob> _records = const [];
  _ConvertState _state = _ConvertState.idle;
  String? _statusText;
  String? _errorText;

  final _mainTexController = TextEditingController(text: 'main-jos.tex');

  @override
  void dispose() {
    _mainTexController.dispose();
    super.dispose();
  }

  void _addLog(String message) {
    if (!mounted) return;
    final now = DateTime.now();
    final stamp =
        '${now.hour.toString().padLeft(2, '0')}:${now.minute.toString().padLeft(2, '0')}:${now.second.toString().padLeft(2, '0')}';
    setState(() => _logs = ['$stamp $message', ..._logs].take(12).toList());
  }

  Future<void> _pickFile() async {
    final strings = AppStrings.of(context);
    if (widget.accessToken == null) {
      setState(() {
        _state = _ConvertState.error;
        _errorText = strings.t('convert.signInRequired');
      });
      return;
    }
    final result = await pickZipFile();
    if (result == null) return;

    final (bytes, fileName) = result;
    final sizeMB = bytes.length / (1024 * 1024);
    if (sizeMB >= 10) {
      setState(() {
        _state = _ConvertState.error;
        _errorText = strings.t('convert.fileTooLarge').fill({
          'size': sizeMB.toStringAsFixed(1),
        });
        _zipBytes = null;
        _zipFileName = null;
        _zipSizeBytes = null;
      });
      _addLog(strings.t('convert.logRejectedSize'));
      return;
    }

    setState(() {
      _zipBytes = bytes;
      _zipFileName = fileName;
      _zipSizeBytes = bytes.length;
      _docxBytes = null;
      _state = _ConvertState.idle;
      _statusText = null;
      _errorText = null;
    });
    _addLog(
      strings.t('convert.logFileSelected').fill({
        'file': fileName,
        'size': sizeMB.toStringAsFixed(2),
      }),
    );
  }

  Future<void> _startConvert() async {
    final bytes = _zipBytes;
    if (bytes == null) return;

    final strings = AppStrings.of(context);
    final mainTex = _mainTexController.text.trim().isEmpty
        ? 'main-jos.tex'
        : _mainTexController.text.trim();
    final token = widget.accessToken;
    if (token == null) {
      setState(() {
        _state = _ConvertState.error;
        _errorText = strings.t('convert.signInRequired');
      });
      return;
    }

    setState(() {
      _state = _ConvertState.converting;
      _statusText = strings.t('convert.converting');
      _errorText = null;
    });
    _addLog(strings.t('convert.logStarted').fill({'main': mainTex}));

    try {
      final t0 = DateTime.now();
      final docx = await _convertCloud(bytes, mainTex, token);
      if (!mounted) return;

      _elapsedMs = DateTime.now().difference(t0).inMilliseconds;
      _docxBytes = docx;
      const minDocxBytes = 1024;
      if (docx.length < minDocxBytes) {
        throw Exception('docx too small: ${docx.length} bytes');
      }
      if (docx[0] != 0x50 || docx[1] != 0x4B) {
        throw Exception('docx header is not ZIP');
      }

      setState(() {
        _state = _ConvertState.success;
        _statusText = strings.t('convert.cloudSuccess').fill({
          'size': (docx.length / 1024).toStringAsFixed(1),
          'elapsed': _elapsedMs ?? 0,
        });
      });
      _addLog(strings.t('convert.logFinished'));
      await _loadRecords(showStatus: false);
    } on Object catch (e) {
      if (!mounted) return;
      setState(() {
        _state = _ConvertState.error;
        _errorText = e.toString();
      });
      _addLog(strings.t('convert.logFailed').fill({'error': e}));
    }
  }

  Future<void> _loadRecords({bool showStatus = true}) async {
    final strings = AppStrings.of(context);
    final token = widget.accessToken;
    if (token == null) {
      setState(() {
        _state = _ConvertState.error;
        _errorText = strings.t('convert.signInRequired');
      });
      return;
    }
    try {
      final client = CommercialApiClient(widget.apiBaseUrl);
      final records = await client.conversions(token);
      if (!mounted) return;
      setState(() {
        _records = records;
        if (showStatus) {
          _statusText = strings.t('convert.recordsLoaded').fill({
            'count': records.length,
          });
        }
      });
      if (showStatus) _addLog(strings.t('convert.logRecordsLoaded'));
    } on Object catch (e) {
      if (mounted) setState(() => _errorText = e.toString());
    }
  }

  Future<Uint8List> _convertCloud(
    Uint8List bytes,
    String mainTex,
    String accessToken,
  ) async {
    final strings = AppStrings.of(context);
    final client = CommercialApiClient(widget.apiBaseUrl);
    _addLog(strings.t('convert.logUploading'));
    final upload = await client.uploadProjectZip(
      accessToken: accessToken,
      bytes: bytes,
      fileName: _zipFileName ?? 'project.zip',
    );
    _addLog(strings.t('convert.logUploaded').fill({'upload': upload.uploadId}));
    final created = await client.createConversion(
      accessToken: accessToken,
      uploadId: upload.uploadId,
      mainTex: mainTex,
      profile: 'jos',
      quality: 'high',
    );
    _addLog(strings.t('convert.logJobCreated').fill({'job': created.jobId}));
    var job = created;
    for (var attempt = 0; attempt < 120; attempt += 1) {
      if (job.status == ConversionStatus.completed) {
        final docx = await client.downloadConversionDocx(
          accessToken: accessToken,
          jobId: job.jobId,
        );
        return Uint8List.fromList(docx);
      }
      if (job.status == ConversionStatus.failed ||
          job.status == ConversionStatus.expired) {
        final detail = job.error ?? job.errorCode ?? job.status.name;
        throw Exception('cloud conversion failed: $detail');
      }
      await Future<void>.delayed(const Duration(seconds: 1));
      job = await client.getConversion(
        accessToken: accessToken,
        jobId: job.jobId,
      );
      _addLog(
        strings.t('convert.logPolling').fill({'status': job.status.name}),
      );
    }
    throw Exception('cloud conversion timeout: ${job.jobId}');
  }

  void _downloadDocx() {
    final docx = _docxBytes;
    if (docx == null) return;
    final base = _zipFileName?.replaceAll(RegExp(r'\.[^.]+$'), '') ?? 'output';
    downloadBlob(docx, '$base.docx');
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final tokens = Theme.of(context).extension<AppColorTokens>()!;

    final selectedFile = _zipFileName == null
        ? strings.t('convert.noFile')
        : '$_zipFileName (${((_zipSizeBytes ?? 0) / (1024 * 1024)).toStringAsFixed(2)} MB)';
    final signedIn = widget.accessToken != null;

    return AppCard(
      key: const ValueKey('convert-card'),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          AppSectionHeader(
            title: strings.t('convert.title'),
            description: strings.t('convert.description'),
          ),
          const SizedBox(height: AppSpacing.lg),
          _KeyValueList(
            entries: [
              strings.t('convert.stepUpload'),
              strings.t('convert.stepMainTex'),
              strings.t('convert.stepConvert'),
            ],
          ),
          const SizedBox(height: AppSpacing.sm),
          StatusPill(
            icon: signedIn ? Icons.verified_user_outlined : Icons.lock,
            label: signedIn
                ? strings.t('convert.signedInReady')
                : strings.t('convert.signInRequired'),
            color: signedIn ? tokens.info : tokens.disabledText,
          ),
          const SizedBox(height: AppSpacing.md),
          Text(
            strings.t('convert.packageHint'),
            style: Theme.of(context).textTheme.bodySmall,
          ),
          const SizedBox(height: AppSpacing.lg),
          Wrap(
            spacing: AppSpacing.sm,
            runSpacing: AppSpacing.sm,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              OutlinedButton.icon(
                onPressed: !signedIn || _state == _ConvertState.converting
                    ? null
                    : _pickFile,
                icon: const Icon(Icons.upload_file),
                label: Text(strings.t('common.upload')),
              ),
              StatusPill(
                icon: _zipFileName == null
                    ? Icons.inbox_outlined
                    : Icons.folder_zip,
                label: selectedFile,
                color: _zipFileName == null ? tokens.disabledText : tokens.info,
              ),
            ],
          ),
          const SizedBox(height: AppSpacing.md),
          AppTextField(
            controller: _mainTexController,
            label: strings.t('convert.mainTex'),
            hint: strings.t('convert.mainTexHint'),
          ),
          const SizedBox(height: AppSpacing.md),
          Wrap(
            spacing: AppSpacing.sm,
            runSpacing: AppSpacing.sm,
            children: [
              FilledButton.icon(
                onPressed:
                    !signedIn ||
                        _state == _ConvertState.converting ||
                        _zipBytes == null
                    ? null
                    : _startConvert,
                icon: _state == _ConvertState.converting
                    ? const SizedBox(
                        width: 16,
                        height: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.play_arrow),
                label: Text(strings.t('common.convert')),
              ),
              OutlinedButton.icon(
                onPressed: signedIn && _state != _ConvertState.converting
                    ? _loadRecords
                    : null,
                icon: const Icon(Icons.manage_search),
                label: Text(strings.t('convert.queryRecords')),
              ),
            ],
          ),
          const SizedBox(height: AppSpacing.md),
          AnimatedSwitcher(
            duration: AppMotion.normal,
            child: switch (_state) {
              _ConvertState.converting => LoadingState(
                key: const ValueKey('convert-loading'),
                label: strings.t('convert.converting'),
              ),
              _ConvertState.error => ErrorState(
                key: ValueKey(_errorText),
                message: _errorText ?? strings.t('common.error'),
              ),
              _ConvertState.success => _ResultCard(
                key: const ValueKey('convert-success'),
                status: _statusText ?? '',
                docxBytes: _docxBytes?.length ?? 0,
                elapsedMs: _elapsedMs ?? 0,
                onDownload: _downloadDocx,
              ),
              _ConvertState.idle => EmptyState(
                key: const ValueKey('convert-empty'),
                label: strings.t('empty.noData'),
              ),
            },
          ),
          const SizedBox(height: AppSpacing.md),
          _RecordPreview(
            title: strings.t('convert.logs'),
            emptyLabel: strings.t('empty.noData'),
            items: _logs,
          ),
          const SizedBox(height: AppSpacing.md),
          _RecordPreview(
            title: strings.t('convert.records'),
            emptyLabel: strings.t('empty.noData'),
            items: _records
                .map(
                  (job) =>
                      '${job.jobId} / ${job.status.name} / ${job.mainTex ?? '-'}',
                )
                .toList(),
          ),
        ],
      ),
    );
  }
}

class _ResultCard extends StatelessWidget {
  final String status;
  final int docxBytes;
  final int elapsedMs;
  final VoidCallback onDownload;

  const _ResultCard({
    super.key,
    required this.status,
    required this.docxBytes,
    required this.elapsedMs,
    required this.onDownload,
  });

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final tokens = Theme.of(context).extension<AppColorTokens>()!;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        StatusPill(
          icon: Icons.check_circle_outline,
          label: status,
          color: tokens.success,
        ),
        const SizedBox(height: AppSpacing.sm),
        Text(
          '${strings.t('convert.output')}: ${(docxBytes / 1024).toStringAsFixed(1)} KB, ${elapsedMs}ms',
          style: Theme.of(context).textTheme.bodySmall,
        ),
        const SizedBox(height: AppSpacing.sm),
        FilledButton.tonalIcon(
          onPressed: onDownload,
          icon: const Icon(Icons.download),
          label: Text(strings.t('common.download')),
        ),
      ],
    );
  }
}
