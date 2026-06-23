# 多期刊 Profile：paper2/paper3 DOCX 泛化验证报告

**日期**：2026-06-22
**依据方案**：`docs-zh/verify/journal-profile-paper2-paper3-docx-generalization-verification-plan-20260622.md`
**执行范围**：`paper2`、`paper3` 两篇样稿 × 7 个默认期刊 profile，共 14 项主矩阵。
**输出目录**：`examples/output/journal-profile-generalization`

## 一、执行结论

本轮已完成 14 项矩阵转换验证，14 个 DOCX 均已生成，DOCX ZIP/核心 OOXML 部件均可被自动检查打开，未出现 F 级“无法生成/无法打开”项。

自动评级结果为：

| 等级 | 数量 |
| --- | ---: |
| A | 5 |
| B | 6 |
| C | 0 |
| D | 3 |
| F | 0 |

按原方案门槛判断：

| 发布阶段 | 是否满足 | 说明 |
| --- | --- | --- |
| 邀请制 Beta | 基本满足 | A/B=11，超过 10，且无 F。 |
| 付费 Beta | 未满足 | 要求 A/B>=12 且 D<=1；当前 A/B=11、D=3。 |
| GA | 未满足 | 要求无 D/F，且核心 profile 均达到 A/B 并完成真实 Word/WPS/LibreOffice 打开确认。 |

本轮最重要结论是：引擎对真实长文稿的 DOCX 生成链路已经具备可运行基础，但“多期刊专用格式泛化”仍未正式成立。原因是 `tacl/cvpr/nature/springer` 虽可通过动态 profile 指定并生成 DOCX，但输入仍主要复用 paper2/paper3 原始入口，没有完成各期刊专用模板壳的真实重写；因此本轮验证更准确地说是“多 profile 渲染兼容性验证”，还不是完整“多期刊投稿模板质量确认”。

## 二、产物清单

已生成：

- `examples/output/journal-profile-generalization/docx/`：14 个 DOCX。
- `examples/output/journal-profile-generalization/reports/`：14 个 report JSON。其中 2 个为质量门禁失败后补写的 synthetic report。
- `examples/output/journal-profile-generalization/logs/`：14 个转换日志。
- `examples/output/journal-profile-generalization/snapshots/`：14 个包级打开检查快照。
- `examples/output/journal-profile-generalization/verify-summary.json`
- `examples/output/journal-profile-generalization/verify-summary.md`
- `examples/output/journal-profile-generalization/inputs/manifest.json`
- `scripts/verify_paper2_paper3_journal_profiles.ps1`：本轮新增可复跑验证脚本。

## 三、验证矩阵结果

| 文稿 | 请求 profile | 实际 profile | 后端 | 兼容分 | DOCX 字节 | 段落 | 表格 | 媒体 | OMML | raw 命中 | 未解引用 | 等级 | 主要问题 |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |
| paper2 | generic | generic-article | RuleBased | 94 | 39061 | 793 | 19 | 0 | 225 | 4 | 0 | A | 无媒体文件；请求/实际 profile ID 规范不一致 |
| paper2 | jos-paper | 未写出完整 report | 未写出完整 report | 0 | 21474 | 282 | 8 | 0 | 73 | 4 | 0 | D | quality gate 失败；IEEE/JOS 宏 raw fallback；无媒体 |
| paper2 | tacl | tacl | RuleBased | 82 | 39061 | 793 | 19 | 0 | 225 | 4 | 0 | A | 无媒体文件 |
| paper2 | cvpr | cvpr | RuleBased | 88 | 18680 | 282 | 8 | 0 | 73 | 4 | 0 | D | DOCX 小于 20KB；无媒体文件 |
| paper2 | nature | nature | RuleBased | 82 | 39065 | 793 | 19 | 0 | 225 | 4 | 0 | A | 无媒体文件 |
| paper2 | springer | springer | RuleBased | 82 | 39062 | 793 | 19 | 0 | 225 | 4 | 0 | A | 无媒体文件 |
| paper2 | chinese-academic | chinese-academic | RuleBased | 88 | 38415 | 794 | 21 | 0 | 177 | 2 | 0 | A | 无媒体文件 |
| paper3 | generic | generic-article | RuleBased | 76 | 3054778 | 650 | 11 | 10 | 192 | 0 | 0 | B | 请求/实际 profile ID 规范不一致 |
| paper3 | jos-paper | jos-paper | RuleBased | 64 | 3057672 | 650 | 11 | 10 | 192 | 0 | 0 | B | 兼容分低于 70；但 JOS 奇偶页页眉已正确抽取 |
| paper3 | tacl | tacl | RuleBased | 64 | 3054778 | 650 | 11 | 10 | 192 | 0 | 0 | B | 兼容分低于 70；仍非真实 TACL 模板壳 |
| paper3 | cvpr | cvpr | RuleBased | 64 | 3054778 | 650 | 11 | 10 | 192 | 0 | 0 | B | 兼容分低于 70；仍非真实 CVPR 模板壳 |
| paper3 | nature | nature | RuleBased | 64 | 3054782 | 650 | 11 | 10 | 192 | 0 | 0 | B | 兼容分低于 70；仍非真实 Nature 模板壳 |
| paper3 | springer | 未写出完整 report | 未写出完整 report | 0 | 3054777 | 650 | 11 | 10 | 192 | 0 | 0 | D | quality gate 失败；兼容分 58 < preview 阈值 60 |
| paper3 | chinese-academic | chinese-academic | RuleBased | 70 | 3053677 | 632 | 11 | 10 | 200 | 2 | 0 | B | 达到本轮阈值下限 |

## 四、关键质量观察

### 4.1 DOCX 包完整性

14 个 DOCX 均通过以下自动检查：

- 文件存在且可作为 ZIP 打开。
- 包含 `[Content_Types].xml`。
- 包含 `word/document.xml`。
- 包含 `word/styles.xml`。
- 包含 `word/_rels/document.xml.rels`。

这说明当前 writer/packer 在主矩阵中没有出现 DOCX 包损坏或核心部件缺失问题。

### 4.2 paper3 的 JOS 页眉验证

`paper3__jos-paper__auto.docx` 的自动快照显示：

- `header1_text=石 洪 雷 等:基于动态关注清单的微服务日志定向采集方法 1`
- `header2_text=1 Journal of Software 软件学报`

这与 `rjthesis.cls` 中“首页单独处理、后续奇数页短题名左/页码右、偶数页页码左/期刊名右”的规则一致。上一轮修复的 `\rjhead` 抽取与奇偶页页眉布局已在真实 paper3 转换中生效。

### 4.3 paper2 图片/媒体缺失

paper2 所有 profile 的 `media_files=0`。这说明 paper2 的图像资源没有进入 DOCX 包，可能原因包括：

- paper2 原始图像多为 SVG，而当前 DOCX 图片嵌入链路更偏向 PNG/JPEG。
- profile 入口复用原始 TeX 时，图片路径或格式未被转换器完整接入。
- 当前自动评分对“无媒体”只记 issue，未强制降到 C/D，因此 paper2 多项自动 A 需要人工复核后谨慎解释。

建议把 paper2 的无媒体问题作为下一轮最高优先级之一处理，否则不能用它证明图文类论文的泛化质量。

### 4.4 动态 profile 与真实模板壳仍有差距

`tacl/cvpr/nature/springer` 当前可以作为 `ProfileRef::Id` 被指定，并生成对应 active profile 的报告，但本轮没有真正生成各期刊专用模板壳。结果表现为：

- paper3 在 tacl/cvpr/nature 下仍主要沿用 JOS 正文入口，兼容分均为 64。
- paper3 springer 质量门禁失败，兼容分 58。
- paper2 cvpr 虽有效 profile 为 cvpr，但 DOCX 仅 18,680 字节，低于 20KB 文件级阈值。

因此，这些 profile 当前只能证明“profile 指定和基础渲染不会崩”，不能证明“专用投稿格式已达标”。

## 五、失败/降级项分类

| 项 | 分类 | 现象 | 建议归属 |
| --- | --- | --- | --- |
| paper2 + jos-paper | `profile-adaptation` / `structure-loss` | `\begin{abstract}`、`\begin{IEEEkeywords}` fallback，quality gate 失败 | rule-engine / profile adapter |
| paper2 + cvpr | `structure-loss` / `docx-package-threshold` | DOCX 18,680 字节，小于 20KB；媒体缺失 | profile input / renderer |
| paper3 + springer | `profile-adaptation` | compatibility score 58 < 60，quality gate 失败 | profile rules / compatibility analyzer |
| paper2 全部 profile | `media-quality` | media_files=0 | image resolver / SVG conversion |
| generic profile | `profile-id-normalization` | requested `generic`，effective `generic-article` | profile alias/report normalization |

## 六、是否达到方案验收

| 验收项 | 结果 |
| --- | --- |
| 14 个 DOCX 输出 | 通过 |
| 14 个 report JSON 和 log | 通过；其中 2 个 report 为 synthetic failure report |
| 自动质量指标汇总 | 通过 |
| A/B >= 10 且无 F | 通过 |
| A/B >= 12 且 D <= 1 | 未通过 |
| JOS、中文学术、generic 核心 profile 至少一篇真实文稿达到 B | 通过 |
| 全部 profile 专用格式质量正式确认 | 未通过 |
| Word/WPS/LibreOffice 真实打开验证 | 未完成；本机未发现可自动调用的 `soffice`/`winword` 命令 |

## 七、下一轮修复计划

1. 优先修复 paper2 图片嵌入：确认 SVG 输入策略，必要时在验证前生成 PNG 派生图，或在 renderer 中支持 SVG 转换/降级。
2. 为 `tacl/cvpr/nature/springer` 实现真正的 profile 专用入口生成器，而不是仅复用原始 main 文件。
3. 修复 paper2 JOS/IEEE 宏 fallback：至少覆盖 `abstract`、`IEEEkeywords` 等环境。
4. 调整 CLI：即使 quality gate 失败，也应先写出完整 CompileReport，再以退出码表达失败，避免本轮这种 synthetic report 补写。
5. 对 paper3 springer 做兼容性规则复核，降低“JOS 正文套 Springer profile”时的误伤，或在报告中明确标为不支持的适配方式。
6. 增加 Word/WPS/LibreOffice 打开验证；CI 可用 LibreOffice，发布机保留人工 Word/WPS 抽检。

## 八、最终判定

当前项目可以进入“邀请制 Beta 前的技术验证继续推进”阶段，但还不能宣称已全面支持多期刊高质量 TeX 到 DOCX 转换。

建议商业化宣传口径保持保守：可强调“已支持多 profile 识别、转换和质量报告，JOS/paper3 长文稿页眉页脚等关键格式已改善”；暂不宣传 “TACL/CVPR/Nature/Springer 投稿模板一键高保真转换”。

## 九、Office 打开验证补充

**补充日期**：2026-06-23
**验证前提**：本机已安装并登录 Microsoft Word，LibreOffice 也已安装。

验证工具：

- Microsoft Word：`C:/Program Files/Microsoft Office/root/Office16/WINWORD.EXE`，版本 `16.0.20026.20182`。
- LibreOffice：`C:/Program Files/LibreOffice/program/soffice.com`，版本 `26.2.4.2`。

验证结果：

| 工具 | 方式 | 通过 | 失败 | 结论 |
| --- | --- | ---: | ---: | --- |
| Microsoft Word | COM `OpenNoRepairDialog` 打开/关闭 | 0 | 14 | 全部报“文件可能已经损坏。” |
| LibreOffice | headless 转 PDF | 14 | 0 | 全部可打开并转出 PDF |

详细结果已写入：

- `examples/output/journal-profile-generalization/word-open-results.json`
- `examples/output/journal-profile-generalization/soffice-open-results.json`
- `examples/output/journal-profile-generalization/soffice-pdf/`
- 各项 `examples/output/journal-profile-generalization/snapshots/*.word-open.txt`

### 补充结论

上一轮“DOCX 包级/OOXML 部件检查通过”的结论仍然成立，但 Microsoft Word 真实打开验证全部失败，说明当前 DOCX 对 Word 的严格 OOXML 兼容性仍有结构性问题。LibreOffice 能打开并转 PDF，说明文件不是完全不可读；问题更可能集中在 Word 更严格检查的 OOXML schema、relationship、section/header/footer、styles 或某些 run/field 结构上。

因此，本报告的发布判定需要收紧：

| 发布阶段 | 原判定 | Office 打开验证后判定 |
| --- | --- | --- |
| 邀请制 Beta | 基本满足 | 暂缓；只能用于内部技术验证 |
| 付费 Beta | 未满足 | 未满足 |
| GA | 未满足 | 未满足 |

下一轮最高优先级调整为：

1. 先定位 Word 报“文件可能已经损坏”的最小复现 DOCX。
2. 用 Word 可打开性作为最高优先级质量门禁，修复前不得将 DOCX 输出视为可交付。
3. 对 `docx-writer` 生成的核心部件逐项做 Word 兼容性差分：`document.xml`、`styles.xml`、`numbering.xml`、`settings.xml`、header/footer relationships、field/simple field、table grid。
4. 修复后重新跑 14 项矩阵，要求 Word 14/14 可打开，再继续评估排版质量。
