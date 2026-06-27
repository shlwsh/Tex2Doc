/**
 * API Client for Tex2Doc Commercial API
 *
 * TypeScript implementation based on Rust commercial-api-client
 * Handles all API communication with the Tex2Doc backend.
 */

import type {
  AuthResponse,
  UserProfile,
  UsageSummary,
  PlanSummary,
  BillingSession,
  RedeemCodeResult,
} from '@/shared/types';
import { RedeemCodeRequest } from '@/shared/types';
import { ApiError, AuthError, QuotaExceededError } from '@/shared/errors';
import { API_BASE_URL, API_VERSION } from '@/shared/constants';

// ============================================
// Request/Response Types
// ============================================

export interface ClientConfig {
  baseUrl?: string;
  apiKey?: string;
  timeout?: number;
}

export interface LoginRequest {
  email: string;
  password: string;
}

export interface RegisterRequest {
  email: string;
  password: string;
  display_name?: string;
}

export interface RefreshRequest {
  refresh_token: string;
}

export interface CreateConversionRequest {
  upload_id: string;
  main_tex: string;
  profile: string;
  quality: string;
}

export interface UploadResult {
  upload_id: string;
  filename: string;
  size: number;
}

export interface CheckoutRequest {
  plan_id: string;
  success_url?: string;
  cancel_url?: string;
}

export interface BillingPortalRequest {
  return_url?: string;
}

export interface RechargeRecord {
  recharge_id: string;
  recharge_type: string;
  package_id: string;
  quantity: number;
  amount_cents: number;
  currency: string;
  status: string;
  provider: string;
  provider_trade_id: string;
  created_at: string;
}

const DEFAULT_TIMEOUT = 30000;

export class ApiClient {
  private baseUrl: string;
  private apiKey: string;
  private timeout: number;

  constructor(config: ClientConfig = {}) {
    this.baseUrl = config.baseUrl ?? API_BASE_URL;
    this.apiKey = config.apiKey ?? '';
    this.timeout = config.timeout ?? DEFAULT_TIMEOUT;
  }

  private getUrl(path: string): string {
    return `${this.baseUrl}/${API_VERSION}/${path.replace(/^\//, '')}`;
  }

  private async request<T>(
    method: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH',
    path: string,
    body?: unknown,
    authRequired = false
  ): Promise<T> {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.timeout);

    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    };

    if (this.apiKey) {
      headers['Authorization'] = `Bearer ${this.apiKey}`;
    }

    try {
      const response = await fetch(this.getUrl(path), {
        method,
        headers,
        body: body ? JSON.stringify(body) : undefined,
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        let errorData: Record<string, unknown> = {};
        try {
          errorData = await response.json();
        } catch {
          // Ignore JSON parse errors
        }

        const code = (errorData.code as string) ?? `HTTP_${response.status}`;
        const message = (errorData.message as string) ?? response.statusText;

        if (response.status === 401) {
          throw new AuthError(message, code);
        }

        if (response.status === 403 && code === 'QUOTA_EXCEEDED') {
          const used = (errorData.used as number) ?? 0;
          const limit = (errorData.limit as number) ?? 0;
          throw new QuotaExceededError(used, limit);
        }

        throw new ApiError(message, code, response.status, errorData);
      }

      if (response.status === 204) {
        return {} as T;
      }

      return response.json();
    } catch (error) {
      clearTimeout(timeoutId);

      if (error instanceof AuthError || error instanceof QuotaExceededError || error instanceof ApiError) {
        throw error;
      }

      if (error instanceof TypeError && error.message.includes('abort')) {
        throw new ApiError('Request timeout', 'TIMEOUT');
      }

      throw new ApiError(
        error instanceof Error ? error.message : 'Network error',
        'NETWORK_ERROR'
      );
    }
  }

  // ============================================
  // Auth endpoints
  // ============================================

  /**
   * Login with email and password
   */
  async login(request: LoginRequest): Promise<AuthResponse> {
    return this.request<AuthResponse>('POST', '/auth/login', request);
  }

  /**
   * Register a new account
   */
  async register(request: RegisterRequest): Promise<AuthResponse> {
    return this.request<AuthResponse>('POST', '/auth/register', request);
  }

  /**
   * Refresh access token
   */
  async refresh(request: RefreshRequest): Promise<AuthResponse> {
    return this.request<AuthResponse>('POST', '/auth/refresh', request);
  }

  /**
   * Get current user profile
   */
  async me(): Promise<UserProfile> {
    return this.request<UserProfile>('GET', '/me', undefined, true);
  }

  // ============================================
  // Usage & Plans
  // ============================================

  /**
   * Get current usage summary
   */
  async usage(): Promise<UsageSummary> {
    return this.request<UsageSummary>('GET', '/usage', undefined, true);
  }

  /**
   * Get available plans
   */
  async plans(): Promise<PlanSummary[]> {
    return this.request<PlanSummary[]>('GET', '/plans');
  }

  // ============================================
  // Billing
  // ============================================

  /**
   * Create checkout session
   */
  async createCheckout(request: CheckoutRequest): Promise<BillingSession> {
    return this.request<BillingSession>('POST', '/billing/checkout', request, true);
  }

  /**
   * Create billing portal session
   */
  async createBillingPortal(request: BillingPortalRequest): Promise<BillingSession> {
    return this.request<BillingSession>('POST', '/billing/portal', request, true);
  }

  // ============================================
  // Redeem Codes
  // ============================================

  /**
   * Redeem a code
   */
  async redeemCode(request: RedeemCodeRequest): Promise<RedeemCodeResult> {
    return this.request<RedeemCodeResult>('POST', '/redeem-codes/redeem', request, true);
  }

  /**
   * Get redeem code options/available codes
   */
  async redeemCodeOptions(): Promise<unknown> {
    return this.request('GET', '/redeem-codes/options', undefined, true);
  }

  /**
   * Get redeem code records
   */
  async redeemCodeRecords(): Promise<unknown[]> {
    return this.request<unknown[]>('GET', '/redeem-codes/records', undefined, true);
  }

  // ============================================
  // Uploads
  // ============================================

  /**
   * Upload a project ZIP file
   */
  async uploadProjectZip(zipBytes: Uint8Array, filename: string): Promise<UploadResult> {
    const formData = new FormData();
    const blob = new Blob([zipBytes.buffer as ArrayBuffer], { type: 'application/zip' });
    formData.append('file', blob, filename);

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.timeout * 2);

    try {
      const headers: Record<string, string> = {};
      if (this.apiKey) {
        headers['Authorization'] = `Bearer ${this.apiKey}`;
      }

      const response = await fetch(this.getUrl('/uploads'), {
        method: 'POST',
        headers,
        body: formData,
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new ApiError(
          (errorData.message as string) ?? 'Upload failed',
          (errorData.code as string) ?? 'UPLOAD_FAILED',
          response.status
        );
      }

      return response.json();
    } finally {
      clearTimeout(timeoutId);
    }
  }

  // ============================================
  // Conversions
  // ============================================

  /**
   * Create a conversion job
   */
  async createConversion(request: CreateConversionRequest): Promise<unknown> {
    return this.request('POST', '/conversions', request, true);
  }

  /**
   * Get a conversion job
   */
  async getConversion(jobId: string): Promise<unknown> {
    return this.request('GET', `/conversions/${jobId}`, undefined, true);
  }

  /**
   * List all conversion jobs
   */
  async conversions(): Promise<unknown[]> {
    return this.request<unknown[]>('GET', '/conversions', undefined, true);
  }

  /**
   * Download conversion DOCX
   */
  async downloadConversionDocx(jobId: string): Promise<Uint8Array> {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.timeout * 3);

    try {
      const headers: Record<string, string> = {};
      if (this.apiKey) {
        headers['Authorization'] = `Bearer ${this.apiKey}`;
      }

      const response = await fetch(this.getUrl(`/conversions/${jobId}/download/docx`), {
        method: 'GET',
        headers,
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new ApiError(
          (errorData.message as string) ?? 'Download failed',
          (errorData.code as string) ?? 'DOWNLOAD_FAILED',
          response.status
        );
      }

      const buffer = await response.arrayBuffer();
      return new Uint8Array(buffer);
    } finally {
      clearTimeout(timeoutId);
    }
  }

  /**
   * Get conversion report
   */
  async getConversionReport(jobId: string): Promise<unknown> {
    return this.request('GET', `/conversions/${jobId}/report`, undefined, true);
  }

  // ============================================
  // Recharges
  // ============================================

  /**
   * Get recharge records
   */
  async rechargeRecords(): Promise<RechargeRecord[]> {
    return this.request<RechargeRecord[]>('GET', '/recharges', undefined, true);
  }

  // ============================================
  // Feedback
  // ============================================

  /**
   * Get feedback threads
   */
  async feedbackThreads(): Promise<unknown[]> {
    return this.request<unknown[]>('GET', '/feedback/threads', undefined, true);
  }

  /**
   * Create a feedback thread
   */
  async createFeedbackThread(request: unknown): Promise<unknown> {
    return this.request('POST', '/feedback/threads', request, true);
  }

  /**
   * Get feedback thread messages
   */
  async feedbackThreadMessages(threadId: string): Promise<unknown[]> {
    return this.request<unknown[]>(
      'GET',
      `/feedback/threads/${threadId}/messages`,
      undefined,
      true
    );
  }
}

/**
 * Create an anonymous API client (no auth)
 */
export function createAnonymousClient(config?: ClientConfig): ApiClient {
  return new ApiClient({ ...config, apiKey: '' });
}

/**
 * Create an authenticated API client
 */
export function createAuthenticatedClient(
  config: ClientConfig & { accessToken: string }
): ApiClient {
  return new ApiClient({ ...config, apiKey: config.accessToken });
}
