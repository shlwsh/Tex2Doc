/**
 * Usage API module
 *
 * Handles usage tracking and quota management
 */

import { ApiClient } from './api-client';
import type { UsageSummary } from '@/shared/types';

const USAGE_CACHE_KEY = 'tex2doc_usage_cache';
const USAGE_CACHE_TTL = 5 * 60 * 1000; // 5 minutes

interface CachedUsage {
  usage: UsageSummary;
  cached_at: number;
}

/**
 * Get current usage summary
 */
export async function getUsage(client: ApiClient, forceRefresh = false): Promise<UsageSummary> {
  if (!forceRefresh) {
    const cached = getCachedUsage();
    if (cached && Date.now() - cached.cached_at < USAGE_CACHE_TTL) {
      return cached.usage;
    }
  }

  const usage = await client.usage();
  cacheUsage(usage);
  return usage;
}

/**
 * Check if user has quota available
 */
export async function hasQuota(client: ApiClient): Promise<boolean> {
  const usage = await getUsage(client);
  return usage.cloud_conversions_used < usage.cloud_conversions_limit || usage.count_balance > 0;
}

/**
 * Get remaining conversions
 */
export async function getRemainingConversions(client: ApiClient): Promise<number> {
  const usage = await getUsage(client);
  const monthlyRemaining = Math.max(
    0,
    usage.cloud_conversions_limit - usage.cloud_conversions_used
  );
  return monthlyRemaining + usage.count_balance;
}

/**
 * Get usage percentage
 */
export async function getUsagePercentage(client: ApiClient): Promise<number> {
  const usage = await getUsage(client);
  if (usage.cloud_conversions_limit === 0) {
    return 0;
  }
  return Math.min(100, (usage.cloud_conversions_used / usage.cloud_conversions_limit) * 100);
}

// ============================================
// Private helpers
// ============================================

function getCachedUsage(): CachedUsage | null {
  try {
    const cached = localStorage.getItem(USAGE_CACHE_KEY);
    if (cached) {
      return JSON.parse(cached);
    }
  } catch {
    // Ignore errors
  }
  return null;
}

function cacheUsage(usage: UsageSummary): void {
  try {
    const cached: CachedUsage = {
      usage,
      cached_at: Date.now(),
    };
    localStorage.setItem(USAGE_CACHE_KEY, JSON.stringify(cached));
  } catch {
    // Ignore errors
  }
}

export function clearUsageCache(): void {
  try {
    localStorage.removeItem(USAGE_CACHE_KEY);
  } catch {
    // Ignore errors
  }
}
