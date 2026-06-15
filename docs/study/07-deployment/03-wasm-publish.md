# WASM 产物发布

> 本节描述 `doc-wasm` 产物的构建、复制、发布流程。WASM 是 Flutter Web 和 Chrome 扩展的共享底层。

---

## 1. 工具链

### 1.1 wasm-pack

```bash
# 安装
curl https://rustwasm.github.io/wasm-pack/installer/init.sh | sh
# 或：cargo install wasm-pack

# 验证
wasm-pack --version
```

* **版本要求**：≥ 0.12。
* 自动包含：wasm-bindgen、wasm-opt。

### 1.2 Rust target

```bash
rustup target add wasm32-unknown-unknown
```

### 1.3 wabt（可选，用于反汇编 / 验证）

```bash
# 仅调试用
cargo install wabt
wasm2wat target/wasm32-unknown-unknown/.../doc_engine_bg.wasm -o doc.wat
```

---

## 2. 构建命令

### 2.1 项目内置 npm 脚本

```bash
# 仓库根
npm run build:wasm
# 等价于：
#   wasm-pack build crates/wasm \
#     --target web \
#     --out-dir ../flutter_app/wasm/pkg \
#     --out-name doc_engine \
#     --dev
```

* `--target web`：生成 ESM 模块（`import.meta.url` 解析）。
* `--out-dir ../flutter_app/wasm/pkg`：输出目录。
* `--out-name doc_engine`：导出文件名前缀。
* `--dev`：dev build（关闭 LTO / 优化，编译快）。

### 2.2 Release 构建

```bash
wasm-pack build crates/wasm \
  --target web \
  --out-dir ../flutter_app/wasm/pkg \
  --out-name doc_engine \
  --release
```

* 体积减小 30%+，但编译慢。

### 2.3 完整命令列表

| 命令 | 用途 |
|------|------|
| `wasm-pack build --target web` | Web（ESM） |
| `wasm-pack build --target bundler` | Webpack / Rollup 集成 |
| `wasm-pack build --target nodejs` | Node.js（CommonJS） |
| `wasm-pack build --target deno` | Deno |
| `wasm-pack build --target no-modules` | 旧 `<script>` 加载 |

Tex2Doc 选 `--target web`（最通用）。

---

## 3. 产物

```
flutter_app/wasm/pkg/
├── doc_engine.js                  # ESM 入口（~14 KB）
├── doc_engine.d.ts                # TypeScript 类型（~3 KB）
├── doc_engine_bg.wasm             # WASM 字节流（~3.5 MB dev / ~2.5 MB release）
├── doc_engine_bg.wasm.d.ts        # TypeScript 声明
├── package.json                   # npm 元信息
├── README.md                      # 由 wasm-pack 生成
└── .gitignore                     # 由 wasm-pack 生成
```

---

## 4. 复制到目标位置

### 4.1 Flutter Web

```bash
# Flutter build web 会自动嵌入 wasm/
flutter build web --release

# 或手动复制（开发期）
cp flutter_app/wasm/pkg/doc_engine.js flutter_app/web/wasm/
cp flutter_app/wasm/pkg/doc_engine_bg.wasm flutter_app/web/wasm/
```

### 4.2 Chrome 扩展

```bash
cp flutter_app/wasm/pkg/doc_engine.js extension/popup/wasm/
cp flutter_app/wasm/pkg/doc_engine_bg.wasm extension/popup/wasm/
```

### 4.3 自动化（推荐）

把复制步骤加到 `npm run build:wasm` 之后：

```json
{
  "scripts": {
    "build:wasm": "wasm-pack build crates/wasm --target web --out-dir ../flutter_app/wasm/pkg --out-name doc_engine --dev",
    "build:wasm:release": "wasm-pack build crates/wasm --target web --out-dir ../flutter_app/wasm/pkg --out-name doc_engine --release",
    "copy:wasm": "node scripts/copy_wasm.mjs"
  }
}
```

或 PowerShell：

```powershell
# scripts/copy_wasm.ps1
Copy-Item flutter_app/wasm/pkg/doc_engine.js extension/popup/wasm/ -Force
Copy-Item flutter_app/wasm/pkg/doc_engine_bg.wasm extension/popup/wasm/ -Force
Write-Host "✅ WASM copied to extension"
```

---

## 5. 在 JS 中使用

### 5.1 Web（HTML）

```html
<!DOCTYPE html>
<html>
<head><title>Doc-engine</title></head>
<body>
<script type="module">
  import init, { convert_zip_to_docx, version } from './wasm/doc_engine.js';
  
  await init();  // 默认 fetch + instantiate
  
  console.log('Version:', version());
  
  // 假设已获取 zip 字节
  const zipBytes = new Uint8Array(...);
  const docxBytes = convert_zip_to_docx(zipBytes, 'main-jos.tex', '');
  console.log('docx:', docxBytes.length, 'bytes');
</script>
</body>
</html>
```

### 5.2 内联 init

```javascript
// 用预加载的 wasm bytes
const wasmUrl = chrome.runtime.getURL('popup/wasm/doc_engine_bg.wasm');
const resp = await fetch(wasmUrl);
const wasmBuffer = await resp.arrayBuffer();
const mod = await import('./wasm/doc_engine.js');
await mod.default({ wasmBinary: wasmBuffer });
const docxBytes = mod.convert_zip_to_docx(zipBytes, 'main.tex', '');
```

### 5.3 TypeScript

```typescript
import init, { convert_zip_to_docx, version } from './wasm/doc_engine';
import type { ConvertResultJs } from './wasm/doc_engine';

await init();
const v: string = version();
const r: Uint8Array = convert_zip_to_docx(zip, 'main.tex', '');
```

---

## 6. 在 Flutter（Dart）使用

```dart
// lib/wasm_bridge.dart
final JSFunction fn = globalContext
    .getProperty<JSObject>('docEngine'.toJS)
    .getProperty<JSFunction>('convert_zip_to_docx'.toJS);

final out = fn.callAsFunction(
    ns,
    zipBytes.toJS,                  // Dart → JS（零拷贝）
    mainTexPath.toJS,
    (optionsJson ?? '').toJS,
);
return (out as JSUint8Array).toDart;  // JS → Dart（零拷贝）
```

详见 [04-architecture/03-frontend-bridges.md](../04-architecture/03-frontend-bridges.md)。

---

## 7. 性能优化

### 7.1 体积

* Release 模式：`--release` 标志。
* `wasm-opt -Oz`：默认开启。
* 进一步压缩：`brotli` / `gzip`（HTTP 层）。

```bash
# brotli
brotli -k doc_engine_bg.wasm
# 产物：doc_engine_bg.wasm.br（~1 MB）

# 在 nginx 中启用
brotli on;
brotli_types application/wasm;
add_header Content-Encoding br;
```

### 7.2 启动时间

* **Streaming compilation**：`new WebAssembly.Module(wasmBytes, { builtins: ['js-string'] })` + `WebAssembly.instantiate`。
* **预加载**：`index.html` 中 `<link rel="preload" as="fetch" href="doc_engine_bg.wasm" crossorigin>`。
* **SharedArrayBuffer**：需要 COOP/COEP（V2 路线）。

### 7.3 内存

* 默认 `wasm32-unknown-unknown` 限制 4 GB 虚拟内存。
* `wasm-pack` 默认无 `--shared-memory`（不需要）。

---

## 8. 调试

### 8.1 Chrome DevTools

* `chrome://inspect` → 找到 wasm 模块。
* Source panel → `doc_engine_bg.wasm`（带 DWARF 信息）。

### 8.2 console.log

```rust
// crates/wasm/src/lib.rs
#[wasm_bindgen]
impl WasmError {
    pub fn message(&self) -> String {
        self.message.clone()
    }
}
```

JS 端：

```javascript
try {
  await mod.default();
} catch (e) {
  console.error('WASM init failed:', e);
}
```

### 8.3 panic hook

`crates/wasm/Cargo.toml`：

```toml
[features]
default = []
console_error_panic_hook = ["dep:console_error_panic_hook"]
```

启用：

```bash
wasm-pack build --target web -- --features doc-wasm/console_error_panic_hook
```

JS 端 panic 会打印到 console。

### 8.4 性能 profiling

* Chrome DevTools → Performance → 录制。
* `console.time('convert')` / `console.timeEnd('convert')`。

---

## 9. 故障排查

### 9.1 `wasm-pack not found`

```bash
cargo install wasm-pack
```

### 9.2 `wasm32-unknown-unknown target not installed`

```bash
rustup target add wasm32-unknown-unknown
```

### 9.3 `failed to fetch wasm`

* 路径错：检查 `wasm/doc_engine.js` 与 `wasm/doc_engine_bg.wasm` 相对位置。
* HTTP 限制：WASM 需 HTTPS（localhost 例外）。
* CORS：服务器需 `Cross-Origin-Embedder-Policy: require-corp` 或同源。

### 9.4 WASM 体积过大

* 启用 `wasm-opt -Oz`（默认）。
* 剥离符号：`wasm-strip`。
* 移除无用 crate 依赖。

### 9.5 `RuntimeError: out of memory`

* 检查输入：超大 zip / 巨大图片。
* 用 streaming 编译减少初始内存。
* 拆分输入。

---

## 10. 发布 checklist

发布新版本 WASM 产物时：

- [ ] `wasm-pack build --release`
- [ ] 复制到 `flutter_app/wasm/pkg/`
- [ ] 复制到 `flutter_app/web/wasm/`（如手动维护）
- [ ] 复制到 `extension/popup/wasm/`
- [ ] 跑 `cargo test -p doc-wasm`
- [ ] 跑 `node scripts/e2e_paper3.mjs`（Web 验证）
- [ ] 跑 `node scripts/e2e_extension.mjs`（扩展验证）
- [ ] 更新 `flutter_app/wasm/pkg/package.json` 版本号
- [ ] 写 CHANGELOG
- [ ] git tag（如版本号变更）

---

## 11. 进一步阅读

* [01-rust-build.md](./01-rust-build.md) — Rust 核心构建
* [02-flutter-build.md](./02-flutter-build.md) — Flutter 构建
* [05-extension-pack.md](./05-extension-pack.md) — 扩展打包
* [04-architecture/03-frontend-bridges.md](../04-architecture/03-frontend-bridges.md) — WASM 集成
