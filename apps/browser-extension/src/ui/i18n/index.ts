export const translations = {
  en: {
    // Common
    appName: 'Tex2Doc',
    loading: 'Loading...',
    error: 'Error',
    success: 'Success',
    cancel: 'Cancel',
    confirm: 'Confirm',
    save: 'Save',
    delete: 'Delete',
    retry: 'Retry',
    close: 'Close',
    refresh: 'Refresh',
    download: 'Download',
    upload: 'Upload',
    signIn: 'Sign In',
    signOut: 'Sign Out',
    register: 'Register',

    // Auth
    email: 'Email',
    password: 'Password',
    displayName: 'Display Name',
    forgotPassword: 'Forgot Password?',
    noAccount: "Don't have an account?",
    hasAccount: 'Already have an account?',
    signInTitle: 'Sign in to Tex2Doc',
    signOutSuccess: 'Signed out successfully',

    // Account
    account: 'Account',
    usage: 'Usage',
    plan: 'Plan',
    remaining: 'Remaining',
    quotaExceeded: 'Quota Exceeded',
    upgrade: 'Upgrade',

    // Conversion
    convert: 'Convert',
    converting: 'Converting...',
    conversionComplete: 'Conversion Complete',
    conversionFailed: 'Conversion Failed',
    selectFile: 'Select File',
    selectZipFile: 'Select ZIP file',
    mainTexFile: 'Main TeX file',
    profile: 'Profile',
    quality: 'Quality',
    mode: 'Mode',
    localMode: 'Local (WASM)',
    cloudMode: 'Cloud',
    autoMode: 'Auto',
    profiles: {
      standard: 'Standard',
      academic: 'Academic',
      publication: 'Publication',
    },
    qualities: {
      preview: 'Preview',
      balanced: 'Balanced',
      strict: 'Strict',
    },

    // Jobs
    jobs: 'Jobs',
    jobHistory: 'Job History',
    currentJob: 'Current Job',
    noJobs: 'No conversion jobs yet',
    jobStatus: {
      pending: 'Pending',
      processing: 'Processing',
      completed: 'Completed',
      failed: 'Failed',
      expired: 'Expired',
    },

    // Billing
    billing: 'Billing',
    plans: 'Plans',
    recharge: 'Recharge',
    redeemCode: 'Redeem Code',
    enterCode: 'Enter code',
    checkout: 'Checkout',
    portal: 'Billing Portal',
    rechargeSuccess: 'Recharge successful',

    // Feedback
    feedback: 'Feedback',
    submitFeedback: 'Submit Feedback',
    feedbackTitle: 'Title',
    feedbackContent: 'Description',
    feedbackType: 'Type',
    feedbackTypes: {
      issue: 'Issue',
      requirement: 'Feature Request',
      other: 'Other',
    },

    // Settings
    settings: 'Settings',
    apiBaseUrl: 'API Base URL',
    defaultSettings: 'Default Settings',
    fileSizeLimit: 'File Size Limit (WASM)',
    language: 'Language',
    theme: 'Theme',
    themes: {
      light: 'Light',
      dark: 'Dark',
      system: 'System',
    },
    domainPermissions: 'Domain Permissions',

    // Content Scripts
    convertPage: 'Convert This Page',
    overleafContext: 'Convert from Overleaf',
    arxivContext: 'Download from arXiv',

    // Errors
    errors: {
      networkError: 'Network error. Please check your connection.',
      authError: 'Authentication failed. Please sign in again.',
      quotaExceeded: 'Conversion quota exceeded. Please upgrade your plan.',
      conversionFailed: 'Conversion failed. Please try again or contact support.',
      fileTooLarge: 'File is too large. Maximum size is {size}.',
      invalidFile: 'Invalid file format. Please select a ZIP file.',
      wasmLoadFailed: 'Failed to load local conversion engine.',
      sessionExpired: 'Session expired. Please sign in again.',
    },

    // Tooltips
    tooltips: {
      wasmPrivate: 'Local conversion keeps your files private - nothing is uploaded.',
      cloudFast: 'Cloud conversion supports larger files and complex templates.',
    },
  },

  zh: {
    // Common
    appName: 'Tex2Doc',
    loading: '加载中...',
    error: '错误',
    success: '成功',
    cancel: '取消',
    confirm: '确认',
    save: '保存',
    delete: '删除',
    retry: '重试',
    close: '关闭',
    refresh: '刷新',
    download: '下载',
    upload: '上传',
    signIn: '登录',
    signOut: '退出登录',
    register: '注册',

    // Auth
    email: '邮箱',
    password: '密码',
    displayName: '显示名称',
    forgotPassword: '忘记密码？',
    noAccount: '没有账号？',
    hasAccount: '已有账号？',
    signInTitle: '登录 Tex2Doc',
    signOutSuccess: '已成功退出登录',

    // Account
    account: '账户',
    usage: '用量',
    plan: '套餐',
    remaining: '剩余',
    quotaExceeded: '额度用尽',
    upgrade: '升级',

    // Conversion
    convert: '转换',
    converting: '转换中...',
    conversionComplete: '转换完成',
    conversionFailed: '转换失败',
    selectFile: '选择文件',
    selectZipFile: '选择 ZIP 文件',
    mainTexFile: '主 TeX 文件',
    profile: '配置文件',
    quality: '质量',
    mode: '模式',
    localMode: '本地（WASM）',
    cloudMode: '云端',
    autoMode: '自动',
    profiles: {
      standard: '标准',
      academic: '学术',
      publication: '出版',
    },
    qualities: {
      preview: '预览',
      balanced: '平衡',
      strict: '严格',
    },

    // Jobs
    jobs: '任务',
    jobHistory: '任务历史',
    currentJob: '当前任务',
    noJobs: '暂无转换任务',
    jobStatus: {
      pending: '等待中',
      processing: '处理中',
      completed: '已完成',
      failed: '失败',
      expired: '已过期',
    },

    // Billing
    billing: '计费',
    plans: '套餐',
    recharge: '充值',
    redeemCode: '兑换码',
    enterCode: '输入兑换码',
    checkout: '结账',
    portal: '账单门户',
    rechargeSuccess: '充值成功',

    // Feedback
    feedback: '反馈',
    submitFeedback: '提交反馈',
    feedbackTitle: '标题',
    feedbackContent: '描述',
    feedbackType: '类型',
    feedbackTypes: {
      issue: '问题',
      requirement: '功能请求',
      other: '其他',
    },

    // Settings
    settings: '设置',
    apiBaseUrl: 'API 基础 URL',
    defaultSettings: '默认设置',
    fileSizeLimit: '文件大小限制（WASM）',
    language: '语言',
    theme: '主题',
    themes: {
      light: '浅色',
      dark: '深色',
      system: '跟随系统',
    },
    domainPermissions: '域名权限',

    // Content Scripts
    convertPage: '转换此页面',
    overleafContext: '从 Overleaf 转换',
    arxivContext: '从 arXiv 下载',

    // Errors
    errors: {
      networkError: '网络错误，请检查网络连接。',
      authError: '认证失败，请重新登录。',
      quotaExceeded: '转换额度已用尽，请升级套餐。',
      conversionFailed: '转换失败，请重试或联系客服。',
      fileTooLarge: '文件过大，最大支持 {size}。',
      invalidFile: '文件格式无效，请选择 ZIP 文件。',
      wasmLoadFailed: '本地转换引擎加载失败。',
      sessionExpired: '会话已过期，请重新登录。',
    },

    // Tooltips
    tooltips: {
      wasmPrivate: '本地转换保护隐私 - 文件不会上传。',
      cloudFast: '云端转换支持更大文件和复杂模板。',
    },
  },
} as const;

export type Locale = keyof typeof translations;
export type TranslationKey = keyof (typeof translations)['en'];

// Helper function to get nested translation
export function t(locale: Locale, key: string, params?: Record<string, string | number>): string {
  const keys = key.split('.');
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let value: any = translations[locale];

  for (const k of keys) {
    value = value?.[k];
  }

  if (typeof value !== 'string') {
    return key;
  }

  if (params) {
    return value.replace(/\{(\w+)\}/g, (_, k) => String(params[k] ?? `{${k}}`));
  }

  return value;
}
