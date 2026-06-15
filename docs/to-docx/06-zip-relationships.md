# 06 · docx 物理包结构（zip + relationships）

> 一个 `.docx` 文件就是一份 ZIP。本章复刻 ZIP 内部每一个 entry 的命名、内容、关系，Rust 重构时**严格按本表的路径与 XML 字面**写文件。

## 6.1 ZIP 内容总览

```text
[Content_Types].xml           ← 内容类型清单
_rels/.rels                    ← 顶级关系
word/document.xml              ← 主文档
word/_rels/document.xml.rels   ← 文档级关系
word/styles.xml                ← 样式表
word/settings.xml              ← 文档设置
word/header0.xml               ← 首页页眉（first）
word/header1.xml               ← 默认页眉（default / 奇数页）
word/header2.xml               ← 偶数页页眉（even）
word/footer1.xml               ← 首页页脚（first）
word/footer2.xml               ← 默认页脚
word/footer3.xml               ← 偶数页页脚
word/media/image1.png          ← 第 1 张图（如果有）
word/media/image2.png          ← 第 2 张图
...                             ← 视 figs 数量
```

**最少 9 个 entry**（无图时）：`[Content_Types].xml` / `_rels/.rels` / `word/document.xml` / `word/_rels/document.xml.rels` / `word/styles.xml` / `word/settings.xml` / `word/header0.xml` / `word/header1.xml` / `word/header2.xml` / `word/footer1.xml` / `word/footer2.xml` / `word/footer3.xml` —— 实际 12 个。

注意 **footer1/2/3 在无图时仍然存在**（footer 始终是 3 个），headers 也是 3 个。

## 6.2 `[Content_Types].xml`

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="png" ContentType="image/png"/>
  <Default Extension="jpg" ContentType="image/jpeg"/>
  <Default Extension="jpeg" ContentType="image/jpeg"/>
  <Override PartName="/word/document.xml"
            ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml"
            ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
  <Override PartName="/word/settings.xml"
            ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/>
  <Override PartName="/word/header0.xml"
            ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/>
  <Override PartName="/word/header1.xml"
            ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/>
  <Override PartName="/word/header2.xml"
            ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/>
  <Override PartName="/word/footer1.xml"
            ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"/>
  <Override PartName="/word/footer2.xml"
            ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"/>
  <Override PartName="/word/footer3.xml"
            ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"/>
</Types>
```

要点：

- `Default` 给出按文件后缀的默认类型。
- `Override` 显式声明每个 part 的 MIME 类型（Word 打开时**先**看 Override）。
- `+xml` 后缀是 OPC 标准的固定后缀（与 ECMA-376 规范一致）。
- `image/png` `image/jpeg` 是 RFC 编号的图像 MIME。

## 6.3 `_rels/.rels` —— 顶级关系

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
                Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument"
                Target="word/document.xml"/>
</Relationships>
```

顶级关系**只指明**主文档位置。其余 part 的关系都挂在 `word/_rels/document.xml.rels` 上。

## 6.4 `word/_rels/document.xml.rels`

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
                Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles"
                Target="styles.xml"/>
  <Relationship Id="rId2"
                Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings"
                Target="settings.xml"/>
  <Relationship Id="rId3"
                Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header"
                Target="header1.xml"/>
  <Relationship Id="rId4"
                Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header"
                Target="header2.xml"/>
  <Relationship Id="rId5"
                Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer"
                Target="footer1.xml"/>
  <Relationship Id="rId6"
                Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer"
                Target="footer2.xml"/>
  <Relationship Id="rId7"
                Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer"
                Target="footer3.xml"/>
  <Relationship Id="rId8"
                Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header"
                Target="header0.xml"/>
  <Relationship Id="rId20"
                Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
                Target="media/image1.png"/>
  <Relationship Id="rId21"
                Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
                Target="media/image2.png"/>
  ...
</Relationships>
```

### 6.4.1 rId 编号规则

| 范围 | 用途 | 备注 |
|------|------|------|
| rId1 | styles | 固定 |
| rId2 | settings | 固定 |
| rId3 | header1 (default) | 固定 |
| rId4 | header2 (even) | 固定 |
| rId5 | footer1 (first) | 固定 |
| rId6 | footer2 (default) | 固定 |
| rId7 | footer3 (even) | 固定 |
| rId8 | header0 (first) | 固定 |
| rId20+ | 图片 | 从 20 起递增（`self.next_rid = 20`），见 §4.5 |

> **注意**：在 `document.xml` 的 `<w:sectPr>` 中，`<w:headerReference w:type="first" r:id="rId8"/>`、`type="default" rId3`、`type="even" rId4`—— `type` 与 rId 的**对应关系**严格固定。Rust 端不要重新分配。

### 6.4.2 关系 Target 路径

- styles/settings/header/footer 都是**相对 `word/` 目录**的路径。
- 图片是 `media/image1.png`（**不带 `word/` 前缀**——关系挂载点是 `word/document.xml`，所以相对 `word/`）。

## 6.5 `word/document.xml`

由 `DocxBuilder.document_xml()` 生成。命名空间：

```xml
<w:document
  xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
  xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
  xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
  xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
  xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
  <w:body>
    ...所有 <w:p> / <w:tbl> / <w:drawing>...
    <w:sectPr>...</w:sectPr>
  </w:body>
</w:document>
```

## 6.6 `word/styles.xml`

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:styleId="Normal">
    <w:name w:val="Normal"/>
    <w:pPr><w:jc w:val="both"/><w:spacing w:line="260" w:lineRule="exact"/></w:pPr>
    <w:rPr>
      <w:rFonts w:ascii="Times New Roman" w:hAnsi="Times New Roman" w:eastAsia="宋体" w:cs="Times New Roman"/>
      <w:sz w:val="18"/><w:szCs w:val="18"/>
    </w:rPr>
  </w:style>
  ...（共 21 个 style）
</w:styles>
```

**没有 `<w:docDefaults>`**——所有样式都必须显式包含 `rFonts/sz`。这是简化做法，Word 接受但不推荐长期使用。Rust 端**应当**补一个 `<w:docDefaults>` 块：

```xml
<w:docDefaults>
  <w:rPrDefault>
    <w:rPr>
      <w:rFonts w:ascii="Times New Roman" w:hAnsi="Times New Roman" w:eastAsia="宋体" w:cs="Times New Roman"/>
      <w:sz w:val="21"/><w:szCs w:val="21"/>
      <w:lang w:val="en-US" w:eastAsia="zh-CN" w:bidi="ar-SA"/>
    </w:rPr>
  </w:rPrDefault>
  <w:pPrDefault>
    <w:pPr><w:spacing w:after="0" w:line="312" w:lineRule="auto"/></w:pPr>
  </w:pPrDefault>
</w:docDefaults>
```

**功能等价**——只是更标准。

## 6.7 `word/settings.xml`

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:evenAndOddHeaders/>
  <w:characterSpacingControl w:val="doNotCompress"/>
</w:settings>
```

`evenAndOddHeaders` 是打开文档"奇偶页不同页眉"的开关。如果禁用，header2 会被忽略（但仍要存在 part）。

## 6.8 页眉/页脚 part

详见 §5.9。每个 part 根元素分别是 `<w:hdr>` / `<w:ftr>`，都带 `xmlns:w`。

## 6.9 媒体（图片）part

每张图片的 part 路径：`word/media/imageN.<suffix>`（N 从 1 起，suffix 与源文件一致）。

二进制内容**直接复制**源文件：

```python
for src, arcname in builder.media:
    zf.write(src, arcname)
```

Rust 端：

```rust
for (src, arcname) in &builder.media {
    let bytes = std::fs::read(src)?;
    zip.write_all(arcname, &bytes)?;
}
```

### 6.9.1 图片尺寸计算

```python
with Image.open(path) as img:
    px_w, px_h = img.size
width_cm = min(text_width_cm * width_factor, text_width_cm)
height_cm = width_cm * px_h / max(px_w, 1)
cx = int(width_cm * EMU_PER_CM)
cy = int(height_cm * EMU_PER_CM)
```

要点：

- 按 width_factor 缩放到版心宽度。**width_factor 来自 LaTeX `width=0.85\textwidth`**，默认 0.9。
- `min()` 防止用户传入 width=1.5 之类把图扩出页面。
- `max(px_w, 1)` 防止 0 除。
- 单位 EMU = 360000 per cm。

> ⚠️ PIL 不可用时，**应当 fail**——别用 `Image.open` 的 stub。但本项目里在 shell 入口就强制要求 PIL 安装。

## 6.10 写盘的 Rust 伪代码

```rust
use std::fs::File;
use std::io::Write;
use zip::write::{SimpleFileOptions, ZipWriter};
use zip::CompressionMethod;

fn write_docx(builder: &DocxBuilder, out_path: &Path, ms: &Manuscript,
             profile: &DocxProfile) -> Result<(), Box<dyn Error>> {
    let file = File::create(out_path)?;
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated);

    // 1) 固定 part 顺序
    zip.start_file("[Content_Types].xml", opts)?;
    zip.write_all(content_types_xml().as_bytes())?;
    zip.start_file("_rels/.rels", opts)?;
    zip.write_all(package_rels_xml().as_bytes())?;

    // 2) 文档主体
    zip.start_file("word/document.xml", opts)?;
    zip.write_all(builder.document_xml().as_bytes())?;

    // 3) 文档关系
    zip.start_file("word/_rels/document.xml.rels", opts)?;
    zip.write_all(builder.rels_xml().as_bytes())?;

    // 4) 样式 / 设置
    zip.start_file("word/styles.xml", opts)?;
    zip.write_all(styles_xml().as_bytes())?;
    zip.start_file("word/settings.xml", opts)?;
    zip.write_all(settings_xml().as_bytes())?;

    // 5) 页眉
    let tw = builder.text_width_twips;
    zip.start_file("word/header0.xml", opts)?;
    zip.write_all(first_header_xml(tw, &profile.first_header_rows).as_bytes())?;
    let running = ms.running_header.clone().unwrap_or_else(|| derived_running_header(ms));
    zip.start_file("word/header1.xml", opts)?;
    zip.write_all(header_xml(&running, tw).as_bytes())?;
    zip.start_file("word/header2.xml", opts)?;
    zip.write_all(header_xml(&profile.even_header_text, tw).as_bytes())?;

    // 6) 页脚
    let first_footer = ms.first_footer_text.clone().unwrap_or_else(|| profile.first_footer_text.clone());
    zip.start_file("word/footer1.xml", opts)?;
    zip.write_all(footer_xml(&first_footer, profile.first_footer_indent_twips).as_bytes())?;
    zip.start_file("word/footer2.xml", opts)?;
    zip.write_all(footer_xml("", 0).as_bytes())?;
    zip.start_file("word/footer3.xml", opts)?;
    zip.write_all(footer_xml("", 0).as_bytes())?;

    // 7) 媒体
    for (src, arcname) in &builder.media {
        let bytes = std::fs::read(src)?;
        zip.start_file(arcname, opts)?;
        zip.write_all(&bytes)?;
    }

    zip.finish()?;
    Ok(())
}
```

## 6.11 写盘顺序的"非强制"约束

OOXML 规范**不**强制 part 在 ZIP 中的物理顺序。但 Word 工具链（macOS Pages / LibreOffice）有时会按 entry 顺序显示，**建议**把 `[Content_Types].xml` 放最前。

## 6.12 校验 docx 是否完整

最简校验（用 `unzip -l docx.docx`）：

```bash
$ unzip -l docs/to-docx/v64-论文稿件-jos-20260613-174000.docx | head -25
Archive:  v64-论文稿件-jos-20260613-174000.docx
  Length      Date    Time    Name
---------  ---------- -----   ----
     1770  2026-06-13 17:40   [Content_Types].xml
      336  2026-06-13 17:40   _rels/.rels
   118896  2026-06-13 17:40   word/document.xml
     2191  2026-06-13 17:40   word/_rels/document.xml.rels
    12391  2026-06-13 17:40   word/styles.xml
      219  2026-06-13 17:40   word/settings.xml
     1558  2026-06-13 17:40   word/header0.xml
      994  2026-06-13 17:40   word/header1.xml
      967  2026-06-13 17:40   word/header2.xml
     1117  2026-06-13 17:40   word/footer1.xml
      214  2026-06-13 17:40   word/footer2.xml
      214  2026-06-13 17:40   word/footer3.xml
   126571  2026-06-13 17:40   word/media/image1.png
    ...
```

Rust 端用 `zip::ZipArchive::new(File::open(path)?)` 即可枚举。

## 6.13 与现有 docx 二进制对比

> 如果 Rust 重构后的产物与 Python 产物**字节级不一致**是正常的——ZIP 时间戳、压缩块位置、XML 属性顺序都会差。但 **OOXML 语义应一致**（即 Word 打开后看到相同内容）。

要校验语义一致，可以：

1. 用 Word 打开两份 docx，对比视觉。
2. 跑 `verify_jos_docx.py`（如保留 Python）或在 Rust 里复刻它（见 §8）。
3. 用 `pandoc -t plain` 抽两份 docx 文本做 diff——**注意** `[Content_Types].xml` 等不影响文本。
