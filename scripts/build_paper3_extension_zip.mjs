#!/usr/bin/env node
/**
 * 把 examples/paper3 打包成 D:\temp\upload.zip，
 * 方便用户直接在 Chrome 扩展里上传做真实环境回归测试。
 *
 * 用法：
 *   node scripts/build_paper3_extension_zip.mjs
 *
 * 输出：
 *   D:\temp\upload.zip  （~6 MB，含主 tex + 多个 include + bib + 10 张 PNG）
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { spawnSync } from 'child_process';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, '..');

const latexDir = path.join(repoRoot, 'examples', 'paper3', 'latex');
const figuresDir = path.join(repoRoot, 'examples', 'paper3', 'figures');
const outZip = path.join('D:', 'temp', 'upload.zip');

fs.mkdirSync(path.dirname(outZip), { recursive: true });

function walk(root) {
  const out = [];
  const stack = [root];
  while (stack.length) {
    const cur = stack.pop();
    let rd;
    try { rd = fs.readdirSync(cur, { withFileTypes: true }); } catch { continue; }
    for (const e of rd) {
      const p = path.join(cur, e.name);
      if (e.isDirectory()) stack.push(p);
      else out.push(p);
    }
  }
  return out;
}

if (!fs.existsSync(latexDir)) {
  console.error(`[error] 未找到 latex 目录：${latexDir}`);
  process.exit(1);
}

console.log(`📦 打包 paper3 zip: ${outZip}`);

// 收集所有源文件
const files = [];
for (const p of walk(latexDir)) {
  files.push({ abs: p, rel: path.relative(latexDir, p).replaceAll('\\', '/') });
}
if (fs.existsSync(figuresDir)) {
  for (const p of walk(figuresDir)) {
    const name = path.basename(p);
    if (!name.toLowerCase().endsWith('.png')) continue;
    files.push({ abs: p, rel: name });
  }
}

console.log(`  - ${files.length} 个条目（${latexDir} + ${figuresDir}）`);

// 优先用 PowerShell 的 Compress-Archive（系统自带、零依赖、Chrome 接受）
if (process.platform === 'win32') {
  // 先把所有文件铺到一个临时目录，用相同的相对路径，然后 Compress-Archive
  const stagingDir = path.join(repoRoot, 'target', 'paper3-zip-staging');
  fs.rmSync(stagingDir, { recursive: true, force: true });
  fs.mkdirSync(stagingDir, { recursive: true });
  for (const f of files) {
    const dst = path.join(stagingDir, f.rel);
    fs.mkdirSync(path.dirname(dst), { recursive: true });
    fs.copyFileSync(f.abs, dst);
  }
  fs.rmSync(outZip, { force: true });
  const res = spawnSync('powershell', [
    '-NoProfile', '-Command',
    `Compress-Archive -Path "${stagingDir}\\*" -DestinationPath "${outZip}" -Force`,
  ], { stdio: 'inherit' });
  fs.rmSync(stagingDir, { recursive: true, force: true });
  if (res.status !== 0) {
    console.error('[error] Compress-Archive 失败');
    process.exit(res.status || 1);
  }
} else {
  console.error('[error] 此脚本仅支持 Windows（PowerShell Compress-Archive）');
  process.exit(1);
}

const stat = fs.statSync(outZip);
console.log(`✅ 已生成 ${outZip} (${(stat.size / 1024 / 1024).toFixed(2)} MiB)`);
console.log(`\n下一步：在 Chrome 中打开扩展 popup → 上传 ${outZip}`);
console.log(`       期望产出 main-jos.docx，含 10 张图片 + 110 段 + 6 列表 + 4 公式 + 11 表格 + 50 标题。`);
