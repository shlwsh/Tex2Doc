# Tex2Doc Browser Extension — Privacy Policy

> 适用版本：当前 `dev0625` 分支（v0.1.0）
> 生效日期：2026-06-28
> 适用范围：Tex2Doc 浏览器扩展（Chrome / Edge / Firefox / Safari）

Tex2Doc 浏览器扩展（以下简称"扩展"）由 Tex2Doc 项目团队开发。本隐私声明按照 [Chrome Web Store Program Policies](https://developer.chrome.com/docs/webstore/program-policies/) 及等效商店（Edge Add-ons、Firefox AMO、Safari App Store）的要求，公开说明扩展收集、使用、存储和传输的数据范围。

---

## 1. 我们不收集的内容

- 您的 LaTeX 源文件（除非您主动选择"云端转换"，见 §2）
- 您的访问历史、浏览记录、cookie
- 您的设备指纹、硬件序列号
- 任何用于广告投放或画像的用户画像标签
- 任何出售给第三方数据中介的信息

---

## 2. 数据收集与使用范围

### 2.1 本地 WASM 转换（默认模式）

**完全在浏览器内进行，无任何网络上传。**

- 转换引擎通过 WebAssembly 在浏览器内执行
- `.tex` / `.zip` 文件**只**由扩展进程读取，不发送到任何服务器
- 生成的 `.docx` 通过浏览器 `chrome.downloads` API 保存到本地
- 唯一写入存储的是 `ExtensionSettings`（API base URL、默认 profile / quality / 模式、语言、主题、轮询间隔等）

### 2.2 云端转换（需登录）

当您选择"云端转换"并登录账户时，以下数据会上传到 `https://api.tex2doc.cn`：

| 数据 | 用途 | 保留 |
|---|---|---|
| `.zip` 项目包 | 服务端执行 LaTeX → DOCX 转换 | 任务完成后 24h 自动清理 |
| `main.tex` 文件名 | 路由引擎决策 | 同上 |
| Profile / Quality 设置 | 决定转换规则与质量阈值 | 任务级别 |
| 输出 `.docx` | 供您下载 | 任务完成后 7 天 |

> 我们**不会**永久保留您的项目文件副本。如需隐私保护更严格的场景，请使用本地 WASM 模式。

### 2.3 账户与兑换码

| 数据 | 用途 | 保留 |
|---|---|---|
| 邮箱 + 密码（密码经服务端 bcrypt 加盐哈希） | 登录鉴权 | 直到您删除账户 |
| Access token + Refresh token | API 调用鉴权 | 加密存储在浏览器 `storage.local` |
| 用户显示名、套餐 ID | UI 渲染 | 直到您删除账户 |
| 兑换码 | 充值配额 / 自动开账户 | 仅服务端校验，本地不保留原码 |

> **Token 不上传给第三方**。`refresh_token` 仅存放在扩展本地存储（见 §3）。

### 2.4 Overleaf / arXiv 集成（可选 host permission）

当您打开 Overleaf 项目或 arXiv 论文页面时，扩展通过可选 host 权限 `https://*.overleaf.com/*` 与 `https://*.arxiv.org/*` 读取页面 DOM（仅"Convert"按钮触发时）。我们**不**：
- 记录您访问的页面 URL
- 读取页面正文 / 注释 / 评审
- 与任何第三方共享这些页面访问

### 2.5 反馈 / 诊断包（可选）

在 SidePanel 的 Feedback 面板，您可以：
- 提交反馈标题 + 内容（默认不含源文件 / token / 邮箱 / 文件名）
- 一键导出诊断包（200 条以内事件环形缓冲）

诊断包**显式不包含**：
- 任何 `.tex` / `.zip` 源文件
- `access_token` / `refresh_token`
- 您的邮箱地址
- 文件内容

---

## 3. 本地存储说明

| 存储 | 内容 | 清除方式 |
|---|---|---|
| `chrome.storage.local` | 用户会话（access / refresh token）、设置、最近一次配额快照 | 扩展"Sign Out" 或卸载扩展 |
| IndexedDB `tex2doc_extension.jobs` | 任务历史（ID、文件名、main_tex、状态、进度、错误码） | "Clear completed jobs" 按钮或卸载 |
| IndexedDB `tex2doc_extension.events` | 最近 1000 条事件环形缓冲（脱敏后） | 卸载扩展 |
| `chrome.notifications` | 转换完成 / 失败提醒 | 系统通知中心手动清除 |

**所有敏感字段（token、密码 hash）均不上报到任何分析平台**。`refresh_token` 未来计划迁移至 Web Crypto 加密存储（见报告 §9 / P2-3）。

---

## 4. 第三方服务

| 服务 | 场景 | 数据 |
|---|---|---|
| `https://api.tex2doc.cn` | 云端转换 / 账户 / 计费 | 见 §2.2 / §2.3 |
| `https://*.overleaf.com` | Overleaf 项目转换（需您手动触发） | DOM 读取 |
| `https://*.arxiv.org` | arXiv 论文下载（需您手动触发） | 公开摘要 URL |

扩展**不**嵌入任何第三方分析 SDK、广告 SDK 或社交追踪像素。

---

## 5. 权限说明（与 manifest 严格对齐）

| 权限 | 用途 |
|---|---|
| `storage` | 存会话、设置、任务 |
| `downloads` | 保存转换后的 `.docx` |
| `contextMenus` | "Open Tex2Doc" 右键菜单 |
| `notifications` | 转换完成 / 失败通知 |
| `sidePanel`（仅 Edge 构建） | Edge 原生侧边栏入口 |
| `host_permissions: https://api.tex2doc.cn/*` | 云端 API |
| `optional_host_permissions: overleaf / arxiv` | 可选集成 |

每项权限均可通过"Options → Permissions"标签页单独管理（见报告 §9 / P0-5）。

---

## 6. 儿童隐私

Tex2Doc 浏览器扩展不面向 13 岁以下儿童。如果发现未授权的儿童数据收集，请通过 `privacy@tex2doc.cn` 联系我们，我们会在 7 个工作日内删除。

---

## 7. 数据安全

- 传输层：所有云端 API 调用走 HTTPS / TLS 1.2+
- 鉴权：Bearer Token + 短有效期 access token + 长期 refresh token
- 日志：服务端日志保留 30 天，仅用于安全审计，不含个人可识别信息
- 备份：数据库每日加密备份，保留 7 天

---

## 8. 您的权利（GDPR / CCPA 对齐）

- **访问权**：通过 SidePanel 的 Account 面板查看账户信息
- **更正权**：通过 Options → Account 修改显示名
- **删除权**：Sign Out 并卸载扩展可清空本地数据；服务端账户删除请联系 `privacy@tex2doc.cn`
- **可携带权**：导出任务列表为 JSON（即将在 P1-3 提供）
- **拒绝权**：随时通过 Options → Permissions 撤销可选 host 权限

---

## 9. 政策变更

本政策可能随功能迭代更新。任何实质变更将通过以下渠道提前 14 天通知：
- 扩展内通知（`chrome.notifications`）
- GitHub Releases（https://github.com/your-org/tex2doc/releases）
- 商店更新说明

---

## 10. 联系方式

- 邮箱：`privacy@tex2doc.cn`
- 项目主页：https://tex2doc.cn
- 问题追踪：https://github.com/your-org/tex2doc/issues
- 数据保护官邮箱：`dpo@tex2doc.cn`（如适用）

---

## 11. 关联文档

- 整体商业化进度报告：`docs-zh/extension/Tex2Doc-浏览器插件商业化改造开发进展报告-20260628.md`
- 风险登记表（PRIV-* 风险）：报告 §8 + 附录 C
- 商店元数据：`wxt.config.ts::manifest`（含 `short_name` / `author` / `homepage_url` 等）
- 用户使用手册：`docs-zh/extension/README.md`

---

> 本文档由 Tex2Doc 法务 + 工程团队联合审阅。如发现与代码实现不一致之处，请以工程 PR 为准，并同步更新本文档。