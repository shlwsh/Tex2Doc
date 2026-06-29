# Tex2Doc Flutter Web 主页面集成浏览器插件 - 技术实现方案

> 创建时间：2026-06-29
> 目标：
> 1. 在 Flutter Web 主页面中添加浏览器插件推广模块
> 2. 新增浏览器插件详情页面
> 3. 实现新的浏览器插件模块入口

## 1. 背景与目标

### 1.1 当前现状

Tex2Doc 项目已发布浏览器扩展，覆盖 Chrome / Edge / Firefox / Safari：

- 项目位置：`apps/browser-extension/`
- 技术栈：WXT + React + TypeScript + WebAssembly
- 已发布版本：v0.1.0

但浏览器扩展目前与 Flutter Web 主页相互独立，用户需要：
1. 访问浏览器应用商店搜索安装
2. 用户无法在主页获取插件介绍与安装入口

### 1.2 目标

| 目标 | 说明 |
|------|------|
| **主页面推广模块** | 在 `ProductHomePage` 添加浏览器插件推广板块 |
| **独立详情页面** | 新建 `ExtensionPage` 介绍插件功能、使用与安装 |
| **导航入口** | 用户可在主页跳转到插件详情页面 |
| **多语言支持** | 兼容现有 zh-CN / en-US 双语架构 |

## 2. 整体设计

### 2.1 页面结构变化

```
ProductHomePage (现有)
├── _HomeNav               # 顶部导航（增加插件入口）
├── _HeroCopy              # Hero 区（增加插件 CTA）
├── _FeatureGrid           # 核心能力（保持）
└── _ReleaseBand           # 发布带（增加插件区块）
    ↓ 新增
ExtensionPage             # 浏览器插件独立详情页
├── _ExtensionHero         # 功能介绍区
├── _ExtensionFeatures     # 核心功能列表
├── _ExtensionUsage        # 使用说明
└── _ExtensionInstall      # 安装引导（按浏览器）
```

### 2.2 路由策略

复用现有 Flutter Web 单页架构，使用 **Path-based 路由**：

| 路径 | 页面 | 说明 |
|------|------|------|
| `/` | `ProductHomePage` | 主页（默认） |
| `/extension` | `ExtensionPage` | 浏览器插件详情页 |
| `/user/...` | `UserApp` | 用户端 |
| `/admin/...` | `AdminApp` | 管理端 |

### 2.3 模块位置图

```
┌──────────────────────────────────────────────────────────────┐
│  Home Web (ProductHomePage)                                   │
│                                                               │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  NavBar [Tex2Doc]   [管理端] [用户登录] [🌐 浏览器插件]   │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                               │
│  ┌──────────────┐ ┌──────────────┐                            │
│  │   Hero       │ │  Hero Panel  │                            │
│  │   + 插件链接  │ │              │                            │
│  └──────────────┘ └──────────────┘                            │
│                                                               │
│  核心能力网格                                                   │
│  [云端转换] [本地桌面端] [商业化]                                │
│                                                               │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  ✨ 浏览器插件                                            │ │
│  │  在浏览器里一键把 LaTeX 转 Word，一键转换 Overleaf/arXiv  │ │
│  │  [立即查看 →]                                             │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                               │
└──────────────────────────────────────────────────────────────┘
                                  ↓ 点击跳转
┌──────────────────────────────────────────────────────────────┐
│  Extension Page (/extension)                                  │
│                                                               │
│  ┌──────────────┐ ┌──────────────┐                            │
│  │ Plugin Hero  │ │ Plugin CTA   │                            │
│  └──────────────┘ └──────────────┘                            │
│                                                               │
│  📦 核心功能                                                  │
│  ┌───────┐ ┌───────┐ ┌───────┐                                │
│  │本地转换│ │云端转换│ │Overleaf│                                │
│  └───────┘ └───────┘ └───────┘                                │
│                                                               │
│  🚀 安装指南                                                  │
│  [Chrome 安装] [Edge 安装] [Firefox 安装] [Safari 安装]      │
│                                                               │
│  📖 使用说明                                                  │
│  弹出窗口 / 侧边面板 / 网站集成 / 云端转换                     │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

## 3. 文件改动清单

### 3.1 新增文件

| 文件路径 | 说明 |
|----------|------|
| `flutter_app/lib/product/extension_page.dart` | 浏览器插件详情页面（新模块） |
| `flutter_app/lib/product/extension_promo_card.dart` | 主页推广插件卡片组件 |
| `docs-zh/extension/flutter_web_extension_integration_design.md` | 本方案文档 |

### 3.2 修改文件

| 文件路径 | 修改内容 |
|----------|----------|
| `flutter_app/lib/main.dart` | 增加 `/extension` 路径分发 |
| `flutter_app/lib/product/product_home_app.dart` | 在 NavBar 增加插件入口 |
| `flutter_app/lib/ui/app_i18n.dart` | 增加插件相关文案 |

## 4. 实施步骤

### 4.1 主页面改动（`product_home_app.dart`）

#### 4.1.1 导航栏入口

在 `_HomeNav.actions` 中增加插件入口按钮：

```dart
final actions = Wrap(
  spacing: AppSpacing.xs,
  runSpacing: AppSpacing.xs,
  children: [
    OutlinedButton.icon(
      onPressed: () => _openExtension(context),
      icon: const Icon(Icons.extension_outlined),
      label: const Text('浏览器插件'),
    ),
    TextButton(
      onPressed: () => _openWorkspace(context, DocEngineAppMode.admin),
      child: const Text('管理端'),
    ),
    FilledButton(
      onPressed: () => _openWorkspace(context, DocEngineAppMode.user),
      child: const Text('用户登录'),
    ),
  ],
);
```

#### 4.1.2 主页面新增推广模块

在 `_ReleaseBand` 之后增加 `_ExtensionPromoBand`：

```dart
const SizedBox(height: AppSpacing.xxl),
const _ExtensionPromoBand(),
```

#### 4.1.3 路由跳转逻辑

```dart
void _openExtension(BuildContext context) {
  // Web 平台：使用路径跳转
  if (kIsWeb) {
    // dart:html 的 setLocation 在 web 端可用
    // 使用 uri 跳转保留相对路径
    final path = Uri.base.path;
    // 主页路径固定为 '/'
    // 跳转到 /extension
    // 可使用 AnchorElement 点击实现
  } else {
    // 桌面端：使用 Navigator 跳转
    Navigator.of(context).push(
      MaterialPageRoute<void>(builder: (_) => const ExtensionPage()),
    );
  }
}
```

#### 4.1.4 推广卡片 UI 设计

```dart
class _ExtensionPromoBand extends StatelessWidget {
  const _ExtensionPromoBand();

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.all(AppSpacing.lg),
      decoration: BoxDecoration(
        gradient: LinearGradient(
          colors: [
            theme.colorScheme.secondaryContainer,
            theme.colorScheme.primaryContainer,
          ],
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
        ),
        borderRadius: BorderRadius.circular(AppRadius.md),
      ),
      child: Row(
        children: [
          const Icon(Icons.extension, size: 48),
          const SizedBox(width: AppSpacing.lg),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('Tex2Doc 浏览器插件',
                    style: theme.textTheme.headlineSmall),
                const SizedBox(height: AppSpacing.xs),
                Text(
                  '在 Chrome / Edge / Firefox / Safari 中一键把 LaTeX '
                  '转 Word。一键转换 Overleaf、arXiv 项目。',
                  style: theme.textTheme.bodyMedium,
                ),
              ],
            ),
          ),
          FilledButton.icon(
            onPressed: () => _openExtension(context),
            icon: const Icon(Icons.arrow_forward),
            label: const Text('立即查看'),
          ),
        ],
      ),
    );
  }
}
```

### 4.2 浏览器插件详情页（`extension_page.dart`）

#### 4.2.1 页面入口

**Web 端入口**：`http://82.156.234.59/extension`

#### 4.2.2 页面布局

```dart
class ExtensionPage extends StatelessWidget {
  const ExtensionPage({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Tex2Doc Browser Extension',
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
```

#### 4.2.3 主要区域

**A. Hero 区（顶部介绍）**：
- 标题：Tex2Doc 浏览器插件
- 副标题：LaTeX → DOCX，无处不在
- 主 CTA：Chrome / Edge / Firefox / Safari 安装按钮（4 个）

**B. 核心功能区（Features Grid）**：
| 功能 | 图标 | 说明 |
|------|------|------|
| 本地 WASM 转换 | `memory` | 文件不离开设备 |
| 云端转换 | `cloud_sync` | 上传至 Tex2Doc 引擎 |
| Overleaf 集成 | `web_stories` | 一键转换 Overleaf 项目 |
| arXiv 支持 | `article` | 一键下载并转换 arXiv |
| 账户/配额管理 | `account_circle` | 套餐、配额、兑换 |
| 侧边面板管理 | `space_dashboard` | Chrome/Edge SaaS 风格面板 |

**C. 安装指南区（按浏览器分卡）**：

每张卡包含：

- 浏览器图标
- 商店链接（点击跳转）
- 分步图文安装步骤

**D. 使用指南区**：

- 弹出窗口使用
- 侧边面板使用
- 网站集成（Overleaf / arXiv）
- 账户与配额

#### 4.2.4 安装按钮跳转逻辑

```dart
// 各浏览器商店 URL
const _chromeWebStoreUrl = 'https://chrome.google.com/webstore/detail/tex2doc';
const _edgeAddonsUrl = 'https://microsoftedge.microsoft.com/addons/detail/tex2doc';
const _firefoxAddonsUrl = 'https://addons.mozilla.org/firefox/addon/tex2doc';
const _safariAppStoreUrl = 'https://apps.apple.com/app/tex2doc';

// 点击安装按钮
onPressed: () => openExternalUrl(_chromeWebStoreUrl),
```

### 4.3 路由分发（`main.dart`）

修改 `main.dart` 中的路径分发逻辑：

```dart
void main() {
  final path = Uri.base.path;
  if (kIsWeb) {
    if (path.startsWith('/admin')) {
      runApp(const AdminApp(isWeb: true));
    } else if (path.startsWith('/app')) {
      runApp(UserApp(isWeb: true));
    } else if (path.startsWith('/extension')) {
      runApp(const ExtensionPage());
    } else {
      runApp(const ProductHomeApp());
    }
  } else {
    runApp(UserApp(isWeb: false));
  }
}
```

### 4.4 i18n 文案（`app_i18n.dart`）

新增以下键值（zh-CN + en-US）：

```dart
// zh-CN
'extension.title': 'Tex2Doc 浏览器插件',
'extension.subtitle': '在浏览器里一键把 LaTeX 转为 Word',
'extension.cta': '立即查看',
'extension.install.chrome': 'Chrome 安装',
'extension.install.edge': 'Edge 安装',
'extension.install.firefox': 'Firefox 安装',
'extension.install.safari': 'Safari 安装',
'extension.feature.local.title': '本地转换',
'extension.feature.local.body': '基于 WebAssembly 在浏览器本地转换，文件不上传。',
'extension.feature.cloud.title': '云端转换',
'extension.feature.cloud.body': '上传至 Tex2Doc 云端引擎，支持复杂文档与模板。',
'extension.feature.overleaf.title': 'Overleaf 集成',
'extension.feature.overleaf.body': '在 Overleaf 项目页面右下角一键转换。',
'extension.feature.arxiv.title': 'arXiv 支持',
'extension.feature.arxiv.body': '在 arXiv 摘要页一键下载并转换为 Word。',
'extension.feature.account.title': '账户与配额',
'extension.feature.account.body': '登录后查看配额、订阅升级、兑换码充值。',
'extension.feature.sidepanel.title': '侧边面板',
'extension.feature.sidepanel.body': 'Chrome/Edge 提供 SaaS 风格管理面板（Jobs / Billing / Feedback）。',
'extension.installGuide.title': '安装指南',
'extension.installGuide.chromeSteps': '1. 打开 Chrome Web Store；2. 点击"添加至 Chrome"；3. 在浏览器工具栏固定图标。',
'extension.installGuide.edgeSteps': '1. 打开 Edge Add-ons；2. 点击"获取"；3. 在 Edge 中启用扩展。',
'extension.installGuide.firefoxSteps': '1. 打开 Firefox Add-ons；2. 点击"添加到 Firefox"；3. 允许必要权限。',
'extension.installGuide.safariSteps': '1. 打开 Mac App Store；2. 下载安装 Tex2Doc 应用；3. 在 Safari 偏好设置中启用扩展。',

// en-US
'extension.title': 'Tex2Doc Browser Extension',
'extension.subtitle': 'Convert LaTeX to Word in your browser',
// ... 对应英文翻译
```

## 5. Web 路由与服务端配置

### 5.1 SPA 路由处理

由于 Flutter Web 是 SPA 应用，所有路径需要指向 `index.html`：

当前 Nginx 配置（位于 `/etc/nginx/sites-available/tex2doc`）已通过 `try_files $uri $uri/ /index.html` 处理 SPA fallback。

新增的 `/extension` 路径会自动落到 index.html 处理。

### 5.2 部署影响

- 修改 `flutter_app/lib/` 三个文件
- 重新构建 Home Web：
  ```bash
  .\scripts\release\build-flutter-home.ps1
  ```
- GitHub Actions 自动触发 `deploy-production.yml`
- 服务器 `/opt/tex2doc/current/static/home/index.html` 包含新路由逻辑

## 6. 验证步骤

### 6.1 本地验证

```bash
cd flutter_app
flutter build web --release --target lib/main.dart --base-href /
# 启动本地服务（可选）
flutter run -d chrome
```

### 6.2 本地联调

- 打开 `http://localhost:8080/`
- 验证主页显示浏览器插件推广模块
- 点击"立即查看"按钮跳转到 `/extension`
- 验证详情页面正确显示插件介绍、安装指南、使用说明
- 点击各浏览器安装按钮，验证跳转链接

### 6.3 生产验证

- 部署完成后访问 `http://82.156.234.59/`
- 验证主页和浏览器插件详情页面正常加载
- 验证 4 个浏览器安装按钮可正确跳转

## 7. 风险与注意事项

### 7.1 路径兼容性

- **风险**：老用户可能访问 `/extension` 出现 404
- **缓解**：Nginx try_files 已 fallback 到 index.html

### 7.2 Web 平台差异

- **风险**：web 端和桌面端跳转逻辑不同
- **缓解**：使用 `kIsWeb` 区分行为

### 7.3 商店链接

- **风险**：商店 URL 在扩展尚未发布时不准确
- **缓解**：使用占位 URL，发布后更新 `extension_page.dart` 的常量

### 7.4 浏览器扩展与主页语言一致

- 当前主页使用 `AppLocale.zhCn` 默认值
- 扩展详情页保持一致

## 8. 后续优化

| 优化项 | 说明 |
|--------|------|
| 用户行为埋点 | 跟踪 CTA 点击、停留时间 |
| A/B 测试 | 不同文案转化率对比 |
| 多语言扩展 | 增加 ja / ko 等语言 |
| 插件市场统计 | 显示已有 N 用户安装 |

## 9. 改动文件清单

```
flutter_app/
├── lib/
│   ├── main.dart                              # 修改：增加 /extension 路由
│   ├── product/
│   │   ├── product_home_app.dart              # 修改：增加推广模块 + 导航
│   │   └── extension_page.dart                # 新建：浏览器插件详情页
│   └── ui/
│       └── app_i18n.dart                      # 修改：增加插件相关 i18n

docs-zh/extension/
└── flutter_web_extension_integration_design.md  # 新建：本方案文档
```

## 10. 时间估算

| 步骤 | 预计耗时 |
|------|----------|
| 创建 extension_page.dart | 0.5 人天 |
| 修改 product_home_app.dart | 0.3 人天 |
| 修改 main.dart 路由 | 0.1 人天 |
| i18n 文案补充 | 0.2 人天 |
| 本地联调 + 测试 | 0.3 人天 |
| 部署 + 验证 | 0.1 人天 |
| **合计** | **~1.5 人天** |
