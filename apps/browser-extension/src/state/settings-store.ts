import type { ExtensionSettings } from '@/shared/types';
import { STORAGE_KEYS, API_BASE_URL } from '@/shared/constants';
import { supportsSyncStorage } from '@/browser/compat';

const SETTINGS_KEY = `${STORAGE_KEYS.SETTINGS}`;

export const DEFAULT_SETTINGS: ExtensionSettings = {
  api_base_url: API_BASE_URL,
  default_profile: 'standard',
  default_quality: 'balanced',
  default_mode: 'auto',
  wasm_file_size_limit: 10 * 1024 * 1024,
  language: 'en',
  theme: 'system',
  polling_interval: 2000,
};

export async function getSettings(): Promise<ExtensionSettings> {
  try {
    const result = await browser.storage.sync.get(SETTINGS_KEY);
    const settings = result[SETTINGS_KEY] as Partial<ExtensionSettings> | undefined;
    if (!settings) return { ...DEFAULT_SETTINGS };
    return { ...DEFAULT_SETTINGS, ...settings };
  } catch {
    try {
      const result = await browser.storage.local.get(SETTINGS_KEY);
      const settings = result[SETTINGS_KEY] as Partial<ExtensionSettings> | undefined;
      return { ...DEFAULT_SETTINGS, ...settings };
    } catch {
      return { ...DEFAULT_SETTINGS };
    }
  }
}

export async function saveSettings(settings: Partial<ExtensionSettings>): Promise<void> {
  const current = await getSettings();
  const updated = { ...current, ...settings };
  const storage = supportsSyncStorage() ? browser.storage.sync : browser.storage.local;
  await storage.set({ [SETTINGS_KEY]: updated });
}

export async function resetSettings(): Promise<void> {
  try {
    if (supportsSyncStorage()) await browser.storage.sync.remove(SETTINGS_KEY);
  } catch {}
  await browser.storage.local.remove(SETTINGS_KEY);
}

export async function getSetting<K extends keyof ExtensionSettings>(key: K): Promise<ExtensionSettings[K]> {
  const settings = await getSettings();
  return settings[key];
}

export async function updateSetting<K extends keyof ExtensionSettings>(key: K, value: ExtensionSettings[K]): Promise<void> {
  await saveSettings({ [key]: value });
}

export async function getApiBaseUrl(): Promise<string> {
  return getSetting('api_base_url');
}

export async function setApiBaseUrl(url: string): Promise<void> {
  await updateSetting('api_base_url', url);
}

export async function getLanguage(): Promise<'en' | 'zh'> {
  return getSetting('language');
}

export async function setLanguage(language: 'en' | 'zh'): Promise<void> {
  await updateSetting('language', language);
}

export async function getTheme(): Promise<'light' | 'dark' | 'system'> {
  return getSetting('theme');
}

export async function setTheme(theme: 'light' | 'dark' | 'system'): Promise<void> {
  await updateSetting('theme', theme);
}

export function onSettingsChanged(callback: (settings: ExtensionSettings) => void): () => void {
  const listener = async (changes: Record<string, { oldValue?: unknown; newValue?: unknown }>) => {
    if (SETTINGS_KEY in changes) {
      const settings = await getSettings();
      callback(settings);
    }
  };
  browser.storage.onChanged.addListener(listener);
  return () => browser.storage.onChanged.removeListener(listener);
}
