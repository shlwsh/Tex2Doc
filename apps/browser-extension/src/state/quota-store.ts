/**
 * Quota Store
 *
 * Manages usage quota state in memory with persistence
 */

import type { UsageSummary } from '@/shared/types';
import { getSession } from './session-store';

let cachedUsage: UsageSummary | null = null;
let lastFetchTime: number = 0;
const CACHE_TTL = 5 * 60 * 1000; // 5 minutes

/**
 * Get cached usage
 */
export function getCachedUsage(): UsageSummary | null {
  return cachedUsage;
}

/**
 * Update cached usage
 */
export function setCachedUsage(usage: UsageSummary | null): void {
  cachedUsage = usage;
  lastFetchTime = Date.now();
}

/**
 * Check if cache is still valid
 */
export function isCacheValid(): boolean {
  return cachedUsage !== null && Date.now() - lastFetchTime < CACHE_TTL;
}

/**
 * Get remaining monthly conversions
 */
export function getRemainingMonthly(): number {
  if (!cachedUsage) return 0;
  return Math.max(0, cachedUsage.cloud_conversions_limit - cachedUsage.cloud_conversions_used);
}

/**
 * Get count balance
 */
export function getCountBalance(): number {
  return cachedUsage?.count_balance ?? 0;
}

/**
 * Get total remaining (monthly + count)
 */
export function getTotalRemaining(): number {
  return getRemainingMonthly() + getCountBalance();
}

export function getTotalRemainingForUsage(usage: UsageSummary | null): number {
  if (!usage) return 0;
  return (
    Math.max(0, usage.cloud_conversions_limit - usage.cloud_conversions_used) +
    Math.max(0, usage.count_balance)
  );
}

export function canUseCloud(usage: UsageSummary | null, signedIn: boolean): boolean {
  return signedIn && getTotalRemainingForUsage(usage) > 0;
}

/**
 * Check if user has quota available
 */
export function hasQuota(): boolean {
  return getTotalRemaining() > 0;
}

/**
 * Get usage percentage
 */
export function getUsagePercentage(): number {
  if (!cachedUsage || cachedUsage.cloud_conversions_limit === 0) {
    return 0;
  }
  return Math.min(
    100,
    (cachedUsage.cloud_conversions_used / cachedUsage.cloud_conversions_limit) * 100
  );
}

/**
 * Get quota display text
 */
export function getQuotaDisplay(): string {
  if (!cachedUsage) {
    return '--';
  }

  const remaining = getTotalRemaining();
  const used = cachedUsage.cloud_conversions_used;
  const limit = cachedUsage.cloud_conversions_limit;

  if (remaining === 0) {
    return `Quota exceeded (${used}/${limit})`;
  }

  if (cachedUsage.count_balance > 0) {
    return `${remaining} (${used}/${limit} + ${cachedUsage.count_balance} bonus)`;
  }

  return `${remaining} remaining (${used}/${limit})`;
}

/**
 * Increment usage counter (optimistic update)
 */
export async function incrementUsage(): Promise<void> {
  if (!cachedUsage) return;

  const updated: UsageSummary = {
    ...cachedUsage,
    cloud_conversions_used: cachedUsage.cloud_conversions_used + 1,
  };

  cachedUsage = updated;
}

/**
 * Refresh quota from session
 */
export async function refreshFromSession(): Promise<void> {
  const session = await getSession();
  if (session?.usage) {
    cachedUsage = session.usage;
    lastFetchTime = Date.now();
  }
}

/**
 * Reset quota cache
 */
export function resetQuotaCache(): void {
  cachedUsage = null;
  lastFetchTime = 0;
}

/**
 * Format quota for display
 */
export function formatQuota(usage: UsageSummary | null): string {
  if (!usage) {
    return '--';
  }

  const remaining = getTotalRemainingForUsage(usage);

  return `${remaining} / ${usage.cloud_conversions_limit} (+${usage.count_balance})`;
}

/**
 * Get quota color (for UI)
 */
export function getQuotaColor(): 'green' | 'yellow' | 'red' {
  const percentage = getUsagePercentage();

  if (percentage >= 100) return 'red';
  if (percentage >= 80) return 'yellow';
  return 'green';
}
