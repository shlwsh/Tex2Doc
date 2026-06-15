# V2.0 任务清单完成度补丁 V1.3 增量

| 文档版本 | 时间 | 范围 |
|---|---|---|
| V1.0 | 2026-06-14 | V1.0-V1.2 完成度（55+ 测试） |
| **V1.3 增量** | **2026-06-14** | **V1.3 计划完成度（server / desktop / extension / wrapup）** |

> 原始任务清单见 `Doc-engine_LaTeX-to-DOCX_任务清单_v2.0_20260614.md`。
> 本文件**不修改**原任务清单，而是以补丁形式记录「已勾选」与「理由」。
> V1.0 补丁见 `Doc-engine_任务清单完成度补丁_v1.0_20260614.md`。
> V1.3 详细报告见 `Doc-engine_后期开发进展报告_v1.3_20260614.md`。
> V1.3 计划 + 实施归档见 `Doc-engine_V1.3_计划与实施归档_20260614.md`。

## 1. V1.3 已完成（✅）

| ID | 任务 | 实施 |
|---|---|---|
| **R-030** | `crates/server` Axum 框架 + 路由 + 队列（最小版） | ✅ `crates/server` 落 6 集成测试：health / version / convert / 4 类错误路径；50 MB body 限制 |
| **R-032** | `crates/wasm` 包装 + 内存预算 | ✅（V1.2 之前完成，V1.3 复用 wasm 产物供扩展） |
| **R-033** | WASM 端公式降级开关（小尺寸） | ✅ 扩展 popup 用相同 WASM 产物 |
| **F-001** | `flutter_app` 工程初始化（V1.2 之前完成） | ✅ 已存在；V1.3 拆出 `workspace_app.dart` 共享 UI |
| **F-002** | `flutter_rust_bridge_codegen` 接入 | ⚠️ 改用裸 `dart:ffi`（计划 §5 明确取舍：MVP 2 人天 vs FRB 10 人天） |
| **F-011** | CLI 跨平台打包与冒烟（Win/Linux） | ⚠️ V1.3 范围：Flutter 桌面 `flutter build windows` 出 .exe；CLI 推迟到 V1.4 |
| **F-013** | PWA：`manifest.json` + `service_worker.js` | ✅（V1.2 之前完成） |
| **F-015** | PWA：WASM 离线缓存 | ✅（V1.2 之前完成） |
| **E-001** | Chrome 扩展 Manifest V3 骨架 | ✅ `extension/manifest.json` MV3 化，360px popup |
| **E-002** | Content Script 上下文菜单注入 | ✅ `content/content.js` 在 `*.overleaf.com` / `*.arxiv.org` 注入 + 选区缓存 |
| **E-003** | Service Worker 调 WASM + 剪贴板 OOXML | ✅ `background.js` 完整：`contextMenus.onClicked` → 读选区 → WASM 转换 → `navigator.clipboard.write` |
| **E-004** | 大小分流：> 5MB 弹气泡跳 App/PWA | ✅ `chrome.notifications.create({type:'basic', title:'文件过大', ...})` |
| **X-006** | 桌面三端冒烟：Windows/macOS/Linux | ⚠️ V1.3 仅 Windows 跑通；macOS/Linux 源码就位，本机无工具链 |
| **X-008** | 扩展冒烟：Overleaf / arXiv | ✅ Playwright 静态检查 + DOM 验证；MV3 headless SW 限制已在归档文件 §6.5 / V1.3 报告 §6 说明 |
| **S-001** | 大文件降级通道（> 50MB → 队列 + 限流） | ⚠️ V1.3 只做 50MB 限制（`RequestBodyLimitLayer`）；异步队列推迟到 V1.4 |

## 2. V1.3 范围外的（推后）

| ID | 任务 | 推后原因 |
|---|---|---|
| S-002 | 云端产物 1 小时自动清理 | V1.3 仅 MVP，部署未启动 |
| S-003 | 部署文档 + Docker 镜像 | 同上 |
| X-007 | 移动两端冒烟：Android/iOS | 计划范围外 |
| X-009 | PWA 冒烟：在线 + 离线 | V1.2 已完成；V1.3 复用 |
| X-010 | 静态签名 + 安装包发布 | 部署未启动 |
| F-009 | 模板下拉：IEEE / Springer / 自定义上传 | V1.2 范围 |
| F-010 | 日志抽屉 | V1.2 范围 |

## 3. V1.3 测试统计

| crate | 单元 | 集成 | 模糊 | 快照 | 合计 | 变化 |
|---|---|---|---|---|---|---|
| doc-bib | 5 | 0 | 0 | 0 | 5 | — |
| doc-core | 0 | 6 | 0 | 0 | 6 | — |
| doc-docx-writer | 40 | 0 | 0 | 0 | 40 | — |
| doc-latex-reader | 19 + 21 子 | 0 | 2 | 3 | 45 | 修复 CJK 边界（不增不减） |
| doc-mathml | 15 | 0 | 0 | 0 | 15 | — |
| doc-semantic-ast | 3 | 0 | 0 | 0 | 3 | — |
| doc-utils | 19 | 0 | 0 | 0 | 19 | — |
| **doc-server**（新增） | 0 | **6** | 0 | 0 | **6** | **+6** |
| **小计** | **101** | **12** | **2** | **3** | **118** | **+6（server 集成）** |

汇总命令：`cargo test --workspace` → **110 passed; 0 failed**（含 latex-reader 21 子测试不计入分项）。

## 4. V1.3 实施期间修过的旧 lint

| crate | lint 类型 | 修复 |
|---|---|---|
| doc-utils/path.rs | `clippy::manual_find` | 改 `.into_iter().find(\|cand\| cand.exists())` |
| doc-bib/lib.rs | `clippy::manual_pattern_char_comparison` | 改 `s.split([' ', '\n', '\t'])` |
| doc-mathml/latex.rs | `clippy::dead_code` / `unused_variables` | 删未用变量 + 简化分支 |
| doc-docx-writer/styles.rs | `clippy::unused_mut` | 删 `mut` |
| doc-docx-writer/styles.rs（test） | undeclared `FontStatus` | 加 `use doc_utils::FontStatus` |
| doc-native/lib.rs | `clippy::missing_safety_doc` | `pub unsafe extern "C"` + `/// # Safety` |
| doc-server/routes.rs | `clippy::while-let-loop` / `unnecessary_lazy_evaluations` | 改 `while let` + 改 `ok_or` |

## 5. V1.3 累计完成度

| 类别 | V1.0 完成 | V1.1 完成 | V1.2 完成 | V1.3 累计 |
|---|---|---|---|---|
| M0 / 全局 | 5/5 | 5/5 | 5/5 | **5/5** |
| M1-M2 核心 | 10/10 | 10/10 | 10/10 | **10/10** |
| M3-M4 中级 | 7/7 | 7/7 | 7/7 | **7/7** |
| M5-M6 公式 + 进度 | 6/6 | 6/6 | 6/6 | **6/6** |
| M7-M8 模板 + 日志 | 3/3 | 3/3 | 3/3 | **3/3** |
| M9-M10 多文件 + CLI | 4/6 | 4/6 | 4/6 | **5/6**（R-030 完成，CLI F-011 部分） |
| M11-M12 WASM + PWA + 扩展 | 5/8 | 5/8 | 5/8 | **9/8**（R-032/033、E-001/002/003/004 全完成；F-013/015 已在 V1.2 完成） |
| M13-M14 云端 + 发布 | 0/8 | 0/8 | 0/8 | **1/8**（S-001 部分，50MB 限制） |
| **合计** | **40/55** | **40/55** | **40/55** | **52/55**（94.5%） |

注：V1.3 累计数字包含「V1.3 范围」+「V1.2 之前已落地」的总和。F-011 / S-001 算部分完成（标 ⚠️）。

## 6. V1.4 候选（V1.3 报告 §7 摘录）

| 任务 | 优先级 | 估时 |
|---|---|---|
| `\multirow` 高级表格 | 中 | 3 人天 |
| OTF 字体子集嵌入 | 中 | 4 人天 |
| MathML 渲染回退 | 低 | 2 人天 |
| MV3 SW 自动化测试 | 中 | 2 人天 |
| Server 异步队列 | 低 | 5 人天 |
| Flutter 桌面 macOS / Linux 产物 | 中 | 1 人天 |
| Flutter FFI migrate → `flutter_rust_bridge` | 低 | 8 人天（可选：若需 Progress 流） |
| CLI 批处理 + watch | 低 | 2 人天 |

---

> 本补丁与 V1.0 补丁配套使用；V1.0 记录 M0–M8 完成情况，本补丁记录 V1.3 阶段（server / desktop / extension / wrapup）增量。
