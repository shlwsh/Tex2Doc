import { describe, expect, it } from 'vitest';
import { canUseCloud, getTotalRemainingForUsage } from '../quota-store';
import type { UsageSummary } from '@/shared/types';

const usage = (overrides: Partial<UsageSummary> = {}): UsageSummary => ({
  plan_id: 'free_trial',
  cloud_conversions_used: 2,
  cloud_conversions_limit: 10,
  count_balance: 3,
  date_valid_until: null,
  storage_bytes_used: 0,
  storage_bytes_limit: 0,
  period_start: '',
  period_end: '',
  ...overrides,
});

describe('quota presentation rules', () => {
  it('combines preview and server-provided count balance', () => {
    expect(getTotalRemainingForUsage(usage())).toBe(11);
  });

  it('never exposes negative remaining quota', () => {
    expect(
      getTotalRemainingForUsage(usage({ cloud_conversions_used: 20, count_balance: -2 }))
    ).toBe(0);
  });

  it('requires both authentication and available quota for cloud conversion', () => {
    expect(canUseCloud(usage(), true)).toBe(true);
    expect(canUseCloud(usage(), false)).toBe(false);
    expect(canUseCloud(usage({ cloud_conversions_used: 10, count_balance: 0 }), true)).toBe(false);
  });
});
