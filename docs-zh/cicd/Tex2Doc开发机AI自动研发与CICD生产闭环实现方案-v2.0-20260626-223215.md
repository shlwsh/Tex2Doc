# Tex2Doc 开发机 AI 自动研发与 CICD 生产闭环实现方案

版本：v2.0
时间戳：20260626-223215
日期：2026-06-26
状态：规划方案

## 1. 目标

本方案在“用户反馈到 AI 编程与 CICD 全链路自动化”的基础上，进一步细化一台专用开发机的落地形态：开发机定时从生产服务器拉取数据库并恢复到本地，Codex 类 AI 开发工具自动领取开发任务、完成代码实现和本地集成验证，验证通过后通过 GitHub PR 与现有 CICD 部署到生产环境，并同步通知后台、反馈线程和客户端。

核心目标：

1. 建立接近生产的本地开发验证环境，但不暴露生产密钥。
2. 让 AI 开发工具从统一任务中心自动领取任务，而不是人工复制需求。
3. AI 只在开发分支和隔离 worktree 中工作，不直接改 `main`。
4. 本地预检、PR CI、预发/生产部署形成分层质量门禁。
5. 部署完成后，通过 release manifest、反馈线程和客户端通知闭环告知用户。

## 2. 当前项目可复用基础

### 2.1 反馈与申请来源

现有服务端已经具备用户反馈线程：

- `POST /v1/feedback/threads`
- `GET /v1/feedback/threads`
- `POST /v1/feedback/threads/:id/messages`
- `GET /admin/v1/feedback/threads`
- `GET /admin/v1/feedback/threads/export.xlsx`
- `PATCH /admin/v1/feedback/threads/:id`
- `POST /admin/v1/feedback/threads/:id/messages`

可复用能力：

- 用户问题和需求可以从 `feedback_threads` 进入统一申请中心。
- `conversion_job_id` 可关联转换任务、报告和日志。
- 后台回复能力可作为自动处理进度回写渠道。

### 2.2 发布与客户端升级通知

现有服务端已具备发布查询和管理接口：

- `GET /v1/releases/:channel`
- `GET /admin/v1/releases`
- `POST /admin/v1/releases`
- `POST /admin/v1/releases/:id/rollback`
- `GET /admin/v1/release-audit`

现有客户端基础：

- Slint 桌面端有 `apps/slint-user/src/updater.rs` 和 `desktop_update.rs`，已支持 manifest 解析、版本比较和 SHA-256 校验。
- 后台 Flutter 有 `AdminReleasesPanel`，可维护发布清单并触发回滚。
- 发布 manifest 已包含 `strategy`、`release_title`、`release_notes` 等字段，可承接推荐升级、强制升级和灰度通知。

### 2.3 CICD 基础

现有工作流：

- `.github/workflows/ci.yml`：Rust fmt/clippy、Flutter analyze。
- `.github/workflows/deploy-production.yml`：构建 Rust 服务端和 Flutter 三入口，部署到腾讯云生产服务器。
- `.github/workflows/release-packages.yml`：构建 Windows/Linux 原生包和 Web 静态包。

现有本地预检入口：

```bash
npm run ci:preflight
```

该命令适合作为开发机 AI 实现后的本地集成验证主入口。

## 3. 总体架构

```mermaid
flowchart LR
    P["生产服务器\nPostgreSQL / doc-server"] -->|定时备份/加密传输| D["开发机\n本地 docdb_dev"]
    F["用户反馈/后台录入/GitHub Issue"] --> R["统一任务中心\nautomation_requests"]
    D --> A["AI 开发代理\nCodex-like Agent"]
    R -->|claim lease| A
    A --> W["隔离 worktree\ncodex/auto-* 分支"]
    W --> V["本地集成验证\nci:preflight / DB smoke / E2E"]
    V -->|通过| PR["GitHub PR"]
    PR --> CI["GitHub CI\nfmt / clippy / analyze / tests"]
    CI -->|合并 main| CD["Deploy Production"]
    CD --> PROD["生产环境"]
    PROD --> N["客户端通知\nrelease manifest / 反馈线程 / 后台消息"]
    N --> U["用户端 Web / Slint / Admin"]
```

关键边界：

- 生产服务器只提供只读备份或加密 dump，不给开发机生产写权限。
- 开发机只负责开发、验证、开 PR，不直接部署生产。
- GitHub CI/CD 是进入生产的唯一通道。
- 客户端通知以生产服务端数据库和 release manifest 为准，不由开发机直接通知用户。

## 4. 开发机部署形态

### 4.1 推荐规格

| 项目 | 建议 |
| --- | --- |
| 操作系统 | Ubuntu 22.04/24.04 LTS 优先；Windows 也可用 PowerShell + Task Scheduler |
| CPU | 8 核以上 |
| 内存 | 32GB 以上，最低 16GB |
| 磁盘 | 500GB SSD 以上，预留数据库快照和多个 worktree |
| 网络 | 可稳定访问 GitHub、生产服务器 SSH、AI 服务 |
| 账号 | 单独 `tex2doc-dev` 用户或专用 Windows 用户 |

### 4.2 本地目录规划

```text
/opt/tex2doc-dev/
  repo/
    Tex2Doc/                       # 主仓库
    worktrees/
      request-<id>/                # AI 任务隔离 worktree
  db/
    dumps/                         # 从生产拉取的加密/压缩 dump
    restore-logs/
    sanitize/
  logs/
    ai-agent/
    preflight/
  secrets/
    dev-agent.env                  # 仅开发机可读
```

Windows 可对应为：

```text
D:\tex2doc-dev\
  repo\Tex2Doc\
  repo\worktrees\
  db\dumps\
  logs\ai-agent\
  secrets\dev-agent.env
```

### 4.3 本地服务端口

沿用项目端口约束，开发机建议使用：

| 服务 | 地址 | 端口 |
| --- | --- | --- |
| 本地 doc-server | `127.0.0.1` | `2625` |
| 本地 Admin Web | `127.0.0.1` | `2626` |
| AI agent health | `127.0.0.1` | `2627` |
| 本地对象存储模拟，可选 | `127.0.0.1` | `2628` |
| PostgreSQL | `127.0.0.1` | `5432` |

## 5. 生产数据库同步到开发机

### 5.1 推荐策略

推荐采用“开发机定时拉取”而非“生产主动推送”：

1. 生产服务器创建只读备份用户和受限 SSH 用户。
2. 开发机按计划通过 SSH 触发生产侧 `pg_dump`。
3. dump 通过 SSH 流式传输到开发机。
4. 开发机校验、恢复到 `docdb_dev_next`。
5. 执行脱敏 SQL。
6. 通过数据库 rename/swap 切换到 `docdb_dev`。
7. 重启本地 `doc-server` 或刷新连接池。

优点：

- 生产服务器不需要持有开发机凭据。
- 开发机可以控制恢复窗口。
- dump 不落明文到生产磁盘，或只保留短期加密文件。

### 5.2 生产侧备份账号

生产 PostgreSQL 建议创建只读备份角色：

```sql
CREATE ROLE tex2doc_backup LOGIN PASSWORD 'REPLACE_ME';
GRANT CONNECT ON DATABASE docdb TO tex2doc_backup;
GRANT USAGE ON SCHEMA public TO tex2doc_backup;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO tex2doc_backup;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO tex2doc_backup;
```

生产服务器 SSH 建议创建受限用户：

```bash
sudo adduser --disabled-password --gecos "" tex2doc-backup
sudo mkdir -p /home/tex2doc-backup/.ssh
sudo chown -R tex2doc-backup:tex2doc-backup /home/tex2doc-backup/.ssh
```

`authorized_keys` 可限制 command，只允许执行备份脚本。

### 5.3 开发机拉取脚本

Linux 示例：

```bash
#!/usr/bin/env bash
set -euo pipefail

TS="$(date +%Y%m%d-%H%M%S)"
ROOT="/opt/tex2doc-dev"
DUMP="$ROOT/db/dumps/docdb-$TS.dump"
LOG="$ROOT/db/restore-logs/restore-$TS.log"

mkdir -p "$ROOT/db/dumps" "$ROOT/db/restore-logs"

ssh tex2doc-prod-backup \
  "pg_dump --format=custom --no-owner --no-acl --dbname=\$PROD_DATABASE_URL" \
  > "$DUMP"

createdb docdb_dev_next
pg_restore --clean --if-exists --no-owner --dbname=docdb_dev_next "$DUMP" 2>&1 | tee "$LOG"
psql docdb_dev_next -f "$ROOT/db/sanitize/sanitize_dev.sql"

psql postgres <<'SQL'
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE datname IN ('docdb_dev', 'docdb_dev_old');
DROP DATABASE IF EXISTS docdb_dev_old;
ALTER DATABASE docdb_dev RENAME TO docdb_dev_old;
ALTER DATABASE docdb_dev_next RENAME TO docdb_dev;
SQL
```

Windows PowerShell 示例：

```powershell
$ErrorActionPreference = "Stop"
$ts = Get-Date -Format "yyyyMMdd-HHmmss"
$root = "D:\tex2doc-dev"
$dump = "$root\db\dumps\docdb-$ts.dump"

New-Item -ItemType Directory -Force -Path "$root\db\dumps" | Out-Null
ssh tex2doc-prod-backup "pg_dump --format=custom --no-owner --no-acl --dbname=`$PROD_DATABASE_URL" > $dump
createdb docdb_dev_next
pg_restore --clean --if-exists --no-owner --dbname=docdb_dev_next $dump
psql docdb_dev_next -f "$root\db\sanitize\sanitize_dev.sql"
```

### 5.4 脱敏规则

恢复到开发机后必须脱敏：

```sql
UPDATE app_users
SET email = concat('user+', id::text, '@dev.local'),
    password_hash = 'disabled-in-dev';

UPDATE app_access_tokens
SET revoked_at = now();

UPDATE feedback_messages
SET content = regexp_replace(content, '[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}', '[email]', 'g');

UPDATE conversion_jobs
SET source_zip_key = NULL,
    result_docx_key = NULL,
    result_report_key = NULL,
    result_log_key = NULL;
```

生产数据进入 AI 前还要做二次最小化：

- 只提供问题相关字段。
- 日志截断到必要片段。
- 附件默认不进入 AI，上线前由人工审批。
- token、邮箱、手机号、订单号、支付流水号全部脱敏。

### 5.5 定时机制

推荐频率：

| 环境 | 频率 | 说明 |
| --- | --- | --- |
| 日常开发 | 每天凌晨 03:30 | 覆盖大部分问题复现 |
| 发布前 | 手动触发 | 确保用最新生产数据回归 |
| 故障处理 | 手动触发指定 dump | 避免自动任务覆盖现场 |

Linux systemd timer：

```ini
[Unit]
Description=Tex2Doc dev database refresh

[Timer]
OnCalendar=*-*-* 03:30:00
Persistent=true

[Install]
WantedBy=timers.target
```

Windows Task Scheduler：

```powershell
schtasks /Create /TN "Tex2DocDevDbRefresh" /SC DAILY /ST 03:30 `
  /TR "pwsh -NoProfile -File D:\tex2doc-dev\scripts\refresh-dev-db.ps1"
```

## 6. 统一任务中心

### 6.1 任务来源

任务来源统一进入 `automation_requests`：

| 来源 | 入口 |
| --- | --- |
| 用户反馈 | `feedback_threads` 自动汇总 |
| 后台手工录入 | Admin 自动化申请页面 |
| GitHub Issue | GitHub webhook 或定时同步 |
| CI 失败 | workflow_run webhook 自动生成修复任务 |
| 生产异常 | 日志监控/健康检查事件 |

### 6.2 任务状态

建议状态机：

```text
submitted -> aggregated -> triaged -> approved -> queued_for_dev
queued_for_dev -> claimed -> coding -> local_validating
local_validating -> pr_open -> ci_running -> ready_for_merge
ready_for_merge -> merged -> staging_deployed -> production_deployed
production_deployed -> notified -> closed
```

失败分支：

```text
triaged -> needs_human
claimed -> blocked
local_validating -> local_failed
ci_running -> ci_failed
production_deployed -> rollback_required
```

### 6.3 任务领取模型

为避免多个开发机或多个 agent 同时处理同一任务，需要租约机制。

建议新增字段：

```sql
ALTER TABLE automation_requests
    ADD COLUMN IF NOT EXISTS claimed_by TEXT,
    ADD COLUMN IF NOT EXISTS claim_token TEXT,
    ADD COLUMN IF NOT EXISTS claim_expires_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS dev_branch TEXT,
    ADD COLUMN IF NOT EXISTS worktree_path TEXT,
    ADD COLUMN IF NOT EXISTS local_validation_status TEXT,
    ADD COLUMN IF NOT EXISTS local_validation_log_key TEXT;
```

领取逻辑：

```sql
WITH next_task AS (
    SELECT id
    FROM automation_requests
    WHERE status = 'queued_for_dev'
      AND risk_level IN ('low', 'medium')
    ORDER BY priority DESC, created_at ASC
    FOR UPDATE SKIP LOCKED
    LIMIT 1
)
UPDATE automation_requests r
SET status = 'claimed',
    claimed_by = $agent_id,
    claim_token = gen_random_uuid()::text,
    claim_expires_at = now() + interval '45 minutes',
    updated_at = now()
FROM next_task
WHERE r.id = next_task.id
RETURNING r.*;
```

心跳续租：

- AI agent 每 60 秒调用一次 `POST /admin/v1/automation/requests/:id/heartbeat`。
- 超过 `claim_expires_at` 未续租，任务回到 `queued_for_dev` 或进入 `stale_claim`。
- 同一任务最多自动重试 3 次，超过后转人工。

### 6.4 API 设计

| API | 用途 |
| --- | --- |
| `POST /admin/v1/automation/requests/claim` | 开发机 agent 领取任务 |
| `POST /admin/v1/automation/requests/:id/heartbeat` | 续租任务 |
| `POST /admin/v1/automation/requests/:id/events` | 写入开发事件 |
| `POST /admin/v1/automation/requests/:id/local-validation` | 回写本地验证结果 |
| `POST /admin/v1/automation/requests/:id/pr` | 回写 PR URL 和分支 |
| `POST /admin/v1/automation/requests/:id/notify` | 触发用户/客户端通知 |
| `GET /admin/v1/automation/agent-config` | 拉取 agent 策略和门禁配置 |

agent token 权限必须独立于 admin token，只允许访问自动化任务 API，不允许发布生产版本或操作账务。

## 7. Codex 类 AI 开发代理

### 7.1 Agent 职责

开发机上的 AI agent 负责：

1. 领取任务。
2. 拉取最新 `main`。
3. 创建隔离 worktree 和 `codex/auto-*` 分支。
4. 读取任务包、相关反馈、日志摘要和验收标准。
5. 使用 GitNexus 查询相关执行流和符号。
6. 修改代码前执行 impact 分析。
7. 实现代码、测试和必要文档。
8. 执行本地预检。
9. 通过后 push 分支并创建 PR。
10. 把 PR、验证结果和风险摘要回写任务中心。

### 7.2 Agent 循环

```text
启动
  -> 注册 heartbeat
  -> claim 任务
  -> 创建 worktree
  -> 构建上下文包
  -> AI 生成计划
  -> GitNexus impact 门禁
  -> 实现
  -> 本地验证
  -> detect_changes
  -> push 分支
  -> 创建 PR
  -> 回写状态
  -> 清理 worktree 或保留现场
```

伪代码：

```bash
while true; do
  task="$(tex2doc-agent claim || true)"
  if [ -z "$task" ]; then sleep 60; continue; fi

  tex2doc-agent prepare-worktree "$task"
  tex2doc-agent build-context "$task"
  codex run --task-file ".tex2doc/tasks/$task.json"
  tex2doc-agent run-preflight "$task"
  tex2doc-agent detect-changes "$task"
  tex2doc-agent push-pr "$task"
  tex2doc-agent report "$task"
done
```

### 7.3 分支与 worktree 规范

分支命名：

```text
codex/auto-<request-short-id>-<slug>
```

示例：

```text
codex/auto-a13f9-feedback-export-filter
codex/auto-b72d1-release-manifest-notify
```

worktree：

```bash
git fetch origin main
git worktree add /opt/tex2doc-dev/repo/worktrees/request-a13f9 -b codex/auto-a13f9-feedback-export-filter origin/main
```

规则：

- 一个任务一个分支。
- 一个任务一个 worktree。
- 不复用上一个任务的构建产物作为判断依据。
- 失败现场至少保留 72 小时。

### 7.4 GitNexus 强制门禁

AI 修改任何函数、类、方法前必须执行：

```text
impact({target: "<symbol>", direction: "upstream"})
```

处理规则：

| impact 风险 | 处理 |
| --- | --- |
| LOW | 可自动继续 |
| MEDIUM | 可继续，但 PR 中必须写明影响范围 |
| HIGH | 停止自动编辑，转 `needs_human` |
| CRITICAL | 停止自动编辑，通知管理员 |

提交前必须执行：

```text
detect_changes()
```

PR body 必须包含：

- 改动符号清单。
- 受影响执行流。
- 风险等级。
- 本地验证命令和结果。
- 是否触及数据库、权限、支付、部署和发布模块。

### 7.5 自动化允许范围

可自动处理：

- 文档补充。
- 测试补充。
- 小范围 UI 修复。
- 明确可复现的低风险 bug。
- 非破坏性日志和可观测性增强。
- 后台列表、筛选、导出等局部功能修复。

必须人工审批：

- 支付、兑换码、账户余额。
- 登录、token、权限。
- 数据库结构迁移和数据修复。
- 生产部署脚本。
- Release/rollback 策略。
- 大范围重构。
- GitNexus impact HIGH/CRITICAL。

## 8. 本地集成验证

### 8.1 验证分层

| 层级 | 名称 | 触发 | 目的 |
| --- | --- | --- | --- |
| L0 | 快速语法检查 | 每次代码生成后 | 尽早发现格式和编译错误 |
| L1 | 本地预检 | PR 前必须 | 覆盖主要 Rust/Flutter 检查 |
| L2 | 数据库集成验证 | 涉及服务端/数据时 | 使用恢复后的生产近似数据验证 |
| L3 | PR CI | PR 创建后 | GitHub 环境复核 |
| L4 | 部署验证 | 合并后 | 生产健康检查和客户端通知 |

### 8.2 推荐本地命令

基础预检：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cd flutter_app && flutter analyze
```

完整预检：

```bash
npm run ci:preflight
```

数据库 API 集成：

```bash
export DATABASE_URL=postgres://tex2doc_dev:REPLACE_ME@127.0.0.1:5432/docdb_dev
npm run ci:preflight
```

Windows PowerShell：

```powershell
$env:DATABASE_URL="postgres://tex2doc_dev:REPLACE_ME@127.0.0.1:5432/docdb_dev"
npm run ci:preflight
```

### 8.3 本地服务 smoke test

开发机应在验证前启动本地服务：

```bash
DOC_SERVER_ADDR=127.0.0.1:2625 \
DATABASE_URL=postgres://tex2doc_dev:REPLACE_ME@127.0.0.1:5432/docdb_dev \
cargo run -p doc-server
```

检查：

```bash
curl -fsS http://127.0.0.1:2625/api/v1/health
curl -fsS http://127.0.0.1:2625/v1/releases/stable
```

涉及反馈或自动化任务时，补充：

- 创建测试反馈。
- 后台列表读取。
- 自动化申请创建。
- 任务事件回写。
- 用户反馈线程可见状态更新。

## 9. PR 与 CICD 流程

### 9.1 PR 创建

AI agent 本地验证通过后：

1. push `codex/auto-*` 分支。
2. 创建 PR 到 `main`。
3. 添加 label：`ai-generated`、`automation-request`、`risk-low` 或 `risk-medium`。
4. PR body 写入任务来源、验收标准、影响分析、验证结果。
5. 回写 `automation_requests.pr_url`。

PR 标题建议：

```text
[Auto][REQ-<short-id>] <任务标题>
```

### 9.2 GitHub CI 门禁

建议逐步增强 `.github/workflows/ci.yml`：

短期保留快速门禁：

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `flutter pub get`
- `flutter analyze`

中期增加：

- `cargo test --workspace`
- `flutter test`
- 服务端 API smoke test。
- SQL migration dry-run。

### 9.3 合并规则

自动化 PR 不允许自动合并到生产，必须满足：

- PR CI 全绿。
- `detect_changes` 报告存在。
- HIGH/CRITICAL 风险不存在，或已有人工审批。
- 至少一名维护者 review。
- 涉及发布和数据库时，需要二次审批。

### 9.4 生产部署

合并到 `main` 后沿用现有 `.github/workflows/deploy-production.yml`：

1. `ubuntu-22.04` 构建，避免 glibc 版本高于生产服务器。
2. 构建 `doc-server`。
3. 构建 Flutter home/user/admin 三入口。
4. 产物打包上传。
5. 生产服务器解压到新 release 目录。
6. 切换 `/opt/tex2doc/current`。
7. 重启 `tex2doc-server`。
8. `nginx -t` 并 reload。
9. 调用 `http://127.0.0.1:2624/api/v1/health`。

建议补充：

- 部署成功后调用后端内部 API 写入自动化任务部署事件。
- 部署失败时自动把任务置为 `deploy_failed`。
- 保留最近 5 个 release 的同时，把 release id 写入 `automation_request_events`。

## 10. 客户端通知闭环

### 10.1 通知通道

| 通道 | 目标 | 用途 |
| --- | --- | --- |
| 反馈线程自动回复 | 提交问题的用户 | 告知已修复、已上线、如何验证 |
| Release manifest | Slint 桌面端、未来 Flutter 桌面端 | 版本升级提示 |
| Web 静态页面刷新 | Flutter Web 用户 | 新版本 Web 入口上线后自动获取 |
| 后台通知中心 | 管理员/客服 | 申请处理完成、失败、回滚 |
| 邮件/企业微信，可选 | 内部团队 | 高风险部署、失败、回滚通知 |

### 10.2 反馈线程回写

部署成功后，自动向对应反馈线程追加系统消息：

```text
你的反馈已完成处理并发布到生产环境。

处理结果：<摘要>
上线版本：<release id / commit sha>
验证方式：请刷新 Web 页面或在桌面端检查更新。
如问题仍存在，可在本反馈线程继续回复。
```

状态同步：

- `automation_requests.status = 'notified'`
- `feedback_threads.status = 'resolved'`
- `feedback_messages.sender_type = 'system'`

### 10.3 Release manifest 通知

对于桌面端：

1. CICD 或后台发布管理写入 `release_manifests`。
2. 客户端调用 `GET /v1/releases/stable`。
3. `updater.rs` 判断版本是否更新。
4. UI 显示 release title、release notes、下载地址和策略。

建议在发布时把自动化任务关联到 release notes：

```text
## 修复
- 修复用户反馈 #REQ-a13f9：反馈导出筛选条件未生效。

## 验证
- 通过服务端健康检查。
- 通过本地 ci:preflight。
- 通过 GitHub CI。
```

### 10.4 Web 客户端通知

Flutter Web 部署后用户刷新即可获取新版本。为提升可见性，建议新增：

```text
GET /v1/client-notifications
```

返回：

```json
{
  "items": [
    {
      "id": "release-20260626-223215",
      "type": "release",
      "title": "Tex2Doc 已更新",
      "message": "本次修复了反馈导出筛选问题。",
      "level": "info",
      "created_at": "2026-06-26T22:32:15+08:00"
    }
  ]
}
```

Flutter Web 在用户登录后拉取并显示顶部 banner 或通知中心消息。

## 11. 新增数据表建议

### 11.1 开发 agent 注册表

```sql
CREATE TABLE IF NOT EXISTS automation_agents (
    id TEXT PRIMARY KEY,
    hostname TEXT NOT NULL,
    agent_version TEXT NOT NULL,
    capabilities JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL DEFAULT 'online',
    last_heartbeat_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### 11.2 本地验证记录

```sql
CREATE TABLE IF NOT EXISTS automation_validations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    request_id UUID NOT NULL REFERENCES automation_requests(id) ON DELETE CASCADE,
    run_id UUID REFERENCES automation_runs(id) ON DELETE SET NULL,
    validation_level TEXT NOT NULL,
    command TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('running', 'passed', 'failed', 'skipped')),
    log_key TEXT,
    duration_ms BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at TIMESTAMPTZ
);
```

### 11.3 客户端通知

```sql
CREATE TABLE IF NOT EXISTS client_notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    notification_type TEXT NOT NULL CHECK (notification_type IN ('release', 'feedback', 'maintenance', 'incident')),
    audience TEXT NOT NULL DEFAULT 'all',
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    level TEXT NOT NULL DEFAULT 'info',
    related_request_id UUID REFERENCES automation_requests(id) ON DELETE SET NULL,
    related_release_id UUID,
    active BOOLEAN NOT NULL DEFAULT true,
    starts_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

## 12. 安全设计

### 12.1 密钥隔离

开发机允许持有：

- GitHub fine-grained token，只能 push `codex/auto-*` 分支和创建 PR。
- 后端 automation agent token，只能领取任务和回写任务事件。
- 生产备份只读凭据，不能写生产库。
- AI 服务凭据。

开发机禁止持有：

- 生产 SSH 部署私钥。
- 生产数据库写权限。
- 支付、兑换码、账户余额相关密钥。
- GitHub production environment 审批权限。

### 12.2 生产数据保护

必须做到：

- 开发机数据库只能监听本机。
- dump 文件加密或保留时间不超过 7 天。
- 恢复后立即脱敏。
- AI 输入只取最小必要上下文。
- 任何附件进入 AI 前需人工确认。

### 12.3 AI 行为限制

禁止 AI agent：

- 修改生产服务器。
- 直接合并 PR。
- 直接触发生产部署。
- 修改 GitHub Secrets。
- 绕过 GitNexus impact。
- 在日志中输出密钥、token、完整用户数据。

## 13. 监控与告警

开发机指标：

- 最近一次生产库恢复时间。
- dump 大小、恢复耗时、脱敏耗时。
- agent 在线状态。
- 当前处理任务数。
- 本地验证成功率。
- 平均任务处理时长。

CICD 指标：

- PR CI 成功率。
- 自动化 PR 一次通过率。
- 合并到部署耗时。
- 部署成功率。
- 回滚次数。

通知指标：

- 已通知用户数。
- 反馈线程自动关闭率。
- 用户重开率。
- 客户端更新检查成功率。

## 14. 实施阶段

### P0：开发机基础环境

周期：1-2 天

交付：

- 安装 Rust、Flutter、Node、PostgreSQL、GitHub CLI、GitNexus、Codex 类工具。
- 本地可运行 `npm run ci:preflight`。
- 本地 `doc-server` 可连接 `docdb_dev`。
- 建立目录和密钥权限规范。

验收：

- `cargo fmt`、`cargo clippy`、`flutter analyze` 可运行。
- `curl http://127.0.0.1:2625/api/v1/health` 成功。

### P1：生产库定时恢复

周期：1-2 天

交付：

- 生产只读备份账号。
- 开发机拉取脚本。
- 脱敏 SQL。
- systemd timer 或 Windows Task Scheduler。
- 恢复日志和失败告警。

验收：

- 开发机每天自动恢复一次生产近似数据。
- 恢复后敏感 token 被撤销，邮箱等 PII 被脱敏。

### P2：统一任务领取 API

周期：2-4 天

交付：

- `automation_requests` 完整状态机。
- `automation_agents` 注册与 heartbeat。
- `claim/heartbeat/events/local-validation/pr` API。
- 后台展示任务、agent、事件时间线。

验收：

- 多 agent 并发领取不会重复处理同一任务。
- 租约过期任务可自动回队列。

### P3：AI agent 自动开发

周期：4-7 天

交付：

- 自动创建 worktree 和分支。
- 构造任务上下文包。
- Codex 类工具自动实现。
- GitNexus impact/detect_changes 强制门禁。
- 本地预检通过后创建 PR。

验收：

- 低风险文档/测试/小修复任务可自动生成 PR。
- 本地失败能回写日志和状态。
- HIGH/CRITICAL 风险自动转人工。

### P4：CICD 回写与生产部署闭环

周期：3-5 天

交付：

- GitHub webhook 回写 PR、CI、merge、deploy 状态。
- 部署成功后写入任务事件。
- 部署失败后转 `deploy_failed`。
- release manifest 自动带上任务摘要。

验收：

- PR 创建、CI 失败、CI 通过、合并、生产部署状态准确同步。
- 生产部署仍由 GitHub Actions 和 production environment 控制。

### P5：客户端通知与运营报表

周期：3-5 天

交付：

- 反馈线程自动回复。
- `client_notifications`。
- Web 客户端通知 banner。
- Slint 桌面端升级提示强化。
- 后台自动化处理报表。

验收：

- 用户能在反馈线程看到处理结果。
- 桌面端能通过 release manifest 看到更新。
- Web 端能展示发布通知。

## 15. 推荐优先级

第一批最值得自动化的任务：

1. 文档和方案补充。
2. 单元测试和集成测试补齐。
3. 后台列表筛选、导出、小型 UI 文案修复。
4. release manifest 展示和通知增强。
5. 反馈线程状态回写。

暂缓自动化的任务：

1. 支付和账户余额。
2. 生产部署脚本重构。
3. 数据库结构破坏性迁移。
4. 大规模编译引擎重构。
5. 客户端安装器自动执行。

## 16. 最小可行闭环

建议先实现一个最小闭环：

```text
用户反馈
  -> 后台生成 automation_request
  -> 管理员批准低风险任务
  -> 开发机 agent 领取
  -> AI 修改文档或测试
  -> npm run ci:preflight
  -> 创建 PR
  -> GitHub CI
  -> 人工合并
  -> deploy-production.yml
  -> 反馈线程自动回复
```

该闭环不需要一开始就让 AI 修改核心业务代码，也不需要自动合并；先证明任务领取、实现、验证、PR、部署、通知的状态链路稳定，再逐步扩大自动化范围。

## 17. 结论

开发机的价值不是替代 CICD，而是把“AI 编程”放到受控、可复现、可审计的工程环境里。生产数据库定时恢复让问题复现更接近真实场景；任务中心和租约机制让 Codex 类工具可以自动领取工作；GitNexus impact、`npm run ci:preflight`、PR CI 和生产部署门禁保证改动不会绕过工程流程；最后通过 release manifest、反馈线程和客户端通知把结果回到用户侧。

推荐按 P0 到 P5 分阶段实施，先做低风险任务闭环，再逐步接入更复杂的服务端修复、客户端通知和发布策略自动化。
