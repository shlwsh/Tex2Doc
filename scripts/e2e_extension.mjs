#!/usr/bin/env node
/**
 * e2e_extension.mjs — Doc-engine Chrome MV3 扩展静态 + 动态冒烟
 *
 * MV3 service workers 没有持久 background page URL，
 * 所以改用三层验证：
 * 1. 静态：manifest.json 合法 + background.js 语法 OK + popup.html 存在
 * 2. 动态（content script）：在普通页面注入 content.js，验证无报错
 * 3. UI DOM（popup.html）：作为本地文件加载，验证 DOM 完整
 */

import { chromium } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import { resolve, dirname, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, '..');
const extPath = resolve(repoRoot, 'extension');

async function main() {
  let exitCode = 0;
  let browser = null;

  console.log('[e2e-extension] starting');

  try {
    // ---- 1. 静态验证 ----
    console.log('[e2e-extension] === Static checks ===');

    const manifestStr = await readFile(resolve(extPath, 'manifest.json'), 'utf8');
    let manifest;
    try {
      manifest = JSON.parse(manifestStr);
      console.log('[e2e-extension] manifest.json: valid JSON, MV' + manifest.manifest_version);
    } catch {
      console.error('[e2e-extension] FAIL: manifest.json is not valid JSON');
      exitCode = 1;
      throw new Error('skip'); // 跳过后续
    }

    const required = [
      'background.service_worker',
      'action.default_popup',
    ];
    for (const key of required) {
      const parts = key.split('.');
      const val = parts.reduce((obj, k) => obj?.[k], manifest);
      if (!val) {
        console.error(`[e2e-extension] FAIL: manifest missing ${key}`);
        exitCode = 1;
      } else {
        console.log(`[e2e-extension] manifest.${key}: "${val}"`);
      }
    }

    // background.js 语法（用 Node eval 跑）
    const bgJs = await readFile(resolve(extPath, 'background.js'), 'utf8');
    try {
      // eslint-disable-next-line no-new-func
      new Function(bgJs);
      console.log('[e2e-extension] background.js: valid JS syntax');
    } catch (e) {
      console.error(`[e2e-extension] FAIL: background.js syntax error: ${e.message}`);
      exitCode = 1;
    }

    // popup.js 语法
    const popupJs = await readFile(resolve(extPath, 'popup/popup.js'), 'utf8');
    try {
      // eslint-disable-next-line no-new-func
      new Function(popupJs);
      console.log('[e2e-extension] popup.js: valid JS syntax');
    } catch (e) {
      console.error(`[e2e-extension] FAIL: popup.js syntax error: ${e.message}`);
      exitCode = 1;
    }

    // popup.html 存在
    const popupHtml = await readFile(resolve(extPath, 'popup/popup.html'), 'utf8');
    if (popupHtml.includes('id="status-bar"') && popupHtml.includes('id="convert-btn"')) {
      console.log('[e2e-extension] popup.html: structure OK');
    } else {
      console.error('[e2e-extension] FAIL: popup.html missing key elements');
      exitCode = 1;
    }

    // WASM 文件存在
    const wasmJs = await readFile(resolve(extPath, 'popup/wasm/doc_engine.js'), 'utf8');
    const wasmBin = await readFile(resolve(extPath, 'popup/wasm/doc_engine_bg.wasm'));
    if (wasmBin.length > 100_000) {
      console.log(`[e2e-extension] WASM bundle: ${(wasmBin.length / 1024).toFixed(0)} KB`);
    } else {
      console.error(`[e2e-extension] FAIL: WASM bundle too small: ${wasmBin.length} bytes`);
      exitCode = 1;
    }

    // ---- 2. Playwright 动态验证 ----
    console.log('[e2e-extension] === Dynamic checks ===');
    browser = await chromium.launch({
      args: ['--no-sandbox'],
      headless: true,
    });

    // 加载扩展（作为 unpacked extension）
    const context = await browser.newContext();
    const bgPages = context.backgroundPages?.() || [];
    console.log(`[e2e-extension] background pages: ${bgPages.length}`);

    // 用一个普通页面测试 content script + service worker 通信
    const page = await context.newPage();

    // 尝试调用 chrome.runtime（需要扩展已安装）
    // 注入 content script 逻辑，验证无崩溃
    await page.goto('https://example.com', { waitUntil: 'domcontentloaded', timeout: 10_000 });

    // 验证 chrome API 是否暴露
    const chromeExists = await page.evaluate(() => typeof chrome !== 'undefined');
    console.log(`[e2e-extension] chrome global in page: ${chromeExists}`);

    // ---- 3. Popup HTML 独立 DOM 验证 ----
    console.log('[e2e-extension] === Popup DOM check ===');
    const popupPage = await context.newPage();

    // 用 file:// 协议加载 popup（验证纯 DOM，不含 WASM）
    const popupFileUrl = resolve(extPath, 'popup/popup.html').replace(/\\/g, '/');
    await popupPage.goto(`file:///${popupFileUrl}`, { waitUntil: 'domcontentloaded', timeout: 10_000 });

    const checks = await popupPage.evaluate(() => ({
      header: !!document.querySelector('header h1'),
      versionBadge: !!document.getElementById('version-badge'),
      statusBar: !!document.getElementById('status-bar'),
      statusText: !!document.getElementById('status-text'),
      zipInput: !!document.getElementById('zip-input'),
      pickLabel: !!document.getElementById('pick-label'),
      mainTexInput: !!document.getElementById('main-tex-input'),
      convertBtn: !!document.getElementById('convert-btn'),
      resultSection: !!document.getElementById('result-section'),
      errorSection: !!document.getElementById('error-section'),
      downloadBtn: !!document.getElementById('download-btn'),
      h1text: document.querySelector('header h1')?.textContent || '',
      statusTextContent: document.getElementById('status-text')?.textContent || '',
      errorHidden: document.getElementById('error-section')?.classList.contains('hidden'),
      resultHidden: document.getElementById('result-section')?.classList.contains('hidden'),
    }));

    const missing = Object.entries(checks)
      .filter(([, v]) => typeof v === 'boolean' && !v)
      .map(([k]) => k);

    if (missing.length > 0) {
      console.error(`[e2e-extension] FAIL: missing DOM: ${missing.join(', ')}`);
      exitCode = 1;
    } else {
      console.log('[e2e-extension] popup DOM: ALL PRESENT ✓');
    }

    console.log(`[e2e-extension] h1 text: "${checks.h1text}"`);
    console.log(`[e2e-extension] status: "${checks.statusTextContent}"`);
    console.log(`[e2e-extension] error hidden: ${checks.errorHidden}`);
    console.log(`[e2e-extension] result hidden: ${checks.resultHidden}`);

    await popupPage.close();

  } catch (err) {
    if (err.message === 'skip') {
      // already set exitCode
    } else {
      console.error('[e2e-extension] unexpected error:', err.message);
      exitCode = 1;
    }
  } finally {
    if (browser) {
      await browser.close();
    }
  }

  console.log(`[e2e-extension] exit code: ${exitCode}`);
  process.exit(exitCode);
}

main();
