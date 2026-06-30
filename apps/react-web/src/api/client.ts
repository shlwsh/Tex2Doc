import { HttpClient } from './http';
import type {
  AdminDashboardSummary,
  AdminProfile,
  AdminRedeemCode,
  AdminRedeemCodeListResult,
  AutomationAgent,
  AutomationEvent,
  AutomationRequest,
  AuthResponse,
  ConversionJob,
  CreateFeedbackResponse,
  FeedbackMessage,
  FeedbackThread,
  FeedbackThreadDetail,
  LocalQuotaCheckResponse,
  LocalQuotaConsumeResponse,
  RechargeOptions,
  RechargeRecord,
  RedeemCodeBatch,
  RedeemCodeOptions,
  RedeemCodeRecord,
  RedeemCodeResult,
  ReleaseManifest,
  UploadResponse,
  UsageSummary,
  UserProfile,
} from './types';

export class Tex2DocApi {
  private readonly http: HttpClient;

  constructor(baseUrl: string, accessToken?: string) {
    this.http = new HttpClient(baseUrl, accessToken);
  }

  get baseUrl(): string {
    return this.http.baseUrl;
  }

  register(email: string, password: string, displayName?: string): Promise<AuthResponse> {
    return this.http.post('auth/register', { email, password, display_name: displayName });
  }

  login(email: string, password: string): Promise<AuthResponse> {
    return this.http.post('auth/login', { email, password });
  }

  refresh(refreshToken: string): Promise<AuthResponse> {
    return this.http.post('auth/refresh', { refresh_token: refreshToken });
  }

  me(): Promise<UserProfile> {
    return this.http.get('me');
  }

  usage(): Promise<UsageSummary> {
    return this.http.get('usage');
  }

  rechargeOptions(): Promise<RechargeOptions> {
    return this.http.get('recharge/options');
  }

  createRecharge(rechargeType: string, packageId: string, quantity?: number): Promise<RechargeRecord> {
    return this.http.post('recharges', {
      recharge_type: rechargeType,
      package_id: packageId,
      quantity,
    });
  }

  recharges(): Promise<RechargeRecord[]> {
    return this.http.get('recharges');
  }

  redeemCodeOptions(): Promise<RedeemCodeOptions> {
    return this.http.get('redeem-codes/options');
  }

  redeemCode(code: string): Promise<RedeemCodeResult> {
    return this.http.post('redeem-codes/redeem', { code });
  }

  redeemCodeRecords(): Promise<RedeemCodeRecord[]> {
    return this.http.get('redeem-codes/records');
  }

  uploadProjectZip(file: File): Promise<UploadResponse> {
    return this.http.upload('uploads', file);
  }

  createConversion(uploadId: string, mainTex: string, profile: string, quality: string): Promise<ConversionJob> {
    return this.http.post('conversions', {
      upload_id: uploadId,
      main_tex: mainTex,
      profile,
      quality,
    });
  }

  getConversion(jobId: string): Promise<ConversionJob> {
    return this.http.get(`conversions/${jobId}`);
  }

  conversions(): Promise<ConversionJob[]> {
    return this.http.get('conversions');
  }

  downloadConversionDocx(jobId: string): Promise<Blob> {
    return this.http.download(`conversions/${jobId}/download/docx`);
  }

  downloadConversionZip(jobId: string): Promise<Blob> {
    return this.http.download(`conversions/${jobId}/download/zip`);
  }

  downloadConversionLog(jobId: string): Promise<Blob> {
    return this.http.download(`conversions/${jobId}/download/log`);
  }

  checkLocalConversion(): Promise<LocalQuotaCheckResponse> {
    return this.http.post('local-conversions/check', {});
  }

  consumeLocalConversion(): Promise<LocalQuotaConsumeResponse> {
    return this.http.post('local-conversions/consume', {});
  }

  feedbackThreads(): Promise<FeedbackThread[]> {
    return this.http.get('feedback/threads');
  }

  feedbackThread(threadId: string): Promise<FeedbackThreadDetail> {
    return this.http.get(`feedback/threads/${threadId}`);
  }

  createFeedbackThread(payload: {
    title: string;
    feedbackType: string;
    priority?: string;
    content: string;
    conversionJobId?: string;
  }): Promise<CreateFeedbackResponse> {
    return this.http.post('feedback/threads', {
      title: payload.title,
      feedback_type: payload.feedbackType,
      priority: payload.priority,
      content: payload.content,
      conversion_job_id: payload.conversionJobId || undefined,
    });
  }

  addFeedbackMessage(threadId: string, content: string): Promise<FeedbackMessage> {
    return this.http.post(`feedback/threads/${threadId}/messages`, { content });
  }

  adminMe(): Promise<AdminProfile> {
    return this.http.adminGet('me');
  }

  adminDashboard(): Promise<AdminDashboardSummary> {
    return this.http.adminGet('dashboard');
  }

  createRedeemCodeBatch(payload: {
    packageId: string;
    quantity: number;
    channel?: string;
    expiresAt?: string;
    note?: string;
  }): Promise<RedeemCodeBatch> {
    return this.http
      .adminPost<RedeemCodeBatch>('redeem-code-batches', {
        package_id: payload.packageId,
        quantity: payload.quantity,
        channel: payload.channel,
        expires_at: payload.expiresAt,
        note: payload.note,
      })
      .then(normalizeRedeemBatch);
  }

  redeemCodeBatches(): Promise<RedeemCodeBatch[]> {
    return this.http
      .adminGet<RedeemCodeBatch[] | { batches?: RedeemCodeBatch[] }>('redeem-code-batches')
      .then((value) => (Array.isArray(value) ? value : value.batches ?? []).map(normalizeRedeemBatch));
  }

  redeemCodeBatchDetail(batchId: string): Promise<RedeemCodeBatch> {
    return this.http.adminGet<RedeemCodeBatch>(`redeem-code-batches/${batchId}`).then(normalizeRedeemBatch);
  }

  exportRedeemCodeBatch(batchId: string): Promise<Blob> {
    return this.http.adminDownload(`redeem-code-batches/${batchId}/export.xlsx`);
  }

  adminListRedeemCodes(query: {
    stockStatus?: string;
    batchId?: string;
    packageId?: string;
    search?: string;
    page?: number;
    pageSize?: number;
  }): Promise<AdminRedeemCodeListResult> {
    return this.http
      .adminGet<AdminRedeemCodeListResult>('redeem-codes', {
        stock_status: query.stockStatus,
        batch_id: query.batchId,
        package_id: query.packageId,
        search: query.search,
        page: query.page ?? 1,
        page_size: query.pageSize ?? 50,
      })
      .then(normalizeRedeemCodeList);
  }

  adminBulkStockRedeemCodes(codeIds: string[]): Promise<{ affected: number }> {
    return this.http.adminPost('redeem-codes', { code_ids: codeIds });
  }

  adminRestockRedeemCodes(codes: string): Promise<{ affected: number }> {
    return this.http.adminPost('redeem-codes/restock', { codes });
  }

  adminExportRedeemCodesExcel(query: { stockStatus?: string; batchId?: string; packageId?: string; search?: string }): Promise<Blob> {
    return this.http.adminDownload('redeem-codes/export.xlsx', {
      stock_status: query.stockStatus,
      batch_id: query.batchId,
      package_id: query.packageId,
      search: query.search,
    });
  }

  adminFeedbackThreads(): Promise<FeedbackThread[]> {
    return this.http.adminGet<FeedbackThread[]>('feedback/threads').then((rows) => rows.map(normalizeFeedbackThread));
  }

  adminUpdateFeedbackThread(threadId: string, status?: string, priority?: string): Promise<FeedbackThread> {
    return this.http
      .adminPatch<FeedbackThread>(`feedback/threads/${threadId}`, { status, priority })
      .then(normalizeFeedbackThread);
  }

  adminReplyFeedbackThread(threadId: string, content: string, isInternal = false): Promise<FeedbackMessage> {
    return this.http.adminPost(`feedback/threads/${threadId}/messages`, {
      content,
      is_internal: isInternal,
    });
  }

  adminReleases(): Promise<ReleaseManifest[]> {
    return this.http
      .adminGet<ReleaseManifest[] | { releases?: ReleaseManifest[] }>('releases')
      .then((value) => (Array.isArray(value) ? value : value.releases ?? []));
  }

  adminPublishRelease(payload: {
    channel: string;
    platform: string;
    arch: string;
    version: string;
    releaseTitle?: string;
    downloadUrl: string;
    sha256: string;
  }): Promise<ReleaseManifest> {
    return this.http.adminPost('releases', {
      channel: payload.channel,
      platform: payload.platform,
      arch: payload.arch,
      version: payload.version,
      release_title: payload.releaseTitle,
      download_url: payload.downloadUrl,
      sha256: payload.sha256,
      is_prerelease: payload.channel !== 'stable',
      strategy: { rollout_percent: 100, audience: 'invite_beta' },
    });
  }

  adminRollbackRelease(releaseId: string): Promise<ReleaseManifest> {
    return this.http.adminPost(`releases/${releaseId}/rollback`, { reason: 'admin panel rollback' });
  }

  adminReleaseAudit(): Promise<Array<Record<string, unknown>>> {
    return this.http
      .adminGet<Array<Record<string, unknown>> | { logs?: Array<Record<string, unknown>> }>('release-audit')
      .then((value) => (Array.isArray(value) ? value : value.logs ?? []));
  }

  adminAutomationSummary(): Promise<Record<string, number>> {
    return this.http.adminGet('automation/summary');
  }

  adminAutomationRequests(query?: Record<string, string>): Promise<AutomationRequest[]> {
    return this.http.adminGet('automation/requests', query);
  }

  adminAutomationRequest(id: string): Promise<AutomationRequest> {
    return this.http.adminGet(`automation/requests/${id}`);
  }

  adminAutomationEvents(id: string): Promise<AutomationEvent[]> {
    return this.http.adminGet(`automation/requests/${id}/events`);
  }

  adminAutomationApprove(id: string): Promise<AutomationRequest> {
    return this.http.adminPost(`automation/requests/${id}/approve`);
  }

  adminAutomationReject(id: string, reason: string): Promise<AutomationRequest> {
    return this.http.adminPost(`automation/requests/${id}/reject`, { reason });
  }

  adminAutomationRetry(id: string): Promise<AutomationRequest> {
    return this.http.adminPost(`automation/requests/${id}/retry`);
  }

  adminAutomationEscalate(id: string, assignee: string): Promise<AutomationRequest> {
    return this.http.adminPost(`automation/requests/${id}/escalate`, { assignee });
  }

  adminAutomationAgents(): Promise<AutomationAgent[]> {
    return this.http.adminGet('automation/agents');
  }

  adminAutomationPauseAgent(id: string): Promise<AutomationAgent> {
    return this.http.adminPost(`automation/agents/${id}/pause`);
  }

  adminAutomationResumeAgent(id: string): Promise<AutomationAgent> {
    return this.http.adminPost(`automation/agents/${id}/resume`);
  }
}

function normalizeFeedbackThread(thread: FeedbackThread): FeedbackThread {
  return {
    ...thread,
    id: thread.id ?? thread.thread_id ?? '',
  };
}

function normalizeRedeemBatch(batch: RedeemCodeBatch): RedeemCodeBatch {
  const id = batch.id ?? batch.batch_id ?? '';
  return {
    ...batch,
    id,
    batch_id: batch.batch_id ?? id,
    codes: (batch.codes ?? []).map((code) => ({
      ...(typeof code === 'string' ? { code } : code),
      id: typeof code === 'string' ? code : code.id ?? code.code_id ?? code.code,
      code_id: typeof code === 'string' ? code : code.code_id ?? code.id,
    })),
  };
}

function normalizeRedeemCodeList(result: AdminRedeemCodeListResult): AdminRedeemCodeListResult {
  const rows = (result.items ?? result.codes ?? result.records ?? []).map(normalizeRedeemCode);
  return {
    ...result,
    items: rows,
    codes: rows,
    records: rows,
  };
}

function normalizeRedeemCode(code: AdminRedeemCode): AdminRedeemCode {
  const id = code.id ?? code.code_id ?? '';
  return {
    ...code,
    id,
    code_id: code.code_id ?? id,
  };
}
