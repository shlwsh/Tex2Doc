// Doc-engine MV3 Content Script
// 在 Overleaf / arXiv 页面注入：监听 selectionchange，缓存当前选区文本。
// popup 打开时可读取缓存（通过 chrome.storage.session）。

(function () {
  // 仅在特定域名运行
  const host = window.location.hostname;
  if (!host.endsWith('.overleaf.com') && !host.endsWith('.arxiv.org')) {
    return;
  }

  // 监听选区变化
  document.addEventListener('selectionchange', () => {
    const sel = window.getSelection();
    const text = sel ? sel.toString().trim() : '';
    if (text.length > 0) {
      chrome.storage.session.set({ selectedText: text }).catch(() => {
        // storage.session 可能不可用（Manifest V3 要求 Chrome 120+）
      });
    }
  });

  // 通知 background 已就绪
  chrome.runtime.sendMessage({ type: 'CONTENT_SCRIPT_READY', url: window.location.href }).catch(() => {});
})();
