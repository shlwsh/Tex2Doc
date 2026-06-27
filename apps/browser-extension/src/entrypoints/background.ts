/**
 * Background Service Worker for Tex2Doc Extension
 */

import { ApiClient } from '@/api/api-client';
import { login as apiLogin, register as apiRegister, refreshSession } from '@/api/auth';
import { getUsage } from '@/api/usage';
import { createAndPollConversion } from '@/api/conversions';
import { redeemCode } from '@/api/feedback';
import { startCheckout, openBillingPortal } from '@/api/billing';
import { getSession, saveSession, clearSession, getAccessToken } from '@/state/session-store';
import { getSettings, getApiBaseUrl } from '@/state/settings-store';
import { saveJob, getJob, getAllJobs, updateJobStatus } from '@/state/job-store';
import { downloadBytes } from '@/browser/downloads';
import { openUrl } from '@/browser/compat';
import { CONTEXT_MENU_IDS, MESSAGE_TYPES } from '@/shared/constants';
import type { JobRecord, ConversionJob } from '@/shared/types';
import { AuthError } from '@/shared/errors';

const activePolls = new Map<string, number>();
const POLL_INTERVAL = 2000;

export default defineBackgroundScript(() => {
  browser.runtime.onInstalled.addListener(() => {
    console.log('[Tex2Doc Background] Extension installed');
    createContextMenus();
  });

  browser.runtime.onStartup.addListener(async () => {
    console.log('[Tex2Doc Background] Extension startup');
    await restorePollingJobs();
  });

  browser.runtime.onMessage.addListener(handleMessage);
  browser.contextMenus.onClicked.addListener(handleContextMenuClick);

  console.log('[Tex2Doc Background] Service worker started');
});

function createContextMenus(): void {
  browser.contextMenus.create({
    id: CONTEXT_MENU_IDS.OPEN_POPUP,
    title: 'Open Tex2Doc',
    contexts: ['all'],
  });
}

async function handleMessage(message: Record<string, unknown>): Promise<unknown> {
  const { type, ...payload } = message;

  try {
    switch (type) {
      case MESSAGE_TYPES.LOGIN:
        return await handleLogin(payload);
      case MESSAGE_TYPES.REGISTER:
        return await handleRegister(payload);
      case MESSAGE_TYPES.LOGOUT:
        return await handleLogout();
      case MESSAGE_TYPES.REFRESH_SESSION:
        return await handleRefreshSession();
      case MESSAGE_TYPES.FETCH_USAGE:
        return await handleFetchUsage();
      case MESSAGE_TYPES.START_CONVERSION:
        return await handleStartConversion(payload);
      case MESSAGE_TYPES.CANCEL_CONVERSION:
        return await handleCancelConversion(payload);
      case MESSAGE_TYPES.FETCH_JOBS:
        return await handleFetchJobs();
      case MESSAGE_TYPES.DOWNLOAD_DOCX:
        return await handleDownloadDocx(payload);
      case MESSAGE_TYPES.FETCH_PLANS:
        return await handleFetchPlans();
      case MESSAGE_TYPES.CREATE_CHECKOUT:
        return await handleCreateCheckout(payload);
      case MESSAGE_TYPES.CREATE_PORTAL:
        return await handleCreatePortal();
      case MESSAGE_TYPES.REDEEM_CODE:
        return await handleRedeemCode(payload);
      case MESSAGE_TYPES.CREATE_FEEDBACK:
        return await handleCreateFeedback(payload);
      case MESSAGE_TYPES.GET_SETTINGS:
        return await getSettings();
      default:
        return { error: 'Unknown message type' };
    }
  } catch (error) {
    console.error('[Tex2Doc Background] Message error:', error);
    return { error: error instanceof Error ? error.message : 'Unknown error' };
  }
}

async function handleLogin(payload: Record<string, unknown>): Promise<unknown> {
  const { email, password } = payload as { email: string; password: string };
  const baseUrl = await getApiBaseUrl();
  const session = await apiLogin(baseUrl, email, password);
  await saveSession({
    access_token: session.access_token,
    refresh_token: session.refresh_token,
    user: session.user,
    usage: session.usage,
    expires_at: session.expires_at,
  });
  notifyUI('SESSION_UPDATED', { signedIn: true });
  return { success: true, user: session.user, usage: session.usage };
}

async function handleRegister(payload: Record<string, unknown>): Promise<unknown> {
  const { email, password, displayName } = payload as { email: string; password: string; displayName?: string };
  const baseUrl = await getApiBaseUrl();
  const session = await apiRegister(baseUrl, email, password, displayName);
  await saveSession({
    access_token: session.access_token,
    refresh_token: session.refresh_token,
    user: session.user,
    usage: session.usage,
    expires_at: session.expires_at,
  });
  notifyUI('SESSION_UPDATED', { signedIn: true });
  return { success: true, user: session.user, usage: session.usage };
}

async function handleLogout(): Promise<unknown> {
  await clearSession();
  notifyUI('SESSION_UPDATED', { signedIn: false });
  return { success: true };
}

async function handleRefreshSession(): Promise<unknown> {
  const baseUrl = await getApiBaseUrl();
  try {
    const session = await refreshSession(baseUrl);
    await saveSession({
      access_token: session.access_token,
      refresh_token: session.refresh_token,
      user: session.user,
      usage: session.usage,
      expires_at: session.expires_at,
    });
    return { success: true, user: session.user, usage: session.usage };
  } catch (error) {
    if (error instanceof AuthError) {
      await clearSession();
      notifyUI('SESSION_UPDATED', { signedIn: false });
    }
    throw error;
  }
}

async function handleFetchUsage(): Promise<unknown> {
  const baseUrl = await getApiBaseUrl();
  const accessToken = await getAccessToken();
  if (!accessToken) throw new AuthError('Not logged in', 'NOT_AUTHENTICATED');
  const client = new ApiClient({ baseUrl, apiKey: accessToken });
  return getUsage(client);
}

async function handleStartConversion(payload: Record<string, unknown>): Promise<unknown> {
  const { uploadId, mainTex, profile, quality, fileName, mode } = payload as {
    uploadId: string; mainTex: string; profile: string; quality: string; fileName: string; mode: 'local' | 'cloud';
  };
  const baseUrl = await getApiBaseUrl();
  const accessToken = await getAccessToken();
  if (!accessToken) throw new AuthError('Not logged in', 'NOT_AUTHENTICATED');

  const jobId = crypto.randomUUID();
  const job: JobRecord = {
    id: jobId,
    job_id: undefined,
    file_name: fileName,
    main_tex: mainTex,
    profile,
    quality,
    mode,
    status: 'pending',
    progress: 0,
    created_at: Date.now(),
    updated_at: Date.now(),
  };
  await saveJob(job);
  if (mode === 'cloud') {
    startCloudConversion(jobId, uploadId, mainTex, profile, quality, baseUrl, accessToken);
  }
  return { success: true, jobId };
}

async function startCloudConversion(
  localJobId: string, uploadId: string, mainTex: string, profile: string, quality: string,
  baseUrl: string, accessToken: string
): Promise<void> {
  const client = new ApiClient({ baseUrl, apiKey: accessToken });
  try {
    await updateJobStatus(localJobId, 'processing', 10);
    const job = await createAndPollConversion(
      client, uploadId, mainTex, profile, quality,
      (update: ConversionJob) => {
        const progress = update.status === 'completed' ? 100 : 50;
        updateJobStatus(localJobId, update.status as JobRecord['status'], progress);
        notifyUI('JOB_UPDATED', { jobId: localJobId, status: update.status });
      },
      { pollInterval: POLL_INTERVAL }
    );

    const localJob = await getJob(localJobId);
    if (localJob) {
      localJob.job_id = job.job_id;
      localJob.status = 'completed';
      localJob.progress = 100;
      localJob.updated_at = Date.now();
      localJob.docx_ready = job.docx_ready;
      await saveJob(localJob);
    }
    notifyUI('JOB_UPDATED', { jobId: localJobId, status: 'completed' });
    browser.notifications?.create({
      type: 'basic',
      title: 'Tex2Doc',
      message: 'Conversion completed!',
      iconUrl: browser.runtime.getURL('/icons/icon48.png'),
    });
  } catch (error) {
    const localJob = await getJob(localJobId);
    if (localJob) {
      localJob.status = 'failed';
      localJob.error_message = error instanceof Error ? error.message : 'Conversion failed';
      localJob.updated_at = Date.now();
      await saveJob(localJob);
    }
    notifyUI('JOB_UPDATED', { jobId: localJobId, status: 'failed', error: error instanceof Error ? error.message : 'Conversion failed' });
    browser.notifications?.create({
      type: 'basic',
      title: 'Tex2Doc',
      message: `Conversion failed: ${error instanceof Error ? error.message : 'Unknown error'}`,
      iconUrl: browser.runtime.getURL('/icons/icon48.png'),
    });
  }
}

async function handleCancelConversion(payload: Record<string, unknown>): Promise<unknown> {
  const { jobId } = payload as { jobId: string };
  activePolls.delete(jobId);
  await updateJobStatus(jobId, 'failed');
  return { success: true };
}

async function handleFetchJobs(): Promise<unknown> {
  return await getAllJobs();
}

async function handleDownloadDocx(payload: Record<string, unknown>): Promise<unknown> {
  const { jobId, cloudJobId } = payload as { jobId: string; cloudJobId: string };
  const baseUrl = await getApiBaseUrl();
  const accessToken = await getAccessToken();
  if (!accessToken) throw new AuthError('Not logged in', 'NOT_AUTHENTICATED');
  const client = new ApiClient({ baseUrl, apiKey: accessToken });
  const docxBytes = await client.downloadConversionDocx(cloudJobId);
  const job = await getJob(jobId);
  const filename = job ? `${job.file_name.replace(/\.[^.]+$/, '')}.docx` : `conversion_${cloudJobId}.docx`;
  await downloadBytes(docxBytes, filename);
  return { success: true, filename };
}

async function handleFetchPlans(): Promise<unknown> {
  const baseUrl = await getApiBaseUrl();
  const client = new ApiClient({ baseUrl });
  return client.plans();
}

async function handleCreateCheckout(payload: Record<string, unknown>): Promise<unknown> {
  const { planId } = payload as { planId: string };
  const baseUrl = await getApiBaseUrl();
  const accessToken = await getAccessToken();
  if (!accessToken) throw new AuthError('Not logged in', 'NOT_AUTHENTICATED');
  const client = new ApiClient({ baseUrl, apiKey: accessToken });
  const session = await client.createCheckout({
    plan_id: planId,
    success_url: `${baseUrl}/billing/success`,
    cancel_url: `${baseUrl}/billing/cancel`,
  });
  if (session.url) await openUrl(session.url);
  return { success: true, url: session.url };
}

async function handleCreatePortal(): Promise<unknown> {
  const baseUrl = await getApiBaseUrl();
  const accessToken = await getAccessToken();
  if (!accessToken) throw new AuthError('Not logged in', 'NOT_AUTHENTICATED');
  const client = new ApiClient({ baseUrl, apiKey: accessToken });
  const session = await client.createBillingPortal({ return_url: baseUrl });
  if (session.url) await openUrl(session.url);
  return { success: true, url: session.url };
}

async function handleRedeemCode(payload: Record<string, unknown>): Promise<unknown> {
  const { code } = payload as { code: string };
  const baseUrl = await getApiBaseUrl();
  const accessToken = await getAccessToken();
  if (!accessToken) throw new AuthError('Not logged in', 'NOT_AUTHENTICATED');
  const client = new ApiClient({ baseUrl, apiKey: accessToken });
  const result = await client.redeemCode({ code });
  const usage = await getUsage(client);
  const session = await getSession();
  if (session) {
    session.usage = usage;
    await saveSession(session);
  }
  notifyUI('SESSION_UPDATED', { usage });
  return { success: true, result };
}

async function handleCreateFeedback(payload: Record<string, unknown>): Promise<unknown> {
  const { title, feedbackType, content, conversionJobId } = payload as {
    title: string; feedbackType: string; content: string; conversionJobId?: string;
  };
  const baseUrl = await getApiBaseUrl();
  const accessToken = await getAccessToken();
  if (!accessToken) throw new AuthError('Not logged in', 'NOT_AUTHENTICATED');
  const client = new ApiClient({ baseUrl, apiKey: accessToken });
  return client.createFeedbackThread({
    title,
    feedback_type: feedbackType as 'issue' | 'requirement' | 'other',
    content,
    conversion_job_id: conversionJobId,
  });
}

async function handleContextMenuClick(): Promise<void> {}

function notifyUI(type: string, data: Record<string, unknown>): void {
  browser.runtime.sendMessage({ type, ...data }).catch(() => {});
}

async function restorePollingJobs(): Promise<void> {
  const jobs = await getAllJobs();
  const pendingJobs = jobs.filter((j) => j.status === 'processing' || j.status === 'pending');
  for (const job of pendingJobs) {
    if (job.job_id) {
      console.log('[Tex2Doc Background] Restoring job:', job.id);
    }
  }
}
