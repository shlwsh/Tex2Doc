/**
 * Background Service Worker for Tex2Doc Extension
 */

// 在 MV3 service worker 里，dynamic `import()` 是被禁止的（HTML spec 限制）。
// 这里必须用 top-level 静态 import 让 Vite/Rolldown 直接打进 bundle。
// 注意：Node 内置模块（fs/promises、fs 等）在浏览器里需要 vite 把它们 inline 进来，
// 或者改用 chrome.downloads 等 service-worker-native API。
// 这里我们用 chrome.downloads.data URL 来保存（如果数据可放到 URL），
// 或者通过 fs.writeFileSync（在扩展 background 里可行）。
// 详见 scripts/post-build-wasm.mjs 关于 ESM 加载的说明。

import { ApiClient } from '@/api/api-client';
import { login as apiLogin, register as apiRegister, refreshSession } from '@/api/auth';
import { getUsage } from '@/api/usage';
import { createAndPollConversion, pollCloudConversion } from '@/api/conversions';
import { redeemCode } from '@/api/feedback';
import { startCheckout, openBillingPortal } from '@/api/billing';
import { getSession, saveSession, clearSession, getAccessToken } from '@/state/session-store';
import { getSettings, getApiBaseUrl } from '@/state/settings-store';
import { saveJob, getJob, getAllJobs, updateJobStatus, setJobStage, getPendingCloudJobs, addEvent } from '@/state/job-store';
import { downloadBytes } from '@/browser/downloads';
import { openUrl } from '@/browser/compat';
import { convertLocal } from '@/conversion/local-wasm';
import { CONTEXT_MENU_IDS, MESSAGE_TYPES } from '@/shared/constants';
import type { JobRecord, ConversionJob } from '@/shared/types';
import { AuthError, ApiError } from '@/shared/errors';
import { buildDiagnostics, exportDiagnosticsBlob } from '@/diagnostics/bundle';
import { exportFunnelJson, track } from '@/analytics/funnel';

const activePolls = new Map<string, number>();
// localJobId → cloudJobId. Survives only within a single SW lifetime.
// On SW restart, restorePollingJobs rehydrates this map by reading IndexedDB.
const activeCloudJobs = new Map<string, string>();
const POLL_INTERVAL = 2000;
// Mutex to avoid double-recovery when onStartup + onInstalled race.
let restoreInFlight: Promise<void> | null = null;

/**
 * P2-1 — `chrome.alarms` is the only reliable way to wake an MV3 service
 * worker after the browser has suspended it. We register a single 1-minute
 * repeating alarm; the listener reuses `restorePollingJobs`, which no-ops
 * when no jobs are in flight, so the cost is one IndexedDB range scan
 * per minute at worst.
 */
const ALARM_RECOVERY = 'tex2doc.recovery.poll';
const ALARM_RECOVERY_PERIOD_MIN = 1;

function scheduleRecoveryAlarm(): void {
  if (!browser.alarms?.create) return;
  // Overwrite any existing alarm so period changes (e.g. for testing) take effect.
  browser.alarms.create(ALARM_RECOVERY, {
    periodInMinutes: ALARM_RECOVERY_PERIOD_MIN,
  }).catch((error: unknown) => {
    console.warn('[Tex2Doc Background] alarms.create failed:', error);
  });
}

export default defineBackground({
  // ESM 输出：允许 dynamic import() 加载 wasm-bindgen 生成的 ESM 胶水；
  // 同时启用 chunk 分割、减小包体。Chrome MV3 only。
  type: 'module',
  main() {
    browser.runtime.onInstalled.addListener(() => {
      console.log('[Tex2Doc Background] Extension installed');
      createContextMenus();
      // After install we may also need to recover in-flight cloud jobs (e.g. browser restart
      // without `onStartup` firing in some Chrome versions).
      scheduleRestore();
      scheduleRecoveryAlarm();
    });

    browser.runtime.onStartup.addListener(() => {
      console.log('[Tex2Doc Background] Extension startup');
      scheduleRestore();
      scheduleRecoveryAlarm();
    });

    browser.runtime.onMessage.addListener(handleMessage);
    browser.contextMenus.onClicked.addListener(handleContextMenuClick);

    // P2-1: wake the SW at most once a minute so an in-flight cloud job keeps
    // polling even after the browser has suspended the SW. The alarm fires
    // regardless of whether anything is open, so we re-use the same
    // restorePollingJobs path (which no-ops when nothing is pending).
    if (browser.alarms?.onAlarm) {
      browser.alarms.onAlarm.addListener((alarm) => {
        if (alarm.name === ALARM_RECOVERY) {
          scheduleRestore();
        }
      });
    }
    // Always (re)arm on SW startup. chrome.alarms persists across SW lifetimes,
    // so this is a no-op after the first install but matters after a manual SW
    // restart triggered by `chrome.runtime.reload`.
    scheduleRecoveryAlarm();

    // e2e 钩子：暴露一个全局函数供 Playwright service worker evaluate 调用，
    // 绕开 message channel。生产构建里同样有效，但只暴露最小 API。
    (globalThis as unknown as { __tex2docConvertZip?: unknown }).__tex2docConvertZip =
      handleStartWasmConversion;
    (globalThis as unknown as { __tex2docDownloads?: unknown }).__tex2docDownloads = {
      downloadBytes,
    };
  },
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
      case MESSAGE_TYPES.START_WASM_CONVERSION:
        return await handleStartWasmConversion(payload);
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
      case MESSAGE_TYPES.REDEEM_CODE_AND_LOGIN:
        return await handleRedeemCodeAndLogin(payload);
      case MESSAGE_TYPES.CREATE_FEEDBACK:
        return await handleCreateFeedback(payload);
      case MESSAGE_TYPES.CLOUD_CONVERT_AND_POLL:
        return await handleCloudConvertAndPoll(payload);
      case MESSAGE_TYPES.EXPORT_DIAGNOSTICS:
        return await handleExportDiagnostics(payload);
      case MESSAGE_TYPES.EXPORT_FUNNEL:
        return await handleExportFunnel(payload);
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

/**
 * @deprecated Since v0.1.0. Use `CLOUD_CONVERT_AND_POLL` (which uploads the
 * zip internally) instead of `START_CONVERSION`. This handler expects the
 * caller to have already uploaded via `POST /uploads` and only forwards an
 * `uploadId`; the new pipeline is one-shot and self-recovers after a
 * service-worker restart. Content scripts (arxiv / overleaf) still trigger
 * this path today — see P2-5 for the migration plan and
 * `src/shared/messaging.md` §7 for the deprecation timeline.
 */
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

/**
 * @deprecated Since v0.1.0. Implementation behind the deprecated
 * `START_CONVERSION` message; only `handleStartConversion` still invokes it.
 * New flows use `runCloudPipeline` (driven by `CLOUD_CONVERT_AND_POLL`),
 * which uploads the zip in-process and survives MV3 service-worker restarts.
 * Removal target: next minor bump after `content/arxiv.content.ts` and
 * `content/overleaf.content.ts` migrate (P2-5).
 */
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

async function handleRedeemCodeAndLogin(payload: Record<string, unknown>): Promise<unknown> {
  const { code } = payload as { code: string };
  const baseUrl = await getApiBaseUrl();

  // Prefer the existing access token if present, so signed-in users stay on
  // their account. Fall back to an anonymous client when not signed in (or
  // when the existing token has expired and a refresh isn't worth racing).
  const existingAccess = await getAccessToken();
  const client = new ApiClient({ baseUrl, apiKey: existingAccess ?? '' });

  let result;
  try {
    result = await client.redeemCode({ code });
  } catch (error) {
    // The server may signal that *this* code can only be redeemed by an
    // already-signed-in user (i.e. the batch is not auto_provision). Surface
    // a stable error so the popup can prompt the user to log in and retry.
    if (error instanceof ApiError && error.code === 'redeem_requires_login') {
      notifyUI('SESSION_UPDATED', {
        signedIn: false,
        redeemRequiresLogin: true,
        error: error.message,
      });
      return {
        success: false,
        error: 'REDEEM_REQUIRES_LOGIN',
        message: error.message,
      };
    }
    throw error;
  }

  // Server issued tokens → persist the session immediately. We don't
  // synthesize a hardcoded 30-day window anymore; usage limits are read
  // fresh from the /usage endpoint below.
  if (!result.access_token || !result.refresh_token || !result.user) {
    notifyUI('SESSION_UPDATED', {
      signedIn: false,
      error: 'REDEEM_REQUIRES_LOGIN',
      message: 'Server did not provision an account; please sign in and retry',
    });
    return {
      success: false,
      error: 'REDEEM_REQUIRES_LOGIN',
      result,
    };
  }

  const SESSION_TOKEN_EXPIRY_MS = 60 * 60 * 1000;
  await saveSession({
    access_token: result.access_token,
    refresh_token: result.refresh_token,
    user: result.user,
    usage: null,
    expires_at: Date.now() + SESSION_TOKEN_EXPIRY_MS,
  });

  // Refresh usage with the new token so the popup shows an accurate balance
  // right away. Failures here don't block the redeem — we'll retry on the
  // next /usage call from the UI.
  let usage = null;
  try {
    const authClient = new ApiClient({ baseUrl, apiKey: result.access_token });
    usage = await getUsage(authClient);
    const session = await getSession();
    if (session) {
      session.usage = usage;
      await saveSession(session);
    }
  } catch (error) {
    console.warn('[Tex2Doc Background] post-redeem /usage refresh failed:', error);
  }

  const isNewAccount = !!result.is_new_account;
  notifyUI('SESSION_UPDATED', { signedIn: true, usage, isNewAccount });
  return { success: true, result, signedIn: true, isNewAccount };
}

async function handleCloudConvertAndPoll(payload: Record<string, unknown>): Promise<unknown> {
  const { zipBytes, fileName, mainTex, profile, quality } = payload as {
    zipBytes: number[]; fileName: string; mainTex: string; profile: string; quality: string;
  };
  const baseUrl = await getApiBaseUrl();
  const accessToken = await getAccessToken();
  if (!accessToken) throw new AuthError('Not logged in', 'NOT_AUTHENTICATED');

  const localJobId = crypto.randomUUID();
  const job: JobRecord = {
    id: localJobId,
    job_id: undefined,
    file_name: fileName,
    main_tex: mainTex,
    profile,
    quality,
    mode: 'cloud',
    status: 'pending',
    progress: 0,
    stage: 'pending',
    created_at: Date.now(),
    updated_at: Date.now(),
  };
  await saveJob(job);
  notifyUI('JOB_UPDATED', { jobId: localJobId, status: 'pending', progress: 0, stage: 'pending' });

  // Fire-and-forget the cloud pipeline; progress events come back via notifyUI.
  // zipBytes intentionally NOT persisted: SW recovery re-runs from stage='pending' /
  // 'uploading' which re-uploads. This is acceptable because uploads are idempotent
  // (uploadProjectZip returns a fresh upload_id) and avoids serializing MBs to disk.
  void runCloudPipeline(localJobId, new Uint8Array(zipBytes), fileName, mainTex, profile, quality, baseUrl, accessToken);
  // P2-1: ensure the recovery alarm is armed so SW wake-up can resume polling
  // even if the user closes the popup before the conversion finishes.
  scheduleRecoveryAlarm();
  return { success: true, jobId: localJobId };
}

async function runCloudPipeline(
  localJobId: string,
  zipBytes: Uint8Array,
  fileName: string,
  mainTex: string,
  profile: string,
  quality: string,
  baseUrl: string,
  accessToken: string
): Promise<void> {
  const client = new ApiClient({ baseUrl, apiKey: accessToken });
  try {
    // Stage 1: upload
    notifyUI('JOB_UPDATED', { jobId: localJobId, status: 'processing', progress: 15, stage: 'uploading' });
    await setJobStage(localJobId, 'uploading', 'processing', 15);
    const upload = await client.uploadProjectZip(zipBytes, fileName);
    await setJobStage(localJobId, 'uploading', 'processing', 25);
    {
      const stored = await getJob(localJobId);
      if (stored) {
        stored.uploadId = upload.upload_id;
        await saveJob(stored);
      }
    }

    // Stage 2: create conversion job
    notifyUI('JOB_UPDATED', { jobId: localJobId, status: 'processing', progress: 30, stage: 'creating', uploadId: upload.upload_id });
    await setJobStage(localJobId, 'creating', 'processing', 30);
    const created = await client.createConversion({
      upload_id: upload.upload_id,
      main_tex: mainTex,
      profile,
      quality,
    }) as { job_id: string };
    activeCloudJobs.set(localJobId, created.job_id);
    {
      const stored = await getJob(localJobId);
      if (stored) {
        stored.cloudJobId = created.job_id;
        stored.job_id = created.job_id;
        await saveJob(stored);
      }
    }

    // Stage 3: poll until terminal
    notifyUI('JOB_UPDATED', { jobId: localJobId, status: 'processing', progress: 50, stage: 'polling', cloudJobId: created.job_id });
    await setJobStage(localJobId, 'polling', 'processing', 50);
    const finalJob = await pollCloudConversion(
      client,
      created.job_id,
      (update) => {
        const pct = update.status === 'completed' ? 100 : 70;
        notifyUI('JOB_UPDATED', {
          jobId: localJobId,
          status: update.status,
          progress: pct,
          stage: 'polling',
          cloudJobId: created.job_id,
        });
        updateJobStatus(localJobId, update.status as JobRecord['status'], pct);
      },
      { pollInterval: POLL_INTERVAL }
    );

    activeCloudJobs.delete(localJobId);
    const stored = await getJob(localJobId);
    if (stored) {
      stored.job_id = finalJob.job_id;
      stored.cloudJobId = finalJob.job_id;
      stored.status = 'completed';
      stored.stage = 'completed';
      stored.progress = 100;
      stored.docx_ready = finalJob.docx_ready;
      stored.updated_at = Date.now();
      await saveJob(stored);
    }
    notifyUI('JOB_UPDATED', { jobId: localJobId, status: 'completed', progress: 100, stage: 'completed', cloudJobId: finalJob.job_id });
    await emitTerminalNotification('Conversion completed!', 'Conversion completed!', { jobId: localJobId, level: 'info' });
  } catch (error) {
    activeCloudJobs.delete(localJobId);
    const stored = await getJob(localJobId);
    if (stored) {
      stored.status = 'failed';
      stored.stage = 'failed';
      stored.error_message = error instanceof Error ? error.message : 'Conversion failed';
      stored.updated_at = Date.now();
      await saveJob(stored);
    }
    notifyUI('JOB_UPDATED', {
      jobId: localJobId,
      status: 'failed',
      progress: 0,
      stage: 'failed',
      error: error instanceof Error ? error.message : 'Unknown error',
    });
    await emitTerminalNotification('Conversion failed', `Conversion failed: ${error instanceof Error ? error.message : 'Unknown error'}`, { jobId: localJobId, level: 'error' });
  }
}

/**
 * Resume a cloud pipeline after SW restart.
 *
 * The original zip bytes were never persisted (see handleCloudConvertAndPoll), so
 * recovery strategy depends on the job's last persisted stage:
 *
 *  - 'polling' + cloudJobId present  → just keep polling until terminal.
 *  - 'creating' + cloudJobId present → keep polling (the server-side create has succeeded).
 *  - 'uploading' / 'pending'         → mark failed with JOB_NOT_FOUND_AFTER_RESTART because we
 *                                       no longer have the zip bytes. Users will need to retry.
 *
 * UI / popup subscribers reconnect when they re-open, and will see the final terminal state.
 */
async function resumeCloudPipeline(job: JobRecord): Promise<void> {
  if (job.mode !== 'cloud') return;
  const baseUrl = await getApiBaseUrl();
  const accessToken = await getAccessToken();
  if (!accessToken) {
    // No session → mark failed, user will need to sign in.
    await setJobStage(
      job.id,
      'failed',
      'failed',
      job.progress ?? 0
    );
    const stored = await getJob(job.id);
    if (stored) {
      stored.error_message = 'Session expired during conversion; please sign in and retry';
      await saveJob(stored);
    }
    return;
  }

  const cloudJobId = job.cloudJobId ?? job.job_id;
  if (!cloudJobId || (job.stage !== 'polling' && job.stage !== 'creating')) {
    // Cannot recover upload bytes; mark failed.
    await setJobStage(job.id, 'failed', 'failed', job.progress ?? 0);
    const stored = await getJob(job.id);
    if (stored) {
      stored.error_message =
        'Conversion interrupted by browser restart before upload completed; please retry';
      stored.error_code = 'JOB_NOT_FOUND_AFTER_RESTART';
      await saveJob(stored);
    }
    notifyUI('JOB_UPDATED', {
      jobId: job.id,
      status: 'failed',
      stage: 'failed',
      error: 'Conversion interrupted by browser restart; please retry',
    });
    await emitTerminalNotification('Conversion interrupted', 'Conversion interrupted by browser restart', { jobId: job.id, level: 'warning' });
    return;
  }

  activeCloudJobs.set(job.id, cloudJobId);
  const client = new ApiClient({ baseUrl, apiKey: accessToken });
  notifyUI('JOB_UPDATED', {
    jobId: job.id,
    status: 'processing',
    progress: 50,
    stage: 'polling',
    cloudJobId,
    resumed: true,
  });

  try {
    const finalJob = await pollCloudConversion(
      client,
      cloudJobId,
      (update) => {
        const pct = update.status === 'completed' ? 100 : 70;
        notifyUI('JOB_UPDATED', {
          jobId: job.id,
          status: update.status,
          progress: pct,
          stage: 'polling',
          cloudJobId,
        });
        updateJobStatus(job.id, update.status as JobRecord['status'], pct);
      },
      { pollInterval: POLL_INTERVAL }
    );
    activeCloudJobs.delete(job.id);
    const stored = await getJob(job.id);
    if (stored) {
      stored.job_id = finalJob.job_id;
      stored.cloudJobId = finalJob.job_id;
      stored.status = 'completed';
      stored.stage = 'completed';
      stored.progress = 100;
      stored.docx_ready = finalJob.docx_ready;
      stored.updated_at = Date.now();
      await saveJob(stored);
    }
    notifyUI('JOB_UPDATED', {
      jobId: job.id,
      status: 'completed',
      progress: 100,
      stage: 'completed',
      cloudJobId: finalJob.job_id,
    });
    await emitTerminalNotification('Conversion completed!', 'Conversion completed!', { jobId: job.id, level: 'info' });
  } catch (error) {
    activeCloudJobs.delete(job.id);
    const stored = await getJob(job.id);
    if (stored) {
      stored.status = 'failed';
      stored.stage = 'failed';
      stored.error_message = error instanceof Error ? error.message : 'Conversion failed after restart';
      stored.updated_at = Date.now();
      await saveJob(stored);
    }
    notifyUI('JOB_UPDATED', {
      jobId: job.id,
      status: 'failed',
      stage: 'failed',
      error: error instanceof Error ? error.message : 'Unknown error',
    });
    await emitTerminalNotification('Conversion failed', `Conversion failed: ${error instanceof Error ? error.message : 'Unknown error'}`, { jobId: job.id, level: 'error' });
  }
}

async function emitTerminalNotification(
  title: string,
  message: string,
  options: { jobId?: string; level?: 'info' | 'warning' | 'error' } = {}
): Promise<void> {
  const level = options.level ?? 'info';
  // Best-effort; never throw out of the diagnostic path.
  addEvent(level, `${title}: ${message}`, undefined, options.jobId).catch(() => undefined);
  if (!browser.notifications?.create) return;
  try {
    await browser.notifications.create({
      type: 'basic',
      title: 'Tex2Doc',
      message,
      iconUrl: browser.runtime.getURL('/icons/icon48.png'),
    });
  } catch (error) {
    console.error('[Tex2Doc Background] notification.create failed:', error);
  }
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

/**
 * P1-2 — Export the anonymous funnel events as a downloadable JSON.
 * Default 7-day window. The file contains no PII by construction (see
 * `analytics/funnel.ts` sanitizeMeta + DROP_KEY policy).
 */
async function handleExportFunnel(payload: Record<string, unknown>): Promise<unknown> {
  const windowDays = (payload as { windowDays?: number }).windowDays ?? 7;
  const json = await exportFunnelJson(windowDays);
  const stamp = new Date().toISOString().replace(/[:.]/g, '-');
  const filename = `tex2doc-funnel-${stamp}.json`;
  const blob = new Blob([json], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  try {
    await browser.downloads.download({ url, filename, saveAs: true });
  } finally {
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
  track('funnel_exported', { stage: 'options' });
  return { success: true, filename };
}

/**
 * P1-3 — Build a sanitized diagnostics bundle for the requested job (or no
 * job, for ad-hoc capture) and trigger a download via chrome.downloads.
 *
 * Two response modes:
 *  - `download: true` (default): write to disk and return `{ success, filename }`
 *  - `download: false`: return the bundle object directly so callers can
 *      inline it into a feedback ticket.
 */
async function handleExportDiagnostics(payload: Record<string, unknown>): Promise<unknown> {
  const jobId = (payload as { jobId?: string }).jobId;
  const eventLimit = (payload as { eventLimit?: number }).eventLimit;
  const download = (payload as { download?: boolean }).download !== false;

  let job: JobRecord | null = null;
  if (jobId) {
    job = (await getJob(jobId)) ?? null;
  }

  if (download) {
    const bundle = await buildDiagnostics({ job, eventLimit });
    const stamp = new Date().toISOString().replace(/[:.]/g, '-');
    const filename = `tex2doc-diagnostics-${stamp}.json`;
    const blob = new Blob([JSON.stringify(bundle, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    try {
      await browser.downloads.download({
        url,
        filename,
        saveAs: true,
      });
    } finally {
      // Release the object URL on the next tick so the download has a chance to start.
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    }
    return { success: true, filename, event_count: bundle.events.length };
  }

  const bundle = await buildDiagnostics({ job, eventLimit });
  return { success: true, bundle };
}

async function handleContextMenuClick(): Promise<void> {}

function notifyUI(type: string, data: Record<string, unknown>): void {
  // Ignore errors: when popup/sidepanel isn't open, sendMessage rejects with
  // "Could not establish connection". The terminal notification (browser.notifications)
  // is the fallback, so silently dropping here is intentional.
  browser.runtime.sendMessage({ type, ...data }).catch(() => {});
}

/**
 * Schedule a recovery run, de-duplicated across overlapping lifecycle events
 * (onInstalled + onStartup may race on first browser launch).
 */
function scheduleRestore(): void {
  if (restoreInFlight) {
    return;
  }
  restoreInFlight = restorePollingJobs().finally(() => {
    restoreInFlight = null;
  });
}

async function restorePollingJobs(): Promise<void> {
  const inFlight = await getPendingCloudJobs();
  if (inFlight.length === 0) {
    console.log('[Tex2Doc Background] No in-flight cloud jobs to restore');
    return;
  }
  console.log(`[Tex2Doc Background] Restoring ${inFlight.length} in-flight cloud job(s)`);
  // Run recoveries concurrently — each tracks its own localJobId + cloudJobId
  // through activeCloudJobs / IndexedDB and emit JOB_UPDATED as they go.
  await Promise.allSettled(inFlight.map((job) => resumeCloudPipeline(job)));
}

async function handleStartWasmConversion(payload: Record<string, unknown>): Promise<unknown> {
  const { zipBytes, fileName, mainTex } = payload as {
    zipBytes: number[];
    fileName: string;
    mainTex: string;
  };

  const trace: string[] = [];
  const addTrace = (msg: string) => {
    const ts = new Date().toISOString().substr(11, 12);
    trace.push(`[${ts}] ${msg}`);
    console.log(`[Tex2Doc WASM] ${msg}`);
  };

  try {
    addTrace('enter handleStartWasmConversion');
    const bytes = new Uint8Array(zipBytes);
    addTrace(`zip bytes: ${bytes.length}`);

    // Validate zip magic bytes
    if (bytes.length < 4 || bytes[0] !== 0x50 || bytes[1] !== 0x4b) {
      throw new Error(`Invalid ZIP file: magic bytes ${bytes[0]?.toString(16)} ${bytes[1]?.toString(16)} (expected 50 4b)`);
    }
    addTrace('zip magic bytes OK');

    const result = await convertLocal(bytes, { fileName, mainTex });
    addTrace(`convertLocal done: docx ${result.docxBytes.length} bytes`);
    addTrace(`elapsed: ${result.elapsedMs}ms`);

    // e2e 钩子：当 payload 含 _e2eReturnBytes 时，把 docx 字节以 number[] 形式
    // 返回给 Playwright evaluate 调用方（SW 不支持 fs.writeFile / URL.createObjectURL）。
    // 调用方（e2e_wasm_convert.mjs）拿到字节后在 Node 端写文件。
    const wantReturnBytes = payload._e2eReturnBytes as boolean | undefined;
    if (wantReturnBytes) {
      return {
        success: true,
        jobId: result.jobId,
        docxBytes: Array.from(result.docxBytes),
        docxFilename: result.docxFilename,
        trace,
      };
    }
    await downloadBytes(result.docxBytes, result.docxFilename);
    return { success: true, jobId: result.jobId };
  } catch (error) {
    addTrace(`ERROR: ${error instanceof Error ? error.message : String(error)}`);

    // Extract detailed error info
    let detailedMessage = 'WASM conversion failed';
    let errorType = 'UnknownError';

    if (error instanceof Error) {
      detailedMessage = error.message;

      // Check for WasmError (has message property)
      if ('message' in error) {
        errorType = 'WasmError';
        // WasmError from wasm-bindgen has the actual Rust error in message
        detailedMessage = error.message;
        addTrace(`WasmError.message: ${error.message}`);
      }

      // Add stack trace to trace
      if (error.stack) {
        const stackLines = error.stack.split('\n').slice(0, 5);
        stackLines.forEach(line => addTrace(`  ${line.trim()}`));
      }
    } else {
      detailedMessage = String(error);
    }

    // Get current WASM state for debugging
    const g = globalThis as unknown as Record<string, unknown>;
    addTrace(`__tex2docWbg exists: ${!!g.__tex2docWbg}`);
    addTrace(`__tex2docApi exists: ${!!g.__tex2docApi}`);
    if (g.__tex2docApi && typeof g.__tex2docApi === 'object') {
      const api = g.__tex2docApi as Record<string, unknown>;
      addTrace(`  convert_zip: ${typeof api.convert_zip}`);
      addTrace(`  convert_zip_to_docx: ${typeof api.convert_zip_to_docx}`);
    }

    return {
      success: false,
      error: detailedMessage,
      errorType,
      stack: error instanceof Error ? error.stack : null,
      trace,
      // Include helpful debugging info
      debug: {
        fileName,
        mainTex,
        zipBytesLength: zipBytes.length,
        timestamp: new Date().toISOString(),
      },
    };
  }
}
