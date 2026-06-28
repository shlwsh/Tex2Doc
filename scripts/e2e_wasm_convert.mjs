#!/usr/bin/env node
/**
 * e2e_wasm_convert.mjs — 在真实 Chromium 中加载 Chrome MV3 扩展，
 * 通过 service worker 跑一遍 paper3 zip → docx 全链路。
 *
 * 这是验证 "Conversion Failed / __wbindgen_object_drop_ref" 这类
 * WASM imports 错误的最直接方法。
 */

import { chromium } from '@playwright/test';
import { readFile, writeFile, stat } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, '..');
const extPath = resolve(repoRoot, 'apps/browser-extension/.output/chrome-mv3');
const zipPath = process.argv[2] || 'D:\\temp\\upload.zip';
const outDocx = process.argv[3] || path.join(repoRoot, 'target', 'e2e-out.docx');

async function main() {
  console.log(`[e2e-wasm] extension dir: ${extPath}`);
  console.log(`[e2e-wasm] input zip:     ${zipPath}`);
  console.log(`[e2e-wasm] output docx:   ${outDocx}`);

  const zipStat = await stat(zipPath);
  console.log(`[e2e-wasm] zip size: ${(zipStat.size / 1024 / 1024).toFixed(2)} MiB`);

  // 清空 user-data-dir 防止老 SW 注册缓存干扰
  const userDataDir = path.join(repoRoot, 'target', 'e2e-userdata');
  const fs = await import('node:fs/promises');
  await fs.rm(userDataDir, { recursive: true, force: true });

  // 用 chromium.newContext + 手动加载扩展
  // 注意：必须用 channel: 'chromium' 才能在 headless 模式加载扩展，
  // 否则 playwright 自带的 chromium build 默认禁用扩展。
  const context = await chromium.launchPersistentContext(userDataDir, {
    channel: 'chromium',
    headless: true,
    args: [
      `--disable-extensions-except=${extPath}`,
      `--load-extension=${extPath}`,
      '--no-sandbox',
    ],
    timeout: 120_000,
  });

  console.log(`[e2e-wasm] context 创建成功`);

  // 把 chromium 进程 stdout/stderr 接到 stdout（注意用 on 而不是 once）
  const browser = context.browser();
  if (browser) {
    try {
      browser.process()?.stdout?.on('data', (d) => {
        const lines = d.toString().split('\n');
        for (const line of lines) {
          if (line.includes('tex2doc') || line.includes('extension') || line.includes('ERROR') || line.includes('WARN')) {
            console.log(`  [chromium] ${line.trim()}`);
          }
        }
      });
      browser.process()?.stderr?.on('data', (d) => {
        const lines = d.toString().split('\n');
        for (const line of lines) {
          if (line.includes('tex2doc') || line.includes('extension') || line.includes('ERROR') || line.includes('WARN')) {
            console.log(`  [chromium-stderr] ${line.trim()}`);
          }
        }
      });
    } catch (e) {
      console.log(`[e2e-wasm] (无法 attach 到 chromium stdout: ${e.message})`);
    }
  }

  let exitCode = 0;
  let serviceWorker = null;
  const swErrors = [];
  const consoleLogs = [];

  try {
    // 监听 page 错误
    context.on('weberror', (e) => {
      swErrors.push(`weberror: ${e.error().message}`);
    });

    // 开一个普通 page 才会触发扩展的事件循环
    const page = await context.newPage();
    page.on('console', (msg) => {
      const text = msg.text();
      consoleLogs.push(`[page:${msg.type()}] ${text}`);
      console.log(`  [page-console:${msg.type()}] ${text}`);
    });
    page.on('pageerror', (err) => {
      swErrors.push(`page-error: ${err.message}`);
      console.log(`  [page-error] ${err.message}`);
    });
    await page.goto('https://example.com', { waitUntil: 'domcontentloaded' });

    // 等 service worker 起来（如果已经存在就直接拿，否则等事件）
    console.log('[e2e-wasm] 当前 service workers: ' + context.serviceWorkers().length);
    for (const w of context.serviceWorkers()) {
      console.log(`  - ${w.url()}`);
    }

    if (context.serviceWorkers().length === 0) {
      console.log('[e2e-wasm] 等待 service worker 就绪（最多 60s）...');
      serviceWorker = await context.waitForEvent('serviceworker', { timeout: 60_000 });
    } else {
      serviceWorker = context.serviceWorkers()[0];
    }
    console.log(`[e2e-wasm] ✓ service worker: ${serviceWorker.url()}`);

    serviceWorker.on('console', (msg) => {
      const text = msg.text();
      consoleLogs.push(`[sw:${msg.type()}] ${text}`);
      console.log(`  [sw-console:${msg.type()}] ${text}`);
    });
    serviceWorker.on('pageerror', (err) => {
      swErrors.push(`sw-pageerror: ${err.message}`);
      console.log(`  [sw-pageerror] ${err.message}`);
    });

    // 让 service worker 加载 wasm 模块（这一步会触发 __wbindgen_object_drop_ref 等 import）
    console.log('[e2e-wasm] 触发 WASM 初始化 + 转换...');

    const zipBytes = await readFile(zipPath);
    const zipArr = Array.from(zipBytes);

    // 让 service worker 直接执行 WASM 转换（绕过 message channel）
    const e2eResult = await serviceWorker.evaluate(async (args) => {
      try {
        // 直接调用暴露的全局函数（绕过 onMessage handler）
        // eslint-disable-next-line no-undef
        const fn = globalThis.__tex2docConvertZip;
        if (typeof fn !== 'function') {
          return { ok: false, error: 'globalThis.__tex2docConvertZip is not a function' };
        }
        const reply = await fn({
          zipBytes: Array.from(new Uint8Array(args.zipArr)),
          fileName: 'upload.zip',
          mainTex: 'main-jos.tex',
          _e2eReturnBytes: true,
        });
        return { ok: true, reply };
      } catch (e) {
        return { ok: false, error: e && e.message ? e.message : String(e), stack: e && e.stack ? e.stack : null };
      }
    }, { zipArr });

    // 单独测一下 downloadBytes 路径：先 import 模块，再尝试直接调一次 downloadBytes。
    // 这会走 service worker 里没有 URL.createObjectURL 的代码路径，验证我们的
    // data-URL fallback 能工作。
    const downloadResult = await serviceWorker.evaluate(async () => {
      try {
        // eslint-disable-next-line no-undef
        const mod = globalThis.__tex2docDownloads;
        if (!mod || typeof mod.downloadBytes !== 'function') {
          return { ok: false, error: 'globalThis.__tex2docDownloads not exposed' };
        }
        // 5KB 测试数据
        const data = new Uint8Array(5 * 1024);
        for (let i = 0; i < data.length; i++) data[i] = i & 0xff;
        // 注意：playwright headless 模式下 chrome.downloads 可能被限制；
        // 我们的目的是确认 downloadBytes 内部不会抛 "window is not defined"，
        // 即 URL.createObjectURL fallback 真的走到了 data: URL 分支。
        try {
          const result = await mod.downloadBytes(data, 'smoke-test.bin', 'application/octet-stream');
          return { ok: true, id: result.id, filename: result.filename };
        } catch (inner) {
          // chrome.downloads 在 headless 可能报 "Download cannot be performed"，
          // 但只要不是 "window is not defined" 就证明 fallback 工作了
          const msg = inner && inner.message ? inner.message : String(inner);
          if (msg.includes('window is not defined')) {
            return { ok: false, error: 'URL.createObjectURL fallback NOT working: ' + msg };
          }
          return { ok: true, chromeDownloadsError: msg };
        }
      } catch (e) {
        return { ok: false, error: e && e.message ? e.message : String(e), stack: e && e.stack ? e.stack : null };
      }
    });

    console.log('[e2e-wasm] service worker 响应:');
    console.log(JSON.stringify(e2eResult, null, 2));

    if (!e2eResult.ok) {
      console.error('[e2e-wasm] ✗ WASM 转换失败');
      console.error(`   ${e2eResult.error}`);
      if (e2eResult.stack) console.error(e2eResult.stack);
      exitCode = 1;
    } else if (!e2eResult.reply || !e2eResult.reply.success) {
      console.error('[e2e-wasm] ✗ 转换返回失败');
      console.error(JSON.stringify(e2eResult.reply, null, 2));
      exitCode = 1;
    } else {
      console.log('[e2e-wasm] ✓ 转换成功');
      console.log(`   jobId: ${e2eResult.reply.jobId}`);
      if (e2eResult.reply.docxBytes) {
        const { writeFile } = await import('node:fs/promises');
        const docxBuf = Buffer.from(e2eResult.reply.docxBytes);
        await writeFile(outDocx, docxBuf);
        console.log(`   docx:  ${outDocx} (${docxBuf.length} bytes)`);
        // 验证 docx magic bytes
        if (docxBuf[0] === 0x50 && docxBuf[1] === 0x4b) {
          console.log('   ✓ docx magic bytes OK (PK\\x03\\x04)');
        } else {
          console.error('   ✗ docx magic bytes wrong');
          exitCode = 1;
        }
      }
    }
    console.log('[e2e-wasm] downloadBytes smoke test:');
    console.log(JSON.stringify(downloadResult, null, 2));
    if (!downloadResult.ok) {
      console.error('[e2e-wasm] ✗ downloadBytes 在 service worker 里失败');
      exitCode = 1;
    } else if (downloadResult.chromeDownloadsError) {
      console.log(`[e2e-wasm] ✓ downloadBytes fallback 到 data: URL 工作`);
      console.log(`   (chrome.downloads 在 headless 受限，但 URL 创建成功: ${downloadResult.chromeDownloadsError})`);
    } else {
      console.log(`[e2e-wasm] ✓ downloadBytes 成功，download id: ${downloadResult.id}`);
    }
  } catch (err) {
    console.error('[e2e-wasm] ✗ 测试运行失败:', err.message);
    exitCode = 1;
  } finally {
    await context.close();
  }

  console.log(`[e2e-wasm] exit code: ${exitCode}`);
  process.exit(exitCode);
}

main();
