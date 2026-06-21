# Semantic TeX Engine 整体开发进展报告

**时间戳**：20260622-064213
**基准计划**：`docs-zh/plan-0621.md`
**当前目标**：继续完成 P5 阶段相关开发任务，并同步输出整体开发进展
**当前结论**：P0-P4 已完成；P5 桌面 MVP 已进入可演示闭环并继续收口；P6-P9 仍为 Preview/进行中状态。

---

## 一、总体结论

当前 Tex2Doc / Semantic TeX Engine 已完成商业化 Preview 的核心基础：

- 语义转换核心已具备 RuleBased、XeLaTeX Hook、LuaTeX Node 三路径。
- P0-P4 已完成，CLI、profile、RuleEngine、QualityGate、semantic verify 等核心能力已经落地。
- P5 Slint Desktop 已具备本地转换、云端转换、账号、账单、用量、recent jobs、诊断包、更新检查和 report 打开能力。
- P6/P7 已具备 preview API 与内存态云端 worker。
- P8/P9 已具备 realistic fixture、nightly regression、release manifest 和桌面端更新检查骨架。

但 P0-P9 总目标尚未完成。当前不能标记为全部完成，原因是：

- P5 仍缺 Windows/macOS/Linux 三平台真实 GUI 验收与安装包构建。
- P6 仍是 demo auth / preview billing，没有生产级 JWT、refresh token 存储、PostgreSQL 和支付 webhook。
- P7 仍是内存队列与内存 artifact，没有 sandbox、对象存储和持久化队列。
- P8 真实样本规模和质量 dashboard 尚未达到商业发布标准。
- P9 仍缺真实 artifact 下载、签名验签、安装执行和回滚。

---

## 二、P0-P9 状态对照

| 阶段 | 计划内容 | 当前状态 | 说明 |
|---|---|---|---|
| P0 | Preview 发布门禁修复 | 已完成 | commercial verify、semantic verify、CLI JSON、fixture 验证已完成 |
| P1 | ActiveProfile 全链路重构 | 已完成 | ActiveProfile、ProfileRef、profile 映射修复已完成 |
| P2 | Profile-aware RuleEngine 接入 | 已完成 | build rule engine、profile 规则、报告能力已接入 |
| P3 | QualityGate V2 | 已完成 | 多项质量检查、TOML 配置、report 集成已完成 |
| P4 | CLI 产品化 | 已完成 | JSON 输出、错误码、semantic-verify、quality 选项已完成 |
| P5 | Slint Desktop MVP | 基本完成 / 待平台验收 | 本地转换、报告展示、后台任务、recent jobs、诊断包、Open Report 已完成 |
| P6 | 账号、订阅、用量 API | 进行中 | preview client/server 可用，生产 auth/billing 未完成 |
| P7 | 云端转换 SaaS Worker | 进行中 | 内存 worker 可转换，sandbox/持久化未完成 |
| P8 | 真实样本回归与质量指标 | 进行中 | realistic fixture 和 nightly regression 已启动 |
| P9 | 自动升级与三平台分发 | 进行中 | manifest 检查已接入，安装/签名/分发未完成 |

---

## 三、本轮 P5 新增能力

### 3.1 Open Report 桌面入口

修改文件：

```text
crates/desktop-slint/src/ui/main.slint
crates/desktop-slint/src/main.rs
```

新增 UI：

```text
Open Report
```

行为：

```text
output.docx
  -> 推导 output.report.json
  -> 检查 report 文件存在
  -> 使用系统默认程序打开
  -> UI Status 展示成功/失败
```

意义：

- P5 不再只是把 report 写到磁盘或展示摘要。
- 用户可以从桌面端直接打开完整 JSON report，方便人工审查、PoC 支持和质量问题定位。

### 3.2 诊断包自动包含 compile report

修改文件：

```text
crates/desktop-slint/src/diagnostics.rs
```

诊断包现在会自动推导当前输出文件对应的 report：

```text
paper.docx -> paper.report.json
```

如果 report 文件存在，则写入诊断 zip：

```text
compile-report.json
```

同时 `diagnostics.json` 增加：

```json
{
  "report_path": "...",
  "report_included": true
}
```

意义：

- 用户导出的诊断包不再只有 UI 状态和 recent jobs 文本。
- 支持人员可以直接看到编译 report、profile、quality、compatibility、backend、warnings 等关键数据。
- 为后续“job 级诊断包”和云端支持工单打基础。

### 3.3 本地转换 fixture 验收测试

修改文件：

```text
crates/desktop-slint/src/commands.rs
```

新增测试：

```text
commands::tests::local_convert_writes_docx_and_report_for_generic_fixture
```

测试覆盖：

```text
examples/journals/generic/minimal.tex
  -> desktop commands::run_local_convert
  -> generic-minimal.docx
  -> generic-minimal.report.json
```

断言：

- DOCX 文件存在。
- report JSON 文件存在。
- `docx_bytes > 0`。
- report 路径命名符合 `*.report.json` 规则。

意义：

- P5.3 “本地转换集成”不再只依赖人工操作或 UI wiring。
- 已有可自动化验证的桌面命令层 fixture 测试。

### 3.4 Report 路径规则统一

当前本地转换、云端转换、recent jobs、Open Report、Diagnostics 均统一采用：

```text
<output-stem>.report.json
```

例如：

```text
paper.docx
paper.report.json
```

这使 P5 后续任务详情页可以复用同一套路径推导规则。

---

## 四、P5 当前能力清单

当前 P5 已具备：

- `app_state.rs`：账号、refresh token、用量、任务状态共享状态。
- `commands.rs`：profile 检测、本地转换命令封装、report summary 输出。
- `local_convert.rs`：调用 `SemanticTexEngine` 本地转换，不消耗云端额度。
- `job.rs`：后台任务封装，避免 UI 阻塞。
- `report.rs`：CompileReport 到 UI 摘要。
- `settings.rs`：output、quality、profile、API base URL、release channel 等持久化。
- `job_history.rs`：recent jobs 跨重启持久化。
- `diagnostics.rs`：诊断包导出，包含状态、recent jobs、compile report。
- `desktop_dialog.rs`：目录、zip、输出 DOCX 选择。
- `credential_store.rs`：refresh token 安全存储 preview adapter。
- `desktop_update.rs` / `updater.rs`：release manifest 检查和更新状态展示。
- `main.slint`：账户、账单、更新、项目、选项、转换、报告、状态、recent jobs 的主 UI。

从 `docs-zh/plan-0621.md` P5 角度看：

| P5 子项 | 状态 | 证据 |
|---|---|---|
| P5.1 创建工程模块 | 已完成 | 相关模块均已存在并参与编译 |
| P5.2 扩展 MainWindow UI | 已完成 / 持续增强 | 文件选择、profile、进度、Open Output、Open Report、recent jobs 已存在 |
| P5.3 本地转换集成 | 已完成 | `commands::run_local_convert` 调用 `SemanticTexEngine`，fixture 测试通过 |
| P5.4 Windows/Linux 平台构建 | 部分完成 | Linux `cargo check/test` 通过；Windows/macOS 真实构建和 GUI 验收未执行 |

---

## 五、验证结果

本轮已执行：

```bash
npx gitnexus analyze
cargo fmt -p doc-desktop-slint
cargo test -p doc-desktop-slint diagnostics -- --nocapture
cargo test -p doc-desktop-slint local_convert_writes_docx_and_report_for_generic_fixture -- --nocapture
cargo test -p doc-desktop-slint -- --nocapture
cargo check -p doc-desktop-slint
git diff --check
```

结果：

| 命令 | 结果 |
|---|---|
| `npx gitnexus analyze` | PASS，索引更新到 10,228 nodes / 17,135 edges |
| `cargo fmt -p doc-desktop-slint` | PASS，仍有既有 stable rustfmt 不支持 nightly 配置的 warning |
| `cargo test -p doc-desktop-slint diagnostics -- --nocapture` | PASS，5 tests |
| `cargo test -p doc-desktop-slint local_convert_writes_docx_and_report_for_generic_fixture -- --nocapture` | PASS，1 test |
| `cargo test -p doc-desktop-slint -- --nocapture` | PASS，25 tests |
| `cargo check -p doc-desktop-slint` | PASS |
| `git diff --check` | PASS |

说明：

- 当前 warning 主要来自既有 unused import / unused variable / dead code，不是本轮 P5 report 入口引入的错误。
- `cargo test -p doc-desktop-slint -- --nocapture` 中的 `write_paragraph RUNS` 输出来自现有 DOCX writer 调试打印，不影响测试结果。

---

## 六、GitNexus 检查

本轮执行了 GitNexus 索引更新和影响分析。

影响分析：

| 目标 | 风险 | 影响范围 |
|---|---|---|
| `main` | LOW | 无上游调用 |
| `export_diagnostic_bundle` | LOW | 2 个直接调用方，影响桌面主流程和诊断测试 |
| `run_local_convert` | LOW | impacted count = 0 |
| `find_main_tex` | LOW | 直接影响 `detect_profile`、`run_local_convert`，间接影响 `main` |

变更检测：

```text
gitnexus detect_changes(scope=all)
```

当前整体 dirty worktree 风险仍为 HIGH，原因是当前未提交改动累计跨越：

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

本轮 P5 单点改动集中于桌面端 report 入口、诊断包和本地转换测试，局部影响为 LOW。

---

## 七、当前未完成内容

### P5 剩余

P5 功能层面已接近完成，但还剩商业化发布前的验收项：

1. Windows 原生构建与运行验收。
2. macOS 原生构建与运行验收。
3. Linux GUI 手工点击验收。
4. 任务详情页：
   - 打开 DOCX。
   - 打开 report。
   - 打开诊断包。
   - 复制错误。
   - 重新转换。
5. 拖拽 project/zip。
6. GUI 自动化或半自动化验收脚本。

### P6 剩余

1. JWT access token。
2. refresh token hash 存储、轮换、撤销。
3. PostgreSQL users / sessions / plans / subscriptions / usage_events。
4. usage ledger 与 quota reservation。
5. billing webhook 签名验证与幂等。

### P7 剩余

1. 上传、任务、产物落库。
2. 对象存储保存 DOCX/report/logs。
3. 持久化队列。
4. worker retry/cancel/timeout/dead letter。
5. sandbox：no network、CPU/memory/disk/process/wall-clock 限制。

### P8 剩余

1. 扩充真实样本库。
2. 增加 failure/golden fixture。
3. profile 质量趋势报告。
4. Word/LibreOffice openability 门禁常态化。

### P9 剩余

1. 真实 release artifact 下载。
2. manifest 签名规范与验签。
3. Windows MSI/MSIX。
4. macOS DMG/pkg + codesign + notarization。
5. Linux AppImage/deb/rpm。
6. updater 安装执行和失败回滚。

---

## 八、下一步建议

建议下一轮仍然先收口 P5，避免过早切回 P6/P7 大改：

1. 增加任务详情页或 Recent Jobs 操作区。
2. 将 Open Output / Open Report / Diagnostics 绑定到最近一次成功 job，而不是只依赖当前 output path。
3. 完成 Linux GUI 手工点击验收记录。
4. 为 Windows/macOS 构建准备最小打包脚本或构建文档。

完成上述后，P5 可标记为“功能完成，三平台发布验证待 P9 接管”。随后再进入 P6/P7 的生产化改造。
