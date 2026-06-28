/**
 * Domain Permissions Store
 *
 * Persists user-approved domain list and keeps the UI in sync with
 * `chrome.permissions.getAll()` so that toggling in Options reflects the
 * real grant state (P0-5).
 *
 * Notes
 *  - `chrome.permissions.request` returns true only when the user accepts
 *    the native prompt. A rejection must roll back the local list, otherwise
 *    the UI shows "enabled" while the browser reports "denied".
 *  - We deliberately keep this in `storage.local` (not sync). Permissions
 *    are device-specific and surfacing them across machines would confuse
 *    users who only approved one device.
 */

import { STORAGE_KEYS } from '@/shared/constants';

const DOMAIN_KEY = `${STORAGE_KEYS.SETTINGS}.domain_permissions`;

export interface PersistedDomain {
  id: string;
  domain: string;
  /** Whether the user wants this domain enabled. The OS may still reject. */
  enabled: boolean;
  /** Reflects the last known `chrome.permissions.contains` state. */
  granted: boolean;
  updatedAt: number;
}

export function toOriginPattern(domain: string): string {
  const d = domain.trim().toLowerCase();
  if (d.startsWith('http://') || d.startsWith('https://')) {
    return d.includes('*') ? d : `${d.replace(/\/$/, '')}/*`;
  }
  return `https://${d}/*`;
}

export async function getDomains(): Promise<PersistedDomain[]> {
  const result = await browser.storage.local.get(DOMAIN_KEY);
  const list = result[DOMAIN_KEY] as PersistedDomain[] | undefined;
  return Array.isArray(list) ? list : [];
}

export async function saveDomains(domains: PersistedDomain[]): Promise<void> {
  await browser.storage.local.set({ [DOMAIN_KEY]: domains });
}

/**
 * Reconcile local list with the browser's real permission state. Used on
 * Options open and after every grant/remove action.
 */
export async function refreshGrantedFlags(domains: PersistedDomain[]): Promise<PersistedDomain[]> {
  const reconciled: PersistedDomain[] = [];
  for (const d of domains) {
    let granted = false;
    try {
      granted = await browser.permissions.contains({ origins: [toOriginPattern(d.domain)] });
    } catch {
      granted = false;
    }
    reconciled.push({ ...d, granted });
  }
  return reconciled;
}