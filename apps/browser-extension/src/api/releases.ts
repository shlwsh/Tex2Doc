/**
 * API Client for Tex2Doc Commercial API
 */

import type { Browser } from 'webextension-polyfill';

/**
 * Get extension version
 */
export function getExtensionVersion(): string {
  return (browser as Browser).runtime.getManifest().version;
}

/**
 * Get extension ID
 */
export function getExtensionId(): string {
  return (browser as Browser).runtime.id;
}

/**
 * Check for extension updates
 */
export async function checkForUpdate(): Promise<boolean> {
  const [result] = await (browser as Browser).runtime.requestUpdateCheck();
  return result === 'update_available';
}

/**
 * Reload extension to apply update
 */
export function reloadExtension(): void {
  (browser as Browser).runtime.reload();
}

/**
 * Open extension store page
 */
export async function openStorePage(): Promise<void> {
  const browserName = getBrowserName();

  const storeUrls: Record<string, string> = {
    chrome: 'https://chrome.google.com/webstore',
    edge: 'https://microsoftedge.microsoft.com/addons',
    firefox: 'https://addons.mozilla.org',
    safari: 'https://apps.apple.com',
  };

  const url = storeUrls[browserName] ?? 'https://tex2doc.cn';
  await (browser as Browser).tabs.create({ url, active: true });
}

function getBrowserName(): string {
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes('edg/')) return 'edge';
  if (ua.includes('chrome/')) return 'chrome';
  if (ua.includes('firefox/')) return 'firefox';
  if (ua.includes('safari/')) return 'safari';
  return 'unknown';
}

/**
 * Get update available notification
 */
export function getUpdateNotification(): string | null {
  return `A new version of Tex2Doc is available!`;
}
