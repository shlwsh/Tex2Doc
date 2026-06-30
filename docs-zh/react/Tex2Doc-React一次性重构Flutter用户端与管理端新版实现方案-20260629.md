# Tex2Doc React 一次性重构 Flutter 用户端与管理端新版实现方案

> 更新日期：2026-06-29
> 适用范围：当前尚未上线阶段，将 Flutter Web 用户端和 Web 管理端整体替换为 React 版本
> 核心策略：Flutter 模块完整保留作为备份；React 一次性实现完整功能闭环并接管 `/app` 与 `/admin`

## 1. 背景变化

上一版方案按“并行灰度迁移”设计，假设线上已有 Flutter Web 用户需要平滑迁移。现在约束发生变化：

1. 项目尚未正式上线，没有存量线上用户迁移压力。
2. Flutter 相关模块可以不动，完整保留为备份和对照验收基线。
3. React 版本可以一次性实现用户端与管理端全部功能，然后统一切换入口。
4. 浏览器插件已经采用 WXT + React + TypeScript，项目内已经具备 React 技术栈基础。

因此，新版方案从“逐步灰度替换”调整为：

```text
开发期:
  Flutter Web 保留不动
  React Web 完整开发 /app-react 与 /admin-react

验收期:
  Flutter 与 React 跑同一业务验收表
  React 通过后切换正式路径

上线期:
  /app   -> React 用户端
  /admin -> React 管理端
  Flutter 构建产物保留为备份，不继续扩展 Web 新功能
```

## 2. 总体结论

建议选用 **React + TypeScript + Vite** 完成 Web 端一次性重构。

不建议本次改用 Vue，原因不是 Vue 不适合，而是当前 Tex2Doc 已经存在 React 工程基础：

1. `apps/browser-extension` 已使用 `React 18.3.1`、`TypeScript`、`WXT`、`Tailwind CSS`、`Zustand`、`Vitest`。
2. 浏览器插件和 Web 用户端都会复用登录、兑换码、转换任务、反馈、WASM、本地文件处理等模型。
3. 继续使用 React 可以统一 TypeScript DTO、API SDK、状态模型、测试工具和 UI 组件思路。
4. 如果新增 Vue，会形成 Flutter + React 插件 + Vue Web 三套前端技术栈，长期维护更分散。

一句话：**React 更贴合当前项目技术惯性，Vue 技术上可行但不如 React 收敛。**

## 3. React 与 Vue 选型评估

| 维度 | React | Vue | 本项目判断 |
|---|---|---|---|
| 当前项目匹配度 | 浏览器插件已使用 React/TS | 当前仓库无 Vue 主工程 | React 优势明显。 |
| 商业化官网 | React + Next.js 生态成熟 | Vue + Nuxt 也成熟 | 两者都可行。 |
| 管理后台 | Ant Design、Arco、TanStack Table 成熟 | Element Plus、Naive UI 成熟 | 两者都可行，React 更贴合插件栈。 |
| TypeScript 复杂模型 | 非常适合 API SDK、hooks、DTO | 也适合，但组合式写法需团队统一规范 | React 更利于和插件共享模型。 |
| WASM/Worker 集成 | 插件已有 WXT/Vite/WASM 经验 | 技术上可行，但项目经验少 | React 更稳。 |
| 团队招聘 | 前端工程师生态大 | 国内 Vue 也好招 | 持平。 |
| 工程统一性 | 可与插件统一 React 技术栈 | 会引入新 UI 技术栈 | React 更好。 |
| 性能 | 与 Vue 接近，关键看拆包和数据策略 | 与 React 接近 | 性能不是主要差异。 |

最终选择：

```text
React + TypeScript + Vite
```

若未来官网 SEO、博客、内容营销成为核心，可将官网部分升级为 Next.js；用户端和管理端仍可继续用 Vite SPA。

## 4. 新版实施原则

1. Flutter 不删除、不改造、不继续追加新 Web 功能。
2. React 一次性覆盖 Web 用户端和 Web 管理端全部核心能力。
3. Rust 后端、数据库、REST API、WASM/native 转换引擎保持稳定。
4. 所有商业链路先以 TypeScript API SDK 固化契约。
5. React 版本通过完整 E2E 后再切换 `/app` 与 `/admin`。
6. Flutter 作为备份和行为对照，至少保留一个大版本周期。

## 5. 目标功能范围

### 5.1 用户端一次性覆盖范围

| 模块 | 必须实现能力 |
|---|---|
| 快捷助手 | 兑换码激活、影子账号登录/注册、额度恢复、本地 WASM 转换、云端专业转换、日志、DOCX 下载。 |
| 会员中心 | 登录、注册、退出、账号信息、用量摘要、主题和语言偏好。 |
| 充值 | 购买链接、兑换码输入、兑换结果、兑换记录、套餐说明。 |
| 云端转换 | ZIP 上传、主 TeX 输入、任务创建、轮询、成功下载、失败展示。 |
| 转换记录 | 任务列表、状态、DOCX/ZIP/LOG 下载入口。 |
| 充值记录 | 充值列表、金额、渠道、状态、创建时间。 |
| 反馈 | 反馈列表、创建问题/需求、关联转换任务、详情聊天、继续回复。 |
| 关于 | 产品介绍、能力说明、公司信息。 |

### 5.2 管理端一次性覆盖范围

| 模块 | 必须实现能力 |
|---|---|
| 登录门禁 | 管理员登录、`/admin/v1/me` 校验、普通用户拒绝。 |
| 仪表盘 | 兑换码批次、待处理反馈、套餐数量、发布通道、模块列表。 |
| 账号 | 管理员资料和用量摘要。 |
| 兑换码生成 | 套餐、数量、渠道、过期时间、备注、生成、Excel 导出。 |
| 兑换码批次 | 批次列表、详情、预览码、导出。 |
| 兑换码库存 | 状态筛选、搜索、分页、勾选、批量上货、导入重置、导出 Excel。 |
| 反馈管理 | 线程列表、状态调整、管理员回复。 |
| 发布管理 | 发布清单表单、必填校验、列表、回滚。 |
| 审计中心 | 发布/回滚/策略变更日志。 |
| 自动化研发 | 摘要卡、请求筛选、详情、审批、驳回、升级、重试、Agent 暂停/恢复。 |

## 6. 推荐技术栈

| 分类 | 推荐 |
|---|---|
| 工程 | `apps/react-web` 独立 Vite 工程 |
| 语言 | TypeScript |
| 框架 | React |
| 构建 | Vite |
| 路由 | React Router |
| 请求缓存 | TanStack Query |
| 状态 | Zustand |
| 表单 | React Hook Form + Zod |
| 管理端 UI | Ant Design 或 Arco Design |
| 用户端/营销页样式 | Tailwind CSS + CSS variables |
| 测试 | Vitest + React Testing Library + Playwright |
| Mock | MSW |
| API 类型 | 手写 TS DTO 起步，后续转 OpenAPI 生成 |

管理端 UI 组件库建议优先选 **Ant Design** 或 **Arco Design**。如果希望更商业后台、表格筛选和批量操作开箱即用，Ant Design 更稳；如果希望视觉更轻、更现代，Arco 也可以。

## 7. 工程目录

```text
apps/react-web/
  package.json
  index.html
  vite.config.ts
  tsconfig.json
  src/
    main.tsx
    app/
      App.tsx
      routes.tsx
      providers/
    api/
      http.ts
      auth.ts
      usage.ts
      redeem.ts
      recharge.ts
      conversion.ts
      feedback.ts
      admin-dashboard.ts
      admin-redeem.ts
      admin-feedback.ts
      admin-release.ts
      admin-audit.ts
      admin-automation.ts
      types.ts
    features/
      home/
      auth/
      quick-assistant/
      member/
      conversion/
      records/
      feedback/
      admin/
        dashboard/
        redeem/
        feedback/
        release/
        audit/
        automation/
    components/
      layout/
      form/
      table/
      status/
      dialog/
    stores/
      auth-store.ts
      admin-auth-store.ts
      quick-store.ts
      preference-store.ts
    wasm/
      doc-engine.ts
      worker/
    i18n/
      zh-CN.ts
      en-US.ts
    styles/
      globals.css
      tokens.css
```

## 8. 路由设计

开发与验收期：

```text
/react
/app-react
/admin-react
```

正式切换后：

```text
/          -> React 产品首页
/app       -> React 用户端
/admin     -> React 管理端
```

Flutter 备份入口建议保留为构建产物或隐藏路径：

```text
/flutter-app
/flutter-admin
```

如果不希望暴露备份路径，也可以只在发布包中保留 Flutter 构建产物，作为紧急回滚时重新映射静态目录使用。

## 9. API SDK 设计

React 重构必须先建立 TypeScript API SDK，等价替换 Flutter `CommercialApiClient`。

### 9.1 用户端 API

| 分组 | 方法 |
|---|---|
| Auth | `login`、`register`、`refresh`、`me` |
| Usage | `usage` |
| Recharge | `rechargeOptions`、`createRecharge`、`recharges` |
| Redeem | `redeemCodeOptions`、`redeemCode`、`redeemCodeRecords` |
| Conversion | `uploadProjectZip`、`createConversion`、`getConversion`、`conversions`、`downloadConversionDocx`、`downloadConversionZip`、`downloadConversionLog` |
| Local quota | `checkLocalConversion`、`consumeLocalConversion` |
| Feedback | `feedbackThreads`、`feedbackThread`、`createFeedbackThread`、`addFeedbackMessage` |

### 9.2 管理端 API

| 分组 | 方法 |
|---|---|
| Admin auth | `adminMe` |
| Dashboard | `adminDashboard` |
| Redeem | `createRedeemCodeBatch`、`redeemCodeBatches`、`redeemCodeBatchDetail`、`exportRedeemCodeBatch`、`adminListRedeemCodes`、`adminBulkStockRedeemCodes`、`adminRestockRedeemCodes`、`adminExportRedeemCodesExcel` |
| Feedback | `adminFeedbackThreads`、`adminUpdateFeedbackThread`、`adminReplyFeedbackThread` |
| Release | `adminReleases`、`adminPublishRelease`、`adminRollbackRelease` |
| Audit | `adminReleaseAudit` |
| Automation | `adminAutomationSummary`、`adminAutomationRequests`、`adminAutomationRequest`、`adminAutomationEvents`、`adminAutomationApprove`、`adminAutomationReject`、`adminAutomationRetry`、`adminAutomationEscalate`、`adminAutomationAgents`、`adminAutomationPauseAgent`、`adminAutomationResumeAgent` |

### 9.3 错误模型

统一错误类型：

```ts
export class ApiError extends Error {
  constructor(
    public status: number,
    public payload: unknown,
    message: string,
  ) {
    super(message);
  }
}
```

错误展示规则：

1. 401：清空对应 session，回登录页。
2. 403：显示权限不足，不重试。
3. 409：兑换码已使用等冲突场景按业务处理。
4. 5xx：显示服务异常，允许重试。
5. 网络失败：显示连接失败和 API Base URL。

## 10. 会话与权限

必须隔离三类会话：

| Key | 用途 |
|---|---|
| `tex2doc.user.auth` | 会员中心真实用户。 |
| `tex2doc.quick.redeemCode` | 快捷助手恢复激活。 |
| `tex2doc.admin.auth` | 管理端管理员会话。 |

管理端启动流程：

```text
读取 tex2doc.admin.auth
  -> 无 token：显示登录
  -> 有 token：调用 /admin/v1/me
  -> 成功且 role=admin：进入后台
  -> 失败：清空 admin auth
```

注意：前端门禁只负责体验，所有 `/admin/v1/*` 接口必须由后端校验管理员角色。

## 11. 快捷助手关键流程

### 11.1 激活流程

```text
输入兑换码 code
  -> login(email=code, password=code)
  -> 401/404 时 register(email=code, password=code)
  -> redeemCode(code)
  -> 若 login 成功且 redeem 返回 409，按恢复成功处理
  -> usage()
  -> 写入 quick session
```

### 11.2 本地转换扣费流程

必须保持：

```text
checkLocalConversion
  -> allowed=false：阻止转换
  -> allowed=true：WASM convertZipToDocx
  -> consumeLocalConversion
  -> consumed=true：触发 DOCX 下载
  -> usage 刷新
```

禁止：

1. 转换前扣费。
2. 转换失败后扣费。
3. consume 失败仍下载 DOCX。
4. 未激活时允许上传转换。

## 12. WASM 集成

React Web 复用 Rust `wasm-pack` 产物。建议新增脚本：

```json
{
  "scripts": {
    "build:wasm:react": "wasm-pack build crates/wasm --target web --out-dir ../../apps/react-web/src/wasm/pkg --out-name doc_engine --dev"
  }
}
```

首版可以主线程加载 WASM；如果转换期间 UI 明显卡顿，再引入 Web Worker。

WASM 模块必须懒加载：

1. 打开首页不加载。
2. 打开管理端不加载。
3. 用户端进入快捷助手也不立即加载。
4. 用户选择本地转换并点击转换时再加载。

## 13. 一次性替换实施计划

### Phase 0：冻结接口和工程准备

周期：2-3 天

1. 建立 `apps/react-web`。
2. 建立 TypeScript API SDK。
3. 建立 DTO 类型。
4. 建立主题、i18n、路由、布局、状态管理。
5. 建立 Vitest、MSW、Playwright。

验收：

1. React 工程可启动。
2. API SDK 单测通过。
3. `/app-react`、`/admin-react` 空壳可访问。

### Phase 1：管理端完整实现

周期：1-2 周

1. 管理端登录和权限校验。
2. Dashboard。
3. 兑换码生成、批次、库存。
4. 反馈管理。
5. 发布管理。
6. 审计中心。
7. 自动化研发面板。

验收：

1. 管理员全链路可操作。
2. 普通用户无法进入。
3. 表格筛选、分页、导出、批量操作可用。

### Phase 2：用户端会员中心完整实现

周期：1 周

1. 登录/注册。
2. 账号和用量。
3. 充值兑换。
4. 云端转换。
5. 转换记录。
6. 充值记录。
7. 反馈会话。
8. 关于页。

验收：

1. 会员中心核心功能完整。
2. 云端转换可上传、轮询、下载。
3. 反馈可创建、查看、回复。

### Phase 3：快捷助手完整实现

周期：1 周

1. 兑换码激活。
2. 影子账号恢复。
3. 本地 WASM 转换。
4. 本地转换额度 check/consume。
5. 专业版云端转换。
6. 日志和下载。

验收：

1. 有效兑换码可激活。
2. 重启或刷新可恢复激活状态。
3. 额度不足不转换。
4. 转换失败不扣费。
5. consume 成功后才下载。

### Phase 4：统一验收与正式切换

周期：3-5 天

1. React 和 Flutter 跑同一验收表。
2. React 用户端 E2E 通过。
3. React 管理端 E2E 通过。
4. 性能指标优于 Flutter Web。
5. 将 `/app` 和 `/admin` 切换到 React。
6. Flutter 产物保留为备份。

## 14. 构建与部署

建议新增：

```text
scripts/build_react_static_release.ps1
```

输出：

```text
apps/rust-service/static/home
apps/rust-service/static/app
apps/rust-service/static/admin
```

Flutter 备份输出可以保留：

```text
apps/rust-service/static/flutter-app
apps/rust-service/static/flutter-admin
```

根脚本建议：

```json
{
  "scripts": {
    "react:dev": "cd apps/react-web && npm run dev",
    "react:build": "cd apps/react-web && npm run build",
    "react:test": "cd apps/react-web && npm run test",
    "react:e2e": "cd apps/react-web && npm run e2e",
    "build:web:react": "npm run build:wasm:react && npm run react:build"
  }
}
```

## 15. 性能目标

| 指标 | 目标 |
|---|---:|
| 产品首页 JS gzip | < 200 KB |
| 用户端首包 JS gzip | < 350 KB |
| 管理端首包 JS gzip | < 450 KB |
| FCP | < 1.5s |
| LCP | < 2.5s |
| 管理端表格筛选响应 | < 200ms |
| 路由切换 | < 200ms |

措施：

1. 路由级懒加载。
2. 管理端模块分包。
3. WASM 懒加载。
4. 文件下载不进入 React 大状态。
5. 表格后端分页。
6. 静态资源 gzip/brotli。

## 16. 测试矩阵

| 类型 | 覆盖 |
|---|---|
| 单元测试 | API SDK、auth store、quick store、表单校验、状态映射。 |
| 组件测试 | 登录、兑换码表单、转换状态、管理端表格操作。 |
| E2E 用户端 | 快捷激活、本地转换、云端转换、充值、记录、反馈。 |
| E2E 管理端 | 登录门禁、兑换码、反馈、发布、审计、自动化。 |
| 性能测试 | 首包大小、FCP/LCP、WASM 懒加载、表格交互。 |
| 对照验收 | Flutter 与 React 同流程输出一致。 |

## 17. 风险与处理

| 风险 | 等级 | 处理 |
|---|---:|---|
| React 一次性实现范围大 | 高 | 内部按 Phase 开发，但上线统一切换。 |
| API SDK 漏实现 | 高 | 以 Flutter `CommercialApiClient` 为清单，逐项对齐。 |
| 本地转换扣费顺序错误 | 高 | 单测和 E2E 强约束。 |
| 管理端权限误判 | 高 | 后端强校验，前端只做门禁体验。 |
| WASM 集成卡顿 | 中 | 懒加载，必要时 Worker 化。 |
| React/Vue 争议影响决策 | 中 | 当前明确 React，避免多技术栈分裂。 |
| Flutter 备份过期 | 中 | 备份期只做 bug 修复，不新增 Web 功能。 |

## 18. 验收标准

正式切换 `/app` 和 `/admin` 前必须满足：

1. React 用户端覆盖 Flutter 用户端全部核心功能。
2. React 管理端覆盖 Flutter 管理端全部核心功能。
3. TypeScript API SDK 覆盖 Flutter `CommercialApiClient` 全部使用中的接口。
4. 本地转换扣费链路无回归。
5. 普通用户无法进入管理端。
6. 管理端导出、批量上货、发布回滚和自动化操作可用。
7. E2E 测试通过。
8. 首屏性能优于 Flutter Web。
9. Flutter 备份构建产物可恢复。

## 19. 最终建议

当前项目更适合直接采用：

```text
React + TypeScript + Vite
```

并采用：

```text
React 一次性完整实现 Web 替换版本
Flutter 保留备份
Rust 后端和 WASM 引擎保持稳定
```

不建议切换 Vue，除非团队已经明确以 Vue 为主力。以当前仓库现状看，React 能与浏览器插件、TypeScript API SDK、WASM 集成和测试体系形成更好的技术收敛。
