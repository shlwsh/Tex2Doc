# 00 · V2 PDF 流水线整体综述

> 本章是 V2 设计与实现状态的"章前导读"。后 5 篇（[01](./01-pipeline-overview.md) ~ [05](./05-implementation-roadmap.md)）各自聚焦 1 个 crate 或 1 个阶段；本章站在更高一层回答：
> 1. V2 是什么、要解决什么、相比 V1 多了什么；
> 2. 5 篇之间的**衔接关系**与"先读哪篇、后读哪篇"；
> 3. V2 草稿与 [`../../to-docx/`](../../to-docx/) 9 篇（Python→Rust 重构蓝图）如何**对照**、如何避免重复。
>
> **目标读者**：项目维护者、新加入的工程师、要做 V2 立项评审的 TL。建议**先读本章，再按 §0.5 阅读路径读后 5 篇**。

---

## 0.1 V2 是什么

V1（[../01-overview/01-features.md §1.5](../01-overview/01-features.md)）只做一件事：**`.tex` → Rust 解析 → `.docx`**。它的边界是"纯解析器，不调用 TeX"。

V2 在 V1 之外**叠加** 3 条新路径，并已通过 `doc-engine` CLI 串联：

| 路径 | 何时跑 | 做什么 | 失败时 |
|------|-------|-------|-------|
| **A. TeX oracle 编译** | 校验阶段 | 用外部 `xelatex` / `tectonic` / `latexmk` 把 `.tex` 编译成 `*.oracle.pdf`，并抽 oracle 文本 | 仅 warning，不阻断 |
| **B. docx → PDF 转换** | 校验阶段 | 用 LibreOffice headless 把 V1 写出的 docx 转成 `*.pdf` | exit 1（主失败） |
| **C. 三层质量对比** | 校验阶段 | 把 `*.docx` ↔ `*.pdf` ↔ `*.oracle.pdf` 三者做结构 / 文本 / 视觉三层对比 | 按层定退出码 |

> 这 3 条路径的最终产物是 Rust DOCX、Rust PDF、TeX oracle PDF 与一份三层合并质量报告。具体命名由 `doc-engine build` 或 paper3 脚本决定。

## 0.2 V2 要解决什么问题

| 痛点 | V1 现状 | V2 解法 |
|------|--------|--------|
| 稿子要"Word + PDF 双交付"，V1 只能给 Word | 用户得自己拿 docx → LibreOffice → pdf，**没人验证质量** | 路径 B 在 V1 流水线内**自动产出 PDF**，路径 C 验证它**不低**原生 |
| Rust 端"自以为写对了"，但与原生 TeX 比对没有锚点 | [to-docx/08-verification.md](../../to-docx/08-verification.md) 33 项只覆盖 docx↔PDF 文本 | 路径 A 引入 oracle 锚点；路径 C 扩展到 3 层 66+ 项 |
| "质量回归"靠人眼抽检 | 没有 CI 卡点 | `build_docx_and_pdf.sh` + `verify --layer all` + 三层 exit code 0/1/2 |
| 自由地"调" V1 docx 序列化代码——但没有任何 oracle 守护 | 没人能在改 docx writer 后判断"是否变差" | oracle PDF 是**只读锚点**，可重入 diff 报告 |

## 0.3 V1 ↔ V2 边界对照

V1 主线（[crates/latex-reader/](../../../crates/latex-reader/) → [crates/docx-writer/](../../../crates/docx-writer/)）继续保留。V2 已在 workspace 增量落地以下 crate：

| V1 crate/入口 | V2 新增 crate | 关系 |
|----------------|--------------|------|
| `latex-reader` / `semantic-ast` / `docx-writer` / 其它 6 个 | `tex-facade` | V2 路径 A；**只读** `.tex` 源 |
| 同上 | `docx-pdf` | V2 路径 B；**只读** V1 写出的 `.docx` |
| 同上 | `quality` | V2 路径 C；**只读** docx + 两份 PDF |
| 同上 | `cli` (`doc-engine`) | 命令行串联 convert / oracle / pdf / verify |
| 同上 | `compiler-engine` | Semantic TeX Engine facade，输出 `DocumentGraph` 和 `CompileReport` |

> **关键不变量**：V2 任何路径失败**不修改**已写盘的 docx；oracle 命名带 `.oracle.pdf` 后缀，**绝不与 Rust PDF 同名**（避免覆盖）。

## 0.4 V2 草稿与 `to-docx/` 9 篇的对照

V2 不是从零写——`to-docx/` 9 篇是 Python→Rust 重构蓝图，提供了 33 项结构校验、IR 模型、crate 选型、模块划分。V2 直接**沿用其脚手架**，避免重复造轮子：

| 关注点 | `to-docx/` 9 篇（V1 蓝图） | V2 5 篇（V2 草稿） |
|--------|--------------------------|-------------------|
| DOCX 生成 | [05-wpml-emission.md](../../to-docx/05-wpml-emission.md) + [06-zip-relationships.md](../../to-docx/06-zip-relationships.md)：详尽到行级算法 | **沿用** V1 已有 crate，不重写 |
| PDF 生成 | ❌ 不涉及 | **是**——[03-docx-to-pdf.md](./03-docx-to-pdf.md) 新增 `docx-pdf` crate |
| TeX 调用 | shell 脚本直调 | **抽象为 trait**——[02-tex-facade.md](./02-tex-facade.md) 新增 `tex-facade` crate |
| 质量对比 | 33 项 docx↔PDF 文本（[08-verification.md](../../to-docx/08-verification.md)） | **扩展到 3 层 66+ 项**——[04-quality-comparison.md](./04-quality-comparison.md) |
| 实施排期 | [09-rust-port.md §9.9](../../to-docx/09-rust-port.md) LOC 估算 | **沿用 LOC 估算，重排时间轴**——[05-implementation-roadmap.md](./05-implementation-roadmap.md) |
| Rust 数据结构 | [09-rust-port.md §9.4](../../to-docx/09-rust-port.md) `Manuscript` / `Block` / `AlgRow` | **沿用**，V2 不引入新 IR 类型 |
| 关键术语 | `twip` / `half-point` / `EMU` / `rId` / `Twp` / `WPML` | **沿用**；V2 仅新增 3 个术语，见 §0.7 |

> **重要约束**：V2 5 篇中**凡涉及 IR、crate 选型、33 项校验方法的描述，必须与 `to-docx/` 9 篇保持一致**。如有不一致，以 `to-docx/` 为准（V2 是消费者，不是定义者）。

## 0.5 推荐阅读路径

按下面顺序读 5 篇，每读一篇回到本章的"衔接点"看下它在 V2 整体中的位置：

1. **第 1 站：[01-pipeline-overview.md](./01-pipeline-overview.md)** —— V2 端到端长什么样
   - **衔接点**：看完 §1.2 数据流图后，回 §0.2 验证 3 条路径已覆盖你的关注点
2. **第 2 站：[02-tex-facade.md](./02-tex-facade.md)** —— `tex-facade` crate 设计
   - **衔接点**：看完 §2.4 trait 定义后，回 §0.3 确认它**不**被 V1 crate 引用
3. **第 3 站：[03-docx-to-pdf.md](./03-docx-to-pdf.md)** —— `docx-pdf` crate 设计
   - **衔接点**：看完 §3.3 进程管理后，回 §0.3 确认它**只读** docx
4. **第 4 站：[04-quality-comparison.md](./04-quality-comparison.md)** —— `quality` crate 三层对比
   - **衔接点**：看完 §4.2 三层阈值后，回 §0.3 确认它**不写回**任何 V1 产物
5. **第 5 站：[05-implementation-roadmap.md](./05-implementation-roadmap.md)** —— M1–M5 时间轴
   - **衔接点**：看完 §5.3 风险后，回 §0.6 检查风险是否在 V1.5 边界表里

## 0.6 风险与 V1.5 边界表的关系

V1 的"边界表"是 V2 唯一不能动的约束。V2 落地时必须保证：

| V1.5 边界声明（[01-features.md §1.5](../01-overview/01-features.md)） | V2 是否守住 | 措施 |
|----------------------------------|----------|------|
| ❌ 完整 LaTeX 引擎 | ✅ 守住 | TeX **仅**在 `tex-facade` 内被拉起；V1 crate 零依赖 |
| ❌ PDF 生成 | ❌ **打破**——V2 **新增** PDF 生成 | 在 V1.5 边界表追加 1 行："✅ V2 已支持：docx→PDF 同步输出" |
| ❌ 真实 TeX oracle 对比 | ❌ **打破**——V2 **新增** oracle 路径 | 在 V1.5 边界表追加 1 行："✅ V2 已支持：TeX oracle 质量对比" |
| ❌ docx/PDF 之外的格式（EPUB / HTML） | ✅ 守住 | V2 不涉及 |
| ❌ 视觉层 OCR 逐字符校验 | ✅ 守住 | 仅做 spike，不进 V2 主线（见 [05 §5.3 关键风险 #3](./05-implementation-roadmap.md)） |

> **任何 V2 设计如果与上表冲突，必须回到本章 §0.6 同步更新 V1.5 边界表**——而不是悄悄打破 V1 边界。

## 0.7 V2 新增术语（仅 3 个）

V2 不重新发明 IR 词汇表，只在 V1 基础上**新增 3 个与"质量对比"相关的术语**：

| 术语 | 含义 | 出现位置 |
|------|------|---------|
| **oracle PDF** | 由 `tex-facade` 调用原生 TeX（xelatex / tectonic / latexmk）编译出的 `*.oracle.pdf`；是 docx/pdf 质量的"锚点" | [01 §1.3](./01-pipeline-overview.md) / [02 §2.1](./02-tex-facade.md) |
| **三层质量对比** | 结构层（33+4 项）+ 文本层（字符比例、marker 覆盖、章节覆盖）+ 视觉层（SSIM、像素差、OCR） | [04 §4.1](./04-quality-comparison.md) |
| **视觉降级（exit 2）** | 视觉层 fail 时退出码 2，与"主失败（exit 1）"区分；CI 上发 PR comment 而非强制失败 | [01 §1.5](./01-pipeline-overview.md) / [04 §4.6](./04-quality-comparison.md) |

## 0.8 状态

- 本章保留设计综述，同时反映 2026-06-20 的实现状态。
- `tex-facade`、`docx-pdf`、`quality`、`cli` 已进入 workspace。
- `doc-compiler-engine` 已新增为 Semantic TeX Engine facade，支持 source/dir/zip/VFS → DOCX。
- paper3 compiler-engine 脚本已可生成 `examples/paper3/output/to-docx/*-compiler-engine.docx`。
- 最新细节见 [07-progress-2026-06-20.md](./07-progress-2026-06-20.md)。

## 0.9 配套原始文档

* [01-pipeline-overview.md](./01-pipeline-overview.md) ~ [05-implementation-roadmap.md](./05-implementation-roadmap.md) — V2 5 篇设计基线
* [07-progress-2026-06-20.md](./07-progress-2026-06-20.md) — 最新实现快照
* [../../to-docx/00-index.md](../../to-docx/00-index.md) ~ [09-rust-port.md](../../to-docx/09-rust-port.md) — Python→Rust 转换蓝图与 33 项校验方法
* [../01-overview/01-features.md](../01-overview/01-features.md) — V1 功能与边界（**V2 唯一不能动的约束**）
* [../04-architecture/01-end-to-end-pipeline.md](../04-architecture/01-end-to-end-pipeline.md) — V1 端到端流水线（V2 的主干）
* [../05-key-tech/04-docx-serialization.md](../05-key-tech/04-docx-serialization.md) — V1 docx 序列化细节
* [../07-deployment/06-ci-and-hooks.md](../07-deployment/06-ci-and-hooks.md) — V1 CI 矩阵
