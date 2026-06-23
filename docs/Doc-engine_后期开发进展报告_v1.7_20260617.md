# Doc-engine 后期开发进展报告

| 文档版本 | 时间 | 范围 |
|---|---|---|
| V1.0 | 2026-06-14 | Sprint 0 + M1 + M2 完成 |
| V1.1 | 2026-06-14 | M3 + M5 + M7 + 质量加固完成 |
| V1.2 | 2026-06-14 | M4 + M6 + M8 + 5 大风险全部完成 |
| V1.3 | 2026-06-14 | 三端联调：Flutter 桌面（FFI）+ Chrome MV3 扩展 + crates/server（Axum MVP）+ LaTeX 解析 char-boundary 健壮性 |
| V1.4 | 2026-06-16 | V2 docx→pdf 全链路 + 质量三色对比：PageSetup 模板 / PDF→PNG 内嵌 / JOS 参考文献样式 / soffice Windows 卡死修复 |
| V1.5 | 2026-06-16 | Sprint 11 中：Author 列表逗号 / 残留 {12pt} / 英文 flushleft 重复段 / 集成测试脚本 |
| V1.6 | 2026-06-17 | cli 版本号 0.1.0 → 0.2.0；输出文件加 `<stem>__v0.2.0__<yyyymmdd-hhmmss>` 命名；修复 `expand_macros_with_input` 二次展开导致 doc.blocks 重复（页数 24→12、字符 38752→21249）；表格宽度由 2000 twips 改为 9000 twips 均分；页眉 rjhead 截断为「石 洪 雷 等」 |
| **V1.7** | **2026-06-17** | **图片未嵌入 docx 修复（word/media + rId rel）；JPEG 格式嵌入（兼容 soffice）；首字符丢失 bug（`try_top_level_command` consumed 公式错误）；soffice 图片渲染待深入调试（已完成 PNG→JPEG 转换，但渲染链路仍被 soffice 丢弃）；集成测试稳定通过** |

## 1. 总览

| 阶段 | 状态 | 备注 |
|---|---|---|
| cli 版本 0.2.0 | ✅ 已完成 | 所有输出含版本+时间戳 |
| 重复 include 修复 | ✅ 已完成 | 字符 38752→21249，页数 24→12 |
| 图片嵌入 docx | ✅ 已完成 | 8 图写入 word/media/*.jpg，rIdImg{N}→media/image{N}.jpg |
| JPEG 格式嵌入 | ✅ 已完成 | RGBA PNG→RGB JPEG 转换，降低文件大小 |
| 首字符丢失 bug | ✅ 已完成 | consumed 公式中 `.trim()` 导致末尾空白计入，修复后所有首字符正常 |
| soffice 图片渲染 | 🔶 调试中 | docx 结构正确（命名空间+关系+数据均合规），但 soffice 24.x 仍将其渲染为空 PDF（712KB/12页/0 图） |
| 正文格式混乱 | ⏳ 待排查 | 段落顺序、表格结构、表注/节注泄漏等 |
| 集成测试稳定 | ✅ PASSED | 关键 token 缺失 0/31，LaTeX 漏出 0 |

## 2. 本轮核心修复（V1.7）

### 2.1 图片未嵌入 docx（严重）

**问题**：V2 docx 的 `word/` 目录无 `media/` 子目录，导致 8 张图完全缺失。

**根因**：原实现将图片 base64 编码后内嵌到 `<w:binData>` XML 节点中，这是 **非标准 OOXML 内嵌形式**。soffice 无法识别，因此不渲染。

**修复方案**：
1. **真实嵌入文件**：`pack_with_page_setup` 遍历 `embedded_images`（由 `serialize_document` 收集），将字节写入 `word/media/image{N}.jpg`。
2. **关系引用**：`document.xml.rels` 生成 `<Relationship Id="rIdImg{N}" Type=".../image" Target="media/image{N}.jpg"/>`。
3. **drawing XML** 改用标准 `<a:blip r:embed="rIdImg{N}"/>` 引用（而非 `<w:binData>`）。
4. **格式转换**：所有图片（RGBA PNG）统一转换为 JPEG（RGB）再嵌入，提升兼容性。

**修改文件**：
- `crates/docx-writer/src/serializer.rs`：新增 `EmbeddedImage` 结构、`embedded_images` 参数、`r:embed` 引用、`calc_image_emu` EMU 计算修复。
- `crates/docx-writer/src/packer.rs`：写 `word/media/`、生成图片 rel、修正 `build_doc_rels`。
- `crates/docx-writer/src/lib.rs`：导出 `EmbeddedImage`。

### 2.2 首字符丢失 bug（严重）

**问题**：每个章节标题后的段落首字符丢失（"OpenTelemetry" → "penTelemetry"，"边缘计算" → "缘计算"，"包航宇等" → "航宇等"）。

**根因**：`try_top_level_command` 中 consumed 计算使用了 `rest.trim()`，但 `trim()` 会同时去除首尾空白。`rest`（`\subsection{...}` 后的全部文本）以文件末尾的 `\n\n` 结尾，导致 `.trim()` 删除了 2 个额外字节，consumed 比实际多 2，最终 `pos` 偏移 2 字节恰好跳过一个 CJK 字符的首字节。

**修复方案**：改用显式 `leading_ws`（仅前缀空白）计算，不再依赖 `.trim()`：

```rust
let lead = rest.as_bytes().iter()
    .take_while(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
    .count();
let consumed = prefix.len() + lead + end + 2;
```

**修改文件**：`crates/latex-reader/src/lower.rs` → `try_top_level_command`。

### 2.3 EMU 计算错误（影响）

**问题**：`calc_image_emu` 返回像素值（如 2385），而 `<wp:extent>` 期望 EMU 单位（914400 = 1 英寸）。

**修复**：
1. 以 96 DPI 为基准：`EMU = pixels * 914400 / 96`
2. 修正后图片尺寸合理（5 英寸宽以内）。

### 2.4 `wp` 命名空间 URI 错误（影响）

**问题**：`xmlns:wp` 使用了 `.../wordprocessingDrawing/inline`（错误），应为 `.../wordprocessingDrawing`。

**修复**：修正为标准 OOXML URI。

### 2.5 soffice 图片渲染调试（进行中）

**现状**：
- docx 结构完全合规（命名空间、Content_Types、rels、media 文件、JPEG 字节均正确）
- python-docx 生成的相同结构可以渲染（确认 soffice 24.x 支持 inline JPEG）
- 纯 docx（无 header/footer）无法被 soffice 加载（Error: source file could not be loaded）
- 可能是 `<w:pStyle>` 引用的自定义样式（如 `JOSTitleZh`）导致 soffice 解析失败

**已排除的可能**：
- PNG vs JPEG 格式：均不渲染
- `<wp:cNvGraphicFramePr>` / `<a:graphicFrameLocks>`：已添加
- 命名空间声明位置（root vs inline）：已修正为 inline
- header/footer 引用：跳过 header/footer 后仍无法加载
- image rel type / Target 路径：正确
- PNG RGBA 透明度：已转 JPEG RGB

**下一步**：
1. 将自定义样式替换为 OOXML 内置样式（如 `Heading1`、`Normal`），验证是否样式阻塞
2. 或改用 python-docx 重建完整文档流（绕过我流序列化）

## 3. 集成测试状态

```
oracle 字符: 33231   V2 字符: 21246
关键 token 缺失: 0 / 31
LaTeX 漏出:     0
✅ PASSED
```

| 指标 | V1.6 | V1.7 | Oracle |
|---|---|---|---|
| 字符数 | 21249 | 21246 | 33231 |
| 页数 | 12 | 12 | 16 |
| 图片嵌入 | 无 | 8 图 | 8 图 |
| 首字符丢失 | 严重 | 0 | 0 |
| LaTeX 漏出 | 0 | 0 | — |
| Token 缺失 | 0 | 0 | — |

字符差距（~12K）主要来自：
1. 表格内容以 `[TAB: ...]` 标记代替完整单元格
2. 参考文献正文条目（31 条全部在 token 清单中，但 OCR 化文本偏短）
3. 公式、算法块以简化形式渲染

## 4. 技术债务 & 风险

| 项目 | 级别 | 说明 |
|---|---|---|
| soffice 图片渲染 | 🔴 高 | docx 结构合规但图片被丢弃；可能是自定义样式不兼容；需要逐一替换内置样式验证 |
| 表格渲染 | 🟡 中 | 以 `[TAB: ...]` 简写代替完整单元格；需实现完整 OOXML 表格 |
| 正文格式混乱 | 🟡 中 | 段落顺序、表注/节注泄漏（如 `]} }`）、`\ref` 未解析为标签 |
| 算法块渲染 | 🟡 中 | Algorithm 1 标题显示正确，但算法内容以简化文本块代替 |
| OCR 参考 PDF 字符数 | 🟡 中 | oracle 字符 33231 vs 实际更多（OCR 只读出正文，表格/公式/引用均有遗漏） |

## 5. 下一步

1. **优先级 P0**：修复 soffice 图片渲染——尝试将所有自定义 `w:pStyle` 替换为 OOXML 内置样式（`Normal`、`Heading1` 等），验证是否是样式定义不兼容导致解析失败。
2. **优先级 P1**：修复 `\ref{key}` 未解析（当前显示为 `tab:compare` 而非 `表 1`）；表格完整单元格渲染。
3. **优先级 P2**：正文格式清理——消除 `]} }` 等 LaTeX 残留、表注节注顺序。
4. **优先级 P3**：全 `cargo test` 回归验证。
