export const translations = {
  en: {
    // Common
    appName: 'Tex2Doc',
    tagline: 'LaTeX to DOCX in your browser',
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
    redeem: 'Redeem',
    redeemCodeShort: 'Redeem Code',
    language: 'Language',
    theme: 'Theme',

    // Auth
    email: 'Email',
    password: 'Password',
    displayName: 'Display Name',
    forgotPassword: 'Forgot Password?',
    noAccount: "Don't have an account?",
    hasAccount: 'Already have an account?',
    signInTitle: 'Sign in to Tex2Doc',
    signOutSuccess: 'Signed out successfully',
    authTabs: {
      signIn: 'Sign In',
      redeem: 'Redeem Code',
    },
    redeemTitle: 'Redeem a Code',
    redeemDescription: 'Paste your code below to unlock cloud conversions.',
    redeemAutoRegister: 'No account yet? A new account will be created and signed in automatically.',
    redeemPlaceholder: 'Enter your code (e.g. T2D-XXXX-XXXX)',
    redeemRequiresLogin: 'Please sign in first, or use a code that provisions an account.',
    redeemSuccessNewAccount: 'Account created and signed in.',
    redeemSuccessRecharged: 'Code redeemed successfully.',
    redeemFailed: 'Failed to redeem code',

    // Account
    account: 'Account',
    usage: 'Usage',
    plan: 'Plan',
    remaining: 'Remaining',
    quotaExceeded: 'Quota Exceeded',
    upgrade: 'Upgrade',
    signInRequired: 'Sign in required',
    signInOrRedeem: 'Sign in or redeem a code to use cloud conversion',

    // Conversion
    convert: 'Convert',
    converting: 'Converting...',
    conversionComplete: 'Conversion Complete',
    conversionFailed: 'Conversion Failed',
    selectFile: 'Select a ZIP file to begin',
    selectZipFile: 'Select ZIP file',
    selectZipHint: 'Drag a .zip here or click to browse',
    mainTexFile: 'Main TeX file',
    mainTexAutoDetected: 'Auto-detected main TeX',
    mainTexPickFromList: 'Found {count} .tex files — pick the main one',
    noTexFound: 'No .tex files found in the ZIP archive',
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
    cloud: {
      uploading: 'Uploading ZIP...',
      creating: 'Creating conversion job...',
      polling: 'Server is converting...',
      completed: 'Conversion completed',
      failed: 'Cloud conversion failed',
      stageLabel: {
        uploading: 'Uploading',
        creating: 'Creating',
        polling: 'Converting',
        completed: 'Done',
        failed: 'Failed',
      },
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
    noPlans: 'No plans available',

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
    settingsTabs: {
      general: 'General',
      conversion: 'Conversion Defaults',
      permissions: 'Domain Permissions',
      about: 'About',
    },
    apiBaseUrl: 'API Base URL',
    defaultSettings: 'Default Settings',
    defaultMode: 'Default Mode',
    defaultProfile: 'Default Profile',
    defaultQuality: 'Default Quality',
    fileSizeLimit: 'File Size Limit (WASM)',
    themeSettings: {
      light: 'Light',
      dark: 'Dark',
      system: 'System',
    },
    domainPermissions: 'Domain Permissions',
    domainPermissionsDescription: 'Tex2Doc can read LaTeX content from these domains.',
    domainAdd: 'Add domain',
    domainPlaceholder: 'example.com',
    aboutVersion: 'Version',
    aboutLinks: 'Useful Links',
    aboutCopyright: 'Copyright',

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
      invalidCode: 'Invalid code format',
      unknown: 'An unexpected error occurred.',
    },

    // Empty / loading
    empty: {
      noJobs: {
        title: 'No conversion jobs yet',
        description: 'Upload a LaTeX ZIP and run your first conversion to see results here.',
      },
      noPlans: {
        title: 'No plans available',
        description: 'Pricing options will appear once the billing service is reachable.',
      },
      noFeedback: {
        title: 'No feedback threads',
        description: 'Submit your first piece of feedback to start a thread.',
      },
    },
    loadingStates: {
      preparingConversion: 'Preparing conversion...',
      loadingPlans: 'Loading plans...',
      loadingUsage: 'Loading usage...',
      loadingJobs: 'Loading jobs...',
    },

    // Tooltips
    tooltips: {
      wasmPrivate: 'Local conversion keeps your files private - nothing is uploaded.',
      cloudFast: 'Cloud conversion supports larger files and complex templates.',
    },

    // Actions
    actions: {
      copyErrorLog: 'Copy Error Log',
      copied: 'Copied',
      showDetails: 'Show Details',
      hideDetails: 'Hide Details',
      errorDetails: 'Error Details',
      signInToRedeem: 'Sign in to redeem',
      or: 'or',
      auto: 'Auto',
    },
  },

  zh: {
    // Common
    appName: 'Tex2Doc',
    tagline: '浏览器内 LaTeX 转 Word',
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
    redeem: '兑换',
    redeemCodeShort: '兑换码',
    language: '语言',
    theme: '主题',

    // Auth
    email: '邮箱',
    password: '密码',
    displayName: '显示名称',
    forgotPassword: '忘记密码？',
    noAccount: '没有账号？',
    hasAccount: '已有账号？',
    signInTitle: '登录 Tex2Doc',
    signOutSuccess: '已成功退出登录',
    authTabs: {
      signIn: '登录',
      redeem: '兑换码',
    },
    redeemTitle: '兑换码登录 / 充值',
    redeemDescription: '在下方粘贴兑换码，即可解锁云端转换。',
    redeemAutoRegister: '尚未注册？输入兑换码将自动创建账户并登录。',
    redeemPlaceholder: '请输入兑换码（如 T2D-XXXX-XXXX）',
    redeemRequiresLogin: '请先登录，或使用会自动开通账户的兑换码。',
    redeemSuccessNewAccount: '账户已创建并已登录。',
    redeemSuccessRecharged: '兑换码已成功使用。',
    redeemFailed: '兑换失败',

    // Account
    account: '账户',
    usage: '用量',
    plan: '套餐',
    remaining: '剩余',
    quotaExceeded: '额度用尽',
    upgrade: '升级',
    signInRequired: '请先登录',
    signInOrRedeem: '登录或使用兑换码以解锁云端转换',

    // Conversion
    convert: '转换',
    converting: '转换中...',
    conversionComplete: '转换完成',
    conversionFailed: '转换失败',
    selectFile: '请选择 ZIP 文件开始',
    selectZipFile: '选择 ZIP 文件',
    selectZipHint: '将 .zip 拖到此处，或点击浏览',
    mainTexFile: '主 TeX 文件',
    mainTexAutoDetected: '已自动识别主 TeX 文件',
    mainTexPickFromList: '共识别到 {count} 个 .tex 文件，请选择主文件',
    noTexFound: 'ZIP 包内未找到任何 .tex 文件',
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
    cloud: {
      uploading: '正在上传 ZIP...',
      creating: '正在创建转换任务...',
      polling: '服务端正在转换...',
      completed: '转换完成',
      failed: '云端转换失败',
      stageLabel: {
        uploading: '上传',
        creating: '创建',
        polling: '转换中',
        completed: '完成',
        failed: '失败',
      },
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
    noPlans: '暂无可用套餐',

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
    settingsTabs: {
      general: '通用',
      conversion: '默认转换',
      permissions: '域名权限',
      about: '关于',
    },
    apiBaseUrl: 'API 基础 URL',
    defaultSettings: '默认设置',
    defaultMode: '默认模式',
    defaultProfile: '默认配置',
    defaultQuality: '默认质量',
    fileSizeLimit: '文件大小限制（WASM）',
    themeSettings: {
      light: '浅色',
      dark: '深色',
      system: '跟随系统',
    },
    domainPermissions: '域名权限',
    domainPermissionsDescription: 'Tex2Doc 可以从以下域名读取 LaTeX 内容。',
    domainAdd: '添加域名',
    domainPlaceholder: 'example.com',
    aboutVersion: '版本',
    aboutLinks: '相关链接',
    aboutCopyright: '版权',

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
      invalidCode: '兑换码格式无效',
      unknown: '出现未知错误。',
    },

    // Empty / loading
    empty: {
      noJobs: {
        title: '暂无转换任务',
        description: '上传 LaTeX ZIP 包并完成首次转换，结果将显示在这里。',
      },
      noPlans: {
        title: '暂无可用套餐',
        description: '账单服务可达后将在此显示套餐选项。',
      },
      noFeedback: {
        title: '暂无反馈',
        description: '提交第一条反馈，开启对话。',
      },
    },
    loadingStates: {
      preparingConversion: '正在准备转换...',
      loadingPlans: '正在加载套餐...',
      loadingUsage: '正在加载用量...',
      loadingJobs: '正在加载任务...',
    },

    // Tooltips
    tooltips: {
      wasmPrivate: '本地转换保护隐私 - 文件不会上传。',
      cloudFast: '云端转换支持更大文件和复杂模板。',
    },

    // Actions
    actions: {
      copyErrorLog: '复制错误日志',
      copied: '已复制',
      showDetails: '显示详情',
      hideDetails: '隐藏详情',
      errorDetails: '错误详情',
      signInToRedeem: '登录后兑换',
      or: '或',
      auto: '自动',
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