#!/usr/bin/env node
/**
 * Tex2Doc 端到端视觉验证脚本（Playwright 驱动）
 *
 * 用法：
 *     node scripts/verify_paper3.mjs
 *
 * 行为契约：
 * 1. 调用 Rust 集成测试 `cargo test -p doc-core --test paper3_e2e` 生成 docx；
 *    （若 docx 已存在且 `--no-cargo` 标记给出，则跳过 cargo 调用。）
 * 2. 用 `fflate` 把 docx 解压，读 `word/document.xml`；
 * 3. 抽取所有 `<w:t>` 文本内容，做关键短语断言；
 * 4. 用 Playwright 打开一个临时 HTML 报告页（含 docx 摘要、纯文本预览），
 *    截全屏 PNG 到 `examples/paper3/output/preview.png`；
 * 5. 退出码 0=通过，1=失败。
 *
 * 失败时所有失败项都会打到 stderr，并通过 HTML 报告页红字高亮。
 */

import { execFileSync, spawnSync } from "node:child_process";
import { mkdirSync, writeFileSync, existsSync, readFileSync, statSync } from "node:fs";
import { resolve, dirname, join, basename } from "node:path";
import { fileURLToPath } from "node:url";
import { unzipSync, strFromU8 } from "fflate";
import { chromium } from "playwright";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, "..");
const DOCX_PATH = join(ROOT, "examples/paper3/output/main-jos.docx");
const PREVIEW_PATH = join(ROOT, "examples/paper3/output/preview.png");
const REPORT_PATH = join(ROOT, "examples/paper3/output/report.html");

// ---- 配置：关键短语断言（与 paper3_e2e.rs 同步漂移） ------------------
const REQUIRED_PHRASES = [
  "微服务架构下",
  "网关",
  "Grafana Loki",
  "石洪雷",
  "赵涓涓",
];
const FORBIDDEN = [
  "\\AbstractContentZh",
  "\\AbstractContentEn",
  "\\KeywordsZh",
  "\\KeywordsEn",
  "\\documentclass",
  "\\usepackage",
  "\\PassOptionsToClass",
  "\\geometry",
  "\\begin{CJK}",
  "\\hypersetup",
  "\\newcommand",
  "\\fancyhead",
  "\\rjtitle",
  "\\rjauthor",
  "\\rjinfor",
  "\\rjkeywords",
  "\\rjcategory",
  "\\rjmaketitle",
  "\\bibliographystyle",
  "{ctexart}",
  "{rjthesis}",
];
// ------------------------------------------------------------------------

function log(msg) { process.stderr.write(`[verify] ${msg}\n`); }

function ensureDocx() {
  if (process.argv.includes("--no-cargo") && existsSync(DOCX_PATH)) {
    log(`复用已存在的 docx：${DOCX_PATH}`);
    return;
  }
  log("调用 cargo test -p doc-core --test paper3_e2e 生成 docx ...");
  const r = spawnSync(
    "cargo",
    ["test", "-p", "doc-core", "--test", "paper3_e2e"],
    { stdio: "inherit", cwd: ROOT }
  );
  if (r.status !== 0) {
    log(`cargo test 失败（exit=${r.status}）`);
    process.exit(1);
  }
  if (!existsSync(DOCX_PATH)) {
    log(`未找到产物：${DOCX_PATH}`);
    process.exit(1);
  }
}

function readDocxXml(docxPath) {
  const buf = readFileSync(docxPath);
  const unzipped = unzipSync(new Uint8Array(buf));
  if (!unzipped["word/document.xml"]) {
    throw new Error("docx 缺少 word/document.xml");
  }
  return strFromU8(unzipped["word/document.xml"]);
}

function extractParagraphs(xml) {
  // 极简提取：匹配所有 <w:t>...</w:t> 与 <w:p>...</w:p> 边界，把段落分开。
  // 兼容 run / pPr 之间的嵌套。
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
    out.push(text.join(""));
  }
  return out;
}

function decodeXmlEntities(s) {
  return s
    .replace(/&amp;/g, "&")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&apos;/g, "'")
    .replace(/&#(\d+);/g, (_, n) => String.fromCharCode(Number(n)));
}

function runContentAssertions(paragraphs) {
  const allText = paragraphs.join("\n");
  const failures = [];
  for (const phrase of REQUIRED_PHRASES) {
    if (!allText.includes(phrase)) {
      failures.push(`缺少关键短语：${JSON.stringify(phrase)}`);
    }
  }
  for (const bad of FORBIDDEN) {
    if (allText.includes(bad)) {
      failures.push(`不应出现 LaTeX 杂质：${JSON.stringify(bad)}`);
    }
  }
  return { allText, failures };
}

function htmlEscape(s) {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function buildReportHtml({ docxBytes, paragraphs, allText, failures, headerOk }) {
  const status = failures.length === 0 && headerOk
    ? '<span class="ok">通过</span>'
    : '<span class="fail">不通过</span>';
  const li = (s) => `<li>${htmlEscape(s)}</li>`;
  return `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8" />
<title>Tex2Doc · paper3 验证报告</title>
<style>
  body { font-family: -apple-system, "Microsoft YaHei", "PingFang SC", sans-serif;
         margin: 24px; max-width: 1200px; color: #1a1a1a; }
  h1 { font-size: 22px; }
  .ok { color: #1a7f37; font-weight: 600; }
  .fail { color: #c0392b; font-weight: 600; }
  .meta { color: #555; font-size: 13px; }
  .block { background: #f7f7f9; border: 1px solid #e1e1e8; border-radius: 6px;
           padding: 12px 16px; margin: 16px 0; }
  .block h2 { font-size: 16px; margin: 0 0 8px 0; }
  .preview { white-space: pre-wrap; word-wrap: break-word; font-size: 13px;
             line-height: 1.6; max-height: 600px; overflow: auto;
             background: #fff; padding: 12px; border: 1px dashed #ccc; }
  .failures { background: #fdecea; border-color: #f5c2c0; }
</style>
</head>
<body>
  <h1>Tex2Doc · paper3 端到端验证报告</h1>
  <p class="meta">docx = <code>${htmlEscape(basename(DOCX_PATH))}</code> · ${docxBytes} bytes · 段落数 = ${paragraphs.length} · 结果：${status}</p>

  <div class="block">
    <h2>内容断言</h2>
    ${failures.length === 0
      ? '<p class="ok">所有关键短语命中，所有杂质命令被剥离。</p>'
      : `<ul class="failures">${failures.map(li).join("")}</ul>`}
  </div>

  <div class="block">
    <h2>关键短语（必须出现）</h2>
    <ul>
      ${REQUIRED_PHRASES.map(p =>
        `<li>${htmlEscape(p)} — ${allText.includes(p) ? '<span class="ok">命中</span>' : '<span class="fail">缺失</span>'}</li>`
      ).join("")}
    </ul>
  </div>

  <div class="block">
    <h2>LaTeX 杂质（必须不存在）</h2>
    <ul>
      ${FORBIDDEN.map(p =>
        `<li><code>${htmlEscape(p)}</code> — ${allText.includes(p) ? '<span class="fail">出现</span>' : '<span class="ok">已剥离</span>'}</li>`
      ).join("")}
    </ul>
  </div>

  <div class="block">
    <h2>正文预览（前 30 段）</h2>
    <div class="preview">${paragraphs.slice(0, 30).map(htmlEscape).join("\n")}</div>
  </div>
</body>
</html>`;
}

async function renderReportAndScreenshot(reportHtml) {
  mkdirSync(dirname(REPORT_PATH), { recursive: true });
  writeFileSync(REPORT_PATH, reportHtml, "utf8");
  log(`报告 HTML：${REPORT_PATH}`);

  const browser = await chromium.launch();
  try {
    const ctx = await browser.newContext({ viewport: { width: 1280, height: 1024 } });
    const page = await ctx.newPage();
    await page.goto("file:///" + REPORT_PATH.replace(/\\/g, "/"));
    // 等到 <h1> 渲染出来即可
    await page.waitForSelector("h1");
    await page.screenshot({ path: PREVIEW_PATH, fullPage: true });
    log(`截图：${PREVIEW_PATH}`);
  } finally {
    await browser.close();
  }
}

function checkZipHeader(docxPath) {
  const fd = readFileSync(docxPath);
  if (fd.length < 4) return false;
  return fd[0] === 0x50 && fd[1] === 0x4b && fd[2] === 0x03 && fd[3] === 0x04;
}

async function main() {
  ensureDocx();
  const stat = statSync(DOCX_PATH);
  const headerOk = checkZipHeader(DOCX_PATH);
  if (!headerOk) {
    log("docx 缺少 ZIP 魔数");
    process.exit(1);
  }
  const xml = readDocxXml(DOCX_PATH);
  const paragraphs = extractParagraphs(xml);
  log(`docx 共 ${paragraphs.length} 段，全文 ${xml.length} 字节 XML`);
  const { allText, failures } = runContentAssertions(paragraphs);
  for (const f of failures) log(`✗ ${f}`);

  const reportHtml = buildReportHtml({
    docxBytes: stat.size,
    paragraphs,
    allText,
    failures,
    headerOk,
  });
  await renderReportAndScreenshot(reportHtml);

  if (failures.length > 0) {
    log(`FAIL：${failures.length} 项内容断言失败`);
    process.exit(1);
  }
  log("OK：所有断言通过");
}

main().catch((e) => {
  log(`未捕获异常：${e?.stack ?? e}`);
  process.exit(1);
});
