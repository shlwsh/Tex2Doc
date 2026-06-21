# Semantic TeX Engine 当前开发进展报告

**时间戳**：20260622-022137
**基准计划**：`docs-zh/plan-0621.md`
**当前阶段**：P5-P9 持续推进
**报告范围**：当前工作区真实进展、已验证能力、本轮新增收口、剩余工作清单

---

## 一、总体结论

当前项目已经完成 P0-P4，P5-P9 均处于持续开发状态。其中 P5 桌面客户端已经从 UI 骨架推进到可演示 Preview 闭环，P6/P7 商业 API 与云端 Worker 已具备 preview 合约和内存态转换链路，P8/P9 已具备 nightly regression 与 updater manifest 骨架。

当前仍不能认定 P0-P9 全部完成。主要原因是：

- P5 桌面端还缺 GUI 手工矩阵验收、任务详情页、report 打开入口、拖拽、安装包。
- P6 账号/订阅/用量仍是 preview/demo token 和内存态，不是生产级 auth/billing。
- P7 Worker 仍是内存队列和内存 artifact，缺 sandbox、持久化队列、对象存储。
- P8 样本库和质量指标已启动，但未达到商业化大样本质量基准。
- P9 updater 已能检查 manifest，但缺真实下载、签名验签、安装执行和三平台分发。

---

## 二、计划状态对照

| 阶段 | 计划内容 | 当前状态 | 说明 |
|---|---|---|---|
| P0 | Preview 发布门禁修复 | 已完成 | commercial verify、semantic verify、CLI JSON、fixture 验证已完成 |
| P1 | ActiveProfile 全链路重构 | 已完成 | profile ref、active profile、映射修复已完成 |
| P2 | Profile-aware RuleEngine 接入 | 已完成 | profile 规则接入、报告能力已完成 |
| P3 | QualityGate V2 | 已完成 | 多项质量检查、report 集成已完成 |
| P4 | CLI 产品化 | 已完成 | semantic 子命令、JSON 输出、错误码、verify 已完成 |
| P5 | Slint Desktop MVP | 进行中 | 本地/云端转换、账号、账单、recent jobs、诊断包、更新检查已实现 preview |
| P6 | 账号、订阅、用量 API | 进行中 | preview server/client 端点已实现，生产 auth/billing 未完成 |
| P7 | 云端转换 SaaS Worker | 进行中 | 内存 worker 可转换，sandbox/持久化未完成 |
| P8 | 真实样本回归与质量指标 | 进行中 | realistic fixture 与 nightly regression 已启动 |
| P9 | 自动升级与三平台分发 | 进行中 | manifest 检查已接入桌面端，安装/签名/分发未完成 |

---

## 三、本轮新增收口

本轮重点收口 P5 桌面端 recent jobs 与 report 路径追踪。

### 3.1 JobEntry 记录 report path

修改模块：

```text
crates/desktop-slint/src/app_state.rs
```

`JobEntry` 新增：

```rust
pub report_path: Option<String>
```

设计说明：

- 使用 `#[serde(default)]` 保持旧版 `recent_jobs.json` 可兼容读取。
- `JobUpdate::Succeeded` 从单一输出路径升级为结构体变体，携带 `output_path` 和 `report_path`。
- 成功任务会同时记录 DOCX 路径和 report JSON 路径。

### 3.2 本地转换写出 report JSON

修改模块：

```text
crates/desktop-slint/src/commands.rs
crates/desktop-slint/src/job.rs
```

行为变化：

```text
local convert
  -> output.docx
  -> output.report.json
  -> recent jobs 记录 docx/report 两个路径
```

本地 report 命名规则与云端转换保持一致：

```text
paper.docx -> paper.report.json
```

### 3.3 云端转换记录 report path

修改模块：

```text
crates/desktop-slint/src/main.rs
```

云端转换成功后现在会写入：

```rust
JobUpdate::Succeeded {
    output_path,
    report_path,
}
```

这修复了此前 `JobUpdate::Succeeded(result.docx_path...)` 旧签名残留导致的潜在编译失败。

### 3.4 Recent Jobs 展示 report path

修改模块：

```text
crates/desktop-slint/src/main.rs
```

Recent Jobs 展示格式从：

```text
created_at | status | profile | output | error
```

扩展为：

```text
created_at | status | profile | output | report <report_path> | error
```

当前仍是文本展示，后续需要扩展为任务详情页和打开 report 按钮。

### 3.5 job history 测试兼容

修改模块：

```text
crates/desktop-slint/src/job_history.rs
```

测试 helper 已补齐 `report_path: None`，确保新增字段不会破坏 recent jobs 持久化测试。

---

## 四、GitNexus 影响分析

本轮代码改动前已执行影响分析：

| 目标 | 风险 | 影响范围 |
|---|---|---|
| `crates/desktop-slint/src/main.rs::main` | LOW | impacted count = 0 |
| `recent_jobs_for_ui` | LOW | 1 个直接调用方：`main` |
| `JobUpdate` | LOW | impacted count = 0 |
| `LocalConvertResult` | LOW | 1 个直接调用方：`run_local_convert` |
| `run_local_convert` | LOW | impacted count = 0 |

判断：

- 本轮变更集中在桌面端任务状态与 report 路径展示。
- 未触碰语义转换核心、server worker、docx writer 主渲染路径。
- 影响面为 LOW。

---

## 五、验证结果

已执行：

```bash
cargo fmt -p doc-desktop-slint
cargo test -p doc-desktop-slint job_history -- --nocapture
cargo test -p doc-desktop-slint -- --nocapture
cargo check -p doc-desktop-slint
git diff --check
```

结果：

| 命令 | 结果 |
|---|---|
| `cargo fmt -p doc-desktop-slint` | PASS，仍有项目既有 stable rustfmt 不支持 nightly 配置的 warning |
| `cargo test -p doc-desktop-slint job_history -- --nocapture` | PASS，2 tests |
| `cargo test -p doc-desktop-slint -- --nocapture` | PASS，22 tests |
| `cargo check -p doc-desktop-slint` | PASS |
| `git diff --check` | PASS |

说明：

- 当前 warning 主要来自既有未清理代码，例如 unused import、unused variable、dead code，不是本轮 report path 改动引入的编译错误。
- 桌面端主程序已通过 `cargo check`，旧的 `JobUpdate::Succeeded(...)` 签名残留已清除。

---

## 六、当前工作区状态说明

当前工作区仍有较大范围未提交改动，覆盖：

```text
commercial-api-client
desktop-slint
server
worker_service
docx-writer
docs-zh
examples/journals realistic fixtures
scripts/nightly_regression.sh
```

另外：

- `AGENTS.md`、`CLAUDE.md` 已处于修改状态，不属于本轮主动修改内容。
- 本轮未提交代码，未推送。
- 按项目规则，提交前仍需执行 `gitnexus_detect_changes(scope=all)` 并复核 dirty worktree 的整体风险。

---

## 七、剩余工作清单

### P5 下一步

1. 增加 recent jobs 任务详情页。
2. 增加打开 DOCX、打开 report、打开输出目录按钮。
3. 将诊断包与具体 job 绑定，自动带入 report path。
4. 增加拖拽 project/zip。
5. 完成 Windows/macOS/Linux GUI 手工验收矩阵。
6. 完成安装包前的桌面端发布检查清单。

### P6 下一步

1. 替换 demo token 为 JWT access token。
2. refresh token hash 存储、轮换和撤销。
3. PostgreSQL users/sessions/plans/subscriptions/usage_events。
4. usage ledger 与 quota reservation。
5. Stripe 或等价 billing provider webhook 与幂等。

### P7 下一步

1. 上传、任务、产物落库。
2. DOCX/report/logs 迁移到对象存储。
3. in-memory queue 替换为持久化队列。
4. worker 增加 retry/cancel/timeout/dead letter。
5. sandbox runner：no network、CPU/memory/disk/process/wall-clock 限制。

### P8 下一步

1. 扩充每个 profile 的 realistic/failure/golden fixture。
2. nightly regression 输出 profile 维度统计。
3. 强化 Word/LibreOffice openability 门禁。
4. 增加公式、表格、图片、引用、样式覆盖率指标。

### P9 下一步

1. release manifest 改为可配置源。
2. 接入真实 artifact 下载。
3. 完成 manifest 签名规范和验签。
4. Windows MSI/MSIX、macOS DMG/pkg、Linux AppImage/deb/rpm。
5. updater 安装执行与失败回滚。

---

## 八、当前结论

当前 P5 有实质进展：桌面端本地/云端转换完成后，recent jobs 已能持久化并展示 DOCX 与 report JSON 路径，支持后续任务详情、报告打开和诊断包联动。

但 P0-P9 总目标尚未完成。下一步建议优先继续 P5 收口，不再横向扩展 UI 功能，重点完成：

```text
任务详情页
打开 DOCX/report
job 级诊断包
GUI 手工验收矩阵
```

随后再进入 P6/P7 的生产化改造：auth/billing/usage 持久化、云端 worker sandbox 与 artifact 持久化。
