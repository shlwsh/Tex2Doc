# 关键技术 4：语义 AST → OOXML 序列化

> 本节深入解析 `doc-docx-writer`。解决的核心问题：把 `doc-semantic-ast::Document` 序列化为 ECMA-376 兼容的 docx。

---

## 1. 模块组成

| 文件 | 职责 |
|------|------|
| `model.rs` | 扁平 OOXML 结构体（`Paragraph` / `Run`）—— 极简中间表示 |
| `packer.rs` | 写 docx 包（`[Content_Types].xml` / `_rels` / `document.xml` / `styles.xml`） |
| `serializer.rs` | `Document` → `word/document.xml` 字节流 |
| `styles.rs` | 默认 `word/styles.xml` + 字体探测应用 |
| `template.rs` | `reference.docx` 解析与样式合并（M7 简化） |

---

## 2. OOXML 命名空间

`serializer.rs` 写入的根元素：

```rust
let mut root = BytesStart::new("w:document");
root.push_attribute(("xmlns:w", "http://schemas.openxmlformats.org/wordprocessingml/2006/main"));
root.push_attribute(("xmlns:r", "http://schemas.openxmlformats.org/officeDocument/2006/relationships"));
root.push_attribute(("xmlns:m", "http://schemas.openxmlformats.org/officeDocument/2006/math"));
root.push_attribute(("xmlns:wp", "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing/inline"));
root.push_attribute(("xmlns:a", "http://schemas.openxmlformats.org/drawingml/2006/main"));
root.push_attribute(("xmlns:pic", "http://schemas.openxmlformats.org/drawingml/2006/picture"));
```

* `w:` = wordprocessingml（主）
* `r:` = relationships
* `m:` = math（OMML）
* `wp:` / `a:` / `pic:` = DrawingML（图片）

---

## 3. `Block::*` → OOXML 映射

### 3.1 总览

| `Block::*` | 序列化产物 |
|------------|------------|
| `Heading { level, text, number }` | `<w:p>` + `w:pStyle="HeadingN"` + 编号 + 文本 |
| `Paragraph { runs }` | `<w:p>` + `w:pStyle="BodyText"` + 多 `<w:r>` |
| `List { is_ordered, items }` | 多段（每项一个 `<w:p>` + 编号/项目符号） |
| `Table { rows, caption, number }` | `<w:tbl>` + 边框 + `<w:tr>` + `<w:tc>` |
| `Figure { path, caption, number }` | `<w:drawing>` + `<wp:inline>` + base64 `<w:binData>` |
| `Equation { latex, is_block }` | `<w:p>` + 嵌入 `<m:oMath>` |
| `Bibliography { entries }` | "参考文献" 标题 + `[key] title (year)` 列表 |
| `RawFallback { text }` | 原文段落 |

### 3.2 Heading

```rust
Block::Heading { level, text, number, .. } => {
    let style = match level {
        1 => STYLE_HEADING1,    // "Heading1"
        2 => STYLE_HEADING2,    // "Heading2"
        3 => STYLE_HEADING3,    // "Heading3"
        _ => STYLE_TITLE,       // level 4+ → Title
    };
    let display_text = match number {
        Some(n) => format!("{} {}", n, text),     // "1.1.1 Intro"
        None => text.clone(),
    };
    let para = Paragraph {
        style_id: Some(style.to_string()),
        runs: vec![Run {
            text: display_text,
            style_id: Some(style.to_string()),
            bold: true,
            italic: false,
        }],
    };
    write_paragraph(&mut w, &para);
}
```

### 3.3 Paragraph

```rust
Block::Paragraph { runs, .. } => {
    let para = Paragraph {
        style_id: Some(STYLE_BODY.to_string()),  // "BodyText"
        runs: runs.iter().map(|r| Run {
            text: r.text.clone(),
            style_id: None,
            bold: matches!(r.style, TextStyle::Bold | TextStyle::BoldItalic),
            italic: matches!(r.style, TextStyle::Italic | TextStyle::BoldItalic | TextStyle::MathInline),
        }).collect(),
    };
    write_paragraph(&mut w, &para);
}
```

### 3.4 List

```rust
Block::List { is_ordered, items, .. } => {
    let style = if *is_ordered { STYLE_LIST_NUMBER } else { STYLE_LIST_BULLET };
    for (idx, sub) in items.iter().enumerate() {
        let label = if *is_ordered { format!("{}.", idx + 1) } else { "•".to_string() };
        let para = Paragraph {
            style_id: Some(style.to_string()),
            runs: vec![Run {
                text: format!("{} {}", label, summarize(sub)),  // V1 简化：每项压平为一段
                style_id: None, bold: false, italic: false,
            }],
        };
        write_paragraph(&mut w, &para);
    }
}
```

* V1 简化：嵌套子项用 `summarize()` 压成单段。
* V2 计划：用 `numPr` 真正的 Word 列表。

### 3.5 Table

```rust
Block::Table { rows, caption, number, .. } => {
    write_table(&mut w, rows, caption.as_deref(), number.as_deref());
}
```

* `write_table`（约 100 行）：构造 `<w:tbl>` + `<w:tblPr>` + `<w:tblW>` + `<w:tblBorders>` + 多个 `<w:tr>` + `<w:tc>`。
* 支持：
  * `colspan`（通过 `<w:gridSpan w:val="N">`）
  * `bg_color`（通过 `<w:shd w:fill="...">`）
  * `runs`（单元格内文本）
* 边框：默认 `single` 1/4 pt。

### 3.6 Figure

```rust
Block::Figure { path, caption, number, .. } => {
    fig_counter += 1;
    let fig_id = fig_counter;
    let fig_key = path.trim();

    if let Some(assets) = image_assets {
        if !fig_key.is_empty() {
            if let Some(bytes) = assets.get(fig_key) {
                let ext = if bytes[0..4] == [0x89, b'P', b'N', b'G'] { "png" } else { "jpg" };
                let media_name = format!("image{}.{}", fig_id, ext);
                let (cx, cy) = calc_image_emu(bytes, 4572000, 3429000);
                let b64 = STANDARD.encode(bytes);
                let drawing = format!(
                    r#"<w:drawing><wp:inline dist="0"><wp:extent cx="{}" cy="{}"/><wp:docPr id="{}" name="Picture {}" descr="{}"/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:nvPicPr><pic:cNvPr id="{}" name="{}"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip><w:binData w:name="word/media/{}">{}</w:blip></a:blip><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{}" cy="{}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing>"#,
                    cx, cy, fig_id, fig_id, xml_escape(fig_key),
                    fig_id, xml_escape(&media_name),
                    xml_escape(&format!("word/media/{}", media_name)),
                    b64, cx, cy
                );
                // 写入
                // caption 单独写一行
                continue;
            }
        }
    }

    // 回退占位文本
    let runs = vec![Run {
        text: format!("[图片：{}]", if fig_key.is_empty() { "（未提供）" } else { fig_key }),
        style_id: None, bold: false, italic: true,
    }];
    // ...
}
```

* **关键技巧**：图片用 `<w:binData>` 内联 base64（不需要单独的关系部件 + content type）。
* **尺寸计算**：`calc_image_emu` 用 `image::load_from_memory` 读宽高，最大宽度 5 英寸 = 4,572,000 EMU。
* **fallback**：图片未在 assets 中 → 输出占位 `[图片：path]`。
* **caption**：单独段落 + `STYLE_CAPTION` 样式 + 编号前缀。

### 3.7 Equation

```rust
Block::Equation { latex, is_block, .. } => {
    write_equation(&mut w, latex, *is_block);
}
```

```rust
fn write_equation(w: &mut Writer<Vec<u8>>, latex: &str, is_block: bool) {
    w.write_event(Event::Start(BytesStart::new("w:p"))).unwrap();
    if is_block {
        let ppr = BytesStart::new("w:pPr");
        let mut jc = BytesStart::new("w:jc");
        jc.push_attribute(("w:val", "center"));
        w.write_event(Event::Start(ppr.clone())).unwrap();
        w.write_event(Event::Empty(jc)).unwrap();
        w.write_event(Event::End(BytesEnd::new("w:pPr"))).unwrap();
    }
    let expr = parse_latex_math(latex);
    let omml = to_omml(&expr);
    let omml_str = String::from_utf8_lossy(&omml);
    if let Some(start) = omml_str.find("<m:oMath") {
        if let Some(end) = omml_str[start..].find("</m:oMath>") {
            let inner = &omml_str[start..start + end + "</m:oMath>".len()];
            use std::io::Write;
            let _ = w.get_mut().write_all(inner.as_bytes());
        }
    }
    w.write_event(Event::End(BytesEnd::new("w:p"))).unwrap();
}
```

* 调 `doc_mathml::parse_latex_math` + `to_omml`，剥出 `<m:oMath>` 段嵌入。
* 块级 equation 居中（`<w:jc w:val="center">`）。
* 行内 equation 不加 `pPr`，与正文共段落。

### 3.8 Bibliography

```rust
Block::Bibliography { entries } => {
    // 标题
    let para = Paragraph { style_id: Some(STYLE_HEADING2.to_string()), runs: vec![Run { text: "参考文献".into(), ... }] };
    write_paragraph(&mut w, &para);
    // 条目
    for e in entries {
        let line = format!("[{}] {} ({})", e.key, e.title, e.year);
        // ...
    }
}
```

* V1 简化：仅 `[key] title (year)`。
* V2 计划：根据 `BibStyle::Numeric` / `AuthorYear` 切换格式。

### 3.9 RawFallback

```rust
Block::RawFallback { text, .. } => {
    let para = Paragraph {
        style_id: Some(STYLE_BODY.to_string()),
        runs: vec![Run { text: text.clone(), style_id: None, bold: false, italic: false }],
    };
    write_paragraph(&mut w, &para);
}
```

---

## 4. 默认 styles.xml

`styles.rs::write_styles` 生成 9 个样式：

| styleId | w:name | 字体 | 字号 | 粗体 |
|---------|--------|------|------|------|
| `Title` | `Title` | Calibri | 32 | true |
| `Heading1` | `heading 1` | Calibri | 28 | true |
| `Heading2` | `heading 2` | Calibri | 24 | true |
| `Heading3` | `heading 3` | Calibri | 22 | true |
| `BodyText` | `Normal` | Calibri | 22 | false |
| `ListBullet` | `List Bullet` | Calibri | 22 | false |
| `ListNumber` | `List Number` | Calibri | 22 | false |
| `Caption` | `Caption` | Calibri | 20 | false |
| `TableHeader` | `TableHeader` | Calibri | 22 | true |

* 字号单位：半磅（half-point），22 = 11pt。
* 全部 Calibri（可被 `apply_font_probes` 替换为 CTeX 字体）。

### 字体探测应用（`apply_font_probes`）

```rust
pub fn apply_font_probes(styles_xml: &mut Vec<u8>, probes: &[FontProbe]) {
    if probes.is_empty() { return; }
    let xml_str = String::from_utf8_lossy(styles_xml).to_string();
    let mut modified = xml_str;
    for probe in probes {
        if probe.needs_fallback() {
            for attr in &["w:ascii", "w:hAnsi", "w:eastAsia", "w:cs"] {
                let pattern = format!("{}=\"{}\"", attr, probe.name);
                let replacement = format!("{}=\"{}\"", attr, probe.recommended);
                if modified.contains(&pattern) {
                    modified = modified.replace(&pattern, &replacement);
                }
            }
        }
    }
    *styles_xml = modified.into_bytes();
}
```

* 仅对 `FontStatus::Fallback` 的字体做替换。
* 替换所有四个 `rFonts` 子属性。

---

## 5. ZIP 打包（`packer.rs`）

### 5.1 入口

```rust
pub fn pack(doc: &Document) -> Result<Vec<u8>, DocxWriteError> {
    pack_with_assets(doc, None, None)
}

pub fn pack_with_template(doc, template_bytes) -> Result<Vec<u8>, DocxWriteError> {
    pack_with_assets(doc, template_bytes, None)
}

pub fn pack_with_assets(doc, template_bytes, image_assets) -> Result<Vec<u8>, DocxWriteError> {
    let document_xml = serialize_document(doc, image_assets);
    let mut styles_xml = write_styles();
    let template_styles: Option<TemplateStyles> = template_bytes.and_then(|b| parse_template(b).ok());
    if let Some(ts) = &template_styles { merge_styles(&mut styles_xml, ts); }
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut zip = zip::ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    write_zip(&mut zip, "[Content_Types].xml", CONTENT_TYPES, opts)?;
    write_zip(&mut zip, "_rels/.rels", ROOT_RELS, opts)?;
    write_zip(&mut zip, "word/_rels/document.xml.rels", DOC_RELS, opts)?;
    write_zip(&mut zip, "word/document.xml", &document_xml, opts)?;
    write_zip(&mut zip, "word/styles.xml", &styles_xml, opts)?;
    let cursor = zip.finish().map_err(|e| DocxWriteError(e.to_string()))?;
    Ok(cursor.into_inner())
}
```

### 5.2 固定常量

* `CONTENT_TYPES`：注册 `.rels` / `.xml` / `.png` / `.jpg` / `.jpeg` + `document.xml` / `styles.xml` Override。
* `ROOT_RELS`：根 `.rels` 指向 `word/document.xml`。
* `DOC_RELS`：`word/_rels/document.xml.rels` 指向 `styles.xml`。
* **不**为 `word/media/*` 注册 Override（因为图片用 `<w:binData>` 内联）。

### 5.3 压缩

* `CompressionMethod::Deflated`（deflate 算法）。
* 平衡大小与速度。

### 5.4 输出大小

* 最小 docx（仅 Title）：~1.9 KB（满足 4 KiB 阈值的关键）。
* paper3 完整转换：~38 KB。

---

## 6. 模板继承（`template.rs`）

### 6.1 解析（`parse_template`）

```rust
pub fn parse_template(docx_bytes: &[u8]) -> Result<TemplateStyles, TemplateError> {
    let cursor = std::io::Cursor::new(docx_bytes);
    let mut zip = zip::ZipArchive::new(cursor).map_err(TemplateError::Zip)?;
    let mut entry = zip.by_name("word/styles.xml").map_err(|_| TemplateError::MissingStyles)?;
    let mut buf = String::new();
    entry.read_to_string(&mut buf).map_err(TemplateError::Io)?;
    Ok(parse_styles_xml(&buf))
}
```

* 解压 docx → 读 `word/styles.xml` → 调 `parse_styles_xml`。

### 6.2 styles.xml 解析（`parse_styles_xml`）

```rust
pub fn parse_styles_xml(xml: &str) -> TemplateStyles {
    let mut out = TemplateStyles::default();
    let bytes = xml.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let pos1 = find_substring(bytes, b"<w:style ", i);
        let pos2 = find_substring(bytes, b"<w:style>", i);
        let rel = /* min(pos1, pos2) */;
        let (block, end) = if let Some(close_rel) = find_substring(bytes, b"/>", rel) {
            // 自闭合
            let block = &xml[rel..close_rel + 2];
            (block.to_string(), close_rel + 2)
        } else if let Some(end_rel) = find_substring(bytes, b"</w:style>", rel) {
            // 成对
            let block = &xml[rel..end_rel + "</w:style>".len()];
            (block.to_string(), end_rel + "</w:style>".len())
        } else { break; };
        let id = extract_attr(&block, "w:styleId").unwrap_or_default();
        let name = extract_w_name(&block);
        if !id.is_empty() {
            out.by_id.insert(id.clone(), block);
            if let Some(n) = name { out.name_to_id.insert(n, id); }
        }
        i = end;
    }
    out
}
```

* 朴素扫描 `<w:style ...>...</w:style>` 块。
* 提取 `w:styleId` + `<w:name w:val="...">`。
* 保留完整 XML 字符串（**不**重新构造 rPr / pPr）。

### 6.3 合并（`merge_styles`）

```rust
pub fn merge_styles(target_xml: &mut Vec<u8>, template: &TemplateStyles) {
    if template.by_id.is_empty() { return; }
    let target_str = String::from_utf8_lossy(target_xml).to_string();
    let mut existing: Vec<String> = Vec::new();
    // 扫描 target 中已有的 styleId
    // ...
    let mut append = String::new();
    for (id, block) in &template.by_id {
        if !existing.iter().any(|e| e == id) {
            append.push_str(block);
        }
    }
    if append.is_empty() { return; }
    let closing = "</w:styles>";
    let closing_pos = target_str.rfind(closing).unwrap_or(target_str.len());
    let mut new_xml = String::with_capacity(target_str.len() + append.len() + closing.len() + 1);
    new_xml.push_str(&target_str[..closing_pos]);
    new_xml.push_str(&append);
    new_xml.push_str(&target_str[closing_pos..]);
    *target_xml = new_xml.into_bytes();
}
```

* **同名覆盖**：用户样式 ID 已存在 → 保留用户版本（不追加模板版本）。
* **缺失补全**：模板中有、用户没有 → 追加到 `</w:styles>` 之前。
* **不重建 XML**：保留模板原始 rPr / pPr。

### 6.4 继承策略

| 场景 | 行为 |
|------|------|
| 模板 `Heading1` + 用户 `Heading1` | 用户版本胜出 |
| 模板 `Heading1` + 用户无 | 自动补全 |
| 模板无 + 用户有 | 用户保留 |
| 模板无 + 用户无 | 都不在 |

---

## 7. 错误处理

```rust
#[derive(Debug, thiserror::Error)]
#[error("docx write error: {0}")]
pub struct DocxWriteError(pub String);
```

* 包装为字符串（与 `doc-core::CoreError::Serialize` 兼容）。
* `zip::write` 错误 + XML 序列化错误统一捕获。

---

## 8. 测试

| 文件 | 覆盖 |
|------|------|
| `src/packer.rs::tests` | `pack_minimal` —— `PK\x03\x04` 头 + 长度 > 100 |
| `src/template.rs::tests` | `parse_styles_xml_basic`、`merge_adds_missing`、`round_trip_via_zip` |
| `src/styles.rs::tests` | `apply_font_probes_no_change_when_empty`、`apply_font_probes_fallback_replaces` |

---

## 9. 已知限制

| 当前限制 | 影响 | V2 方向 |
|----------|------|---------|
| 列表用「编号 + 文本」压扁，V1 不生成 numPr | Word 列表样式不严格 | V2 用 `numPr` |
| 嵌套 tabular 扁平化 | 视觉损失 | V2 真正嵌套表 |
| 表格没有列宽 | 列等宽 | V2 解析 `\|` 列分隔 |
| 引用按 `[n]` 编号 | 失真 | V2 用 `fldChar` 域 |
| 没有 numbering.xml / header / footer | 简版 | M7 简化版，未来扩展 |
| `caption` 仅 figure 路径；其它 caption 缺失 | — | V2 加 |

---

## 10. 进一步阅读

* [05-math-pipeline.md](./05-math-pipeline.md) — 公式独立管道
* [06-vfs-and-fonts.md](./06-vfs-and-fonts.md) — VFS / 字体
* [04-architecture/02-layered-architecture.md](../../04-architecture/02-layered-architecture.md) — 分层
