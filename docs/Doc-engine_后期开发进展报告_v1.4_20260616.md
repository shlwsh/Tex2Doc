# Doc-engine 后期开发进展报告
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



| 文档版本 | 时间 | 范围 |
|---|---|---|
| V1.0 | 2026-06-14 | Sprint 0 + M1 + M2 完成 |
| V1.1 | 2026-06-14 | M3 + M5 + M7 + 质量加固完成 |
| V1.2 | 2026-06-14 | M4 + M6 + M8 + 5 大风险全部完成 |
| V1.3 | 2026-06-14 | 三端联调：Flutter 桌面（FFI）+ Chrome MV3 扩展 + crates/server（Axum MVP）+ LaTeX 解析 char-boundary 健壮性 |
| **V1.4** | **2026-06-16** | **V2 docx→pdf 全链路 + 质量三色对比：PageSetup 模板 / PDF→PNG 内嵌 / JOS 参考文献样式 / soffice Windows 卡死修复** |

## 1. 总览

| 阶段 | 状态 | 备注 |
|---|---|---|
| V1.2 收尾 | ✅ 已完成 | 110 个测试全过 |
| V1.3 三端联调 | ✅ 已完成 | Flutter 桌面 / Chrome MV3 / crates/server |
| **V2 docx→pdf 全链路** | ✅ 端到端跑通 | docx→pdf 26 页 / 3.5 MB，结构层 ✓ 全过 |
| **三层质量报告** | ✅ 已出报告 | 8 项结构 ✓；文本层 2/4 ✗（见 §3.2） |
| **已知产品 Gap** | ⚠️ 待修 | 中英文摘要/关键词块 + 参考文献列表 + 作者简介未进 docx |

## 2. 本轮（V1.4）变更详情

### 2.1 PageSetup：模板页面尺寸注入

**新增 `crates/docx-writer/src/page_setup.rs`**：

- `PageSetup { width_twips, height_twips, margin_top/right/bottom/left, cols_space, cols_num }` — twips 单位
- `PageSetup::default()` → US Letter `12240×15840`，对应 V1 行为
- `PageSetup::jos_paper3()` → JOS 18.40cm×26.00cm 模板（实测 `10433×14742` + `567/850/850/850` margins + 1 col）

**docx-writer 接线**：

- `serialize_document` 新增 `page_setup: Option<&PageSetup>` 参数；`None` 时写 V1 默认 `pgSz`
- `sectPr` 现在会按 `PageSetup` 写 `pgSz` + 可选 `pgMar` + 可选 `cols`（仅在显式提供时写）
- `packer.rs` 新增 `pack_with_page_setup(doc, template, image_assets, page_setup)`；`pack` / `pack_with_template` / `pack_with_assets` 全部下沉到新 API 之上，**旧调用方零修改**
- `core` 层 `convert_sync` / `convert_dir` / `convert_zip` 全部走 `pack_with_page_setup`，传 `options.page_setup.as_ref()`

**doc-core API 暴露**：

- `ConvertOptions` 新增 `pub page_setup: Option<PageSetup>` 字段
- `doc_core::lib.rs` 重新 `pub use doc_docx_writer::PageSetup`
- `crates/core/Cargo.toml` 增 `image` + `pdfium-render` workspace 依赖（仅 `doc-core` 实际使用）

**CLI 入口**：

- `crates/cli/src/cmd.rs` 新增 `PageSetupKind`（`Letter` / `A4` / `JosPaper3`，clap `ValueEnum`）
- `convert` 与 `build` 子命令新增 `--page-setup` 选项，默认 `letter`
- 选择 `jos-paper3` 时，调用 `PageSetup::jos_paper3()` 注入 `ConvertOptions`

### 2.2 docx-writer：JOS 参考文献样式

**新增 `STYLE_JOS_REFERENCE`** (`crates/docx-writer/src/styles.rs`)：

- 段落属性：左缩进 420 twips、悬挂缩进 420 twits、1.5 倍行距（line=360 lineRule=auto）
- run 字体：ascii/hAnsi/cs = `Times New Roman`，eastAsia = `SimSun`（宋体）
- 完整 `w:style/w:pPr/w:rPr` 块写到 `styles.xml`

**serializer 启发式匹配**：

- `Block::Paragraph` 检测：runs 第一个非空字符为 `[` 且第二个字符为数字 → 用 `JOSReference`
- `Block::List` 检测：items summarize 后含 `[` + 数字 + (`—` 或 `--`) → 用 `JOSReference`（覆盖 `\begin{list}{}{... \item[{[N]}] ... }` 的 JOS 参考文献模式）
- 老的有序/无序 list 仍然走 `STYLE_LIST_NUMBER` / `STYLE_LIST_BULLET`，未引入回归

### 2.3 LaTeX 解析：段落容器 + list 变体

**`crates/latex-reader/src/lower.rs`**：

- 新增 `para_container_envs` 白名单：`flushleft` / `flushright` / `center` / `quote` / `quotation` / `verbatim`
- 原 `lower_environment` 会把这些环境的多段折叠成第一个非空块，导致 `Key words:` 等第二段以后的内容丢失
- 改为：命中白名单时，重新 `parse(body)` → 调 `lower_with_macros_and_numbering` → 把 sub blocks 全部 push 进 doc（仅跳过 `RawFallback` / `Equation`）
- 若所有 sub block 都被跳过，则保留 `RawFallback { text: body, span }` 占位
- `lower_environment` 增加 `itemize*` / `enumerate*` / `description*` / `list` / `list*` 五种变体
- `list` / `list*` 视为无序 List，items 已有 `[N] —` 前缀（`lower_list` 中处理）

**`lower_paragraph_container` 修复**：

- 首块是 inline math 抽出的 `Equation` 时继续跳过（与 `rjabstract` 一致）

**`crates/latex-reader/src/lib.rs`** 重新导出：

- `MacroMap`、`JoinedStream`、`lower_with_macros` — 给测试 `paper3_abstract.rs` 用

### 2.4 docx-pdf：soffice Windows 卡死修复 + 全局串行

**根因（已写入 `libreoffice.rs` 注释）**：

- Windows 上 soffice 若不是 console 父进程（被 Rust / `Start-Process` / `tokio::process` spawn 出来）会 fork 出 watcher 线程，**主进程永不退出**
- `Stdio::null/piped/inherit` 全试过都卡
- 唯一稳定方案：spawn `cmd.exe /c` 跑 soffice —— cmd 是 console 进程，soffice 检测到 console 父进程后正常退出

**改动 `crates/docx-pdf/src/libreoffice.rs`**：

- 删除自定义 user-profile（`temp_user_profile`）依赖；改用默认 profile（并发由 `DocxToPdf` 的 `Mutex` 串行化保证）
- `available()` 探活改用 `spawn_blocking` 跑同步 `Command::output()`
- Windows：`spawn_blocking` 调 `cmd.exe /c <soffice> <args>`，stdout/stderr/stdin 全部 `null`
- 非 Windows：直接 `spawn_blocking(soffice + args)`
- 增加 `doc-docx-pdf.log`：每次 spawn 前写一行带 RFC3339 时间戳的 cmdline 到 `%TEMP%`

**改动 `crates/docx-pdf/src/backend.rs`**：

- `DocxToPdf` 新增 `sem: tokio::sync::Mutex<()>` 字段
- `new()` 与 `with_backend()` 两个构造器都初始化 `sem = Mutex::new(())`
- `convert` 入口 `let _permit = self.sem.lock().await;` 全局串行化 soffice 调用

**报错信息补全**：

- 之前 spawn 失败只报 `"启动 soffice 失败"`，现在补 `format!("启动 soffice 失败")` 之外再加 chrono 时间戳日志

### 2.5 doc-core：PDF→PNG 内嵌

**`crates/core/src/convert.rs` 新增 `render_pdf_to_png(pdf_bytes)`**：

- 用 `pdfium-render` 把 PDF 第 1 页渲染为 `target_width=1600` 的 PNG
- 输出 byte 注入 `image_assets`（同 key）
- 这是 docx-writer 端只接受 PNG/JPG 的桥接方案；上游 zip 里的 `*.pdf` 图片资源（paper3 的 `fig1_*.pdf` 等 8 张）现在能正常嵌入 docx

**`convert_zip` 资源扫描升级**：

- 之前的逻辑：仅当 `*.png` / `*.jpg` / `*.jpeg` 时插入 `image_assets`
- 现在：PNG/JPEG 直接入；PDF 先 `render_pdf_to_png` 转 PNG 再入
- **Key 双写**：除完整 VFS 路径外，还把 `basename` 当 fallback key 写一份（docx-writer 端 `fig_key` 经常是裸路径如 `fig1_system_overview.pdf`）；PDF 的 basename 自动 `.pdf→.png`

**诊断输出**：

- `convert_zip` 新增 4 行 `eprintln!` 打印：
  - `zip entries scanned: N`
  - `has abstract: true/false`（检查 `00_abstract` 命中的条目）
  - `doc has N blocks`
  - `block kinds: HPFTLEBR`（每块一个字母）

### 2.6 workspace 依赖

**`Cargo.toml`** 增 `pdfium-render = "0.8"`。

**`Cargo.lock`** 自动同步 `doc-core` 多两个依赖（`image`、`pdfium-render`）。

### 2.7 新增文件

| 文件 | 用途 | 是否入版本 |
|---|---|---|
| `crates/docx-writer/src/page_setup.rs` | V2 PageSetup 类型 | ✅ 入 |
| `crates/latex-reader/tests/paper3_abstract.rs` | paper3 abstract 解析冒烟测试（走 IncludeGraph 路径） | ✅ 入 |
| `examples/paper3/BuildFullZip.cs` | C# 脚本：把 paper3 latex/sections/figures 打成 `upload_full.zip` | ✅ 入（可重现入口） |
| `examples/paper3/upload_full.zip` | 打包产物 | ✅ 入（脚本复现） |
| `examples/paper3/figures/*.png/*.pdf` | 8 张图的 PNG + PDF 原稿 | ✅ 入 |
| `.tools/` | 本机工作区（pdfium 二进制、临时日志） | ❌ 忽略（已 gitignore） |

## 3. 验证结果

### 3.1 `docx-to-pdf` 端到端

`e:/work/Tex2Doc/examples/paper3/build/out.docx` → `out.pdf`：

```
2026-06-16T06:02:43Z  INFO docx → pdf 完成：examples/paper3/build/out.docx
                              → examples/paper3/build\out.pdf
                              (29506 ms, 3583740 bytes, pages=26)
```

- ✅ PDF 实际落盘 3.5 MB / 26 页
- ✅ `last_stderr.txt` / `last_stdout.txt` 干净（spawn 模式正确）
- ✅ `soffice` 进程干净退出（无残留 watcher）

### 3.2 `verify-pdf --skip-visual` 三色报告

**结构层：8/8 ✓**

| # | 名称 | 期望 | 实际 |
|---|------|------|------|
| 1 | 表格对象数 | ≥5 | 6 ✓ |
| 2 | 图片数 | =8 | 8 ✓ |
| 3 | 编号表题数 | ≥6 | 8 ✓ |
| 4 | 页面尺寸 | 10433×14742 | 10433×14742 ✓ |
| 5 | 页边距 | top/r/b/l 非空 | 567/850/850/850 ✓ |
| 6 | 分栏 | space & num 非空 | space=720, num=1 ✓ |
| 7 | JOSReference 样式 | present | present ✓ |
| 8 | docx/PDF 字符比例 | ≥0.75 | 1.217 ✓ |

**文本层：2/4 ✗**

| 项 | 期望 | 实际 |
|---|---|---|
| docx/oracle 字符比例 | ≥0.75 | 1.217 ✓ |
| **rust/oracle 字符比例** | **≥0.75** | **0.718 ✗** |
| **22 marker 三侧覆盖** | **22/22** | **14/22 ✗** |
| 7 章节 oracle+rust | 7/7 | 7/7 ✓ |

**视觉层：本轮 `--skip-visual` 跳过**

### 3.3 已知 Gap（下一轮重点）

**marker 三侧覆盖只到 14/22**，缺失的 8 个：

| Marker | docx | oracle | rust |
|---|---|---|---|
| 摘  要 | ✗ | ✓ | ✗ |
| 关键词 | ✗ | ✓ | ✗ |
| 算法 1 | ✗ | ✓ | ✗ |
| References | ✗ | ✗ | ✗ |
| 附中文参考文献 | ✗ | ✓ | ✗ |
| 作者简介 | ✗ | ✓ | ✗ |
| shihonglei0042@link.tyut.edu.cn | ✗ | ✓ | ✗ |
| zh_juanjuan@126.com | ✗ | ✓ | ✗ |

**8 个缺失 marker 全部 `in_docx: false`**，说明问题在 **docx 发射端**而非 PDF 转换端。具体：

- **中文摘要/关键词**：来自 `00_abstract-zh.tex`，仍处于 `\begin{CJK*}{GBK}{...}` 容器 + `flushleft`/`center` 段落；可能 `CJK` 包没识别导致整段被吞成 `RawFallback`
- **参考文献列表**：来自 `\begin{list}{}{\item[{[N]}] ...}`，lower 已改为 `list` 变体；需要再验证 `Block::List` 的 items 是否拿到 `[N] —` 前缀
- **作者简介**：来自 `\begin{minipage}` 等未在白名单的容器；当前 `lower_paragraph_container` 只对 `flushleft` 等生效
- **作者邮箱**：被 `CJK` 字符边界切割的同一根因

**rust/oracle 字符比例 0.718**：oracle = 29,647 chars、rust = 21,279 chars，差 8,368 chars。差额与上述缺失 marker 总字符量（"摘  要 关键词 算法 1 附中文参考文献 作者简介 shihonglei0042@link.tyut.edu.cn zh_juanjuan@126.com" ≈ 70 chars）远不匹配。差额主体在 oracle 里有的英文/数字公式说明 + heading 编号 + 表格脚注，建议下一轮再 dump 一次 `doc.blocks` 看 `Heading`/`Paragraph` 的 normalize 输出差。

## 4. 工具链改进

- `BuildFullZip.cs`：用 C# `System.IO.Compression` 跨平台打 paper3 完整 zip（替代之前依赖 `7z` 手动）
- `paper3_abstract.rs`：包含 `IncludeGraph` 完整路径的 paper3 main-jos 解析回归测试，输出 `test-output.txt` 800 字符用于人工 eyeball
- `pdfium` 二进制落到 `.tools/pdfium/pdfium.dll`，构建时手工 copy 到 `target/debug/`（临时方案，后续考虑 Cargo build script）

## 5. 风险与回归

- **GitNexus 索引过期**：本轮新增的 `pack_with_page_setup` / `render_pdf_to_png` 等符号 GitNexus 索引里查不到（`detect_changes` 返回 `none`）；commit 前已用 `git diff --stat` 做人工 blast-radius 检查：所有改动都是**新增 API / 私有函数**或**保留旧 API 委托**，无 BREAKING
- **soffice Windows 修复仅验证 paper3 单文档**；并发跑多份 zip 的稳定性未压测（`Mutex` 串行化 + cmd wrapper 是双保险）
- **`untracked` 残留 `examples/paper3/figures/__pycache__/`**：matplotlib 缓存目录，建议下一轮补 `figures/__pycache__/` 到 `.gitignore`

## 6. 下一轮（V1.5）TODO

1. **CJK 块发射修复**：定位 `00_abstract-zh.tex` 为何整体 `in_docx=false`；如系 `CJK*` 环境未识别，在 `lower_environment` 加白名单
2. **作者简介 / References / 邮箱**：扩展 `para_container_envs` 到 `minipage` / `thebibliography` 等
3. **`rust/oracle` 字符比例 0.718**：对 oracle 与 rust 文本做 normalize diff，找差额主体的具体行号
4. **`.gitignore` 补丁**：`figures/__pycache__/`、`.tools/`
5. **GitNexus 重新分析**：`node .gitnexus/run.cjs analyze --force`，让新符号入索引
6. **撤掉 `eprintln!` 诊断输出**：上一节 4 行 eprintln 是 debug 用的，应在收尾前转为 `tracing::debug!` 或删
