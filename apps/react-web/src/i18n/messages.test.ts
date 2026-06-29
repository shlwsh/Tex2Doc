import { describe, expect, it } from 'vitest';
import { fieldLabel, messages, statusLabel } from './messages';

describe('react web messages', () => {
  it('keeps zh-CN and en-US dictionaries structurally aligned', () => {
    expect(Object.keys(messages['zh-CN'])).toEqual(Object.keys(messages['en-US']));
    expect(Object.keys(messages['zh-CN'].fields)).toEqual(Object.keys(messages['en-US'].fields));
    expect(Object.keys(messages['zh-CN'].statuses)).toEqual(Object.keys(messages['en-US'].statuses));
    expect(Object.keys(messages['zh-CN'].copy)).toEqual(Object.keys(messages['en-US'].copy));
  });

  it('has labels for key table fields in both locales', () => {
    for (const key of ['created_at', 'code_id', 'risk_level']) {
      expect(fieldLabel('zh-CN', key)).toBeTruthy();
      expect(fieldLabel('en-US', key)).toBeTruthy();
    }
  });

  it('maps known statuses and falls back for unknown values', () => {
    expect(statusLabel('zh-CN', 'high')).toBe('高');
    expect(statusLabel('en-US', 'high')).toBe('High');
    expect(statusLabel('en-US', 'not_a_real_status')).toBeUndefined();
  });
});
