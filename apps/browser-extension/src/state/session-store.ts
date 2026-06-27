/**
 * Session Store
 */

import type { UserProfile, UsageSummary } from '@/shared/types';
import { STORAGE_KEYS } from '@/shared/constants';

const SESSION_KEY = `${STORAGE_KEYS.SESSION}`;

export interface StoredSession {
  access_token: string;
  refresh_token: string;
  user: UserProfile;
  usage: UsageSummary | null;
  expires_at: number;
}

export async function getSession(): Promise<StoredSession | null> {
  try {
    const result = await browser.storage.local.get(SESSION_KEY);
    const session = result[SESSION_KEY] as StoredSession | undefined;

    if (!session) return null;
    if (session.expires_at && session.expires_at < Date.now()) {
      await clearSession();
      return null;
    }
    return session;
  } catch {
    return null;
  }
}

export async function saveSession(session: StoredSession): Promise<void> {
  await browser.storage.local.set({ [SESSION_KEY]: session });
}

export async function clearSession(): Promise<void> {
  await browser.storage.local.remove(SESSION_KEY);
}

export async function updateTokens(accessToken: string, refreshToken: string, expiresInMs = 3600000): Promise<void> {
  const session = await getSession();
  if (session) {
    session.access_token = accessToken;
    session.refresh_token = refreshToken;
    session.expires_at = Date.now() + expiresInMs;
    await saveSession(session);
  }
}

export async function updateSession(session: Partial<StoredSession> & { refresh_token: string }): Promise<void> {
  const current = await getSession();
  if (current) {
    await saveSession({ ...current, ...session });
  }
}

export async function updateUserProfile(user: UserProfile): Promise<void> {
  const session = await getSession();
  if (session) {
    session.user = user;
    await saveSession(session);
  }
}

export async function isLoggedIn(): Promise<boolean> {
  const session = await getSession();
  return session !== null && !!session.refresh_token;
}

export async function getAccessToken(): Promise<string | null> {
  const session = await getSession();
  return session?.access_token ?? null;
}

export async function getRefreshToken(): Promise<string | null> {
  const session = await getSession();
  return session?.refresh_token ?? null;
}

export async function getUserProfile(): Promise<UserProfile | null> {
  const session = await getSession();
  return session?.user ?? null;
}

export async function getUsage(): Promise<UsageSummary | null> {
  const session = await getSession();
  return session?.usage ?? null;
}

export function onSessionChanged(callback: (session: StoredSession | null) => void): () => void {
  const listener = (
    changes: Record<string, { oldValue?: unknown; newValue?: unknown }>,
    areaName: string
  ) => {
    if (areaName === 'local' && SESSION_KEY in changes) {
      callback(changes[SESSION_KEY].newValue as StoredSession | null);
    }
  };

  browser.storage.onChanged.addListener(listener);
  return () => browser.storage.onChanged.removeListener(listener);
}
