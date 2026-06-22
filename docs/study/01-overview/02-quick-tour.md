# 10 分钟快速上手

> 本节面向第一次接触 Tex2Doc 的工程师：在最小环境内跑通「LaTeX → docx」端到端链路。

---

## 2.1 准备环境

### 必需

| 工具 | 版本 | 用途 |
|------|------|------|
| **Rust** | stable ≥ 1.82 | 核心编译 |
| **cargo** | 随 Rust | 工作区构建 |
| **Node.js** | ≥ 18 LTS | 端到端 Playwright 验证 |
| **Playwright** | Chromium | 浏览器驱动（`@playwright/test@^1.49.0`） |

### 可选

| 工具 | 用途 |
|------|------|
| **Flutter SDK** ≥ 3.12 | Web / 桌面端构建 |
| **wasm-pack** | WASM 产物构建 |
| **Chrome 114+** | MV3 扩展联调 |
| **LibreOffice / TeX / pandoc** | V2 PDF 质量闭环和 pandoc 对照，非最小 DOCX 转换必需 |

> 仓库根的 `rust-toolchain.toml` 固定 stable channel，包含 `rustfmt` / `clippy` / `rust-src` 三个组件。

---

## 2.2 拉取代码

```bash
git clone <your-tex2doc-remote>
cd Tex2Doc
./scripts/install_commit_push_hook.ps1   # Windows: 启用 post-commit 自动 push
# 等价：git config core.hooksPath .githooks
```

---

## 2.3 最短路径：跑 paper3 compiler-engine 转换

```bash
bash scripts/build_paper3_compiler_engine_docx.sh
```

预期输出：

```text
docx: examples/paper3/output/to-docx/v13-论文稿件-jos-<TS>-compiler-engine.docx
blocks: 250
image-assets: 10
stage: SourceMount Completed
...
stage: DocxRender Completed
```

该命令只依赖 Rust/Cargo，直接调用 `doc-compiler-engine` 的 `paper3_to_docx` example。

---

## 2.4 跑核心库单元测试

```bash
cargo test --workspace --all-targets
```

预期看到（节选）：

```
running 6 tests
test tests::insert_and_read ... ok
test tests::missing_returns_error ... ok
...
test paper3_e2e::paper3_main_jos_to_docx ... ok

test result: ok. X passed; 0 failed
```

> `paper3_e2e` 是端到端集成测试，会把 `examples/paper3/latex/main-jos.tex` 转换为 docx 并落盘到 `examples/paper3/output/main-jos.docx`。

---

## 2.5 跑端到端视觉验证（Playwright）

```bash
# 一次性安装 Chromium
npx playwright install chromium

# 构建 WASM + Flutter Web（首次较慢，约 5-10 分钟）
npm run build:wasm
npm run build:web

# 启 PWA 服务（默认 4173 端口）
node scripts/serve_flutter_web.mjs &
sleep 3

# 跑 e2e：截图 + 内容断言
node scripts/e2e_paper3.mjs
```

预期产出：

* `examples/paper3/output/flutter-app.png` — Flutter Web 渲染截图
* `examples/paper3/output/playwright-report.html` — 报告
* `examples/paper3/output/main-jos.docx` — docx 落盘

---

## 2.6 桌面端冒烟（仅 Windows / macOS / Linux）

```bash
# 构建 Rust native cdylib + Flutter Windows app
npm run build:desktop

# 跑冒烟脚本
cd flutter_app && dart run bin/native_smoke.dart
```

预期：`examples/paper3/output/desktop-main-jos.docx` 出现。

---

## 2.7 命令行 one-shot 转换

把任意 LaTeX 项目转换到 docx：

```rust
// examples/one_shot/src/main.rs
use doc_core::{convert_dir, ConvertOptions};
use std::path::Path;

fn main() {
    let project_root = Path::new("./examples/paper3/latex");
    let main_tex = project_root.join("main-jos.tex");
    let result = convert_dir(project_root, &main_tex, &ConvertOptions::default()).unwrap();
    std::fs::write("output.docx", &result.docx).unwrap();
    println!("OK: {} bytes", result.docx.len());
}
```

或者直接调用集成测试（推荐做 CI 回归）：

```bash
cargo test -p doc-core --test paper3_e2e -- --nocapture
```

V2 CLI:

```bash
cargo run -p doc-engine -- convert \
  --zip examples/paper3/upload.zip \
  --main-tex main-jos.tex \
  --page-setup jos-paper3 \
  --out examples/paper3/output/to-docx/paper3-rust.docx
```

---

## 2.8 启动 HTTP 服务端

```bash
# 默认监听 0.0.0.0:8080
DOC_SERVER_ADDR=0.0.0.0:8080 cargo run -p doc-server

# 验证
curl http://127.0.0.1:8080/api/v1/health
# {"status":"ok"}

# 提交转换
curl -X POST http://127.0.0.1:8080/api/v1/convert \
  -F "file=@examples/paper3/upload.zip" \
  -F "main_tex=main-jos.tex" \
  -o out.docx
```

> 单请求体最大 50 MiB（`tower_http::RequestBodyLimitLayer`）。

---

## 2.9 常见问题

* **`cannot find -lffi`（Linux）**：缺 `libffi-dev` / `libclang-dev`。
* **WASM 编译失败**：升级 `wasm-pack` 至 ≥ 0.12，且 `wasm32-unknown-unknown` target 已安装（`rustup target add wasm32-unknown-unknown`）。
* **Flutter Web 加载慢**：首次 `flutter build web` 后约 50 MB（canvaskit），启用 `--no-source-maps --no-tree-shake-icons` 可减小。

---

## 2.10 下一步

* 想深入架构：读 [04-architecture/](../04-architecture/)
* 想看技术细节：读 [05-key-tech/](../05-key-tech/)
* 想部署：读 [07-deployment/](../07-deployment/)
