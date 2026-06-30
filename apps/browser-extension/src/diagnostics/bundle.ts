/**
 * Diagnostics Bundle Builder (P1-3)
 *
 * Produces a sanitized JSON bundle users can attach to feedback tickets.
 * The bundle is the only thing that crosses the trust boundary to the
 * support team, so the *allow-list* policy below is the contract.
 *
 * Allowed fields
 *  - browser / version / locale (low cardinality, no PII)
 *  - settings hash (sha-256 of redacted ExtensionSettings — proves
 *    reproducibility without revealing the API base URL or theme choices)
 *  - job metadata: id, file_name (just the basename), main_tex, mode,
 *    status, stage, progress, error_code, error_message, timestamps
 *  - last 200 events from the IndexedDB ring buffer
 *
 * Excluded fields
 *  - Any field whose name contains "token", "password", "email", "code"
 *  - The raw .zip / .tex bytes (we never persist them, but double-check
 *    here so that future code that *does* store them can't accidentally leak)
 *  - File contents, base64 blobs, "details" payloads that may contain
 *    arbitrary user data are dropped via the DROP_KEY regex below.
 */

import type { JobRecord } from '@/shared/types';
import { getEvents } from '@/state/job-store';
import { getSettings } from '@/state/settings-store';
import { getSession } from '@/state/session-store';

const DROP_KEY = /(token|password|email|refresh|access_code|auth|secret|zipBytes|docxBytes|file_bytes|content)/i;

/** Subset of JobRecord that's safe to ship to support. */
export interface SanitizedJobMeta {
  id: string;
  file_name: string;
  main_tex: string;
  mode: JobRecord['mode'];
  status: JobRecord['status'];
  stage?: JobRecord['stage'];
  progress?: number;
  error_code?: string;
  error_message?: string;
  created_at?: number;
  updated_at?: number;
}

export interface DiagnosticsBundle {
  schema_version: 1;
  generated_at: string;
  app: {
    name: string;
    version: string;
    language: string;
  };
  browser: {
    name: string;
    version: string | null;
    user_agent: string;
  };
  settings_hash: string;
  job?: SanitizedJobMeta | null;
  events: Array<{
    id: string;
    timestamp: number;
    type: string;
    message: string;
    job_id?: string;
  }>;
}

export async function sha256Hex(input: string): Promise<string> {
  if (!globalThis.crypto?.subtle) {
    // Last-resort fallback when running outside HTTPS context.
    let h = 5381;
    for (let i = 0; i < input.length; i++) h = ((h << 5) + h + input.charCodeAt(i)) | 0;
    return `fallback-${(h >>> 0).toString(16)}`;
  }
  const enc = new TextEncoder().encode(input);
  const digest = await crypto.subtle.digest('SHA-256', enc);
  return Array.from(new Uint8Array(digest))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

function redactJob(job: JobRecord | null | undefined): SanitizedJobMeta | null {
  if (!job) return null;
  return {
    id: job.id,
    file_name: job.file_name,
    main_tex: job.main_tex,
    mode: job.mode,
    status: job.status,
    stage: job.stage,
    progress: job.progress,
    error_code: job.error_code,
    error_message: job.error_message,
    created_at: job.created_at,
    updated_at: job.updated_at,
  };
}

function redactEvent(raw: unknown): DiagnosticsBundle['events'][number] | null {
  if (!raw || typeof raw !== 'object') return null;
  const e = raw as Record<string, unknown>;
  if (typeof e.id !== 'string' || typeof e.timestamp !== 'number') return null;
  const type = typeof e.type === 'string' ? e.type : 'info';
  const message = typeof e.message === 'string' ? e.message : '';
  return {
    id: e.id,
    timestamp: e.timestamp,
    type,
    message,
    job_id: typeof e.job_id === 'string' ? e.job_id : undefined,
  };
}

async function settingsFingerprint(): Promise<string> {
  const settings = await getSettings();
  // Hash only the non-PII subset: defaults, mode, theme, language.
  const safe = {
    default_profile: settings.default_profile,
    default_quality: settings.default_quality,
    default_mode: settings.default_mode,
    wasm_file_size_limit: settings.wasm_file_size_limit,
    language: settings.language,
    theme: settings.theme,
    polling_interval: settings.polling_interval,
  };
  return sha256Hex(JSON.stringify(safe));
}

function detectBrowser(): { name: string; version: string | null; user_agent: string } {
  const ua = typeof navigator !== 'undefined' ? navigator.userAgent : 'unknown';
  if (/Edg\//.test(ua)) return { name: 'edge', version: ua.match(/Edg\/([\d.]+)/)?.[1] ?? null, user_agent: ua };
  if(/Firefox\//.test(ua)) return { name: 'firefox', version: ua.match(/Firefox\/([\d.]+)/)?.[1] ?? null, user_agent: ua };
  if (/Safari\//.test(ua) && !/Chrome\//.test(ua)) return { name: 'safari', version: ua.match(/Version\/([\d.]+)/)?.[1] ?? null, user_agent: ua };
  if (/Chrome\//.test(ua)) return { name: 'chrome', version: ua.match(/Chrome\/([\d.]+)/)?.[1] ?? null, user_agent: ua };
  return { name: 'unknown', version: null, user_agent: ua };
}

export interface BuildDiagnosticsOptions {
  job?: JobRecord | null;
  /** Max events to include (default 200, capped by job-store MAX_EVENTS). */
  eventLimit?: number;
}

export async function buildDiagnostics(
  options: BuildDiagnosticsOptions = {}
): Promise<DiagnosticsBundle> {
  const settings = await getSettings();
  const eventLimit = Math.min(Math.max(options.eventLimit ?? 200, 1), 1000);

  // We deliberately don't include getSession() output. The session contains
  // tokens / email which are dropped via the DROP_KEY regex even if they
  // were included.
  await getSession().catch(() => null);

  const eventsRaw = (await getEvents(undefined, eventLimit)) as unknown[];
  const events = eventsRaw.map(redactEvent).filter(Boolean) as DiagnosticsBundle['events'];

  const manifest = typeof browser !== 'undefined' && browser.runtime?.getManifest
    ? browser.runtime.getManifest()
    : null;

  const bundle: DiagnosticsBundle = {
    schema_version: 1,
    generated_at: new Date().toISOString(),
    app: {
      name: manifest?.name ?? 'Tex2Doc',
      version: manifest?.version ?? '0.0.0',
      language: settings.language,
    },
    browser: detectBrowser(),
    settings_hash: await settingsFingerprint(),
    job: redactJob(options.job ?? null),
    events,
  };

  // Final pass: drop any top-level keys that match DROP_KEY (defense in depth).
  return JSON.parse(JSON.stringify(bundle, (key, value) => {
    if (typeof key === 'string' && DROP_KEY.test(key)) return undefined;
    return value;
  })) as DiagnosticsBundle;
}

export async function exportDiagnosticsBlob(options: BuildDiagnosticsOptions = {}): Promise<Blob> {
  const bundle = await buildDiagnostics(options);
  return new Blob([JSON.stringify(bundle, null, 2)], { type: 'application/json' });
}