# Tex2Doc React 重构文档索引

> 更新日期：2026-06-29
> 适用范围：Flutter Web 用户端与 Web 管理端的 React 重构评估和实施
> 相关现状文档：`docs-zh/flutter/README.md`

## 1. 结论

基于当前项目尚未上线、Flutter 模块可完整保留作为备份、浏览器插件已采用 React 技术栈，建议采用 **React + TypeScript + Vite 一次性完整实现 Web 用户端和 Web 管理端替换版本**：

1. React 适合作为 Web 商业化入口、用户工作台和运营后台。
2. Rust 后端、REST API、WASM/native 转换引擎不应重写。
3. Flutter 相关模块不改动、不删除，作为备份、对照验收和紧急回滚来源。
4. React 版本内部仍按模块分阶段开发，但上线切换可以一次性替换 `/app` 与 `/admin`。
5. 在 React 与 Vue 之间，本项目更推荐 React；主要原因是 `apps/browser-extension` 已经使用 WXT + React + TypeScript + Tailwind + Zustand + Vitest。

## 2. 文档清单

| 文档 | 内容 |
|---|---|
| [Tex2Doc-React商业化推广界面级重构设计实现方案-20260629.md](./Tex2Doc-React商业化推广界面级重构设计实现方案-20260629.md) | 界面级商业化重构方案：现状审计、产品定位、信息架构、设计 token、组件体系、用户端和管理端页面改造、i18n、响应式、验收标准。 |
| [Tex2Doc-React一次性重构Flutter用户端与管理端新版实现方案-20260629.md](./Tex2Doc-React一次性重构Flutter用户端与管理端新版实现方案-20260629.md) | 新版方案：未上线场景下 React 一次性完整替换 Web 用户端和管理端，Flutter 保留备份，并补充 React vs Vue 选型评估。 |
| [Tex2Doc-React重构Flutter用户端与管理端技术实现方案-20260629.md](./Tex2Doc-React重构Flutter用户端与管理端技术实现方案-20260629.md) | 完整技术方案：选型、架构、目录、API SDK、模块拆分、迁移步骤、性能、安全、测试、灰度发布。 |

## 3. 推荐技术栈

| 分类 | 推荐 |
|---|---|
| 构建/框架 | Vite + React + TypeScript；若官网 SEO 优先，可升级 Next.js。 |
| 路由 | React Router。 |
| 请求缓存 | TanStack Query。 |
| 状态管理 | Zustand。 |
| 表单 | React Hook Form + Zod。 |
| 管理后台 UI | Ant Design 或 Arco Design。 |
| 样式 | Tailwind CSS + CSS variables；后台可优先使用组件库 token。 |
| 表格 | 组件库 Table 或 TanStack Table。 |
| 测试 | Vitest、React Testing Library、Playwright。 |
| API 类型 | OpenAPI 生成 TypeScript SDK，或先手写 `src/api/client.ts` 过渡。 |

## 4. 推荐实施顺序

1. 建立 React Web 工程、API SDK、类型模型、鉴权和基础布局。
2. 一次性实现管理端全部模块。
3. 一次性实现用户端会员中心全部模块。
4. 实现快捷助手、本地 WASM 转换和额度扣减链路。
5. 完成 Flutter vs React 对照验收。
6. 将 `/app` 与 `/admin` 一次性切换到 React，保留 Flutter 构建产物作为备份。

## 5. 关键风险

| 风险 | 等级 | 处理 |
|---|---:|---|
| `CommercialApiClient` 等价迁移不完整 | 高 | 先冻结 API 契约，生成或手写 TS SDK，并做契约测试。 |
| 额度扣减顺序错误 | 高 | 快捷助手本地转换必须保持 check -> convert -> consume -> download。 |
| 管理端权限绕过 | 高 | `/admin/v1/me` 和所有 admin API 后端强校验 role，前端只做体验门禁。 |
| WASM 集成差异 | 中 | 复用当前 Rust wasm-pack 产物，单独做 React WASM smoke test。 |
| React/Vue 技术栈分裂 | 中 | 当前统一到 React，避免 Flutter + React 插件 + Vue Web 三套前端并行。 |
| 双前端过渡期能力漂移 | 中 | Flutter Web 功能冻结，仅作为备份；用同一 E2E 场景做对照验收。 |
