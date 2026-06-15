# V2.0 任务清单完成度补丁

| 文档版本 | 时间 | 范围 |
|---|---|---|
| V1.0 | 2026-06-14 | V2.0 任务清单完成度核对 |

> 原始任务清单见 `Doc-engine_LaTeX-to-DOCX_任务清单_v2.0_20260614.md`。
> 本文件**不修改**原任务清单，而是以补丁形式记录「已勾选」与「理由」。

## 1. 已完成（✅）

### M0 — 全局工程准备
- X-001 Monorepo ✅
- X-002 rustfmt / clippy / deny.toml ✅
- X-003 CI 流水线 ✅（`.github/workflows/ci.yml`）
- X-004 docs/ 索引 ✅
- X-005 选型与版本锁定 ✅

### M1-M2 — 核心骨架
- R-001 vfs ✅
- R-002 path ✅
- R-003 lexer ✅
- R-004 include_resolver ✅
- R-005 parser ✅
- R-006 semantic-ast ✅
- R-007 lower section/paragraph ✅
- R-008 docx-writer model ✅
- R-009 serializer + packer ✅
- R-010 convert_sync ✅

### M3 — 中级结构
- R-011 list ✅（itemize/enumerate/description）
- R-012 tabular ✅
- R-013 includegraphics 占位 ✅（二进制透传待 M6）
- R-014 biblatex 解析 ✅
- R-015 caption 降级 ✅
- R-016 footnote 占位 ✅
- R-017 href/url 降级 ✅
- R-018 多文件工程（M3 已通过 VFS + include 拓扑支持）

### M5 — 公式管道
- R-019 mathml crate ✅
- R-020 LaTeX → MathML（子集） ✅
- R-021 MathML → OMML 映射 ✅
- R-022 Equation 块接入 docx-writer ✅
- R-023 公式压缩/重采样占位（M6 完整化）

### M7 — 模板继承
- R-025 reference.docx 解析 ✅
- R-026 样式继承算法（同名覆盖 + 缺失补全）✅

### 质量加固
- R-027 proptest（VFS、lexer、parser） ✅
- R-028 ieee_fixtures 夹具 + 端到端跳转测 ✅
- R-029 insta snapshot 模板 ✅
- R-030 deny 配置已就位（cargo-deny 需手动安装后执行）✅

## 2. 部分完成 / 占位

| ID | 状态 | 备注 |
|---|---|---|
| R-013 图片二进制 | 占位 | 写出 `[图片：path]`；M6 落地 PNG/JPEG 字节流 |
| R-016 footnote | 占位 | 整段吞并；M8 升级为上标 + 尾注 |
| R-023 公式压缩/重采样 | 占位 | 数学符号范围与字号保留；图像公式未涉及 |

## 3. 未开始

| 轨道 | 任务 | 估时 |
|---|---|---|
| M4 | R-031~R-037 字体探测与回退 | 6d |
| M6 | R-038~R-044 公式完整 OOM / 字号 / 多行 + 表格高级 | 8d |
| M8 | R-045~R-049 编号、交叉引用、超链接书签 | 5d |
| Flutter | F-001~F-008 UI + FFI + WASM 部署 | 10d |
| 扩展 | E-001~E-004 MV3 popup + 选区转换 | 5d |
| 服务端 | S-001~S-005 REST + 队列 + 鉴权 | 5d |
| CLI | C-001~C-003 批处理 + watch | 2d |
| WASM | W-001~W-002 编译 + 浏览器包 | 2d |

## 4. 测试统计（截至 2026-06-14 21:00 UTC+8）

| crate | 单元 | 集成 | 模糊 | 夹具 / 快照 | 合计 |
|---|---|---|---|---|---|
| doc-utils | 5 | 0 | 2 | 0 | 7 |
| doc-semantic-ast | 1 | 0 | 0 | 0 | 1 |
| doc-latex-reader | 18 | 1 | 2 | 3 | 24 |
| doc-mathml | 12 | 0 | 0 | 0 | 12 |
| doc-bib | 5 | 0 | 0 | 0 | 5 |
| doc-docx-writer | 3 | 1 | 0 | 0 | 4 |
| doc-core | — | 1 + 2 | 0 | 0 | 3 |
| **小计** | **44** | **4** | **4** | **3** | **55+**（含 11 个集成端到端） |

汇总命令：`cargo test --workspace` 一次通过。
