# Semantic TeX Engine P8 Word-open 回归验证进展报告

**时间戳**：20260622-004126  
**基准计划**：`docs-zh/plan-0621.md`  
**关联阶段**：P8 真实样本回归与质量指标  
**本轮目标**：将 nightly regression 从“DOCX ZIP 头检查”推进到“可选 LibreOffice/Word 实际打开验证”，并修复发现的 DOCX XML 兼容性问题。

---

## 一、当前结论

本轮完成了 P8 nightly regression 的质量门禁增强：

- `scripts/nightly_regression.sh` 新增可选 LibreOffice headless 打开验证。
- 新增 LibreOffice verifier preflight，区分“验证器不可用”和“DOCX 打不开”。
- 新增 `NIGHTLY_WORD_OPEN_REQUIRED=true` 强门禁模式。
- nightly summary 新增 `docx_structure_valid`、`docx_xml_valid`、`word_open_check_available`、`word_openable`、`word_open_skipped` 等指标。
- 修复 `doc-docx-writer` 输出 `word/document.xml` 时根节点重复 namespace 的问题。
- 新增 `document_root_namespaces_are_unique` 单元测试。

当前结论：

```text
P8 已具备 Word-open 验证入口和强门禁模式。
当前执行环境中的 LibreOffice verifier 自检不可用，因此本机回归会将 Word-open 记为 skipped。
DOCX 根 namespace 重复问题已修复，避免该类非法 XML 再次进入回归样本。
```

---

## 二、脚本能力变化

### 2.1 新增环境变量

`scripts/nightly_regression.sh` 新增：

```text
NIGHTLY_WORD_OPEN_CHECK=true
```

启用 LibreOffice headless 验证。

```text
NIGHTLY_WORD_OPEN_REQUIRED=true
```

当 LibreOffice verifier 不可用时，将整轮回归判为失败。

```text
NIGHTLY_WORD_OPEN_TIMEOUT_SECONDS=60
```

控制单个 LibreOffice 转换验证的超时时间。

### 2.2 新增 preflight

脚本会先生成一个临时 `input.txt`，用 LibreOffice headless 转 PDF 自检：

```text
word-open-selftest.log
```

如果自检失败：

- `word_open_check_available=false`
- 每个 fixture 的 `word_open_skipped=true`
- 默认不把 DOCX 判为失败，仍使用 ZIP 结构检查作为有效门禁
- 若设置 `NIGHTLY_WORD_OPEN_REQUIRED=true`，脚本最终退出 1

这样可以避免在没有可用 LibreOffice 的 CI 或沙箱环境中，把“验证器不可用”误判成“DOCX 不可打开”。

### 2.3 summary 字段变化

`conversion_stats.json` 从 `p8-nightly-v1` 升级为 `p8-nightly-v3`：

```json
{
  "version": "p8-nightly-v3",
  "word_open_check": true,
  "word_open_check_required": false,
  "word_open_check_available": false,
  "word_open_timeout_seconds": 60,
  "docx_openable": 4,
  "docx_zip_openable": 4,
  "word_openable": 0,
  "word_open_skipped": 4
}
```

每条 fixture 结果新增：

```json
{
  "docx_zip_openable": true,
  "docx_structure_valid": true,
  "docx_xml_valid": true,
  "docx_xml_skipped": false,
  "word_open_check": true,
  "word_openable": false,
  "word_open_skipped": true,
  "docx_structure_log_path": "...",
  "docx_xml_log_path": "...",
  "word_log_path": "..."
}
```

### 2.4 DOCX 结构与 XML 校验

脚本现在会对每个生成的 DOCX 做包结构检查：

```text
[Content_Types].xml
_rels/.rels
word/_rels/document.xml.rels
word/styles.xml
word/document.xml
```

缺少任意必需 part 时，`docx_structure_valid=false`。

当 `xmllint` 可用时，脚本会对上述 XML part 执行 well-formed 校验：

```text
docx_xml_valid=true|false
docx_xml_skipped=false
```

当 `xmllint` 不可用时：

```text
docx_xml_valid=false
docx_xml_skipped=true
```

这让 nightly 在 LibreOffice 不可用时仍能提供比 ZIP header 更强的 DOCX 质量证据。

---

## 三、DOCX Writer 修复

### 3.1 问题发现

启用 Word-open 验证后，`generic` profile 的 4 个 fixture 最初表现为：

```text
docx_zip_openable = 4
word_openable = 0
```

检查 `word/document.xml` 后发现根节点重复声明：

```xml
xmlns:wp="..."
xmlns:wp="..."
xmlns:a="..."
xmlns:a="..."
```

这是非法 XML，会导致严格 OOXML 解析器或办公软件打开失败。

### 3.2 修复内容

修改文件：

```text
crates/docx-writer/src/serializer.rs
```

修复点：

- 删除重复的 `xmlns:wp`。
- 删除重复的 `xmlns:a`。
- 保留唯一的 `xmlns:w`、`xmlns:r`、`xmlns:m`、`xmlns:a`、`xmlns:wp`、`xmlns:pic`。

新增测试：

```text
document_root_namespaces_are_unique
```

测试覆盖：

```text
xmlns:w
xmlns:r
xmlns:m
xmlns:a
xmlns:wp
xmlns:pic
```

每个根 namespace 均只能出现一次。

---

## 四、验证结果

### 4.1 单元测试

已执行：

```bash
cargo fmt -p doc-docx-writer
cargo test -p doc-docx-writer document_root_namespaces_are_unique -- --nocapture
cargo test -p doc-docx-writer --lib -- --nocapture
```

结果：

```text
PASS
```

说明：

- `doc-docx-writer` lib 测试 37 项通过。
- `cargo fmt` 仍输出项目历史 rustfmt nightly-only 配置 warning，不影响格式化结果。
- `doc-docx-writer` 仍有历史 warning，包括 unused/dead_code/non_snake_case，非本轮新增。

### 4.2 generic 小范围回归

已执行：

```bash
NIGHTLY_PROFILES=generic \
NIGHTLY_WORD_OPEN_CHECK=true \
ALLOW_FAILURES=true \
scripts/nightly_regression.sh
```

输出目录：

```text
examples/journals/output/nightly/20260621T163957Z
```

结果：

| 指标 | 数值 |
|---|---:|
| Total fixtures | 4 |
| Succeeded | 4 |
| Failed | 0 |
| DOCX openable | 4 |
| DOCX ZIP openable | 4 |
| Word/LibreOffice openable | 0 |
| Word/LibreOffice skipped | 4 |
| Reports generated | 4 |
| Profile detection matched | 4 |
| Panic detected | 0 |

当前环境中的 LibreOffice 自检失败：

```text
LibreOffice conversion failed with status 1
```

因此本轮 Word-open 被标记为 skipped，而不是失败。

### 4.3 generic 结构/XML 回归

新增结构/XML 指标后已执行：

```bash
NIGHTLY_PROFILES=generic \
NIGHTLY_WORD_OPEN_CHECK=true \
ALLOW_FAILURES=true \
scripts/nightly_regression.sh
```

输出目录：

```text
examples/journals/output/nightly/20260621T164745Z
```

结果：

| 指标 | 数值 |
|---|---:|
| Total fixtures | 4 |
| Succeeded | 4 |
| Failed | 0 |
| DOCX openable | 4 |
| DOCX ZIP openable | 4 |
| DOCX structure valid | 4 |
| DOCX XML valid | 4 |
| DOCX XML skipped | 0 |
| Word/LibreOffice openable | 0 |
| Word/LibreOffice skipped | 4 |
| Reports generated | 4 |
| Profile detection matched | 4 |
| Panic detected | 0 |

### 4.4 强门禁行为验证

已执行：

```bash
NIGHTLY_PROFILES=__missing \
NIGHTLY_WORD_OPEN_CHECK=true \
NIGHTLY_WORD_OPEN_REQUIRED=true \
scripts/nightly_regression.sh
```

结果：

```text
退出码 1，符合预期
Nightly regression failed: LibreOffice word-open verifier is unavailable
```

这证明在生产 CI 中可以通过 `NIGHTLY_WORD_OPEN_REQUIRED=true` 强制要求 LibreOffice verifier 可用。

---

## 五、GitNexus 影响分析

### 5.1 nightly 脚本

`scripts/nightly_regression.sh` 当前仍是未跟踪文件，GitNexus 对该文件返回：

```text
Target not found
risk = UNKNOWN
```

因此脚本变更通过 shell 语法检查和实际回归结果验证。

### 5.2 serializer 变更

对 `serialize_document` 做 impact 分析：

```text
risk = CRITICAL
direct callers = 13
affected processes = 5
affected modules = 6
```

风险原因：

- `serialize_document` 是共享 DOCX 输出根函数。
- 影响 `pack_with_page_setup`。
- 影响旧 `doc_core` 转换路径。
- 影响多个 serializer 单元测试和图片/表格/公式输出流程。

本轮实际修改被严格限制在：

```text
w:document 根节点 namespace 去重
```

没有改变正文块、表格、图片、公式、引用的序列化逻辑。

---

## 六、剩余 P8 工作

P8 仍保持 `in_progress`，原因如下：

1. 当前环境 LibreOffice verifier 不可用，尚未在可用环境中完成真实 Word-open 通过率统计。
2. 尚未把 `NIGHTLY_WORD_OPEN_CHECK=true NIGHTLY_WORD_OPEN_REQUIRED=true` 接入 CI。
3. 当前已具备 DOCX 结构/XML 指标，但公式成功率、表格成功率、图片完整率、style coverage 仍未进入 nightly summary。
4. 每 profile 当前为 4 个 fixture，Beta 需要扩展到 10+，GA 需要扩展到 30+。
5. 失败样本库和失败分类仍未建立。

---

## 七、下一步计划

建议下一步继续 P8：

1. 在具备可用 LibreOffice 的 CI runner 中运行：

```bash
NIGHTLY_WORD_OPEN_CHECK=true \
NIGHTLY_WORD_OPEN_REQUIRED=true \
scripts/nightly_regression.sh
```

2. 若 Word-open 仍失败，按 fixture 收集：

```text
word-open log
document.xml
styles.xml
[Content_Types].xml
conversion report
```

3. 将失败分类写入 summary：

```text
verifier_unavailable
docx_zip_invalid
word_open_failed
xml_invalid
missing_relationship
unsupported_ooxml
```

4. 接入质量指标：

```text
formula_success_rate
table_success_rate
image_success_rate
style_coverage
unknown_macro_count
raw_fallback_count
```

5. 扩展 fixture 数量到 Beta 门槛：

```text
每 profile 10+ realistic fixture
```

---

## 八、商业化意义

本轮工作把 P8 从“文件生成验证”推进到“办公软件打开验证入口”：

```text
ZIP header check
  -> DOCX structure check
  -> XML well-formed check
  -> LibreOffice verifier preflight
  -> optional Word-open check
  -> required commercial gate
```

这对商业化很关键，因为用户实际关心的是：

```text
DOCX 能否被 Word/LibreOffice 打开
能否继续编辑
失败时能否诊断原因
```

本轮尚未证明所有 DOCX 都能实际打开，但已经建立了可复现的检测入口，并修复了一个真实的 OOXML 兼容性问题。
