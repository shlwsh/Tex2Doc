#!/usr/bin/env node
/**
 * 构建 paper3 的项目 zip，用于 Playwright E2E 上传。
 *
 * 用法：
 *   node scripts/build_paper3_zip.mjs [输出路径]
 *   # 默认输出 examples/paper3/upload.zip
 */
import { readFile, writeFile, readdir, stat } from 'node:fs/promises';
import { resolve, join, dirname, basename, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { zipSync, strToU8 } from 'fflate';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '..');
const SRC_DIR = join(ROOT, 'examples', 'paper3', 'latex');
const DEFAULT_OUT = join(ROOT, 'examples', 'paper3', 'upload.zip');

const outPath = process.argv[2]
  ? resolve(process.argv[2])
  : DEFAULT_OUT;

async function listFilesRecursive(dir, base = dir) {
  const out = [];
  const entries = await readdir(dir, { withFileTypes: true });
  for (const e of entries) {
    const full = join(dir, e.name);
    if (e.isDirectory()) {
      // 跳过 build 输出目录
      if (e.name === 'output') continue;
      out.push(...(await listFilesRecursive(full, base)));
    } else if (e.isFile()) {
      const rel = relative(base, full).split(sep).join('/');
      out.push({ rel, full });
    }
  }
  return out;
}

async function main() {
  // 1) 收集 paper3/latex 下的所有 .tex / .bib / .cls 文件
  const files = await listFilesRecursive(SRC_DIR);
  const keep = files.filter(f => {
    const lower = f.rel.toLowerCase();
    return lower.endsWith('.tex') ||
      lower.endsWith('.bib') ||
      lower.endsWith('.cls') ||
      lower.endsWith('.bst') ||
      lower.endsWith('.sty');
  });

  if (keep.length === 0) {
    throw new Error(`未在 ${SRC_DIR} 找到 .tex/.bib 文件`);
  }

  // 2) 读所有文件 → 字节字典
  const entries = {};
  for (const f of keep) {
    entries[f.rel] = await readFile(f.full);
  }

  // 3) 打包
  const buf = zipSync(entries, { level: 6 });
  await writeFile(outPath, buf);

  console.log(`[paper3-zip] 写入 ${outPath}`);
  console.log(`[paper3-zip] 条目数 = ${Object.keys(entries).length}`);
  for (const name of Object.keys(entries).sort()) {
    console.log(`  - ${name} (${entries[name].length} bytes)`);
  }
}

main().catch((e) => {
  console.error(e.stack ?? e);
  process.exit(1);
});
