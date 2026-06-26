# Tex2Doc 开发机 AI 自动研发前端可控化最小闭环实现设计方案

版本：v2.1
时间戳：20260626-224350
日期：2026-06-26
状态：实现设计方案

## 1. 本版目标

本版在 v2.0“开发机 AI 自动研发与 CICD 生产闭环”基础上，重点补齐前端 UI 与可控化、可追溯能力，目标是先实现一个最小可行闭环，而不是一次性做全自动研发平台。

最小闭环定义：

```text
用户反馈
  -> 后台生成自动化申请
  -> 管理员在 UI 中审批
  -> 开发机 Agent 自动领取
  -> AI 在隔离分支实现
  -> 本地验证结果回写
  -> 创建 PR
  -> GitHub CI 状态回写
  -> 人工合并
  -> 生产部署
  -> 反馈线程和客户端通知
```

首期必须做到：

1. 每个自动化申请都有可见状态、负责人、风险、来源、PR、CI、部署记录。
2. 管理员可以在后台明确批准、拒绝、暂停、重试和转人工。
3. 用户可以在反馈线程看到处理进度和最终结果。
4. 开发机 Agent 的行为可追溯：领取、心跳、分支、验证、PR、失败原因都能查。
5. 生产部署仍走现有 GitHub Actions 和人工合并，不做 AI 自动合并。

## 2. 现有前端基础

### 2.1 Flutter Admin

现有入口：

- `flutter_app/lib/admin/admin_app.dart`
- `flutter_app/lib/shared/workspace_app.dart`
- `flutter_app/lib/admin/pages/dashboard/admin_dashboard_panel.dart`
- `flutter_app/lib/admin/pages/feedback/admin_feedback_panel.dart`
- `flutter_app/lib/admin/pages/releases/admin_releases_panel.dart`
- `flutter_app/lib/admin/pages/audit/admin_audit_panel.dart`

可复用组件：

- `AppSectionHeader`
- `AppCard`
- `MetricTile`
- `StatusPill`
- `LoadingState`
- `EmptyState`
- `ErrorState`
- `PageContainer`

导航现状：

- `_NavSection` 统一定义后台和用户端导航。
- Admin 当前包含 dashboard、feedback、releases、audit 等模块。
- 新增自动化研发页面时，应扩展 `_NavSection`，新增 `automation` 导航项，图标建议使用 `Icons.auto_awesome_motion_outlined` 或 `Icons.account_tree_outlined`。

### 2.2 用户端反馈界面

现有入口：

- Flutter 用户端：`flutter_app/lib/ui/feedback_panel.dart`
- Flutter 反馈线程：`flutter_app/lib/ui/feedback_thread_panel.dart`
- Slint 用户端：`apps/slint-user/src/ui_bindings/feedback.rs`

现有能力：

- 用户可以提交 issue/requirement 类型反馈。
- 用户可以刷新反馈线程。
- 反馈线程已有 status、priority、message_count、conversion_job_id 等字段。

首期应在这些界面上补充“自动化处理状态”，而不是另做一个复杂入口。

### 2.3 客户端升级通知

现有入口：

- 服务端 `GET /v1/releases/:channel`
- Slint `apps/slint-user/src/updater.rs`
- Slint `apps/slint-user/src/desktop_update.rs`
- Slint `apps/slint-user/src/ui_bindings/update.rs`
- Flutter Admin `AdminReleasesPanel`

首期建议复用 release manifest 和反馈线程自动回复做通知；Web 客户端通知中心可作为 P2。

## 3. UI 信息架构

### 3.1 Admin 新增“自动化研发”导航

新增后台页面：

```text
自动化研发
  ├─ 总览
  ├─ 申请队列
  ├─ 任务详情
  ├─ Agent 状态
  ├─ 验证与 CI
  └─ 通知记录
```

首期不需要拆成多个顶级导航，建议做成一个页面内的 tabs：

| Tab | 用途 |
| --- | --- |
| 总览 | 指标、队列积压、失败项、最近部署 |
| 申请队列 | 全部自动化申请列表与筛选 |
| Agent | 开发机 Agent 在线、领取、心跳、当前任务 |
| 验证 | 本地验证、PR CI、部署状态 |
| 通知 | 反馈回写和客户端通知记录 |

### 3.2 页面布局原则

后台是运营控制台，不做营销式页面。界面应紧凑、可扫描、可批量处理。

推荐布局：

```text
顶部：标题 + 刷新 + 自动刷新开关
指标行：待审批 / 开发中 / 本地验证失败 / CI 失败 / 已上线
筛选行：状态 / 风险 / 来源 / Agent / 时间范围 / 搜索
主体：申请表格或列表
右侧/弹窗：详情抽屉，展示时间线和操作
```

移动端 Admin：

- 保持单列。
- 列表每条显示标题、状态、风险、更新时间。
- 详情进入独立页面或底部抽屉。

## 4. 自动化研发工作台设计

### 4.1 总览区

指标建议：

| 指标 | 含义 | 操作 |
| --- | --- | --- |
| 待审批 | `triaged/needs_approval` | 点击筛选待审批 |
| 等待开发 | `queued_for_dev` | 查看队列 |
| 开发中 | `claimed/coding/local_validating` | 查看 Agent |
| 本地失败 | `local_failed` | 查看日志并重试 |
| CI 失败 | `ci_failed` | 查看 PR/CI |
| 已上线 | `production_deployed/notified` | 查看通知 |

UI 组件：

- 使用 `MetricTile` 展示数字和图标。
- 数值颜色只用于风险提示：失败为 error，高风险为 warning，正常为 primary。
- 卡片保持同一高度，避免刷新时布局跳动。

### 4.2 申请队列列表

列表字段：

| 字段 | 说明 |
| --- | --- |
| 申请编号 | 短 ID，例如 `REQ-a13f9` |
| 标题 | 来源反馈标题或 AI 归纳标题 |
| 来源 | feedback / github_issue / admin_manual / ci_failure |
| 类型 | bug / requirement / docs / test / ops |
| 风险 | low / medium / high / critical |
| 状态 | 状态机当前状态 |
| Agent | claimed_by |
| PR | PR 链接或空 |
| 更新时间 | 最新事件时间 |

筛选控件：

- Segmented control：全部 / 待审批 / 开发中 / 验证失败 / 已上线。
- Dropdown：风险等级。
- Dropdown：来源类型。
- SearchField：标题、申请编号、反馈线程 ID。
- Switch：仅显示需要我处理。

列表行动作：

- 查看详情。
- 批准。
- 拒绝。
- 转人工。
- 重试。
- 暂停。

首期只对 `triaged` 和 `needs_approval` 显示批准/拒绝；对 `local_failed`、`ci_failed` 显示重试；其他状态只读。

### 4.3 任务详情抽屉

详情建议使用右侧抽屉或全宽详情页，包含：

1. 摘要区
   - 标题、状态、风险、类型、优先级。
   - 来源反馈线程和转换任务链接。
   - 当前负责人/Agent。

2. AI 分诊
   - 问题摘要。
   - 复现步骤。
   - 影响范围。
   - 验收标准。
   - 测试计划。
   - 转人工原因。

3. 执行信息
   - 分支名。
   - worktree 路径。
   - PR URL。
   - CI run URL。
   - 部署 release id。

4. 时间线
   - submitted
   - triaged
   - approved
   - claimed
   - coding
   - local_validating
   - pr_open
   - ci_running
   - ready_for_merge
   - production_deployed
   - notified

5. 操作区
   - 批准自动开发。
   - 拒绝自动开发。
   - 暂停任务。
   - 重试本地验证。
   - 重试 AI 修复。
   - 标记已人工处理。

操作必须有确认弹窗：

- 批准：显示风险、影响范围和允许的自动化范围。
- 拒绝：必须填写原因。
- 重试：显示上次失败阶段和日志摘要。
- 转人工：必须选择负责人或输入说明。

### 4.4 时间线组件

时间线是可追溯的核心。建议新增共享组件：

```text
AutomationTimeline
AutomationTimelineItem
```

字段：

| 字段 | 说明 |
| --- | --- |
| event_type | 事件类型 |
| actor_type | user / admin / ai / agent / github / system |
| actor_name | 操作人或系统 |
| from_status | 原状态 |
| to_status | 新状态 |
| message | 人类可读摘要 |
| payload | 可折叠 JSON |
| created_at | 时间 |

UI 行为：

- 默认展示摘要。
- 点击展开 payload。
- 对失败事件显示错误摘要和日志链接。
- 对 PR/CI/部署事件显示外链按钮。

### 4.5 Agent 状态页

字段：

| 字段 | 说明 |
| --- | --- |
| Agent ID | 开发机唯一标识 |
| Hostname | 开发机主机名 |
| 状态 | online / offline / busy / paused |
| 当前任务 | request id |
| 最近心跳 | last_heartbeat_at |
| 能力 | rust / flutter / db / e2e |
| 版本 | agent_version |
| 成功率 | 最近 7 天通过率 |

操作：

- 暂停领取。
- 恢复领取。
- 释放超时任务。
- 查看最近日志。

首期只做只读展示 + 暂停/恢复。

## 5. 用户端 UI 闭环

### 5.1 反馈列表状态增强

在用户反馈列表中新增“处理进度”：

| 用户可见状态 | 内部状态映射 |
| --- | --- |
| 已收到 | submitted / aggregated |
| 正在分析 | triaged |
| 等待处理 | needs_approval / queued_for_dev |
| 正在开发 | claimed / coding |
| 正在验证 | local_validating / pr_open / ci_running |
| 已发布 | production_deployed / notified |
| 需要人工处理 | needs_human / blocked |
| 已关闭 | closed |

用户端不要展示：

- Agent ID。
- worktree 路径。
- 内部日志。
- GitHub token 或内部 PR 分支权限。
- 生产部署细节。

### 5.2 反馈线程消息

自动追加系统消息：

```text
系统：你的反馈已进入自动化处理队列。
当前阶段：正在分析
预计下一步：生成复现和验收标准
```

部署成功后：

```text
系统：你的反馈已完成处理并发布。
上线版本：20260626-224350
验证方式：请刷新 Web 页面，或在桌面端检查更新。
若问题仍存在，请继续回复本线程。
```

### 5.3 桌面端升级提示

首期利用现有 Slint 更新检查：

- 服务端 release manifest 携带修复摘要。
- Slint 端 `wire_update` 展示更新状态。
- 后续再补充推荐升级/强制升级 UI。

首期不做自动安装，只提示下载和版本信息。

## 6. 最小后端 API 设计

为了支撑 UI，首期 API 不求完整自动化，只求状态可控和可追溯。

### 6.1 Admin API

| API | 用途 |
| --- | --- |
| `GET /admin/v1/automation/summary` | 总览指标 |
| `GET /admin/v1/automation/requests` | 申请列表 |
| `GET /admin/v1/automation/requests/:id` | 申请详情 |
| `GET /admin/v1/automation/requests/:id/events` | 时间线 |
| `POST /admin/v1/automation/requests/:id/approve` | 批准进入开发 |
| `POST /admin/v1/automation/requests/:id/reject` | 拒绝自动化 |
| `POST /admin/v1/automation/requests/:id/retry` | 重试失败阶段 |
| `POST /admin/v1/automation/requests/:id/escalate` | 转人工 |
| `GET /admin/v1/automation/agents` | Agent 状态 |
| `POST /admin/v1/automation/agents/:id/pause` | 暂停 Agent |
| `POST /admin/v1/automation/agents/:id/resume` | 恢复 Agent |

### 6.2 Agent API

| API | 用途 |
| --- | --- |
| `POST /admin/v1/automation/agent/register` | 注册开发机 Agent |
| `POST /admin/v1/automation/agent/heartbeat` | 心跳 |
| `POST /admin/v1/automation/requests/claim` | 领取任务 |
| `POST /admin/v1/automation/requests/:id/events` | 写事件 |
| `POST /admin/v1/automation/requests/:id/local-validation` | 回写本地验证 |
| `POST /admin/v1/automation/requests/:id/pr` | 回写 PR |

### 6.3 用户 API

| API | 用途 |
| --- | --- |
| `GET /v1/feedback/threads/:id/automation-status` | 查询用户可见处理状态 |
| `GET /v1/client-notifications` | 用户通知列表，P2 可做 |

首期也可以不新增用户 API，直接把处理状态写入反馈系统消息，降低实现成本。

## 7. 前端数据模型

Flutter `commercial_api.dart` 建议新增模型：

```dart
class AutomationRequest {
  final String requestId;
  final String title;
  final String sourceType;
  final String requestType;
  final String status;
  final String priority;
  final String riskLevel;
  final String? feedbackThreadId;
  final String? conversionJobId;
  final String? claimedBy;
  final String? branchName;
  final String? prUrl;
  final String? ciRunUrl;
  final String? deployedVersion;
  final String createdAt;
  final String updatedAt;
}
```

```dart
class AutomationEvent {
  final String eventId;
  final String eventType;
  final String actorType;
  final String? actorId;
  final String? fromStatus;
  final String? toStatus;
  final String message;
  final Map<String, dynamic> payload;
  final String createdAt;
}
```

```dart
class AutomationAgent {
  final String agentId;
  final String hostname;
  final String status;
  final String agentVersion;
  final String? currentRequestId;
  final String lastHeartbeatAt;
  final Map<String, dynamic> capabilities;
}
```

## 8. 页面与组件拆分

建议新增文件：

```text
flutter_app/lib/admin/pages/automation/
  admin_automation_panel.dart
  automation_request_list.dart
  automation_request_detail.dart
  automation_timeline.dart
  automation_agent_panel.dart
  automation_status.dart
```

`admin_automation_panel.dart`：

- 页面顶层。
- 加载 summary、requests、agents。
- 处理 tab、筛选、刷新。

`automation_request_list.dart`：

- 列表与筛选。
- 行点击打开详情。
- 行内状态徽标。

`automation_request_detail.dart`：

- 摘要、AI 分诊、执行信息、操作按钮。
- 操作按钮调用 approve/reject/retry/escalate。

`automation_timeline.dart`：

- 独立时间线组件。
- 可复用于反馈详情、发布审计详情。

`automation_agent_panel.dart`：

- Agent 列表。
- pause/resume。

`automation_status.dart`：

- 状态到文案、颜色、图标的映射。
- 保证后台和用户端显示一致。

## 9. 状态视觉设计

### 9.1 状态颜色

| 状态组 | 颜色语义 | 图标 |
| --- | --- | --- |
| 待处理 | neutral | `Icons.inbox_outlined` |
| 分析中 | primary | `Icons.manage_search_outlined` |
| 等审批 | warning | `Icons.rule_folder_outlined` |
| 开发中 | primary | `Icons.code` |
| 验证中 | info | `Icons.science_outlined` |
| 失败 | error | `Icons.error_outline` |
| 已上线 | success | `Icons.verified_outlined` |
| 转人工 | warning | `Icons.support_agent` |

实现建议：

- 使用 `StatusPill`，新增 `AutomationStatusPill` 包装颜色和图标。
- 避免大面积彩色背景，只用徽标、图标和细线强调状态。
- 失败状态显示明确操作：查看日志、重试、转人工。

### 9.2 风险展示

风险必须常驻可见：

| 风险 | UI |
| --- | --- |
| low | 绿色/普通 |
| medium | 蓝色或橙色 |
| high | 橙色，操作前二次确认 |
| critical | 红色，禁止自动批准 |

审批按钮规则：

- `high/critical` 不显示“批准自动开发”，只显示“转人工”。
- `medium` 批准时弹窗要求确认影响范围。
- `low` 可直接批准，但仍记录审批事件。

## 10. 可追溯数据设计

首期至少记录这些事件：

| 事件 | 触发方 |
| --- | --- |
| `request_created` | system |
| `triage_completed` | ai |
| `approval_requested` | system |
| `approved` | admin |
| `rejected` | admin |
| `agent_claimed` | agent |
| `agent_heartbeat` | agent |
| `branch_created` | agent |
| `impact_checked` | agent |
| `local_validation_started` | agent |
| `local_validation_passed` | agent |
| `local_validation_failed` | agent |
| `pr_opened` | github/agent |
| `ci_started` | github |
| `ci_passed` | github |
| `ci_failed` | github |
| `merged` | github |
| `production_deployed` | github |
| `feedback_notified` | system |

事件 payload 示例：

```json
{
  "branch": "codex/auto-a13f9-feedback-export-filter",
  "commands": ["npm run ci:preflight"],
  "duration_ms": 184000,
  "log_key": "automation/a13f9/preflight.log",
  "affected_processes": ["AdminFeedbackExport"],
  "risk": "medium"
}
```

## 11. 最小数据库变更

首期不必一次性实现完整表群，但至少需要三张表。

### 11.1 `automation_requests`

```sql
CREATE TABLE IF NOT EXISTS automation_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_type TEXT NOT NULL,
    source_id TEXT NOT NULL,
    feedback_thread_id UUID REFERENCES feedback_threads(id) ON DELETE SET NULL,
    conversion_job_id UUID REFERENCES conversion_jobs(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    request_type TEXT NOT NULL DEFAULT 'unknown',
    status TEXT NOT NULL DEFAULT 'submitted',
    priority TEXT NOT NULL DEFAULT 'normal',
    risk_level TEXT NOT NULL DEFAULT 'unknown',
    ai_summary TEXT,
    ai_spec JSONB NOT NULL DEFAULT '{}'::jsonb,
    acceptance_criteria JSONB NOT NULL DEFAULT '[]'::jsonb,
    claimed_by TEXT,
    branch_name TEXT,
    pr_url TEXT,
    ci_run_url TEXT,
    deployed_version TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (source_type, source_id)
);
```

### 11.2 `automation_request_events`

```sql
CREATE TABLE IF NOT EXISTS automation_request_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    request_id UUID NOT NULL REFERENCES automation_requests(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    actor_type TEXT NOT NULL,
    actor_id TEXT,
    from_status TEXT,
    to_status TEXT,
    message TEXT,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### 11.3 `automation_agents`

```sql
CREATE TABLE IF NOT EXISTS automation_agents (
    id TEXT PRIMARY KEY,
    hostname TEXT NOT NULL,
    agent_version TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'online',
    current_request_id UUID REFERENCES automation_requests(id) ON DELETE SET NULL,
    capabilities JSONB NOT NULL DEFAULT '{}'::jsonb,
    last_heartbeat_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

## 12. 最小实现步骤

### M1：后台只读可视化

目标：先让管理端看见自动化申请和事件。

实现：

1. 新增数据库表。
2. 从 `feedback_threads` 手动或定时生成 `automation_requests`。
3. 新增 `GET /admin/v1/automation/summary`。
4. 新增 `GET /admin/v1/automation/requests`。
5. 新增 `GET /admin/v1/automation/requests/:id/events`。
6. Flutter Admin 新增“自动化研发”导航和列表页。

验收：

- 管理端可看到来自反馈的申请。
- 申请状态、风险、来源、时间可筛选。
- 详情页可看到事件时间线。

### M2：审批与可控操作

目标：管理员能控制任务是否进入自动开发。

实现：

1. `approve/reject/escalate` API。
2. 详情页操作按钮。
3. 操作确认弹窗。
4. 操作写入事件。
5. `high/critical` 风险禁止自动批准。

验收：

- 批准后状态进入 `queued_for_dev`。
- 拒绝后必须有原因。
- 转人工后用户反馈线程追加系统消息。

### M3：Agent 状态与任务领取

目标：开发机 Agent 可领取任务，后台能看到它在做什么。

实现：

1. Agent 注册和 heartbeat。
2. 任务 claim API。
3. Agent 列表页。
4. 当前任务、最近心跳、状态显示。
5. 超时任务提示。

验收：

- Agent 在线状态可见。
- 任务领取后显示 `claimed_by`。
- 心跳停止后 UI 标红并提示释放任务。

### M4：本地验证与 PR 回写

目标：AI 完成后，后台能看到本地验证和 PR。

实现：

1. 本地验证回写 API。
2. PR 回写 API。
3. 详情页显示命令、结果、日志链接。
4. `local_failed` 显示重试按钮。
5. `pr_open` 显示 GitHub PR 链接。

验收：

- `npm run ci:preflight` 结果可在后台查看。
- 失败能看到摘要。
- 成功后能跳转 PR。

### M5：CI/CD 与通知闭环

目标：PR/部署状态回写，用户得到结果通知。

实现：

1. GitHub webhook 记录 CI 和 merge 事件。
2. `deploy-production.yml` 成功后回写生产部署事件。
3. 反馈线程追加系统消息。
4. release notes 带上申请摘要。
5. 桌面端通过 release manifest 看到更新说明。

验收：

- PR CI 状态在后台可见。
- 部署成功后申请进入 `production_deployed`。
- 用户反馈线程收到“已发布”消息。

## 13. 首期不做的内容

为了保证最小闭环稳定，首期明确不做：

- AI 自动合并 PR。
- AI 自动触发生产部署。
- 自动安装桌面端更新。
- 高风险任务自动开发。
- 完整通知中心和消息已读状态。
- 多开发机复杂调度。
- 成本报表和模型选择策略。

这些能力可以在 v2.2/v3.0 中扩展。

## 14. 验收清单

| 编号 | 验收项 |
| --- | --- |
| 1 | 后台出现“自动化研发”导航 |
| 2 | 用户反馈可生成自动化申请 |
| 3 | 申请列表支持状态、风险、来源筛选 |
| 4 | 申请详情展示 AI 摘要、验收标准和时间线 |
| 5 | 管理员可批准、拒绝、转人工 |
| 6 | high/critical 风险不能自动批准 |
| 7 | Agent 在线、忙碌、离线状态可见 |
| 8 | Agent 领取任务后后台显示 claimed_by |
| 9 | 本地验证结果和日志链接可见 |
| 10 | PR URL 可见并能跳转 GitHub |
| 11 | CI 失败/成功状态可回写 |
| 12 | 生产部署成功后申请状态变为已上线 |
| 13 | 反馈线程收到自动系统消息 |
| 14 | release manifest/release notes 包含本次修复摘要 |

## 15. 结论

当前项目已经有 Admin 工作台、反馈管理、发布管理、审计面板、Slint 更新检查和 GitHub Actions 部署链路。最小可行闭环不需要先做复杂平台，而应先把“自动化申请”作为后台可控工作台接入现有导航：让管理员看得见、批得准、停得住、追得到。

首期的关键交付是五件事：申请列表、详情时间线、审批操作、Agent 状态、本地验证与 PR/部署回写。只要这五件事跑通，后续再扩大 AI 自动开发范围，风险就会低很多，整个链路也能从第一天开始形成审计资产。
