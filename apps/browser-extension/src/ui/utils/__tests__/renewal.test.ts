import { describe, expect, it } from 'vitest';
import { parseValidUntil, renewalBucket } from '../renewal';

describe('renewal presentation', () => {
  it('parses server epoch-second expiry values', () => {
    const parsed = parseValidUntil('1782864000', new Date('2026-06-29T00:00:00Z'), 'en');
    expect(parsed.date).not.toBeNull();
    expect(parsed.daysUntil).toBe(2);
  });

  it('marks seven-day expiries as imminent', () => {
    expect(renewalBucket(7)).toBe('imminent');
  });
});
