// popup.js — Doc-engine MV3 Popup（本地 WASM 驱动）
//
// 职责：
// 1. 加载 WASM（popup/wasm/doc_engine.js → init()）
// 2. 文件选择 → bytes
// 3. 调用 docEngine.convert_zip_to_docx(bytes, main_tex)
// 4. 下载 .docx（Blob URL）
// 5. 大小分流：>= 5 MB → chrome.notifications 提示跳 App

// ---- DOM refs ----
const versionBadge = document.getElementById('version-badge');
const statusBar    = document.getElementById('status-bar');
const statusIcon   = document.getElementById('status-icon');
const statusText   = document.getElementById('status-text');
const zipInput     = document.getElementById('zip-input');
const pickLabel    = document.getElementById('pick-label');
const fileName     = document.getElementById('file-name');
const mainTexInput = document.getElementById('main-tex-input');
const convertBtn   = document.getElementById('convert-btn');
const resultSec    = document.getElementById('result-section');
const resultInfo   = document.getElementById('result-info');
const downloadBtn  = document.getElementById('download-btn');
const errorSec     = document.getElementById('error-section');
const errorText    = document.getElementById('error-text');

// ---- State ----
let zipBytes = null;
let zipFileName = null;
let docxBytes = null;

// ---- WASM engine ----
/** @type {any} */
let docEngine = null;
/** @type {WebAssembly.Instance | null} */
let wasmReady = false;

async function initWasm() {
  setStatus('loading', '正在加载 WASM 引擎…');
  try {
    // popup/wasm/doc_engine.js 用 import.meta.url 解析同目录 wasm
    const mod = await import('./wasm/doc_engine.js');
    // 找到 wasm URL（wasm-pack --target web 会把 wasm 路径嵌到 init() 参数）
    const wasmUrl = chrome.runtime.getURL('popup/wasm/doc_engine_bg.wasm');
    const resp = await fetch(wasmUrl);
    if (!resp.ok) throw new Error(`fetch wasm failed: ${resp.status}`);
    const wasmBuffer = await resp.arrayBuffer();
    await mod.default({ wasmBinary: wasmBuffer });
    docEngine = mod;
    versionBadge.textContent = 'v' + (docEngine.VERSION || '0.1.0');
    wasmReady = true;
    setStatus('ready', '就绪');
  } catch (err) {
    setStatus('error', 'WASM 加载失败：' + err.message);
    console.error('[Doc-engine popup] WASM init error:', err);
  }
}

// ---- UI helpers ----
function setStatus(type, text, icon) {
  statusBar.className = 'status-bar ' + type;
  statusIcon.textContent = icon || { loading: '\u23F3', ready: '\u2713', error: '\u2717' }[type] || '?';
  statusText.textContent = text;
}

function showError(msg) {
  errorSec.classList.remove('hidden');
  errorText.textContent = msg;
}

function hideError() {
  errorSec.classList.add('hidden');
}

function showResult(info) {
  resultSec.classList.remove('hidden');
  resultInfo.textContent = info;
}

function hideResult() {
  resultSec.classList.add('hidden');
}

// ---- File pick handler ----
zipInput.addEventListener('change', () => {
  const file = zipInput.files?.[0];
  if (!file) return;
  const sizeMB = file.size / (1024 * 1024);
  if (sizeMB >= 5) {
    showError(`文件 ${sizeMB.toFixed(1)} MB，超过 5 MB 上限。\n请使用 Doc-engine 桌面 App 或 PWA。`);
    chrome.notifications?.create?.('doc-engine-size', {
      type: 'basic',
      title: '文件过大',
      message: `文件 ${sizeMB.toFixed(1)} MB。请使用 Doc-engine 桌面端 App。`,
      iconUrl: chrome.runtime.getURL('icons/icon48.png'),
    });
    zipInput.value = '';
    zipBytes = null;
    fileName.textContent = '未选择文件（超过 5 MB）';
    convertBtn.disabled = true;
    return;
  }
  hideError();
  fileName.textContent = file.name + ' (' + sizeMB.toFixed(1) + ' MB)';
  zipFileName = file.name;
  const reader = new FileReader();
  reader.onload = (e) => {
    zipBytes = new Uint8Array(e.target.result);
    convertBtn.disabled = false;
  };
  reader.onerror = () => showError('文件读取失败');
  reader.readAsArrayBuffer(file);
});

// ---- Convert ----
convertBtn.addEventListener('click', async () => {
  if (!wasmReady || !zipBytes) {
    showError('WASM 未就绪或未选择文件');
    return;
  }
  const mainTex = mainTexInput.value.trim() || 'main-jos.tex';
  hideError();
  hideResult();
  setStatus('loading', '正在转换…');
  convertBtn.disabled = true;
  try {
    const t0 = Date.now();
    // docEngine.convert_zip_to_docx(zipBytes: Uint8Array, mainTexPath: string, _options: string)
    const result = docEngine.convert_zip_to_docx(zipBytes, mainTex, '');
    const elapsed = Date.now() - t0;
    docxBytes = new Uint8Array(result);
    if (docxBytes.length < 4 * 1024) throw new Error(`docx 过小：${docxBytes.length} bytes`);
    if (docxBytes[0] !== 0x50 || docxBytes[1] !== 0x4B) {
      throw new Error('docx 头部非 ZIP（PK\\x03\\x04）');
    }
    setStatus('ready', `完成 ${(docxBytes.length / 1024).toFixed(1)} KB（${elapsed}ms）`);
    showResult(`产物：${(docxBytes.length / 1024).toFixed(1)} KB，耗时 ${elapsed}ms`);
  } catch (err) {
    setStatus('error', '转换失败');
    showError(err.message || String(err));
    console.error('[Doc-engine popup] convert error:', err);
  } finally {
    convertBtn.disabled = false;
  }
});

// ---- Download ----
downloadBtn.addEventListener('click', () => {
  if (!docxBytes) return;
  const blob = new Blob([docxBytes], {
    type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
  });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = (zipFileName || 'output').replace(/\.[^.]+$/, '') + '.docx';
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 30_000);
});

// ---- Boot ----
initWasm();
