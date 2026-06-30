#!/usr/bin/env node
/**
 * post-build-wasm.mjs
 *
 * 在 `npm run build:wasm:extension` 之后执行。
 *
 * 目的：把 wasm-bindgen 生成的 ESM 胶水（doc_engine.js）转成可以在
 * Chrome MV3 service worker 里直接静态 import 的脚本（去掉所有 export，
 * 用 globalThis 暴露 API），并复制一份到 src/workers/wasm-glue/ 让
 * wasm-worker.ts 通过静态 import 直接内联进 bundle。
 *
 * 为什么必须这么做：
 * - Chrome MV3 service worker 禁止 `import()`（HTML spec 限制）；
 *   禁止 `eval()` / `new Function()`（CSP 不允许 unsafe-eval）；
 *   禁止 dynamic import 的 fallback 方案。
 * - 静态 import 是唯一可行的加载方式：Vite/Rolldown 会把 doc_engine.js
 *   的代码直接合并进 wasm-worker 模块的 bundle，从而绕过 SW 的 import 限制。
 *
 * 改写规则（必须在拷贝到 src/ 之前完成，否则胶水里残留 export/import.meta）：
 *   1. 顶层 `export { ... };` → 构造 globalThis.__tex2docWbg = { ... }
 *   2. 所有 `export class X` / `export function X` → 去掉 `export ` 关键字
 *      （保留函数/类定义即可，外部通过 globalThis 访问）
 *   3. 所有 `import.meta.url` → `globalThis.__tex2docBaseUrl`
 *   4. 在末尾追加 `globalThis.__tex2docApi = { convert_zip, ... }`
 *   5. 用 IIFE 包裹：`(function(){ ... }).call(globalThis);`
 *
 * 用法：
 *   node scripts/post-build-wasm.mjs [public-wasm-dir]
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const REPO_ROOT = path.resolve(__dirname, '..');

const argDir = process.argv[2];
const PUBLIC_WASM = argDir
  ? path.resolve(REPO_ROOT, argDir)
  : path.resolve(REPO_ROOT, 'apps/browser-extension/public/wasm');
const GLUE_JS = path.join(PUBLIC_WASM, 'doc_engine.js');

if (!fs.existsSync(GLUE_JS)) {
  console.error(`[post-build-wasm] 未找到 ${GLUE_JS}，跳过（可能没构建 wasm）`);
  process.exit(0);
}

let src = fs.readFileSync(GLUE_JS, 'utf8');

// 1) 去掉 `export class` / `export function` / `export const` / `export let` / `export var` 前缀
src = src.replace(/^export\s+(class|function|const|let|var|async function)\s+/gm, '$1 ');

// 2) 顶层 `export { ... };` → 构造 globalThis.__tex2docWbg = { ... }
src = src.replace(
  /export\s*\{\s*([^}]+)\s*\}\s*;?\s*$/m,
  (_match, names) => {
    const parts = names
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean)
      .map((part) => {
        const [orig, alias] = part.split(/\s+as\s+/).map((s) => s.trim());
        return alias ? `      ${alias}: ${orig}` : `      ${orig}: ${orig}`;
      });
    return `globalThis.__tex2docWbg = {\n${parts.join(',\n')},\n    };`;
  },
);

// 3) `import.meta.url` → `globalThis.__tex2docBaseUrl`
src = src.replaceAll('import.meta.url', 'globalThis.__tex2docBaseUrl');

// 4) 收集所有原本是 export function 的名字（已经去掉 export），挂到 globalThis.__tex2docApi
const exportedFns = [...src.matchAll(/^function\s+(\w+)\s*\(/gm)]
  .map((m) => m[1])
  .filter((n) => n !== '__wbg_init' && !n.startsWith('__')); // 排除内部 helper

if (exportedFns.length === 0) {
  console.error('[post-build-wasm] 警告：未找到任何 export function，可能胶水格式变了');
}

const apiExports = exportedFns
  .map((n) => `      ${n}: typeof ${n} === 'function' ? ${n} : undefined`)
  .join(',\n');

// 5) IIFE 包裹
const iife = `(function(){\n${src}\n    globalThis.__tex2docApi = {\n${apiExports},\n    };\n}).call(globalThis);\n`;

fs.writeFileSync(GLUE_JS, iife, 'utf8');

// 6) 复制到 src/workers/wasm-glue/ 供静态 import
const GLUE_COPY_DIR = path.resolve(__dirname, '..', 'apps/browser-extension/src/workers/wasm-glue');
const GLUE_COPY = path.join(GLUE_COPY_DIR, 'doc_engine.js');
fs.mkdirSync(GLUE_COPY_DIR, { recursive: true });
fs.copyFileSync(GLUE_JS, GLUE_COPY);
console.log(`[post-build-wasm] 已复制到 ${GLUE_COPY}（供 wasm-worker.ts 静态 import）`);

console.log(`[post-build-wasm] 已将 ${GLUE_JS} 转成 IIFE + globalThis 形式`);
console.log(`[post-build-wasm]   - export { ... } → globalThis.__tex2docWbg = { ... }`);
console.log(`[post-build-wasm]   - import.meta.url → globalThis.__tex2docBaseUrl`);
console.log(`[post-build-wasm]   - 提升 API 到 globalThis.__tex2docApi: ${exportedFns.join(', ')}`);