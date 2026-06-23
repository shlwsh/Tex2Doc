# 多期刊 TeX 到 DOCX 泛化能力验证方案

**日期**：2026-06-22
**范围**：项目默认支持的多种期刊 profile，对 `examples/paper2` 与 `examples/paper3` 两篇真实文稿进行专用格式转换验证，并将 DOCX、报告和核验结果统一输出到 `examples/output`。
**目标**：确认 Tex2Doc / Semantic TeX Engine 对多期刊模板的泛化能力，识别当前转换质量短板，并形成可重复执行的发布门禁。

## 一、验证结论口径

本方案不只验证“能生成 DOCX”，而是验证以下四件事：

1. 默认支持的期刊 profile 能被正确识别或显式指定。
2. `paper2` 与 `paper3` 在各期刊专用格式下均能生成 DOCX。
3. 输出 DOCX 在结构、公式、图片、表格、引用、caption、字体与样式上达到可接受质量。
4. 失败项可以被归类、复现、量化，并反向进入 profile / rule / renderer 改进清单。

当前已有 `scripts/verify_journal_profiles.sh` 可以验证 `examples/journals/*/minimal.tex`，但它主要覆盖最小 fixture，不足以证明真实论文泛化。本方案新增真实文稿矩阵验证。

## 二、默认期刊 Profile 范围

首批验证以下 7 个 profile：

| Profile ID | 期刊/模板 | 典型检测信号 | 备注 |
| --- | --- | --- | --- |
| `jos-paper` | IEEE/JOS Paper | `IEEEtran[journal]`、`IEEEkeywords`、`markboth` | paper2 的 `main-ral.tex` 和 paper3 的 `main-jos.tex` 可作为重点样本 |
| `tacl` | ACL/TACL Paper | `acl[aclang]`、`natbib`、`aclfinalcopy` | 需要为 paper2/paper3 构造 TACL 套壳入口 |
| `cvpr` | CVPR/ICCV Paper | `IEEEtran[conference]`、`cvprfinalcopy`、`cvprPaperID` | 与 JOS 同属 IEEE 类，需要重点验证 profile 区分 |
| `nature` | Nature Research Article | `nature` class、`naturemag` | 需要构造 Nature 套壳入口 |
| `springer` | Springer Journal Article | `springer` / `svjour3` / `llncs`、`institute` | 需要构造 Springer 套壳入口 |
| `chinese-academic` | 中文学术论文 | `ctexart`、`ctex`、`xeCJK` | paper2 `main-zh.tex` 与 paper3 `main-zh.tex` 是重点样本 |
| `generic` | 通用 article/arXiv | `article` / fallback | 作为未知模板和降级能力基线 |

注意：当前 `crates/compiler-engine/examples/paper3_to_docx.rs` 中，`tacl/cvpr/nature/springer` 仍映射到相近 enum 变体用于验证，尚不是完整动态 profile。验证报告必须记录 `profile_requested`、`profile_effective`、`backend_selected` 和 fallback 原因。

## 三、输入样本范围

### 3.1 原始文稿

| 文稿 | 根目录 | 已有入口 | 内容特征 |
| --- | --- | --- | --- |
| paper2 | `examples/paper2/latex` | `main.tex`、`main-ral.tex`、`main-zh.tex` | 英文/IEEE 风格、中文版本、公式、图表、算法、Bib |
| paper3 | `examples/paper3/latex` | `main-jos.tex`、`main-zh.tex` | JOS/中文学报风格、复杂中文、算法、公式、图片、Bib |

### 3.2 专用格式变体

`paper2` 和 `paper3` 原始主文件不天然覆盖所有 profile，因此验证前需要为每篇文稿生成“profile 专用入口文件”。入口文件只做模板套壳，不改正文内容。

建议新增本地生成目录：

```text
examples/output/journal-profile-generalization/inputs/
  paper2/
    generic/main.tex
    jos-paper/main.tex
    tacl/main.tex
    cvpr/main.tex
    nature/main.tex
    springer/main.tex
    chinese-academic/main.tex
  paper3/
    generic/main.tex
    jos-paper/main.tex
    tacl/main.tex
    cvpr/main.tex
    nature/main.tex
    springer/main.tex
    chinese-academic/main.tex
```

构造规则：

| Profile | paper2 基准入口 | paper3 基准入口 | 套壳要求 |
| --- | --- | --- | --- |
| `generic` | `main.tex` | 从 `main-zh.tex` 或 `main-jos.tex` 抽正文后套 `article` | 保持普通 article，验证 fallback |
| `jos-paper` | `main-ral.tex` | `main-jos.tex` | 使用 IEEE/JOS citation、keywords、markboth |
| `tacl` | `main.tex` 正文 | `main-jos.tex` 正文 | 使用 ACL/TACL title/author/abstract/citation 套壳 |
| `cvpr` | `main-ral.tex` 正文 | `main-jos.tex` 正文 | 使用 IEEEtran conference / CVPR 宏 |
| `nature` | `main.tex` 正文 | `main-jos.tex` 正文 | 使用 Nature title/affil/corres/bibliography style |
| `springer` | `main.tex` 正文 | `main-jos.tex` 正文 | 使用 Springer title/institute/keywords 套壳 |
| `chinese-academic` | `main-zh.tex` | `main-zh.tex` | 使用 ctex/XeLaTeX 路线，重点验证中文 caption、字体、断行 |

套壳原则：

- 只替换 preamble、title/author/abstract/keywords、bibliography style 和模板宏。
- 正文章节、公式、表格、图片、算法、引用键保持原样。
- 每个生成入口必须在文件头标注来源、profile、生成时间和是否人工改写。
- 如果某 profile 无法保持原正文结构，应在 manifest 中记录 `unsupported_reason`，不能静默跳过。

## 四、输出目录与命名规范

所有产物统一放在：

```text
examples/output/journal-profile-generalization/
```

目录结构：

```text
examples/output/journal-profile-generalization/
  inputs/
  docx/
  reports/
  logs/
  snapshots/
  verify-summary.json
  verify-summary.md
```

DOCX 命名：

```text
examples/output/journal-profile-generalization/docx/
  paper2__generic__auto.docx
  paper2__jos-paper__auto.docx
  paper2__tacl__auto.docx
  paper2__cvpr__auto.docx
  paper2__nature__auto.docx
  paper2__springer__auto.docx
  paper2__chinese-academic__auto.docx
  paper3__generic__auto.docx
  paper3__jos-paper__auto.docx
  paper3__tacl__auto.docx
  paper3__cvpr__auto.docx
  paper3__nature__auto.docx
  paper3__springer__auto.docx
  paper3__chinese-academic__auto.docx
```

对应报告：

```text
reports/{paper_id}__{profile_id}__auto.report.json
logs/{paper_id}__{profile_id}__auto.log
snapshots/{paper_id}__{profile_id}__word-open.txt
```

## 五、验证矩阵

主矩阵共 14 项：

| 文稿 | generic | jos-paper | tacl | cvpr | nature | springer | chinese-academic |
| --- | --- | --- | --- | --- | --- | --- | --- |
| paper2 | 必测 | 必测 | 必测 | 必测 | 必测 | 必测 | 必测 |
| paper3 | 必测 | 必测 | 必测 | 必测 | 必测 | 必测 | 必测 |

扩展矩阵：

| 维度 | 取值 | 用途 |
| --- | --- | --- |
| backend | `auto`、`rule-based`、`xelatex-hook`、`luatex-node` | 定位 profile 问题还是 backend 问题 |
| fallback | allow / no fallback | 区分真实支持和降级成功 |
| strict | strict / non-strict | 发布门禁用 strict，研发排查用 non-strict |
| platform | Windows / Linux / macOS | Beta 前至少 Windows 必测，GA 前三平台必测 |

首轮建议只跑主矩阵 + `backend=auto`，失败后再对失败项跑扩展矩阵。

## 六、自动化执行流程

建议新增脚本：

```text
scripts/verify_paper2_paper3_journal_profiles.ps1
scripts/verify_paper2_paper3_journal_profiles.sh
```

### 6.1 PowerShell 首选命令

```powershell
.\scripts\verify_paper2_paper3_journal_profiles.ps1 `
  -OutputDir examples\output\journal-profile-generalization `
  -Papers paper2,paper3 `
  -Profiles generic,jos-paper,tacl,cvpr,nature,springer,chinese-academic `
  -Backend auto
```

### 6.2 单项转换命令模板

```powershell
cargo run -p doc-compiler-engine --example paper3_to_docx -- `
  --project-root examples\output\journal-profile-generalization\inputs\paper2\jos-paper `
  --main-tex examples\output\journal-profile-generalization\inputs\paper2\jos-paper\main.tex `
  --profile jos-paper `
  --semantic-backend auto `
  --out examples\output\journal-profile-generalization\docx\paper2__jos-paper__auto.docx `
  --report examples\output\journal-profile-generalization\reports\paper2__jos-paper__auto.report.json
```

对 `paper3` 和其他 profile 替换 `{paper_id}`、`{profile_id}` 即可。

### 6.3 脚本步骤

1. 清理或创建 `examples/output/journal-profile-generalization`。
2. 检查 `examples/paper2/latex`、`examples/paper3/latex`、`crates/compiler-engine/profiles` 是否存在。
3. 生成 14 个 profile 专用入口。
4. 逐项执行 `cargo run -p doc-compiler-engine --example paper3_to_docx`。
5. 记录 stdout/stderr 到 `logs/`。
6. 检查 DOCX zip header 和核心 OOXML 部件。
7. 解包 DOCX，统计结构指标。
8. 如本机有 Word/LibreOffice，执行打开验证并记录结果。
9. 生成 `verify-summary.json` 和 `verify-summary.md`。
10. 对失败项给出分类和重跑建议。

## 七、自动质量核实指标

### 7.1 文件级检查

| 指标 | 通过标准 |
| --- | --- |
| DOCX 是否存在 | 必须存在 |
| 文件大小 | 大于 20 KB，且非异常 0 字节 |
| ZIP header | 必须为合法 DOCX zip |
| 核心部件 | 必须包含 `[Content_Types].xml`、`word/document.xml`、`word/styles.xml` |
| media | 有图片输入的样本应有 `word/media/*` |
| relationships | 必须包含 `word/_rels/document.xml.rels` |

### 7.2 结构级检查

| 指标 | 通过标准 |
| --- | --- |
| 段落数量 | 不低于同源文稿预期下限 |
| 标题层级 | 至少识别一级标题，章节顺序不乱 |
| 表格数量 | 主要表格不应全部丢失 |
| 图片数量 | 主要图片不应全部丢失 |
| caption | figure/table caption 前缀符合 profile |
| bibliography | 存在参考文献区或 report 中说明降级 |
| hyperlinks/bookmarks | 不得造成 DOCX 打开错误 |

### 7.3 内容级检查

| 指标 | 通过标准 |
| --- | --- |
| raw LaTeX 泄漏 | 常见命令泄漏数量低于阈值 |
| 公式 | OMML 数量大于 0；复杂公式 fallback 必须记录 |
| 引用 | `\cite`、`\ref`、`\label` 不应大量原样泄漏 |
| 中文 | 中文样本不应出现大面积乱码或字符丢失 |
| 算法 | algorithm/algorithm2e 至少保留可读文本和编号 |
| 表格 | booktabs/tabular 结构保持可读，不应退化为连续乱码 |

建议首轮阈值：

| 指标 | paper2 阈值 | paper3 阈值 |
| --- | ---: | ---: |
| compatibility score | >= 70 | >= 70 |
| raw fallback blocks | <= 15 | <= 20 |
| unresolved references | <= 10 | <= 15 |
| OMML equations | > 0 | > 0 |
| image assets | > 0 | > 0 |
| DOCX openable | 100% | 100% |

JOS 与中文学术 profile 应使用更高标准：

| Profile | 额外要求 |
| --- | --- |
| `jos-paper` | caption、reference、algorithm、中文/英文元数据重点核验 |
| `chinese-academic` | 中文标题、摘要、关键词、caption、字体和换行重点核验 |

## 八、人工核验清单

每个 DOCX 至少抽查以下内容：

| 区域 | 检查项 |
| --- | --- |
| 首页 | 标题、作者、机构、摘要、关键词是否出现且顺序合理 |
| 正文 | 章节顺序、标题层级、段落断裂、列表缩进 |
| 公式 | 行内/独立公式是否可读，是否可编辑，是否残留 raw LaTeX |
| 图片 | 图片是否出现、比例是否明显异常、caption 是否相邻 |
| 表格 | 表头、列宽、边框、跨行跨列、booktabs 效果 |
| 算法 | 算法标题、编号、步骤缩进、伪代码可读性 |
| 引用 | 文内引用、交叉引用、参考文献区 |
| 中文 | 字符、标点、断行、中文 caption、参考文献标题 |
| Word 打开 | Word/WPS/LibreOffice 打开是否提示修复 |

人工评级：

| 等级 | 定义 |
| --- | --- |
| A | 可直接作为可编辑 Word 初稿使用，仅需少量人工微调 |
| B | 内容完整，排版有明显问题，但可作为修订基础 |
| C | 主要内容可读，但公式/表格/引用有较多失败 |
| D | DOCX 可打开但内容严重缺失或大量 raw LaTeX |
| F | 无法生成或无法打开 |

发布门禁建议：

- 邀请制 Beta：14 个主矩阵中 A/B >= 10，且无 F。
- 付费 Beta：14 个主矩阵中 A/B >= 12，且 D <= 1，无 F。
- GA：14 个主矩阵中 A/B >= 13，无 D/F，JOS 和中文学术均为 A/B。

## 九、报告格式

`verify-summary.json` 建议结构：

```json
{
  "version": "1.0",
  "generated_at": "2026-06-22T00:00:00+08:00",
  "output_dir": "examples/output/journal-profile-generalization",
  "profiles": ["generic", "jos-paper", "tacl", "cvpr", "nature", "springer", "chinese-academic"],
  "papers": ["paper2", "paper3"],
  "results": [
    {
      "paper": "paper2",
      "profile_requested": "jos-paper",
      "profile_effective": "jos-paper",
      "backend_requested": "auto",
      "backend_selected": "luatex-node",
      "docx": "docx/paper2__jos-paper__auto.docx",
      "report": "reports/paper2__jos-paper__auto.report.json",
      "status": "passed",
      "grade": "B",
      "metrics": {
        "bytes": 0,
        "paragraphs": 0,
        "tables": 0,
        "images": 0,
        "omml_equations": 0,
        "raw_latex_hits": 0,
        "unresolved_references": 0,
        "compatibility_score": 0
      },
      "issues": []
    }
  ],
  "summary": {
    "total": 14,
    "passed": 0,
    "failed": 0,
    "grade_a_or_b": 0
  }
}
```

`verify-summary.md` 应包含：

1. 总体结论。
2. 14 项矩阵表。
3. 每个失败项的失败阶段。
4. 每个 profile 的共性问题。
5. paper2 与 paper3 的差异问题。
6. 下一轮修复建议。

## 十、失败分类

| 分类 | 说明 | 归属 |
| --- | --- | --- |
| `profile-detection` | profile 未识别或识别错误 | JournalDetector / ProfileRegistry |
| `profile-adaptation` | 专用入口生成失败或套壳不合法 | Fixture generator |
| `backend-selection` | backend 与 profile 不匹配，fallback 异常 | Backend selector |
| `compile-failed` | 编译或转换过程失败 | compiler-engine |
| `docx-package` | DOCX 缺核心部件或 zip 损坏 | docx-writer / packer |
| `structure-loss` | 标题、段落、表格、图片大量丢失 | semantic lowering / renderer |
| `math-quality` | 公式无法读或 raw LaTeX 泄漏 | mathml / OMML writer |
| `citation-quality` | 引用和参考文献异常 | bib / citation renderer |
| `cjk-quality` | 中文乱码、字体、断行异常 | profile font policy / CJK renderer |
| `style-quality` | caption、字体、页边距、标题样式不符 | profile style mapping |
| `open-failed` | Word/WPS/LibreOffice 无法打开 | OOXML 兼容性 |

## 十一、实施任务拆分

### V0：方案确认

交付物：

- 本方案文档。
- 14 项验证矩阵确认。
- 输出目录与报告 schema 确认。

### V1：样本入口生成器

交付物：

- `scripts/generate_journal_profile_inputs.ps1`
- `scripts/generate_journal_profile_inputs.sh`
- `examples/output/journal-profile-generalization/inputs/manifest.json`

验收：

- 14 个 `main.tex` 均生成。
- 每个入口可追溯到 paper2/paper3 原始章节。
- manifest 记录 profile、source、template、unsupported reason。

### V2：DOCX 批量生成脚本

交付物：

- `scripts/verify_paper2_paper3_journal_profiles.ps1`
- `scripts/verify_paper2_paper3_journal_profiles.sh`

验收：

- 14 个 DOCX 输出到 `examples/output/journal-profile-generalization/docx`。
- 14 个 report JSON 输出到 `reports`。
- 14 个 log 输出到 `logs`。
- 单个失败不影响后续矩阵继续执行。

### V3：自动质量核验

交付物：

- DOCX zip 部件检查。
- XML 指标统计。
- raw LaTeX 泄漏扫描。
- OMML、图片、表格、引用统计。
- `verify-summary.json` 与 `verify-summary.md`。

验收：

- 每个 DOCX 至少输出文件级、结构级、内容级指标。
- 失败项能自动分类。

### V4：人工核验与基线冻结

交付物：

- 人工核验表。
- 每个 DOCX 的 A/B/C/D/F 评级。
- 首轮质量基线。
- 下一轮修复 backlog。

验收：

- JOS、中文学术、generic 至少达到 B。
- 其余 profile 至少能生成可打开 DOCX，并有明确质量缺口。

## 十二、当前已知缺口

| 缺口 | 对验证的影响 | 建议 |
| --- | --- | --- |
| `tacl/cvpr/nature/springer` 在示例入口中仍映射到相近 enum | 可能无法真实体现 profile-specific style/rule | 报告中必须记录 effective profile；后续补动态 profile |
| `paper2` 当前为未跟踪本地目录 | CI 中可能不可用 | 若作为正式样本，应纳入版本控制或提供获取脚本 |
| 中文 profile 配置文件在当前终端显示乱码 | 控制台显示可能误导核验 | 以 UTF-8 文件和 Word 输出为准，核验时打开实际 DOCX |
| Word 打开验证未必在 CI 可用 | 自动门禁缺少最终 OOXML 兼容确认 | CI 用 LibreOffice；发布机补 Word/WPS 手工核验 |
| profile 专用入口尚未生成 | 无法直接跑完整 14 项矩阵 | V1 先实现套壳生成器 |

## 十三、首轮验收标准

首轮验证通过条件：

1. `examples/output/journal-profile-generalization/docx` 下生成 14 个 DOCX，或对未生成项给出明确失败分类。
2. 每个 DOCX 都有对应 report JSON 和 log。
3. `verify-summary.md` 给出矩阵结果和 A/B/C/D/F 人工评级。
4. `jos-paper`、`chinese-academic`、`generic` 三类核心 profile 至少对一篇真实文稿达到 B。
5. 所有失败项进入 backlog，并标记归属模块。

正式确认泛化能力的通过条件：

1. 14 个主矩阵全部可生成并可打开。
2. A/B 等级不少于 12 项。
3. 无 F，D 不超过 1 项。
4. JOS、中文学术、generic 均不少于 B。
5. `profile_requested` 与 `profile_effective` 不一致的项必须有明确说明或修复计划。

## 十四、建议下一步

1. 先实现 `scripts/generate_journal_profile_inputs.ps1`，把 paper2/paper3 构造成 14 个专用入口。
2. 再实现批量转换脚本，将所有 DOCX 输出到 `examples/output/journal-profile-generalization/docx`。
3. 接入 DOCX XML 自动检查，生成 `verify-summary.json`。
4. 手工打开 14 个 DOCX，完成 A/B/C/D/F 评级。
5. 根据失败分类，优先修复 profile detection、style mapping、math、citation、CJK 五类问题。
