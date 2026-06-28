/**
 * Main Popup Application - Commercial UI refactor
 */

import React, { useState, useEffect } from 'react';
import Button from '../../ui/components/Button';
import Input from '../../ui/components/Input';
import Badge from '../../ui/components/Badge';
import Progress from '../../ui/components/Progress';
import Card from '../../ui/components/Card';
import Select from '../../ui/components/Select';
import Toast from '../../ui/components/Toast';
import { Tabs } from '@/ui/components/Tabs';
import { RenewalHint } from '@/ui/components/RenewalHint';
import { track, rotateSessionId } from '@/analytics/funnel';
import { sendToBackground } from '../../browser/messaging';
import { MESSAGE_TYPES } from '../../shared/constants';
import { analyzeZip, type TexFileInfo } from '../../conversion/local-wasm';
import { scanFolder } from '../../conversion/folder-scanner';
import { buildZipFromFolder } from '../../conversion/folder-packager';
import type { FolderEntry } from '../../conversion/folder-types';
import { MAX_FILE_COUNT } from '../../conversion/folder-types';
import type { JobRecord, Session, UsageSummary } from '../../shared/types';
import { useI18n } from '@/ui/i18n/useI18n';

type ConversionMode = 'local' | 'cloud';
type ConversionStage = 'idle' | 'uploading' | 'creating' | 'polling' | 'completed' | 'failed';
type AuthTab = 'signIn' | 'redeem';

function sizeBucket(bytes: number): 'lt_1mb' | '1_to_5mb' | '5_to_10mb' | 'gt_10mb' {
  const mb = bytes / (1024 * 1024);
  if (mb < 1) return 'lt_1mb';
  if (mb < 5) return '1_to_5mb';
  if (mb < 10) return '5_to_10mb';
  return 'gt_10mb';
}

function fileCountBucket(count: number): 'lt_10' | '10_to_50' | '50_to_200' | '200_to_1000' | 'gt_1000' {
  if (count < 10) return 'lt_10';
  if (count < 50) return '10_to_50';
  if (count < 200) return '50_to_200';
  if (count < 1000) return '200_to_1000';
  return 'gt_1000';
}

interface ConversionState {
  stage: ConversionStage;
  progress: number;
  message: string;
  error?: string;
  packaging?: {
    phase: 'reading' | 'packing';
    current: number;
    total: number;
  };
}

interface ZipAnalysis {
  texFiles: TexFileInfo[];
  detectedMainTex: string | null;
}

/** Unified source: either a user-selected ZIP file or a scanned folder. */
interface ZipSource {
  kind: 'zip';
  file: File;
}

interface FolderSource {
  kind: 'folder';
  entries: FolderEntry[];
  excludedCount: number;
  totalSize: number;
  truncated: boolean;
}

type SourceSelection = ZipSource | FolderSource | null;

interface JobUpdatePayload {
  jobId?: string;
  status?: string;
  progress?: number;
  stage?: ConversionStage | string;
  error?: string;
}

export default function PopupApp() {
  const { t, locale, setLocale } = useI18n();
  const [session, setSession] = useState<Session | null>(null);
  const [usage, setUsage] = useState<UsageSummary | null>(null);
  const [source, setSource] = useState<SourceSelection>(null);
  const [mainTex, setMainTex] = useState('main.tex');
  const [mode, setMode] = useState<ConversionMode>('local');
  const [profile, setProfile] = useState('standard');
  const [quality, setQuality] = useState('balanced');
  const [conversion, setConversion] = useState<ConversionState>({
    stage: 'idle',
    progress: 0,
    message: '',
  });
  const [authTab, setAuthTab] = useState<AuthTab>('signIn');
  const [loginEmail, setLoginEmail] = useState('');
  const [loginPassword, setLoginPassword] = useState('');
  const [loginError, setLoginError] = useState('');
  const [isLoggingIn, setIsLoggingIn] = useState(false);
  const [redeemInput, setRedeemInput] = useState('');
  const [isRedeeming, setIsRedeeming] = useState(false);
  const [recentJobs, setRecentJobs] = useState<JobRecord[]>([]);
  const [zipAnalysis, setZipAnalysis] = useState<ZipAnalysis | null>(null);
  const [isAnalyzingZip, setIsAnalyzingZip] = useState(false);
  const [isScanningFolder, setIsScanningFolder] = useState(false);
  const [zipError, setZipError] = useState<string | null>(null);
  const [currentJobId, setCurrentJobId] = useState<string | null>(null);

  useEffect(() => {
    loadSession();
    loadJobs();
    // P1-2: anonymous funnel event when popup opens.
    track('popup_open', { stage: 'popup' });
    const listener = (msg: { type?: string; [key: string]: unknown }) => {
      if (msg?.type === 'JOB_UPDATED' && msg.jobId === currentJobId) {
        applyJobUpdate(msg as JobUpdatePayload & { type: string });
      }
    };
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (browser.runtime.onMessage as any).addListener(listener);
    return () => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (browser.runtime.onMessage as any).removeListener(listener);
    };
  }, [currentJobId]);

  const applyJobUpdate = (msg: JobUpdatePayload & { type: string }) => {
    const stage = (msg.stage as ConversionStage) || 'polling';
    const progress = typeof msg.progress === 'number' ? msg.progress : 0;
    let messageKey = 'converting';
    if (stage === 'uploading') messageKey = 'cloud.uploading';
    else if (stage === 'creating') messageKey = 'cloud.creating';
    else if (stage === 'polling') messageKey = 'cloud.polling';
    else if (stage === 'completed') messageKey = 'cloud.completed';
    else if (stage === 'failed') messageKey = 'cloud.failed';
    setConversion({
      stage,
      progress,
      message: t(messageKey),
      error: msg.error,
    });
    if (stage === 'completed') {
      loadJobs();
    }
  };

  const loadSession = async () => {
    try {
      const result = await sendToBackground<{ signedIn: boolean; user?: unknown; usage?: UsageSummary }>(
        { type: MESSAGE_TYPES.REFRESH_SESSION }
      );
      if (result.signedIn) {
        setSession(result as unknown as Session);
        if (result.usage) setUsage(result.usage);
      }
    } catch (error) {
      console.error('Failed to load session:', error);
    }
  };

  const loadJobs = async () => {
    try {
      const jobs = await sendToBackground<JobRecord[]>({ type: MESSAGE_TYPES.FETCH_JOBS });
      setRecentJobs(jobs.slice(0, 3));
    } catch (error) {
      console.error('Failed to load jobs:', error);
    }
  };

  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsLoggingIn(true);
    setLoginError('');

    try {
      const result = await sendToBackground<{ success: boolean; error?: string }>({
        type: MESSAGE_TYPES.LOGIN,
        email: loginEmail,
        password: loginPassword,
      });

      if (result.success) {
        setLoginEmail('');
        setLoginPassword('');
        await loadSession();
      } else {
        setLoginError(result.error || t('errors.authError'));
      }
    } catch (error) {
      setLoginError(error instanceof Error ? error.message : t('errors.authError'));
    } finally {
      setIsLoggingIn(false);
    }
  };

  const handleRedeemAndLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!redeemInput.trim()) return;
    setIsRedeeming(true);
    setLoginError('');

    try {
      const result = await sendToBackground<{
        success: boolean;
        signedIn?: boolean;
        isNewAccount?: boolean;
        error?: string;
      }>({
        type: MESSAGE_TYPES.REDEEM_CODE_AND_LOGIN,
        code: redeemInput.trim().toUpperCase(),
      });

      if (result.success && result.signedIn) {
        setRedeemInput('');
        await loadSession();
        // P1-2: redeem_used (success path)
        track('redeem_used', { stage: 'popup', meta: { outcome: 'success', is_new_account: !!result.isNewAccount } });
      } else {
        setLoginError(result.error || t('redeemRequiresLogin'));
        track('redeem_used', { stage: 'popup', meta: { outcome: 'requires_login' } });
      }
    } catch (error) {
      setLoginError(error instanceof Error ? error.message : t('redeemFailed'));
    } finally {
      setIsRedeeming(false);
    }
  };

  const handleLogout = async () => {
    try {
      await sendToBackground({ type: MESSAGE_TYPES.LOGOUT });
      setSession(null);
      setUsage(null);
      // P1-2: rotate session_id so the next signed-in user gets a clean funnel.
      await rotateSessionId();
    } catch (error) {
      console.error('Logout failed:', error);
    }
  };

  const resetConversion = () => {
    setConversion({ stage: 'idle', progress: 0, message: '' });
    setZipError(null);
    setZipAnalysis(null);
    setCurrentJobId(null);
  };

  /** Unified source selection: ZIP file OR folder. */
  const handleSourceSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (!files || files.length === 0) return;

    resetConversion();

    // Single .zip file → ZIP mode
    if (files.length === 1 && files[0].name.toLowerCase().endsWith('.zip')) {
      const file = files[0];
      setSource({ kind: 'zip', file });
      track('file_selected', { stage: 'popup', meta: { size_bucket: sizeBucket(file.size) } });
      setIsAnalyzingZip(true);
      try {
        const arrayBuffer = await file.arrayBuffer();
        const zipBytes = new Uint8Array(arrayBuffer);
        const analysis = await analyzeZip(zipBytes);
        setZipAnalysis({ texFiles: analysis.texFiles, detectedMainTex: analysis.detectedMainTex });
        if (analysis.detectedMainTex) {
          setMainTex(analysis.detectedMainTex);
        } else if (analysis.texFiles.length === 0) {
          setZipError(t('noTexFound'));
          setMainTex('');
        } else {
          setMainTex('');
        }
      } catch (error) {
        console.error('ZIP analysis failed:', error);
        setZipError(t('errors.unknown'));
        setZipAnalysis(null);
      } finally {
        setIsAnalyzingZip(false);
      }
      return;
    }

    // Multiple files (webkitdirectory) → Folder mode
    setIsScanningFolder(true);
    try {
      const scan = await scanFolder(files);
      setSource({
        kind: 'folder',
        entries: scan.entries,
        excludedCount: scan.excludedCount,
        totalSize: scan.totalSize,
        truncated: scan.truncated,
      });
      setZipAnalysis({
        texFiles: scan.texFiles.map((t) => ({ ...t, name: t.path.split('/').pop() || t.path })),
        detectedMainTex: scan.detectedMainTex,
      });
      if (scan.detectedMainTex) {
        setMainTex(scan.detectedMainTex);
      } else if (scan.texFiles.length === 0) {
        setZipError(t('noTexFound'));
        setMainTex('');
      } else {
        setMainTex('');
      }
      if (scan.truncated) {
        setZipError(t('folderTruncated', { max: MAX_FILE_COUNT }));
      }
      track('folder_selected', {
        stage: 'popup',
        meta: {
          file_count_bucket: fileCountBucket(scan.entries.length),
          size_bucket: sizeBucket(scan.totalSize),
          excluded_count: scan.excludedCount,
        },
      });
    } catch (error) {
      console.error('Folder scan failed:', error);
      setZipError(t('errors.folderScanFailed'));
      setZipAnalysis(null);
    } finally {
      setIsScanningFolder(false);
    }
  };

  const handleConvert = async () => {
    if (!source) return;
    if (!mainTex || mainTex.trim() === '') {
      setConversion({ stage: 'failed', progress: 0, message: t('mainTexFile'), error: t('mainTexFile') });
      return;
    }

    setConversion({ stage: 'uploading', progress: 0, message: t('cloud.uploading') });
    track('convert_started', {
      stage: 'popup',
      meta: {
        mode,
        source: source.kind,
        size_bucket: sizeBucket(source.kind === 'zip' ? source.file.size : source.totalSize),
      },
    });

    try {
      let zipBytes: Uint8Array;
      let fileName: string;

      if (source.kind === 'zip') {
        zipBytes = new Uint8Array(await source.file.arrayBuffer());
        fileName = source.file.name;
      } else {
        // Folder: pack in-memory, showing client-side progress (0-50%)
        track('folder_packaging_started', {
          stage: 'popup',
          meta: { file_count_bucket: fileCountBucket(source.entries.length) },
        });
        try {
          zipBytes = await buildZipFromFolder(source.entries, {
            onProgress: (phase, current, total) => {
              // Map 0-50% for client-side packing, SW will push 50-100%
              const pct = Math.round((phase === 'reading' ? (current / total) * 0.45 : 0.5) * 100);
              setConversion((c) => ({
                ...c,
                packaging: { phase, current, total },
                progress: pct,
                message: phase === 'reading' ? t('folderReading', { current, total }) : t('folderPacking', { current, total }),
              }));
            },
          });
          const baseName = source.entries[0]?.path.split('/')[0] || 'project';
          fileName = `${baseName}.zip`;
          setConversion((c) => ({ ...c, packaging: undefined }));
          track('folder_packaging_completed', {
            stage: 'popup',
            meta: { output_size_bucket: sizeBucket(zipBytes.byteLength) },
          });
        } catch (error) {
          track('folder_packaging_failed', { stage: 'popup', meta: { error_class: error instanceof Error ? error.name : 'unknown' } });
          const isOOM =
            error instanceof DOMException ||
            (error instanceof Error && (error.name === 'RangeError' || error.name === 'QuotaExceededError'));
          setConversion({
            stage: 'failed',
            progress: 0,
            message: t('folderTooLarge'),
            error: t('folderTooLarge'),
          });
          return;
        }
      }

      if (mode === 'local') {
        const result = await sendToBackground<{
          success: boolean;
          error?: string;
          errorType?: string;
          trace?: string[];
          debug?: Record<string, unknown>;
        }>({
          type: MESSAGE_TYPES.START_WASM_CONVERSION,
          zipBytes: Array.from(zipBytes),
          fileName,
          mainTex,
        });

        if (result.success) {
          setConversion({ stage: 'completed', progress: 100, message: t('conversionComplete') });
          await loadJobs();
          track('convert_completed', { stage: 'popup', meta: { mode: 'local' } });
        } else {
          setConversion({
            stage: 'failed',
            progress: 0,
            message: t('conversionFailed'),
            error: result.error || t('conversionFailed'),
          });
          track('convert_failed', { stage: 'popup', meta: { mode: 'local' } });
        }
      } else {
        const result = await sendToBackground<{ success: boolean; jobId?: string; error?: string }>({
          type: MESSAGE_TYPES.CLOUD_CONVERT_AND_POLL,
          zipBytes: Array.from(zipBytes),
          fileName,
          mainTex,
          profile,
          quality,
        });

        if (result.success && result.jobId) {
          setCurrentJobId(result.jobId);
          track('convert_completed', { stage: 'popup', meta: { mode: 'cloud' } });
        } else {
          setConversion({
            stage: 'failed',
            progress: 0,
            message: t('cloud.failed'),
            error: result.error || t('cloud.failed'),
          });
          track('convert_failed', { stage: 'popup', meta: { mode: 'cloud' } });
        }
      }
    } catch (error) {
      setConversion({
        stage: 'failed',
        progress: 0,
        message: t('conversionFailed'),
        error: error instanceof Error ? error.message : t('errors.unknown'),
      });
    }
  };

  const openSettings = () => {
    browser.runtime.openOptionsPage();
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const isConverting = ['uploading', 'creating', 'polling'].includes(conversion.stage);

  return (
    <div className="w-full max-w-popup mx-auto p-4 space-y-3 bg-white dark:bg-gray-900 min-h-popup">
      {/* Toolbar */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-primary-500 to-primary-700 flex items-center justify-center shadow-sm">
            <span className="text-white font-bold text-xs">T2D</span>
          </div>
          <div className="leading-tight">
            <h1 className="text-sm font-semibold text-gray-900 dark:text-white">{t('appName')}</h1>
            <p className="text-[11px] text-gray-500">{t('tagline')}</p>
          </div>
        </div>
        <div className="flex items-center gap-1">
          <select
            value={locale}
            onChange={(e) => setLocale(e.target.value as typeof locale)}
            className="text-xs bg-transparent border border-gray-200 dark:border-gray-700 rounded-md px-1.5 py-1 text-gray-600 dark:text-gray-300"
          >
            <option value="en">EN</option>
            <option value="zh">中</option>
          </select>
          {session ? (
            <div className="flex items-center gap-1">
              <Badge variant={usage && usage.count_balance > 0 ? 'success' : 'info'}>
                {usage
                  ? `${Math.max(0, usage.cloud_conversions_limit - usage.cloud_conversions_used)}/${usage.cloud_conversions_limit}`
                  : '--'}
              </Badge>
              <button
                onClick={handleLogout}
                className="text-xs text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
              >
                {t('signOut')}
              </button>
            </div>
          ) : null}
        </div>
      </div>

      {session && usage && (
        <RenewalHint dateValidUntil={usage.date_valid_until} variant="banner" />
      )}

      {/* Auth View (only when not signed in) */}
      {!session && (
        <Card className="space-y-3">
          <Tabs
            tabs={[
              { id: 'signIn', label: t('authTabs.signIn') },
              { id: 'redeem', label: t('authTabs.redeem') },
            ]}
            activeTab={authTab}
            onChange={(id) => setAuthTab(id as AuthTab)}
            variant="underline"
          />
          {authTab === 'signIn' ? (
            <form onSubmit={handleLogin} className="space-y-3">
              <Input
                type="email"
                label={t('email')}
                value={loginEmail}
                onChange={(e) => setLoginEmail(e.target.value)}
                placeholder="you@example.com"
                required
              />
              <Input
                type="password"
                label={t('password')}
                value={loginPassword}
                onChange={(e) => setLoginPassword(e.target.value)}
                placeholder="********"
                required
              />
              {loginError && <p className="text-xs text-red-600 dark:text-red-400">{loginError}</p>}
              <Button type="submit" isLoading={isLoggingIn} className="w-full">
                {t('signIn')}
              </Button>
            </form>
          ) : (
            <form onSubmit={handleRedeemAndLogin} className="space-y-3">
              <p className="text-xs text-gray-600 dark:text-gray-400">{t('redeemDescription')}</p>
              <p className="text-xs text-primary-600 dark:text-primary-300">{t('redeemAutoRegister')}</p>
              <Input
                type="text"
                label={t('redeemCodeShort')}
                value={redeemInput}
                onChange={(e) => setRedeemInput(e.target.value)}
                placeholder={t('redeemPlaceholder')}
                required
              />
              {loginError && <p className="text-xs text-red-600 dark:text-red-400">{loginError}</p>}
              <Button type="submit" isLoading={isRedeeming} className="w-full" variant="primary">
                {t('redeem')}
              </Button>
            </form>
          )}
        </Card>
      )}

      {/* Conversion Card */}
      <Card className="space-y-3">
        <div>
          <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1.5">
            {t('selectZipFile')}
          </label>
          <div className="border-2 border-dashed border-gray-300 dark:border-gray-600 rounded-lg p-3 text-center hover:border-primary-400 transition-colors">
            <input
              type="file"
              accept=".zip"
              // @ts-ignore — webkitdirectory / directory are non-standard but widely supported
              webkitdirectory=""
              // @ts-ignore
              directory=""
              multiple
              onChange={handleSourceSelect}
              className="hidden"
              id="source-input"
            />
            <label htmlFor="source-input" className="cursor-pointer block">
              {source ? (
                <div className="text-sm">
                  <p className="font-medium text-gray-900 dark:text-white truncate">{source.kind === 'zip' ? source.file.name : source.entries[0]?.path.split('/')[0] || 'folder'}</p>
                  <p className="text-xs text-gray-500">
                    {source.kind === 'zip'
                      ? formatFileSize(source.file.size)
                      : `${formatFileSize(source.totalSize)} · ${source.entries.length} files`}
                  </p>
                </div>
              ) : (
                <div className="text-gray-500 dark:text-gray-400 text-sm">
                  <svg className="mx-auto h-7 w-7 mb-1.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                  </svg>
                  <p>{t('selectZipHint')}</p>
                </div>
              )}
            </label>
          </div>
        </div>

        {/* Folder selection divider */}
        <div className="flex items-center gap-2">
          <div className="flex-1 h-px bg-gray-200 dark:bg-gray-700" />
          <span className="text-xs text-gray-400">{t('or')}</span>
          <div className="flex-1 h-px bg-gray-200 dark:bg-gray-700" />
        </div>
        <div>
          <label htmlFor="source-input" className="cursor-pointer block">
            <div className="border border-dashed border-gray-300 dark:border-gray-600 rounded-lg p-2 text-center hover:border-primary-400 transition-colors">
              <div className="flex items-center justify-center gap-2 text-gray-600 dark:text-gray-400 text-xs">
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                </svg>
                <span>{t('selectFolder')}</span>
              </div>
              <p className="text-[10px] text-gray-400 mt-0.5">{t('selectFolderHint')}</p>
            </div>
          </label>
        </div>

        {(isAnalyzingZip || isScanningFolder) && (
          <div className="flex items-center gap-2 text-xs text-gray-500">
            <svg className="animate-spin h-3.5 w-3.5" fill="none" viewBox="0 0 24 24">
              <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
              <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            {isScanningFolder ? t('folderScanning') : t('loading')}
          </div>
        )}

        {zipError && (
          <div className="text-xs text-red-600 dark:text-red-400 flex items-center gap-2">
            <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            {zipError}
          </div>
        )}

        {/* Folder excluded count */}
        {source?.kind === 'folder' && source.excludedCount > 0 && !isScanningFolder && (
          <div className="text-xs text-gray-400 dark:text-gray-500 flex items-center gap-1.5">
            <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13.879 16.122a3 3 0 01.365-5.855L18.59 8a3 3 0 010 5.06l-8.257 5.061a3 3 0 01-4.366-.002 3 3 0 01.365-5.855L7.879 11a3 3 0 010-5.06l8.257-5.061a3 3 0 014.366.002z" />
            </svg>
            {t('folderExcluded', { count: source.excludedCount })}
          </div>
        )}

        {zipAnalysis && !isAnalyzingZip && !isScanningFolder && (
          <div className="space-y-2">
            {zipAnalysis.texFiles.length === 1 && zipAnalysis.detectedMainTex && source !== null && (
              <div className="text-xs text-green-600 dark:text-green-400 flex items-center gap-1.5">
                <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                </svg>
                {t(source.kind === 'folder' ? 'folderMainTexFromFolder' : 'mainTexAutoDetected')}: {zipAnalysis.detectedMainTex}
              </div>
            )}
            {zipAnalysis.texFiles.length > 1 && (
              <div className="space-y-1">
                <div className="text-xs text-gray-600 dark:text-gray-400">
                  {t('mainTexPickFromList', { count: zipAnalysis.texFiles.length })}
                </div>
                <div className="max-h-28 overflow-y-auto border border-gray-200 dark:border-gray-700 rounded-lg divide-y divide-gray-200 dark:divide-gray-700">
                  {zipAnalysis.texFiles.map((tex, index) => (
                    <button
                      key={index}
                      onClick={() => setMainTex(tex.path)}
                      className={`w-full px-3 py-1.5 text-left text-xs flex items-center justify-between hover:bg-gray-50 dark:hover:bg-gray-800 ${mainTex === tex.path ? 'bg-primary-50 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300' : 'text-gray-700 dark:text-gray-300'}`}
                    >
                      <span className="truncate">{tex.path}</span>
                      <span className="text-[11px] text-gray-400 ml-2">{formatFileSize(tex.size)}</span>
                    </button>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}

        {!isAnalyzingZip && !isScanningFolder && !zipAnalysis && source && (
          <Input
            label={t('mainTexFile')}
            value={mainTex}
            onChange={(e) => setMainTex(e.target.value)}
            placeholder="main.tex"
          />
        )}

        {/* Mode selector */}
        <div>
          <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1.5">{t('mode')}</label>
          <div className="grid grid-cols-2 gap-2">
            <button
              onClick={() => setMode('local')}
              className={`py-2 px-3 rounded-lg border text-xs font-medium transition-colors ${mode === 'local' ? 'border-primary-500 bg-primary-50 text-primary-700 dark:bg-primary-900/30 dark:text-primary-300' : 'border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:border-gray-400'}`}
            >
              {t('localMode')}
            </button>
            <button
              onClick={() => setMode('cloud')}
              disabled={!session}
              className={`py-2 px-3 rounded-lg border text-xs font-medium transition-colors ${mode === 'cloud' ? 'border-primary-500 bg-primary-50 text-primary-700 dark:bg-primary-900/30 dark:text-primary-300' : 'border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:border-gray-400'} disabled:opacity-50 disabled:cursor-not-allowed`}
              title={!session ? t('signInOrRedeem') : undefined}
            >
              {t('cloudMode')}
            </button>
          </div>
        </div>

        {mode === 'cloud' && session && (
          <div className="grid grid-cols-2 gap-2">
            <Select
              label={t('profile')}
              value={profile}
              onChange={setProfile}
              options={[
                { value: 'standard', label: t('profiles.standard') },
                { value: 'academic', label: t('profiles.academic') },
                { value: 'publication', label: t('profiles.publication') },
              ]}
            />
            <Select
              label={t('quality')}
              value={quality}
              onChange={setQuality}
              options={[
                { value: 'preview', label: t('qualities.preview') },
                { value: 'balanced', label: t('qualities.balanced') },
                { value: 'strict', label: t('qualities.strict') },
              ]}
            />
          </div>
        )}

        <Button
          onClick={handleConvert}
          disabled={!source || isConverting}
          isLoading={isConverting}
          className="w-full"
          size="md"
        >
          {isConverting ? conversion.message : t('convert')}
        </Button>

        {isConverting && (
          <div className="space-y-1.5">
            <Progress value={conversion.progress} showLabel />
            <p className="text-xs text-gray-500 text-center">{conversion.message}</p>
          </div>
        )}

        {conversion.stage === 'completed' && (
          <Toast type="success" title={t('conversionComplete')} onClose={() => setConversion({ stage: 'idle', progress: 0, message: '' })}>
            {conversion.message}
          </Toast>
        )}
        {conversion.stage === 'failed' && (
          <Toast type="error" title={t('conversionFailed')} onClose={() => setConversion({ stage: 'idle', progress: 0, message: '' })}>
            {conversion.error || t('errors.unknown')}
          </Toast>
        )}
      </Card>

      {/* Recent Jobs */}
      {recentJobs.length > 0 && (
        <Card className="space-y-2">
          <h3 className="text-xs font-medium text-gray-700 dark:text-gray-300">{t('currentJob')}</h3>
          <div className="space-y-1.5">
            {recentJobs.map((job) => (
              <div
                key={job.id}
                className="flex items-center justify-between py-1.5 border-b border-gray-100 dark:border-gray-700 last:border-0"
              >
                <div className="flex-1 min-w-0">
                  <p className="text-xs font-medium text-gray-900 dark:text-white truncate">{job.file_name}</p>
                  <p className="text-[11px] text-gray-500">{job.main_tex}</p>
                </div>
                <Badge
                  variant={
                    job.status === 'completed'
                      ? 'success'
                      : job.status === 'failed'
                        ? 'error'
                        : job.status === 'processing'
                          ? 'warning'
                          : 'default'
                  }
                >
                  {t(`jobStatus.${job.status}`)}
                </Badge>
              </div>
            ))}
          </div>
        </Card>
      )}

      <div className="flex justify-between pt-1 border-t border-gray-200 dark:border-gray-700">
        <Button variant="ghost" size="sm" onClick={openSettings}>
          {t('settings')}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => browser.sidebarAction?.open?.()}
        >
          {t('jobs')}
        </Button>
      </div>
    </div>
  );
}