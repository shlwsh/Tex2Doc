/**
 * Generic Content Script
 *
 * Fallback content script for pages without specific handlers.
 * Can be used to provide minimal functionality on any page.
 */

(() => {
  // Check if we should run
  function shouldRun(): boolean {
    // Don't run on file:// URLs
    if (window.location.protocol === 'file:') return false;

    // Don't run on browser internal pages
    if (window.location.hostname === '') return false;

    return true;
  }

  // Handle text selection
  function handleTextSelection(): void {
    const selection = window.getSelection();
    if (!selection || selection.isCollapsed) return;

    const selectedText = selection.toString().trim();
    if (selectedText.length === 0) return;

    // Could be used to detect LaTeX content
    if (selectedText.includes('\\') || selectedText.includes('begin{')) {
      // LaTeX-like content detected
      console.log('[Tex2Doc] Potential LaTeX content detected:', selectedText.slice(0, 100));
    }
  }

  // Initialize
  function init(): void {
    if (!shouldRun()) return;

    document.addEventListener('mouseup', handleTextSelection);
    console.log('[Tex2Doc] Generic content script initialized on:', window.location.hostname);
  }

  // Run
  init();
})();
