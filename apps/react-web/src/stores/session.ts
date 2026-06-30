import { create } from 'zustand';
import { defaultApiBaseUrl } from '../api/http';
import type { UsageSummary, UserProfile } from '../api/types';

export const USER_AUTH_KEY = 'tex2doc.user.auth';
export const QUICK_CODE_KEY = 'tex2doc.quick.redeemCode';
export const QUICK_AUTH_KEY = 'tex2doc.quick.auth';
export const ADMIN_AUTH_KEY = 'tex2doc.admin.auth';
export const PREFERENCE_KEY = 'tex2doc.react.preferences';

export interface AuthSession {
  apiBaseUrl: string;
  accessToken: string;
  refreshToken?: string;
  user: UserProfile;
  usage?: UsageSummary;
}

interface SessionState {
  userSession?: AuthSession;
  quickSession?: AuthSession & { redeemCode: string };
  adminSession?: AuthSession;
  theme: 'light' | 'dark';
  locale: 'zh-CN' | 'en-US';
  setUserSession: (session?: AuthSession) => void;
  setQuickSession: (session?: AuthSession & { redeemCode: string }) => void;
  setAdminSession: (session?: AuthSession) => void;
  setPreferences: (theme: 'light' | 'dark', locale: 'zh-CN' | 'en-US') => void;
}

function readJson<T>(key: string): T | undefined {
  try {
    const value = localStorage.getItem(key);
    return value ? (JSON.parse(value) as T) : undefined;
  } catch {
    return undefined;
  }
}

function writeJson(key: string, value: unknown): void {
  if (value === undefined) {
    localStorage.removeItem(key);
  } else {
    localStorage.setItem(key, JSON.stringify(value));
  }
}

const prefs = readJson<{ theme?: 'light' | 'dark'; locale?: 'zh-CN' | 'en-US' }>(PREFERENCE_KEY);

export const useSessionStore = create<SessionState>((set) => ({
  userSession: readJson<AuthSession>(USER_AUTH_KEY),
  quickSession: readJson<AuthSession & { redeemCode: string }>(QUICK_AUTH_KEY),
  adminSession: readJson<AuthSession>(ADMIN_AUTH_KEY),
  theme: prefs?.theme ?? 'light',
  locale: prefs?.locale ?? 'zh-CN',
  setUserSession: (session) => {
    writeJson(USER_AUTH_KEY, session);
    set({ userSession: session });
  },
  setQuickSession: (session) => {
    writeJson(QUICK_AUTH_KEY, session);
    if (session?.redeemCode) {
      localStorage.setItem(QUICK_CODE_KEY, session.redeemCode);
    } else {
      localStorage.removeItem(QUICK_CODE_KEY);
    }
    set({ quickSession: session });
  },
  setAdminSession: (session) => {
    writeJson(ADMIN_AUTH_KEY, session);
    set({ adminSession: session });
  },
  setPreferences: (theme, locale) => {
    writeJson(PREFERENCE_KEY, { theme, locale });
    document.documentElement.dataset.theme = theme;
    set({ theme, locale });
  },
}));

export function storedQuickCode(): string {
  return localStorage.getItem(QUICK_CODE_KEY) ?? '';
}

export function initialApiBaseUrl(): string {
  return (
    readJson<AuthSession>(USER_AUTH_KEY)?.apiBaseUrl ??
    readJson<AuthSession>(ADMIN_AUTH_KEY)?.apiBaseUrl ??
    defaultApiBaseUrl()
  );
}
