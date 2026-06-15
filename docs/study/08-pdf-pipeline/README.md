# 第八章 · PDF 流水线（V2 草案）

> 本章是 V1 → V2 的演进路线。V1（[../01-overview/01-features.md](../01-overview/01-features.md) §1.5）只生成 docx；
> V2 在 Rust 流水线之外新增 **docx→PDF 同步生成** 与 **TeX oracle 质量对比** 两条新路径，
> 实现「docx 与 pdf 同源于 `.tex`，并以原生 TeX PDF 为质量锚点」的目标。
>
> **面向读者**：要在 V1 基础上扩出 PDF 通道、把当前 docx 质量与原生 TeX 编译结果做闭环对比的工程师 / 架构师。

---

## 本章目标

1. 在 V1 纯 Rust 解析主线之外，新增 **3 个 Rust crate**（`tex-facade` / `docx-pdf` / `quality`），共约 1800 行新增代码（参考 [../../to-docx/09-rust-port.md](../../to-docx/09-rust-port.md) §9.9 的 LOC 估算）。
2. 在原 V1 端到端产物 `vN-论文稿件-jos-TS.docx` 基础上，同步产出 `vN-论文稿件-jos-TS.pdf`（LibreOffice headless 二次转换）与 `vN-论文稿件-jos-TS.oracle.pdf`（TeX 原生编译）。
3. 引入 **结构 + 文本 + 视觉三层质量回归**，让"docx/pdf 质量不低原生"成为可量化的 CI 卡点。

## 与 V1 边界的关系

V1 文档 [../01-overview/01-features.md §1.5](../01-overview/01-features.md) 明确写道：

> "完整 LaTeX 引擎（编译数学宏包、引用解析）❌ 不支持 — 纯解析器，不调用 TeX。"

V2 **不打破这条边界**：

- **V1 主线（产物生成）依旧纯 Rust**：自 [crates/latex-reader/](../../../crates/latex-reader/) 到 [crates/docx-writer/](../../../crates/docx-writer/) 不引入任何 TeX / LibreOffice 进程。
- **V2 新增（仅用于质量对比）**：`crates/tex-facade` 在 **校验阶段** 才拉起外部 TeX 引擎与 LibreOffice，并把结果作为 oracle 留痕；这些产物不进入 [crates/docx-writer/](../../../crates/docx-writer/) 的输入。
- **失败策略分级**：oracle 编译失败 → 仅 warning（不强阻断 V1 主线）；docx→pdf 失败 → exit 1；视觉层超阈值 → exit 2。

## 与 `docs/to-docx/` 的关系

| 关注点 | `to-docx/` 9 篇 | 本章 5 篇 |
|--------|----------------|----------|
| 文档定位 | Python→Rust 重构蓝图（已有 `build_jos_docx.py`） | V1→V2 演进路线（在 V1 Rust 流水线外增量） |
| DOCX 生成 | 是（详尽到单行算法） | 沿用 [crates/docx-writer/](../../../crates/docx-writer/)，不重写 |
| PDF 生成 | 否（仅作为 oracle 文本源） | **是**（新增 `docx-pdf` crate） |
| 质量对比 | 仅 DOCX↔PDF 文本覆盖（[08-verification.md](../../to-docx/08-verification.md) 33 项） | 三层（结构 + 文本 + 视觉） |
| TeX 调用 | shell 脚本 | Rust facade crate，可插拔 |

## 阅读路径

| # | 文档 | 解决的问题 |
|---|------|----------|
| 0 | [00-v2-overview.md](./00-v2-overview.md) | V2 整体综述：是什么、解决什么、5 篇之间的衔接、与 `to-docx/` 的对照、风险与 V1.5 边界的关系 |
| 1 | [01-pipeline-overview.md](./01-pipeline-overview.md) | V2 端到端长什么样、产物怎么命名、CI 怎么跑 |
| 2 | [02-tex-facade.md](./02-tex-facade.md) | `crates/tex-facade` 如何在 Rust 端封装 xelatex / tectonic / latexmk |
| 3 | [03-docx-to-pdf.md](./03-docx-to-pdf.md) | `crates/docx-pdf` 如何把 docx 转成 PDF，含 LibreOffice headless 进程管理 |
| 4 | [04-quality-comparison.md](./04-quality-comparison.md) | 结构 / 文本 / 视觉三层怎么测、怎么判定"不低" |
| 5 | [05-implementation-roadmap.md](./05-implementation-roadmap.md) | M1–M5 五个阶段任务、依赖、风险、回滚预案 |
| 6 | [06-progress-2026-06-15.md](./06-progress-2026-06-15.md) | **实施过程日志**：M2 `tex-facade` 进展快照（已完成代码、当前阻塞、已尝试修复、下一步）。**非设计稿**，不入发布版。 |

## 状态

- 本章（[00](./00-v2-overview.md) ~ [05](./05-implementation-roadmap.md)）为 **设计稿**（V2 草案）。
- **更新（2026-06-15 11:20）**：M1 骨架已落（HEAD `0c3fa10`）；M2 `tex-facade` 编码 **完成**（1585 行）；单元测试 **16 / 16 通过 / 3 `#[ignore]`**——3 项 `#[ignore]` 集成测试需 CI runner + 预热 xelatex FNDB，本机 MiKTeX 未预热故跳过。`>60s` 阻塞根因是 multi-thread tokio `Runtime::drop()` 在 Windows 上 join worker 线程，改 `current_thread` runtime 解决，详见 [06-progress-2026-06-15.md §6.10](./06-progress-2026-06-15.md)。
- 实施需在 M1 阶段同时获取 `examples/paper3/latex/main-jos.pdf` 与 `main-jos.bbl` 作为 oracle 锚点（已就绪）。
- 本章不进入 V1.3 发布版；若开始实施则同步更新 [../01-overview/01-features.md §1.5](../01-overview/01-features.md) 与 [../README.md §"配套原始文档"](../README.md)。

## 配套原始文档

* [00-v2-overview.md](./00-v2-overview.md) — V2 整体综述（章前导读）
* [../../to-docx/00-index.md](../../to-docx/00-index.md) ~ [09-rust-port.md](../../to-docx/09-rust-port.md) — Python→Rust 转换蓝图与 33 项校验方法
* [../01-overview/01-features.md](../01-overview/01-features.md) — V1 功能与边界
* [../04-architecture/01-end-to-end-pipeline.md](../04-architecture/01-end-to-end-pipeline.md) — V1 端到端流水线
* [../05-key-tech/04-docx-serialization.md](../05-key-tech/04-docx-serialization.md) — V1 docx 序列化细节
* [../07-deployment/06-ci-and-hooks.md](../07-deployment/06-ci-and-hooks.md) — V1 CI 矩阵
