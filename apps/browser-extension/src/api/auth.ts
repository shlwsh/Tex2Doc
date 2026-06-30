/**
 * Auth module for Tex2Doc API
 *
 * Handles login, registration, session management, and token refresh
 */

import { ApiClient, createAnonymousClient } from './api-client';
import type { UserProfile, UsageSummary, Session } from '@/shared/types';
import { AuthError } from '@/shared/errors';
import { setStorageItem, removeStorageItem } from '@/browser/storage';
import { STORAGE_KEYS } from '@/shared/constants';

const SESSION_TOKEN_EXPIRY_MS = 60 * 60 * 1000; // 1 hour
const POST_AUTH_USAGE_TIMEOUT_MS = 5000;

export interface StoredSession {
  refresh_token: string;
  user: UserProfile;
  usage: UsageSummary | null;
  stored_at: number;
}

/**
 * Login with email and password
 */
export async function login(
  baseUrl: string,
  email: string,
  password: string
): Promise<Session> {
  const client = createAnonymousClient({ baseUrl });

  try {
    const auth = await client.login({ email, password });

    // Store session
    await storeSession(baseUrl, {
      refresh_token: auth.refresh_token,
      user: auth.user,
      usage: null,
      stored_at: Date.now(),
    });

    // Get usage after login
    let usage: UsageSummary | null = null;
    try {
      const authClient = new ApiClient({
        baseUrl,
        apiKey: auth.access_token,
        timeout: POST_AUTH_USAGE_TIMEOUT_MS,
      });
      usage = await authClient.usage();
    } catch {
      // Ignore usage fetch errors
    }

    return {
      access_token: auth.access_token,
      refresh_token: auth.refresh_token,
      user: auth.user,
      usage,
      expires_at: Date.now() + SESSION_TOKEN_EXPIRY_MS,
    };
  } catch (error) {
    if (error instanceof AuthError) {
      throw error;
    }
    throw new AuthError(
      error instanceof Error ? error.message : 'Login failed',
      'LOGIN_FAILED'
    );
  }
}

/**
 * Register a new account
 */
export async function register(
  baseUrl: string,
  email: string,
  password: string,
  displayName?: string
): Promise<Session> {
  const client = createAnonymousClient({ baseUrl });

  try {
    const auth = await client.register({ email, password, display_name: displayName });

    // Store session
    await storeSession(baseUrl, {
      refresh_token: auth.refresh_token,
      user: auth.user,
      usage: null,
      stored_at: Date.now(),
    });

    // Get usage after registration
    let usage: UsageSummary | null = null;
    try {
      const authClient = new ApiClient({
        baseUrl,
        apiKey: auth.access_token,
        timeout: POST_AUTH_USAGE_TIMEOUT_MS,
      });
      usage = await authClient.usage();
    } catch {
      // Ignore usage fetch errors
    }

    return {
      access_token: auth.access_token,
      refresh_token: auth.refresh_token,
      user: auth.user,
      usage,
      expires_at: Date.now() + SESSION_TOKEN_EXPIRY_MS,
    };
  } catch (error) {
    if (error instanceof AuthError) {
      throw error;
    }
    throw new AuthError(
      error instanceof Error ? error.message : 'Registration failed',
      'REGISTRATION_FAILED'
    );
  }
}

/**
 * Refresh the access token
 */
export async function refreshSession(baseUrl: string): Promise<Session> {
  const stored = await getStoredSession();
  if (!stored?.refresh_token) {
    throw new AuthError('No refresh token available', 'NO_REFRESH_TOKEN');
  }

  const client = createAnonymousClient({ baseUrl });

  try {
    const auth = await client.refresh({ refresh_token: stored.refresh_token });

    // Update stored session
    await storeSession(baseUrl, {
      refresh_token: auth.refresh_token,
      user: auth.user,
      usage: stored.usage,
      stored_at: Date.now(),
    });

    // Get updated usage
    let usage: UsageSummary | null = stored.usage;
    try {
      const authClient = new ApiClient({ baseUrl, apiKey: auth.access_token });
      usage = await authClient.usage();
    } catch {
      // Ignore usage fetch errors
    }

    return {
      access_token: auth.access_token,
      refresh_token: auth.refresh_token,
      user: auth.user,
      usage,
      expires_at: Date.now() + SESSION_TOKEN_EXPIRY_MS,
    };
  } catch (error) {
    // Clear session on refresh failure
    await logout(baseUrl);
    throw new AuthError(
      error instanceof Error ? error.message : 'Session refresh failed',
      'REFRESH_FAILED'
    );
  }
}

/**
 * Get current session
 */
export async function getSession(baseUrl: string): Promise<Session | null> {
  const stored = await getStoredSession();
  if (!stored) {
    return null;
  }

  // Check if stored session is too old (7 days)
  const maxAge = 7 * 24 * 60 * 60 * 1000;
  if (Date.now() - stored.stored_at > maxAge) {
    await logout(baseUrl);
    return null;
  }

  try {
    // Refresh if needed
    return await refreshSession(baseUrl);
  } catch {
    // Return stored session without refresh
    return {
      access_token: '',
      refresh_token: stored.refresh_token,
      user: stored.user,
      usage: stored.usage,
      expires_at: 0,
    };
  }
}

/**
 * Logout and clear session
 */
export async function logout(_baseUrl: string): Promise<void> {
  await removeStorageItem(STORAGE_KEYS.SESSION);
}

/**
 * Check if user is logged in
 */
export async function isLoggedIn(): Promise<boolean> {
  const stored = await getStoredSession();
  return stored !== null && !!stored.refresh_token;
}

// ============================================
// Private helpers
// ============================================

async function storeSession(baseUrl: string, session: StoredSession): Promise<void> {
  const key = `${STORAGE_KEYS.SESSION}_${new URL(baseUrl).host}`;
  await setStorageItem(key, session);
}

async function getStoredSession(): Promise<StoredSession | null> {
  // Try all possible keys
  const items = await browser.storage.local.get(null);
  const keys = Object.keys(items).filter((k) =>
    k.startsWith(STORAGE_KEYS.SESSION)
  );

  for (const key of keys) {
    const session = items[key] as StoredSession | undefined;
    if (session?.refresh_token) {
      return session;
    }
  }

  return null;
}
