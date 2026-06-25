# 第六章 · CLI 与脚本使用

> 本节描述 **命令行 / Node 脚本** 形式的使用方式。最适合：CI 验证、本地一次性转换、批量处理。

---

## 1. Semantic TeX Engine 脚本（推荐 paper3 快速验证）

最新 paper3 语义编译入口：

```bash
bash scripts/build_paper3_compiler_engine_docx.sh
```

行为：

1. 读取 `examples/paper3/latex/main-jos.tex`。
2. 调用 `cargo run -p doc-compiler-engine --example paper3_to_docx`。
3. 使用 `EngineProfile::JosPaper` 和 `PageSetup::jos_paper3()`。
4. 输出到 `examples/paper3/output/to-docx/v13-论文稿件-jos-<TS>-compiler-engine.docx`。
5. 打印 `bytes`、`blocks`、`image-assets` 和每个编译阶段状态。

已验证的 paper3 输出特征：

```text
blocks: 250
image-assets: 10
stage: SourceMount Completed
stage: IncludeGraph Completed
stage: TexParse Completed
stage: SemanticCollect Completed
stage: DocumentGraph Completed
stage: DocxRender Completed
```

也可以直接运行 example：

```bash
cargo run -p doc-compiler-engine --example paper3_to_docx -- \
  --project-root examples/paper3/latex \
  --main-tex examples/paper3/latex/main-jos.tex \
  --profile jos-paper \
  --out examples/paper3/output/to-docx/paper3-compiler-engine.docx
```

---

## 2. V2 CLI（`doc-engine`）

统一 CLI 位于 `crates/cli`：

```bash
cargo run -p doc-engine -- --help
```

子命令：

| 子命令 | 用途 |
|---|---|
| `convert` | zip → DOCX，兼容 `doc-core` |
| `tex-compile` | TeX → oracle PDF |
| `docx-to-pdf` | DOCX → PDF，默认 LibreOffice headless |
| `verify-pdf` | 结构/文本/视觉三层质量对比 |
| `build` | 串联 convert、tex-compile、docx-to-pdf、verify-pdf |
| `ast-dump` | 输出标准文档 AST |
| `render-dump` | 输出 DOCX 渲染树 |
| `docx-diff` | 对比两个 DOCX 的内容、样式和 OOXML hash |

示例：

```bash
cargo run -p doc-engine -- convert \
  --zip examples/paper3/upload.zip \
  --main-tex main-jos.tex \
  --page-setup jos-paper3 \
  --out examples/paper3/output/to-docx/paper3-rust.docx
```

完整质量闭环：

```bash
cargo run -p doc-engine -- build \
  --zip examples/paper3/upload.zip \
  --main-tex main-jos.tex \
  --latex-main main-jos.tex \
  --page-setup jos-paper3 \
  --outdir examples/paper3/output/to-docx
```

---

## 3. 集成测试入口（`paper3_e2e`）

最常用的「跑一下看看」入口。

```bash
# 在仓库根
cargo test -p doc-core --test paper3_e2e -- --nocapture
```

行为：
1. 读 `examples/paper3/latex/main-jos.tex`（含 6 个 `\input` + `references.bib`）。
2. `convert_dir` 转换。
3. 写 `examples/paper3/output/main-jos.docx`。
4. 断言：`PK\x03\x04` 头 + 含 `word/document.xml` + `word/styles.xml` + 块统计 + 关键短语 + 杂质剥离。

输出示例：
```
📊 块统计：para=42 list=8 eq=12 fig=6 tbl=5 h=15 raw=3
✅ docx 落盘：examples/paper3/output/main-jos.docx
```

---

## 4. 完整 workspace 测试

```bash
cargo test --workspace --all-targets
```

| 范围 | 命令 |
|------|------|
| 全部 | `cargo test --workspace` |
| 含集成测试 | `cargo test --workspace --all-targets` |
| 单 crate | `cargo test -p doc-latex-reader` |
| 单测试 | `cargo test -p doc-latex-reader lower_inline_math` |
| 显示 println | 加 `-- --nocapture` |

---

## 5. 端到端视觉验证（Node 脚本）

### 5.1 PowerShell 版

```powershell
.\scripts\verify_paper3.ps1
```

* 调 cargo test 生成 docx
* 调 Playwright 截图 + 内容断言
* 写 `examples/paper3/output/report.html` + `preview.png`
* 退出码 0=通过，1=失败

### 5.2 Node 版

```bash
node scripts/verify_paper3.mjs
```

* 跳过 cargo（复用现有 docx）：`node scripts/verify_paper3.mjs --no-cargo`

---

## 6. Playwright e2e（Web PWA）

```bash
# 一次性
npm run build:wasm
npm run build:web

# 启 PWA 服务
node scripts/serve_flutter_web.mjs &
sleep 3

# 跑 e2e
node scripts/e2e_paper3.mjs
```

行为：
1. 启 Playwright Chromium。
2. 访问 `http://127.0.0.1:2627/`。
3. 等 Flutter 容器挂载 + 2s 渲染 → 截图 `flutter-app.png`。
4. 等 `window.docEngine` 就绪 → 读 `version()`。
5. 上传 `examples/paper3/upload.zip` → 调 `window.docEngine.convert_zip_to_docx`。
6. 解压 docx → 抽 `word/document.xml` → 关键短语 + 杂质断言。
7. 写 `playwright-report.html`。

> 需要 `npm install` 安装 `playwright` + `fflate`（已在 `package.json` 锁定）。

---

## 7. 桌面端冒烟

### 7.1 自动构建（Windows）

```bash
npm run build:desktop
# 等价于：cargo build -p doc-native && cd flutter_app && flutter build windows --debug
```

CMake 会自动调 `cargo build -p doc-native` 并把 `doc_native.dll` 拷贝到 `bin/`。

### 7.2 跑冒烟

```bash
cd flutter_app
dart run bin/native_smoke.dart
# 或：dart run bin/native_smoke.dart main-zh.tex
```

行为：
1. 读 `examples/paper3/upload.zip`。
2. 调 `NativeBridge.instance.convertZipToDocx`。
3. 写 `examples/paper3/output/desktop-main-jos.docx`。
4. 断言：≥ 4 KiB + `PK\x03\x04` 头。

退出码：
* 0 = 通过
* 2 = 缺 zip
* 3 = 库 init 失败
* 4 = docx 过小
* 5 = 魔数错

### 7.3 环境变量

* `DOC_ENGINE_LIB` 覆盖默认动态库名（CI / 联调时）：
  ```bash
  DOC_ENGINE_LIB=custom_doc_engine.dll dart run bin/native_smoke.dart
  ```

---

## 8. HTTP 服务端

### 8.1 启动

```bash
DOC_SERVER_ADDR=0.0.0.0:2624 cargo run --release -p doc-server
```

### 8.2 健康检查

```bash
curl http://127.0.0.1:2624/api/v1/health
# {"status":"ok"}
```

### 8.3 转换

```bash
curl -X POST http://127.0.0.1:2624/api/v1/convert \
  -F "file=@examples/paper3/upload.zip" \
  -F "main_tex=main-jos.tex" \
  -o out.docx

file out.docx
# Microsoft Word 2007+
```

### 8.4 限制

* 请求体 ≤ 50 MiB。
* docx 至少 4 KiB + `PK\x03\x04` 头。

---

## 9. 自定义转换（Rust API）

如果想在自己的 Rust 项目中调用 `doc-core`：

```rust
// Cargo.toml
[dependencies]
doc-core = { path = "path/to/Tex2Doc/crates/core" }

[build-dependencies]
# 也许需要 doc-latex-reader / doc-docx-writer 跨 crate 类型
```

```rust
use doc_core::{convert_dir, ConvertOptions};
use std::path::Path;

fn main() {
    let opts = ConvertOptions::default();
    let result = convert_dir(
        Path::new("./examples/paper3/latex"),
        Path::new("./examples/paper3/latex/main-jos.tex"),
        &opts,
    ).expect("转换失败");
    std::fs::write("output.docx", &result.docx).unwrap();
    println!("✅ {} bytes", result.docx.len());
}
```

### 9.1 `ConvertOptions`

```rust
pub struct ConvertOptions {
    pub bib_style: BibStyle,                  // Numeric | AuthorYear
    pub template: Option<Vec<u8>>,            // 旧字段，保留
    pub attachments: Vec<Attachment>,         // 旧字段
    pub template_bytes: Option<Vec<u8>>,      // 新字段：reference.docx 字节
}
```

### 9.2 `ConvertResult`

```rust
pub struct ConvertResult {
    pub docx: Vec<u8>,
    pub warnings: Vec<String>,
}
```

### 9.3 三个入口对比

| 入口 | 何时用 |
|------|--------|
| `convert_sync(main_tex, source, opts)` | V1 测试；不推荐产线 |
| `convert_dir(project_root, main_tex, opts)` | 本地 / CLI / 桌面端 |
| `convert_zip(zip_bytes, main_tex_path, opts)` | WASM / HTTP |

---

## 10. 脚本速查

| 脚本 | 用途 |
|------|------|
| `scripts/build_paper3_zip.mjs` | 把 `examples/paper3/latex/` 打包为 `upload.zip` |
| `scripts/build_paper3_compiler_engine_docx.sh` | 用 `doc-compiler-engine` 直接把 paper3 转为 DOCX |
| `scripts/build_paper3_dual_docx.sh` | 生成 sh/rust 双版本 DOCX，并在可用时生成 pandoc 对照 |
| `scripts/build_paper3_pandoc_docx.sh` | 用 pandoc 生成 paper3 DOCX 对照基线 |
| `scripts/serve_flutter_web.mjs` | 静态服务器（端口 2627） |
| `scripts/verify_install.mjs` | 环境自检 |
| `scripts/verify_paper3.mjs` | 端到端验证（Playwright + 报告） |
| `scripts/e2e_paper3.mjs` | Playwright 验证 Flutter Web |
| `scripts/e2e_server.mjs` | curl 验证 HTTP 服务 |
| `scripts/e2e_extension.mjs` | Playwright 验证 Chrome 扩展 |
| `scripts/verify_paper3.ps1` | PowerShell 版 verify |
| `scripts/commit_push.ps1` | 自动 add / commit / push |
| `scripts/install_commit_push_hook.ps1` | 启用 post-commit 钩子 |
| `scripts/link_cursor_skills.sh` | 链接 .cursor/skills 到 .agent/skills |

---

## 11. 进一步阅读

* [02-pwa-web.md](./02-pwa-web.md) — Flutter Web PWA 使用
* [03-desktop.md](./03-desktop.md) — Flutter Desktop 使用
* [04-chrome-extension.md](./04-chrome-extension.md) — Chrome 扩展使用
* [05-http-server.md](./05-http-server.md) — HTTP 服务端使用
