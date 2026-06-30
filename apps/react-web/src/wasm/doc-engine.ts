export interface LocalConvertOptions {
  mainTex: string;
  profile: string;
  quality: string;
}

type WasmModule = {
  default?: () => Promise<void>;
  convert_zip_to_docx?: (zip: Uint8Array, mainTex: string, optionsJson?: string | null) => Uint8Array;
  convertZipToDocx?: (zip: Uint8Array, mainTex: string, optionsJson?: string | null) => Uint8Array;
};

export async function convertZipToDocx(file: File, options: LocalConvertOptions): Promise<Blob> {
  let wasm: WasmModule;
  try {
    const wasmPath = './pkg/doc_engine.js';
    wasm = (await import(/* @vite-ignore */ wasmPath)) as WasmModule;
  } catch (error) {
    const reason = error instanceof Error ? ` ${error.message}` : '';
    throw new Error(`React WASM 产物尚未构建，请先运行 npm run build:wasm:react 或提供 apps/react-web/src/wasm/pkg/doc_engine.js。${reason}`);
  }

  if (wasm.default) {
    await wasm.default();
  }

  const bytes = new Uint8Array(await file.arrayBuffer());
  const convert = wasm.convert_zip_to_docx ?? wasm.convertZipToDocx;
  if (!convert) {
    throw new Error('WASM 模块未导出 convert_zip_to_docx。');
  }
  const optionsJson = JSON.stringify({
    bib_style: 'numeric',
  });
  const docx = convert(bytes, options.mainTex, optionsJson);
  const copy = new Uint8Array(docx.byteLength);
  copy.set(docx);
  return new Blob([copy], {
    type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
  });
}
