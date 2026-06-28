#!/usr/bin/env node
/**
 * Chrome MV3 后处理脚本
 * 目的：彻底修复 Chrome 不接受的 CSP 和字段。
 *
 * 原因：WXT 会基于 entrypoints 自动注入 `side_panel` 与 `sidePanel` 权限，
 * 并默认在 CSP 中允许 `blob:`。Chrome MV3 明确禁止这两者。
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

// 1. 移除 Edge 专属字段
delete manifest.side_panel;

// 2. 移除 Chrome 不支持的权限
if (Array.isArray(manifest.permissions)) {
  manifest.permissions = manifest.permissions.filter((p) => p !== 'sidePanel');
}

// 3. 强制设置安全的 CSP（去掉所有 blob: 值）
manifest.content_security_policy = {
  extension_pages:
    "script-src 'self' 'wasm-unsafe-eval'; worker-src 'self'; object-src 'self'",
};

// 4. service worker 必须声明为 module，否则无法用 dynamic import 加载
//    wasm-bindgen 生成的 ESM 胶水（doc_engine.js）。
if (manifest.background?.service_worker) {
  manifest.background.type = 'module';
}

fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));
console.log('[fix-chrome-manifest] Chrome manifest 已清理：');
console.log('  - 移除 side_panel 字段');
console.log('  - 移除 sidePanel 权限');
console.log('  - CSP 已修复（无 blob:）');
console.log('  - service_worker 设为 module（支持 ESM dynamic import）');
