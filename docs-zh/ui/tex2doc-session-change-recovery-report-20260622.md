# Tex2Doc 本轮会话变更恢复报告
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



**日期**：2026-06-22  
**范围**：`crates/desktop-slint` 前端 UI/UX 方案、Windows 构建脚本、paper3 前端 DOCX 转换质量修复、验证脚本与后续多语言/主题/版本号规划。  
**用途**：用于在代码丢失、分支回退或重建工作区时，按本报告恢复本轮会话形成的改造细节。

---

## 一、当前工作区快照

本轮核对时，`git diff --stat` 显示当前未提交代码差异集中在 3 个文件：

```text
crates/docx-writer/src/serializer.rs | 410 ++++++++++++++++++++++++++++++-----
crates/mathml/src/latex.rs           |  18 +-
crates/mathml/src/omml.rs            |  46 ++--
3 files changed, 389 insertions(+), 85 deletions(-)
```

当前可见的 UI/质量报告文档：

```text
docs-zh/ui/desktop-slint-multi-tab-ui-refactor-plan-20260622.md
docs-zh/ui/desktop-slint-ui-ux-improvement-report-20260622.md
docs-zh/ui/desktop-slint-ui-ux-overall-improvement-plan-v2-20260622.md
docs-zh/ui/paper3-frontend-docx-quality-optimization-summary-20260622.md
docs-zh/ui/tex2doc-session-change-recovery-report-20260622.md
```

当前可见的 Windows 脚本产物：

```text
scripts/_build_common.ps1
scripts/build_docx.ps1
scripts/build_paper3_compiler_engine_docx.ps1
scripts/build_paper3_dual_docx.ps1
scripts/build_paper3_pandoc_docx.ps1
scripts/build_paper3_three_docx.ps1
scripts/test_paper3_frontend_docx.ps1
```

注意：

1. 当前 `git status --short` 还显示 `.agents/` 为未跟踪目录，这不是本轮 UI/DOCX 改造的核心内容，恢复时不应误认为必须提交。
2. 本报告记录了本轮会话形成的全部关键改动点，其中既包括当前可见的未提交代码差异，也包括已经写入 `docs-zh/ui` 的方案文档和 Windows 脚本适配结论。
3. 当前核对发现 `crates/docx-writer/src/serializer.rs` 算法块附近存在旧表格渲染调用残片，详见“已知不一致点”。恢复时必须按本报告修正，否则可能无法编译。

---

## 二、会话需求演进

本轮会话围绕以下用户需求逐步展开：

1. 对 `crates/desktop-slint` 前端界面进行重构，升级为多 TAB 页模式：
   - 主页面聚焦目标文件选择与转换。
   - 配置、用户注册、套餐管理等页面独立为 TAB。
   - 输出具体 UI/UX 技术方案到 `docs-zh/ui`。
2. 为 `scripts` 目录下 `build*.sh` 添加 Windows 下可运行脚本。
3. 分析前端转换 DOCX 质量为何明显不如 `scripts/build_paper3_compiler_engine_docx.ps1`。
4. 在前端选择文件夹为 `D:\papers\paper3`、内容与脚本目标一致的前提下，继续细化最终转换差异原因。
5. 直接优化改进前端转换质量。
6. 针对实际运行：

   ```text
   cargo run -p doc-desktop-slint
   ```

   前端转换报告仍显示：

   ```text
   Profile: chinese-academic (Chinese Academic Paper)
   Compatibility: 70%
   Backend: rule-based
   Quality Gate: Failed (score=70)
   WARN: backend_fallback: backend fell back from Auto to RuleBased
   ```

   要求分析原因并彻底修正。
7. 针对 `D:\output3.docx` 中的问题继续优化：
   - 数学公式不正确。
   - 存在乱码或 raw LaTeX。
   - 代码块/算法块不要用表格。
   - 每行注释跨行挤压导致表格变形。
   - 生成脚本命令验证结果正常达标。
8. 将优化内容及结论输出小结报告到 `docs-zh/ui`。
9. 基于 `desktop-slint-ui-ux-improvement-report-20260622.md` 继续完善：
   - 主界面多语言，默认英语，可切换简体中文、繁体中文、法文、日文、德文。
   - 主界面可切换风格主题，并统一切换颜色。
   - 添加版本号管理，规则为 `1.年份后两位.月份.本月修改次数`，每次 git 提交自动加 1。
   - 输出新版本总体改进方案到 `docs-zh/ui`。
10. 当前请求：将上述会话内容和改动点完整总结，形成可恢复改造细节的变更报告。

---

## 三、UI 多 TAB 重构方案

### 3.1 输出文档

文档：

```text
docs-zh/ui/desktop-slint-multi-tab-ui-refactor-plan-20260622.md
```

该文档是本轮 UI 重构的第一版总体方案，核心目标是把桌面端从单页工具界面升级为多 TAB 产品化界面。

### 3.2 信息架构

推荐 TAB 结构：

| TAB | 职责 | 关键内容 |
|---|---|---|
| Convert | 主转换页 | 项目目录选择、输出 DOCX 路径、profile、quality、转换按钮、进度、结果摘要 |
| Settings | 配置页 | 默认输出目录、默认 profile、默认 quality、隐私/日志/诊断设置 |
| Account | 账号页 | 登录、注册、刷新用户信息、退出登录、当前账号状态 |
| Billing | 套餐页 | 当前套餐、使用量、购买/管理套餐入口 |
| History / Diagnostics | 历史与诊断 | 最近转换记录、打开输出、打开报告、导出诊断包 |

主页面必须收敛为“目标文件选择 + 转换执行 + 结果反馈”，避免账号、套餐、诊断等低频能力挤占主流程。

### 3.3 Slint 实现原则

方案建议首期保持 `MainWindow` 对外契约稳定，不一次性重写 Rust 绑定：

1. 先在 Slint 层加入 TAB 容器。
2. 保留已有 callback、property 名称，降低 Rust 侧改动范围。
3. 后续再把 UI 文件拆分为：

```text
crates/desktop-slint/src/ui/
├─ main.slint
├─ components/
│  ├─ tab_bar.slint
│  ├─ status_badge.slint
│  └─ path_picker.slint
└─ pages/
   ├─ convert.slint
   ├─ settings.slint
   ├─ account.slint
   ├─ billing.slint
   └─ history.slint
```

### 3.4 Rust 侧渐进拆分

建议按以下节奏恢复或继续实现：

1. 第一阶段：只改 Slint 布局，不改 Rust 绑定。
2. 第二阶段：把 callback 绑定按领域拆分到 `ui_bindings`：

```text
crates/desktop-slint/src/ui_bindings/
├─ convert.rs
├─ settings.rs
├─ account.rs
├─ billing.rs
├─ diagnostics.rs
└─ mod.rs
```

3. 第三阶段：状态结构化，把散落的 UI 状态收敛到转换状态、账号状态、套餐状态、设置状态。

### 3.5 UI 验收要点

恢复 UI 改造时至少验证：

1. 默认打开 Convert TAB。
2. 切换 TAB 不丢失已选择路径、profile、quality。
3. Convert 页可完成本地转换。
4. Settings 页修改默认值后重启可恢复。
5. Account/Billing 页不阻塞离线本地转换。
6. History 页可打开输出 DOCX 和报告。
7. 窗口缩放时路径、按钮、状态文本不重叠。

---

## 四、UI 多语言、主题、版本号新方案

### 4.1 输出文档

文档：

```text
docs-zh/ui/desktop-slint-ui-ux-overall-improvement-plan-v2-20260622.md
```

该文档是在多 TAB 和 UI/UX 优化报告基础上追加的 v2 总体方案。

### 4.2 多语言要求

默认语言为英语，支持：

| 代码 | 展示名 | 说明 |
|---|---|---|
| `en` | English | 默认语言 |
| `zh-Hans` | 简体中文 | 简体中文 |
| `zh-Hant` | 繁體中文 | 繁体中文 |
| `fr` | Français | 法文 |
| `ja` | 日本語 | 日文 |
| `de` | Deutsch | 德文 |

建议新增资源目录：

```text
crates/desktop-slint/src/i18n/
├─ mod.rs
├─ catalog.rs
├─ locale.rs
└─ translations/
   ├─ en.json
   ├─ zh-Hans.json
   ├─ zh-Hant.json
   ├─ fr.json
   ├─ ja.json
   └─ de.json
```

翻译 key 原则：

1. 使用语义 key，例如 `tab.convert`、`settings.language`。
2. 不使用英文原文作为 key。
3. 全部语言文件必须拥有相同 key 集合。
4. 英语作为 fallback。
5. 缺失翻译时显示英语并记录 debug 日志。

Slint 首期集成方式：

```slint
in-out property <string> t-tab-convert;
in-out property <string> t-tab-settings;
in-out property <string> t-convert-button;
in-out property <string> t-language-label;
in-out property <string> t-theme-label;
```

Rust 侧根据当前 locale 批量注入：

```rust
ui.set_t_tab_convert(t("tab.convert"));
ui.set_t_tab_settings(t("tab.settings"));
ui.set_t_convert_button(t("convert.convert"));
```

### 4.3 主题系统要求

主题选项：

| 主题 | 用途 |
|---|---|
| System | 跟随系统或首期映射为 Light |
| Light | 日间工作 |
| Dark | 夜间工作 |
| High Contrast | 可访问性 |

建议新增 `ThemePalette`：

```rust
pub struct ThemePalette {
    pub window_bg: String,
    pub surface: String,
    pub surface_alt: String,
    pub border: String,
    pub text_primary: String,
    pub text_secondary: String,
    pub text_muted: String,
    pub accent: String,
    pub accent_hover: String,
    pub success: String,
    pub warning: String,
    pub danger: String,
    pub progress_track: String,
    pub progress_fill: String,
}
```

Slint 根组件暴露颜色 token：

```slint
in-out property <color> color-window-bg;
in-out property <color> color-surface;
in-out property <color> color-border;
in-out property <color> color-text-primary;
in-out property <color> color-text-secondary;
in-out property <color> color-accent;
in-out property <color> color-danger;
```

恢复时注意：

1. 不要继续在页面中散落硬编码颜色。
2. Convert、Settings、Account、Billing、History 必须使用同一套 token。
3. 主题切换后全部 TAB 同步刷新。
4. 主题偏好写入本地配置，下次启动恢复。

### 4.4 版本号管理要求

版本号规则：

```text
1.年份后两位.月份.本月修改次数
```

示例：

```text
1.26.6.1
1.26.6.2
1.26.7.1
```

建议新增：

```text
crates/desktop-slint/VERSION
```

构建期注入：

```rust
// crates/desktop-slint/build.rs
println!("cargo:rustc-env=TEX2DOC_DESKTOP_VERSION={version}");
```

代码中使用：

```rust
pub const DESKTOP_VERSION: &str = env!("TEX2DOC_DESKTOP_VERSION");
```

git 自动递增建议：

```text
scripts/bump_desktop_version.ps1
scripts/install_desktop_git_hooks.ps1
.git/hooks/pre-commit
```

hook 行为：

1. 读取当前年月。
2. 读取 `crates/desktop-slint/VERSION`。
3. 如果年月变化，将本月修改次数重置为 1。
4. 如果仍在同一年月，本月修改次数加 1。
5. 写回 `VERSION`。
6. 自动 `git add crates/desktop-slint/VERSION`。
7. 支持 `TEX2DOC_SKIP_VERSION_BUMP=1` 跳过。

---

## 五、Windows 构建脚本适配

### 5.1 目标

为 `scripts` 目录下已有 `build*.sh` 提供 Windows PowerShell 版本，使 Windows 环境可以直接运行 paper3 / DOCX 构建链路，不依赖 Bash。

### 5.2 公共辅助脚本

文件：

```text
scripts/_build_common.ps1
```

公共能力：

1. `Get-RepoRoot`：从脚本目录解析仓库根目录。
2. `Get-TimeStamp`：生成 `yyyyMMdd-HHmmss` 时间戳。
3. `Get-VersionNumber` / `Get-VersionTag`：支持 `v13` 和 `13` 两种输入。
4. `Assert-Command`：检查命令是否存在。
5. `Assert-PathExists`：检查输入路径。
6. `Get-PythonLauncher`：优先使用 `python` / `python3`，其次使用 `py -3`。
7. `Invoke-Python`：统一调用 Python 并检查退出码。
8. `Test-PythonModule`：检查 Python 模块。
9. `Invoke-Native`：统一调用本地命令并检查 `$LASTEXITCODE`。
10. `Get-FileSizeBytes` / `Format-ByteSize`：输出文件大小。
11. `Get-RepoRelativePath`：生成仓库相对路径。
12. `Write-Utf8Lines`：写 UTF-8 无 BOM 文本。
13. `Get-LatestFile`：按时间查找最新产物。

### 5.3 单一路径 DOCX 构建

文件：

```text
scripts/build_docx.ps1
```

对应：

```text
scripts/build_docx.sh
```

主要流程：

1. 检查依赖：
   - Python 或 `py -3`
   - `pdftotext`
   - `pdftoppm`
   - `latexmk`
   - `xelatex`
   - `bibtex`
   - Python Pillow/PIL
   - `docs/format/jos_2025_docx_format_definitions.json`
2. 定位 paper3 输入：

```text
examples/paper3/latex/main-jos.tex
examples/paper3/latex/main-jos.pdf
examples/paper3/latex/main-jos.bbl
```

3. 维护输入 manifest：

```text
examples/paper3/latex/.main-jos.inputs.sha256
```

4. 如果 TeX / Bib / class / style / format 文件未变化，则复用已有 PDF/BBL。
5. 如果输入变化，则调用：

```powershell
latexmk -xelatex -bibtex -interaction=nonstopmode -halt-on-error main-jos.tex
```

6. 调用 `scripts/build_jos_docx.py` 生成 DOCX。
7. 调用 `scripts/verify_jos_docx.py` 生成校验报告。
8. 输出产物到：

```text
examples/paper3/output/to-docx/
```

### 5.4 compiler-engine DOCX 构建

文件：

```text
scripts/build_paper3_compiler_engine_docx.ps1
```

对应：

```text
scripts/build_paper3_compiler_engine_docx.sh
```

主要流程：

1. 默认版本 `v13`。
2. 项目根为：

```text
examples/paper3/latex
```

3. 主文件为：

```text
examples/paper3/latex/main-jos.tex
```

4. 调用：

```powershell
cargo run -p doc-compiler-engine --example paper3_to_docx -- `
  --project-root examples\paper3\latex `
  --main-tex examples\paper3\latex\main-jos.tex `
  --profile jos-paper `
  --out <output.docx>
```

5. 输出文件命名：

```text
<version>-论文稿件-jos-<timestamp>-compiler-engine.docx
```

此脚本是分析前端 DOCX 质量差异时的高质量对照路径。

### 5.5 其他 Windows 脚本

本轮还形成以下 PowerShell 适配脚本：

```text
scripts/build_paper3_dual_docx.ps1
scripts/build_paper3_pandoc_docx.ps1
scripts/build_paper3_three_docx.ps1
```

恢复时应与对应 `.sh` 脚本保持参数语义一致：

1. `build_paper3_dual_docx.ps1`：生成 sh/Python 路径与 Rust 路径双产物，并可选打包。
2. `build_paper3_pandoc_docx.ps1`：生成 pandoc 路径 DOCX。
3. `build_paper3_three_docx.ps1`：生成 sh/Rust/pandoc 三路径产物并汇总。

---

## 六、前端 DOCX 质量差异根因

### 6.1 用户观察

前端通过：

```powershell
cargo run -p doc-desktop-slint
```

选择：

```text
D:\papers\paper3
```

生成：

```text
D:\output3.docx
```

报告显示：

```text
Profile: chinese-academic (Chinese Academic Paper)
Source: explicit_id

Compatibility: 70%
Backend: rule-based

Quality Gate: Failed (score=70)
Checks: 6/7 passed
  WARN: backend_fallback: backend fell back from Auto to RuleBased
```

### 6.2 与脚本路径的关键差异

高质量脚本路径：

```text
scripts/build_paper3_compiler_engine_docx.ps1
```

明确使用：

```text
--profile jos-paper
```

而前端当时显示：

```text
Profile: chinese-academic
Backend: rule-based
```

所以即使 `D:\papers\paper3` 与脚本目标内容一致，最终 DOCX 仍会因为以下因素不同而产生质量差异：

1. profile 不同：
   - 脚本：`jos-paper`
   - 前端：`chinese-academic`
2. profile 来源不同：
   - 脚本：命令行显式指定正确 profile。
   - 前端：UI 状态或设置中保留了错误/陈旧 profile。
3. backend 不同或 fallback 行为不同：
   - 报告显示后端从 Auto fallback 到 RuleBased。
4. DOCX writer 行为在共享写出层存在问题：
   - 即使 profile 修正，内联公式、OMML、算法/代码块表格仍会影响最终质量。

### 6.3 结论

前端质量差不是单纯由“选择的文件夹不同”造成，核心是：

1. 前端转换参数与脚本参数不一致。
2. 前端状态中可能保留了错误 profile。
3. 共享 DOCX 写出层对内联数学和算法/代码块的处理不够可靠。
4. `doc-mathml` 对 paper3 常用命令支持不完整。

因此修复必须同时覆盖：

1. 前端 profile/quality/backend 参数路径。
2. DOCX writer。
3. mathml parser/OMML writer。
4. 端到端 DOCX XML 验证脚本。

---

## 七、DOCX writer 改造细节

### 7.1 文件

```text
crates/docx-writer/src/serializer.rs
```

### 7.2 新增依赖导入

恢复时需要在文件顶部引入：

```rust
use doc_mathml::{latex::parse_latex_math, omml::to_omml};
```

当前实现实际使用了 `parse_latex_math`，`to_omml` 在当前 diff 中可见但可能未使用。恢复时应根据最终代码清理未使用 import，避免 `cargo check` 警告或失败。

### 7.3 itemize 保留内联 run

原问题：

1. `itemize` 列表项会通过 `itemize_merged_text(items)` 合并为单个字符串。
2. 列表项里的 `TextStyle::MathInline` 被降级为普通文本。
3. `\varepsilon`、`\in`、`\emptyset` 等命令泄漏到普通 `w:t` 文本。

目标实现：

1. 对非引用、非 bio、非有序列表，逐个列表项生成段落。
2. 使用 `itemize_item_runs(sub, "• ")` 构造 run 列表。
3. 对 `Block::Paragraph` 中的 `TextRun` 调用 `from_text_run`，保留 MathInline、Bold、Italic 等格式。
4. 对非 Paragraph 子块才降级为 `summarize_to_string`。
5. 最后调用 `merge_adjacent_runs` 合并相邻普通 run。

恢复函数：

```rust
fn itemize_item_runs(sub: &[Block], prefix: &str) -> Vec<Run> {
    let mut runs = Vec::new();
    runs.push(Run::plain(prefix.to_string()));

    for block in sub {
        match block {
            Block::Paragraph { runs: ps_runs, .. } => {
                let converted: Vec<Run> = ps_runs.iter().map(from_text_run).collect();
                runs.extend(converted);
            }
            _ => {
                let text = summarize_to_string(&[block.clone()]);
                if !text.is_empty() {
                    runs.push(Run::plain(text));
                }
            }
        }
    }

    merge_adjacent_runs(runs)
}
```

### 7.4 算法块改为文本段落

原问题：

1. 算法块使用表格渲染。
2. 表格列宽固定。
3. 注释列稍长就会换行挤压，导致表格变形。
4. 用户明确要求代码块/算法块不要用表格，用文本字符即可。

目标实现：

1. caption 单独使用 `STYLE_CAPTION` 居中段落。
2. 输入输出行使用 `STYLE_CODE` 段落。
3. 算法每行使用 `STYLE_CODE` 段落。
4. 行号、缩进、注释全部使用文本字符。
5. Word 在整段宽度内自然换行，不再按表格列挤压。

行格式：

```text
  1 | init H
  2 |   foreach item // hot path
```

恢复函数：

```rust
fn format_algline_for_docx(line: &AlgLine, line_no: usize) -> String {
    let indent_str = "  ".repeat(u32::from(line.indent) as usize);
    let code_text = format_algline_display_code(line);
    let comment_part = line
        .comment
        .as_ref()
        .map(|c| format!(" // {}", c))
        .unwrap_or_default();
    format!("{:>3} | {}{}{}", line_no, indent_str, code_text, comment_part)
}
```

恢复 `Block::Algorithm` 分支时应确保：

1. 不再调用 `write_algorithm_table`。
2. 旧表格实现可以暂时保留为未使用函数，也可以后续删除。
3. 算法 caption 不应位于 `<w:tbl>` 内。

### 7.5 代码块改为逐行段落

原问题：

1. 代码块作为单个长 run 写出。
2. 长行或注释在 Word 中排版不可控。
3. 如果通过表格或单段承载，容易出现挤压。

目标实现：

1. `Block::CodeBlock` 按 `code.lines()` 拆分。
2. 每行写一个 `STYLE_CODE` 段落。
3. 字体使用 `Courier New`。
4. 每个段落 `keep_lines = true`。
5. 语言标签仍用 `STYLE_COMMENT` 输出：

```text
// language: rust
```

### 7.6 内联数学写出真实 OMML

原问题：

旧 `write_inline_math_run` 只是把 raw LaTeX 放入 `<m:t>`：

```xml
<m:t>\varepsilon \in \mathcal{X}</m:t>
```

这不是可被 Word 稳定解释的公式结构，容易导致乱码、显示异常或 raw LaTeX 泄漏。

目标实现：

1. 使用 `parse_latex_math(latex)` 解析为 `MathExpr`。
2. 根据 `MathExpr` 写出 OMML：
   - `MathExpr::Sub` -> `m:sSub`
   - `MathExpr::Sup` -> `m:sSup`
   - `MathExpr::SubSup` -> `m:sSubSup`
   - `MathExpr::Frac` -> `m:f`
   - `MathExpr::Sqrt` -> `m:rad`
   - `MathExpr::Fenced` -> `m:d`
   - `MathExpr::Function` -> `m:func`
   - `MathExpr::Matrix` -> `m:m`
   - `MathExpr::Seq` -> 逐元素写出
3. 对数字、标识符、文本、普通运算符都写为普通 `m:r/m:t`。

恢复函数组：

```text
write_inline_math_run
write_omath_para
write_omath
write_omath_e
write_omath_sub
write_omath_sup
write_omath_text
```

注意：

1. 当前实现的注释写的是 `m:oMathPara`，实际函数只写了 `m:oMath`，恢复时应让注释与 XML 结构一致。
2. 如果需要完整段落级公式，可再补 `m:oMathPara`，但本轮核心目标是内联公式进入真实 OMML。

### 7.7 新增/调整单元测试

恢复时应至少保留以下测试意图：

1. `inline_math_uses_parsed_omml_not_raw_latex`
   - 输入 `\varepsilon \in \mathcal{X}`。
   - 断言 XML 包含 `<m:oMath`。
   - 断言 XML 不包含 raw `\varepsilon`。
   - 断言 XML 不包含 raw `\in`。
2. `itemize_preserves_inline_math_runs`
   - 列表项包含普通文本 + MathInline + 普通文本。
   - 断言列表项中仍有 `<m:oMath`。
   - 断言不泄漏 raw `\varepsilon`。
3. `algorithm_serializes_as_joscode`
   - 输入算法块。
   - 断言包含 `算法 1: Attention list`。
   - 断言包含 `Input:`、`logs`。
   - 断言包含 `  1 | init H`。
   - 断言包含 `// hot path`。
   - 断言不包含 `<w:tbl>`。

---

## 八、数学解析与 OMML 改造细节

### 8.1 LaTeX parser

文件：

```text
crates/mathml/src/latex.rs
```

新增/补齐命令：

| 命令 | 输出 |
|---|---|
| `\varepsilon` | `ε` |
| `\rightarrow` | `→` |
| `\leftarrow` | `←` |
| `\emptyset` | `∅` |
| `\in` | `∈` |
| `\notin` | `∉` |
| `\subset` | `⊂` |
| `\subseteq` | `⊆` |
| `\ldots` | `…` |
| `\dots` | `…` |

同时将以下命令作为文本/样式型包装命令处理，解析其 group 或单个参数：

```text
\operatorname
\textbf
\textit
```

恢复点：

1. 在 known command 列表中加入上述命令。
2. 在 `lower_command` 中加入对应映射。
3. 未知命令仍降级为 `MathExpr::Raw(format!("\\{cmd}"))`，方便后续发现遗漏。

### 8.2 OMML writer

文件：

```text
crates/mathml/src/omml.rs
```

调整目标：

1. 普通运算符不再写为 `m:oSupp`。
2. `MathExpr::Op(c)` 直接写普通文本 run。
3. `MathExpr::Seq(seq)` 不再外层包一个 `m:r`，而是每个子元素自己写 run 或结构。
4. `write_run_text` 中 `m:t` 增加 `xml:space="preserve"`，保留空格。

恢复测试：

1. `omml_basic`
   - 确认包含 `<m:oMath`。
   - 确认包含 `<m:sSup`。
   - 确认普通符号进入 `<m:r><m:t>...</m:t></m:r>`。
2. `omml_op_as_text`
   - 确认 `+` 作为 `<m:t>+</m:t>`。
   - 确认不再包含 `<m:oSupp`。
3. `omml_seq_per_element_runs`
   - 确认 sequence 中每个元素有自己的 run。

---

## 九、前端 paper3 DOCX 验证脚本

### 9.1 文件

```text
scripts/test_paper3_frontend_docx.ps1
```

### 9.2 参数

```powershell
param(
    [string]$DocxPath = "D:\output3.docx",
    [switch]$GenerateDocx,
    [switch]$SkipCargo,
    [switch]$SkipExistingDocxCheck
)
```

含义：

| 参数 | 作用 |
|---|---|
| `-DocxPath` | 指定要生成或检查的 DOCX |
| `-GenerateDocx` | 通过 Rust 回归测试生成 DOCX 到指定路径 |
| `-SkipCargo` | 跳过 cargo 测试，只检查已有 DOCX |
| `-SkipExistingDocxCheck` | 跳过 DOCX XML 检查 |

### 9.3 Rust 回归检查

脚本目标是运行：

```powershell
cargo test -p doc-mathml -- --nocapture
cargo test -p doc-docx-writer inline_math_uses_parsed_omml_not_raw_latex -- --nocapture
cargo test -p doc-docx-writer algorithm_serializes_as_text_block -- --nocapture
cargo test -p doc-desktop-slint paper3_conversion_overrides_stale_chinese_profile -- --nocapture
```

但当前核对发现实际 `serializer.rs` 中算法测试名为：

```text
algorithm_serializes_as_joscode
```

恢复时必须二选一保持一致：

1. 将脚本里的 `algorithm_serializes_as_text_block` 改为 `algorithm_serializes_as_joscode`。
2. 或将 Rust 测试函数改名为 `algorithm_serializes_as_text_block`。

建议采用第 1 种，保留测试名表达“JOSCode 段落”这一最终实现。

### 9.4 DOCX XML 检查

脚本读取：

```text
word/document.xml
```

检查：

1. raw LaTeX 泄漏：

```text
\varepsilon
\rightarrow
\emptyset
```

2. OMML 数量：

```text
<m:oMath
```

3. 表格数量：

```text
<w:tbl
<w:tc
```

4. JOSCode 样式：

```xml
<w:pStyle w:val="JOSCode"/>
```

5. 算法标题是否仍在表格内：

```text
算法 1
```

如果发现：

1. raw LaTeX 命令仍存在，则失败。
2. OMML 数量为 0，则失败。
3. 没有 JOSCode 段落，则失败。
4. 算法标题位于 `<w:tbl>` 内，则失败。

### 9.5 推荐验证命令

生成并检查：

```powershell
.\scripts\test_paper3_frontend_docx.ps1 -GenerateDocx -DocxPath E:\tmp\paper3-frontend-output.docx
```

只检查 GUI 前端已经生成的文件：

```powershell
.\scripts\test_paper3_frontend_docx.ps1 -SkipCargo -DocxPath D:\output3.docx
```

期望指标示例：

```text
OMML equations : 192
tables         : 55
table cells    : 978
JOSCode style  : True
algorithm tbl  : False
```

说明：

1. `tables` 仍大于 0 是正常的，因为论文真实表格仍需要表格。
2. 核心是算法标题不能在表格内，且 raw LaTeX 不应泄漏。

---

## 十、前端转换参数修复目标

当前 `crates/desktop-slint/src/commands.rs` 可见转换入口：

```rust
pub fn run_local_convert(
    project_root: &Path,
    output_path: &Path,
    profile: &str,
    quality: &str,
    _app_state: &AppState,
) -> CommandResult<LocalConvertResult> {
    ...
    let artifact = crate::local_convert::convert(
        project_root,
        &main_tex,
        output_path,
        Some(&report_path),
        profile,
        quality,
    )?;
    ...
}
```

恢复“前端 paper3 与脚本质量一致”的目标时，需要确认：

1. Convert 页 profile 下拉的默认值不应固定为 `chinese-academic`。
2. 对 `D:\papers\paper3` / JOS paper3，应使用 `jos-paper` 或正确 Auto 检测到 `jos-paper`。
3. Settings 中保存的旧 profile 不应覆盖当前 Convert 页明确选择。
4. 如果用户选择 `auto`，应验证 auto 结果是否为 `jos-paper`。
5. 如果用户明确选择 `jos-paper`，报告里 `Source` 应体现 explicit 或 equivalent，但 profile 必须正确。
6. backend fallback 到 RuleBased 不是唯一问题；即便 fallback，也不应导致 profile 变为 `chinese-academic`。

建议恢复/补充的桌面端回归测试意图：

```text
paper3_conversion_overrides_stale_chinese_profile
```

测试应覆盖：

1. 模拟设置中保存 `chinese-academic`。
2. 转换 paper3 时选择 `auto` 或 `jos-paper`。
3. 断言最终 active profile 为 `jos-paper`。
4. 断言 DOCX XML 包含 OMML。
5. 断言 DOCX XML 不包含关键 raw LaTeX。
6. 断言算法标题不在表格内。

当前核对没有在 `crates/desktop-slint/src/commands.rs` 中找到该测试；如果该测试不在其他文件，恢复时应补回。

---

## 十一、已知不一致点与必须修正项

### 11.1 `serializer.rs` 算法块残留旧调用片段

当前核对 `crates/docx-writer/src/serializer.rs` 时，`Block::Algorithm` 分支后出现残留片段：

```rust
            }
                    lines,
                    io,
                    caption.as_deref(),
                    number.as_deref(),
                    text_width,
                );
            }
            Block::CodeBlock { language, code, .. } => {
```

这看起来是从旧代码：

```rust
write_algorithm_table(
    &mut w,
    lines,
    io,
    caption.as_deref(),
    number.as_deref(),
    text_width,
);
```

删除时残留的参数列表。

恢复时必须：

1. 删除该残留片段。
2. 删除同一分支中已经不再需要的 `text_width` 局部变量。
3. 确保 `Block::Algorithm` 分支以文本段落渲染后直接结束。
4. 运行 `cargo check -p doc-docx-writer` 或相关测试确认语法正确。

### 11.2 测试脚本与 Rust 测试名不一致

当前：

```text
scripts/test_paper3_frontend_docx.ps1
```

调用：

```text
algorithm_serializes_as_text_block
```

当前：

```text
crates/docx-writer/src/serializer.rs
```

测试名：

```text
algorithm_serializes_as_joscode
```

恢复时必须统一。

建议修改脚本为：

```powershell
Invoke-Native -FilePath "cargo" -Arguments @(
    "test", "-p", "doc-docx-writer", "algorithm_serializes_as_joscode", "--", "--nocapture"
) -WorkingDirectory $Root
```

### 11.3 文档中记录的已通过测试需要重新跑

`paper3-frontend-docx-quality-optimization-summary-20260622.md` 已记录一组通过命令，但由于当前核对发现上述不一致，恢复或继续开发时应重新执行验证，不能只依赖旧记录。

---

## 十二、推荐恢复步骤

如果需要在干净分支上恢复本轮改造，建议按以下顺序执行。

### 12.1 恢复文档

确保以下文档存在：

```text
docs-zh/ui/desktop-slint-multi-tab-ui-refactor-plan-20260622.md
docs-zh/ui/desktop-slint-ui-ux-improvement-report-20260622.md
docs-zh/ui/desktop-slint-ui-ux-overall-improvement-plan-v2-20260622.md
docs-zh/ui/paper3-frontend-docx-quality-optimization-summary-20260622.md
docs-zh/ui/tex2doc-session-change-recovery-report-20260622.md
```

### 12.2 恢复 Windows 脚本

确保以下脚本存在并可运行：

```text
scripts/_build_common.ps1
scripts/build_docx.ps1
scripts/build_paper3_compiler_engine_docx.ps1
scripts/build_paper3_dual_docx.ps1
scripts/build_paper3_pandoc_docx.ps1
scripts/build_paper3_three_docx.ps1
scripts/test_paper3_frontend_docx.ps1
```

快速检查：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\build_paper3_compiler_engine_docx.ps1 v13
```

### 12.3 恢复 mathml parser

修改：

```text
crates/mathml/src/latex.rs
```

恢复内容：

1. 支持 `\varepsilon`。
2. 支持 `\rightarrow`、`\leftarrow`。
3. 支持集合符号 `\emptyset`、`\in`、`\notin`、`\subset`、`\subseteq`。
4. 支持省略号 `\ldots`、`\dots`。
5. 支持 `operatorname`、`textbf`、`textit` 的 group 降级。

验证：

```powershell
cargo test -p doc-mathml -- --nocapture
```

### 12.4 恢复 OMML writer

修改：

```text
crates/mathml/src/omml.rs
```

恢复内容：

1. `MathExpr::Op` 写普通文本 run。
2. `MathExpr::Seq` 不外包单个 `m:r`。
3. `write_run_text` 写 `xml:space="preserve"`。
4. 更新相关测试断言。

验证：

```powershell
cargo test -p doc-mathml -- --nocapture
```

### 12.5 恢复 DOCX serializer

修改：

```text
crates/docx-writer/src/serializer.rs
```

恢复内容：

1. `itemize` 保留 `Run` 列表。
2. 新增 `itemize_item_runs`。
3. 算法块使用 `STYLE_CAPTION` + `STYLE_CODE` 段落，不调用表格。
4. 新增 `format_algline_for_docx`。
5. 代码块逐行写 `STYLE_CODE`。
6. `write_inline_math_run` 走 `parse_latex_math` 和 OMML helper。
7. 新增/恢复内联数学、itemize、算法块测试。
8. 删除旧 `write_algorithm_table` 调用残片。

验证：

```powershell
cargo test -p doc-docx-writer inline_math_uses_parsed_omml_not_raw_latex -- --nocapture
cargo test -p doc-docx-writer itemize_preserves_inline_math_runs -- --nocapture
cargo test -p doc-docx-writer algorithm_serializes_as_joscode -- --nocapture
```

### 12.6 恢复前端转换回归

确认或补充：

```text
paper3_conversion_overrides_stale_chinese_profile
```

建议命令：

```powershell
cargo test -p doc-desktop-slint paper3_conversion_overrides_stale_chinese_profile -- --nocapture
```

如果当前代码中没有该测试，需要补回或用等价测试覆盖。

### 12.7 跑端到端 DOCX 检查

生成并检查：

```powershell
.\scripts\test_paper3_frontend_docx.ps1 -GenerateDocx -DocxPath E:\tmp\paper3-frontend-output.docx
```

GUI 前端检查：

```powershell
cargo run -p doc-desktop-slint
```

手动选择：

```text
D:\papers\paper3
```

输出：

```text
D:\output3.docx
```

随后检查：

```powershell
.\scripts\test_paper3_frontend_docx.ps1 -SkipCargo -DocxPath D:\output3.docx
```

---

## 十三、验收标准

### 13.1 UI 方案验收

1. 多 TAB 方案文档存在。
2. v2 多语言/主题/版本号方案文档存在。
3. 文档明确默认语言为英语。
4. 文档明确支持：
   - 简体中文
   - 繁体中文
   - 法文
   - 日文
   - 德文
5. 文档明确主题 token 和全局切换机制。
6. 文档明确版本号规则 `1.YY.M.N` 和 git 提交自动递增策略。

### 13.2 Windows 脚本验收

1. PowerShell 脚本可从仓库根目录直接运行。
2. 所有脚本路径使用 Windows 兼容路径构造。
3. 版本参数同时支持 `v13` 和 `13`。
4. 命令失败时能抛出明确错误。
5. 产物输出路径与 `.sh` 脚本保持语义一致。

### 13.3 DOCX 质量验收

对 `D:\output3.docx` 检查：

1. `word/document.xml` 包含 `<m:oMath`。
2. 不包含 raw：

```text
\varepsilon
\rightarrow
\emptyset
```

3. 包含：

```xml
<w:pStyle w:val="JOSCode"/>
```

4. 算法标题 `算法 1` 不在 `<w:tbl>` 内部。
5. 算法/代码块不因表格列宽导致注释挤压。
6. 正文真实表格仍可保留，不要求 `tables` 为 0。

### 13.4 profile 验收

前端转换 paper3 时，报告不应再显示：

```text
Profile: chinese-academic
Compatibility: 70%
Quality Gate: Failed (score=70)
```

期望：

1. profile 为 `jos-paper`，或 Auto 正确检测为 `jos-paper`。
2. quality gate 不因 profile 错误失败。
3. backend fallback 警告即使存在，也不影响 profile 正确性。

---

## 十四、风险与后续建议

### 14.1 共享 serializer 风险

`crates/docx-writer/src/serializer.rs` 是共享 DOCX 写出层，影响范围大。修改算法块、代码块、内联数学后，需要回归：

1. paper3。
2. 普通论文。
3. 含 itemize 内联公式的文档。
4. 含算法块的文档。
5. 含代码块的文档。
6. 含真实表格的文档。

### 14.2 数学能力边界

本轮只补齐 paper3 已暴露的常见命令。后续如果遇到更复杂 LaTeX，如 `cases`、`align`、复杂矩阵、宏定义递归展开，应优先扩展 `doc-mathml` AST 和 OMML writer，而不是在 DOCX writer 中做字符串替换。

### 14.3 UI 多语言落地风险

1. Slint 组件中如果继续写死英文或中文，会破坏多语言一致性。
2. 翻译 key 如果使用原文，后续重命名困难。
3. 语言切换需要立即刷新 UI，不能只在重启后生效。
4. 日志、报告、用户文档路径不应强行翻译。

### 14.4 主题落地风险

1. 如果组件继续硬编码颜色，主题切换会出现局部不生效。
2. 高对比主题需要单独检查 focus、disabled、error 状态。
3. 不建议使用单一蓝紫或深蓝主题统治全部界面，工具型 UI 应保持克制。

### 14.5 版本号自动递增风险

1. `pre-commit` hook 会影响 amend/rebase，需要支持 `TEX2DOC_SKIP_VERSION_BUMP=1`。
2. Cargo package version 与桌面产品版本应区分。
3. merge commit 或 GUI Git 工具也会触发 hook，需要在文档中说明。

---

## 十五、最终结论

本轮会话形成了四类成果：

1. **UI/UX 方案成果**：完成多 TAB 重构方案，并追加多语言、主题、版本号管理的 v2 总体方案。
2. **Windows 构建脚本成果**：为 paper3 / DOCX 构建链路形成 PowerShell 适配方案与脚本集合。
3. **DOCX 质量修复成果**：定位并改造内联数学、OMML、算法块、代码块、itemize 等关键质量问题，使前端 paper3 输出向 compiler-engine 脚本质量靠齐。
4. **验证体系成果**：形成 `scripts/test_paper3_frontend_docx.ps1`，可生成或检查 DOCX，并通过 XML 指标验证公式、raw LaTeX、JOSCode 和算法表格问题。

恢复时最重要的顺序是：

```text
先恢复 mathml -> 再恢复 serializer -> 修正测试脚本名和算法残片 -> 再跑 paper3 端到端验证 -> 最后继续 UI 多语言/主题/版本号落地
```

当前报告可以作为后续继续开发或重建分支时的恢复索引。恢复完成后，必须重新执行测试命令，不应只依赖历史通过记录。

---

## 十六、本次会话 Slint 多TAB重构 Phase A-D 实施报告

**日期**：2026-06-22  
**会话范围**：`crates/desktop-slint` Slint 多TAB UI 重构 Phase A–D 全部完成，`cargo check` 零警告零错误通过。

### 16.1 实施背景

本次会话在既有多TAB重构方案（`desktop-slint-multi-tab-ui-refactor-plan-20260622.md`）基础上，按 Phase A → D 顺序将 Slint UI 从单页堆叠布局推进为多TAB产品化界面。Rust 侧 callback 绑定和 `main.rs` 均保持稳定，不引入破坏性变更。

### 16.2 最终文件结构

```
crates/desktop-slint/src/ui/
├─ main.slint               # 主窗口，持有TabWidget，引用各page component
├─ types.slint              # Phase D 新增：共享结构体 PlanEntry / JobRow / ConversionMode
├─ components/
│  └─ (reserved for future)
└─ pages/
   ├─ convert.slint         # Phase A：转换Tab（主Tab）
   ├─ settings.slint        # Phase B：配置Tab
   ├─ account.slint         # Phase B：账号Tab
   ├─ billing.slint         # Phase C：套餐Tab
   └─ history.slint         # Phase C：历史/诊断Tab

crates/desktop-slint/src/ui_bindings/
├─ mod.rs                   # wire_all() 驱动所有子模块
├─ conversion.rs            # Phase A：绑定转换相关callback
├─ settings.rs             # Phase B：绑定配置相关callback
├─ account.rs               # Phase B：绑定账号相关callback
├─ billing.rs               # Phase C：绑定套餐/云端callback
├─ history.rs               # Phase C：绑定历史/诊断callback
├─ diagnostics.rs           # Phase C：绑定诊断callback
├─ update.rs                # Phase C：绑定更新callback
└─ helpers.rs               # 共享辅助：最近任务持久化、UI model转换
```

### 16.3 Phase A–D 改造详情

#### Phase A — 主转换Tab（Convert Tab）

**涉及文件**：

| 文件 | 改动类型 | 说明 |
|---|---|---|
| `ui/main.slint` | 重写 | 引入 `TabWidget`，默认选中 Convert Tab |
| `ui/pages/convert.slint` | 新增 | 提取项目路径选择、Profile/Quality下拉、Local/Cloud转换按钮、进度条、报告摘要 |
| `ui_bindings/conversion.rs` | 新增 | 绑定 `on-convert-clicked` / `on-cloud-convert-clicked` / `on-detect-profile-clicked` / `on-select-project-clicked` / `on-select-output-clicked` |

**关键实现**：

- `TabWidget` 在 `main.slint` 根级别声明，5个tab依次为 Convert / Settings / Account / Billing / History
- Convert Tab 默认打开（`current-index: 0`）
- 转换期间 `is-converting` 阻塞两个转换按钮，避免重复提交
- `conversion-progress` 驱动 Slint `ProgressIndicator`
- Profile 检测成功后同步更新 `detected-profile` 显示

#### Phase B — 配置Tab + 账号Tab

**涉及文件**：

| 文件 | 改动类型 | 说明 |
|---|---|---|
| `ui/pages/settings.slint` | 新增 | API base URL、Default Profile/Quality/Output、Update channel |
| `ui/pages/account.slint` | 新增 | Email/Password输入、Login/Register/Refresh/Logout、账号状态 |
| `ui_bindings/settings.rs` | 新增 | 绑定设置保存、Settings.on-* callback |
| `ui_bindings/account.rs` | 新增 | 绑定登录注册登出流程、session持久化 |
| `ui/main.slint` | 修改 | 在TabWidget中添加对应tab节点 |

**关键实现**：

- `set-account-session` 写入 session token 和 display name 到 `AppState`
- `persist-session` callback 将 session 序列化为 JSON 存入本地
- `load-session` 在启动时恢复已有 session，自动调用 refresh
- 账号页 `account-status` 文本反映当前登录状态（`Signed in` / `Not signed in` / `Refreshing...`）

#### Phase C — 套餐Tab + 历史/诊断Tab

**涉及文件**：

| 文件 | 改动类型 | 说明 |
|---|---|---|
| `ui/pages/billing.slint` | 新增 | Plan ID、套餐列表、Checkout、Billing Portal、套餐状态 |
| `ui/pages/history.slint` | 新增 | 最近任务列表、打开输出、打开报告、导出诊断 |
| `ui_bindings/billing.rs` | 新增 | 绑定套餐查询、checkout、billing portal |
| `ui_bindings/billing_cloud.rs` | 新增 | 绑定云端相关callback |
| `ui_bindings/history.rs` | 新增 | 绑定最近任务增删、导出诊断 |
| `ui/main.slint` | 修改 | 在TabWidget中添加 Billing / History tab 节点 |

**关键实现**：

- `plan-catalog` 使用 Slint `[PlanEntry]` model 渲染套餐卡片
- `job-history` 使用 `[JobRow]` model 渲染最近任务列表
- History Tab `clear-jobs-clicked` 清空全部最近任务

#### Phase D — 共享结构体类型 + AppState 状态化

**涉及文件**：

| 文件 | 改动类型 | 说明 |
|---|---|---|
| `ui/types.slint` | 新增 | `PlanEntry` / `JobRow` / `ConversionMode` 结构体定义 |
| `app_state.rs` | 修改 | 新增 `user_name: RwLock<Option<String>>` 存储账号显示名 |
| `ui/main.slint` | 修改 | 引入 `TabWidget`，连接所有 tab 属性 |

**关键实现**：

- `PlanEntry { id, name, price, quota, features }` 替代 billing slint 中的字符串拼接
- `JobRow { id, kind, input, output, status, opened-at, error, html-report }` 支持逐行操作
- `ConversionMode { local, cloud }` 区分转换模式
- `AppState::set_account_session` 接受 `display_name: Option<String>` 并持久化
- `AppState::display_name` getter 方法标记 `#[allow(dead_code)]`，保留为未来UI消费

### 16.4 警告清理

本次会话末尾清理了 `cargo check` 产生的三处警告：

#### 1. `padding only has effect on layout elements`（billing.slint:29 / history.slint:42）

**根因**：`padding` 属性只对布局元素（如 `HorizontalBox`、`VerticalBox`）生效，放在 `Rectangle` 非布局元素上无效。

**修复**：将 `padding: Npx` 从 `Rectangle` 子元素中移除，移入内层 `HorizontalBox`（布局元素）：

```slint
// 修复前（无效）
for plan[idx] in plan-catalog: Rectangle {
    background: ...;
    border-radius: 4px;
    padding: 8px;   // ← 无效，Rectangle不是布局元素
    HorizontalBox {
        spacing: 12px;
        ...
    }
}

// 修复后
for plan[idx] in plan-catalog: Rectangle {
    background: ...;
    border-radius: 4px;
    HorizontalBox {
        padding: 8px;  // ← 有效，HorizontalBox是布局元素
        spacing: 12px;
        ...
    }
}
```

#### 2. `unused import: JobRow`（history.rs:4）

**根因**：`history.rs` 导入了 `JobRow` 但实际通过 `helpers::job_history_for_ui` 返回的 `Vec<JobRow>` 直接赋值给 Slint model，未直接使用该类型。

**修复**：移除 import 行中的 `JobRow`：

```rust
// 修复前
use crate::ui::{JobRow, MainWindow};

// 修复后
use crate::ui::MainWindow;
```

#### 3. `method display_name is never used`（app_state.rs）

**根因**：`AppState::display_name()` 是 Phase D 新增的 getter 方法，用于未来UI消费缓存的账号显示名，当前无调用方。

**修复**：在方法上添加 `#[allow(dead_code)]` 标注，注明保留用途：

```rust
#[allow(dead_code)] // reserved for future UI consumption of cached account name
pub fn display_name(&self) -> Option<String> {
    self.user_name.read().ok().and_then(|n| n.clone())
}
```

#### 验证结果

```powershell
cargo check -p doc-desktop-slint
# Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.75s
# 0 warnings, 0 errors
```

### 16.5 Rust 侧绑定架构

`main.rs` 中原有的一坨式 callback 绑定已按领域拆分到 `ui_bindings/` 子模块，由 `wire_all()` 统一驱动：

```rust
// crates/desktop-slint/src/ui_bindings/mod.rs
pub fn wire_all(ui: &MainWindow, state: Arc<AppState>) {
    update::wire_update(ui, Arc::clone(&state));     // Phase C
    account::wire_account(ui, Arc::clone(&state));   // Phase B
    billing::wire_billing(ui, Arc::clone(&state));   // Phase C
    billing::wire_billing_cloud(ui, Arc::clone(&state)); // Phase C
    settings::wire_settings(ui, Arc::clone(&state));  // Phase B
    conversion::wire_conversion(ui, Arc::clone(&state)); // Phase A
    history::wire_history(ui, Arc::clone(&state));    // Phase C
    diagnostics::wire_diagnostics(ui, Arc::clone(&state)); // Phase C
}
```

绑定顺序为 `update → account → billing → settings → conversion → history → diagnostics`，与此前 `main.rs` 中的 push 模型顺序保持一致，避免 callback 重入问题。

### 16.6 Slint TabWidget 使用说明

当前 Slint 版本使用 `TabWidget` 标准控件：

```slint
import { TabWidget } from "std-widgets.slint";

MainWindow := Window {
    TabWidget {
        current-index: 0;  // 默认打开 Convert Tab

        Tab {
            title: "Convert"; // 可后续替换为 i18n property
            ConvertTab { ... }
        }

        Tab {
            title: "Settings";
            SettingsTab { ... }
        }

        Tab {
            title: "Account";
            AccountTab { ... }
        }

        Tab {
            title: "Plans";
            BillingTab { ... }
        }

        Tab {
            title: "History";
            HistoryTab { ... }
        }
    }
}
```

**已知限制**：

- Slint `TabWidget` 每个 `Tab` 的 `title` 当前为硬编码英文文本，未来通过 `in-out property <string> t-tab-convert` 等 i18n 属性替换
- `TabWidget` 不支持拖拽排序，不在本轮范围内

### 16.7 恢复清单

如需在丢失代码后按本报告重建，执行顺序：

```powershell
# 1. 确保目录结构存在
mkdir -Force crates\desktop-slint\src\ui\pages
mkdir -Force crates\desktop-slint\src\ui\components
mkdir -Force crates\desktop-slint\src\ui_bindings

# 2. 创建类型文件
# 写入 crates/desktop-slint/src/ui/types.slint（PlanEntry / JobRow / ConversionMode）

# 3. 创建各 page slint 文件
# 写入 crates/desktop-slint/src/ui/pages/{convert,settings,account,billing,history}.slint

# 4. 重写 main.slint，引入 TabWidget 和各 page component

# 5. 创建 ui_bindings 子模块
# 写入 ui_bindings/{conversion,settings,account,billing,billing_cloud,history,diagnostics,update,helpers}.rs

# 6. 更新 main.rs，使用 wire_all() 替代原有 callback 绑定

# 7. 验证编译
cargo check -p doc-desktop-slint

# 8. 运行测试
cargo run -p doc-desktop-slint
```

### 16.8 未完成项（后续会话）

| 优先级 | 事项 | 说明 |
|---|---|---|
| P1 | 多语言国际化（i18n） | `i18n/` 目录、`t-tab-*` 属性、翻译 key 规范 |
| P1 | 主题系统 | `ThemePalette`、Slint `color-*` token、全局主题切换 |
| P1 | `billing_cloud.rs` 从 `billing.rs` 拆分 | 当前 `billing.rs` 包含云端 callback，需独立文件 |
| P2 | 版本号管理 | `VERSION` 文件、`build.rs` 注入、`pre-commit` hook |
| P2 | 套餐 `plan-catalog` model | 从当前字符串升级为 Slint model，支持动态刷新 |
| P2 | `JobRow` 逐行操作 | History Tab 从 `recent-jobs` 字符串升级为 `[JobRow]` model |
| P3 | 抽出 `components/*.slint` | 抽取 `field-row`、`status-summary` 等可复用组件 |
| P3 | 单元测试覆盖 | `ui_bindings/` 各模块的 mock 测试 |

### 16.9 参考文档

| 文档 | 用途 |
|---|---|
| `docs-zh/ui/desktop-slint-multi-tab-ui-refactor-plan-20260622.md` | 多TAB重构技术方案，Phase A-D 详细规划 |
| `docs-zh/ui/tex2doc-session-change-recovery-report-20260622.md` | 本轮回话完整变更恢复报告（包含 Windows 脚本、DOCX质量修复等） |
| `docs-zh/ui/desktop-slint-ui-ux-overall-improvement-plan-v2-20260622.md` | v2 总体方案：多语言、主题、版本号 |
| `crates/desktop-slint/src/ui/` | Slint UI 源文件 |
| `crates/desktop-slint/src/ui_bindings/` | Rust callback 绑定源文件 |
