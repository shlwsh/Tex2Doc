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
    'nav.recharge': '充值',
    'nav.convert': '转换',
    'nav.convertRecords': '转换记录',
    'nav.rechargeRecords': '充值记录',
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
    'common.confirm': '确定',
    'common.cancel': '取消',
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
    'account.confirmPassword': '确认密码',
    'account.plan': '套餐',
    'account.signInGate': '请先登录或注册。',
    'account.signedInShort': '已登录',
    'account.overviewTitle': '账号总览',
    'account.overviewDescription': '查看当前账号、套餐额度、充值记录与转换记录。',
    'account.queryRecords': '查询账号记录',
    'account.overviewLoaded': '已加载 {recharges} 条充值记录、{conversions} 条转换记录。',
    'account.registered': '已注册 {email}，套餐 {plan}',
    'account.signedIn': '已登录 {email}，套餐 {plan}',
    'account.usage': '套餐 {plan}：{used}/{limit}，剩余 {remaining}；权益 {entitlement}',
    'account.plansLoaded': '已加载套餐',
    'account.logout': '退出登录',
    'account.changePassword': '修改密码',
    'account.viewProfile': '查看详情',
    'account.profileTitle': '账号详情',
    'account.currentPlan': '当前套餐',
    'account.memberSince': '注册时间',
    'account.quotaUsed': '已用额度',
    'account.quotaRemaining': '剩余额度',
    'account.changePasswordTitle': '修改密码',
    'account.oldPassword': '当前密码',
    'account.newPassword': '新密码',
    'account.confirmNewPassword': '确认新密码',
    'account.passwordMismatch': '两次输入的密码不一致',
    'account.passwordChanged': '密码修改成功',
    'account.passwordChangeFailed': '密码修改失败',
    'account.passwordTooShort': '密码长度不能少于 6 位',
    'auth.loginTab': '登录',
    'auth.registerTab': '注册',
    'auth.loginTitle': '登录账号',
    'auth.registerTitle': '注册账号',
    'auth.signInFirst': '请先登录以继续操作。',
    'recharge.title': '充值',
    'recharge.description': '输入兑换码即可兑换转换额度，兑换成功后额度立即到账。',
    'recharge.packageTitle': '可兑换套餐',
    'recharge.queryRecords': '查询充值记录',
    'recharge.records': '充值记录',
    'recharge.signInRequired': '请先登录后再充值。',
    'recharge.redeemProvider': '兑换码充值已启用',
    'recharge.codeLabel': '兑换码',
    'recharge.codeHint': 'T2D-XXXX-XXXX-XXXX-XX',
    'recharge.codeRequired': '请输入兑换码。',
    'recharge.submitCode': '立即兑换',
    'recharge.redeemed': '兑换成功：{package} 已到账 {quantity} 次，当前按次余额 {balance}。',
    'recharge.redeemRecords': '兑换记录',
    'convert.title': '文档转换',
    'convert.description': '上传 TeX 项目 ZIP，选择主文件并生成 DOCX。',
    'convert.stepUpload': '1. 将完整 LaTeX 项目打包为 ZIP 后上传。',
    'convert.stepMainTex': '2. 填写 ZIP 内主 TeX 文件相对路径。',
    'convert.stepConvert': '3. 启动云端语义引擎并下载 DOCX。',
    'convert.packageHint': 'ZIP 根目录应包含主 tex、bib、图片、cls/sty 等依赖；不要只上传单个 tex 文件。',
    'convert.signedInReady': '已登录，可使用转换功能。',
    'convert.mainTex': '主 TeX 文件',
    'convert.mainTexHint': 'main-jos.tex',
    'convert.noFile': '未选择文件',
    'convert.fileTooLarge': '文件 {size} MB，超过 10 MB 上限。请使用桌面 App。',
    'convert.signInRequired': '请先注册或登录，以使用云端语义引擎转换。',
    'convert.converting': '正在转换...',
    'convert.success': '完成 {size} KB，用时 {elapsed} ms',
    'convert.cloudSuccess': '云端语义引擎完成 {size} KB，用时 {elapsed} ms',
    'convert.output': '产物',
    'convert.queryRecords': '查询转换记录',
    'convert.records': '转换记录',
    'convert.recordsLoaded': '已加载 {count} 条转换记录。',
    'convert.logs': '转换日志',
    'convert.logRejectedSize': '文件超过 10 MB，已拒绝上传。',
    'convert.logFileSelected': '已选择 {file}，大小 {size} MB。',
    'convert.logStarted': '开始转换，主文件 {main}。',
    'convert.logUploading': '正在上传 ZIP 到商业 API。',
    'convert.logUploaded': '上传完成，upload_id={upload}。',
    'convert.logJobCreated': '转换任务已创建，job_id={job}。',
    'convert.logPolling': '轮询任务状态：{status}。',
    'convert.logFinished': '转换完成，DOCX 已可下载。',
    'convert.logFailed': '转换失败：{error}',
    'convert.logRecordsLoaded': '已查询转换记录。',
    'metrics.quota': '云端额度',
    'metrics.entitlement': '转换权益',
    'metrics.countBalance': '按次余额 {count}',
    'metrics.dateValidUntil': '有效期至 {time}',
    'metrics.previewQuota': '预览额度',
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
    'nav.recharge': 'Recharge',
    'nav.convert': 'Convert',
    'nav.convertRecords': 'Conversion Records',
    'nav.rechargeRecords': 'Recharge Records',
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
    'common.confirm': 'OK',
    'common.cancel': 'Cancel',
    'status.engineReady': 'Conversion engine is ready',
    'status.engineError': 'Conversion engine failed to initialize',
    'status.signedOut': 'Signed out',
    'status.working': 'Working...',
    'status.signInFirst': 'Sign in before refreshing usage.',
    'account.title': 'Cloud Account',
    'account.description':
        'Connect to the commercial API for subscriptions and cloud quota.',
    'account.apiBaseUrl': 'API Base URL',
    'account.email': 'Email',
    'account.password': 'Password',
    'account.confirmPassword': 'Confirm Password',
    'account.plan': 'Plan',
    'account.signInGate': 'Sign in or register first.',
    'account.signedInShort': 'Signed in',
    'account.overviewTitle': 'Account Overview',
    'account.overviewDescription':
        'Review account profile, plan quota, recharge records, and conversions.',
    'account.queryRecords': 'Query Account Records',
    'account.overviewLoaded':
        'Loaded {recharges} recharge records and {conversions} conversions.',
    'account.registered': 'Registered {email}, plan {plan}',
    'account.signedIn': 'Signed in {email}, plan {plan}',
    'account.usage':
        'Plan {plan}: {used}/{limit}, remaining {remaining}; entitlement {entitlement}',
    'account.plansLoaded': 'Plans loaded',
    'account.logout': 'Sign Out',
    'account.changePassword': 'Change Password',
    'account.viewProfile': 'View Profile',
    'account.profileTitle': 'Account Details',
    'account.currentPlan': 'Current Plan',
    'account.memberSince': 'Member Since',
    'account.quotaUsed': 'Quota Used',
    'account.quotaRemaining': 'Quota Remaining',
    'account.changePasswordTitle': 'Change Password',
    'account.oldPassword': 'Current Password',
    'account.newPassword': 'New Password',
    'account.confirmNewPassword': 'Confirm New Password',
    'account.passwordMismatch': 'Passwords do not match',
    'account.passwordChanged': 'Password changed successfully',
    'account.passwordChangeFailed': 'Failed to change password',
    'account.passwordTooShort': 'Password must be at least 6 characters',
    'auth.loginTab': 'Login',
    'auth.registerTab': 'Register',
    'auth.loginTitle': 'Sign In',
    'auth.registerTitle': 'Create Account',
    'auth.signInFirst': 'Please sign in to continue.',
    'recharge.title': 'Recharge',
    'recharge.description':
        'Enter a redeem code to add conversion credits immediately.',
    'recharge.packageTitle': 'Redeemable packages',
    'recharge.queryRecords': 'Query recharge records',
    'recharge.records': 'Recharge records',
    'recharge.signInRequired': 'Sign in before recharge.',
    'recharge.redeemProvider': 'Redeem code recharge enabled',
    'recharge.codeLabel': 'Redeem code',
    'recharge.codeHint': 'T2D-XXXX-XXXX-XXXX-XX',
    'recharge.codeRequired': 'Enter a redeem code.',
    'recharge.submitCode': 'Redeem',
    'recharge.redeemed':
        'Redeemed {package}: {quantity} conversions added, count balance {balance}.',
    'recharge.redeemRecords': 'Redeem records',
    'convert.title': 'Document conversion',
    'convert.description':
        'Upload a TeX project ZIP, choose the main file, and export DOCX.',
    'convert.stepUpload': '1. Package the full LaTeX project as a ZIP.',
    'convert.stepMainTex': '2. Enter the main TeX path inside the ZIP.',
    'convert.stepConvert':
        '3. Run the cloud semantic engine and download DOCX.',
    'convert.packageHint':
        'The ZIP root should include the main tex, bib, images, cls/sty and other dependencies; do not upload only one tex file.',
    'convert.signedInReady': 'Signed in. Conversion is available.',
    'convert.mainTex': 'Main TeX file',
    'convert.mainTexHint': 'main-jos.tex',
    'convert.noFile': 'No file selected',
    'convert.fileTooLarge':
        'File {size} MB exceeds the 10 MB limit. Use the desktop app.',
    'convert.signInRequired':
        'Register or sign in first to use the cloud semantic engine.',
    'convert.converting': 'Converting...',
    'convert.success': 'Completed {size} KB in {elapsed} ms',
    'convert.cloudSuccess':
        'Cloud semantic engine completed {size} KB in {elapsed} ms',
    'convert.output': 'Output',
    'convert.queryRecords': 'Query conversion records',
    'convert.records': 'Conversion records',
    'convert.recordsLoaded': 'Loaded {count} conversion records.',
    'convert.logs': 'Conversion logs',
    'convert.logRejectedSize': 'File exceeds 10 MB and was rejected.',
    'convert.logFileSelected': 'Selected {file}, {size} MB.',
    'convert.logStarted': 'Started conversion with main file {main}.',
    'convert.logUploading': 'Uploading ZIP to the commercial API.',
    'convert.logUploaded': 'Upload completed, upload_id={upload}.',
    'convert.logJobCreated': 'Conversion job created, job_id={job}.',
    'convert.logPolling': 'Polling job status: {status}.',
    'convert.logFinished': 'Conversion completed. DOCX is ready.',
    'convert.logFailed': 'Conversion failed: {error}',
    'convert.logRecordsLoaded': 'Conversion records loaded.',
    'metrics.quota': 'Cloud quota',
    'metrics.entitlement': 'Conversion entitlement',
    'metrics.countBalance': 'count balance {count}',
    'metrics.dateValidUntil': 'valid until {time}',
    'metrics.previewQuota': 'preview quota',
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
