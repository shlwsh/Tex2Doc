# Tex2Doc React 重构 Flutter 用户端与 Web 管理端技术实现方案

> 更新日期：2026-06-29
> 适用范围：`flutter_app` 中 Flutter Web 用户端、Flutter Web 管理端
> 目标：用 React 重构 Web 侧商业化入口和运营后台，提升推广效率、Web 性能、工程迭代速度和后台可维护性

## 1. 背景与目标

当前项目已经具备 Flutter Web 用户端与 Web 管理端：

| 模块 | 当前入口 | 当前能力 |
|---|---|---|
| 用户端 | `/app`、`main_user.dart` | 快捷助手、会员中心、账号、充值、云端转换、转换记录、充值记录、反馈。 |
| 管理端 | `/admin`、`main_admin.dart` | 管理仪表盘、兑换码生成/批次/库存、反馈管理、发布管理、审计中心、自动化研发面板。 |
| 共享能力 | `shared/workspace_app.dart`、`commercial_api.dart` | 认证、API Base URL、主题语言、文件选择、下载、WASM/native 桥接。 |

为商业化推广和性能优化，准备使用 React 重构 Web 用户端和 Web 管理端。核心目标：

1. 降低 Web 首屏包体和加载成本。
2. 提升官网/落地页 SEO、营销内容迭代和 A/B 测试效率。
3. 利用 React 后台生态提升表格、筛选、批量操作、弹窗和表单体验。
4. 保留现有 Rust 服务端、数据库、商业 API、WASM 转换引擎。
5. 使用并行重构和灰度切换，降低对现有 Flutter Web 的破坏性。

## 2. 适合性评估

### 2.1 GitNexus 影响结论

本次评估针对关键 Flutter 符号做了上游影响分析：

| 目标 | 风险 | 影响摘要 | 对 React 重构的含义 |
|---|---:|---|---|
| `DocEngineApp` | MEDIUM | 10 个上游影响，7 个直接依赖 | Web 壳层可替换，但不要直接删除影响桌面和产品首页的共享入口。 |
| `CommercialApiClient` | CRITICAL | 86 个影响，49 个直接依赖，8 条流程 | API 客户端是商业链路核心，必须先冻结接口契约，不能边迁移边改协议。 |
| `QuickAssistantPanel` | LOW | 11 个影响，2 个直接依赖 | 快捷助手可作为用户端迁移试点，但要重点验证额度扣减和 WASM 转换。 |
| `AdminAutomationPanel` | LOW | 10 个影响，2 个直接依赖 | 自动化面板可按独立模块迁移。 |

结论：React 重构 Web 两端是合适的，但应视为 **新 Web 前端并行实现**，而不是对 Flutter 共享壳层做原地大改。

### 2.2 React 的收益

| 诉求 | React 收益 |
|---|---|
| 商业化推广 | React/Next.js 更适合 SEO、落地页、埋点、内容运营、广告投放和 A/B 测试。 |
| 首屏性能 | Vite/React 可做按路由拆包、懒加载、组件级优化；相比 Flutter Web 通常更轻。 |
| 管理后台 | Ant Design/Arco/TanStack 生态成熟，表格、筛选、批量操作、弹窗和表单成本更低。 |
| 招聘与维护 | React/TypeScript 工程师生态更大，后台和营销页迭代速度更快。 |
| API 协作 | TypeScript SDK 与 OpenAPI、Mock、契约测试更自然。 |

### 2.3 React 不能解决的问题

| 问题 | 说明 |
|---|---|
| 核心转换速度 | TeX 到 DOCX 的性能主要取决于 Rust/WASM/native 引擎和服务端队列。 |
| 计费准确性 | 额度扣减、账本、兑换码幂等必须由后端保证。 |
| 大文件转换稳定性 | 需要后端上传、队列、轮询、失败恢复和报告链路配合。 |
| 管理端安全 | 前端路由不能替代后端 admin role 校验。 |

## 3. 总体策略

推荐采用 **React Web 新工程 + Flutter Web 保留回滚** 的策略。

```text
现状:
  /              -> Flutter ProductHomeApp
  /app           -> Flutter UserApp
  /admin         -> Flutter AdminApp

过渡期:
  /react         -> React ProductHome
  /app-react     -> React User App
  /admin-react   -> React Admin App
  /app           -> Flutter UserApp
  /admin         -> Flutter AdminApp

稳定后:
  /              -> React ProductHome
  /app           -> React User App
  /admin         -> React Admin App
  /flutter-app   -> Flutter fallback
  /flutter-admin -> Flutter fallback
```

原则：

1. 先并行，后替换。
2. 先管理端，后用户端快捷转换。
3. 先 API SDK，后 UI 页面。
4. 后端权限、计费、转换流程不跟随前端重构做破坏性变更。
5. 每个模块迁移完成后都要有 E2E 验收。

## 4. 技术选型

### 4.1 推荐版本与依赖

| 分类 | 推荐 | 说明 |
|---|---|---|
| 语言 | TypeScript | 前端模型与 API 响应必须强类型。 |
| 构建 | Vite | 启动快、产物轻，适合 SPA 用户端和管理端。 |
| UI 框架 | React | 主框架。 |
| 路由 | React Router | 支持 `/app/*`、`/admin/*` 子路由。 |
| 请求 | TanStack Query | 管理服务端状态、缓存、轮询、重试和失效刷新。 |
| 全局状态 | Zustand | 管理 auth、theme、locale、quick session。 |
| 表单 | React Hook Form + Zod | 登录、兑换码、发布清单、反馈等表单校验。 |
| 后台组件 | Ant Design 或 Arco Design | 管理端表格、筛选、分页、弹窗效率高。 |
| 样式 | Tailwind CSS + CSS variables | 用户端和营销页灵活；后台可混用组件库 token。 |
| 单测 | Vitest + Testing Library | API client、hooks、组件状态。 |
| E2E | Playwright | 覆盖登录、兑换、转换、管理操作。 |
| Mock | MSW | 本地模拟 `/v1` 与 `/admin/v1`。 |

### 4.2 Vite 与 Next.js 取舍

| 方案 | 优点 | 缺点 | 建议 |
|---|---|---|---|
| Vite SPA | 简单、快、适合后台和工作台、可直接静态托管 | SEO 弱，需要额外处理营销页 | 第一阶段推荐。 |
| Next.js | SEO、SSR/SSG、内容营销更强 | 部署复杂度更高，和 Rust 静态托管需要额外规划 | 当官网/博客/落地页成为重点后引入。 |

建议第一阶段使用 Vite + React 做 `/app-react` 和 `/admin-react`，官网可先做静态 React 页面；后续如果 SEO 和内容增长成为核心，再将官网升级为 Next.js。

## 5. 工程结构

建议新增 `apps/react-web`，避免与 `flutter_app` 互相污染。

```text
apps/react-web/
  package.json
  index.html
  vite.config.ts
  tsconfig.json
  src/
    main.tsx
    app/
      AppRouter.tsx
      providers/
        QueryProvider.tsx
        ThemeProvider.tsx
        I18nProvider.tsx
    api/
      http.ts
      auth.ts
      user.ts
      billing.ts
      redeem.ts
      conversion.ts
      feedback.ts
      admin.ts
      automation.ts
      types.ts
    features/
      auth/
      quick-assistant/
      member-center/
      conversion/
      records/
      feedback/
      admin-dashboard/
      admin-redeem/
      admin-feedback/
      admin-release/
      admin-audit/
      admin-automation/
    components/
      layout/
      form/
      table/
      feedback/
      status/
    wasm/
      docEngine.ts
    stores/
      authStore.ts
      quickStore.ts
      preferenceStore.ts
    styles/
      tokens.css
      globals.css
    tests/
```

路径规划：

| 路径 | 模块 |
|---|---|
| `/` | React 产品首页或跳转页。 |
| `/app` | React 用户端正式路径。 |
| `/admin` | React 管理端正式路径。 |
| `/app-react` | 过渡期 React 用户端灰度路径。 |
| `/admin-react` | 过渡期 React 管理端灰度路径。 |

## 6. API SDK 方案

### 6.1 为什么 API SDK 优先

GitNexus 显示 `CommercialApiClient` 是 CRITICAL 影响点。React 重构的第一步必须是 TypeScript API SDK，而不是页面开发。

目标：

1. 完整覆盖 Flutter `CommercialApiClient` 当前能力。
2. 将 `/v1/*` 和 `/admin/v1/*` 清晰分组。
3. 统一错误模型。
4. 统一 token 注入、401 处理、JSON 解析、文件下载。
5. 为 Vitest 契约测试提供固定入口。

### 6.2 API 分组

```text
api/
  http.ts              # fetch 封装、baseUrl、headers、错误处理
  auth.ts              # login/register/refresh/me/adminMe
  usage.ts             # usage
  redeem.ts            # redeem options/redeem/records/admin redeem
  conversion.ts        # upload/create/get/list/download/local quota
  recharge.ts          # recharges/create/options
  feedback.ts          # user/admin feedback
  release.ts           # admin releases/audit
  automation.ts        # admin automation
  types.ts             # DTO 类型
```

### 6.3 HTTP 封装

```ts
export class ApiError extends Error {
  constructor(
    public status: number,
    public body: unknown,
    message: string,
  ) {
    super(message);
  }
}

export interface ApiClientOptions {
  baseUrl: string;
  accessToken?: string;
}

export async function requestJson<T>(
  path: string,
  options: ApiClientOptions & RequestInit,
): Promise<T> {
  const url = new URL(path, normalizeBaseUrl(options.baseUrl));
  const headers = new Headers(options.headers);
  headers.set("content-type", "application/json");
  if (options.accessToken) {
    headers.set("authorization", `Bearer ${options.accessToken}`);
  }

  const response = await fetch(url, { ...options, headers });
  const text = await response.text();
  const body = text ? JSON.parse(text) : null;
  if (!response.ok) {
    throw new ApiError(response.status, body, extractMessage(body, text));
  }
  return body as T;
}
```

默认 API base URL：

| 环境 | 规则 |
|---|---|
| 浏览器同源部署 | `new URL('/v1/', location.origin)` |
| 本地开发 | `.env` 中 `VITE_API_BASE_URL=http://127.0.0.1:2624/v1/` |
| 管理端 | 用户 API 仍使用 `/v1/`，管理 API 使用同 origin 的 `/admin/v1/` 或由 SDK 统一拼接。 |

### 6.4 DTO 类型

核心 DTO 应与 Flutter 模型一一对齐：

| Flutter 模型 | TypeScript 类型 |
|---|---|
| `AuthResponse` | `AuthResponse` |
| `UserProfile` | `UserProfile` |
| `UsageSummary` | `UsageSummary` |
| `RedeemCodeOptions` | `RedeemCodeOptions` |
| `RedeemCodeRecord` | `RedeemCodeRecord` |
| `RedeemCodeBatch` | `RedeemCodeBatch` |
| `ConversionJob` | `ConversionJob` |
| `ConversionReport` | `ConversionReport` |
| `FeedbackThread` | `FeedbackThread` |
| `FeedbackThreadDetail` | `FeedbackThreadDetail` |
| `AutomationSummary` | `AutomationSummary` |
| `AutomationRequest` | `AutomationRequest` |
| `AutomationAgent` | `AutomationAgent` |

建议后续由 Rust API 生成 OpenAPI，再由 `openapi-typescript` 或 `orval` 生成类型和请求函数。第一阶段可以手写，必须补契约测试。

## 7. 认证与会话设计

### 7.1 会话隔离

用户端、快捷助手和管理端必须使用不同 localStorage key：

| Key | 用途 |
|---|---|
| `tex2doc.user.auth` | 会员中心真实账号 token、用户资料。 |
| `tex2doc.quick.redeemCode` | 快捷助手兑换码。 |
| `tex2doc.quick.session` | 可选，短期缓存快捷助手 token 和 usage。 |
| `tex2doc.admin.auth` | 管理端 token、管理员资料。 |
| `tex2doc.preferences` | 主题、语言、表格密度等偏好。 |

管理端不要复用用户端 token key，避免普通用户切换路径时出现错误状态。

### 7.2 管理端门禁

前端流程：

```text
打开 /admin-react
  -> 读取 tex2doc.admin.auth
  -> 若无 token，显示登录页
  -> 若有 token，调用 /admin/v1/me 校验
  -> role 合法，进入后台
  -> role 非法或 401，清空 admin session
```

注意：前端门禁只改善体验，安全必须由后端 `/admin/v1/*` 统一强制校验。

### 7.3 快捷助手影子账号

React 需要保持 Flutter 当前行为：

```text
输入兑换码 code
  -> login(email=code, password=code)
  -> 若 401/404，register(email=code, password=code, displayName=`Quick ${code}`)
  -> redeemCode(code)
  -> 若登录成功且 redeem 返回 409，可视为恢复场景
  -> usage()
  -> 写入 quick session
```

## 8. 用户端 React 设计

### 8.1 路由

```text
/app-react
  /quick
  /member
    /account
    /recharge
    /convert
    /conversions
    /recharges
    /feedback
    /feedback/:threadId
    /about
```

默认进入 `/app-react/quick`。

### 8.2 页面模块

| 页面 | React feature | 关键能力 |
|---|---|---|
| 快捷助手 | `features/quick-assistant` | 激活兑换码、本地转换、云端转换、日志、额度。 |
| 登录/注册 | `features/auth` | API Base URL、邮箱、密码、注册校验。 |
| 账号 | `features/member-center/account` | 用户资料、套餐、额度、退出登录。 |
| 充值 | `features/member-center/recharge` | 购买链接、兑换码、兑换记录、套餐说明。 |
| 转换 | `features/conversion` | ZIP 上传、主 TeX、云端任务创建、轮询、下载。 |
| 转换记录 | `features/records/conversions` | 任务列表、状态、DOCX/ZIP/LOG 下载。 |
| 充值记录 | `features/records/recharges` | 充值列表。 |
| 反馈 | `features/feedback` | 线程列表、创建、详情、回复。 |

### 8.3 快捷助手本地转换流程

React 必须严格保持额度链路：

```text
checkLocalConversion
  -> allowed=false: 不转换
  -> allowed=true: wasmConvertZipToDocx
  -> consumeLocalConversion
  -> consumed=true: downloadDocx
  -> usage refresh
```

不能提前下载 DOCX，也不能在转换失败时扣减额度。

### 8.4 WASM 集成

现有构建脚本：

```json
"build:wasm": "wasm-pack build crates/wasm --target web --out-dir ../flutter_app/wasm/pkg --out-name doc_engine --dev"
```

React 方案建议新增产物目录：

```powershell
wasm-pack build crates/wasm --target web --out-dir ../../apps/react-web/src/wasm/pkg --out-name doc_engine --dev
```

封装：

```ts
let wasmReady: Promise<typeof import("./pkg/doc_engine")> | null = null;

export function ensureDocEngine() {
  wasmReady ??= import("./pkg/doc_engine").then(async (mod) => {
    await mod.default();
    return mod;
  });
  return wasmReady;
}

export async function convertZipToDocx(zip: Uint8Array, mainTex: string) {
  const mod = await ensureDocEngine();
  return mod.convert_zip_to_docx(zip, mainTex);
}
```

如果 wasm-bindgen 导出 API 名称不同，以实际 `pkg` 产物为准。

## 9. 管理端 React 设计

### 9.1 路由

```text
/admin-react
  /login
  /dashboard
  /account
  /redeem/batches/new
  /redeem/batches
  /redeem/codes
  /feedback
  /releases
  /audit
  /automation
    /requests
    /agents
    /history
  /about
```

### 9.2 布局

建议后台布局：

1. 左侧固定 Sidebar。
2. 顶部 Header：环境、API 状态、刷新、主题、语言、管理员头像。
3. 主内容区采用页面级标题 + 操作栏 + 数据区。
4. 表格页统一支持筛选、搜索、分页、导出、批量操作。

### 9.3 管理模块映射

| Flutter 模块 | React 模块 | 迁移要点 |
|---|---|---|
| `AdminDashboardPanel` | `admin-dashboard` | 摘要卡、模块列表、刷新。 |
| `AdminRedeemManagePanel` | `admin-redeem/batch-create` | 套餐、数量、渠道、过期时间、备注、生成、导出。 |
| `AdminRedeemRecordsPanel` | `admin-redeem/batches` | 批次表格、详情 Drawer、导出。 |
| `AdminRedeemCodesPanel` | `admin-redeem/codes` | 状态筛选、搜索、分页、批量上货、导入重置、导出。 |
| `AdminFeedbackPanel` | `admin-feedback` | 状态下拉、回复弹窗、线程详情。 |
| `AdminReleasesPanel` | `admin-release` | 发布表单、清单列表、回滚确认。 |
| `AdminAuditPanel` | `admin-audit` | 审计列表、刷新。 |
| `AdminAutomationPanel` | `admin-automation` | 摘要卡、请求筛选、详情弹窗、审批/驳回/重试/升级、Agent 暂停/恢复。 |

### 9.4 表格规范

所有管理端表格应统一：

1. 后端分页优先，不在前端全量加载。
2. 筛选条件进入 URL query，便于复制和刷新恢复。
3. 默认页大小 50，可选 20、50、100、200。
4. 导出使用当前筛选条件。
5. 批量操作必须有确认弹窗。
6. 高风险操作使用二次确认，必要时要求输入原因。

## 10. 性能优化方案

### 10.1 首屏性能

目标：

| 指标 | 目标 |
|---|---:|
| 首页 JS 首包 gzip | < 200 KB |
| 用户端工作台首包 gzip | < 350 KB |
| 管理端首包 gzip | < 450 KB |
| FCP | < 1.5s |
| LCP | < 2.5s |
| 路由切换 | < 200ms |

措施：

1. 路由级懒加载：用户端、管理端、自动化、WASM 独立 chunk。
2. 管理端图表/表格组件按页加载。
3. WASM 只在快捷助手本地转换模式初始化。
4. 产品首页不加载管理端代码。
5. 生产构建启用 gzip/brotli 和 CDN 缓存。
6. 图片使用 WebP/AVIF，首屏图片预加载。

### 10.2 数据性能

1. TanStack Query 设置合理 stale time。
2. 列表页使用分页、筛选和搜索参数。
3. 轮询仅用于转换任务和自动化请求详情，不全局高频刷新。
4. 管理端导出走文件流或下载接口，不把大文件放入 React 状态。
5. 转换日志只保留最近 N 条，避免长任务撑爆状态。

### 10.3 WASM 性能

1. 懒加载 WASM。
2. 大文件转换放入 Web Worker，避免阻塞主线程。
3. ZIP 文件继续保持 10 MB 前端限制。
4. 后续可考虑 streaming 或 worker pool。

第一阶段可以先主线程集成，若 Playwright/手动验收发现 UI 卡顿，再引入 Worker。

## 11. 安全设计

| 风险 | 方案 |
|---|---|
| 管理端前端代码暴露 | 后端强校验 admin role；管理端可单独构建和部署，避免普通用户入口加载后台代码。 |
| Token 泄漏 | localStorage 仅保存短期 access token；后续可改 HttpOnly Cookie。 |
| 用户/admin session 混用 | 使用独立 storage key 和独立 auth store。 |
| CSRF | Bearer token 模式风险较低；若改 Cookie 需加 CSRF token。 |
| XSS | 禁止直接渲染后端 HTML；反馈内容按纯文本显示。 |
| 兑换码明文暴露 | 管理端表格只展示 code preview；导出接口权限控制。 |
| 高风险自动化请求误审批 | 前端隐藏 high/critical 自动批准，后端也应拒绝自动批准。 |

## 12. 国际化与主题

### 12.1 国际化

建议使用 `i18next` 或轻量自研字典：

```text
src/i18n/
  zh-CN.ts
  en-US.ts
  index.ts
```

第一阶段至少覆盖：

1. 导航。
2. 登录注册。
3. 快捷助手。
4. 充值兑换。
5. 转换状态。
6. 管理端表格和操作按钮。

### 12.2 主题

使用 CSS variables：

```css
:root {
  --color-primary: #2563eb;
  --color-success: #16a34a;
  --color-warning: #d97706;
  --color-danger: #dc2626;
  --radius-md: 8px;
  --spacing-md: 16px;
}

[data-theme="dark"] {
  --color-bg: #0f172a;
  --color-text: #e5e7eb;
}
```

后台组件库 token 与项目 CSS variables 做映射，避免一套 UI 多套颜色逻辑。

## 13. 构建与部署

### 13.1 package scripts

根 `package.json` 可新增：

```json
{
  "scripts": {
    "react:install": "cd apps/react-web && npm install",
    "react:dev": "cd apps/react-web && npm run dev",
    "react:build": "cd apps/react-web && npm run build",
    "react:test": "cd apps/react-web && npm run test",
    "react:e2e": "cd apps/react-web && npm run e2e"
  }
}
```

### 13.2 静态产物

建议构建输出：

```text
apps/react-web/dist/
  index.html
  assets/
```

部署到 Rust 服务端静态目录：

```text
apps/rust-service/static/react/
apps/rust-service/static/app/
apps/rust-service/static/admin/
```

过渡期可以：

| 路径 | 产物 |
|---|---|
| `/app` | Flutter 用户端 |
| `/admin` | Flutter 管理端 |
| `/app-react` | React 用户端 |
| `/admin-react` | React 管理端 |

稳定后切换：

| 路径 | 产物 |
|---|---|
| `/app` | React 用户端 |
| `/admin` | React 管理端 |
| `/flutter-app` | Flutter 用户端回滚入口 |
| `/flutter-admin` | Flutter 管理端回滚入口 |

## 14. 测试方案

### 14.1 单元测试

| 范围 | 测试 |
|---|---|
| API SDK | URL 拼接、Bearer token、错误解析、文件下载、DTO 解析。 |
| Auth Store | 登录、退出、session 恢复、admin/user key 隔离。 |
| 快捷助手 | 影子账号登录/注册、409 恢复、状态流转。 |
| 本地转换 | check 不允许时不转；转换失败不 consume；consume 成功才下载。 |
| 管理端操作 | 批量上货确认、发布必填校验、高风险自动化不显示 Approve。 |

### 14.2 E2E 测试

用户端：

1. 打开 `/app-react` 默认进入快捷助手。
2. 输入兑换码，激活成功。
3. 选择 ZIP，快捷版转换成功并下载 DOCX。
4. 专业版创建云端任务并轮询完成。
5. 登录会员中心，查看账号、充值、转换记录。
6. 创建反馈并回复。

管理端：

1. 普通用户登录 `/admin-react` 被拒绝。
2. 管理员登录成功。
3. 创建兑换码批次并导出 Excel。
4. 兑换码库存筛选、搜索、分页、批量上货。
5. 反馈状态修改和回复。
6. 发布清单、回滚、审计记录出现。
7. 自动化请求详情、审批/驳回/重试/升级。
8. Agent 暂停和恢复。

### 14.3 对比验收

迁移期间，React 与 Flutter 需要跑同一业务验收表：

| 功能 | Flutter | React | 是否一致 |
|---|---|---|---|
| 登录/注册 | 通过 | 通过 | 是 |
| 快捷激活 | 通过 | 通过 | 是 |
| 本地转换扣费 | 通过 | 通过 | 是 |
| 云端转换 | 通过 | 通过 | 是 |
| 充值兑换 | 通过 | 通过 | 是 |
| 反馈会话 | 通过 | 通过 | 是 |
| 管理端兑换码 | 通过 | 通过 | 是 |
| 管理端发布/审计 | 通过 | 通过 | 是 |

## 15. 分阶段实施计划

### Phase 0：接口冻结与基础设施

周期：2-3 天

1. 整理 `/v1` 和 `/admin/v1` API 清单。
2. 建立 TypeScript DTO 与 API SDK。
3. 建立 React Vite 工程。
4. 接入 ESLint、Prettier、Vitest、Playwright。
5. 完成 base layout、主题、i18n、auth store。

交付物：

1. `apps/react-web` 可启动。
2. API SDK 单测通过。
3. `/app-react` 和 `/admin-react` 空壳可访问。

### Phase 1：管理端优先迁移

周期：1-2 周

1. 管理端登录门禁。
2. Dashboard。
3. 兑换码生成、批次、库存。
4. 反馈管理。
5. 发布管理和审计。
6. 自动化面板。

验收：

1. 管理员完整操作闭环可用。
2. 普通用户不能进入后台。
3. 表格筛选、分页、导出可用。

### Phase 2：用户会员中心迁移

周期：1 周

1. 登录/注册。
2. 账号页。
3. 充值页。
4. 云端转换页。
5. 转换记录和充值记录。
6. 用户反馈。

验收：

1. 会员中心可替代 Flutter Web。
2. 云端转换和记录一致。
3. 反馈会话可用。

### Phase 3：快捷助手与 WASM

周期：1 周

1. 快捷助手激活流程。
2. 本地 WASM 转换。
3. 本地额度 check/consume。
4. 专业版云端转换。
5. 日志、下载、错误处理。

验收：

1. 未激活不能转换。
2. 额度不足不转换。
3. 转换失败不扣费。
4. consume 成功后才下载 DOCX。
5. Web Worker 优化按需引入。

### Phase 4：灰度上线与替换

周期：3-5 天

1. `/admin-react` 内测。
2. `/app-react` 内测。
3. 小流量切 `/admin`。
4. 小流量切 `/app`。
5. 保留 Flutter 回滚路径。
6. 更新部署文档和用户手册。

## 16. 目录与文件变更建议

新增：

```text
apps/react-web/
docs-zh/react/
scripts/build_react_static_release.ps1
```

调整：

```text
package.json
apps/rust-service/static/
apps/rust-service static router 配置
```

暂不删除：

```text
flutter_app/
scripts/build_flutter_static_release.ps1
docs-zh/flutter/
```

## 17. 风险清单

| 风险 | 等级 | 缓解 |
|---|---:|---|
| API SDK 与后端响应不一致 | 高 | 契约测试、MSW mock、E2E 联调。 |
| 管理端权限绕过 | 高 | 后端强校验，前端只做体验门禁。 |
| 本地转换扣费顺序错误 | 高 | 单测和 E2E 强制覆盖 check/convert/consume/download 顺序。 |
| React 首包过大 | 中 | 路由拆包、WASM 懒加载、后台模块懒加载。 |
| Flutter 与 React 双维护成本 | 中 | 限定过渡期，功能冻结 Flutter Web，仅修 bug。 |
| 表格大数据卡顿 | 中 | 后端分页、虚拟滚动、导出走接口。 |
| 浏览器兼容 | 中 | Chrome/Edge/Firefox/Safari smoke test。 |
| SEO 未改善 | 低中 | 若营销页成为主战场，升级官网到 Next.js。 |

## 18. 验收标准

React 重构完成需满足：

1. `/app` 用户端 React 版本覆盖快捷助手和会员中心全部核心能力。
2. `/admin` 管理端 React 版本覆盖运营后台全部核心能力。
3. Flutter Web 可作为回滚入口保留至少一个发布周期。
4. React 首屏性能优于当前 Flutter Web。
5. 用户端、管理端 E2E 通过。
6. 管理端普通用户登录被拒绝，admin API 后端权限测试通过。
7. 本地转换额度扣减链路无回归。
8. 发布脚本可将 React 产物部署到 Rust 服务端静态目录。

## 19. 最终建议

建议立项，但不要以“替换 Flutter”为第一目标，而是以“Web 商业化前端升级”为目标：

1. React 承接 Web 用户端、Web 管理端和营销页。
2. Flutter/Slint 继续承担桌面端或历史回滚。
3. Rust 后端和转换引擎保持稳定。
4. 先做 API SDK 和管理端，再做用户端，最后做快捷助手 WASM。

这样收益最大，风险最可控。
