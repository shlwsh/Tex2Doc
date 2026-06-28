/**
 * Small set of date helpers used by the renewal / expiry UX (P2-8).
 *
 * Kept in `src/ui/` rather than `src/shared/` because formatting is purely
 * presentation; downstream code (background, state) does not depend on it.
 */

const MS_PER_DAY = 86_400_000;

export interface ParsedDate {
  date: Date | null;
  /** Whole days until `date` from `now`. Negative when already past. */
  daysUntil: number | null;
  formatted: string | null;
}

/**
 * Parse a `date_valid_until` string (ISO 8601 / YYYY-MM-DD / RFC 3339) into
 * a normalized view. `null`/empty/invalid input becomes `date: null` so
 * callers can branch on it without try/catch.
 */
export function parseValidUntil(
  raw: string | null | undefined,
  now: Date = new Date(),
  locale: 'en' | 'zh' = 'en'
): ParsedDate {
  if (!raw) return { date: null, daysUntil: null, formatted: null };
  const d = new Date(raw);
  if (Number.isNaN(d.getTime())) {
    return { date: null, daysUntil: null, formatted: null };
  }
  const diffMs = d.getTime() - now.getTime();
  const days = Math.ceil(diffMs / MS_PER_DAY);
  const formatted = d.toLocaleDateString(locale === 'zh' ? 'zh-CN' : 'en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
  return { date: d, daysUntil: days, formatted };
}

/**
 * Returns the warning bucket used by the popup banner / sidepanel card.
 *
 * - 'expired'    : already past (daysUntil < 0)
 * - 'imminent'   : within the next 7 days (inclusive of today)
 * - 'soon'       : within 30 days
 * - 'future'     : more than 30 days away, or no expiry known
 */
export type RenewalWarningBucket = 'expired' | 'imminent' | 'soon' | 'future';

export function renewalBucket(daysUntil: number | null): RenewalWarningBucket {
  if (daysUntil === null) return 'future';
  if (daysUntil < 0) return 'expired';
  if (daysUntil <= 7) return 'imminent';
  if (daysUntil <= 30) return 'soon';
  return 'future';
}