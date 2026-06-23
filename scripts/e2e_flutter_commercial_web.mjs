#!/usr/bin/env node
/**
 * Flutter Web commercial integration test.
 *
 * Verifies the browser-facing commercial workflow with Playwright:
 * - Flutter Web page boots and exposes semantic buttons.
 * - Register, login, usage refresh, and plans buttons call the real API.
 * - ZIP selection, local WASM conversion, and DOCX download complete.
 *
 * Environment:
 * - WEB_PORT=4174
 * - API_PORT=8080
 * - SKIP_FLUTTER_BUILD=1 to reuse flutter_app/build/web
 */
import { spawn } from 'node:child_process';
import { mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { dirname, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { strToU8, zipSync } from 'fflate';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '..');
const flutterAppDir = resolve(repoRoot, 'flutter_app');
const targetDir = resolve(repoRoot, 'target', 'playwright');
const webPort = Number(process.env.WEB_PORT ?? 4174);
const apiPort = Number(process.env.API_PORT ?? 8080);
const webUrl = `http://127.0.0.1:${webPort}/`;
const apiBase = `http://127.0.0.1:${apiPort}`;
const spawned = [];

mkdirSync(targetDir, { recursive: true });

function log(message) {
  process.stdout.write(`[e2e-flutter-web] ${message}\n`);
}

function spawnProcess(command, args, options = {}) {
  const child = spawn(command, args, {
    cwd: repoRoot,
    env: { ...process.env, ...options.env },
    shell: process.platform === 'win32',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  child.stdout.on('data', (data) => process.stdout.write(data));
  child.stderr.on('data', (data) => process.stderr.write(data));
  spawned.push(child);
  return child;
}

async function run(command, args, options = {}) {
  log(`running: ${command} ${args.join(' ')}`);
  const child = spawn(command, args, {
    cwd: options.cwd ?? repoRoot,
    env: { ...process.env, ...options.env },
    shell: process.platform === 'win32',
    stdio: 'inherit',
  });
  const code = await new Promise((resolveCode) => child.on('close', resolveCode));
  if (code !== 0) {
    throw new Error(`${command} ${args.join(' ')} exited with ${code}`);
  }
}

async function fetchOk(url) {
  try {
    const response = await fetch(url);
    return response.ok;
  } catch {
    return false;
  }
}

async function waitFor(url, label, timeoutMs = 90000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await fetchOk(url)) return;
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 1000));
  }
  throw new Error(`${label} did not become ready at ${url}`);
}

async function ensureApiServer() {
  const healthUrl = `${apiBase}/api/v1/health`;
  if (await fetchOk(healthUrl)) {
    log(`API already ready: ${healthUrl}`);
    return;
  }
  log(`starting doc-server on 127.0.0.1:${apiPort}`);
  spawnProcess('cargo', ['run', '-p', 'doc-server'], {
    env: { DOC_SERVER_ADDR: `127.0.0.1:${apiPort}` },
  });
  await waitFor(healthUrl, 'doc-server');
}

async function ensureWebServer() {
  if (await fetchOk(webUrl)) {
    log(`Flutter Web static server already ready: ${webUrl}`);
    return;
  }
  log(`starting Flutter Web static server on 127.0.0.1:${webPort}`);
  spawnProcess(process.execPath, ['scripts/serve_flutter_web.mjs'], {
    env: { HOST: '127.0.0.1', PORT: String(webPort) },
  });
  await waitFor(webUrl, 'Flutter Web static server');
}

function createFixtureZip() {
  const paper3 = resolve(repoRoot, 'examples', 'paper3');
  const latex = resolve(paper3, 'latex');
  const figures = resolve(paper3, 'figures');
  const entries = {};

  for (const name of ['main-jos.tex', 'main-zh.tex', 'references.bib', 'rjthesis.cls']) {
    entries[name] = new Uint8Array(readFileSync(resolve(latex, name)));
  }

  function addDir(sourceDir, zipPrefix, accept) {
    for (const entry of readdirSync(sourceDir)) {
      const abs = resolve(sourceDir, entry);
      const stats = statSync(abs);
      if (stats.isDirectory()) {
        addDir(abs, `${zipPrefix}${entry}/`, accept);
      } else if (accept(abs)) {
        const rel = `${zipPrefix}${relative(sourceDir, abs).split(sep).join('/')}`;
        entries[rel] = new Uint8Array(readFileSync(abs));
      }
    }
  }

  addDir(resolve(latex, 'sections'), 'sections/', (file) => file.endsWith('.tex'));
  addDir(figures, 'figures/', (file) => /\.(png|pdf)$/i.test(file));

  const zipPath = resolve(targetDir, 'commercial-web-paper3.zip');
  writeFileSync(zipPath, Buffer.from(zipSync(entries)));
  const size = statSync(zipPath).size;
  if (size >= 10 * 1024 * 1024) {
    throw new Error(`paper3 fixture exceeds frontend upload guard: ${size} bytes`);
  }
  log(`paper3 fixture: ${zipPath} (${size} bytes)`);
  return zipPath;
}

async function enableFlutterSemantics(page) {
  await page.waitForSelector('[aria-label="Enable accessibility"]', {
    state: 'attached',
    timeout: 60000,
  });
  await page.evaluate(() =>
    document.querySelector('[aria-label="Enable accessibility"]')?.click(),
  );
  await page.waitForSelector('flt-semantics[role="button"]', {
    state: 'attached',
    timeout: 60000,
  });
}

async function buttonLabels(page) {
  return page.locator('flt-semantics[role="button"]').evaluateAll((nodes) =>
    nodes.map((node) => node.textContent?.trim() ?? '').filter(Boolean),
  );
}

async function clickButton(page, name) {
  const box = await page.locator('flt-semantics[role="button"]').evaluateAll(
    (nodes, buttonName) => {
      const node =
        nodes.find((item) => item.textContent?.trim() === buttonName) ??
        nodes.find((item) => item.textContent?.trim().includes(buttonName));
      if (!node) return null;
      const rect = node.getBoundingClientRect();
      return {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
      };
    },
    name,
  );
  if (!box) {
    throw new Error(`button not found: ${name}`);
  }
  await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
}

async function fillSemanticInput(page, index, value) {
  const box = await page.locator('input[aria-label]').nth(index).boundingBox();
  if (!box) {
    throw new Error(`input not found at index ${index}`);
  }
  await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
  await page.keyboard.press('End');
  for (let i = 0; i < 80; i += 1) {
    await page.keyboard.press('Backspace');
  }
  await page.keyboard.type(value);
}

async function expectJsonResponse(page, path, trigger) {
  const responsePromise = page.waitForResponse(
    (response) =>
      response.url().includes(path) &&
      response.request().method() !== 'OPTIONS',
    { timeout: 45000 },
  );
  await trigger();
  const response = await responsePromise;
  const body = await response.json();
  log(`response ${response.request().method()} ${response.url()} -> ${response.status()}`);
  if (!response.ok()) {
    throw new Error(`${path} returned ${response.status()}: ${JSON.stringify(body)}`);
  }
  return body;
}

async function runBrowserTest() {
  const fixtureZip = createFixtureZip();
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({
    acceptDownloads: true,
    viewport: { width: 1440, height: 900 },
  });
  const page = await context.newPage();
  page.on('pageerror', (error) => {
    throw error;
  });
  page.on('console', (message) => {
    const text = message.text();
  if (
      text.includes('[doc-engine]') ||
      text.includes('Exception') ||
      text.includes('Error')
    ) {
      log(`browser ${message.type()}: ${text}`);
    }
  });

  await page.goto(webUrl, { waitUntil: 'networkidle', timeout: 60000 });
  await enableFlutterSemantics(page);

  const labels = await buttonLabels(page);
  for (const expected of ['注册', '登录', '刷新', '套餐', '选择 ZIP', '开始转换']) {
    if (!labels.includes(expected)) {
      throw new Error(`missing semantic button ${expected}; found: ${labels.join(', ')}`);
    }
  }

  const corsProbe = await page.evaluate(async (url) => {
    const response = await fetch(`${url}/v1/auth/register`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ email: 'cors-probe@example.com', password: 'demo' }),
    });
    return { ok: response.ok, status: response.status };
  }, apiBase);
  if (!corsProbe.ok) {
    throw new Error(`CORS register probe failed: ${JSON.stringify(corsProbe)}`);
  }

  log('filling demo account');
  await fillSemanticInput(page, 0, `${apiBase}/v1/`);
  await fillSemanticInput(page, 1, 'demo');
  await fillSemanticInput(page, 2, 'demo');

  log('click register');
  const register = await expectJsonResponse(page, '/v1/auth/register', () =>
    clickButton(page, '注册'),
  );
  if (!register.access_token || register.user?.email !== 'demo' || register.user?.plan_id !== 'preview') {
    throw new Error(`unexpected register response: ${JSON.stringify(register)}`);
  }

  log('click login');
  const login = await expectJsonResponse(page, '/v1/auth/login', () =>
    clickButton(page, '登录'),
  );
  if (!login.access_token) {
    throw new Error(`unexpected login response: ${JSON.stringify(login)}`);
  }

  log('click usage refresh');
  const usage = await expectJsonResponse(page, '/v1/usage', () =>
    clickButton(page, '刷新'),
  );
  if (typeof usage.cloud_conversions_limit !== 'number') {
    throw new Error(`unexpected usage response: ${JSON.stringify(usage)}`);
  }

  log('click plans');
  const plans = await expectJsonResponse(page, '/v1/plans', () =>
    clickButton(page, '套餐'),
  );
  if (!Array.isArray(plans) || plans.length === 0) {
    throw new Error(`unexpected plans response: ${JSON.stringify(plans)}`);
  }

  log('select paper3 ZIP');
  await clickButton(page, '选择 ZIP');
  await page.setInputFiles('#zip-file-input', fixtureZip);
  await page.waitForTimeout(1000);
  log('click cloud semantic convert');
  const uploadResponsePromise = page.waitForResponse(
    (response) =>
      response.url().includes('/v1/uploads') &&
      response.request().method() === 'POST',
    { timeout: 45000 },
  );
  const createConversionResponsePromise = page.waitForResponse(
    (response) =>
      response.url().includes('/v1/conversions') &&
      response.request().method() === 'POST',
    { timeout: 45000 },
  );
  const downloadApiResponsePromise = page.waitForResponse(
    (response) =>
      response.url().includes('/download/docx') &&
      response.request().method() === 'GET',
    { timeout: 180000 },
  );
  await clickButton(page, '开始转换');
  for (const response of [
    await uploadResponsePromise,
    await createConversionResponsePromise,
    await downloadApiResponsePromise,
  ]) {
    log(`response ${response.request().method()} ${response.url()} -> ${response.status()}`);
    if (!response.ok()) {
      throw new Error(`cloud conversion request failed: ${response.status()} ${response.url()}`);
    }
  }
  try {
    await page.waitForFunction(() => {
      return Array.from(document.querySelectorAll('flt-semantics[role="button"]'))
        .some((node) => node.textContent?.trim().includes('下载'));
    }, undefined, { timeout: 180000 });
  } catch (error) {
    const failureScreenshot = resolve(targetDir, 'commercial-web-convert-failure.png');
    await page.screenshot({ path: failureScreenshot, fullPage: true });
    const semantics = await page.locator('flt-semantics').evaluateAll((nodes) =>
      nodes.map((node) => node.textContent?.trim() ?? '').filter(Boolean).join('\n'),
    );
    log(`convert failure screenshot: ${failureScreenshot}`);
    log(`convert semantics tail: ${semantics.slice(-2000)}`);
    throw error;
  }

  const downloadPromise = page.waitForEvent('download', { timeout: 30000 });
  await clickButton(page, '下载');
  const download = await downloadPromise;
  const docxPath = resolve(targetDir, 'commercial-web-output.docx');
  await download.saveAs(docxPath);
  const docx = readFileSync(docxPath);
  if (docx.length < 1024 || docx[0] !== 0x50 || docx[1] !== 0x4B) {
    throw new Error(`invalid DOCX download: ${docx.length} bytes`);
  }

  const screenshotPath = resolve(targetDir, 'commercial-web-buttons.png');
  await page.screenshot({ path: screenshotPath, fullPage: true });
  await context.close();
  await browser.close();

  log(`PASS register/login/usage/plans/convert/download`);
  log(`screenshot: ${screenshotPath}`);
  log(`download: ${docxPath} (${docx.length} bytes)`);
}

async function main() {
  if (process.env.SKIP_FLUTTER_BUILD !== '1') {
    await run('flutter', ['build', 'web', '--debug'], { cwd: flutterAppDir });
  }
  await ensureApiServer();
  await ensureWebServer();
  await runBrowserTest();
}

main()
  .catch((error) => {
    console.error(error);
    process.exitCode = 1;
  })
  .finally(() => {
    for (const child of spawned.reverse()) {
      if (!child.killed) child.kill();
    }
  });
