/* tslint:disable */
/* eslint-disable */

/**
 * JS 可见的友好错误：把 `CoreError` 平铺到字符串。
 */
export class WasmError {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    readonly message: string;
}

/**
 * 同步转换入口：把 zip 字节流转换为 docx 字节流。
 *
 * - `zip_bytes`: 完整项目 zip 字节（包含主 .tex、include 的 .tex、.bib、图片等）
 * - `main_tex_path`: zip 内主 .tex 的相对 POSIX 路径（例：`main-jos.tex`）
 * - `options_js`: 可选 JSON 字符串（V1 仅消费 `bib_style`）
 *
 * 成功返回 `ConvertResultJs { docx, docx_len, warnings }`。
 * 失败抛出 `WasmError`。
 */
export function convert_zip(zip_bytes: Uint8Array, main_tex_path: string, options_js?: string | null): any;

/**
 * 便捷入口：返回 docx 的 `Uint8Array`（不附带元信息）。
 *
 * 适合前端只关心文件内容的场景；`download` API 需要 `Uint8Array` / `Blob`。
 */
export function convert_zip_to_docx(zip_bytes: Uint8Array, main_tex_path: string, options_js?: string | null): Uint8Array;

/**
 * 版本号（编译期常量）。
 */
export function version(): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmerror_free: (a: number, b: number) => void;
    readonly convert_zip: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly convert_zip_to_docx: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly version: (a: number) => void;
    readonly wasmerror_message: (a: number, b: number) => void;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
