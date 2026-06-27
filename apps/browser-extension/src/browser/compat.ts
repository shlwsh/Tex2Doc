/**
 * Browser Compatibility Layer
 */

export type BrowserName = 'chrome' | 'firefox' | 'safari' | 'edge' | 'unknown';

/**
 * Detect current browser
 */
export function getBrowserName(): BrowserName {
  const ua = navigator.userAgent.toLowerCase();

  if (ua.includes('edg/')) return 'edge';
  if (ua.includes('chrome/') && !ua.includes('chromium')) return 'chrome';
  if (ua.includes('firefox/')) return 'firefox';
  if (ua.includes('safari/') && !ua.includes('chrome')) return 'safari';

  return 'unknown';
}

/**
 * Check if current browser supports sidePanel API
 */
export function supportsSidePanel(): boolean {
  const browserName = getBrowserName();
  return browserName === 'chrome' || browserName === 'edge';
}

/**
 * Check if browser.storage.sync is available
 */
export function supportsSyncStorage(): boolean {
  const browserName = getBrowserName();
  return browserName !== 'safari';
}

/**
 * Wrap a potentially async operation with retry logic
 */
export async function withRetry<T>(
  operation: () => Promise<T>,
  options: {
    maxRetries?: number;
    initialDelayMs?: number;
    maxDelayMs?: number;
    backoffMultiplier?: number;
    shouldRetry?: (error: unknown) => boolean;
  } = {}
): Promise<T> {
  const {
    maxRetries = 3,
    initialDelayMs = 1000,
    maxDelayMs = 10000,
    backoffMultiplier = 2,
    shouldRetry = () => true,
  } = options;

  let lastError: unknown;
  let delay = initialDelayMs;

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      return await operation();
    } catch (error) {
      lastError = error;

      if (attempt === maxRetries || !shouldRetry(error)) {
        throw error;
      }

      await sleep(delay);
      delay = Math.min(delay * backoffMultiplier, maxDelayMs);
    }
  }

  throw lastError;
}

/**
 * Sleep for specified milliseconds
 */
export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Wrap browser.runtime.sendMessage with proper typing
 */
export async function sendMessage<T = unknown>(message: Record<string, unknown>): Promise<T> {
  return browser.runtime.sendMessage(message) as Promise<T>;
}

/**
 * Send message to specific tab
 */
export async function sendMessageToTab<T = unknown>(
  tabId: number,
  message: Record<string, unknown>
): Promise<T> {
  return browser.tabs.sendMessage(tabId, message) as Promise<T>;
}

/**
 * Get current tab
 */
export async function getCurrentTab(): Promise<browser.Tabs.Tab | null> {
  const [tab] = await browser.tabs.query({ active: true, currentWindow: true });
  return tab || null;
}

/**
 * Open a new tab with optional URL
 */
export async function openTab(url?: string, active = true): Promise<browser.Tabs.Tab | null> {
  if (url) {
    return browser.tabs.create({ url, active });
  }
  return null;
}

/**
 * Open URL in new tab
 */
export async function openUrl(url: string): Promise<void> {
  await browser.tabs.create({ url, active: true });
}

/**
 * Create a blob URL for download
 */
export function createBlobUrl(data: Uint8Array | Blob, mimeType: string): string {
  const blob = data instanceof Blob ? data : new Blob([new Uint8Array(data)], { type: mimeType });
  return URL.createObjectURL(blob);
}

/**
 * Revoke a blob URL
 */
export function revokeBlobUrl(url: string): void {
  URL.revokeObjectURL(url);
}
