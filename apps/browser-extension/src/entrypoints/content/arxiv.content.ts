/**
 * arXiv Content Script
 */

(() => {
  const ARXIV_DOMAINS = ['arxiv.org', 'export.arxiv.org'];
  const BUTTON_ID = 'tex2doc-arxiv-btn';

  function isArxivPage(): boolean {
    return ARXIV_DOMAINS.some((domain) => window.location.hostname.endsWith(domain));
  }

  function isAbstractPage(): boolean {
    return window.location.pathname.includes('/abs/');
  }

  function getPaperId(): string | null {
    const match = window.location.pathname.match(/\/abs\/([0-9.]+)/);
    return match ? match[1] : null;
  }

  function getPaperTitle(): string {
    const metaTitle = document.querySelector('meta[name="citation_title"]');
    if (metaTitle) return metaTitle.getAttribute('content') || 'Unknown Paper';
    const h1 = document.querySelector('.title.mathjax');
    if (h1) return h1.textContent?.replace(/\s+/g, ' ').trim() || 'Unknown Paper';
    return 'Unknown Paper';
  }

  function createDownloadButton(): HTMLElement {
    const container = document.createElement('div');
    container.id = BUTTON_ID;

    const button = document.createElement('button');
    button.innerHTML = `<span>Download & Convert with Tex2Doc</span>`;
    button.style.cssText = `
      display: inline-flex; align-items: center; gap: 8px; padding: 10px 20px;
      background: linear-gradient(135deg, #0ea5e9 0%, #0284c7 100%); color: white; border: none;
      border-radius: 8px; font-size: 14px; font-weight: 500; cursor: pointer;
      transition: all 0.2s; box-shadow: 0 2px 8px rgba(14, 165, 233, 0.3);
    `;

    button.addEventListener('click', handleDownload);
    container.appendChild(button);
    return container;
  }

  function findDownloadContainer(): HTMLElement | null {
    const selectors = ['#download-button', '.download-button', '[data-download-btn]', '.arxiv-button-box'];
    for (const selector of selectors) {
      const el = document.querySelector(selector);
      if (el) return el as HTMLElement;
    }
    return null;
  }

  async function handleDownload(): Promise<void> {
    const paperId = getPaperId();
    const paperTitle = getPaperTitle();

    if (!paperId) {
      showNotification('Could not detect paper ID', 'error');
      return;
    }

    try {
      const btn = document.querySelector(`#${BUTTON_ID} button`) as HTMLButtonElement;
      if (btn) { btn.disabled = true; btn.textContent = 'Preparing...'; }

      (browser as any).runtime.sendMessage({
        type: 'CONTENT_SCRIPT_CONVERT',
        context: 'arxiv',
        paperId,
        paperTitle,
        url: window.location.href,
      });

      showNotification('Download started!', 'success');
    } catch (error) {
      console.error('[Tex2Doc] Failed to start download:', error);
      showNotification('Failed to start download.', 'error');
      const btn = document.querySelector(`#${BUTTON_ID} button`) as HTMLButtonElement;
      if (btn) { btn.disabled = false; btn.textContent = 'Download & Convert with Tex2Doc'; }
    }
  }

  function showNotification(message: string, type: 'success' | 'error' = 'success'): void {
    const existing = document.getElementById('tex2doc-notification');
    if (existing) existing.remove();

    const bgColor = type === 'success' ? '#059669' : '#dc2626';
    const notification = document.createElement('div');
    notification.innerHTML = `
      <div style="position: fixed; top: 20px; right: 20px; background: ${bgColor}; color: white;
        padding: 12px 20px; border-radius: 8px; font-size: 14px; z-index: 10000;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);">
        ${message}
      </div>
    `;
    document.body.appendChild(notification);
    setTimeout(() => notification.remove(), 3000);
  }

  function init(): void {
    if (!isArxivPage() || !isAbstractPage()) return;

    const tryInsert = () => {
      const container = findDownloadContainer();
      if (container) {
        container.appendChild(createDownloadButton());
        console.log('[Tex2Doc] arXiv download button added');
      } else if (document.readyState !== 'complete') {
        setTimeout(tryInsert, 500);
      }
    };

    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', tryInsert);
    } else {
      tryInsert();
    }

    console.log('[Tex2Doc] arXiv content script initialized');
  }

  init();
})();
