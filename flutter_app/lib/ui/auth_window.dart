import 'package:flutter/material.dart';

import '../commercial_api.dart';
import 'app_components.dart';
import 'app_i18n.dart';
import 'app_tokens.dart';

class AuthWindow extends StatefulWidget {
  final String apiBaseUrl;
  final void Function(
    String apiBaseUrl,
    String accessToken,
    UserProfile profile,
  )
  onSignedIn;

  const AuthWindow({
    super.key,
    required this.apiBaseUrl,
    required this.onSignedIn,
  });

  @override
  State<AuthWindow> createState() => _AuthWindowState();
}

class _AuthWindowState extends State<AuthWindow>
    with SingleTickerProviderStateMixin {
  late TabController _tabController;
  final _baseUrlController = TextEditingController();
  final _emailController = TextEditingController();
  final _passwordController = TextEditingController();
  final _confirmPasswordController = TextEditingController();

  String? _status;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
    _baseUrlController.text = widget.apiBaseUrl;
    _emailController.text = _defaultRegisterEmail();
    _passwordController.text = '123456';
    _confirmPasswordController.text = '123456';
  }

  @override
  void dispose() {
    _tabController.dispose();
    _baseUrlController.dispose();
    _emailController.dispose();
    _passwordController.dispose();
    _confirmPasswordController.dispose();
    super.dispose();
  }

  CommercialApiClient _client() => CommercialApiClient(_baseUrlController.text);

  Future<void> _run(
    Future<void> Function(CommercialApiClient client) action,
  ) async {
    if (_busy) return;
    setState(() {
      _busy = true;
      _status = null;
    });
    try {
      await action(_client());
    } on Object catch (e) {
      if (!mounted) return;
      setState(() => _status = e.toString());
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _login() async {
    await _run((client) async {
      final email = _emailController.text.trim();
      final password = _passwordController.text;
      if (email.isEmpty || password.isEmpty) {
        throw Exception('Please enter email and password.');
      }
      final auth = await client.login(email: email, password: password);
      if (!mounted) return;
      widget.onSignedIn(_baseUrlController.text, auth.accessToken, auth.user);
    });
  }

  Future<void> _register() async {
    final password = _passwordController.text;
    final confirm = _confirmPasswordController.text;
    if (password != confirm) {
      final strings = AppStrings.of(context);
      setState(() => _status = strings.t('account.passwordMismatch'));
      return;
    }
    if (password.length < 6) {
      final strings = AppStrings.of(context);
      setState(() => _status = strings.t('account.passwordTooShort'));
      return;
    }
    await _run((client) async {
      final email = _emailController.text.trim();
      final AuthResponse auth;
      try {
        auth = await client.register(email: email, password: password);
      } on CommercialApiException catch (error) {
        if (error.statusCode == 409) {
          if (mounted) {
            _tabController.animateTo(0);
          }
          throw Exception(
            'Account already exists. Please sign in or use another email.',
          );
        }
        rethrow;
      }
      if (!mounted) return;
      widget.onSignedIn(_baseUrlController.text, auth.accessToken, auth.user);
    });
  }

  String _defaultRegisterEmail() {
    final timestamp = DateTime.now().millisecondsSinceEpoch;
    return 'demo+$timestamp@example.com';
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final theme = Theme.of(context);

    return Scaffold(
      body: Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(AppSpacing.xl),
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 420),
            child: AppCard(
              padding: const EdgeInsets.all(AppSpacing.xl),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  // Logo + title
                  Row(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      ClipRRect(
                        borderRadius: BorderRadius.circular(AppRadius.sm),
                        child: Image.asset(
                          'assets/app_icon.png',
                          width: 48,
                          height: 48,
                          fit: BoxFit.cover,
                        ),
                      ),
                      const SizedBox(width: AppSpacing.md),
                      Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            strings.t('app.title'),
                            style: theme.textTheme.titleLarge,
                          ),
                          Text(
                            strings.t('app.subtitle'),
                            style: theme.textTheme.bodySmall,
                          ),
                        ],
                      ),
                    ],
                  ),
                  const SizedBox(height: AppSpacing.xl),

                  // Tab bar
                  TabBar(
                    controller: _tabController,
                    tabs: [
                      Tab(text: strings.t('auth.loginTab')),
                      Tab(text: strings.t('auth.registerTab')),
                    ],
                    labelColor: theme.colorScheme.primary,
                    unselectedLabelColor: theme.hintColor,
                    indicatorColor: theme.colorScheme.primary,
                    dividerColor: Colors.transparent,
                  ),
                  const SizedBox(height: AppSpacing.lg),

                  // Tab content
                  SizedBox(
                    height: 320,
                    child: TabBarView(
                      controller: _tabController,
                      children: [
                        _LoginForm(
                          baseUrlController: _baseUrlController,
                          emailController: _emailController,
                          passwordController: _passwordController,
                          busy: _busy,
                          onLogin: _login,
                          strings: strings,
                        ),
                        _RegisterForm(
                          baseUrlController: _baseUrlController,
                          emailController: _emailController,
                          passwordController: _passwordController,
                          confirmPasswordController: _confirmPasswordController,
                          busy: _busy,
                          onRegister: _register,
                          strings: strings,
                        ),
                      ],
                    ),
                  ),

                  const SizedBox(height: AppSpacing.md),

                  // Status
                  if (_busy)
                    LoadingState(label: strings.t('common.loading'))
                  else if (_status != null) ...[
                    StatusPill(
                      icon: Icons.info_outline,
                      label: _status!,
                      color: theme.colorScheme.error,
                    ),
                  ],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _LoginForm extends StatelessWidget {
  final TextEditingController baseUrlController;
  final TextEditingController emailController;
  final TextEditingController passwordController;
  final bool busy;
  final VoidCallback onLogin;
  final AppStrings strings;

  const _LoginForm({
    required this.baseUrlController,
    required this.emailController,
    required this.passwordController,
    required this.busy,
    required this.onLogin,
    required this.strings,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        AppTextField(
          controller: baseUrlController,
          label: strings.t('account.apiBaseUrl'),
        ),
        const SizedBox(height: AppSpacing.md),
        AppTextField(
          controller: emailController,
          label: strings.t('account.email'),
          hint: 'demo@example.com',
        ),
        const SizedBox(height: AppSpacing.md),
        _PasswordField(
          controller: passwordController,
          label: strings.t('account.password'),
          hint: '••••••••',
        ),
        const SizedBox(height: AppSpacing.lg),
        FilledButton(
          onPressed: busy ? null : onLogin,
          child: Text(strings.t('common.login')),
        ),
      ],
    );
  }
}

class _RegisterForm extends StatelessWidget {
  final TextEditingController baseUrlController;
  final TextEditingController emailController;
  final TextEditingController passwordController;
  final TextEditingController confirmPasswordController;
  final bool busy;
  final VoidCallback onRegister;
  final AppStrings strings;

  const _RegisterForm({
    required this.baseUrlController,
    required this.emailController,
    required this.passwordController,
    required this.confirmPasswordController,
    required this.busy,
    required this.onRegister,
    required this.strings,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        AppTextField(
          controller: baseUrlController,
          label: strings.t('account.apiBaseUrl'),
        ),
        const SizedBox(height: AppSpacing.md),
        AppTextField(
          controller: emailController,
          label: strings.t('account.email'),
          hint: 'your@email.com',
        ),
        const SizedBox(height: AppSpacing.md),
        _PasswordField(
          controller: passwordController,
          label: strings.t('account.password'),
          hint: '••••••••',
        ),
        const SizedBox(height: AppSpacing.md),
        _PasswordField(
          controller: confirmPasswordController,
          label: strings.t('account.confirmPassword'),
          hint: '••••••••',
        ),
        const SizedBox(height: AppSpacing.lg),
        FilledButton(
          onPressed: busy ? null : onRegister,
          child: Text(strings.t('common.register')),
        ),
      ],
    );
  }
}

class _PasswordField extends StatefulWidget {
  final TextEditingController controller;
  final String label;
  final String hint;

  const _PasswordField({
    required this.controller,
    required this.label,
    required this.hint,
  });

  @override
  State<_PasswordField> createState() => _PasswordFieldState();
}

class _PasswordFieldState extends State<_PasswordField> {
  bool _obscured = true;

  @override
  Widget build(BuildContext context) {
    return TextField(
      controller: widget.controller,
      obscureText: _obscured,
      decoration: InputDecoration(
        labelText: widget.label,
        hintText: widget.hint,
        suffixIcon: IconButton(
          icon: Icon(
            _obscured ? Icons.visibility_off : Icons.visibility,
            size: 20,
          ),
          onPressed: () => setState(() => _obscured = !_obscured),
          tooltip: _obscured ? 'Show password' : 'Hide password',
        ),
      ),
    );
  }
}
