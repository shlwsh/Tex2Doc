export interface WasmConvertOptions {
  bib_style?: 'numeric' | 'author-year';
}

export interface WasmConvertResult {
  docx: Uint8Array;
  docx_len: number;
  warnings: string[];
}

interface DocEngineModule {
  convert_zip: (zipBytes: Uint8Array, mainTexPath: string, options: string) => WasmConvertResult;
  convert_zip_to_docx: (zipBytes: Uint8Array, mainTexPath: string, options: string) => Uint8Array;
  version: () => string;
}

let wasmModule: DocEngineModule | null = null;
let wasmLoading: Promise<DocEngineModule> | null = null;
let wasmReady = false;

export async function initWasm(): Promise<DocEngineModule> {
  if (wasmModule) return wasmModule;
  if (wasmLoading) return wasmLoading;
  wasmLoading = loadWasmModule();
  wasmModule = await wasmLoading;
  wasmReady = true;
  return wasmModule;
}

async function loadWasmModule(): Promise<DocEngineModule> {
  const wasmUrl = browser.runtime.getURL('/wasm/doc_engine_bg.wasm');
  const jsUrl = browser.runtime.getURL('/wasm/doc_engine.js');

  try {
    const jsResponse = await fetch(jsUrl);
    if (!jsResponse.ok) throw new Error(`Failed to fetch WASM JS: ${jsResponse.status}`);
    const jsCode = await jsResponse.text();
    return await createWasmModule(wasmUrl, jsCode);
  } catch (error) {
    console.error('[WASM] Failed to load module:', error);
    throw new Error(`Failed to load WASM engine: ${error instanceof Error ? error.message : 'Unknown error'}`);
  }
}

async function createWasmModule(wasmUrl: string, jsCode: string): Promise<DocEngineModule> {
  const wasmResponse = await fetch(wasmUrl);
  if (!wasmResponse.ok) throw new Error(`Failed to fetch WASM binary: ${wasmResponse.status}`);
  const wasmBinary = await wasmResponse.arrayBuffer();

  const moduleFactory = new Function('wasmBinary', `
    let exports = {};
    let initWasm;
    ${jsCode.replace(/export\s+/g, '').replace(/import\s*\(/g, '// import(')}
    if (typeof init === 'function') return init({ wasmBinary });
    return exports;
  `);

  return await moduleFactory(wasmBinary) as DocEngineModule;
}

export function isWasmReady(): boolean {
  return wasmReady;
}

export async function getWasmVersion(): Promise<string> {
  const module = await initWasm();
  return module.version();
}

export async function convertZipToDocx(zipBytes: Uint8Array, mainTexPath: string, options?: WasmConvertOptions): Promise<WasmConvertResult> {
  const module = await initWasm();
  const optionsJson = options ? JSON.stringify(options) : '';
  try {
    return module.convert_zip(zipBytes, mainTexPath, optionsJson);
  } catch (error) {
    throw new Error(`WASM conversion failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
  }
}

export async function convertZipToDocxBytes(zipBytes: Uint8Array, mainTexPath: string, options?: WasmConvertOptions): Promise<Uint8Array> {
  const module = await initWasm();
  const optionsJson = options ? JSON.stringify(options) : '';
  try {
    return module.convert_zip_to_docx(zipBytes, mainTexPath, optionsJson);
  } catch (error) {
    throw new Error(`WASM conversion failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
  }
}

export function validateDocx(docxBytes: Uint8Array): boolean {
  return docxBytes.length >= 4096 && docxBytes[0] === 0x50 && docxBytes[1] === 0x4b;
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
