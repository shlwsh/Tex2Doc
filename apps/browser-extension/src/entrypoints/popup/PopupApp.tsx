/**
 * Main Popup Application
 */

import React, { useState, useEffect } from 'react';
import Button from '../../ui/components/Button';
import Input from '../../ui/components/Input';
import Badge from '../../ui/components/Badge';
import Progress from '../../ui/components/Progress';
import Card from '../../ui/components/Card';
import Select from '../../ui/components/Select';
import Toast from '../../ui/components/Toast';
import { sendToBackground } from '../../browser/messaging';
import { MESSAGE_TYPES } from '../../shared/constants';
import type { JobRecord, Session, UsageSummary } from '../../shared/types';
import { t, type Locale } from '../../ui/i18n';

type ConversionMode = 'local' | 'cloud';
type ConversionStatus = 'idle' | 'converting' | 'success' | 'error';

interface ConversionState {
  status: ConversionStatus;
  progress: number;
  message: string;
  error?: string;
}

export default function PopupApp() {
  const [locale, setLocale] = useState<Locale>('en');
  const [session, setSession] = useState<Session | null>(null);
  const [usage, setUsage] = useState<UsageSummary | null>(null);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [mainTex, setMainTex] = useState('main.tex');
  const [mode, setMode] = useState<ConversionMode>('local');
  const [profile, setProfile] = useState('standard');
  const [quality, setQuality] = useState('balanced');
  const [conversion, setConversion] = useState<ConversionState>({
    status: 'idle',
    progress: 0,
    message: '',
  });
  const [showLogin, setShowLogin] = useState(false);
  const [loginEmail, setLoginEmail] = useState('');
  const [loginPassword, setLoginPassword] = useState('');
  const [loginError, setLoginError] = useState('');
  const [isLoggingIn, setIsLoggingIn] = useState(false);
  const [recentJobs, setRecentJobs] = useState<JobRecord[]>([]);

  useEffect(() => {
    loadSession();
    loadJobs();
  }, []);

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
        setShowLogin(false);
        setLoginEmail('');
        setLoginPassword('');
        await loadSession();
      } else {
        setLoginError(result.error || 'Login failed');
      }
    } catch (error) {
      setLoginError(error instanceof Error ? error.message : 'Login failed');
    } finally {
      setIsLoggingIn(false);
    }
  };

  const handleLogout = async () => {
    try {
      await sendToBackground({ type: MESSAGE_TYPES.LOGOUT });
      setSession(null);
      setUsage(null);
    } catch (error) {
      console.error('Logout failed:', error);
    }
  };

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      setSelectedFile(file);
      if (file.name.endsWith('.zip')) {
        setMainTex('main.tex');
      }
    }
  };

  const handleConvert = async () => {
    if (!selectedFile) return;

    setConversion({ status: 'converting', progress: 0, message: t(locale, 'converting') });

    try {
      const arrayBuffer = await selectedFile.arrayBuffer();
      const zipBytes = new Uint8Array(arrayBuffer);

      if (mode === 'local') {
        const result = await sendToBackground<{ success: boolean; error?: string }>({
          type: MESSAGE_TYPES.START_WASM_CONVERSION,
          zipBytes: Array.from(zipBytes),
          fileName: selectedFile.name,
          mainTex,
        });

        if (result.success) {
          setConversion({ status: 'success', progress: 100, message: t(locale, 'conversionComplete') });
          await loadJobs();
        } else {
          throw new Error(result.error || 'Conversion failed');
        }
      } else {
        const result = await sendToBackground<{ success: boolean; error?: string }>({
          type: MESSAGE_TYPES.START_CONVERSION,
          fileName: selectedFile.name,
          mainTex,
          profile,
          quality,
          mode: 'cloud',
        });

        if (result.success) {
          setConversion({ status: 'success', progress: 100, message: t(locale, 'conversionComplete') });
          await loadJobs();
        } else {
          throw new Error(result.error || 'Conversion failed');
        }
      }
    } catch (error) {
      setConversion({
        status: 'error',
        progress: 0,
        message: t(locale, 'conversionFailed'),
        error: error instanceof Error ? error.message : 'Unknown error',
      });
    }
  };

  const openSettings = () => {
    browser.runtime.openOptionsPage();
  };

  return (
    <div className="w-full max-w-popup mx-auto p-4 space-y-4 bg-white dark:bg-gray-900 min-h-popup">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <div className="w-8 h-8 bg-primary-600 rounded-lg flex items-center justify-center">
            <span className="text-white font-bold text-sm">T2D</span>
          </div>
          <h1 className="text-lg font-semibold text-gray-900 dark:text-white">{t(locale, 'appName')}</h1>
        </div>
        <div className="flex items-center gap-2">
          {session ? (
            <>
              <Badge variant={usage && usage.count_balance > 0 ? 'success' : 'info'}>
                {usage
                  ? `${Math.max(0, usage.cloud_conversions_limit - usage.cloud_conversions_used)} / ${usage.cloud_conversions_limit}`
                  : '--'}
              </Badge>
              <button onClick={handleLogout} className="text-sm text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">
                {t(locale, 'signOut')}
              </button>
            </>
          ) : (
            <button onClick={() => setShowLogin(true)} className="text-sm text-primary-600 hover:text-primary-700">
              {t(locale, 'signIn')}
            </button>
          )}
        </div>
      </div>

      {/* Login Form */}
      {showLogin && !session && (
        <Card className="animate-fade-in">
          <form onSubmit={handleLogin} className="space-y-3">
            <Input type="email" label={t(locale, 'email')} value={loginEmail} onChange={(e) => setLoginEmail(e.target.value)} placeholder="you@example.com" required />
            <Input type="password" label={t(locale, 'password')} value={loginPassword} onChange={(e) => setLoginPassword(e.target.value)} placeholder="********" required />
            {loginError && <p className="text-sm text-red-600 dark:text-red-400">{loginError}</p>}
            <div className="flex gap-2">
              <Button type="submit" isLoading={isLoggingIn} className="flex-1">{t(locale, 'signIn')}</Button>
              <Button type="button" variant="secondary" onClick={() => setShowLogin(false)}>{t(locale, 'cancel')}</Button>
            </div>
          </form>
        </Card>
      )}

      {/* Conversion Card */}
      <Card className="space-y-4">
        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">{t(locale, 'selectZipFile')}</label>
          <div className="border-2 border-dashed border-gray-300 dark:border-gray-600 rounded-lg p-4 text-center hover:border-primary-400 transition-colors">
            <input type="file" accept=".zip" onChange={handleFileSelect} className="hidden" id="file-input" />
            <label htmlFor="file-input" className="cursor-pointer">
              {selectedFile ? (
                <div className="text-sm">
                  <p className="font-medium text-gray-900 dark:text-white">{selectedFile.name}</p>
                  <p className="text-gray-500">{(selectedFile.size / 1024).toFixed(1)} KB</p>
                </div>
              ) : (
                <div className="text-gray-500 dark:text-gray-400">
                  <svg className="mx-auto h-8 w-8 mb-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                  </svg>
                  <p>{t(locale, 'selectFile')}</p>
                </div>
              )}
            </label>
          </div>
        </div>

        <Input label={t(locale, 'mainTexFile')} value={mainTex} onChange={(e) => setMainTex(e.target.value)} placeholder="main.tex" />

        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">{t(locale, 'mode')}</label>
          <div className="flex gap-2">
            <button onClick={() => setMode('local')} className={`flex-1 py-2 px-3 rounded-lg border text-sm font-medium transition-colors ${mode === 'local' ? 'border-primary-500 bg-primary-50 text-primary-700 dark:bg-primary-900/30 dark:text-primary-300' : 'border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:border-gray-400'}`}>
              {t(locale, 'localMode')}
            </button>
            <button onClick={() => setMode('cloud')} className={`flex-1 py-2 px-3 rounded-lg border text-sm font-medium transition-colors ${mode === 'cloud' ? 'border-primary-500 bg-primary-50 text-primary-700 dark:bg-primary-900/30 dark:text-primary-300' : 'border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:border-gray-400'}`} disabled={!session} title={!session ? t(locale, 'errors.authError') : undefined}>
              {t(locale, 'cloudMode')}
            </button>
          </div>
        </div>

        {mode === 'cloud' && (
          <div className="grid grid-cols-2 gap-3">
            <Select label={t(locale, 'profile')} value={profile} onChange={setProfile} options={[{ value: 'standard', label: t(locale, 'profiles.standard') }, { value: 'academic', label: t(locale, 'profiles.academic') }, { value: 'publication', label: t(locale, 'profiles.publication') }]} />
            <Select label={t(locale, 'quality')} value={quality} onChange={setQuality} options={[{ value: 'preview', label: t(locale, 'qualities.preview') }, { value: 'balanced', label: t(locale, 'qualities.balanced') }, { value: 'strict', label: t(locale, 'qualities.strict') }]} />
          </div>
        )}

        <Button onClick={handleConvert} disabled={!selectedFile || conversion.status === 'converting'} isLoading={conversion.status === 'converting'} className="w-full" size="lg">
          {conversion.status === 'converting' ? t(locale, 'converting') : t(locale, 'convert')}
        </Button>

        {conversion.status === 'converting' && (
          <div className="space-y-2">
            <Progress value={conversion.progress} showLabel />
            <p className="text-sm text-gray-500 text-center">{conversion.message}</p>
          </div>
        )}

        {conversion.status === 'success' && <Toast type="success" title={t(locale, 'conversionComplete')}>{conversion.message}</Toast>}
        {conversion.status === 'error' && <Toast type="error" title={t(locale, 'conversionFailed')}>{conversion.error || conversion.message}</Toast>}
      </Card>

      {/* Recent Jobs */}
      {recentJobs.length > 0 && (
        <Card>
          <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">{t(locale, 'currentJob')}</h3>
          <div className="space-y-2">
            {recentJobs.map((job) => (
              <div key={job.id} className="flex items-center justify-between py-2 border-b border-gray-100 dark:border-gray-700 last:border-0">
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-gray-900 dark:text-white truncate">{job.file_name}</p>
                  <p className="text-xs text-gray-500">{job.main_tex}</p>
                </div>
                <Badge variant={job.status === 'completed' ? 'success' : job.status === 'failed' ? 'error' : job.status === 'processing' ? 'warning' : 'default'}>
                  {t(locale, `jobStatus.${job.status}`)}
                </Badge>
              </div>
            ))}
          </div>
        </Card>
      )}

      {/* Footer Actions */}
      <div className="flex justify-between pt-2 border-t border-gray-200 dark:border-gray-700">
        <Button variant="ghost" size="sm" onClick={openSettings}>{t(locale, 'settings')}</Button>
        <Button variant="ghost" size="sm">{t(locale, 'jobs')}</Button>
      </div>
    </div>
  );
}
