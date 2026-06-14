# 第六章 · CLI 与脚本使用

> 本节描述 **命令行 / Node 脚本** 形式的使用方式。最适合：CI 验证、本地一次性转换、批量处理。

---

## 1. 集成测试入口（`paper3_e2e`）

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

## 2. 完整 workspace 测试

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

## 3. 端到端视觉验证（Node 脚本）

### 3.1 PowerShell 版

```powershell
.\scripts\verify_paper3.ps1
```

* 调 cargo test 生成 docx
* 调 Playwright 截图 + 内容断言
* 写 `examples/paper3/output/report.html` + `preview.png`
* 退出码 0=通过，1=失败

### 3.2 Node 版

```bash
node scripts/verify_paper3.mjs
```

* 跳过 cargo（复用现有 docx）：`node scripts/verify_paper3.mjs --no-cargo`

---

## 4. Playwright e2e（Web PWA）

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
2. 访问 `http://127.0.0.1:4173/`。
3. 等 Flutter 容器挂载 + 2s 渲染 → 截图 `flutter-app.png`。
4. 等 `window.docEngine` 就绪 → 读 `version()`。
5. 上传 `examples/paper3/upload.zip` → 调 `window.docEngine.convert_zip_to_docx`。
6. 解压 docx → 抽 `word/document.xml` → 关键短语 + 杂质断言。
7. 写 `playwright-report.html`。

> 需要 `npm install` 安装 `playwright` + `fflate`（已在 `package.json` 锁定）。

---

## 5. 桌面端冒烟

### 5.1 自动构建（Windows）

```bash
npm run build:desktop
# 等价于：cargo build -p doc-native && cd flutter_app && flutter build windows --debug
```

CMake 会自动调 `cargo build -p doc-native` 并把 `doc_native.dll` 拷贝到 `bin/`。

### 5.2 跑冒烟

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

### 5.3 环境变量

* `DOC_ENGINE_LIB` 覆盖默认动态库名（CI / 联调时）：
  ```bash
  DOC_ENGINE_LIB=custom_doc_engine.dll dart run bin/native_smoke.dart
  ```

---

## 6. HTTP 服务端

### 6.1 启动

```bash
DOC_SERVER_ADDR=0.0.0.0:8080 cargo run --release -p doc-server
```

### 6.2 健康检查

```bash
curl http://127.0.0.1:8080/api/v1/health
# {"status":"ok"}
```

### 6.3 转换

```bash
curl -X POST http://127.0.0.1:8080/api/v1/convert \
  -F "file=@examples/paper3/upload.zip" \
  -F "main_tex=main-jos.tex" \
  -o out.docx

file out.docx
# Microsoft Word 2007+
```

### 6.4 限制

* 请求体 ≤ 50 MiB。
* docx 至少 4 KiB + `PK\x03\x04` 头。

---

## 7. 自定义转换（Rust API）

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

### 7.1 `ConvertOptions`

```rust
pub struct ConvertOptions {
    pub bib_style: BibStyle,                  // Numeric | AuthorYear
    pub template: Option<Vec<u8>>,            // 旧字段，保留
    pub attachments: Vec<Attachment>,         // 旧字段
    pub template_bytes: Option<Vec<u8>>,      // 新字段：reference.docx 字节
}
```

### 7.2 `ConvertResult`

```rust
pub struct ConvertResult {
    pub docx: Vec<u8>,
    pub warnings: Vec<String>,
}
```

### 7.3 三个入口对比

| 入口 | 何时用 |
|------|--------|
| `convert_sync(main_tex, source, opts)` | V1 测试；不推荐产线 |
| `convert_dir(project_root, main_tex, opts)` | 本地 / CLI / 桌面端 |
| `convert_zip(zip_bytes, main_tex_path, opts)` | WASM / HTTP |

---

## 8. Node 脚本速查

| 脚本 | 用途 |
|------|------|
| `scripts/build_paper3_zip.mjs` | 把 `examples/paper3/latex/` 打包为 `upload.zip` |
| `scripts/serve_flutter_web.mjs` | 静态服务器（端口 4173） |
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

## 9. 进一步阅读

* [02-pwa-web.md](./02-pwa-web.md) — Flutter Web PWA 使用
* [03-desktop.md](./03-desktop.md) — Flutter Desktop 使用
* [04-chrome-extension.md](./04-chrome-extension.md) — Chrome 扩展使用
* [05-http-server.md](./05-http-server.md) — HTTP 服务端使用
