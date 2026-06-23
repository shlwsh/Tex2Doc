import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';

import 'bridge.dart';
import 'commercial_api.dart';
import 'file_web_stub.dart'
    if (dart.library.js_interop) 'file_web_utils_web.dart';
import 'logger.dart';
import 'ui/app_components.dart';
import 'ui/app_i18n.dart';
import 'ui/app_preferences.dart';
import 'ui/app_theme.dart';
import 'ui/app_tokens.dart';

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

class _WorkspaceShell extends StatelessWidget {
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
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final platform = isWeb ? 'Web' : 'Desktop';

    DocLogger.instance.i(LogTags.app, 'DocEngineApp build, platform=$platform');

    return Scaffold(
      body: SafeArea(
        child: LayoutBuilder(
          builder: (context, constraints) {
            final compact = constraints.maxWidth < AppBreakpoints.tablet;
            final content = _WorkspaceContent(
              isWeb: isWeb,
              themeTone: themeTone,
              locale: locale,
              onThemeChanged: onThemeChanged,
              onLocaleChanged: onLocaleChanged,
            );

            if (compact) {
              return Column(
                children: [
                  _TopBar(
                    platform: platform,
                    themeTone: themeTone,
                    locale: locale,
                    onThemeChanged: onThemeChanged,
                    onLocaleChanged: onLocaleChanged,
                  ),
                  Expanded(child: content),
                ],
              );
            }

            return Row(
              children: [
                _Sidebar(strings: strings),
                Expanded(
                  child: Column(
                    children: [
                      _TopBar(
                        platform: platform,
                        themeTone: themeTone,
                        locale: locale,
                        onThemeChanged: onThemeChanged,
                        onLocaleChanged: onLocaleChanged,
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

  const _Sidebar({required this.strings});

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
          Text(strings.t('app.title'), style: theme.textTheme.titleLarge),
          const SizedBox(height: AppSpacing.xs),
          Text(strings.t('app.subtitle'), style: theme.textTheme.bodySmall),
          const SizedBox(height: AppSpacing.xl),
          _NavItem(
            icon: Icons.dashboard_outlined,
            label: strings.t('nav.dashboard'),
            selected: true,
          ),
          _NavItem(icon: Icons.person_outline, label: strings.t('nav.account')),
          _NavItem(icon: Icons.sync_alt, label: strings.t('nav.convert')),
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

  const _NavItem({
    required this.icon,
    required this.label,
    this.selected = false,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return AnimatedContainer(
      duration: AppMotion.fast,
      curve: AppMotion.curve,
      margin: const EdgeInsets.only(bottom: AppSpacing.xs),
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
            color: selected ? theme.colorScheme.primary : theme.hintColor,
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
  final bool isWeb;
  final AppThemeTone themeTone;
  final AppLocale locale;
  final ValueChanged<AppThemeTone> onThemeChanged;
  final ValueChanged<AppLocale> onLocaleChanged;

  const _WorkspaceContent({
    required this.isWeb,
    required this.themeTone,
    required this.locale,
    required this.onThemeChanged,
    required this.onLocaleChanged,
  });

  @override
  Widget build(BuildContext context) {
    return PageContainer(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: const [
          _MetricsRow(),
          SizedBox(height: AppSpacing.lg),
          _ResponsiveCards(),
        ],
      ),
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
  const _ResponsiveCards();

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final stack = constraints.maxWidth < AppBreakpoints.tablet;
        final account = const _CommercialApiPanel();
        final convert = const _ConvertPanel();
        if (stack) {
          return Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              account,
              const SizedBox(height: AppSpacing.lg),
              convert,
            ],
          );
        }
        return Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Expanded(child: account),
            const SizedBox(width: AppSpacing.lg),
            Expanded(child: convert),
          ],
        );
      },
    );
  }
}

class _CommercialApiPanel extends StatefulWidget {
  const _CommercialApiPanel();

  @override
  State<_CommercialApiPanel> createState() => _CommercialApiPanelState();
}

class _CommercialApiPanelState extends State<_CommercialApiPanel> {
  final _baseUrlController = TextEditingController(
    text: 'http://127.0.0.1:8080/v1/',
  );
  final _emailController = TextEditingController(text: 'demo@example.com');
  final _passwordController = TextEditingController(text: 'secret');

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

enum _ConvertState { idle, converting, success, error }

class _ConvertPanel extends StatefulWidget {
  const _ConvertPanel();

  @override
  State<_ConvertPanel> createState() => _ConvertPanelState();
}

class _ConvertPanelState extends State<_ConvertPanel> {
  Uint8List? _zipBytes;
  String? _zipFileName;
  int? _zipSizeBytes;
  Uint8List? _docxBytes;
  int? _elapsedMs;
  _ConvertState _state = _ConvertState.idle;
  String? _statusText;
  String? _errorText;

  final _mainTexController = TextEditingController(text: 'main-jos.tex');

  @override
  void dispose() {
    _mainTexController.dispose();
    super.dispose();
  }

  Future<void> _pickFile() async {
    final strings = AppStrings.of(context);
    final result = await pickZipFile();
    if (result == null) return;

    final (bytes, fileName) = result;
    final sizeMB = bytes.length / (1024 * 1024);
    if (sizeMB >= 5) {
      setState(() {
        _state = _ConvertState.error;
        _errorText = strings.t('convert.fileTooLarge').fill({
          'size': sizeMB.toStringAsFixed(1),
        });
        _zipBytes = null;
        _zipFileName = null;
        _zipSizeBytes = null;
      });
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
  }

  Future<void> _startConvert() async {
    final bytes = _zipBytes;
    if (bytes == null) return;

    final strings = AppStrings.of(context);
    final mainTex = _mainTexController.text.trim().isEmpty
        ? 'main-jos.tex'
        : _mainTexController.text.trim();

    setState(() {
      _state = _ConvertState.converting;
      _statusText = strings.t('convert.converting');
      _errorText = null;
    });

    try {
      final t0 = DateTime.now();
      final docx = await DocEngineFacade.convertZipToDocx(bytes, mainTex);
      if (!mounted) return;

      _elapsedMs = DateTime.now().difference(t0).inMilliseconds;
      _docxBytes = docx;
      if (docx.length < 4 * 1024) {
        throw Exception('docx too small: ${docx.length} bytes');
      }
      if (docx[0] != 0x50 || docx[1] != 0x4B) {
        throw Exception('docx header is not ZIP');
      }

      setState(() {
        _state = _ConvertState.success;
        _statusText = strings.t('convert.success').fill({
          'size': (docx.length / 1024).toStringAsFixed(1),
          'elapsed': _elapsedMs ?? 0,
        });
      });
    } on Object catch (e) {
      if (!mounted) return;
      setState(() {
        _state = _ConvertState.error;
        _errorText = e.toString();
      });
    }
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
          Wrap(
            spacing: AppSpacing.sm,
            runSpacing: AppSpacing.sm,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              OutlinedButton.icon(
                onPressed: _state == _ConvertState.converting
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
          FilledButton.icon(
            onPressed: _state == _ConvertState.converting || _zipBytes == null
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
