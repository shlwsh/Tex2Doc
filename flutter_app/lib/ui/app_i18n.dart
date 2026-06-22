import 'package:flutter/widgets.dart';

enum AppLocale { zhCn, enUs }

extension AppLocaleMeta on AppLocale {
  Locale get locale => switch (this) {
    AppLocale.zhCn => const Locale('zh', 'CN'),
    AppLocale.enUs => const Locale('en', 'US'),
  };

  String get storageKey => switch (this) {
    AppLocale.zhCn => 'zh-CN',
    AppLocale.enUs => 'en-US',
  };

  String get label => switch (this) {
    AppLocale.zhCn => '简体中文',
    AppLocale.enUs => 'English',
  };

  static AppLocale fromStorageKey(String? value) {
    return AppLocale.values.firstWhere(
      (locale) => locale.storageKey == value,
      orElse: () => AppLocale.zhCn,
    );
  }

  static AppLocale fromLocale(Locale locale) {
    if (locale.languageCode.toLowerCase() == 'en') return AppLocale.enUs;
    return AppLocale.zhCn;
  }
}

class AppStrings {
  final AppLocale locale;

  const AppStrings(this.locale);

  static AppStrings of(BuildContext context) {
    final locale = Localizations.localeOf(context);
    return AppStrings(AppLocaleMeta.fromLocale(locale));
  }

  String t(String key) {
    final table = _localized[locale] ?? _localized[AppLocale.zhCn]!;
    return table[key] ?? _localized[AppLocale.enUs]![key] ?? key;
  }
}

class AppStringsDelegate extends LocalizationsDelegate<AppStrings> {
  const AppStringsDelegate();

  @override
  bool isSupported(Locale locale) => ['zh', 'en'].contains(locale.languageCode);

  @override
  Future<AppStrings> load(Locale locale) async {
    return AppStrings(AppLocaleMeta.fromLocale(locale));
  }

  @override
  bool shouldReload(AppStringsDelegate old) => false;
}

const Map<AppLocale, Map<String, String>> _localized = {
  AppLocale.zhCn: {
    'app.title': 'Tex2Doc',
    'app.subtitle': 'LaTeX 到 DOCX 的商业级转换工作台',
    'topbar.platform': '平台',
    'nav.dashboard': '工作台',
    'nav.account': '账号',
    'nav.convert': '转换',
    'settings.theme': '主题',
    'settings.language': '语言',
    'theme.default': '默认',
    'theme.blue': '蓝色',
    'theme.green': '绿色',
    'theme.purple': '紫色',
    'theme.orange': '橙色',
    'theme.dark': '深色',
    'common.register': '注册',
    'common.login': '登录',
    'common.refresh': '刷新',
    'common.plans': '套餐',
    'common.upload': '选择 ZIP',
    'common.convert': '开始转换',
    'common.download': '下载 DOCX',
    'common.ready': '就绪',
    'common.loading': '处理中...',
    'common.empty': '暂无数据',
    'common.error': '出错',
    'common.disabled': '不可用',
    'common.permissionDenied': '权限不足',
    'status.engineReady': '转换引擎已就绪',
    'status.engineError': '转换引擎初始化失败',
    'status.signedOut': '未登录',
    'status.working': '处理中...',
    'status.signInFirst': '请先登录后再刷新用量。',
    'account.title': '云端账号',
    'account.description': '连接本地或云端商业 API，管理订阅和云转换额度。',
    'account.apiBaseUrl': 'API 地址',
    'account.email': '邮箱',
    'account.password': '密码',
    'account.registered': '已注册 {email}，套餐 {plan}',
    'account.signedIn': '已登录 {email}，套餐 {plan}',
    'account.usage': '套餐 {plan}：{used}/{limit}，剩余 {remaining}',
    'account.plansLoaded': '已加载套餐',
    'convert.title': '文档转换',
    'convert.description': '上传 TeX 项目 ZIP，选择主文件并生成 DOCX。',
    'convert.mainTex': '主 TeX 文件',
    'convert.mainTexHint': 'main-jos.tex',
    'convert.noFile': '未选择文件',
    'convert.fileTooLarge': '文件 {size} MB，超过 5 MB 上限。请使用桌面 App。',
    'convert.converting': '正在转换...',
    'convert.success': '完成 {size} KB，用时 {elapsed} ms',
    'convert.output': '产物',
    'metrics.quota': '云端额度',
    'metrics.engine': '引擎状态',
    'metrics.document': '文档产物',
    'empty.noData': '暂无数据。完成一次操作后这里会显示结果。',
    'error.network': '网络或服务异常，请检查 API 地址。',
  },
  AppLocale.enUs: {
    'app.title': 'Tex2Doc',
    'app.subtitle': 'Commercial LaTeX to DOCX conversion workspace',
    'topbar.platform': 'Platform',
    'nav.dashboard': 'Workspace',
    'nav.account': 'Account',
    'nav.convert': 'Convert',
    'settings.theme': 'Theme',
    'settings.language': 'Language',
    'theme.default': 'Default',
    'theme.blue': 'Blue',
    'theme.green': 'Green',
    'theme.purple': 'Purple',
    'theme.orange': 'Orange',
    'theme.dark': 'Dark',
    'common.register': 'Register',
    'common.login': 'Login',
    'common.refresh': 'Refresh',
    'common.plans': 'Plans',
    'common.upload': 'Choose ZIP',
    'common.convert': 'Start conversion',
    'common.download': 'Download DOCX',
    'common.ready': 'Ready',
    'common.loading': 'Working...',
    'common.empty': 'No data',
    'common.error': 'Error',
    'common.disabled': 'Disabled',
    'common.permissionDenied': 'Permission denied',
    'status.engineReady': 'Conversion engine is ready',
    'status.engineError': 'Conversion engine failed to initialize',
    'status.signedOut': 'Signed out',
    'status.working': 'Working...',
    'status.signInFirst': 'Sign in before refreshing usage.',
    'account.title': 'Cloud account',
    'account.description':
        'Connect to the commercial API for subscriptions and cloud quota.',
    'account.apiBaseUrl': 'API base URL',
    'account.email': 'Email',
    'account.password': 'Password',
    'account.registered': 'Registered {email}, plan {plan}',
    'account.signedIn': 'Signed in {email}, plan {plan}',
    'account.usage': 'Plan {plan}: {used}/{limit}, remaining {remaining}',
    'account.plansLoaded': 'Plans loaded',
    'convert.title': 'Document conversion',
    'convert.description':
        'Upload a TeX project ZIP, choose the main file, and export DOCX.',
    'convert.mainTex': 'Main TeX file',
    'convert.mainTexHint': 'main-jos.tex',
    'convert.noFile': 'No file selected',
    'convert.fileTooLarge':
        'File {size} MB exceeds the 5 MB limit. Use the desktop app.',
    'convert.converting': 'Converting...',
    'convert.success': 'Completed {size} KB in {elapsed} ms',
    'convert.output': 'Output',
    'metrics.quota': 'Cloud quota',
    'metrics.engine': 'Engine status',
    'metrics.document': 'Document output',
    'empty.noData': 'No data yet. Results will appear here after an operation.',
    'error.network': 'Network or service error. Check the API base URL.',
  },
};

extension LocalizedTemplate on String {
  String fill(Map<String, Object> values) {
    var result = this;
    for (final entry in values.entries) {
      result = result.replaceAll('{${entry.key}}', entry.value.toString());
    }
    return result;
  }
}
