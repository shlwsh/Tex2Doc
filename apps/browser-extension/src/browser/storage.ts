/**
 * Storage utilities
 */

const DEFAULT_STORAGE_AREA: 'local' | 'sync' = 'local';

/**
 * Get item from storage
 */
export async function getStorageItem<T>(
  key: string,
  area: 'local' | 'sync' = DEFAULT_STORAGE_AREA
): Promise<T | null> {
  const result = await browser.storage[area].get(key);
  return (result[key] as T) ?? null;
}

/**
 * Set item to storage
 */
export async function setStorageItem<T>(
  key: string,
  value: T,
  area: 'local' | 'sync' = DEFAULT_STORAGE_AREA
): Promise<void> {
  await browser.storage[area].set({ [key]: value });
}

/**
 * Remove item from storage
 */
export async function removeStorageItem(
  key: string,
  area: 'local' | 'sync' = DEFAULT_STORAGE_AREA
): Promise<void> {
  await browser.storage[area].remove(key);
}

/**
 * Clear all items from storage area
 */
export async function clearStorage(area: 'local' | 'sync' = DEFAULT_STORAGE_AREA): Promise<void> {
  await browser.storage[area].clear();
}

/**
 * Get multiple items from storage
 */
export async function getStorageItems<T extends Record<string, unknown>>(
  keys: (keyof T)[],
  area: 'local' | 'sync' = DEFAULT_STORAGE_AREA
): Promise<T> {
  const result = await browser.storage[area].get(keys as string[]);
  return result as T;
}

/**
 * Set multiple items to storage
 */
export async function setStorageItems<T extends Record<string, unknown>>(
  items: T,
  area: 'local' | 'sync' = DEFAULT_STORAGE_AREA
): Promise<void> {
  await browser.storage[area].set(items as Record<string, unknown>);
}

/**
 * Subscribe to storage changes
 */
export function onStorageChanged(
  callback: (changes: Record<string, { oldValue?: unknown; newValue?: unknown }>, areaName: string) => void
): () => void {
  browser.storage.onChanged.addListener(callback);
  return () => {
    browser.storage.onChanged.removeListener(callback);
  };
}
