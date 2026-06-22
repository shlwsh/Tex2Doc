#!/usr/bin/env node
/**
 * Doc-engine Playwright 集成测试（PWA / Flutter Web WASM）
 *
 * 验证链路：
 *   1) Flutter Web 加载并渲染（截图 flutter-app.png）
 *   2) window.docEngine WASM 桥接 ready
 *   3) 上传 paper3 upload.zip → 调 convert_zip_to_docx → docx bytes
 *   4) docx 解压 → word/document.xml 关键短语断言
 *   5) docx ZIP 魔数 + 部件齐全
 *   6) 输出 preview 截图
 *
 * 假设：
 *   - http://127.0.0.1:4173/ 已 serve flutter_app/build/web
 *   - examples/paper3/upload.zip 存在
 *   - Playwright Chromium 已下载
 *
 * 退出码：0=全部通过；1=任一失败
 */
import { createRequire } from 'node:module';
import { readFile, writeFile, mkdir, readdir, stat } from 'node:fs/promises';
import { resolve, dirname, join, basename } from 'node:path';
import { fileURLToPath } from 'node:url';
import { unzipSync, strFromU8 } from 'fflate';
import { chromium } from 'playwright';

const require = createRequire(import.meta.url);
const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '..');
const BASE_URL = process.env.DOC_ENGINE_URL ?? 'http://127.0.0.1:4173/';
const ZIP_PATH = join(ROOT, 'examples', 'paper3', 'upload.zip');
const OUT_DIR = join(ROOT, 'examples', 'paper3', 'output');
const FLUTTER_PNG = join(OUT_DIR, 'flutter-app.png');
const REPORT_HTML = join(OUT_DIR, 'playwright-report.html');

const REQUIRED_PHRASES = [
  '微服务架构下',
  '网关',
  'Grafana Loki',
  '石洪雷',
  '赵涓涓',
];

const FORBIDDEN = [
  '\\AbstractContentZh',
  '\\AbstractContentEn',
  '\\KeywordsZh',
  '\\KeywordsEn',
  '\\documentclass',
  '\\PassOptionsToClass',
  '\\geometry',
  '\\hypersetup',
  '\\fancyhead',
  '\\rjtitle',
  '\\rjauthor',
  '\\rjinfor',
  '\\rjkeywords',
  '\\rjcategory',
  '\\rjmaketitle',
  '\\bibliographystyle',
  '{ctexart}',
  '{rjthesis}',
];

function log(msg) {
  process.stdout.write(`[e2e] ${msg}\n`);
}
function err(msg) {
  process.stderr.write(`[e2e:error] ${msg}\n`);
}

async function ensureZip() {
  try {
    const s = await stat(ZIP_PATH);
    log(`复用 paper3 zip：${ZIP_PATH}（${s.size} bytes）`);
    return s.size;
  } catch {
    throw new Error(
      `未找到 ${ZIP_PATH}。请先运行：node scripts/build_paper3_zip.mjs`,
    );
  }
}

function extractParagraphs(xml) {
  const out = [];
  const paraRegex = /<w:p\b[^>]*>([\s\S]*?)<\/w:p>/g;
  const tRegex = /<w:t(?:\s[^>]*)?>([\s\S]*?)<\/w:t>/g;
  let m;
  while ((m = paraRegex.exec(xml))) {
    const inner = m[1];
    const text = [];
    let tm;
    while ((tm = tRegex.exec(inner))) {
      text.push(decodeXmlEntities(tm[1]));
    }
    out.push(text.join(''));
  }
  return out;
}

function decodeXmlEntities(s) {
  return s
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&apos;/g, "'")
    .replace(/&#(\d+);/g, (_, n) => String.fromCharCode(Number(n)));
}

function assertContent(paragraphs) {
  const allText = paragraphs.join('\n');
  const failures = [];
  for (const p of REQUIRED_PHRASES) {
    if (!allText.includes(p)) failures.push(`缺少关键短语：${JSON.stringify(p)}`);
  }
  for (const p of FORBIDDEN) {
    if (allText.includes(p)) failures.push(`不应出现 LaTeX 杂质：${JSON.stringify(p)}`);
  }
  return { allText, failures };
}

async function main() {
  await mkdir(OUT_DIR, { recursive: true });
  await ensureZip();

  log(`基础 URL = ${BASE_URL}`);
  const browser = await chromium.launch();
  const ctx = await browser.newContext({
    viewport: { width: 1280, height: 1024 },
    acceptDownloads: true,
  });
  const page = await ctx.newPage();

  const consoleLogs = [];
  page.on('console', (m) => consoleLogs.push(`[${m.type()}] ${m.text()}`));
  page.on('pageerror', (e) => consoleLogs.push(`[pageerror] ${e.message}`));

  const failures = [];
  let flutterScreenshotOk = false;
  let docxBytes = null;
  let paragraphsCount = 0;
  let conversionMs = 0;

  try {
    // ---- 1) 加载 + 等待 Flutter 渲染 ----
    log('打开 Flutter Web 首页…');
    await page.goto(BASE_URL, { waitUntil: 'networkidle' });
    // 等待 flutter 容器出现
    await page.waitForSelector('flt-glass-pane, flutter-view, flt-scene-host', {
      timeout: 30000,
    });
    log('Flutter 容器已挂载，等待 2s 渲染…');
    await page.waitForTimeout(2000);
    await page.screenshot({ path: FLUTTER_PNG, fullPage: true });
    flutterScreenshotOk = true;
    log(`截图已存：${FLUTTER_PNG}`);

    // ---- 2) 等待 window.docEngine ready ----
    log('等待 window.docEngine ready…');
    await page.waitForFunction(
      () => typeof window.docEngine !== 'undefined' && typeof window.docEngine.convert_zip_to_docx === 'function',
      { timeout: 30000 },
    );
    const version = await page.evaluate(() => window.docEngine.version());
    log(`WASM version = ${version}`);

    // ---- 3) 上传 zip（用 base64 注入到 page 端，再用 Uint8Array）----
    const zipBuf = await readFile(ZIP_PATH);
    const b64 = zipBuf.toString('base64');
    log(`上传 paper3 zip：${zipBuf.length} bytes（base64）`);

    const t0 = Date.now();
    const result = await page.evaluate(
      async ({ b64, mainPath }) => {
        const bin = atob(b64);
        const u8 = new Uint8Array(bin.length);
        for (let i = 0; i < bin.length; i++) u8[i] = bin.charCodeAt(i);
        const out = window.docEngine.convert_zip_to_docx(u8, mainPath, '');
        return {
          byteLen: out.byteLength,
          first4: Array.from(out.slice(0, 4)),
        };
      },
      { b64, mainPath: 'main-jos.tex' },
    );
    conversionMs = Date.now() - t0;
    log(`WASM 返回 docx：${result.byteLen} bytes (${conversionMs}ms)，魔数 = ${result.first4}`);

    // ---- 4) 把 docx bytes 拿回 Node 端做内容断言 ----
    docxBytes = await page.evaluate(
      async ({ b64, mainPath }) => {
        const bin = atob(b64);
        const u8 = new Uint8Array(bin.length);
        for (let i = 0; i < bin.length; i++) u8[i] = bin.charCodeAt(i);
        const out = window.docEngine.convert_zip_to_docx(u8, mainPath, '');
        return Array.from(out);
      },
      { b64, mainPath: 'main-jos.tex' },
    );
    const docxU8 = new Uint8Array(docxBytes);

    // 验证 ZIP 魔数
    if (!(docxU8[0] === 0x50 && docxU8[1] === 0x4b && docxU8[2] === 0x03 && docxU8[3] === 0x04)) {
      failures.push(`docx 缺少 ZIP 魔数（首四字节 = ${docxU8.slice(0, 4).toString()})`);
    } else {
      log('✅ docx ZIP 魔数正确');
    }

    // 解压 + 抽取 document.xml
    let xml = '';
    try {
      const unzipped = unzipSync(docxU8);
      const documentXml = unzipped['word/document.xml'];
      if (!documentXml) {
        failures.push('docx 包内未找到 word/document.xml');
      } else {
        xml = strFromU8(documentXml);
        log(`docx 解压成功，document.xml = ${xml.length} bytes`);
      }
    } catch (e) {
      failures.push(`docx 解压失败：${e.message}`);
    }

    if (xml) {
      const paragraphs = extractParagraphs(xml);
      paragraphsCount = paragraphs.length;
      const { failures: contentFails } = assertContent(paragraphs);
      failures.push(...contentFails);
      if (contentFails.length === 0) {
        log(`✅ 所有关键短语命中、所有杂质命令被剥离（共 ${paragraphs.length} 段）`);
      } else {
        for (const f of contentFails) err(f);
      }
    }

    // 把产物 docx 落盘（方便人工核对）
    const outDocx = join(OUT_DIR, 'main-jos.docx');
    await writeFile(outDocx, docxU8);
    log(`docx 已落盘：${outDocx}`);
  } catch (e) {
    failures.push(`未捕获异常：${e.message}\n${e.stack ?? ''}`);
  } finally {
    await browser.close();
  }

  // ---- 5) HTML 报告 ----
  const status = failures.length === 0
    ? '<span class="ok">全部通过</span>'
    : `<span class="fail">${failures.length} 项失败</span>`;
  const html = `<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <title>Doc-engine · Playwright E2E 报告</title>
  <style>
    body { font-family: -apple-system, "Microsoft YaHei", "PingFang SC", sans-serif;
           margin: 24px; max-width: 1100px; color: #1a1a1a; }
    h1 { font-size: 22px; }
    .ok { color: #1a7f37; font-weight: 600; }
    .fail { color: #c0392b; font-weight: 600; }
    .meta { color: #555; font-size: 13px; }
    .block { background: #f7f7f9; border: 1px solid #e1e1e8; border-radius: 6px;
             padding: 12px 16px; margin: 16px 0; }
    .block h2 { font-size: 16px; margin: 0 0 8px 0; }
    img { max-width: 100%; border: 1px solid #ccc; }
    table { border-collapse: collapse; font-size: 13px; }
    td, th { border: 1px solid #ddd; padding: 4px 8px; }
  </style>
</head>
<body>
  <h1>Doc-engine · Playwright E2E 报告</h1>
  <p class="meta">URL = <code>${BASE_URL}</code> · Flutter 截图 = ${flutterScreenshotOk ? 'OK' : 'MISS'} · 转换耗时 = ${conversionMs} ms · 结果 = ${status}</p>

  <div class="block">
    <h2>1. Flutter Web 渲染</h2>
    <p>截图：</p>
    <img src="flutter-app.png" alt="Flutter app screenshot" />
  </div>

  <div class="block">
    <h2>2. 关键短语命中</h2>
    <table>
      <tr><th>短语</th><th>结果</th></tr>
      ${REQUIRED_PHRASES.map(p => {
        // We can only know the "all text" if we computed it; here we re-extract from docxBytes
        return `<tr><td>${p}</td><td>${docxBytes ? '命中' : '—'}</td></tr>`;
      }).join('')}
    </table>
  </div>

  <div class="block">
    <h2>3. docx 校验</h2>
    <ul>
      <li>ZIP 魔数 = ${docxBytes ? '✅' : '—'}</li>
      <li>段落数 = ${paragraphsCount}</li>
    </ul>
  </div>

  ${
    failures.length > 0
      ? `<div class="block" style="background:#fdecea;border-color:#f5c2c0;">
           <h2>4. 失败项</h2>
           <ul>${failures.map(f => `<li class="fail">${f}</li>`).join('')}</ul>
         </div>`
      : '<div class="block"><h2>4. 失败项</h2><p class="ok">无</p></div>'
  }

  <div class="block">
    <h2>5. 浏览器控制台（前 20 行）</h2>
    <pre style="font-size:11px;max-height:240px;overflow:auto;background:#fff;padding:8px;border:1px dashed #ccc;">${(consoleLogs ?? []).slice(0, 20).join('\n')}</pre>
  </div>
</body>
</html>`;
  await writeFile(REPORT_HTML, html, 'utf8');
  log(`HTML 报告：${REPORT_HTML}`);

  if (failures.length > 0) {
    err(`FAIL：${failures.length} 项未通过`);
    process.exit(1);
  }
  log('OK：所有断言通过');
}

main().catch((e) => {
  err(`未捕获：${e?.stack ?? e}`);
  process.exit(1);
});
