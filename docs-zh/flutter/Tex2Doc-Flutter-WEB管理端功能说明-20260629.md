# Tex2Doc Flutter WEB 管理端功能说明

> 更新日期：2026-06-29
> 适用模块：Flutter Web 管理端
> 入口文件：`flutter_app/lib/main_admin.dart`、`flutter_app/lib/admin/admin_app.dart`、`flutter_app/lib/shared/workspace_app.dart`

## 1. 管理端定位

Flutter Web 管理端面向运营、客服、发布管理员和自动化研发管理员。它复用 `DocEngineApp` 共享壳层，但以 `DocEngineAppMode.admin` 启动。

启动链路：

```text
main_admin.dart
  -> AdminApp(isWeb: true)
  -> DocEngineApp(mode: admin)
  -> _WorkspaceShell
  -> AuthWindow 管理端登录门禁
  -> 管理端导航工作台
```

总入口 `main.dart` 在 Web 环境下也会按 URL path 分流：访问 `/admin` 会进入管理端。

## 2. 登录门禁与权限

管理端未登录时只显示 `AuthWindow`，不会进入工作台。登录成功后会检查 `UserProfile.role`：

| 条件 | 行为 |
|---|---|
| `profile.isAdminRole == true` | 进入管理端工作台。 |
| 非管理员账号 | 清空登录态，显示 `Admin role required.`。 |

登录表单字段：

1. API Base URL。
2. 邮箱。
3. 密码。

注册 Tab 仍可见，但管理端必须使用具备管理员角色的账号，否则无法通过门禁。

## 3. 导航结构

管理端登录后左侧导航包含：

| 导航 | 功能 |
|---|---|
| 管理端仪表盘 | 汇总运营模块和关键数字。 |
| 账号 | 查看当前管理员账号和用量摘要。 |
| 兑换码生成 | 生成兑换码批次并导出 Excel。 |
| 兑换码批次 | 查看历史批次、打开批次详情、导出批次 Excel。 |
| 兑换码库存 | 按状态检索兑换码、批量上货、导入重置、导出清单。 |
| Feedback management | 查看用户反馈、调整状态、回复用户。 |
| 发布管理 | 维护 Beta/Stable 发布清单并回滚。 |
| 审计中心 | 查看发布新增、回滚和策略变更记录。 |
| 自动化 | 管理 AI 自动化研发请求和 Agent。 |
| 关于 | 产品说明。 |

宽屏使用左侧 Sidebar；窄屏隐藏 Sidebar，并在内容顶部用 `SegmentedButton` 切换模块。

## 4. 管理端仪表盘

来源文件：`admin/pages/dashboard/admin_dashboard_panel.dart`

功能：

1. 调用 `adminDashboard` 读取管理端概览。
2. 展示兑换码批次数量。
3. 展示待处理反馈数量。
4. 展示套餐数量。
5. 展示发布通道列表。
6. 展示管理模块列表和更新时间。
7. 支持手动刷新。

操作步骤：

1. 登录管理端后默认进入“管理端仪表盘”。
2. 查看四个指标卡。
3. 需要更新时点击右上角刷新按钮。
4. 如果加载失败，页面会显示错误信息。

## 5. 账号模块

账号模块复用用户端 `_AccountPanel`。

展示内容：

1. 管理员邮箱和头像。
2. 当前套餐/角色相关的 `planId`。
3. 可选显示名。
4. 云端转换用量。
5. 按次余额。
6. 日期有效期。

右上角账号菜单支持查看详情、修改密码、退出登录。当前修改密码是前端模拟成功，尚未真正调用后端密码修改接口。

## 6. 兑换码生成

来源：`AdminRedeemManagePanel`

### 6.1 功能介绍

用于生成一批兑换码，并将结果预览或导出 Excel。

表单字段：

| 字段 | 说明 |
|---|---|
| 套餐 | `count_3`、`count_10`、`count_30`。 |
| 数量 | 生成数量，必须为正整数。 |
| 渠道 | 默认 `web`，用于标记来源渠道。 |
| 过期时间 | 可选，透传给后端 `expires_at`。 |
| 备注 | 可选，用于批次说明。 |
| Admin Token | 如果从管理端登录进入，使用 access token；独立页面无 token 时显示手动输入框。 |

### 6.2 操作流程

1. 进入“兑换码生成”。
2. 选择套餐。
3. 填写数量、渠道、过期时间和备注。
4. 点击“生成”。
5. 页面显示批次编号、批次 ID、状态、创建时间和前 8 个兑换码预览。
6. 点击“下载 Excel”导出该批次完整兑换码。

## 7. 兑换码批次

来源：`AdminRedeemRecordsPanel`

功能：

1. 调用 `redeemCodeBatches` 加载批次列表。
2. 列表展示批次号、套餐、生成数量、状态、渠道、创建时间。
3. 支持打开批次详情。
4. 支持导出批次 Excel。

操作流程：

1. 进入“兑换码批次”。
2. 点击刷新加载批次。
3. 在表格中点击眼睛图标查看详情。
4. 查看批次 ID、已导出数量、备注和最多 60 个兑换码预览。
5. 点击下载图标导出该批次 Excel。

## 8. 兑换码库存

来源：`admin/pages/redeem/admin_redeem_codes_panel.dart`

### 8.1 功能介绍

兑换码库存页用于对单个兑换码做运营管理，支持筛选、搜索、分页、选择、批量上货、导入重置和导出。

状态筛选：

| 状态 | 含义 |
|---|---|
| `new` | 新生成，尚未上货。 |
| `stocked` | 已上货，可售卖或发放。 |
| `redeemed` | 已被用户兑换。 |
| `restocked` | 已导入重置。 |

表格字段：

| 字段 | 说明 |
|---|---|
| 批次号 | 所属批次。 |
| 码预览 | 脱敏后的兑换码。 |
| 套餐 | 套餐名称和数量。 |
| 状态 | 库存状态彩色标记。 |
| 上货时间 | `stockedAt`。 |
| 兑换时间 | `redeemedAt`。 |
| 重置时间 | `restockedAt`。 |
| 创建时间 | `createdAt`。 |

### 8.2 操作流程

搜索与筛选：

1. 点击状态筛选 Chip 切换 `new`、`stocked`、`redeemed`、`restocked`。
2. 在搜索框输入关键字并回车。
3. 点击刷新重新加载。

分页：

1. 底部显示当前区间和总数。
2. 页大小支持 20、50、100、200。
3. 可跳到首页、上一页、下一页、末页。

批量上货：

1. 勾选表格行，或点击“全选”。
2. 点击批量上货按钮。
3. 在确认弹窗中确认数量。
4. 后端返回 affected 数量后自动刷新列表。

导入重置：

1. 点击“导入重置”。
2. 在弹窗中按行粘贴明文兑换码。
3. 点击确认。
4. 后端返回 affected 数量后刷新列表。

导出：

1. 设置当前筛选和搜索条件。
2. 点击“导出 Excel”。
3. 下载当前条件下的兑换码清单 `redeem-codes-list.xlsx`。

## 9. 反馈管理

来源：`admin/pages/feedback/admin_feedback_panel.dart`

功能：

1. 调用 `adminFeedbackThreads` 加载所有反馈线程。
2. 按卡片展示反馈类型、优先级、状态、标题、消息数、创建时间和关联转换任务。
3. 支持修改状态。
4. 支持管理员回复。

状态可选项：

| 状态 | 说明 |
|---|---|
| `open` | 新建或未处理。 |
| `in_progress` | 处理中。 |
| `resolved` | 已解决。 |
| `closed` | 已关闭。 |

操作流程：

1. 进入 Feedback management。
2. 查看反馈卡片。
3. 使用状态下拉框调整处理状态。
4. 点击 Reply，输入回复内容。
5. 点击 Send 后刷新列表。

## 10. 发布管理

来源：`admin/pages/releases/admin_releases_panel.dart`

### 10.1 功能介绍

发布管理用于维护客户端发布清单，支持发布和回滚。

表单字段：

| 字段 | 默认值/说明 |
|---|---|
| 通道 | 默认 `beta`。 |
| 平台 | 默认 `windows`。 |
| 架构 | 默认 `x64`。 |
| 版本 | 必填。 |
| 标题 | 可选。 |
| 下载地址 | 必填。 |
| SHA-256 | 必填。 |

发布时如果通道不是 `stable`，会将 `is_prerelease` 设为 true。当前策略固定写入 `rollout_percent: 100`、`audience: invite_beta`。

### 10.2 发布流程

1. 进入“发布管理”。
2. 填写通道、平台、架构、版本、标题、下载地址、SHA-256。
3. 点击“发布清单”。
4. 成功后提示“发布清单已写入”，并刷新发布列表。

版本、下载地址和 SHA-256 是必填项，缺失时会直接提示。

### 10.3 回滚流程

1. 在发布清单中找到未回滚的版本。
2. 点击“回滚”。
3. 后端写入回滚记录，原因固定为 `admin panel rollback`。
4. 列表刷新后显示已回滚时间。

## 11. 审计中心

来源：`admin/pages/audit/admin_audit_panel.dart`

功能：

1. 调用 `adminReleaseAudit` 加载审计日志。
2. 展示 action、release_id、created_at、actor_user_id、note。
3. 支持手动刷新。

操作流程：

1. 进入“审计中心”。
2. 点击刷新。
3. 查看发布新增、回滚和灰度策略变更记录。

如果没有日志，页面提示“暂无审计日志”。

## 12. 自动化研发面板

来源：`admin/pages/automation/**`

自动化面板用于管理 AI 自动化开发请求、代理运行状态和流程事件。

### 12.1 摘要卡

顶部摘要卡展示：

| 指标 | 含义 |
|---|---|
| Pending Approval | 待审批请求数。 |
| Waiting Dev | 等待开发请求数。 |
| In Development | 开发中请求数。 |
| Local Failed | 本地验证失败数。 |
| CI Failed | CI 失败数。 |
| Deployed | 已部署数。 |
| Total | 总请求数。 |

### 12.2 请求列表

筛选项：

| 筛选 | 可选值 |
|---|---|
| Status | All、Triaged、Needs Approval、Queued、Claimed、Coding、Local Failed、CI Failed、Deployed、Rejected。 |
| Risk | All、Low、Medium、High、Critical。 |
| Source | All、Feedback、GitHub、Manual、CI Failure。 |
| Search | 按标题、ID 或来源搜索。 |

请求卡片展示：

1. 短 ID。
2. 状态标签。
3. 风险标签。
4. 请求类型。
5. 标题。
6. 来源。
7. 认领 Agent。
8. 更新时间。
9. PR 链接图标。当前图标预留，未真正打开 URL。

### 12.3 请求详情与操作

点击请求卡片打开详情弹窗。

详情展示：

1. 短 ID、状态、标题。
2. Type、Risk、Priority、Source、Agent、Branch。
3. AI Summary。
4. PR URL。
5. 操作区。
6. Timeline 事件列表。

可执行操作：

| 操作 | 触发条件 | 说明 |
|---|---|---|
| Approve | 状态为 `triaged` 或 `needs_approval`，且风险不是 `high` / `critical` | 确认后进入自动开发队列。 |
| Reject | 非 `rejected`、`closed`、`production_deployed`、`notified` | 必须填写驳回原因。 |
| Escalate | 可审批状态 | 填写 assignee，转人工。 |
| Retry | `local_failed`、`ci_failed`、`blocked` | 确认后重试失败步骤。 |

高风险和关键风险请求不会显示自动 Approve 按钮，需要升级人工处理。

### 12.4 Agent 列表

Agent 卡片展示：

1. Agent ID。
2. 状态：`online`、`busy`、`paused`、其他视为离线/未知。
3. Hostname。
4. Agent Version。
5. Last Heartbeat。
6. 完成任务数。
7. 失败任务数。
8. 成功率。
9. 当前任务。
10. 能力标签。

操作：

| 状态 | 可执行操作 |
|---|---|
| `online` / `busy` | Pause |
| `paused` | Resume |

### 12.5 历史页与刷新

自动化面板包含 Requests、Agents、History 三个 Tab。History 当前为空态。右上角刷新按钮会重新加载摘要、请求和 Agent。

Auto-refresh 按钮当前只切换 `_autoRefresh` 状态和图标，代码中尚未实现定时器。

## 13. API 对照

| 模块 | API 封装 |
|---|---|
| 管理端资料与仪表盘 | `adminMe`、`adminDashboard` |
| 批次生成与导出 | `createRedeemCodeBatch`、`exportRedeemCodeBatch` |
| 批次列表与详情 | `redeemCodeBatches`、`redeemCodeBatchDetail` |
| 兑换码库存 | `adminListRedeemCodes`、`adminBulkStockRedeemCodes`、`adminRestockRedeemCodes`、`adminExportRedeemCodesExcel` |
| 反馈管理 | `adminFeedbackThreads`、`adminUpdateFeedbackThread`、`adminReplyFeedbackThread` |
| 发布管理 | `adminReleases`、`adminPublishRelease`、`adminRollbackRelease` |
| 审计 | `adminReleaseAudit` |
| 自动化 | `adminAutomationSummary`、`adminAutomationRequests`、`adminAutomationRequest`、`adminAutomationEvents`、`adminAutomationApprove`、`adminAutomationReject`、`adminAutomationRetry`、`adminAutomationEscalate`、`adminAutomationAgents`、`adminAutomationPauseAgent`、`adminAutomationResumeAgent` |

## 14. 使用建议

1. 管理端应部署在受控路径 `/admin`，并确保后端只向管理员角色签发可访问 admin API 的 token。
2. 兑换码生成后先在“兑换码库存”筛选 `new`，确认数据后再批量上货。
3. 售卖或渠道发放前，建议用“导出 Excel”固定当前批次或库存状态快照。
4. 用户反馈进入自动化链路前，应先由客服/运营在反馈管理页确认类型、优先级和状态。
5. 发布清单写入前必须校验下载 URL、SHA-256 和平台/架构字段；回滚后到“审计中心”复核记录。
6. 自动化中 `high` / `critical` 风险请求不应自动批准，应使用 Escalate 转人工。

## 15. 已知限制

1. 管理端登录态没有持久化，页面刷新后需要重新登录。
2. 修改密码当前为前端模拟成功。
3. 自动化 Auto-refresh 未实现定时刷新。
4. 请求卡片中的 PR 链接按钮当前未调用打开 URL。
5. History Tab 当前为空态。
