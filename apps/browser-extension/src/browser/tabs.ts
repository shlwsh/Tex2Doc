/**
 * Tabs utilities
 */

import type Browser from 'webextension-polyfill';

export interface TabInfo {
  id: number;
  url: string;
  title: string;
  active: boolean;
  pinned: boolean;
}

/**
 * Get current active tab
 */
export async function getCurrentTab(): Promise<Browser.Tabs.Tab | null> {
  const [tab] = await browser.tabs.query({ active: true, currentWindow: true });
  return tab || null;
}

/**
 * Get all tabs
 */
export async function getAllTabs(): Promise<Browser.Tabs.Tab[]> {
  return browser.tabs.query({});
}

/**
 * Create a new tab
 */
export async function createTab(options: {
  url?: string;
  active?: boolean;
  pinned?: boolean;
  index?: number;
}): Promise<Browser.Tabs.Tab> {
  return browser.tabs.create(options);
}

/**
 * Update a tab
 */
export async function updateTab(
  tabId: number,
  options: {
    url?: string;
    active?: boolean;
    pinned?: boolean;
    highlighted?: boolean;
  }
): Promise<Browser.Tabs.Tab | null> {
  return browser.tabs.update(tabId, options);
}

/**
 * Close a tab
 */
export async function closeTab(tabId: number): Promise<void> {
  await browser.tabs.remove(tabId);
}

/**
 * Reload a tab
 */
export async function reloadTab(tabId: number, bypassCache = false): Promise<void> {
  await browser.tabs.reload(tabId, { bypassCache });
}

/**
 * Get tab info
 */
export async function getTabInfo(tabId: number): Promise<TabInfo | null> {
  const tab = await browser.tabs.get(tabId).catch(() => null);
  if (!tab) return null;

  return {
    id: tab.id!,
    url: tab.url || '',
    title: tab.title || '',
    active: tab.active,
    pinned: tab.pinned,
  };
}

/**
 * Check if URL matches pattern
 */
export function urlMatchesPattern(url: string, pattern: string): boolean {
  try {
    const regex = pattern.replace(/\./g, '\\.').replace(/\*/g, '.*').replace(/\?/g, '.');
    return new RegExp(`^${regex}$`, 'i').test(url);
  } catch {
    return false;
  }
}

/**
 * Check if tab URL matches host permission
 */
export async function hasHostPermission(url: string): Promise<boolean> {
  try {
    const origin = new URL(url).origin;
    return await browser.permissions.contains({ origins: [origin] });
  } catch {
    return false;
  }
}

/**
 * Get tab by URL
 */
export async function getTabByUrl(url: string): Promise<Browser.Tabs.Tab | null> {
  const tabs = await browser.tabs.query({ url });
  return tabs[0] || null;
}

/**
 * Highlight a tab
 */
export async function highlightTab(tabId: number): Promise<void> {
  await browser.tabs.update(tabId, { active: true, highlighted: true });
}
