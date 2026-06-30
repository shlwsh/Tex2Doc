/**
 * Manifest utilities
 */

/**
 * Get extension version from manifest
 */
export async function getExtensionVersion(): Promise<string> {
  return browser.runtime.getManifest().version;
}

/**
 * Get extension ID
 */
export function getExtensionId(): string {
  return browser.runtime.id;
}

/**
 * Check if extension is installed
 */
export async function isExtensionInstalled(): Promise<boolean> {
  try {
    await browser.runtime.getPlatformInfo();
    return true;
  } catch {
    return false;
  }
}

/**
 * Get platform info
 */
export async function getPlatformInfo() {
  return browser.runtime.getPlatformInfo();
}

/**
 * Check if running in development mode
 */
export function isDevelopmentMode(): boolean {
  return browser.runtime.id?.includes('..') || browser.runtime.id?.endsWith('.crx') === false;
}
