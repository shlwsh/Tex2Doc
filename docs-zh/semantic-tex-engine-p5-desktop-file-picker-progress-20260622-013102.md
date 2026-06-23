# Semantic TeX Engine P5 Desktop File Picker 开发进展报告

**时间戳**：20260622-013102  
**基准计划**：`docs-zh/plan-0621.md`  
**关联阶段**：P5 Slint Desktop MVP  
**本轮目标**：补齐桌面端“手写路径”之外的项目选择与输出路径选择入口，降低真实用户验证成本。

---

## 一、当前结论

本轮继续推进 P5 桌面客户端产品化，新增轻量跨平台路径选择能力：

- 项目目录选择。
- 项目 zip 文件选择。
- 输出 DOCX 保存路径选择。
- 选择项目后，当输出路径为空时自动生成默认输出路径。
- 保持手动输入路径能力不变。
- 不新增外部 GUI dialog crate，避免在 P5 preview 阶段引入额外依赖风险。

当前 P5 已具备：

```text
手动路径输入
  + 项目目录选择
  + 项目 zip 选择
  + 输出 DOCX Save As
  + 本地转换
  + 云端转换
  + 登录/用量
  + recent jobs
```

P5 仍不能标记 completed，因为还缺真实 GUI 操作验收、拖拽、token 安全存储、注册/退出、billing UI 和三平台安装包验证。

---

## 二、实现内容

### 2.1 新增 desktop dialog adapter

新增文件：

```text
crates/desktop-slint/src/desktop_dialog.rs
```

公开函数：

```rust
pick_project_folder(initial)
pick_project_zip(initial)
pick_output_docx(initial)
```

实现策略：

| 平台 | 调用方式 |
|---|---|
| Windows | PowerShell + `System.Windows.Forms` |
| macOS | `osascript` |
| Linux | 优先 `zenity`，失败后尝试 `kdialog` |

说明：

- 如果系统没有可用 dialog 命令，函数返回 `None`。
- 返回 `None` 时 UI 保持现有手动路径输入，不阻断使用。
- 该实现适合作为 P5 preview 方案；Beta/GA 阶段可替换为 `rfd` 或 Slint 官方稳定文件对话框能力。

### 2.2 Slint UI 扩展

修改文件：

```text
crates/desktop-slint/src/ui/main.slint
```

新增 callback：

```text
choose-project-folder-clicked(string, string)
choose-project-zip-clicked(string, string)
choose-output-clicked(string)
```

新增 UI 控件：

- Project 区域：
  - `Folder` 按钮。
  - `Zip` 按钮。
- Options / Output 区域：
  - `Save As` 按钮。

交互效果：

- `Folder` 选择 TeX 工程目录。
- `Zip` 选择已打包 TeX 工程。
- `Save As` 选择输出 DOCX 路径。

### 2.3 main.rs 接入

修改文件：

```text
crates/desktop-slint/src/main.rs
```

新增模块：

```rust
mod desktop_dialog;
```

新增行为：

- `on_choose_project_folder_clicked`
- `on_choose_project_zip_clicked`
- `on_choose_output_clicked`

项目选择后：

1. 写入 `project-path`。
2. 持久化最近项目路径。
3. 如果当前输出路径为空，自动生成默认输出：

```text
<project>/output/to-docx/<project-name>.docx
```

zip 输入时：

```text
<zip-parent>/output/to-docx/<zip-stem>.docx
```

### 2.4 默认输出路径 helper

新增 helper：

```rust
default_output_for_project(project_path)
```

新增单元测试：

```text
default_output_for_directory_uses_to_docx_folder
default_output_for_zip_uses_parent_to_docx_folder
```

---

## 三、验证结果

已执行：

```bash
cargo fmt -p doc-desktop-slint
cargo test -p doc-desktop-slint default_output -- --nocapture
cargo check -p doc-desktop-slint
```

结果：

| 命令 | 结果 |
|---|---|
| `cargo fmt -p doc-desktop-slint` | PASS，仍有项目既有 nightly rustfmt 配置 warning |
| `cargo test -p doc-desktop-slint default_output -- --nocapture` | PASS，2 tests |
| `cargo check -p doc-desktop-slint` | PASS |

当前仍有 warning：

- `doc-latex-reader`、`doc-docx-writer`、`doc-compiler-engine` 等既有 unused warning。
- `doc-desktop-slint` 的 updater 仍有未接 UI 的 dead code warning。

这些 warning 与 P5/P9 未完成状态一致，本轮没有引入新的编译失败。

---

## 四、当前边界

本轮没有完成：

- GUI 真实点击验收。
- 拖拽目录/zip。
- 打开外部浏览器进入 billing checkout/portal。
- token keychain 存储。
- 注册、退出、refresh token 自动刷新。
- Windows/macOS/Linux 三平台安装包验证。

---

## 五、下一步建议

P5 后续建议按以下顺序继续：

1. 启动本地 preview server，使用桌面端完成一次 paper3 云端转换手工验收。
2. 增加 billing checkout/portal UI。
3. 增加 logout 与 refresh token 调用。
4. 接入系统 keychain，停止把 token 只放在内存态。
5. 增加拖拽 zip/project。
6. 形成 Windows/macOS/Linux GUI 验收矩阵。

P6/P7 后续应优先生产化：

1. PostgreSQL 数据模型和 migration。
2. JWT + refresh token hash。
3. usage event ledger。
4. object storage。
5. sandbox worker。
