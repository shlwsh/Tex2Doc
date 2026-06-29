import { describe, expect, it } from 'vitest';
import { defaultApiBaseUrl, normalizeBaseUrl } from './http';

describe('http helpers', () => {
  it('normalizes API base URLs with a trailing slash', () => {
    expect(normalizeBaseUrl('http://127.0.0.1:2624/v1')).toBe('http://127.0.0.1:2624/v1/');
    expect(normalizeBaseUrl('http://127.0.0.1:2624/v1/')).toBe('http://127.0.0.1:2624/v1/');
  });

  it('derives the browser default v1 API URL', () => {
    expect(defaultApiBaseUrl()).toMatch(/\/v1\/$/);
  });
});
