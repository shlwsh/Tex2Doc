// Doc-engine MV3 Service Worker
// 职责：
// 1. 创建右键菜单 "使用 Doc-engine 转换"
// 2. 响应菜单点击：通知 popup 已就绪（popup 承担全部 WASM 逻辑）

self.addEventListener('install', () => {
  // Service Worker 安装时创建右键菜单
  chrome.contextMenus.create({
    id: 'doc-engine-open-popup',
    title: '使用 Doc-engine 转换',
    contexts: ['selection'],
  });
  console.log('[Doc-engine SW] install: context menu created');
});

self.addEventListener('activate', () => {
  console.log('[Doc-engine SW] activate');
});

chrome.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId === 'doc-engine-open-popup') {
    // 通知 popup 已就绪（popup 自己会加载 WASM）
    // popup 通过 chrome.runtime.sendMessage 等待
    chrome.runtime.sendMessage({ type: 'OPEN_POPUP', tabId: tab.id }).catch(() => {
      // popup 可能未打开，忽略
    });
  }
});

// 响应 popup 发来的握手请求
chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  if (msg.type === 'PING') {
    sendResponse({ ok: true, version: '0.1.0' });
    return true;
  }
  if (msg.type === 'WRITE_CLIPBOARD') {
    // 尝试用 clipboard API 写
    if (typeof navigator !== 'undefined' && navigator.clipboard) {
      navigator.clipboard.writeText(msg.text || '').then(() => {
        sendResponse({ ok: true });
      }).catch((e) => {
        sendResponse({ ok: false, error: String(e) });
      });
    } else {
      sendResponse({ ok: false, error: 'clipboard API unavailable in SW' });
    }
    return true; // async response
  }
  return false;
});
