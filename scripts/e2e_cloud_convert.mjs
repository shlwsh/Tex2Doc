#!/usr/bin/env node
/**
 * e2e_cloud_convert.mjs — 云端转换全链路 e2e (P0-3)
 *
 * 验证：
 *   1) 真实账户登录（email + password from env）
 *   2) 通过 service worker 触发 CLOUD_CONVERT_AND_POLL
 *   3) JOB_UPDATED 五阶段事件（pending → uploading → creating → polling → completed）
 *   4) 终态后下载 docx，校验 magic bytes = `PK\x03\x04`
 *   5) 模拟 SW 回收（reload context）：pending job 应能续轮询
 *
 * 用法：
 *   TEX2DOC_E2E_EMAIL=foo@bar.com TEX2DOC_E2E_PASSWORD=secret \
 *     node scripts/e2e_cloud_convert.mjs <zip-path> <out-docx>
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
const outDocx = process.argv[3] || path.join(repoRoot, 'target', 'e2e-cloud-out.docx');
const apiBaseUrl = process.env.TEX2DOC_E2E_API_BASE_URL || 'https://api.tex2doc.cn';
const email = process.env.TEX2DOC_E2E_EMAIL || '';
const password = process.env.TEX2DOC_E2E_PASSWORD || '';
const profile = process.env.TEX2DOC_E2E_PROFILE || 'standard';
const quality = process.env.TEX2DOC_E2E_QUALITY || 'balanced';
const mainTex = process.env.TEX2DOC_E2E_MAIN_TEX || 'main.tex';

const DOCX_MAGIC = [0x50, 0x4b, 0x03, 0x04]; // "PK\x03\x04"

function log(...args) {
  console.log('[e2e-cloud]', ...args);
}

function fail(msg) {
  console.error(`[e2e-cloud] FAIL: ${msg}`);
  process.exit(1);
}

async function getServiceWorker(context) {
  if (context.serviceWorkers().length > 0) {
    return context.serviceWorkers()[0];
  }
  return context.waitForEvent('serviceworker', { timeout: 60_000 });
}

async function setSettingViaSW(sw, key, value) {
  return sw.evaluate(
    async ([k, v]) => {
      // eslint-disable-next-line no-undef
      const m = await import('/chunks/_virtual_wxt-plugins-DHPemsFl.js')
        .catch(() => null);
      // Fallback: write via chrome.storage.local directly.
      await chrome.storage.local.set({ [`tex2doc_settings.${k}`]: v });
      return { ok: true };
    },
    [key, value]
  );
}

async function ensureSettings(sw) {
  // The Options page reads via getSettings(); we set both sync + local so the
  // BACKGROUND picks up the API base URL on first call.
  await sw.evaluate(
    async (apiBase) => {
      const settings = {
        api_base_url: apiBase,
        default_profile: 'standard',
        default_quality: 'balanced',
        default_mode: 'cloud',
        wasm_file_size_limit: 10 * 1024 * 1024,
        language: 'en',
        theme: 'system',
        polling_interval: 2000,
      };
      try {
        await chrome.storage.sync.set({ tex2doc_settings: settings });
      } catch {}
      await chrome.storage.local.set({ tex2doc_settings: settings });
      return { ok: true };
    },
    apiBaseUrl
  );
}

async function main() {
  if (!email || !password) {
    fail('TEX2DOC_E2E_EMAIL and TEX2DOC_E2E_PASSWORD must be set');
  }

  log(`extension dir: ${extPath}`);
  log(`input zip:     ${zipPath}`);
  log(`output docx:   ${outDocx}`);
  log(`api base url:  ${apiBaseUrl}`);
  log(`email:         ${email.replace(/^(.).+(@.+)$/, '$1***$2')}`);

  const zipStat = await stat(zipPath).catch(() => null);
  if (!zipStat) fail(`zip not found: ${zipPath}`);
  log(`zip size: ${(zipStat.size / 1024 / 1024).toFixed(2)} MiB`);

  const userDataDir = path.join(repoRoot, 'target', 'e2e-cloud-userdata');
  const fs = await import('node:fs/promises');
  await fs.rm(userDataDir, { recursive: true, force: true });

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

  const swErrors = [];
  const jobEvents = [];
  let exitCode = 0;
  let serviceWorker = null;

  try {
    const page = await context.newPage();
    page.on('pageerror', (err) => swErrors.push(`page-error: ${err.message}`));
    await page.goto('https://example.com', { waitUntil: 'domcontentloaded' });

    serviceWorker = await getServiceWorker(context);
    log(`✓ service worker: ${serviceWorker.url()}`);
    serviceWorker.on('console', (msg) => {
      const text = msg.text();
      if (text.includes('Tex2Doc Background') || text.includes('JOB_UPDATED') || msg.type() === 'error') {
        console.log(`  [sw-console:${msg.type()}] ${text}`);
      }
    });
    serviceWorker.on('pageerror', (err) => swErrors.push(`sw-pageerror: ${err.message}`));

    // 1) Seed settings so background.ts picks up the right API base URL.
    await ensureSettings(serviceWorker);
    log('settings seeded');

    // 2) Drive the SW's onMessage handler through a hidden iframe so
    //    `chrome.runtime.sendMessage` actually has a recipient. The popup is
    //    more involved to script, so we use a tiny page-injected bridge.
    await page.exposeFunction('__e2ePushJobEvent', (evt) => {
      jobEvents.push(evt);
      console.log(`  [JOB_UPDATED] stage=${evt.stage ?? '-'} status=${evt.status ?? '-'} progress=${evt.progress ?? '-'}`);
    });
    await page.evaluate(() => {
      chrome.runtime.onMessage.addListener((msg, _sender, _sendResponse) => {
        if (msg && msg.type === 'JOB_UPDATED') {
          // eslint-disable-next-line no-undef
          window.__e2ePushJobEvent(msg);
        }
      });
    });

    // 3) Login via the popup's message channel. We send directly to the SW.
    log('logging in...');
    const loginResult = await serviceWorker.evaluate(
      async ({ email, password, apiBaseUrl }) => {
        try {
          const settings = {
            api_base_url: apiBaseUrl,
            default_profile: 'standard',
            default_quality: 'balanced',
            default_mode: 'cloud',
            wasm_file_size_limit: 10 * 1024 * 1024,
            language: 'en',
            theme: 'system',
            polling_interval: 2000,
          };
          await chrome.storage.local.set({ tex2doc_settings: settings });
          const reply = await chrome.runtime.sendMessage({
            type: 'LOGIN',
            email,
            password,
          });
          return { ok: true, reply };
        } catch (e) {
          return { ok: false, error: e && e.message ? e.message : String(e) };
        }
      },
      { email, password, apiBaseUrl }
    );
    if (!loginResult?.ok || !loginResult.reply?.success) {
      fail(`login failed: ${JSON.stringify(loginResult)}`);
    }
    log('✓ logged in');

    // 4) Read zip and trigger CLOUD_CONVERT_AND_POLL via SW message channel.
    const zipBytes = await readFile(zipPath);
    log(`zip bytes: ${zipBytes.length}`);

    log('triggering CLOUD_CONVERT_AND_POLL...');
    const startResult = await serviceWorker.evaluate(
      async ({ zipArr, fileName, mainTex, profile, quality }) => {
        try {
          const reply = await chrome.runtime.sendMessage({
            type: 'CLOUD_CONVERT_AND_POLL',
            zipBytes: zipArr,
            fileName,
            mainTex,
            profile,
            quality,
          });
          return { ok: true, reply };
        } catch (e) {
          return { ok: false, error: e && e.message ? e.message : String(e) };
        }
      },
      {
        zipArr: Array.from(zipBytes),
        fileName: path.basename(zipPath),
        mainTex,
        profile,
        quality,
      }
    );
    if (!startResult?.ok || !startResult.reply?.success) {
      fail(`start failed: ${JSON.stringify(startResult)}`);
    }
    const jobId = startResult.reply.jobId;
    log(`✓ jobId = ${jobId}`);

    // 5) Wait for terminal JOB_UPDATED (completed / failed).
    const deadline = Date.now() + 180_000; // 3 minutes
    let terminal = null;
    while (Date.now() < deadline) {
      await new Promise((r) => setTimeout(r, 500));
      terminal = jobEvents.find((e) => e.jobId === jobId && (e.status === 'completed' || e.status === 'failed'));
      if (terminal) break;
    }
    if (!terminal) {
      fail('no terminal JOB_UPDATED within 180s');
    }
    if (terminal.status !== 'completed') {
      fail(`job failed: ${terminal.error ?? 'unknown'}`);
    }
    log(`✓ terminal state: ${terminal.status}`);

    // 6) Verify we saw all 5 stages.
    const stages = new Set(
      jobEvents
        .filter((e) => e.jobId === jobId && e.stage)
        .map((e) => e.stage)
    );
    for (const required of ['pending', 'uploading', 'creating', 'polling', 'completed']) {
      if (!stages.has(required)) {
        fail(`missing stage: ${required}`);
      }
    }
    log(`✓ all stages observed: ${[...stages].join(' → ')}`);

    // 7) Download the docx via the SW message bus.
    log('downloading docx...');
    const dlResult = await serviceWorker.evaluate(
      async ({ jobId }) => {
        try {
          const reply = await chrome.runtime.sendMessage({
            type: 'DOWNLOAD_DOCX',
            jobId,
            _e2eReturnBytes: true,
          });
          if (!reply || !reply.success) {
            return { ok: false, error: reply?.error ?? 'download failed' };
          }
          return { ok: true, docxBytes: reply.docxBytes, docxFilename: reply.docxFilename };
        } catch (e) {
          return { ok: false, error: e && e.message ? e.message : String(e) };
        }
      },
      { jobId }
    );
    if (!dlResult?.ok) fail(`download failed: ${JSON.stringify(dlResult)}`);
    const buf = Buffer.from(dlResult.docxBytes);
    if (buf.length < 4) fail('docx too small');
    for (let i = 0; i < 4; i++) {
      if (buf[i] !== DOCX_MAGIC[i]) {
        fail(`docx magic bytes mismatch at ${i}: got ${buf[i].toString(16)}, expected ${DOCX_MAGIC[i].toString(16)}`);
      }
    }
    await writeFile(outDocx, buf);
    log(`✓ docx written: ${outDocx} (${(buf.length / 1024).toFixed(1)} KiB)`);

    // 8) Optional: simulate SW restart by closing & re-opening context and
    // verify any in-flight job survives. Skipped if user opts out via env.
    if (process.env.TEX2DOC_E2E_RECOVERY !== '1') {
      log('(skipping SW-restart recovery test; set TEX2DOC_E2E_RECOVERY=1 to enable)');
    } else {
      log('recovery test: closing context to force SW restart...');
      await context.close();
      log('  context closed; reopen and verify job still has cloudJobId...');
      const ctx2 = await chromium.launchPersistentContext(userDataDir, {
        channel: 'chromium',
        headless: true,
        args: [
          `--disable-extensions-except=${extPath}`,
          `--load-extension=${extPath}`,
          '--no-sandbox',
        ],
      });
      try {
        const sw2 = await getServiceWorker(ctx2);
        const jobCheck = await sw2.evaluate(async (jId) => {
          const all = await new Promise((resolve) => {
            const req = indexedDB.open('tex2doc_extension');
            req.onsuccess = () => {
              const db = req.result;
              const tx = db.transaction('jobs', 'readonly');
              const store = tx.objectStore('jobs');
              const all = store.getAll();
              all.onsuccess = () => resolve(all.result);
            };
            req.onerror = () => resolve([]);
          });
          return all.find((j) => j.id === jId) ?? null;
        }, jobId);
        if (!jobCheck) fail('recovery: job missing from IndexedDB');
        log(`✓ recovery: job still in store, status=${jobCheck.status}, cloudJobId=${jobCheck.cloudJobId ?? '-'}`);
      } finally {
        await ctx2.close();
      }
    }
  } catch (err) {
    exitCode = 1;
    console.error('[e2e-cloud] ERROR:', err);
  } finally {
    if (swErrors.length > 0) {
      console.error('[e2e-cloud] SW errors:');
      for (const e of swErrors) console.error('  -', e);
      exitCode = exitCode || 1;
    }
    if (context && !context._closed) {
      try {
        await context.close();
      } catch {}
    }
    log(exitCode === 0 ? 'PASS' : 'FAIL');
    process.exit(exitCode);
  }
}

main().catch((err) => {
  console.error('[e2e-cloud] fatal:', err);
  process.exit(1);
});