/**
 * RenewalHint — surfaces the `date_valid_until` field from UsageSummary as a
 * color-coded badge / banner (P2-8).
 *
 * Three render variants:
 *  - "banner"  : top-of-card callout; used in Account panel + popup header.
 *  - "inline"  : compact pill rendered next to "Count Balance".
 *  - "muted"   : small text inside the usage card when the date is far away.
 */

import React from 'react';
import { useI18n } from '@/ui/i18n/useI18n';
import {
  parseValidUntil,
  renewalBucket,
  type RenewalWarningBucket,
} from '@/ui/utils/renewal';

export interface RenewalHintProps {
  dateValidUntil: string | null | undefined;
  variant?: 'banner' | 'inline' | 'muted';
  className?: string;
}

const VARIANT_STYLES: Record<RenewalWarningBucket, string> = {
  expired: 'border-red-300 bg-red-50 text-red-700 dark:bg-red-900/30 dark:text-red-300',
  imminent: 'border-amber-300 bg-amber-50 text-amber-700 dark:bg-amber-900/30 dark:text-amber-300',
  soon: 'border-yellow-300 bg-yellow-50 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-300',
  future: 'border-gray-200 bg-gray-50 text-gray-600 dark:bg-gray-800/40 dark:text-gray-300',
};

function formatBannerCopy(
  bucket: RenewalWarningBucket,
  daysUntil: number | null,
  formatted: string | null,
  t: (k: string, params?: Record<string, string | number>) => string
): string {
  if (!formatted) return '';
  if (bucket === 'expired') {
    return t('validUntil', { date: formatted });
  }
  if (bucket === 'imminent' && daysUntil !== null) {
    if (daysUntil === 0) return t('expiresToday');
    return t('expiresInDays', { days: daysUntil });
  }
  return t('renewalHint', { date: formatted });
}

export const RenewalHint: React.FC<RenewalHintProps> = ({
  dateValidUntil,
  variant = 'inline',
  className = '',
}) => {
  const { t, locale } = useI18n();
  const parsed = parseValidUntil(dateValidUntil, new Date(), locale);
  if (!parsed.date || !parsed.formatted) {
    return null;
  }
  const bucket = renewalBucket(parsed.daysUntil);
  const text = formatBannerCopy(bucket, parsed.daysUntil, parsed.formatted, t);

  if (variant === 'muted') {
    return (
      <p className={`text-[11px] text-gray-500 ${className}`} aria-label={t('validUntil', { date: parsed.formatted })}>
        {text}
      </p>
    );
  }

  if (variant === 'inline') {
    return (
      <span
        className={`inline-flex items-center px-2 py-0.5 text-[11px] font-medium rounded-full border ${VARIANT_STYLES[bucket]} ${className}`}
        role="status"
        aria-label={t('validUntil', { date: parsed.formatted })}
      >
        {text}
      </span>
    );
  }

  // banner
  return (
    <div
      className={`mt-2 px-3 py-2 text-xs rounded-lg border ${VARIANT_STYLES[bucket]} ${className}`}
      role={bucket === 'expired' || bucket === 'imminent' ? 'alert' : 'status'}
    >
      <strong className="font-semibold">{t('renewalWarningTitle')}:</strong> {text}
    </div>
  );
};

export default RenewalHint;