// API Constants
export const API_BASE_URL = 'https://api.tex2doc.cn';
export const API_VERSION = 'v1';

// Storage Keys
export const STORAGE_KEYS = {
  SESSION: 'tex2doc_session',
  SETTINGS: 'tex2doc_settings',
  JOBS: 'tex2doc_jobs',
} as const;

// IndexedDB
export const DB_NAME = 'tex2doc_extension';
export const DB_VERSION = 1;
export const STORES = {
  JOBS: 'jobs',
  EVENTS: 'events',
} as const;

// Conversion
export const CONVERSION_PROFILES = ['standard', 'academic', 'publication'] as const;
export const CONVERSION_QUALITIES = ['preview', 'balanced', 'strict'] as const;
export type ConversionProfile = (typeof CONVERSION_PROFILES)[number];
export type ConversionQuality = (typeof CONVERSION_QUALITIES)[number];

// WASM
export const WASM_MAX_FILE_SIZE = 10 * 1024 * 1024; // 10MB
export const WASM_DEFAULT_MAIN_TEX = 'main.tex';

// Messages
export const MESSAGE_TYPES = {
  // Popup -> Background
  LOGIN: 'LOGIN',
  LOGOUT: 'LOGOUT',
  REGISTER: 'REGISTER',
  REFRESH_SESSION: 'REFRESH_SESSION',
  FETCH_USAGE: 'FETCH_USAGE',
  START_CONVERSION: 'START_CONVERSION',
  CANCEL_CONVERSION: 'CANCEL_CONVERSION',
  START_WASM_CONVERSION: 'START_WASM_CONVERSION',
  FETCH_JOBS: 'FETCH_JOBS',
  FETCH_JOB_STATUS: 'FETCH_JOB_STATUS',
  DOWNLOAD_DOCX: 'DOWNLOAD_DOCX',
  FETCH_PLANS: 'FETCH_PLANS',
  CREATE_CHECKOUT: 'CREATE_CHECKOUT',
  CREATE_PORTAL: 'CREATE_PORTAL',
  REDEEM_CODE: 'REDEEM_CODE',
  FETCH_CONVERSIONS: 'FETCH_CONVERSIONS',
  FETCH_FEEDBACK: 'FETCH_FEEDBACK',
  CREATE_FEEDBACK: 'CREATE_FEEDBACK',
  GET_SETTINGS: 'GET_SETTINGS',
  UPDATE_SETTINGS: 'UPDATE_SETTINGS',

  // Background -> Popup
  SESSION_UPDATED: 'SESSION_UPDATED',
  JOB_UPDATED: 'JOB_UPDATED',
  NOTIFICATION: 'NOTIFICATION',
  ERROR: 'ERROR',
} as const;

// Context Menu
export const CONTEXT_MENU_IDS = {
  OPEN_POPUP: 'tex2doc-open-popup',
  CONVERT_SELECTION: 'tex2doc-convert-selection',
} as const;
