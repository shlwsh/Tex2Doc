# Tex2Doc API 接口清单

## 1. 基础约定

默认服务地址：

```text
http://127.0.0.1:2624
```

用户端 API 主要使用 `/v1`，部分接口同时保留 `/api/v1` 兼容路径。管理端 API 使用 `/admin/v1`。

认证方式：

```http
Authorization: Bearer <access_token>
```

JSON 请求头：

```http
Content-Type: application/json
```

错误响应通常为 JSON，具体字段由 `ApiError` 转换逻辑生成。文件下载接口失败时也可能返回文本错误信息。

## 2. 健康检查与版本

| 方法 | 路径 | 认证 | 说明 |
| --- | --- | --- | --- |
| GET | `/api/v1/health` | 否 | 健康检查，返回 `{ "status": "ok" }` |
| GET | `/api/v1/version` | 否 | 返回服务包名与版本 |
| GET | `/v1/downloads` | 否 | 返回预览下载信息 |
| GET | `/api/v1/downloads` | 否 | 同上 |

## 3. 等候名单

| 方法 | 路径 | 认证 | 说明 |
| --- | --- | --- | --- |
| POST | `/v1/waitlist` | 否 | 创建等候名单线索 |
| POST | `/api/v1/waitlist` | 否 | 同上 |

请求示例：

```json
{
  "email": "user@example.com",
  "identity": "researcher",
  "paper_type": "journal",
  "current_tool": "manual",
  "pain_point": "formatting",
  "paid_intent": "yes"
}
```

## 4. 认证与用户

| 方法 | 路径 | 认证 | 说明 |
| --- | --- | --- | --- |
| POST | `/v1/auth/register` | 否 | 注册用户 |
| POST | `/api/v1/auth/register` | 否 | 同上 |
| POST | `/v1/auth/login` | 否 | 登录 |
| POST | `/api/v1/auth/login` | 否 | 同上 |
| POST | `/v1/auth/refresh` | 否 | 刷新 access token |
| POST | `/api/v1/auth/refresh` | 否 | 同上 |
| GET | `/v1/me` | 是 | 当前用户信息 |
| GET | `/api/v1/me` | 是 | 同上 |

注册请求：

```json
{
  "email": "demo@example.com",
  "password": "secret",
  "display_name": "Demo"
}
```

登录请求：

```json
{
  "email": "demo@example.com",
  "password": "secret"
}
```

认证响应核心字段：

```json
{
  "access_token": "access-...",
  "refresh_token": "refresh-...",
  "user": {
    "id": "...",
    "email": "demo@example.com",
    "display_name": "Demo",
    "plan_id": "preview",
    "role": "user",
    "status": "active"
  }
}
```

## 5. 用量、套餐与充值

| 方法 | 路径 | 认证 | 说明 |
| --- | --- | --- | --- |
| GET | `/v1/usage` | 是 | 查询当前用户用量 |
| GET | `/api/v1/usage` | 是 | 同上 |
| GET | `/v1/plans` | 否 | 查询可用套餐 |
| GET | `/api/v1/plans` | 否 | 同上 |
| GET | `/v1/recharge/options` | 是 | 查询充值选项 |
| GET | `/api/v1/recharge/options` | 是 | 同上 |
| GET | `/v1/recharges` | 是 | 查询充值记录 |
| POST | `/v1/recharges` | 是 | 创建充值记录 |
| GET | `/api/v1/recharges` | 是 | 同上 |
| POST | `/api/v1/recharges` | 是 | 同上 |

创建充值请求：

```json
{
  "recharge_type": "count",
  "package_id": "count_3",
  "quantity": 3
}
```

## 6. 兑换码

| 方法 | 路径 | 认证 | 说明 |
| --- | --- | --- | --- |
| GET | `/v1/redeem-codes/options` | 是 | 查询兑换码套餐 |
| GET | `/api/v1/redeem-codes/options` | 是 | 同上 |
| POST | `/v1/redeem-codes/redeem` | 可选 | 兑换码充值 |
| POST | `/api/v1/redeem-codes/redeem` | 可选 | 同上 |
| GET | `/v1/redeem-codes/records` | 是 | 查询个人兑换记录 |
| GET | `/api/v1/redeem-codes/records` | 是 | 同上 |

兑换请求：

```json
{
  "code": "T2D-XXXX-XXXX-XXXX-XX"
}
```

说明：

- 普通兑换需要 Bearer token。
- 若兑换码批次启用 `auto_provision`，匿名调用可自动创建账户并返回 token。

## 7. 支付占位接口

| 方法 | 路径 | 认证 | 说明 |
| --- | --- | --- | --- |
| POST | `/v1/billing/checkout` | 是 | 创建结账会话 |
| POST | `/api/v1/billing/checkout` | 是 | 同上 |
| POST | `/v1/billing/portal` | 是 | 创建账单门户会话 |
| POST | `/api/v1/billing/portal` | 是 | 同上 |

当前实现更偏预览和手动订单流程，正式支付供应商接入需按业务计划扩展。

## 8. 上传与异步转换

| 方法 | 路径 | 认证 | 说明 |
| --- | --- | --- | --- |
| POST | `/v1/uploads` | 是 | multipart 上传项目 ZIP |
| POST | `/api/v1/uploads` | 是 | 同上 |
| GET | `/v1/conversions` | 是 | 查询转换任务列表 |
| POST | `/v1/conversions` | 是 | 创建转换任务 |
| GET | `/api/v1/conversions` | 是 | 同上 |
| POST | `/api/v1/conversions` | 是 | 同上 |
| GET | `/v1/conversions/:id` | 是 | 查询单个转换任务 |
| GET | `/api/v1/conversions/:id` | 是 | 同上 |
| GET | `/v1/conversions/:id/download/docx` | 是 | 下载结果 DOCX |
| GET | `/api/v1/conversions/:id/download/docx` | 是 | 同上 |
| GET | `/v1/conversions/:id/report` | 是 | 查询转换报告 |
| GET | `/api/v1/conversions/:id/report` | 是 | 同上 |
| GET | `/v1/conversions/:id/quality-report` | 是 | 查询多维质量报告 |
| GET | `/api/v1/conversions/:id/quality-report` | 是 | 同上 |
| GET | `/v1/conversions/:id/download/zip` | 是 | 下载原始 ZIP |
| GET | `/v1/conversions/:id/download/log` | 是 | 下载转换日志 |

上传要求：

- multipart 字段名：`file`
- 文件内容：`.zip`
- 服务端会检查请求体大小、文件数量、单文件大小、总解压大小和路径安全。

创建转换请求：

```json
{
  "upload_id": "uuid",
  "main_tex": "main.tex",
  "profile": "auto",
  "quality": "standard",
  "engine": "semantic-engine",
  "idempotency_key": "optional-client-key"
}
```

支持的 engine：

- `semantic-engine`
- `semantic`
- `auto`
- `legacy-rule`
- `doc-core`

## 9. 本地转换额度

| 方法 | 路径 | 认证 | 说明 |
| --- | --- | --- | --- |
| POST | `/v1/local-conversions/check` | 是 | 检查本地转换额度 |
| POST | `/api/v1/local-conversions/check` | 是 | 同上 |
| POST | `/v1/local-conversions/consume` | 是 | 消耗本地转换额度 |
| POST | `/api/v1/local-conversions/consume` | 是 | 同上 |

## 10. 同步转换兼容接口

| 方法 | 路径 | 认证 | 说明 |
| --- | --- | --- | --- |
| POST | `/api/v1/convert` | 否 | multipart 上传 ZIP 并同步返回 DOCX |

请求：

- multipart 文件字段：`file`
- 表单字段：`main_tex`，未传时默认 `main-jos.tex`

响应：

- 成功：`application/vnd.openxmlformats-officedocument.wordprocessingml.document`
- 失败：JSON 或错误响应

## 11. 用户反馈

| 方法 | 路径 | 认证 | 说明 |
| --- | --- | --- | --- |
| GET | `/v1/feedback/threads` | 是 | 查询当前用户反馈线程 |
| POST | `/v1/feedback/threads` | 是 | 创建反馈线程 |
| GET | `/v1/feedback/threads/:id` | 是 | 查询反馈线程详情 |
| POST | `/v1/feedback/threads/:id/messages` | 是 | 追加反馈消息 |

创建反馈请求：

```json
{
  "conversion_job_id": "optional-job-id",
  "title": "转换结果异常",
  "feedback_type": "issue",
  "content": "请查看附件或任务日志",
  "priority": "normal"
}
```

## 12. 发布版本

| 方法 | 路径 | 认证 | 说明 |
| --- | --- | --- | --- |
| GET | `/v1/releases/:channel` | 否 | 查询指定渠道最新发布 |
| GET | `/api/v1/releases/:channel` | 否 | 同上 |

常见 channel：

- `stable`
- `beta`
- `preview`

## 13. 管理端基础接口

所有 `/admin/v1` 接口都要求管理员 token。

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/admin/v1/me` | 当前管理员信息和权限 |
| GET | `/admin/v1/dashboard` | 管理看板摘要 |
| GET | `/admin/v1/users` | 用户列表 |
| GET | `/admin/v1/usage-ledger` | 用量流水 |
| GET | `/admin/v1/manual-orders` | 手动订单列表 |
| POST | `/admin/v1/manual-orders` | 创建手动订单 |
| GET | `/admin/v1/waitlist` | 等候名单 |

## 14. 管理端兑换码接口

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/admin/v1/redeem-code-batches` | 批次列表 |
| POST | `/admin/v1/redeem-code-batches` | 创建兑换码批次 |
| GET | `/admin/v1/redeem-code-batches/:id` | 批次详情 |
| GET | `/admin/v1/redeem-code-batches/:id/export.xlsx` | 导出批次 Excel |
| GET | `/admin/v1/redeem-codes` | 兑换码分页列表 |
| POST | `/admin/v1/redeem-codes` | 批量上货 |
| GET | `/admin/v1/redeem-codes/export.xlsx` | 导出兑换码 Excel |
| POST | `/admin/v1/redeem-codes/restock` | 重置兑换码为未使用 |

创建批次请求：

```json
{
  "package_id": "count_10",
  "quantity": 100,
  "channel": "web",
  "note": "preview batch",
  "expires_at": null,
  "auto_provision": false
}
```

## 15. 管理端反馈接口

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/admin/v1/feedback/threads` | 查询反馈线程 |
| GET | `/admin/v1/feedback/threads/export.xlsx` | 导出反馈 Excel |
| PATCH | `/admin/v1/feedback/threads/:id` | 更新状态、优先级、负责人 |
| POST | `/admin/v1/feedback/threads/:id/messages` | 管理员回复 |

## 16. 管理端发布接口

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/admin/v1/releases` | 发布列表 |
| POST | `/admin/v1/releases` | 发布新版本 |
| POST | `/admin/v1/releases/:id/rollback` | 回滚发布 |
| GET | `/admin/v1/release-audit` | 发布审计日志 |

发布请求核心字段：

```json
{
  "channel": "stable",
  "platform": "windows",
  "arch": "x64",
  "version": "0.1.0",
  "download_url": "https://releases.example/Tex2Doc.exe",
  "sha256": "...",
  "signature": "",
  "file_size_bytes": 12345678,
  "release_title": "Tex2Doc 0.1.0",
  "release_notes": "Initial release",
  "strategy_type": "full"
}
```

## 17. 管理端自动化研发接口

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/admin/v1/automation/summary` | 自动化请求摘要 |
| GET | `/admin/v1/automation/requests` | 请求列表 |
| GET | `/admin/v1/automation/requests/:id` | 请求详情 |
| GET | `/admin/v1/automation/requests/:id/events` | 请求事件 |
| POST | `/admin/v1/automation/requests/:id/approve` | 审批 |
| POST | `/admin/v1/automation/requests/:id/reject` | 拒绝 |
| POST | `/admin/v1/automation/requests/:id/retry` | 重试 |
| POST | `/admin/v1/automation/requests/:id/escalate` | 升级给人工 |
| GET | `/admin/v1/automation/agents` | Agent 列表 |
| POST | `/admin/v1/automation/agents/:id/pause` | 暂停 Agent |
| POST | `/admin/v1/automation/agents/:id/resume` | 恢复 Agent |

拒绝请求：

```json
{
  "reason": "需求不完整"
}
```

升级请求：

```json
{
  "assignee": "human"
}
```

