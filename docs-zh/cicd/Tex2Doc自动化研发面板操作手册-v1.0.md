# Tex2Doc 自动化研发面板操作手册

版本：v1.0
日期：2026-06-27
适用范围：一阶段实现

---

## 目录

1. [概述](#1-概述)
2. [数据库部署](#2-数据库部署)
3. [后端 API](#3-后端-api)
4. [Flutter Admin 面板](#4-flutter-admin-面板)
5. [Slint 用户端集成](#5-slint-用户端集成)
6. [故障排查](#6-故障排查)

---

## 1. 概述

### 1.1 功能简介

自动化研发面板是 Tex2Doc 开发机的 AI 自动研发前端可控化最小闭环系统，允许管理员：

- 查看自动化研发申请列表和状态
- 审批/拒绝/转人工处理申请
- 管理开发机 Agent 状态
- 追踪从反馈到部署的完整流程

### 1.2 系统架构

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Slint     │────▶│  Rust API   │────▶│ PostgreSQL  │
│  用户端     │     │  (doc-server)│     │  (docdb)   │
└─────────────┘     └─────────────┘     └─────────────┘
                            │
                            ▼
                    ┌─────────────────┐
                    │ Flutter Admin   │
                    │   自动化面板     │
                    └─────────────────┘
```

### 1.3 申请状态流程

```
submitted → triaged → needs_approval → queued_for_dev → claimed → coding
                                                              ↓
                                                        local_validating
                                                              ↓
                                                        local_failed (可重试)
                                                              ↓
                                                          pr_open → ci_running
                                                              ↓
                                                          ci_failed (可重试)
                                                              ↓
                                                        ready_for_merge → production_deployed → notified
```

---

## 2. 数据库部署

### 2.1 迁移文件

迁移文件位置：`docs-zh/money/004_automation_rnd.sql`

### 2.2 执行迁移

```bash
# 使用 psql 命令行
psql -h <host> -U postgres -d docdb -f docs-zh/money/004_automation_rnd.sql

# 或在 Rust 服务启动时自动执行（推荐）
# 服务会自动按顺序加载 001, 002, 003, 004 迁移
```

### 2.3 数据库表说明

#### automation_requests（自动化申请表）

| 字段 | 类型 | 说明 |
|------|------|------|
| id | UUID | 主键 |
| short_id | TEXT | 短 ID（如 REQ-abc12） |
| source_type | TEXT | 来源类型：feedback/github_issue/admin_manual/ci_failure |
| source_id | TEXT | 来源 ID |
| title | TEXT | 申请标题 |
| status | TEXT | 当前状态 |
| priority | TEXT | 优先级：low/normal/high/urgent |
| risk_level | TEXT | 风险等级：low/medium/high/critical |
| claimed_by | TEXT | 领取的 Agent ID |
| pr_url | TEXT | PR 链接 |
| ... | ... | 其他字段详见 SQL |

#### automation_request_events（事件时间线）

| 字段 | 类型 | 说明 |
|------|------|------|
| id | UUID | 主键 |
| request_id | UUID | 关联的申请 ID |
| event_type | TEXT | 事件类型 |
| actor_type | TEXT | 操作者类型：user/admin/ai/agent |
| message | TEXT | 事件描述 |

#### automation_agents（开发机 Agent）

| 字段 | 类型 | 说明 |
|------|------|------|
| id | TEXT | Agent ID（主机名等） |
| hostname | TEXT | 主机名 |
| status | TEXT | 状态：online/offline/busy/paused |
| current_request_id | UUID | 当前处理的任务 ID |
| total_tasks_completed | INT | 已完成任务数 |
| last_heartbeat_at | TIMESTAMPTZ | 最后心跳时间 |

### 2.4 触发器说明

**自动创建触发器**：当 `feedback_threads` 表中插入 high/urgent 优先级的新记录时，自动创建对应的自动化申请。

---

## 3. 后端 API

### 3.1 API 端点总览

| 端点 | 方法 | 说明 |
|------|------|------|
| `/admin/v1/automation/summary` | GET | 获取总览指标 |
| `/admin/v1/automation/requests` | GET | 获取申请列表 |
| `/admin/v1/automation/requests/:id` | GET | 获取申请详情 |
| `/admin/v1/automation/requests/:id/events` | GET | 获取事件时间线 |
| `/admin/v1/automation/requests/:id/approve` | POST | 批准申请 |
| `/admin/v1/automation/requests/:id/reject` | POST | 拒绝申请 |
| `/admin/v1/automation/requests/:id/retry` | POST | 重试失败任务 |
| `/admin/v1/automation/requests/:id/escalate` | POST | 转人工处理 |
| `/admin/v1/automation/agents` | GET | 获取 Agent 列表 |
| `/admin/v1/automation/agents/:id/pause` | POST | 暂停 Agent |
| `/admin/v1/automation/agents/:id/resume` | POST | 恢复 Agent |

### 3.2 认证

所有 API 需要管理员认证，使用 `Authorization: Bearer <admin_token>` 头部。

### 3.3 API 使用示例

#### 获取总览指标

```bash
curl -X GET "http://localhost:2624/admin/v1/automation/summary" \
  -H "Authorization: Bearer <admin_token>"
```

响应：
```json
{
  "pending_approval": 3,
  "waiting_dev": 5,
  "in_development": 2,
  "local_failed": 1,
  "ci_failed": 0,
  "deployed": 12,
  "total": 23
}
```

#### 获取申请列表

```bash
curl -X GET "http://localhost:2624/admin/v1/automation/requests?status=needs_approval&risk_level=low" \
  -H "Authorization: Bearer <admin_token>"
```

#### 批准申请

```bash
curl -X POST "http://localhost:2624/admin/v1/automation/requests/REQ-abc12/approve" \
  -H "Authorization: Bearer <admin_token>"
```

#### 拒绝申请

```bash
curl -X POST "http://localhost:2624/admin/v1/automation/requests/REQ-abc12/reject" \
  -H "Authorization: Bearer <admin_token>" \
  -H "Content-Type: application/json" \
  -d '{"reason": "风险过高，需要人工评估"}'
```

#### 转人工

```bash
curl -X POST "http://localhost:2624/admin/v1/automation/requests/REQ-abc12/escalate" \
  -H "Authorization: Bearer <admin_token>" \
  -H "Content-Type: application/json" \
  -d '{"assignee": "zhangsan@example.com"}'
```

### 3.4 风险限制

| 风险等级 | 可自动批准 | 说明 |
|----------|-----------|------|
| low | ✅ | 可自动批准 |
| medium | ✅ | 可自动批准 |
| high | ❌ | 需要转人工审批 |
| critical | ❌ | 需要转人工审批 |

---

## 4. Flutter Admin 面板

### 4.1 访问路径

1. 登录 Flutter Admin
2. 在左侧导航栏找到「自动化研发」

### 4.2 面板结构

```
┌─────────────────────────────────────────────────────────┐
│ 自动化研发                                    [🔄] [⚙]   │
├─────────────────────────────────────────────────────────┤
│ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐     │
│ │待审批   │ │待开发   │ │开发中   │ │本地失败 │ ...   │
│ │   3     │ │   5     │ │   2     │ │   1     │       │
│ └─────────┘ └─────────┘ └─────────┘ └─────────┘     │
├─────────────────────────────────────────────────────────┤
│ [申请] [Agent] [历史]                                 │
├─────────────────────────────────────────────────────────┤
│ 状态: [全部 ▼] 风险: [全部 ▼] 来源: [全部 ▼]          │
│ 搜索: [________________________]                     │
├─────────────────────────────────────────────────────────┤
│ ┌─────────────────────────────────────────────────┐   │
│ │ REQ-abc12 [Needs Approval] [HIGH] Bug           │   │
│ │ 用户反馈公式解析错误                              │   │
│ │ 来源: Feedback  Agent: -  更新: 2小时前        │   │
│ └─────────────────────────────────────────────────┘   │
│ ┌─────────────────────────────────────────────────┐   │
│ │ REQ-xyz34 [Coding] [LOW] Requirement           │   │
│ │ 添加批量导出功能                                  │   │
│ │ 来源: Feedback  Agent: dev-machine-1  更新: 5分钟前│   │
│ └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### 4.3 申请列表功能

| 功能 | 说明 |
|------|------|
| 状态筛选 | 全部/待审批/已批准/开发中/失败/已上线等 |
| 风险筛选 | 全部/低/中/高/严重 |
| 来源筛选 | 全部/反馈/GitHub/手动/CI 失败 |
| 搜索 | 支持标题、ID、来源 ID 模糊搜索 |
| 详情查看 | 点击申请卡片打开详情对话框 |

### 4.4 详情对话框

点击申请卡片后显示详情对话框，包含：

- **摘要区**：ID、状态、标题、类型、风险、优先级、来源
- **操作区**：批准/拒绝/转人工/重试按钮（根据状态显示）
- **时间线区**：完整的事件历史记录

### 4.5 操作说明

#### 批准申请

1. 点击申请卡片打开详情
2. 检查风险等级（high/critical 不可自动批准）
3. 点击「批准」按钮
4. 申请状态变为「queued_for_dev」，等待 Agent 领取

#### 拒绝申请

1. 点击申请卡片打开详情
2. 点击「拒绝」按钮
3. 输入拒绝原因
4. 点击确认完成拒绝

#### 转人工

1. 点击申请卡片打开详情
2. 点击「转人工」按钮
3. 输入人工处理人信息
4. 点击确认，申请状态变为「needs_human」

#### 重试失败任务

1. 点击失败的申请卡片
2. 点击「重试」按钮
3. 确认重试操作
4. 申请从失败点重新开始流程

### 4.6 Agent 管理

在「Agent」标签页中：

| 功能 | 说明 |
|------|------|
| 查看状态 | online/offline/busy/paused |
| 暂停 Agent | 点击「暂停」按钮，Agent 停止接收新任务 |
| 恢复 Agent | 点击「恢复」按钮，Agent 恢复接收任务 |

---

## 5. Slint 用户端集成

### 5.1 功能说明

Slint 用户端的反馈列表现在会显示自动化研发状态。

### 5.2 新增字段

`FeedbackThreadRow` 结构新增以下字段：

| 字段 | 类型 | 说明 |
|------|------|------|
| automation_status | string | 自动化状态（无则为 "none"） |
| automation_request_id | string | 关联的自动化申请 ID |

### 5.3 状态显示

用户可以在反馈列表中看到其反馈的自动化处理状态：

| 状态 | 显示含义 |
|------|----------|
| none | 未生成自动化申请 |
| submitted | 已提交，待分类 |
| triaged | 已分类，待审批 |
| needs_approval | 等待审批 |
| queued_for_dev | 排队等待开发 |
| claimed | Agent 已领取 |
| coding | 开发中 |
| local_validating | 本地验证中 |
| local_failed | 本地验证失败 |
| pr_open | PR 已创建 |
| ci_running | CI 运行中 |
| ci_failed | CI 失败 |
| production_deployed | 已部署到生产 |
| needs_human | 需要人工处理 |

---

## 6. 故障排查

### 6.1 数据库问题

**问题：迁移执行失败**

```bash
# 检查 PostgreSQL 连接
psql -h <host> -U postgres -d docdb -c "SELECT 1;"

# 查看迁移文件语法
psql -h <host> -U postgres -d docdb -c "BEGIN; ROLLBACK;"

# 手动执行迁移
psql -h <host> -U postgres -d docdb -f docs-zh/money/004_automation_rnd.sql
```

### 6.2 API 问题

**问题：API 返回 401 Unauthorized**

```bash
# 检查 token 是否正确
echo $ADMIN_TOKEN

# 重新获取 admin token
curl -X POST "http://localhost:2624/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@example.com","password":"..."}'
```

**问题：申请无法批准**

- 检查风险等级是否为 high 或 critical
- 这两个等级需要手动转人工处理

### 6.3 前端问题

**问题：Flutter 编译失败**

```bash
cd flutter_app
flutter pub get
flutter analyze
```

**问题：API 请求超时**

- 检查后端服务是否运行：`curl http://localhost:2624/api/v1/health`
- 检查网络连接
- 查看后端日志

### 6.4 Agent 问题

**问题：Agent 状态显示 offline**

1. 检查 Agent 服务是否运行
2. 检查 Agent 心跳是否正常发送
3. 重启 Agent 服务

**问题：Agent 无法领取任务**

1. 检查 Agent 状态是否为 online
2. 检查是否有可用任务（状态为 queued_for_dev）

---

## 附录 A：API 响应格式

### A.1 申请详情响应

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "short_id": "REQ-abc12",
  "source_type": "feedback",
  "source_id": "...",
  "title": "用户反馈的标题",
  "request_type": "bug",
  "status": "needs_approval",
  "priority": "high",
  "risk_level": "high",
  "ai_summary": "AI 分析摘要...",
  "claimed_by": null,
  "pr_url": null,
  "created_at": "2026-06-27T10:00:00Z",
  "updated_at": "2026-06-27T10:00:00Z"
}
```

### A.2 事件时间线响应

```json
[
  {
    "id": "...",
    "request_id": "...",
    "event_type": "request_created",
    "actor_type": "system",
    "message": "Automation request created from feedback",
    "from_status": null,
    "to_status": "submitted",
    "created_at": "2026-06-27T10:00:00Z"
  }
]
```

---

## 附录 B：环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `DATABASE_URL` | PostgreSQL 连接地址 | `postgres://postgres:postgres@127.0.0.1:5432/docdb` |
| `TEX2DOC_BOOTSTRAP_ADMIN_EMAIL` | 初始管理员邮箱 | - |
| `TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD` | 初始管理员密码 | - |
| `DOC_SERVER_ADDR` | 服务监听地址 | `127.0.0.1:2624` |

---

*手册版本：v1.0*
*最后更新：2026-06-27*
