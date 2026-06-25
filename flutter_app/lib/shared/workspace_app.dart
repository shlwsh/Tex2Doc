import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';

import '../admin/pages/audit/admin_audit_panel.dart';
import '../admin/pages/dashboard/admin_dashboard_panel.dart';
import '../admin/pages/feedback/admin_feedback_panel.dart';
import '../admin/pages/releases/admin_releases_panel.dart';
import '../admin/pages/redeem/admin_redeem_codes_panel.dart';
import '../commercial_api.dart';
import '../file_web_stub.dart'
    if (dart.library.js_interop) '../file_web_utils_web.dart'
    if (dart.library.io) '../file_web_utils_io.dart';
import '../logger.dart';
import '../ui/app_components.dart';
import '../ui/app_i18n.dart';
import '../ui/app_preferences.dart';
import '../ui/app_theme.dart';
import '../ui/app_tokens.dart';
import '../ui/auth_window.dart';
import '../ui/convert_records_panel.dart';
import '../ui/feedback_panel.dart';
import '../ui/recharge_records_panel.dart';

const _appIconAsset = 'assets/app_icon.png';
const _redeemCodePurchaseUrl = 'https://pay.ldxp.cn/item/ns8i2g';

// ─── App entry ────────────────────────────────────────────────────────────────

enum DocEngineAppMode { user, admin }

class DocEngineApp extends StatefulWidget {
  final bool isWeb;
  final DocEngineAppMode mode;

  const DocEngineApp({
    super.key,
    required this.isWeb,
    this.mode = DocEngineAppMode.user,
  });

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
        mode: widget.mode,
        themeTone: _themeTone,
        locale: _locale,
        onThemeChanged: _setTheme,
        onLocaleChanged: _setLocale,
      ),
    );
  }
}

// ─── Auth state ───────────────────────────────────────────────────────────────

class _AuthState {
  final String apiBaseUrl;
  final String accessToken;
  final UserProfile profile;

  const _AuthState({
    required this.apiBaseUrl,
    required this.accessToken,
    required this.profile,
  });
}

// ─── Workspace sections ────────────────────────────────────────────────────────

enum _NavSection {
  adminDashboard,
  account,
  recharge,
  redeemManage,
  redeemRecords,
  redeemCodes,
  convert,
  convertRecords,
  rechargeRecords,
  feedback,
  about,
  releases,
  audit,
}

extension _NavSectionMeta on _NavSection {
  IconData get icon => switch (this) {
    _NavSection.adminDashboard => Icons.dashboard_outlined,
    _NavSection.account => Icons.person_outline,
    _NavSection.recharge => Icons.payments_outlined,
    _NavSection.redeemManage => Icons.confirmation_number_outlined,
    _NavSection.redeemRecords => Icons.fact_check_outlined,
    _NavSection.redeemCodes => Icons.qr_code_2_outlined,
    _NavSection.convert => Icons.sync_alt,
    _NavSection.convertRecords => Icons.history,
    _NavSection.rechargeRecords => Icons.receipt_long,
    _NavSection.feedback => Icons.feedback_outlined,
    _NavSection.about => Icons.info_outline,
    _NavSection.releases => Icons.rocket_launch_outlined,
    _NavSection.audit => Icons.manage_search_outlined,
  };

  String label(AppStrings s) => switch (this) {
    _NavSection.adminDashboard => s.t('nav.adminDashboard'),
    _NavSection.account => s.t('nav.account'),
    _NavSection.recharge => s.t('nav.recharge'),
    _NavSection.redeemManage => s.t('nav.redeemManage'),
    _NavSection.redeemRecords => s.t('nav.redeemRecords'),
    _NavSection.redeemCodes => s.t('nav.redeemCodes'),
    _NavSection.convert => s.t('nav.convert'),
    _NavSection.convertRecords => s.t('nav.convertRecords'),
    _NavSection.rechargeRecords => s.t('nav.rechargeRecords'),
    _NavSection.feedback => s.t('nav.feedback'),
    _NavSection.about => s.t('nav.about'),
    _NavSection.releases => s.t('nav.releases'),
    _NavSection.audit => s.t('nav.audit'),
  };
}

// ─── Workspace shell ──────────────────────────────────────────────────────────

class _WorkspaceShell extends StatefulWidget {
  final bool isWeb;
  final DocEngineAppMode mode;
  final AppThemeTone themeTone;
  final AppLocale locale;
  final ValueChanged<AppThemeTone> onThemeChanged;
  final ValueChanged<AppLocale> onLocaleChanged;

  const _WorkspaceShell({
    required this.isWeb,
    required this.mode,
    required this.themeTone,
    required this.locale,
    required this.onThemeChanged,
    required this.onLocaleChanged,
  });

  @override
  State<_WorkspaceShell> createState() => _WorkspaceShellState();
}

class _WorkspaceShellState extends State<_WorkspaceShell> {
  _NavSection _selectedSection = _NavSection.account;
  _AuthState? _auth;
  String _apiBaseUrl = defaultCommercialApiBaseUrl;
  String? _authNotice;

  List<_NavSection> get _availableSections => switch (widget.mode) {
    DocEngineAppMode.user => const [
      _NavSection.account,
      _NavSection.recharge,
      _NavSection.convert,
      _NavSection.convertRecords,
      _NavSection.rechargeRecords,
      _NavSection.feedback,
      _NavSection.about,
    ],
    DocEngineAppMode.admin => const [
      _NavSection.adminDashboard,
      _NavSection.account,
      _NavSection.redeemManage,
      _NavSection.redeemRecords,
      _NavSection.redeemCodes,
      _NavSection.feedback,
      _NavSection.releases,
      _NavSection.audit,
      _NavSection.about,
    ],
  };

  void _handleSignedIn(
    String apiBaseUrl,
    String accessToken,
    UserProfile profile,
  ) {
    if (widget.mode == DocEngineAppMode.admin && !profile.isAdminRole) {
      setState(() {
        _apiBaseUrl = apiBaseUrl;
        _auth = null;
        _authNotice = 'Admin role required.';
      });
      return;
    }
    setState(() {
      _apiBaseUrl = apiBaseUrl;
      _auth = _AuthState(
        apiBaseUrl: apiBaseUrl,
        accessToken: accessToken,
        profile: profile,
      );
      _authNotice = null;
      if (!_availableSections.contains(_selectedSection)) {
        _selectedSection = _availableSections.first;
      }
    });
  }

  void _handleSignedOut() {
    setState(() => _auth = null);
    DocLogger.instance.i(LogTags.app, 'User signed out');
  }

  void _selectSection(_NavSection section) {
    setState(() => _selectedSection = section);
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final platform = widget.mode == DocEngineAppMode.admin
        ? 'Web Admin'
        : widget.isWeb
        ? 'Web User'
        : 'Desktop';

    DocLogger.instance.i(LogTags.app, 'DocEngineApp build, platform=$platform');

    // Gate: show auth window when not signed in
    if (_auth == null) {
      return Stack(
        children: [
          AuthWindow(apiBaseUrl: _apiBaseUrl, onSignedIn: _handleSignedIn),
          if (_authNotice != null)
            Positioned(
              left: AppSpacing.lg,
              right: AppSpacing.lg,
              top: AppSpacing.lg,
              child: Material(
                elevation: 4,
                borderRadius: BorderRadius.circular(AppRadius.md),
                color: Theme.of(context).colorScheme.errorContainer,
                child: Padding(
                  padding: const EdgeInsets.all(AppSpacing.md),
                  child: Text(
                    _authNotice!,
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.onErrorContainer,
                    ),
                  ),
                ),
              ),
            ),
        ],
      );
    }

    return Scaffold(
      body: SafeArea(
        child: LayoutBuilder(
          builder: (context, constraints) {
            final compact = constraints.maxWidth < AppBreakpoints.tablet;
            final content = _NavContent(
              mode: widget.mode,
              selectedSection: _selectedSection,
              sections: _availableSections,
              compact: compact,
              apiBaseUrl: _auth!.apiBaseUrl,
              accessToken: _auth!.accessToken,
              profile: _auth!.profile,
              onSectionChanged: _selectSection,
              onSignedOut: _handleSignedOut,
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
                    profile: _auth!.profile,
                    onSignedOut: _handleSignedOut,
                    strings: strings,
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
                  sections: _availableSections,
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
                        profile: _auth!.profile,
                        onSignedOut: _handleSignedOut,
                        strings: strings,
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

// ─── Sidebar ──────────────────────────────────────────────────────────────────

class _Sidebar extends StatelessWidget {
  final AppStrings strings;
  final _NavSection selectedSection;
  final List<_NavSection> sections;
  final ValueChanged<_NavSection> onSectionChanged;

  const _Sidebar({
    required this.strings,
    required this.selectedSection,
    required this.sections,
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
          for (final section in sections)
            _NavItem(
              icon: section.icon,
              label: section.label(strings),
              selected: selectedSection == section,
              onTap: () => onSectionChanged(section),
            ),
          const Spacer(),
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

// ─── Top bar ──────────────────────────────────────────────────────────────────

class _TopBar extends StatelessWidget {
  final String platform;
  final AppThemeTone themeTone;
  final AppLocale locale;
  final ValueChanged<AppThemeTone> onThemeChanged;
  final ValueChanged<AppLocale> onLocaleChanged;
  final UserProfile profile;
  final VoidCallback onSignedOut;
  final AppStrings strings;

  const _TopBar({
    required this.platform,
    required this.themeTone,
    required this.locale,
    required this.onThemeChanged,
    required this.onLocaleChanged,
    required this.profile,
    required this.onSignedOut,
    required this.strings,
  });

  @override
  Widget build(BuildContext context) {
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
      child: Row(
        children: [
          ClipRRect(
            borderRadius: BorderRadius.circular(AppRadius.sm),
            child: Image.asset(
              _appIconAsset,
              width: 36,
              height: 36,
              fit: BoxFit.cover,
            ),
          ),
          const SizedBox(width: AppSpacing.md),
          Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(strings.t('app.title'), style: theme.textTheme.titleMedium),
              Text(
                '${strings.t('topbar.platform')}: $platform',
                style: theme.textTheme.bodySmall,
              ),
            ],
          ),
          const Spacer(),
          _ThemeDropdown(value: themeTone, onChanged: onThemeChanged),
          const SizedBox(width: AppSpacing.sm),
          _LocaleDropdown(value: locale, onChanged: onLocaleChanged),
          const SizedBox(width: AppSpacing.md),
          _AccountAvatarButton(
            profile: profile,
            strings: strings,
            onSignedOut: onSignedOut,
          ),
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

// ─── Account avatar button ─────────────────────────────────────────────────────

class _AccountAvatarButton extends StatelessWidget {
  final UserProfile profile;
  final AppStrings strings;
  final VoidCallback onSignedOut;

  const _AccountAvatarButton({
    required this.profile,
    required this.strings,
    required this.onSignedOut,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final initial = profile.email.isNotEmpty
        ? profile.email[0].toUpperCase()
        : '?';

    return PopupMenuButton<String>(
      offset: const Offset(0, 40),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(AppRadius.md),
      ),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: AppSpacing.sm),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            CircleAvatar(
              radius: 16,
              backgroundColor: theme.colorScheme.primary,
              child: Text(
                initial,
                style: const TextStyle(color: Colors.white, fontSize: 14),
              ),
            ),
            const SizedBox(width: AppSpacing.xs),
            Icon(Icons.arrow_drop_down, color: theme.hintColor, size: 20),
          ],
        ),
      ),
      itemBuilder: (context) => [
        PopupMenuItem<String>(
          enabled: false,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(profile.email, style: theme.textTheme.titleSmall),
              Text(
                '${strings.t('account.currentPlan')}: ${profile.planId}',
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.hintColor,
                ),
              ),
            ],
          ),
        ),
        const PopupMenuDivider(),
        PopupMenuItem<String>(
          value: 'profile',
          child: Row(
            children: [
              const Icon(Icons.person_outline, size: 18),
              const SizedBox(width: AppSpacing.sm),
              Text(strings.t('account.viewProfile')),
            ],
          ),
        ),
        PopupMenuItem<String>(
          value: 'password',
          child: Row(
            children: [
              const Icon(Icons.lock_outline, size: 18),
              const SizedBox(width: AppSpacing.sm),
              Text(strings.t('account.changePassword')),
            ],
          ),
        ),
        const PopupMenuDivider(),
        PopupMenuItem<String>(
          value: 'logout',
          child: Row(
            children: [
              Icon(Icons.logout, size: 18, color: theme.colorScheme.error),
              const SizedBox(width: AppSpacing.sm),
              Text(
                strings.t('account.logout'),
                style: TextStyle(color: theme.colorScheme.error),
              ),
            ],
          ),
        ),
      ],
      onSelected: (value) {
        switch (value) {
          case 'logout':
            onSignedOut();
            break;
          case 'password':
            _showChangePasswordDialog(context);
            break;
          case 'profile':
            _showProfileDialog(context);
            break;
        }
      },
    );
  }

  void _showChangePasswordDialog(BuildContext context) {
    showDialog(
      context: context,
      builder: (ctx) => _ChangePasswordDialog(strings: strings),
    );
  }

  void _showProfileDialog(BuildContext context) {
    showDialog(
      context: context,
      builder: (ctx) => _ProfileDialog(profile: profile, strings: strings),
    );
  }
}

// ─── Profile dialog ────────────────────────────────────────────────────────────

class _ProfileDialog extends StatelessWidget {
  final UserProfile profile;
  final AppStrings strings;

  const _ProfileDialog({required this.profile, required this.strings});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final initial = profile.email.isNotEmpty
        ? profile.email[0].toUpperCase()
        : '?';

    return AlertDialog(
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(AppRadius.lg),
      ),
      title: Text(strings.t('account.profileTitle')),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          CircleAvatar(
            radius: 40,
            backgroundColor: theme.colorScheme.primary,
            child: Text(
              initial,
              style: const TextStyle(color: Colors.white, fontSize: 32),
            ),
          ),
          const SizedBox(height: AppSpacing.lg),
          _InfoRow(label: strings.t('account.email'), value: profile.email),
          _InfoRow(
            label: strings.t('account.currentPlan'),
            value: profile.planId,
          ),
          if (profile.displayName != null && profile.displayName!.isNotEmpty)
            _InfoRow(
              label: strings.t('account.displayName'),
              value: profile.displayName!,
            ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(strings.t('common.confirm')),
        ),
      ],
    );
  }
}

class _InfoRow extends StatelessWidget {
  final String label;
  final String value;

  const _InfoRow({required this.label, required this.value});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: AppSpacing.sm),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(label, style: theme.textTheme.bodySmall),
          Flexible(
            child: Text(
              value,
              style: theme.textTheme.bodyMedium,
              textAlign: TextAlign.end,
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }
}

// ─── Change password dialog ───────────────────────────────────────────────────

class _ChangePasswordDialog extends StatefulWidget {
  final AppStrings strings;

  const _ChangePasswordDialog({required this.strings});

  @override
  State<_ChangePasswordDialog> createState() => _ChangePasswordDialogState();
}

class _ChangePasswordDialogState extends State<_ChangePasswordDialog> {
  final _oldController = TextEditingController();
  final _newController = TextEditingController();
  final _confirmController = TextEditingController();
  bool _busy = false;
  String? _error;
  bool _success = false;

  @override
  void dispose() {
    _oldController.dispose();
    _newController.dispose();
    _confirmController.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final strings = widget.strings;
    if (_oldController.text.isEmpty ||
        _newController.text.isEmpty ||
        _confirmController.text.isEmpty) {
      setState(() => _error = 'Please fill in all fields.');
      return;
    }
    if (_newController.text != _confirmController.text) {
      setState(() => _error = strings.t('account.passwordMismatch'));
      return;
    }
    if (_newController.text.length < 6) {
      setState(() => _error = strings.t('account.passwordTooShort'));
      return;
    }

    setState(() {
      _busy = true;
      _error = null;
    });

    // In a real app this would call an API endpoint.
    // For now, simulate success after a brief delay.
    await Future<void>.delayed(const Duration(milliseconds: 800));
    if (!mounted) return;
    setState(() {
      _busy = false;
      _success = true;
    });
    await Future<void>.delayed(const Duration(seconds: 1));
    if (!mounted) return;
    Navigator.of(context).pop();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final strings = widget.strings;

    return AlertDialog(
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(AppRadius.lg),
      ),
      title: Text(strings.t('account.changePasswordTitle')),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _ObscuredTextField(
            controller: _oldController,
            label: strings.t('account.oldPassword'),
          ),
          const SizedBox(height: AppSpacing.md),
          _ObscuredTextField(
            controller: _newController,
            label: strings.t('account.newPassword'),
          ),
          const SizedBox(height: AppSpacing.md),
          _ObscuredTextField(
            controller: _confirmController,
            label: strings.t('account.confirmNewPassword'),
          ),
          if (_error != null) ...[
            const SizedBox(height: AppSpacing.md),
            Text(
              _error!,
              style: TextStyle(color: theme.colorScheme.error, fontSize: 12),
            ),
          ],
          if (_success) ...[
            const SizedBox(height: AppSpacing.md),
            Text(
              strings.t('account.passwordChanged'),
              style: TextStyle(color: Colors.green, fontSize: 12),
            ),
          ],
        ],
      ),
      actions: [
        TextButton(
          onPressed: _busy ? null : () => Navigator.of(context).pop(),
          child: Text(strings.t('common.cancel')),
        ),
        FilledButton(
          onPressed: _busy ? null : _submit,
          child: _busy
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : Text(strings.t('common.confirm')),
        ),
      ],
    );
  }
}

class _ObscuredTextField extends StatefulWidget {
  final TextEditingController controller;
  final String label;

  const _ObscuredTextField({required this.controller, required this.label});

  @override
  State<_ObscuredTextField> createState() => _ObscuredTextFieldState();
}

class _ObscuredTextFieldState extends State<_ObscuredTextField> {
  bool _obscured = true;

  @override
  Widget build(BuildContext context) {
    return TextField(
      controller: widget.controller,
      obscureText: _obscured,
      decoration: InputDecoration(
        labelText: widget.label,
        suffixIcon: IconButton(
          icon: Icon(
            _obscured ? Icons.visibility_off : Icons.visibility,
            size: 20,
          ),
          onPressed: () => setState(() => _obscured = !_obscured),
        ),
      ),
    );
  }
}

// ─── Nav content ───────────────────────────────────────────────────────────────

class _NavContent extends StatelessWidget {
  final DocEngineAppMode mode;
  final _NavSection selectedSection;
  final List<_NavSection> sections;
  final bool compact;
  final String apiBaseUrl;
  final String accessToken;
  final UserProfile profile;
  final ValueChanged<_NavSection> onSectionChanged;
  final VoidCallback onSignedOut;

  const _NavContent({
    required this.mode,
    required this.selectedSection,
    required this.sections,
    required this.compact,
    required this.apiBaseUrl,
    required this.accessToken,
    required this.profile,
    required this.onSectionChanged,
    required this.onSignedOut,
  });

  @override
  Widget build(BuildContext context) {
    final body = switch (selectedSection) {
      _NavSection.adminDashboard => <Widget>[
        AdminDashboardPanel(apiBaseUrl: apiBaseUrl, accessToken: accessToken),
      ],
      _NavSection.account => <Widget>[
        _AccountPanel(
          apiBaseUrl: apiBaseUrl,
          accessToken: accessToken,
          profile: profile,
          onSignedOut: onSignedOut,
        ),
      ],
      _NavSection.recharge => <Widget>[
        _RechargePanel(apiBaseUrl: apiBaseUrl, accessToken: accessToken),
      ],
      _NavSection.redeemManage => <Widget>[
        AdminRedeemManagePanel(apiBaseUrl: apiBaseUrl, adminToken: accessToken),
      ],
      _NavSection.redeemRecords => <Widget>[
        AdminRedeemRecordsPanel(
          apiBaseUrl: apiBaseUrl,
          adminToken: accessToken,
        ),
      ],
      _NavSection.redeemCodes => <Widget>[
        AdminRedeemCodesPanel(
          apiBaseUrl: apiBaseUrl,
          adminToken: accessToken,
        ),
      ],
      _NavSection.convert => <Widget>[
        _ConvertPanel(apiBaseUrl: apiBaseUrl, accessToken: accessToken),
      ],
      _NavSection.convertRecords => <Widget>[
        ConvertRecordsPanel(apiBaseUrl: apiBaseUrl, accessToken: accessToken),
      ],
      _NavSection.rechargeRecords => <Widget>[
        RechargeRecordsPanel(apiBaseUrl: apiBaseUrl, accessToken: accessToken),
      ],
      _NavSection.feedback => <Widget>[
        if (mode == DocEngineAppMode.admin)
          AdminFeedbackPanel(apiBaseUrl: apiBaseUrl, accessToken: accessToken)
        else
          FeedbackPanel(apiBaseUrl: apiBaseUrl, accessToken: accessToken),
      ],
      _NavSection.about => const <Widget>[_AboutPanel()],
      _NavSection.releases => <Widget>[
        AdminReleasesPanel(apiBaseUrl: apiBaseUrl, accessToken: accessToken),
      ],
      _NavSection.audit => <Widget>[
        AdminAuditPanel(apiBaseUrl: apiBaseUrl, accessToken: accessToken),
      ],
    };

    return PageContainer(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (compact) ...[
            _CompactSectionTabs(
              selectedSection: selectedSection,
              sections: sections,
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

class _CompactSectionTabs extends StatelessWidget {
  final _NavSection selectedSection;
  final List<_NavSection> sections;
  final ValueChanged<_NavSection> onSectionChanged;

  const _CompactSectionTabs({
    required this.selectedSection,
    required this.sections,
    required this.onSectionChanged,
  });

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    return SegmentedButton<_NavSection>(
      showSelectedIcon: false,
      segments: [
        for (final section in sections)
          ButtonSegment<_NavSection>(
            value: section,
            icon: Icon(section.icon, size: 16),
            label: Text(
              section.label(strings),
              style: const TextStyle(fontSize: 11),
            ),
          ),
      ],
      selected: {selectedSection},
      onSelectionChanged: (sections) {
        if (sections.isNotEmpty) onSectionChanged(sections.first);
      },
    );
  }
}

class _AboutPanel extends StatelessWidget {
  const _AboutPanel();

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final theme = Theme.of(context);
    final features = [
      strings.t('about.feature.convert'),
      strings.t('about.feature.quality'),
      strings.t('about.feature.records'),
      strings.t('about.feature.commercial'),
    ];

    return AppCard(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              ClipRRect(
                borderRadius: BorderRadius.circular(AppRadius.md),
                child: Image.asset(
                  _appIconAsset,
                  width: 72,
                  height: 72,
                  fit: BoxFit.cover,
                ),
              ),
              const SizedBox(width: AppSpacing.lg),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('Tex2Doc', style: theme.textTheme.headlineSmall),
                    const SizedBox(height: AppSpacing.xs),
                    Text(
                      strings.t('about.company'),
                      style: theme.textTheme.titleMedium?.copyWith(
                        color: theme.colorScheme.primary,
                      ),
                    ),
                    const SizedBox(height: AppSpacing.sm),
                    Text(
                      strings.t('about.description'),
                      style: theme.textTheme.bodyMedium,
                    ),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: AppSpacing.lg),
          Text(
            strings.t('about.goalTitle'),
            style: theme.textTheme.titleMedium,
          ),
          const SizedBox(height: AppSpacing.xs),
          Text(strings.t('about.goal'), style: theme.textTheme.bodyMedium),
          const SizedBox(height: AppSpacing.lg),
          Text(
            strings.t('about.featuresTitle'),
            style: theme.textTheme.titleMedium,
          ),
          const SizedBox(height: AppSpacing.sm),
          for (final feature in features)
            Padding(
              padding: const EdgeInsets.only(bottom: AppSpacing.xs),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Icon(
                    Icons.check_circle_outline,
                    size: 18,
                    color: theme.colorScheme.primary,
                  ),
                  const SizedBox(width: AppSpacing.sm),
                  Expanded(child: Text(feature)),
                ],
              ),
            ),
        ],
      ),
    );
  }
}

// ─── Account panel ─────────────────────────────────────────────────────────────

class _AccountPanel extends StatefulWidget {
  final String apiBaseUrl;
  final String accessToken;
  final UserProfile profile;
  final VoidCallback onSignedOut;

  const _AccountPanel({
    required this.apiBaseUrl,
    required this.accessToken,
    required this.profile,
    required this.onSignedOut,
  });

  @override
  State<_AccountPanel> createState() => _AccountPanelState();
}

class _AccountPanelState extends State<_AccountPanel> {
  UserProfile? _profile;
  UsageSummary? _usage;

  @override
  void initState() {
    super.initState();
    _profile = widget.profile;
    unawaited(_loadUsage());
  }

  Future<void> _loadUsage() async {
    try {
      final client = CommercialApiClient(widget.apiBaseUrl);
      final usage = await client.usage(widget.accessToken);
      if (!mounted) return;
      setState(() => _usage = usage);
    } on Object catch (_) {
      // ignore network errors
    }
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        AppSectionHeader(
          title: strings.t('nav.account'),
          description: strings.t('account.overviewDescription'),
        ),
        const SizedBox(height: AppSpacing.lg),
        LayoutBuilder(
          builder: (context, constraints) {
            final wide = constraints.maxWidth > AppBreakpoints.mobile;
            if (wide) {
              return Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(child: _MetricsRow(usage: _usage)),
                  const SizedBox(width: AppSpacing.lg),
                  Expanded(
                    child: _AccountCard(profile: _profile ?? widget.profile),
                  ),
                ],
              );
            }
            return Column(
              children: [
                _AccountCard(profile: _profile ?? widget.profile),
                const SizedBox(height: AppSpacing.md),
                _MetricsRow(usage: _usage),
              ],
            );
          },
        ),
      ],
    );
  }
}

class _AccountCard extends StatelessWidget {
  final UserProfile profile;

  const _AccountCard({required this.profile});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final strings = AppStrings.of(context);
    final initial = profile.email.isNotEmpty
        ? profile.email[0].toUpperCase()
        : '?';

    return AppCard(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          CircleAvatar(
            radius: 36,
            backgroundColor: theme.colorScheme.primary,
            child: Text(
              initial,
              style: const TextStyle(color: Colors.white, fontSize: 28),
            ),
          ),
          const SizedBox(height: AppSpacing.md),
          Text(
            profile.email,
            style: theme.textTheme.titleMedium,
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: AppSpacing.xs),
          StatusPill(
            icon: Icons.verified_user_outlined,
            label: profile.planId,
            color: theme.colorScheme.primary,
          ),
          const SizedBox(height: AppSpacing.md),
          _InfoRow(
            label: strings.t('account.currentPlan'),
            value: profile.planId,
          ),
          if (profile.displayName != null && profile.displayName!.isNotEmpty)
            _InfoRow(
              label: strings.t('account.displayName'),
              value: profile.displayName!,
            ),
        ],
      ),
    );
  }
}

class _MetricsRow extends StatelessWidget {
  final UsageSummary? usage;

  const _MetricsRow({this.usage});

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
              label: strings.t('metrics.quota'),
              value: usage != null
                  ? '${usage!.cloudConversionsUsed}/${usage!.cloudConversionsLimit}'
                  : '- / -',
              icon: Icons.speed,
            ),
            MetricTile(
              label: strings.t('metrics.countBalance'),
              value: usage != null ? '${usage!.countBalance}' : '-',
              icon: Icons.inventory_2_outlined,
            ),
            MetricTile(
              label: strings.t('metrics.dateValidUntil'),
              value: usage?.dateValidUntil ?? '-',
              icon: Icons.calendar_today_outlined,
            ),
          ],
        );
      },
    );
  }
}

// ─── Recharge panel (existing, simplified) ───────────────────────────────────

class _RechargePanel extends StatefulWidget {
  final String apiBaseUrl;
  final String accessToken;

  const _RechargePanel({required this.apiBaseUrl, required this.accessToken});

  @override
  State<_RechargePanel> createState() => _RechargePanelState();
}

class _RechargePanelState extends State<_RechargePanel> {
  final TextEditingController _codeController = TextEditingController();
  RedeemCodeOptions? _redeemOptions;
  List<RedeemCodeRecord> _redeemRecords = const [];
  String? _status;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    unawaited(_loadRecords());
  }

  @override
  void didUpdateWidget(covariant _RechargePanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.accessToken != widget.accessToken ||
        oldWidget.apiBaseUrl != widget.apiBaseUrl) {
      unawaited(_loadRecords());
    }
  }

  @override
  void dispose() {
    _codeController.dispose();
    super.dispose();
  }

  Future<void> _loadRecords() async {
    try {
      final client = CommercialApiClient(widget.apiBaseUrl);
      final options = await client.redeemCodeOptions(widget.accessToken);
      final records = await client.redeemCodeRecords(widget.accessToken);
      if (!mounted) return;
      setState(() {
        _redeemOptions = options;
        _redeemRecords = records;
      });
    } on Object catch (e) {
      if (!mounted) return;
      setState(() => _status = e.toString());
    }
  }

  Future<void> _redeem() async {
    final strings = AppStrings.of(context);
    final code = _codeController.text.trim();
    if (code.isEmpty) {
      setState(() => _status = strings.t('recharge.codeRequired'));
      return;
    }
    if (_busy) return;
    setState(() {
      _busy = true;
      _status = strings.t('status.working');
    });
    try {
      final client = CommercialApiClient(widget.apiBaseUrl);
      final result = await client.redeemCode(
        accessToken: widget.accessToken,
        code: code,
      );
      final records = await client.redeemCodeRecords(widget.accessToken);
      if (!mounted) return;
      setState(() {
        _codeController.clear();
        _redeemRecords = records;
        _status = strings.t('recharge.redeemed').fill({
          'package': result.packageName,
          'quantity': result.quantity,
          'balance': result.countBalance,
        });
      });
    } on Object catch (e) {
      if (!mounted) return;
      setState(() => _status = e.toString());
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final theme = Theme.of(context);
    final options = _redeemOptions;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        AppSectionHeader(
          title: strings.t('nav.recharge'),
          description: strings.t('recharge.description'),
        ),
        const SizedBox(height: AppSpacing.md),
        AppCard(
          padding: const EdgeInsets.all(AppSpacing.md),
          child: LayoutBuilder(
            builder: (context, constraints) {
              final compactPurchaseCard = constraints.maxWidth < 520;
              final copy = Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Icon(
                    Icons.shopping_bag_outlined,
                    color: theme.colorScheme.primary,
                  ),
                  const SizedBox(width: AppSpacing.md),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          strings.t('recharge.purchaseTitle'),
                          style: theme.textTheme.titleSmall,
                        ),
                        const SizedBox(height: AppSpacing.xs),
                        Text(
                          strings.t('recharge.purchaseNote'),
                          style: theme.textTheme.bodySmall,
                        ),
                      ],
                    ),
                  ),
                ],
              );
              final action = FilledButton.icon(
                onPressed: () => openExternalUrl(_redeemCodePurchaseUrl),
                icon: const Icon(Icons.open_in_new),
                label: Text(strings.t('recharge.purchaseButton')),
              );

              if (compactPurchaseCard) {
                return Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    copy,
                    const SizedBox(height: AppSpacing.md),
                    action,
                  ],
                );
              }

              return Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(child: copy),
                  const SizedBox(width: AppSpacing.md),
                  action,
                ],
              );
            },
          ),
        ),
        const SizedBox(height: AppSpacing.md),
        Row(
          children: [
            OutlinedButton.icon(
              onPressed: _busy ? null : _loadRecords,
              icon: const Icon(Icons.history, size: 18),
              label: Text(strings.t('recharge.queryRecords')),
            ),
            const SizedBox(width: AppSpacing.sm),
            StatusPill(
              icon: Icons.confirmation_number_outlined,
              label: strings.t('recharge.redeemProvider'),
              color: theme.colorScheme.primary,
            ),
          ],
        ),
        const SizedBox(height: AppSpacing.lg),
        TextField(
          controller: _codeController,
          enabled: !_busy,
          textCapitalization: TextCapitalization.characters,
          decoration: InputDecoration(
            labelText: strings.t('recharge.codeLabel'),
            hintText: options?.codeFormatHint ?? strings.t('recharge.codeHint'),
            prefixIcon: const Icon(Icons.confirmation_number_outlined),
            suffixIcon: IconButton(
              tooltip: strings.t('recharge.submitCode'),
              onPressed: _busy ? null : _redeem,
              icon: const Icon(Icons.check_circle_outline),
            ),
          ),
          onSubmitted: (_) => _busy ? null : _redeem(),
        ),
        const SizedBox(height: AppSpacing.sm),
        FilledButton.icon(
          onPressed: _busy ? null : _redeem,
          icon: const Icon(Icons.redeem),
          label: Text(strings.t('recharge.submitCode')),
        ),
        const SizedBox(height: AppSpacing.lg),
        Text(
          strings.t('recharge.packageTitle'),
          style: theme.textTheme.titleSmall,
        ),
        const SizedBox(height: AppSpacing.sm),
        _KeyValueList(
          entries: (options?.packages ?? const [])
              .map((package) => package.label)
              .toList(growable: false),
        ),
        const SizedBox(height: AppSpacing.md),
        Text(
          strings.t('recharge.redeemRecords'),
          style: theme.textTheme.titleSmall,
        ),
        const SizedBox(height: AppSpacing.sm),
        _KeyValueList(
          entries: _redeemRecords.isEmpty
              ? [strings.t('empty.noData')]
              : _redeemRecords
                    .map((record) => record.label)
                    .toList(growable: false),
        ),
        if (_status != null) ...[
          const SizedBox(height: AppSpacing.md),
          Text(_status!, style: theme.textTheme.bodySmall),
        ],
      ],
    );
  }
}

// ─── Redeem code management panel ────────────────────────────────────────────

class AdminRedeemManagePanel extends StatefulWidget {
  final String apiBaseUrl;
  final String? adminToken;

  const AdminRedeemManagePanel({
    super.key,
    required this.apiBaseUrl,
    this.adminToken,
  });

  @override
  State<AdminRedeemManagePanel> createState() => _RedeemManagePanelState();
}

class _RedeemManagePanelState extends State<AdminRedeemManagePanel> {
  final TextEditingController _adminTokenController = TextEditingController(
    text: 'demo-admin',
  );
  final TextEditingController _quantityController = TextEditingController(
    text: '10',
  );
  final TextEditingController _channelController = TextEditingController(
    text: 'web',
  );
  final TextEditingController _noteController = TextEditingController();
  final TextEditingController _expiresAtController = TextEditingController();

  String _packageId = 'count_10';
  RedeemCodeBatch? _batch;
  String? _status;
  bool _busy = false;

  String get _adminToken => widget.adminToken?.trim().isNotEmpty == true
      ? widget.adminToken!.trim()
      : _adminTokenController.text.trim();

  @override
  void dispose() {
    _adminTokenController.dispose();
    _quantityController.dispose();
    _channelController.dispose();
    _noteController.dispose();
    _expiresAtController.dispose();
    super.dispose();
  }

  Future<void> _generateBatch() async {
    final strings = AppStrings.of(context);
    final quantity = int.tryParse(_quantityController.text.trim());
    if (quantity == null || quantity <= 0) {
      setState(() => _status = strings.t('redeemManage.quantityInvalid'));
      return;
    }
    if (_busy) return;
    setState(() {
      _busy = true;
      _status = strings.t('status.working');
    });
    try {
      final client = CommercialApiClient(widget.apiBaseUrl);
      final batch = await client.createRedeemCodeBatch(
        adminToken: _adminToken,
        packageId: _packageId,
        quantity: quantity,
        channel: _channelController.text,
        note: _noteController.text,
        expiresAt: _expiresAtController.text,
      );
      if (!mounted) return;
      setState(() {
        _batch = batch;
        _status = strings.t('redeemManage.generated').fill({
          'batch': batch.batchNo,
          'count': batch.generatedCount,
        });
      });
    } on Object catch (e) {
      if (!mounted) return;
      setState(() => _status = e.toString());
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _downloadExcel() async {
    final batch = _batch;
    if (batch == null || _busy) return;
    setState(() {
      _busy = true;
      _status = AppStrings.of(context).t('status.working');
    });
    try {
      final client = CommercialApiClient(widget.apiBaseUrl);
      final bytes = await client.exportRedeemCodeBatch(
        adminToken: _adminToken,
        batchId: batch.batchId,
      );
      downloadBlob(
        Uint8List.fromList(bytes),
        'redeem-codes-${batch.batchNo}.xlsx',
      );
      if (!mounted) return;
      setState(
        () => _status = AppStrings.of(context).t('redeemManage.exported'),
      );
    } on Object catch (e) {
      if (!mounted) return;
      setState(() => _status = e.toString());
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final theme = Theme.of(context);
    final batch = _batch;
    final previewCodes =
        batch?.codes.take(8).toList(growable: false) ?? const [];

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        AppSectionHeader(
          title: strings.t('nav.redeemManage'),
          description: strings.t('redeemManage.description'),
        ),
        const SizedBox(height: AppSpacing.lg),
        DropdownButtonFormField<String>(
          initialValue: _packageId,
          decoration: InputDecoration(
            labelText: strings.t('redeemManage.package'),
            prefixIcon: const Icon(Icons.inventory_2_outlined),
          ),
          items: const [
            DropdownMenuItem(value: 'count_3', child: Text('3 次转换包')),
            DropdownMenuItem(value: 'count_10', child: Text('10 次转换包')),
            DropdownMenuItem(value: 'count_30', child: Text('30 次转换包')),
          ],
          onChanged: _busy
              ? null
              : (value) {
                  if (value != null) setState(() => _packageId = value);
                },
        ),
        const SizedBox(height: AppSpacing.md),
        Wrap(
          spacing: AppSpacing.md,
          runSpacing: AppSpacing.md,
          children: [
            SizedBox(
              width: 220,
              child: TextField(
                controller: _quantityController,
                enabled: !_busy,
                keyboardType: TextInputType.number,
                decoration: InputDecoration(
                  labelText: strings.t('redeemManage.quantity'),
                  prefixIcon: const Icon(Icons.tag),
                ),
              ),
            ),
            SizedBox(
              width: 220,
              child: TextField(
                controller: _channelController,
                enabled: !_busy,
                decoration: InputDecoration(
                  labelText: strings.t('redeemManage.channel'),
                  prefixIcon: const Icon(Icons.hub_outlined),
                ),
              ),
            ),
            SizedBox(
              width: 260,
              child: TextField(
                controller: _expiresAtController,
                enabled: !_busy,
                decoration: InputDecoration(
                  labelText: strings.t('redeemManage.expiresAt'),
                  prefixIcon: const Icon(Icons.event_outlined),
                ),
              ),
            ),
          ],
        ),
        const SizedBox(height: AppSpacing.md),
        if (widget.adminToken == null) ...[
          TextField(
            controller: _adminTokenController,
            enabled: !_busy,
            obscureText: true,
            decoration: InputDecoration(
              labelText: strings.t('redeemManage.adminToken'),
              prefixIcon: const Icon(Icons.admin_panel_settings_outlined),
            ),
          ),
          const SizedBox(height: AppSpacing.md),
        ],
        TextField(
          controller: _noteController,
          enabled: !_busy,
          minLines: 2,
          maxLines: 3,
          decoration: InputDecoration(
            labelText: strings.t('redeemManage.note'),
            prefixIcon: const Icon(Icons.notes_outlined),
          ),
        ),
        const SizedBox(height: AppSpacing.lg),
        Wrap(
          spacing: AppSpacing.sm,
          runSpacing: AppSpacing.sm,
          children: [
            FilledButton.icon(
              onPressed: _busy ? null : _generateBatch,
              icon: const Icon(Icons.add_card_outlined),
              label: Text(strings.t('redeemManage.generate')),
            ),
            FilledButton.tonalIcon(
              onPressed: _busy || batch == null ? null : _downloadExcel,
              icon: const Icon(Icons.download),
              label: Text(strings.t('redeemManage.downloadExcel')),
            ),
          ],
        ),
        if (_status != null) ...[
          const SizedBox(height: AppSpacing.md),
          Text(_status!, style: theme.textTheme.bodySmall),
        ],
        const SizedBox(height: AppSpacing.lg),
        if (batch != null) ...[
          Text(batch.label, style: theme.textTheme.titleSmall),
          const SizedBox(height: AppSpacing.sm),
          _KeyValueList(
            entries: [
              '${strings.t('redeemManage.batchId')}: ${batch.batchId}',
              '${strings.t('redeemManage.status')}: ${batch.status}',
              '${strings.t('redeemManage.createdAt')}: ${batch.createdAt}',
            ],
          ),
          const SizedBox(height: AppSpacing.md),
          Text(
            strings.t('redeemManage.previewCodes'),
            style: theme.textTheme.titleSmall,
          ),
          const SizedBox(height: AppSpacing.sm),
          _KeyValueList(
            entries: previewCodes.isEmpty
                ? [strings.t('empty.noData')]
                : previewCodes,
          ),
        ],
      ],
    );
  }
}

// ─── Redeem code records panel ───────────────────────────────────────────────

class AdminRedeemRecordsPanel extends StatefulWidget {
  final String apiBaseUrl;
  final String? adminToken;

  const AdminRedeemRecordsPanel({
    super.key,
    required this.apiBaseUrl,
    this.adminToken,
  });

  @override
  State<AdminRedeemRecordsPanel> createState() => _RedeemRecordsPanelState();
}

class _RedeemRecordsPanelState extends State<AdminRedeemRecordsPanel> {
  final TextEditingController _adminTokenController = TextEditingController(
    text: 'demo-admin',
  );

  List<RedeemCodeBatch> _batches = const [];
  RedeemCodeBatch? _selectedBatch;
  String? _status;
  bool _busy = false;

  String get _adminToken => widget.adminToken?.trim().isNotEmpty == true
      ? widget.adminToken!.trim()
      : _adminTokenController.text.trim();

  @override
  void initState() {
    super.initState();
    unawaited(_loadBatches());
  }

  @override
  void dispose() {
    _adminTokenController.dispose();
    super.dispose();
  }

  Future<void> _loadBatches() async {
    if (_busy) return;
    setState(() {
      _busy = true;
      _status = AppStrings.of(context).t('status.working');
    });
    try {
      final client = CommercialApiClient(widget.apiBaseUrl);
      final batches = await client.redeemCodeBatches(adminToken: _adminToken);
      if (!mounted) return;
      setState(() {
        _batches = batches;
        _selectedBatch = null;
        _status = AppStrings.of(
          context,
        ).t('redeemRecords.loaded').fill({'count': batches.length});
      });
    } on Object catch (e) {
      if (!mounted) return;
      setState(() => _status = e.toString());
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _openBatch(RedeemCodeBatch batch) async {
    if (_busy) return;
    setState(() {
      _busy = true;
      _status = AppStrings.of(context).t('status.working');
    });
    try {
      final client = CommercialApiClient(widget.apiBaseUrl);
      final detail = await client.redeemCodeBatchDetail(
        adminToken: _adminToken,
        batchId: batch.batchId,
      );
      if (!mounted) return;
      setState(() {
        _selectedBatch = detail;
        _status = AppStrings.of(
          context,
        ).t('redeemRecords.detailLoaded').fill({'batch': detail.batchNo});
      });
    } on Object catch (e) {
      if (!mounted) return;
      setState(() => _status = e.toString());
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _downloadExcel(RedeemCodeBatch batch) async {
    if (_busy) return;
    setState(() {
      _busy = true;
      _status = AppStrings.of(context).t('status.working');
    });
    try {
      final client = CommercialApiClient(widget.apiBaseUrl);
      final bytes = await client.exportRedeemCodeBatch(
        adminToken: _adminToken,
        batchId: batch.batchId,
      );
      downloadBlob(
        Uint8List.fromList(bytes),
        'redeem-codes-${batch.batchNo}.xlsx',
      );
      if (!mounted) return;
      setState(
        () => _status = AppStrings.of(context).t('redeemManage.exported'),
      );
    } on Object catch (e) {
      if (!mounted) return;
      setState(() => _status = e.toString());
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final theme = Theme.of(context);
    final selected = _selectedBatch;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        AppSectionHeader(
          title: strings.t('nav.redeemRecords'),
          description: strings.t('redeemRecords.description'),
        ),
        const SizedBox(height: AppSpacing.lg),
        if (widget.adminToken == null) ...[
          TextField(
            controller: _adminTokenController,
            enabled: !_busy,
            obscureText: true,
            decoration: InputDecoration(
              labelText: strings.t('redeemManage.adminToken'),
              prefixIcon: const Icon(Icons.admin_panel_settings_outlined),
            ),
          ),
          const SizedBox(height: AppSpacing.md),
        ],
        Wrap(
          spacing: AppSpacing.sm,
          runSpacing: AppSpacing.sm,
          children: [
            FilledButton.icon(
              onPressed: _busy ? null : _loadBatches,
              icon: const Icon(Icons.refresh),
              label: Text(strings.t('common.refresh')),
            ),
            if (selected != null)
              FilledButton.tonalIcon(
                onPressed: _busy ? null : () => _downloadExcel(selected),
                icon: const Icon(Icons.download),
                label: Text(strings.t('redeemManage.downloadExcel')),
              ),
          ],
        ),
        if (_status != null) ...[
          const SizedBox(height: AppSpacing.md),
          Text(_status!, style: theme.textTheme.bodySmall),
        ],
        const SizedBox(height: AppSpacing.lg),
        if (_batches.isEmpty)
          _KeyValueList(entries: [strings.t('empty.noData')])
        else
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: DataTable(
              columns: [
                DataColumn(label: Text(strings.t('redeemRecords.batchNo'))),
                DataColumn(label: Text(strings.t('redeemManage.package'))),
                DataColumn(label: Text(strings.t('redeemRecords.generated'))),
                DataColumn(label: Text(strings.t('redeemManage.status'))),
                DataColumn(label: Text(strings.t('redeemRecords.channel'))),
                DataColumn(label: Text(strings.t('redeemManage.createdAt'))),
                DataColumn(label: Text(strings.t('redeemRecords.actions'))),
              ],
              rows: _batches
                  .map(
                    (batch) => DataRow(
                      selected: selected?.batchId == batch.batchId,
                      cells: [
                        DataCell(Text(batch.batchNo)),
                        DataCell(Text(batch.packageName)),
                        DataCell(Text(batch.generatedCount.toString())),
                        DataCell(Text(batch.status)),
                        DataCell(Text(batch.channel ?? '-')),
                        DataCell(Text(batch.createdAt)),
                        DataCell(
                          Wrap(
                            spacing: AppSpacing.xs,
                            children: [
                              IconButton(
                                tooltip: strings.t('redeemRecords.viewDetail'),
                                onPressed: _busy
                                    ? null
                                    : () => _openBatch(batch),
                                icon: const Icon(Icons.visibility_outlined),
                              ),
                              IconButton(
                                tooltip: strings.t(
                                  'redeemManage.downloadExcel',
                                ),
                                onPressed: _busy
                                    ? null
                                    : () => _downloadExcel(batch),
                                icon: const Icon(Icons.download),
                              ),
                            ],
                          ),
                        ),
                      ],
                    ),
                  )
                  .toList(growable: false),
            ),
          ),
        if (selected != null) ...[
          const SizedBox(height: AppSpacing.lg),
          Text(selected.label, style: theme.textTheme.titleSmall),
          const SizedBox(height: AppSpacing.sm),
          _KeyValueList(
            entries: [
              '${strings.t('redeemManage.batchId')}: ${selected.batchId}',
              '${strings.t('redeemRecords.exported')}: ${selected.exportedCount}',
              '${strings.t('redeemManage.note')}: ${selected.note ?? '-'}',
            ],
          ),
          const SizedBox(height: AppSpacing.md),
          Text(
            strings.t('redeemManage.previewCodes'),
            style: theme.textTheme.titleSmall,
          ),
          const SizedBox(height: AppSpacing.sm),
          _KeyValueList(
            entries: selected.codes.isEmpty
                ? [strings.t('empty.noData')]
                : selected.codes.take(60).toList(growable: false),
          ),
        ],
      ],
    );
  }
}

// ─── Convert panel (existing, kept for conversion section) ─────────────────────

enum _ConvertState { idle, converting, success, error }

class _ConvertPanel extends StatefulWidget {
  final String apiBaseUrl;
  final String accessToken;

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

    setState(() {
      _state = _ConvertState.converting;
      _statusText = strings.t('convert.converting');
      _errorText = null;
    });
    _addLog(strings.t('convert.logStarted').fill({'main': mainTex}));

    try {
      final t0 = DateTime.now();
      final docx = await _convertCloud(bytes, mainTex);
      if (!mounted) return;

      _elapsedMs = DateTime.now().difference(t0).inMilliseconds;
      _docxBytes = docx;

      setState(() {
        _state = _ConvertState.success;
        _statusText = strings.t('convert.cloudSuccess').fill({
          'size': (docx.length / 1024).toStringAsFixed(1),
          'elapsed': _elapsedMs ?? 0,
        });
      });
      _addLog(strings.t('convert.logFinished'));
    } on Object catch (e) {
      if (!mounted) return;
      setState(() {
        _state = _ConvertState.error;
        _errorText = e.toString();
      });
      _addLog(strings.t('convert.logFailed').fill({'error': e}));
    }
  }

  Future<Uint8List> _convertCloud(Uint8List bytes, String mainTex) async {
    final strings = AppStrings.of(context);
    final client = CommercialApiClient(widget.apiBaseUrl);
    _addLog(strings.t('convert.logUploading'));
    final upload = await client.uploadProjectZip(
      accessToken: widget.accessToken,
      bytes: bytes,
      fileName: _zipFileName ?? 'project.zip',
    );
    _addLog(strings.t('convert.logUploaded').fill({'upload': upload.uploadId}));
    final created = await client.createConversion(
      accessToken: widget.accessToken,
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
          accessToken: widget.accessToken,
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
        accessToken: widget.accessToken,
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

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        AppSectionHeader(
          title: strings.t('nav.convert'),
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
          icon: Icons.verified_user_outlined,
          label: strings.t('convert.signedInReady'),
          color: tokens.info,
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
              onPressed: _state == _ConvertState.converting ? null : _pickFile,
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
      ],
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

// ─── Helpers ──────────────────────────────────────────────────────────────────

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
