/**
 * Global type declarations for browser extension
 */

import type { Browser } from 'webextension-polyfill';

// Re-export browser for convenience
export { Browser };

// Declare browser globally for extension contexts
declare global {
  const browser: Browser;
}

// Make sure this file is treated as a module
export {};
