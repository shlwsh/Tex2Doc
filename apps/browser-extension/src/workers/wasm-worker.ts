/**
 * WASM Worker for Tex2Doc Browser Extension
 *
 * 直接消费 wasm-bindgen 0.2.x 生成的官方 ESM 胶水（已由
 * `scripts/post-build-wasm.mjs` 转写为 IIFE + globalThis 形式并复制到
 * `src/workers/wasm-glue/doc_engine.js`）。
 *
 * 历史问题与解决方案：
 * - 旧版手写了一份 wasm-bindgen 0.2.x 的胶水（__wbindgen_malloc / externref
 *   table 等），但新版 wasm-bindgen 改名为 __wbindgen_export / __wbindgen_add_to_stack_pointer
 *   / __wbindgen_object_drop_ref / __wbindgen_object_clone_ref 等，缺少任意
 *   一个 import 都会触发：
 *       WebAssembly.instantiate(): Import #N "./doc_engine_bg.js" "__wbindgen_*":
 *       function import requires a callable
 * - Chrome MV3 service worker 禁止 `import()`（HTML spec 限制）；同时禁止
 *   `eval()` / `new Function()`（CSP `unsafe-eval` 未授权）。
 *   解决：用 Vite 静态 `import` 把 doc_engine.js 直接打进 bundle，
 *   并在打包阶段用 `scripts/post-build-wasm.mjs` 把 ESM 顶层
 *   `export { ... }` 改写成 `globalThis.__tex2docWbg = { ... }`、把
 *   `import.meta.url` 改写成 `globalThis.__tex2docBaseUrl`，把 convert_zip
 *   等导出函数挂到 `globalThis.__tex2docApi`。
 *   这样胶水代码会随 wasm-worker 一起被打进 background.js，不依赖 dynamic import。
 *
 * 运行时流程：
 *   1. fetch wasm URL 拿到字节；
 *   2. 调用 globalThis.__tex2docWbg.__wbg_init（或 default 导出，胶水顶层
 *      `export { __wbg_init as default }`）；
 *   3. 从 globalThis.__tex2docApi 取 convert_zip / convert_zip_to_docx / version；
 *   4. 后续所有转换都直接同步调用。
 *
 * 经过 Vite 静态 import 后，胶水代码在 background.js 顶部以 IIFE 形式执行：
 * 胶水内部会执行 (function(){...}).call(globalThis);
 * 因此只要 background.js 加载，胶水就已经把 API 挂到 globalThis 上了。
 */

// 通过静态 import 把 post-build-wasm.mjs 处理后的 IIFE 胶水代码内联进 bundle。
// 这样它在 service worker 里就直接可用，不需要 dynamic import / eval。
// 由于胶水顶层是 `(function(){...}).call(globalThis)`，副作用是：
//   - globalThis.__tex2docWbg = { initSync, default: __wbg_init }
//   - globalThis.__tex2docApi = { convert_zip, convert_zip_to_docx, version, ... }
// @ts-expect-error - 无 .d.ts 文件
import './wasm-glue/doc_engine.js';

export interface WasmConvertOptions {
  bib_style?: 'numeric' | 'author-year';
}

export interface WasmConvertResult {
  docx: Uint8Array;
  docx_len: number;
  warnings: string[];
}

interface DocEngineApi {
  /** 同步入口：返回 docx + 元信息 */
  convert_zip: (
    zip_bytes: Uint8Array,
    main_tex_path: string,
    options_js: string | null,
  ) => WasmConvertResult;
  /** 便捷入口：只返回 docx 字节流 */
  convert_zip_to_docx: (
    zip_bytes: Uint8Array,
    main_tex_path: string,
    options_js: string | null,
  ) => Uint8Array;
  /** 版本号 */
  version: () => string;
}

interface DocEngineWbg {
  initSync?: (module: unknown) => unknown;
  __wbg_init?: (input: unknown) => Promise<unknown>;
  default?: (input: unknown) => Promise<unknown>;
}

let api: DocEngineApi | null = null;
let loading: Promise<DocEngineApi> | null = null;
let ready = false;

// ============================================================================
// 初始化
// ============================================================================

export async function initWasm(): Promise<DocEngineApi> {
  if (api) return api;
  if (loading) return loading;

  loading = (async () => {
    // 1. 拉取 wasm 二进制
    const wasmUrl = browser.runtime.getURL('/wasm/doc_engine_bg.wasm');
    const wasmResp = await fetch(wasmUrl);
    if (!wasmResp.ok) {
      throw new Error(`fetch wasm failed: ${wasmResp.status} ${wasmResp.statusText}`);
    }
    const wasmBytes = await wasmResp.arrayBuffer();

    // 2. 拿到静态 import 时挂到 globalThis 上的胶水对象
    const g = globalThis as unknown as {
      __tex2docWbg?: DocEngineWbg;
      __tex2docBaseUrl?: string;
      __tex2docApi?: DocEngineApi;
    };

    // wasm-bindgen 0.2.x 的胶水顶层 export 是：
    //   export { initSync, __wbg_init as default };
    // 我们把它改成 globalThis.__tex2docWbg = { initSync, default: __wbg_init };
    // 所以这里 __wbg_init 在打包后被 mangled 成 .default。
    const wbgInit = g.__tex2docWbg?.__wbg_init ?? g.__tex2docWbg?.default;

    if (typeof wbgInit !== 'function') {
      throw new Error('__tex2docWbg.__wbg_init (default export) not found; wasm glue not loaded statically');
    }

    // 3. 注入 import.meta.url 的运行时值；wasm-bindgen 用它来算 wasm URL。
    const baseUrl = wasmUrl.substring(0, wasmUrl.lastIndexOf('/') + 1);
    g.__tex2docBaseUrl = baseUrl;

    // 4. 调用 __wbg_init 触发 WebAssembly.instantiate
    await wbgInit(
      new Response(wasmBytes, { headers: { 'Content-Type': 'application/wasm' } }),
    );

    if (!g.__tex2docApi || typeof g.__tex2docApi.convert_zip_to_docx !== 'function') {
      throw new Error('__tex2docApi.convert_zip_to_docx not found');
    }
    api = g.__tex2docApi;
    ready = true;
    console.log('[wasm-worker] api ready; version =', api.version ? api.version() : 'unknown');
    return api;
  })();

  try {
    return await loading;
  } finally {
    loading = null;
  }
}

export function isWasmReady(): boolean {
  return ready;
}

export async function getWasmVersion(): Promise<string> {
  const a = await initWasm();
  return a.version();
}

export async function convertZipToDocx(
  zipBytes: Uint8Array,
  mainTexPath: string,
  options?: WasmConvertOptions,
): Promise<WasmConvertResult> {
  const a = await initWasm();
  const optionsJson = options ? JSON.stringify(options) : '{}';
  return a.convert_zip(zipBytes, mainTexPath, optionsJson);
}

export async function convertZipToDocxBytes(
  zipBytes: Uint8Array,
  mainTexPath: string,
  options?: WasmConvertOptions,
): Promise<Uint8Array> {
  const a = await initWasm();
  const optionsJson = options ? JSON.stringify(options) : '{}';
  return a.convert_zip_to_docx(zipBytes, mainTexPath, optionsJson);
}

export function validateDocx(docxBytes: Uint8Array): boolean {
  return docxBytes.length >= 4 && docxBytes[0] === 0x50 && docxBytes[1] === 0x4b;
}

export function getFileSizeLimit(): number {
  return 10 * 1024 * 1024;
}

export function isWithinSizeLimit(fileSize: number): boolean {
  return fileSize <= getFileSizeLimit();
}

export function getFileSizeLimitDisplay(): string {
  const bytes = getFileSizeLimit();
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}