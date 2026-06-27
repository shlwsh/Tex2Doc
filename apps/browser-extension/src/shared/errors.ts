// Custom Error Classes for Extension

export class ExtensionError extends Error {
  constructor(
    message: string,
    public code: string,
    public statusCode?: number
  ) {
    super(message);
    this.name = 'ExtensionError';
  }
}

export class AuthError extends ExtensionError {
  constructor(message: string, code: string = 'AUTH_ERROR') {
    super(message, code);
    this.name = 'AuthError';
  }
}

export class ApiError extends ExtensionError {
  constructor(
    message: string,
    code: string,
    statusCode?: number,
    public response?: unknown
  ) {
    super(message, code, statusCode);
    this.name = 'ApiError';
  }
}

export class ConversionError extends ExtensionError {
  constructor(
    message: string,
    code: string,
    public jobId?: string,
    public report?: unknown
  ) {
    super(message, code);
    this.name = 'ConversionError';
  }
}

export class QuotaExceededError extends ConversionError {
  constructor(public used: number, public limit: number) {
    super(
      `Quota exceeded: ${used}/${limit} conversions used`,
      'QUOTA_EXCEEDED'
    );
    this.name = 'QuotaExceededError';
  }
}

export class WasmError extends ExtensionError {
  constructor(message: string, public details?: unknown) {
    super(message, 'WASM_ERROR');
    this.name = 'WasmError';
  }
}

export class StorageError extends ExtensionError {
  constructor(message: string, public operation?: string) {
    super(message, 'STORAGE_ERROR');
    this.name = 'StorageError';
  }
}

// Error codes mapping
export const ERROR_CODES = {
  // Auth errors
  INVALID_CREDENTIALS: 'INVALID_CREDENTIALS',
  SESSION_EXPIRED: 'SESSION_EXPIRED',
  TOKEN_REFRESH_FAILED: 'TOKEN_REFRESH_FAILED',
  NOT_AUTHENTICATED: 'NOT_AUTHENTICATED',

  // API errors
  NETWORK_ERROR: 'NETWORK_ERROR',
  API_ERROR: 'API_ERROR',
  RATE_LIMITED: 'RATE_LIMITED',
  SERVER_ERROR: 'SERVER_ERROR',

  // Conversion errors
  QUOTA_EXCEEDED: 'QUOTA_EXCEEDED',
  CONVERSION_FAILED: 'CONVERSION_FAILED',
  JOB_NOT_FOUND: 'JOB_NOT_FOUND',
  JOB_EXPIRED: 'JOB_EXPIRED',
  UPLOAD_FAILED: 'UPLOAD_FAILED',
  DOWNLOAD_FAILED: 'DOWNLOAD_FAILED',

  // WASM errors
  WASM_LOAD_FAILED: 'WASM_LOAD_FAILED',
  WASM_CONVERSION_FAILED: 'WASM_CONVERSION_FAILED',
  FILE_TOO_LARGE: 'FILE_TOO_LARGE',
  INVALID_FILE: 'INVALID_FILE',

  // Storage errors
  STORAGE_ERROR: 'STORAGE_ERROR',
  DB_ERROR: 'DB_ERROR',

  // Generic
  UNKNOWN_ERROR: 'UNKNOWN_ERROR',
} as const;

// Error message helpers
export function getErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === 'string') {
    return error;
  }
  return 'An unknown error occurred';
}

export function isRetryableError(error: unknown): boolean {
  if (error instanceof ApiError) {
    // Retry on network errors, rate limits, and server errors
    return (
      error.statusCode === undefined ||
      error.statusCode >= 500 ||
      error.code === ERROR_CODES.RATE_LIMITED
    );
  }
  return false;
}
