/**
 * WASM Worker for Tex2Doc Browser Extension
 *
 * Inline wasm-bindgen helpers to avoid CSP issues in service worker
 */

export interface WasmConvertOptions {
  bib_style?: 'numeric' | 'author-year';
}

export interface WasmConvertResult {
  docx: Uint8Array;
  docx_len: number;
  warnings: string[];
}

interface DocEngineExports {
  convert_zip: (zipBytes: Uint8Array, mainTexPath: string, options: string) => WasmConvertResult;
  convert_zip_to_docx: (zipBytes: Uint8Array, mainTexPath: string, options: string) => Uint8Array;
  version: () => string;
}

let wasmModule: DocEngineExports | null = null;
let wasmLoading: Promise<DocEngineExports> | null = null;
let wasmReady = false;

// WASM module reference
let wasm: Record<string, unknown> = {};

// ============================================================================
// wasm-bindgen helpers (inlined from doc_engine.js)
// ============================================================================

let WASM_VECTOR_LEN: number;
const cachedTextEncoder = new TextEncoder();
const cachedTextDecoder = new TextDecoder('utf-8', { fatal: true });

let cachedDataViewMemory0: DataView | null = null;
let cachedUint8ArrayMemory0: Uint8Array | null = null;

function _assertNum(n: unknown): void {
  if (typeof n !== 'number') throw new Error(`expected a number argument, found ${typeof n}`);
}

function getArrayU8FromWasm0(ptr: number, len: number): Uint8Array {
  ptr = ptr >>> 0;
  return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

function getDataViewMemory0(): DataView {
  if (cachedDataViewMemory0 === null || (cachedDataViewMemory0.buffer as ArrayBuffer & {detached?: boolean}).detached === true) {
    cachedDataViewMemory0 = new DataView((wasm.memory as WebAssembly.Memory).buffer);
  }
  return cachedDataViewMemory0;
}

function getStringFromWasm0(ptr: number, len: number): string {
  return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr >>> 0, (ptr >>> 0) + len));
}

function getUint8ArrayMemory0(): Uint8Array {
  if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
    cachedUint8ArrayMemory0 = new Uint8Array((wasm.memory as WebAssembly.Memory).buffer);
  }
  return cachedUint8ArrayMemory0;
}

function isLikeNone(val: unknown): boolean {
  return val === undefined || val === null;
}

function logError<T>(f: (...args: unknown[]) => T, args: IArguments): T {
  try {
    return f.apply(null, args as unknown[]);
  } catch (e) {
    let errorMsg = (function () {
      try {
        return e instanceof Error ? `${e.message}\n\nStack:\n${e.stack}` : String(e);
      } catch(_) {
        return '<failed to stringify thrown value>';
      }
    }());
    console.error('wasm-bindgen: imported JS function that was not marked as `catch` threw an error:', errorMsg);
    throw e;
  }
}

function passArray8ToWasm0(arg: Uint8Array, malloc: (len: number) => number): number {
  const ptr = (malloc(arg.length * 1) >>> 0);
  getUint8ArrayMemory0().set(arg, ptr / 1);
  WASM_VECTOR_LEN = arg.length;
  return ptr;
}

function passStringToWasm0(arg: string, malloc: (len: number) => number, realloc: ((ptr: number, len: number) => number) | undefined): number {
  if (typeof arg !== 'string') throw new Error(`expected a string argument, found ${typeof arg}`);
  if (realloc === undefined) {
    const buf = cachedTextEncoder.encode(arg);
    const ptr = malloc(buf.length) >>> 0;
    getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
    WASM_VECTOR_LEN = buf.length;
    return ptr;
  }
  let len = cachedTextEncoder.encode(arg).length;
  let ptr = -1;
  if (realloc !== undefined) {
    ptr = realloc(0, len);
  }
  if (ptr === -1) {
    throw new Error('Could not allocate memory');
  }
  getUint8ArrayMemory0().subarray(ptr, ptr + len).set(cachedTextEncoder.encode(arg));
  WASM_VECTOR_LEN = len;
  return ptr;
}

function takeFromExternrefTable0(idx: number): unknown {
  if (idx === 0) return undefined;
  const val = (wasm.__wbindgen_externref_table as WebAssembly.Table).get(idx);
  (wasm.__wbindgen_externref_table as WebAssembly.Table).set(idx, undefined);
  return val;
}

function __wbg_Error_fdd633d4bb5dd76a(arg0: number, arg1: number): Error {
  return logError(function() {
    const ret = Error(getStringFromWasm0(arg0, arg1));
    return ret;
  }, arguments);
}

function __wbg_String_8564e559799eccda(arg0: number, arg1: number): string {
  return logError(function() {
    const ret = String(arg1);
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc as (len: number) => number, wasm.__wbindgen_realloc as ((ptr: number, len: number) => number) | undefined);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
  }, arguments) as unknown as string;
}

function __wbg___wbindgen_throw_ea4887a5f8f9a9db(arg0: number, arg1: number): never {
  throw new Error(getStringFromWasm0(arg0, arg1));
}

function __wbg_new_2e117a478906f062(): object {
  return logError(function() {
    return new Object();
  }, arguments) as object;
}

function __wbg_new_36e147a8ced3c6e0(): unknown[] {
  return logError(function() {
    return new Array();
  }, arguments) as unknown[];
}

function __wbg_new_from_slice_543b875b27789a8f(arg0: number, arg1: number): Uint8Array {
  return logError(function() {
    return new Uint8Array(getArrayU8FromWasm0(arg0, arg1));
  }, arguments) as Uint8Array;
}

function __wbg_set_6be42768c690e380(arg0: number, arg1: number, arg2: number): void {
  (arg0 as unknown[])[arg1] = arg2;
}

function __wbg_set_dc601f4a69da0bc2(arg0: number, arg1: number, arg2: number): void {
  (arg0 as Uint8Array)[arg1 >>> 0] = arg2;
}

function __wbindgen_cast_0000000000000001(arg0: number): number {
  return logError(function() {
    return arg0;
  }, arguments) as number;
}

function __wbindgen_cast_0000000000000002(arg0: number, arg1: number): string {
  return logError(function() {
    return getStringFromWasm0(arg0, arg1);
  }, arguments) as string;
}

function __wbindgen_cast_0000000000000003(arg0: number): bigint {
  return logError(function() {
    return BigInt.asUintN(64, BigInt(arg0));
  }, arguments) as bigint;
}

function __wbindgen_init_externref_table(): void {
  const table = wasm.__wbindgen_externref_table as WebAssembly.Table;
  const offset = table.grow(4);
  table.set(0, undefined);
  table.set(offset + 0, undefined);
  table.set(offset + 1, null);
  table.set(offset + 2, true);
  table.set(offset + 3, false);
}

function __wbg_WasmError_8457352905a93c7f(ptr: number): unknown {
  return wasm.__wbg_wasmerror_8457352905a93c7f(ptr);
}

// ============================================================================
// WASM Loading
// ============================================================================

export async function initWasm(): Promise<DocEngineExports> {
  if (wasmModule) return wasmModule;
  if (wasmLoading) return wasmLoading;
  wasmLoading = loadWasmModule();
  wasmModule = await wasmLoading;
  wasmReady = true;
  return wasmModule;
}

async function loadWasmModule(): Promise<DocEngineExports> {
  const wasmUrl = browser.runtime.getURL('/wasm/doc_engine_bg.wasm');

  try {
    // Reset caches
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;

    // Fetch WASM binary
    const wasmResponse = await fetch(wasmUrl);
    if (!wasmResponse.ok) {
      throw new Error(`Failed to fetch WASM binary: ${wasmResponse.status}`);
    }
    const wasmBinary = await wasmResponse.arrayBuffer();

    // Define imports required by wasm-bindgen generated code
    const importObject: WebAssembly.Imports = {
      './doc_engine_bg.js': {
        __wbg_Error_fdd633d4bb5dd76a,
        __wbg_String_8564e559799eccda,
        __wbg___wbindgen_throw_ea4887a5f8f9a9db,
        __wbg_new_2e117a478906f062,
        __wbg_new_36e147a8ced3c6e0,
        __wbg_new_from_slice_543b875b27789a8f,
        __wbg_set_6be42768c690e380,
        __wbg_set_dc601f4a69da0bc2,
        __wbindgen_cast_0000000000000001,
        __wbindgen_cast_0000000000000002,
        __wbindgen_cast_0000000000000003,
        __wbindgen_init_externref_table,
      },
      env: {
        abort: (msg: number, filename: number, line: number, column: number) => {
          console.error(`[WASM] Abort: ${getStringFromWasm0(msg, 20)} at ${filename}:${line}:${column}`);
        },
      },
    };

    // Instantiate WASM
    const result = await WebAssembly.instantiate(wasmBinary, importObject);
    const exports = result.instance.exports;

    // Store WASM reference
    for (const [key, value] of Object.entries(exports)) {
      (wasm as Record<string, unknown>)[key] = value;
    }

    console.log('[WASM] Module instantiated successfully');

    // Get function references
    const convert_zip_fn = exports.convert_zip as Function;
    const convert_zip_to_docx_fn = exports.convert_zip_to_docx as Function;
    const version_fn = exports.version as Function;

    // Get memory for callbacks
    const memory = exports.memory as WebAssembly.Memory;

    console.log('[WASM] Version:', getStringFromWasm0(version_fn() as number, 20));

    return {
      version: () => {
        try {
          const ret = version_fn() as number;
          const deferred0 = getDataViewMemory0().getUint32(ret, true);
          const deferred1 = getDataViewMemory0().getUint32(ret + 4, true);
          const s = getStringFromWasm0(deferred0, deferred1);
          (wasm.__wbindgen_free as Function)(deferred0, deferred1, 1);
          return s;
        } catch {
          return 'unknown';
        }
      },

      convert_zip: (zipBytes, mainTexPath, options) => {
        const ptr0 = passArray8ToWasm0(zipBytes, wasm.__wbindgen_malloc as (len: number) => number);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(mainTexPath, wasm.__wbindgen_malloc as (len: number) => number, wasm.__wbindgen_realloc as ((ptr: number, len: number) => number) | undefined);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = isLikeNone(options) ? 0 : passStringToWasm0(options, wasm.__wbindgen_malloc as (len: number) => number, wasm.__wbindgen_realloc as ((ptr: number, len: number) => number) | undefined);
        const len2 = WASM_VECTOR_LEN;

        try {
          const ret = convert_zip_fn(ptr0, len0, ptr1, len1, ptr2, len2);

          if (ret[2]) {
            const error = takeFromExternrefTable0(ret[1]);
            throw error;
          }

          return takeFromExternrefTable0(ret[0]) as WasmConvertResult;
        } finally {
          // Free inputs
          (wasm.__wbindgen_free as Function)(ptr0, zipBytes.length * 1);
        }
      },

      convert_zip_to_docx: (zipBytes, mainTexPath, options) => {
        const ptr0 = passArray8ToWasm0(zipBytes, wasm.__wbindgen_malloc as (len: number) => number);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(mainTexPath, wasm.__wbindgen_malloc as (len: number) => number, wasm.__wbindgen_realloc as ((ptr: number, len: number) => number) | undefined);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = isLikeNone(options) ? 0 : passStringToWasm0(options, wasm.__wbindgen_malloc as (len: number) => number, wasm.__wbindgen_realloc as ((ptr: number, len: number) => number) | undefined);
        const len2 = WASM_VECTOR_LEN;

        try {
          const ret = convert_zip_to_docx_fn(ptr0, len0, ptr1, len1, ptr2, len2);

          if (ret[2]) {
            const error = takeFromExternrefTable0(ret[1]);
            throw error;
          }

          return takeFromExternrefTable0(ret[0]) as Uint8Array;
        } finally {
          (wasm.__wbindgen_free as Function)(ptr0, zipBytes.length * 1);
        }
      },
    };
  } catch (error) {
    console.error('[WASM] Failed to load module:', error);
    throw new Error(`Failed to load WASM engine: ${error instanceof Error ? error.message : 'Unknown error'}`);
  }
}

export function isWasmReady(): boolean {
  return wasmReady;
}

export async function getWasmVersion(): Promise<string> {
  const module = await initWasm();
  return module.version();
}

export async function convertZipToDocx(
  zipBytes: Uint8Array,
  mainTexPath: string,
  options?: WasmConvertOptions
): Promise<WasmConvertResult> {
  const module = await initWasm();
  const optionsJson = options ? JSON.stringify(options) : '{}';
  return module.convert_zip(zipBytes, mainTexPath, optionsJson);
}

export async function convertZipToDocxBytes(
  zipBytes: Uint8Array,
  mainTexPath: string,
  options?: WasmConvertOptions
): Promise<Uint8Array> {
  const module = await initWasm();
  const optionsJson = options ? JSON.stringify(options) : '{}';
  return module.convert_zip_to_docx(zipBytes, mainTexPath, optionsJson);
}

export function validateDocx(docxBytes: Uint8Array): boolean {
  // DOCX is a ZIP file, check ZIP magic bytes
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
