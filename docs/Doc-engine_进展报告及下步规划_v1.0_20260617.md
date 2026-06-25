# Doc-engine 进展报告及下步规划
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



> 版本：v1.0  
> 日期：2026-06-17  
> 对应方案：`docs/Doc-engine_完善设计实现方案_v2.0_20260617.md`  
> 对应任务清单：`docs/Doc-engine_开发任务清单_v2.0_20260617.md`

## 1. 总体结论

按 `Doc-engine_开发任务清单_v2.0_20260617.md` 定义的开发和验收范围，当前已全部实现，且 `examples/paper3/latex/main-jos.tex` 的 Rust 主链路转换已通过质量门。

但如果按 `Doc-engine_完善设计实现方案_v2.0_20260617.md` 的最终愿景理解，“全部实现”还不能等同于长期完成。当前完成的是 v2.0 里为 paper3/JOS 高保真转换设定的可验收闭环，包括标准规则库雏形、标准 AST、AST dump、DOCX Render Tree、Rust 主链路关键质量修复、质量报告和本地回归脚本。设计方案中更长期的多模板泛化、Profile 完全驱动、质量模块 Rust 原生化、SQLite 运行期规则缓存、多文档回归矩阵等，仍应作为下一阶段推进。

## 2. 当前完成范围

### 2.1 Phase 完成情况

| Phase | 主题 | 状态 | 当前证据 |
|---|---|---|---|
| Phase 0 | JOS 高保真基线固化 | 已完成 | `scripts/build_docx.sh`、`scripts/build_jos_docx.py`、`scripts/verify_jos_docx.py` 已接入，格式 JSON 和输入 hash manifest 已存在 |
| Phase 1 | 标准规则库与 Profile | 已完成 | `standards/`、`profiles/jos-2025/`、`standards.lock.json`、规则缓存策略文档已存在 |
| Phase 2 | 标准文档 AST | 已完成 | `StandardDocument`、metadata、numbering、resources、diagnostics、block/inline/layout 结构已落地 |
| Phase 3 | AST dump | 已完成 | `main-jos.ast.json`、`main-jos.ast.md` 可生成，AST blocks=156，diagnostics=0 |
| Phase 4 | Mapping Registry 与 DOCX Render Tree | 已完成 | `main-jos.render.json` 可生成，render nodes=156，diagnostics=0，media=10 |
| Phase 5 | Rust 主链路 P0/P1 修复 | 已完成 | 图片、算法、表格、引用、label/ref、数学符号、页面设置和参考文献已收敛到质量门通过 |
| Phase 6 | 质量体系产品化 | 已完成 | verify JSON schema、schema 校验、traceability 报告和 `scripts/paper3_regression.sh` 已接入 |

### 2.2 paper3 最新质量结果

最新 `examples/paper3/output/main-jos.verify.json`：

| 指标 | 当前值 |
|---|---:|
| verify passed | true |
| failed checks | 0 |
| DOCX/PDF 字符比例 | 0.8759833630421866 |
| 参考文献条目数 | 56 |
| 图片数 | 10 |
| 表格数 | 12 |

最新 `examples/paper3/output/main-jos.traceability.json`：

| 指标 | 当前值 |
|---|---:|
| traceability passed | true |
| AST blocks | 156 |
| Render nodes | 156 |
| DOCX paragraphs/tables/media | 255 / 12 / 10 |
| Render mapping rule kinds | 8 |

### 2.3 已实现的关键设计点

| 设计点 | 当前状态 |
|---|---|
| `TeX -> Standard Doc AST -> Mapping Registry -> DOCX Render Tree -> Renderer/Packer -> Quality Report` 分层 | 已形成可运行闭环 |
| AST 可输出 JSON/Markdown | 已实现，`ast-dump` 支持 `json` / `md` |
| Render Tree 可输出 JSON/Markdown | 已实现，`render-dump` 支持 `json` / `md` |
| AST 节点标注规则 id | 已实现，覆盖 section、figure、table、tabular、algorithm、math、theorem-like 等 |
| DOCX 映射规则覆盖统计 | 已实现，traceability 报告显示 `map.*.docx` 覆盖 |
| JOS 页面设置、页眉页脚、首页 masthead、图片、表格、算法、公式、引用、参考文献质量门 | 已通过 `verify_jos_docx.py` |
| `.bbl` 引用顺序与参考文献条目进入 Rust 主链路 | 已实现 |
| label/ref 两遍解析 | 已实现 |
| booktabs/tabular* 表格清洗 | 已实现 |
| 常见数学符号和上下标处理 | 已实现 |
| 规则缓存策略 | 已形成文档决策：首版文件规则为事实源，SQLite 仅作为后续可重建派生缓存 |

## 3. 尚未等同“最终完成”的范围

当前实现满足 v2.0 任务清单和 paper3/JOS 验收目标，但以下内容仍属于设计方案的长期目标或生产化增强，不应宣称已经完全完成。

| 未完成方向 | 当前状态 | 建议归属 |
|---|---|---|
| 多模板 Profile 泛化 | 当前重点是 `jos-2025` 和 paper3；其他期刊/通用模板尚未建立完整回归集 | 下一阶段 |
| Profile 完全驱动 writer | 部分规则已文件化，但 `docx-writer` 中仍有 JOS 相关硬编码 | 下一阶段 |
| `quality` crate 原生吸收 Python 校验 | 当前质量门主要由 `scripts/verify_jos_docx.py` 和辅助脚本承担 | 下一阶段 |
| SQLite 规则缓存 | 已评估并暂不引入；目前仍以 YAML/JSON + lock 文件为事实源 | 后续可选 |
| TeX artifact 标准化 API | `.bbl` 已进入主链路，PDF/aux/log artifact 仍需进一步结构化 | 下一阶段 |
| 多文档回归矩阵 | 当前只有 paper3 作为强回归样本 | 下一阶段 |
| OOXML schema 级校验 | 当前以结构检查和 WordprocessingML 经验规则为主，尚未接入完整 XSD/OPC validator | 后续增强 |
| WASM/Web 高保真降级策略 | 当前本地/CLI 高保真优先，WASM 无 TeX 工具链场景仍需明确降级报告 | 后续增强 |

## 4. 最新验证命令

本轮已确认以下命令通过：

```bash
cargo fmt --all --check
cargo check -p doc-engine
cargo test -p doc-docx-writer
cargo test -p doc-core --test paper3_e2e paper3_main_jos_to_docx -- --nocapture
python3 scripts/validate_verify_report_schema.py examples/paper3/output/main-jos.verify.json
./scripts/paper3_regression.sh
```

`./scripts/paper3_regression.sh` 当前返回 `0`，输出包括：

- `examples/paper3/output/main-jos.docx`
- `examples/paper3/output/main-jos.ast.json`
- `examples/paper3/output/main-jos.ast.md`
- `examples/paper3/output/main-jos.render.json`
- `examples/paper3/output/main-jos.render.md`
- `examples/paper3/output/main-jos.verify.json`
- `examples/paper3/output/main-jos.verify.md`
- `examples/paper3/output/main-jos.traceability.json`
- `examples/paper3/output/main-jos.traceability.md`

## 5. 下步规划

### 5.1 P7：Profile 驱动化

目标：减少 writer 层 JOS 硬编码，把模板规则迁移到 `profiles/jos-2025` 和 mapping 文件。

建议任务：

| ID | 任务 | 验收 |
|---|---|---|
| P7-01 | 抽取 JOS 页面设置、页眉、masthead、表格、算法、公式、参考文献规则到 Profile 文件 | 修改 Profile 后 render/verify 行为可追踪变化 |
| P7-02 | `docx-writer` 读取 Profile/MappingRegistry，而不是直接写死 JOS 常量 | `paper3_regression.sh` 仍返回 0 |
| P7-03 | render dump 中输出 Profile override 来源 | traceability 能定位每条 JOS 特例来源 |

### 5.2 P8：质量模块 Rust 原生化

目标：把 Python 校验脚本中的核心检查沉淀到 `crates/quality`，保留 Python 脚本作为 oracle 或兼容入口。

建议任务：

| ID | 任务 | 验收 |
|---|---|---|
| P8-01 | 在 `quality` crate 中实现 DOCX package/XML 结构快照 | Rust 侧可读取 paragraphs/tables/media/rels/page setup |
| P8-02 | 移植 JOS 结构检查：图片、表格、算法、公式、引用、参考文献、页眉页脚 | Rust report 与 Python verify 结果一致 |
| P8-03 | CLI 增加 `quality-check` 子命令 | `doc-engine quality-check --docx ... --pdf ...` 输出 JSON/MD |

### 5.3 P9：TeX artifact 标准化

目标：把 PDF/BBL/AUX/LOG 作为正式转换输入与质量证据，而不是脚本侧临时文件。

建议任务：

| ID | 任务 | 验收 |
|---|---|---|
| P9-01 | 在 core/tex-facade 中定义 `BuildArtifacts` 的运行期结构 | 转换结果记录 artifact 路径、hash、生成命令和时间 |
| P9-02 | 接入 `.aux` label/citation 信息，减少自建编号推断 | label/ref 与 TeX 编译结果一致 |
| P9-03 | 将 artifact hash 纳入 regression manifest | 任一章节、bib、cls、图片变化都会触发重建 |

### 5.4 P10：多样本和多模板回归

目标：验证当前架构不是 paper3 单样本过拟合。

建议任务：

| ID | 任务 | 验收 |
|---|---|---|
| P10-01 | 增加至少 2 篇 JOS 风格样本 | 每个样本有 AST/render/verify/traceability 输出 |
| P10-02 | 增加一个非 JOS 通用 LaTeX 样本 | 明确高保真能力与降级能力边界 |
| P10-03 | 形成回归矩阵脚本 | 单命令输出所有样本质量摘要 |

### 5.5 P11：标准库与缓存演进

目标：完善 TeX/LaTeX/OOXML 标准规则知识库，为后续模板扩展和规则查询做准备。

建议任务：

| ID | 任务 | 验收 |
|---|---|---|
| P11-01 | 扩展 `standards/tex`：LaTeX2e 基础命令、graphicx、booktabs、algorithm2e、natbib | AST rule coverage 更完整 |
| P11-02 | 扩展 `standards/ooxml`：OPC、WordprocessingML、styles、relationships、field code | render mapping coverage 更完整 |
| P11-03 | 实现可选 SQLite 编译缓存原型 | 删除 cache 后可由文件规则重建，lock hash 校验一致 |

## 6. 建议的下一阶段验收门槛

下一阶段不建议只用“功能能跑”作为验收，应继续坚持质量门：

| 类别 | 建议门槛 |
|---|---|
| paper3 JOS 回归 | `scripts/paper3_regression.sh` 必须返回 0 |
| AST | diagnostics=0，RawFallback=0，核心节点均有 rule ids |
| Render | diagnostics=0，核心节点均有 mapping rule ids |
| DOCX 结构 | verify failed checks=0 |
| 字符覆盖 | DOCX/PDF 字符比例不低于当前 0.876，除非校验口径调整 |
| 参考文献 | 条目数不低于 55，正文引用为上标 |
| 图片 | media=10，图题与图片一一对应 |
| 表格/算法 | 表格结构、字体、边框、cantSplit、算法行号与注释列均通过 |

## 7. 结论

当前可以认定：基于 v2.0 设计方案拆出的任务清单已经全部开发实现，并且 paper3/JOS 转换链路已经从“生成但质量门失败”推进到“完整回归返回 0”。  

下一步重点不再是修 paper3 的 P0/P1 阻断项，而是把已经验证的能力从 paper3/JOS 单样本闭环，推进为 Profile 驱动、质量模块原生化、TeX artifact 标准化和多模板回归的生产级转换引擎。
