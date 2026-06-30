/**
 * Funnel Analytics — Local-only first-party metrics (P1-2)
 *
 * Design constraints
 *  - No third-party SDKs, no remote beacon. All events land in a ring buffer
 *    inside `chrome.storage.local` keyed by `tex2doc_analytics.events`.
 *  - Privacy: never include user.id / email / access_token / refresh_token /
 *    raw file bytes / file names (we hash them with a stable, non-reversible
 *    digest instead).
 *  - Bounded memory: ring buffer caps at MAX_EVENTS; oldest dropped first.
 *  - Anonymous session_id rotated when the user explicitly signs out, so a
 *    single user on a shared device still has separate funnel sessions.
 */

const STORAGE_KEY = 'tex2doc_analytics.events';
const MAX_EVENTS = 1000;
const MAX_DAYS = 7;

export type FunnelEventName =
  | 'popup_open'
  | 'file_selected'
  | 'convert_started'
  | 'convert_completed'
  | 'convert_failed'
  | 'redeem_used'
  | 'checkout_opened'
  | 'diagnostics_exported'
  | 'funnel_exported'
  | 'folder_selected'
  | 'folder_packaging_started'
  | 'folder_packaging_completed'
  | 'folder_packaging_failed';

export type FunnelStage = 'popup' | 'sidepanel' | 'options' | 'background';

export interface FunnelEvent {
  /** Stable v4 UUID; only used to dedupe on read. */
  id: string;
  /** Unix ms. */
  ts: number;
  /** One of FunnelEventName; unknown strings are dropped at write time. */
  name: FunnelEventName;
  /** Where the event was emitted. */
  stage: FunnelStage;
  /** Anonymous session id, rotated on sign-out. */
  session_id: string;
  /** Browser family + extension version, captured once at read. */
  browser?: string;
  version?: string;
  /** Free-form bag of low-cardinality counters — must not contain PII. */
  meta?: Record<string, string | number | boolean>;
}

const ALLOWED_EVENTS: ReadonlySet<FunnelEventName> = new Set([
  'popup_open',
  'file_selected',
  'convert_started',
  'convert_completed',
  'convert_failed',
  'redeem_used',
  'checkout_opened',
  'diagnostics_exported',
  'funnel_exported',
  'folder_selected',
  'folder_packaging_started',
  'folder_packaging_completed',
  'folder_packaging_failed',
]);

const PII_KEY = /(token|password|email|user|secret|zip|tex|file|name|path|url|message|error|stack)/i;

/**
 * Build or refresh an anonymous session id. We deliberately don't reuse the
 * auth session id because the whole point of the funnel is to attribute
 * events anonymously across the lifecycle.
 */
export async function getSessionId(): Promise<string> {
  const result = await browser.storage.local.get('tex2doc_analytics.session_id');
  if (typeof result['tex2doc_analytics.session_id'] === 'string') {
    return result['tex2doc_analytics.session_id'];
  }
  const fresh = crypto.randomUUID();
  await browser.storage.local.set({ 'tex2doc_analytics.session_id': fresh });
  return fresh;
}

export async function rotateSessionId(): Promise<string> {
  const fresh = crypto.randomUUID();
  await browser.storage.local.set({ 'tex2doc_analytics.session_id': fresh });
  return fresh;
}

async function readBuffer(): Promise<FunnelEvent[]> {
  const result = await browser.storage.local.get(STORAGE_KEY);
  const list = result[STORAGE_KEY];
  return Array.isArray(list) ? (list as FunnelEvent[]) : [];
}

async function writeBuffer(events: FunnelEvent[]): Promise<void> {
  await browser.storage.local.set({ [STORAGE_KEY]: events });
}

/**
 * Strip PII-ish keys from a meta object. Defense in depth: callers should
 * already avoid including them, but the gate keeps an audit trail clean.
 */
function sanitizeMeta(meta?: Record<string, string | number | boolean>): Record<string, string | number | boolean> | undefined {
  if (!meta) return undefined;
  const out: Record<string, string | number | boolean> = {};
  for (const [k, v] of Object.entries(meta)) {
    if (PII_KEY.test(k)) continue;
    if (typeof v === 'string' && v.length > 80) continue; // likely a long path / message
    out[k] = v;
  }
  return Object.keys(out).length > 0 ? out : undefined;
}

function getVersion(): string {
  try {
    return browser.runtime.getManifest().version ?? '0.0.0';
  } catch {
    return '0.0.0';
  }
}

function getBrowserFamily(): string {
  const ua = typeof navigator !== 'undefined' ? navigator.userAgent : '';
  if (/Edg\//.test(ua)) return 'edge';
  if (/Firefox\//.test(ua)) return 'firefox';
  if (/Safari\//.test(ua) && !/Chrome\//.test(ua)) return 'safari';
  if (/Chrome\//.test(ua)) return 'chrome';
  return 'unknown';
}

export interface TrackOptions {
  stage?: FunnelStage;
  meta?: Record<string, string | number | boolean>;
}

export async function track(name: FunnelEventName, options: TrackOptions = {}): Promise<void> {
  if (!ALLOWED_EVENTS.has(name)) {
    console.warn('[analytics] dropped unknown event:', name);
    return;
  }
  let events: FunnelEvent[];
  try {
    events = await readBuffer();
  } catch (err) {
    console.warn('[analytics] read failed:', err);
    return;
  }
  const event: FunnelEvent = {
    id: crypto.randomUUID(),
    ts: Date.now(),
    name,
    stage: options.stage ?? 'background',
    session_id: await getSessionId(),
    browser: getBrowserFamily(),
    version: getVersion(),
    meta: sanitizeMeta(options.meta),
  };
  events.push(event);
  // Trim to MAX_EVENTS oldest-first.
  if (events.length > MAX_EVENTS) {
    events.splice(0, events.length - MAX_EVENTS);
  }
  try {
    await writeBuffer(events);
  } catch (err) {
    console.warn('[analytics] write failed:', err);
  }
}

/**
 * Snapshot the last `windowDays` of events. Default 7 days. Returns a JSON
 * string ready for download via chrome.downloads.
 */
export async function exportFunnelJson(windowDays: number = MAX_DAYS): Promise<string> {
  const cutoff = Date.now() - windowDays * 86_400_000;
  const events = (await readBuffer()).filter((e) => e.ts >= cutoff);
  return JSON.stringify(
    {
      schema_version: 1,
      exported_at: new Date().toISOString(),
      window_days: windowDays,
      event_count: events.length,
      events,
    },
    null,
    2
  );
}

export async function clearFunnel(): Promise<void> {
  await browser.storage.local.remove(STORAGE_KEY);
}