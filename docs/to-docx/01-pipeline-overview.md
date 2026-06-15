# 01 · 入口与产物（shell → python → docx）

> 本章复刻 `scripts/build_docx.sh` 的全部行为，并标注 Rust 重构时应当保留的语义。

## 1.1 shell 入口的 7 个职责

`build_docx.sh` 只有 ~100 行，承担以下职责（缺一不可）：

1. **解析仓库根**：`cd "$(dirname "${BASH_SOURCE[0]}")/.."` 把脚本目录上一级当作 ROOT，所有路径基于 ROOT 解析。
2. **声明路径变量**：
   - `FORMAT_JSON = ROOT/docs/format/jos_2025_docx_format_definitions.json`
   - `TEX_SRC = ROOT/latex/main-jos.tex`
   - `PDF_SRC = ROOT/latex/main-jos.pdf`（必须已由 `build_pdf_jos.sh` 生成）
   - `OUTPUT_DIR = ROOT/docs/to-docx`
3. **依赖检查** `check_deps()`：缺一即 abort。
4. **版本号** `VERSION`：支持命令行传入 `vN`，否则自动递增（扫 `docs/` 下已存在的 `v*-论文稿件-*` 文件名，解析编号 `+1`）。
5. **时间戳** `TS = date +%Y%m%d-%H%MS`，用于文件命名。
6. **运行子脚本**：`build_jos_docx.py` 生成 docx，`verify_jos_docx.py` 校验生成 MD + JSON 报告。
7. **打印总结**：DOCX 路径、报告路径、文件大小（用 `numfmt --to=iec`）。

Rust 重构时若想完全替换 shell，**这 7 步都对应到 Rust 入口函数的不同分支**：

| shell 行 | Rust 入口函数 | 备注 |
|---------|--------------|------|
| `ROOT=...` | `fn main()` 中 `let root = env::current_dir()?` | 或显式 `--root` 参数 |
| 路径常量 | `const FORMAT_JSON: &str = "docs/format/...";` | |
| `check_deps` | `fn check_deps() -> Result<()>` | 见 §1.3 |
| `MAX_V` 自增 | `fn next_version(root: &Path) -> u32` | 见 §1.4 |
| `TS` | `chrono::Local::now().format("%Y%m%d-%H%M%S")` | |
| `python3 ...` | 调用 `build_jos_docx`/`verify_jos_docx` 库函数 | |

## 1.2 依赖清单

`check_deps` 失败时打印 `✗ <dep>` 并 `exit 1`。Rust 重写时应当返回 `Err(BuildError::MissingDep(...))`。

| 项 | 来源 | 用途 | Rust 替代 |
|----|------|------|----------|
| `python3` | 系统 | 运行 Python 脚本 | 自身就是 Rust 可执行文件 |
| `pdftotext`（包名 `poppler-utils`） | 系统 | `verify_jos_docx.py` 抽 PDF 文本做对比 | `lopdf` crate |
| `PIL` / `Pillow` | `pip install pillow` | 读 PNG/JPEG 像素尺寸以计算图片 EMU 尺寸 | `image` crate |
| `docs/format/jos_2025_docx_format_definitions.json` | 仓库 | 格式定义唯一来源 | `serde_json` 解析 |
| `latex/main-jos.tex` | 仓库 | 源文 | 直接读 |
| `latex/main-jos.pdf` | 仓库（先跑 `build_pdf_jos.sh`） | 校验用 | `lopdf` 读 |

`pdftoppm` 不是 `build_docx.sh` 的硬依赖——只有当源文是 `.pdf` 图片而目标不存在时才被 `build_jos_docx.py` 调用；缺失时该图片会被替换为 "[缺图] <caption>" 占位段。Rust 重构建议**用 `pdfium-render` 或 `lopdf + image` 主动转**而不是依赖外部命令。

## 1.3 版本号策略

```bash
MAX_V="$(find "${ROOT}/docs" -type f -name 'v*-论文稿件-*' -printf '%f\n' \
    | sed -n 's/^v\([0-9]\+\)-论文稿件-.*/\1/p' \
    | sort -n \
    | tail -1 || true)"
VERSION=$(( ${MAX_V:-0} + 1 ))
```

逻辑：在 `docs/` 找 `v<num>-论文稿件-*` 文件，提取 `num`，排序取最大，加 1。等价 Rust 伪代码：

```rust
fn next_version(root: &Path) -> u32 {
    let mut max_v = 0u32;
    for entry in walkdir::WalkDir::new(root.join("docs")) {
        let name = entry.file_name().to_string_lossy();
        if let Some(rest) = name.strip_prefix('v') {
            if let Some((num, _)) = rest.split_once('-') {
                if let Ok(n) = num.parse::<u32>() {
                    max_v = max_v.max(n);
                }
            }
        }
    }
    max_v + 1
}
```

> ⚠️ Python 的 `find ... -name 'v*-论文稿件-*'` **同时匹配 `to-docx/` 下的产物和 `docs/` 根目录下的旧 PDF**。所以 `MAX_V` 实际上是仓库里所有 `v<N>-论文稿件-*` 编号的最大值。Rust 实现要保留这一行为。

## 1.4 产物命名

```text
v${VERSION}-论文稿件-jos-${TS}.docx
v${VERSION}-论文稿件-jos-${TS}-docx校验报告.md
v${VERSION}-论文稿件-jos-${TS}-docx校验报告.json
```

例：`v65-论文稿件-jos-20260614-103045.docx`。

- `jos` = JOS（Journal of Software / 软件学报）
- `TS` 来自 `date +%Y%m%d-%H%M%S`
- 校验报告与 docx **同前缀**，仅多 `-docx校验报告.{md,json}` 后缀
- 报告 MD 给人看、JSON 给 CI 解析

## 1.5 子脚本职责边界

| 脚本 | 是否阻塞 docx 生成 | 是否阻塞报告 | 失败时 shell 行为 |
|------|------------------|-------------|------------------|
| `python3 build_jos_docx.py` | 是 | 否 | `set -euo pipefail` → 整体退出非 0 |
| `python3 verify_jos_docx.py` | 否 | 是 | 整体退出非 0，但 docx 已写盘 |

注意 `verify_jos_docx.py` 的 `main()` 返回 0 或 1，**1 表示 DOCX 不通过校验**。Rust 重构时建议**两步都返回 `Result`**，但 `verify` 失败**不删除 docx**，让用户能看到产物并修复。

## 1.6 shell 与 Python 之间的数据契约

shell 不解析 docx，只**传递文件路径**：

```text
--root  ROOT
--format docs/format/jos_2025_docx_format_definitions.json
--output docs/to-docx/vN-论文稿件-jos-TS.docx
```

校验脚本额外：

```text
--docx 已生成 docx 路径
--pdf  ROOT/latex/main-jos.pdf
--format 同上
--report  docx校验报告.md 路径
--json-report docx校验报告.json 路径
--allowed-footer 1260   ← 页脚距离容差（与样本的 567 twips 不同的微调）
```

Rust 重构时**不需要复刻这层 CLI**——直接把两个入口写成 Rust 的 `build` 与 `verify` 函数，shell 仅剩一个薄包装（或干脆删除 shell）。

## 1.7 时序图（一次完整 build）

```text
shell                              Python 子进程
─────                              ────────────
ROOT=...
check_deps ─────► (系统)
VERSION, TS = ...

mkdir OUTPUT_DIR

python3 build_jos_docx.py ─────► build_jos_docx.main()
    --root ROOT
    --format FORMAT_JSON
    --output DOCX_DST
                                │
                                ├── json.load(FORMAT_JSON)
                                ├── Manuscript = build_manuscript(ROOT)
                                ├── DocxBuilder(format_data, ...)
                                ├── populate(builder, ms)
                                ├── write_docx(...)            ──► [Content_Types].xml
                                │                                  _rels/.rels
                                │                                  word/document.xml
                                │                                  word/styles.xml
                                │                                  word/settings.xml
                                │                                  word/header[012].xml
                                │                                  word/footer[123].xml
                                │                                  word/media/*.png
                                │                              ◄── "DOCX written: ..."
                                │
                                └── exit 0

python3 verify_jos_docx.py ────► verify_jos_docx.main()
    --docx DOCX_DST
    --pdf  PDF_SRC
    --format FORMAT_JSON
    --report  REPORT_DST
    --json-report REPORT_JSON
                                │
                                ├── zipfile.read(docx, xml)
                                ├── pdftotext(pdf)
                                ├── ElementTree(docx).findall(...)
                                ├── 30+ checks
                                ├── make_report(...)
                                └── exit 0/1

stat -c%s DOCX_DST
echo "完成"
```

## 1.8 行为不变性

Rust 重构时**必须保留**的几条不变量：

1. docx 输出**不依赖外部 Word/Pandoc**——纯自写 OOXML。
2. 校验**不修改 docx**，只读 + 写报告。
3. 报告 MD 与 JSON **同时**生成，路径同前缀。
4. 缺图时写占位段 `[缺图] <caption>` 而不是直接 fail。
5. 缺依赖时**明确报错**而不是静默继续。
