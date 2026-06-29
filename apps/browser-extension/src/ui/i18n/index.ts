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
      register: 'Register',
      redeem: 'Redeem Code',
    },
    workspace: 'Open workspace',
    bonus: {
      guestOffer: 'Register to receive {count} cloud conversions, valid for {days} days.',
      disabledOffer: 'Use local conversion free, or purchase and redeem cloud credit.',
      remainingCount: '{count} left',
      exhausted: 'Cloud credit is exhausted. Upgrade or redeem a code to continue.',
    },
    source: {
      title: 'LaTeX project',
      prompt: 'Choose a LaTeX ZIP or project folder',
      supported: 'Supports .zip and direct folder selection',
    },
    modeHelp: {
      local: 'Local conversion keeps files in your browser and uses no cloud credit.',
      cloud: 'Cloud conversion handles complex projects and uses one cloud credit.',
    },
    accountOverview: {
      newConversion: 'Start a new conversion',
      noExpiry: 'No expiry',
    },
    redeemTitle: 'Redeem a Code',
    redeemDescription: 'Paste your code below to unlock cloud conversions.',
    redeemAutoRegister:
      'No account yet? A new account will be created and signed in automatically.',
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
    // Folder upload
    or: 'or',
    selectFolder: 'Select Folder',
    selectFolderHint: 'Pick a LaTeX project folder - no need to zip it first.',
    folderScanning: 'Scanning folder...',
    folderReading: 'Reading... ({current}/{total})',
    folderPacking: 'Packing... ({current}/{total})',
    folderDetected: 'Detected {count} files in folder',
    folderExcluded: 'Excluded {count} build artifacts',
    folderTruncated: 'Folder too large, showing first {max} files',
    folderTooLarge: 'Project too large for browser memory',
    folderNotSupported: 'Your browser does not support folder selection',
    folderMainTexFromFolder: 'Main file detected in folder',
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
    validUntil: 'Valid until {date}',
    renewalHint: 'Renews on {date}',
    expiresInDays: 'Expires in {days} days',
    expiresToday: 'Expires today',
    renewalWarningTitle: 'Plan expiring soon',

    // Funnel analytics (P1-2)
    funnel: {
      title: 'Anonymous usage analytics',
      description:
        'Last 7 days of in-app events (no PII, no file content). Export to share with support.',
      export: 'Export funnel JSON',
    },

    // Diagnostics (P1-3)
    diagnostics: {
      title: 'Diagnostics',
      description:
        'Download a sanitized report of the last 200 events to attach to a support ticket.',
      export: 'Export diagnostics',
      exportSuccess: 'Diagnostics bundle saved.',
      privacyNote:
        'Bundle excludes tokens, source files, file contents, and your email. Only event metadata is included.',
    },

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
      folderScanFailed: 'Failed to scan folder',
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
      register: '注册',
      redeem: '兑换码',
    },
    workspace: '打开工作台',
    bonus: {
      guestOffer: '注册即获 {count} 次云端转换额度，有效期 {days} 天。',
      disabledOffer: '注册后可免费使用本地转换，也可购买或兑换云端额度。',
      remainingCount: '剩余 {count} 次',
      exhausted: '云端额度已用尽，请升级套餐或使用兑换码充值。',
    },
    source: {
      title: 'LaTeX 项目',
      prompt: '选择 LaTeX ZIP 或项目文件夹',
      supported: '支持 .zip，也可直接选择文件夹',
    },
    modeHelp: {
      local: '本地转换不消耗云端额度，文件不会离开浏览器。',
      cloud: '云端转换适合复杂项目，每次消耗 1 次云端额度。',
    },
    accountOverview: {
      newConversion: '开始新转换',
      noExpiry: '长期有效',
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
    // Folder upload
    or: '或',
    selectFolder: '选择文件夹',
    selectFolderHint: '直接选择 LaTeX 项目文件夹，无需先打包',
    folderScanning: '正在扫描文件夹...',
    folderReading: '正在读取...（{current}/{total}）',
    folderPacking: '正在打包...（{current}/{total}）',
    folderDetected: '已识别文件夹内 {count} 个文件',
    folderExcluded: '已排除 {count} 个编译产物',
    folderTruncated: '文件夹过大，仅显示前 {max} 个文件',
    folderTooLarge: '文件夹过大，浏览器内存不足',
    folderNotSupported: '当前浏览器不支持文件夹选择',
    folderMainTexFromFolder: '已从文件夹中识别主文件',
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
    validUntil: '有效期至 {date}',
    renewalHint: '将在 {date} 自动续费',
    expiresInDays: '{days} 天后到期',
    expiresToday: '今日到期',
    renewalWarningTitle: '套餐即将到期',

    // Funnel analytics (P1-2)
    funnel: {
      title: '匿名使用埋点',
      description: '最近 7 天的应用内事件（不含 PII 与文件内容），可导出供客服分析。',
      export: '导出埋点 JSON',
    },

    // Diagnostics (P1-3)
    diagnostics: {
      title: '诊断信息',
      description: '导出最近 200 条事件的脱敏报告，方便附在反馈工单中。',
      export: '导出诊断包',
      exportSuccess: '诊断包已保存。',
      privacyNote: '诊断包不包含 token、源文件、文件内容和邮箱；仅记录事件元数据。',
    },

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
      folderScanFailed: '文件夹扫描失败',
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
