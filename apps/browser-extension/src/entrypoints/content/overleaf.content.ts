/**
 * Overleaf Content Script
 */

(() => {
  const OVERLEAF_DOMAINS = ['overleaf.com', 'overleaf-staging.com'];
  const BUTTON_ID = 'tex2doc-overleaf-btn';

  function isOverleafPage(): boolean {
    return OVERLEAF_DOMAINS.some((domain) => window.location.hostname.endsWith(domain));
  }

  function isProjectPage(): boolean {
    return window.location.pathname.includes('/project/');
  }

  function getProjectName(): string {
    const title = document.title.replace(' - Overleaf', '').trim();
    if (title) return title;
    const pathParts = window.location.pathname.split('/');
    return pathParts[pathParts.length - 1] || 'project';
  }

  function createFloatingButton(): HTMLElement {
    const container = document.createElement('div');
    container.id = BUTTON_ID;

    const button = document.createElement('button');
    button.innerHTML = `
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
        <polyline points="14 2 14 8 20 8"></polyline>
        <line x1="16" y1="13" x2="8" y2="13"></line>
        <line x1="16" y1="17" x2="8" y2="17"></line>
      </svg>
    `;
    button.style.cssText = `
      position: fixed; bottom: 24px; right: 24px; width: 56px; height: 56px; border-radius: 50%;
      background: linear-gradient(135deg, #0ea5e9 0%, #0284c7 100%); color: white; border: none;
      cursor: pointer; box-shadow: 0 4px 12px rgba(14, 165, 233, 0.4); z-index: 9999;
      display: flex; align-items: center; justify-content: center;
      transition: transform 0.2s, box-shadow 0.2s;
    `;

    button.addEventListener('click', handleButtonClick);
    button.addEventListener('mouseenter', () => {
      button.style.transform = 'scale(1.1)';
      button.style.boxShadow = '0 6px 16px rgba(14, 165, 233, 0.5)';
    });
    button.addEventListener('mouseleave', () => {
      button.style.transform = 'scale(1)';
      button.style.boxShadow = '0 4px 12px rgba(14, 165, 233, 0.4)';
    });

    container.appendChild(button);
    return container;
  }

  async function handleButtonClick(): Promise<void> {
    try {
      const projectName = getProjectName();
      browser.runtime.sendMessage({
        type: 'CONTENT_SCRIPT_CONVERT',
        context: 'overleaf',
        projectName,
        url: window.location.href,
      });
      showNotification(`Converting "${projectName}" with Tex2Doc...`);
    } catch (error) {
      console.error('[Tex2Doc] Failed to initiate conversion:', error);
      showNotification('Failed to start conversion.');
    }
  }

  function showNotification(message: string): void {
    const existing = document.getElementById('tex2doc-notification');
    if (existing) existing.remove();

    const notification = document.createElement('div');
    notification.innerHTML = `
      <div style="position: fixed; bottom: 90px; right: 24px; background: #1f2937; color: white;
        padding: 12px 20px; border-radius: 8px; font-size: 14px; z-index: 10000;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2); animation: slideUp 0.3s ease-out;">
        ${message}
      </div>
    `;
    document.body.appendChild(notification);
    setTimeout(() => { notification.remove(); }, 3000);
  }

  function init(): void {
    if (!isOverleafPage() || !isProjectPage()) return;

    const style = document.createElement('style');
    style.textContent = `@keyframes slideUp { from { opacity: 0; transform: translateY(20px); } to { opacity: 1; transform: translateY(0); } }`;
    document.head.appendChild(style);

    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', () => document.body.appendChild(createFloatingButton()));
    } else {
      document.body.appendChild(createFloatingButton());
    }

    console.log('[Tex2Doc] Overleaf content script initialized');
  }

  init();
})();
