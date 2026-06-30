import { beforeEach, describe, expect, it } from 'vitest';
import { QUICK_CODE_KEY, storedQuickCode, useSessionStore } from './session';

describe('session store', () => {
  beforeEach(() => {
    localStorage.clear();
    useSessionStore.getState().setQuickSession(undefined);
  });

  it('persists quick redeem code separately for automatic restore', () => {
    useSessionStore.getState().setQuickSession({
      apiBaseUrl: 'http://127.0.0.1:2624/v1/',
      accessToken: 'token',
      user: { email: 'code' },
      redeemCode: 'CODE-123',
    });

    expect(localStorage.getItem(QUICK_CODE_KEY)).toBe('CODE-123');
    expect(storedQuickCode()).toBe('CODE-123');
  });
});
