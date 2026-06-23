# Semantic TeX Engine P5 Desktop Recent Jobs 开发进展报告

**时间戳**：20260622-020230  
**基准计划**：`docs-zh/plan-0621.md`  
**关联阶段**：P5 Slint Desktop MVP  
**本轮目标**：补齐桌面端最近任务历史的跨重启持久化能力，使 P5 的“最近任务历史”不再只是内存态 UI 展示。

---

## 一、当前结论

本轮完成了 P5 桌面端 recent jobs 持久化 preview 能力：

```text
AppState.jobs
  -> 转换完成后保存 recent_jobs.json
  -> 客户端启动时读取 recent_jobs.json
  -> 回填 AppState
  -> UI 初始 Recent Jobs 直接展示历史
```

这使 P5 从：

```text
任务历史只在本次进程内可见，重启后丢失
```

推进到：

```text
最近任务历史可跨重启保留，Preview/PoC 演示时能追踪上一次转换结果
```

---

## 二、实现内容

### 2.1 新增 job history adapter

新增文件：

```text
crates/desktop-slint/src/job_history.rs
```

公开入口：

```rust
load_recent_jobs()
save_recent_jobs(jobs)
```

持久化位置：

```text
ProjectDirs::from("com", "tex2doc", "Tex2Doc")
  -> data_dir()
  -> recent_jobs.json
```

设计边界：

- 保存 JSON，不引入数据库或额外依赖。
- 当前保存 UI 最近任务列表，保持 preview 实现简单。
- 写入失败不阻断转换，只记录 warning。
- 读取失败不阻断启动，只展示空历史。

### 2.2 任务状态恢复策略

客户端上次退出时仍处于 `Pending` 或 `Running` 的任务，在下次启动加载时会转为：

```text
Failed | Interrupted before completion.
```

原因：

- preview 桌面端没有任务恢复机制。
- 继续显示 Running 会误导用户。
- 标记为 Failed 可以让用户明确知道该任务未完成。

### 2.3 main.rs 接入

修改文件：

```text
crates/desktop-slint/src/main.rs
```

新增模块：

```rust
mod job_history;
```

新增行为：

- 启动阶段调用 `job_history::load_recent_jobs()`。
- 将读取出的任务回填到 `AppState`。
- UI 初始化时通过 `recent_jobs_for_ui(&app_state)` 展示历史，而不是固定显示 `No recent jobs.`。
- 本地转换完成后调用 `persist_recent_jobs(&app)` 保存历史。
- 云端转换完成后调用 `persist_recent_jobs(&app)` 保存历史。

---

## 三、验证结果

已执行：

```bash
cargo fmt -p doc-desktop-slint
cargo test -p doc-desktop-slint job_history -- --nocapture
cargo check -p doc-desktop-slint
cargo test -p doc-desktop-slint -- --nocapture
git diff --check
```

结果：

| 命令 | 结果 |
|---|---|
| `cargo fmt -p doc-desktop-slint` | PASS，仍有项目既有 nightly rustfmt 配置 warning |
| `cargo test -p doc-desktop-slint job_history -- --nocapture` | PASS，2 tests |
| `cargo check -p doc-desktop-slint` | PASS |
| `cargo test -p doc-desktop-slint -- --nocapture` | PASS，18 tests |
| `git diff --check` | PASS |

新增测试：

```text
job_history::tests::loaded_running_jobs_are_marked_failed
job_history::tests::trim_keeps_at_most_recent_limit
```

GitNexus 检查：

- 修改前对 `crates/desktop-slint/src/main.rs::main` 执行 upstream 影响分析：impacted count = 0，风险 LOW。
- 全量 dirty worktree `detect_changes(scope=all)` 仍为 HIGH。
- HIGH 来源于当前 P5-P9 未提交工作区整体跨 desktop、commercial API、server、worker、docs 等模块；本轮 recent jobs 主要新增桌面端持久化 adapter。

---

## 四、当前边界

本轮没有完成：

- recent jobs 全量 50 条 UI 展示，目前 UI 仍展示最近 10 条。
- 任务详情页。
- 任务重新下载 DOCX/report。
- 云端任务恢复轮询。
- 任务取消。
- 诊断包导出。
- GUI 手工重启验收。

---

## 五、下一步建议

P5 后续建议：

1. 增加诊断包导出，将 report、job metadata、路径、错误信息打包。
2. 增加 recent jobs 操作入口：
   - 打开输出目录。
   - 打开 report。
   - 复制错误。
3. 增加 GUI 手工验收矩阵，覆盖：
   - 本地转换后重启。
   - 云端转换后重启。
   - 运行中强退后重启。
4. 如需展示 50 条历史，扩展 UI 或新增 Job History 窗口。

P7 后续建议：

1. 服务端任务持久化后，桌面端 recent jobs 可保存云端 job id 并支持恢复轮询。
2. 失败任务可从服务端下载诊断包。
