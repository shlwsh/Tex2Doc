#!/usr/bin/env node
/**
 * Chrome MV3 后处理脚本
 * 目的：修复 Chrome MV3 CSP，同时保留 Chrome/Edge Side Panel。
 *
 * 原因：WXT 默认 CSP 可能包含 Chrome MV3 不接受的 `blob:`。
 */

import fs from 'fs';
import path from 'path';

const outDir = path.join(process.cwd(), '.output', 'chrome-mv3');
const manifestPath = path.join(outDir, 'manifest.json');

if (!fs.existsSync(manifestPath)) {
  console.error(`[fix-chrome-manifest] 未找到 ${manifestPath}`);
  process.exit(1);
}

const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf-8'));

// 1. 强制设置安全的 CSP（去掉所有 blob: 值）
manifest.content_security_policy = {
  extension_pages:
    "script-src 'self' 'wasm-unsafe-eval'; worker-src 'self'; object-src 'self'",
};

// 2. service worker 必须声明为 module，否则无法用 dynamic import 加载
//    wasm-bindgen 生成的 ESM 胶水（doc_engine.js）。
if (manifest.background?.service_worker) {
  manifest.background.type = 'module';
}

fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));
console.log('[fix-chrome-manifest] Chrome manifest 已清理：');
console.log('  - 保留 side_panel 字段与 sidePanel 权限');
console.log('  - CSP 已修复（无 blob:）');
console.log('  - service_worker 设为 module（支持 ESM dynamic import）');
