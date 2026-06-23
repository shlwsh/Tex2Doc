# Semantic TeX Engine P5 Desktop Diagnostics 开发进展报告

**时间戳**：20260622-020908  
**基准计划**：`docs-zh/plan-0621.md`  
**关联阶段**：P5 Slint Desktop MVP  
**本轮目标**：为桌面客户端补齐第一版诊断包导出能力，支撑 Preview/PoC 阶段的问题排查、失败样本回收和人工支持。

---

## 一、当前结论

本轮完成了 P5 桌面端诊断包导出 preview 能力：

```text
Diagnostics button
  -> 收集当前 UI 状态
  -> 生成 diagnostics zip
  -> 写入 diagnostics.json / status.txt / recent_jobs.txt
  -> UI Status 展示导出路径或错误
```

当前 P5 从：

```text
用户转换失败后只能截图或复制局部错误
```

推进到：

```text
用户可一键导出包含状态、项目路径、输出路径、profile、quality、recent jobs 的诊断包
```

这为后续商业化 PoC 的失败样本回收和支持流程打下基础。

---

## 二、实现内容

### 2.1 新增 diagnostics 模块

新增文件：

```text
crates/desktop-slint/src/diagnostics.rs
```

公开类型：

```rust
DiagnosticInput
DiagnosticError
```

公开入口：

```rust
export_diagnostic_bundle(input)
```

诊断包内容：

```text
diagnostics.json
status.txt
recent_jobs.txt
```

`diagnostics.json` 包含：

- app version
- platform
- arch
- generated_at_unix
- project_path
- output_path
- api_base_url
- profile
- quality
- update_status

### 2.2 诊断包输出位置

路径选择策略：

1. 如果 `output_path` 存在：

```text
<output-parent>/diagnostics/<output-stem>-diagnostics-<timestamp>.zip
```

2. 如果没有 `output_path`，但有 `project_path`：

```text
<project>/output/to-docx/diagnostics/<project-stem>-diagnostics-<timestamp>.zip
```

3. 如果两者都没有：

```text
<current-dir>/tex2doc-diagnostics-<timestamp>.zip
```

说明：

- 文件名 stem 会做轻量 ASCII 安全化。
- zip 写入失败会在 UI Status 中显示错误。
- 当前不复制原始 TeX 文件、图片或 DOCX，避免误把用户论文正文打包进诊断包；后续可在用户确认后加入“完整诊断包”模式。

### 2.3 Slint UI 接入

修改文件：

```text
crates/desktop-slint/src/ui/main.slint
```

新增 callback：

```text
export-diagnostics-clicked(
  project-path,
  output-path,
  api-base-url,
  detected-profile,
  quality-level,
  status-text,
  recent-jobs,
  update-status
)
```

新增按钮：

```text
Diagnostics
```

按钮位置：

```text
Detect Profile / Convert / Cloud Convert / Open Output / Diagnostics
```

### 2.4 main.rs 接入

修改文件：

```text
crates/desktop-slint/src/main.rs
```

新增模块：

```rust
mod diagnostics;
```

新增行为：

- 点击 `Diagnostics` 后同步生成诊断包。
- 成功时在 Status 中显示导出路径。
- 失败时在 Status 中显示错误原因。

---

## 三、验证结果

已执行：

```bash
cargo fmt -p doc-desktop-slint
cargo test -p doc-desktop-slint diagnostics -- --nocapture
cargo check -p doc-desktop-slint
cargo test -p doc-desktop-slint -- --nocapture
git diff --check
```

结果：

| 命令 | 结果 |
|---|---|
| `cargo fmt -p doc-desktop-slint` | PASS，仍有项目既有 nightly rustfmt 配置 warning |
| `cargo test -p doc-desktop-slint diagnostics -- --nocapture` | PASS，4 tests |
| `cargo check -p doc-desktop-slint` | PASS |
| `cargo test -p doc-desktop-slint -- --nocapture` | PASS，22 tests |
| `git diff --check` | PASS |

新增测试：

```text
diagnostics::tests::bundle_path_prefers_output_docx_parent
diagnostics::tests::bundle_path_falls_back_to_project_to_docx
diagnostics::tests::stem_is_sanitized
diagnostics::tests::export_writes_expected_zip_entries
```

GitNexus 检查：

- 修改前对 `crates/desktop-slint/src/main.rs::main` 执行 upstream 影响分析：impacted count = 0，风险 LOW。
- 全量 dirty worktree `detect_changes(scope=all)` 仍为 HIGH。
- HIGH 来源于当前 P5-P9 未提交工作区整体跨 desktop、commercial API、server、worker、docs 等模块；本轮诊断包功能主要新增桌面端辅助模块。

---

## 四、当前边界

本轮没有完成：

- GUI 手工点击验收。
- 复制/上传诊断包到云端支持系统。
- 将编译 report JSON 自动加入诊断包。
- 将 DOCX、TeX 源码、图片资源加入诊断包。
- 支持用户确认后的“完整诊断包”模式。
- 服务端 conversion logs / worker logs 下载。

---

## 五、下一步建议

P5 后续建议：

1. 把本地转换和云端转换生成的 report 路径记录进 job history。
2. 诊断包中加入 report JSON 和 cloud job id。
3. 增加 recent jobs 操作：
   - 打开输出目录。
   - 打开 report。
   - 复制错误。
4. 做 Linux GUI 手工验收：
   - 本地转换失败后导出诊断包。
   - 云端转换失败后导出诊断包。
   - 重启后导出历史任务诊断包。

P7 后续建议：

1. 服务端增加 conversion logs / diagnostic bundle endpoint。
2. 桌面端 recent jobs 保存 cloud job id 后支持下载服务端诊断包。
