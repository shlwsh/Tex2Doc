/**
 * Options Settings Page - Commercial UI refactor
 */

import React, { useState, useEffect } from 'react';
import Button from '@/ui/components/Button';
import Input from '@/ui/components/Input';
import Select from '@/ui/components/Select';
import Card from '@/ui/components/Card';
import Toast from '@/ui/components/Toast';
import Badge from '@/ui/components/Badge';
import { Tabs } from '@/ui/components/Tabs';
import { useI18n } from '@/ui/i18n/useI18n';
import { getSettings, saveSettings } from '@/state/settings-store';
import { sendToBackground } from '@/browser/messaging';
import { MESSAGE_TYPES } from '@/shared/constants';
import {
  getDomains,
  saveDomains,
  refreshGrantedFlags,
  toOriginPattern,
  type PersistedDomain,
} from '@/state/domain-store';
import type { ExtensionSettings } from '@/shared/types';

type SettingsTab = 'general' | 'conversion' | 'permissions' | 'about';

export default function OptionsApp() {
  const { t, locale, setLocale } = useI18n();
  const [activeTab, setActiveTab] = useState<SettingsTab>('general');
  const [settings, setSettings] = useState<ExtensionSettings | null>(null);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isExportingFunnel, setIsExportingFunnel] = useState(false);
  const [domains, setDomains] = useState<PersistedDomain[]>([]);
  const [newDomain, setNewDomain] = useState('');
  const [domainError, setDomainError] = useState<string | null>(null);
  const [isDomainsLoading, setIsDomainsLoading] = useState(true);

  useEffect(() => {
    loadSettings();
    loadDomains();
  }, []);

  const loadSettings = async () => {
    try {
      const s = await getSettings();
      setSettings(s);
    } catch (err) {
      setError('Failed to load settings');
    }
  };

  const handleSave = async () => {
    if (!settings) return;
    try {
      await saveSettings(settings);
      setSaved(true);
      setTimeout(() => setSaved(false), 3000);
    } catch (err) {
      setError('Failed to save settings');
    }
  };

  const handleReset = async () => {
    if (!confirm(t('resetConfirm') || 'Reset all settings to defaults?')) return;
    try {
      const defaults: ExtensionSettings = {
        api_base_url: 'https://api.tex2doc.cn',
        default_profile: 'standard',
        default_quality: 'balanced',
        default_mode: 'auto',
        wasm_file_size_limit: 10 * 1024 * 1024,
        language: locale,
        theme: 'system',
        polling_interval: 2000,
      };
      await saveSettings(defaults);
      setSettings(defaults);
      setSaved(true);
    } catch (err) {
      setError('Failed to reset settings');
    }
  };

  const loadDomains = async () => {
    setIsDomainsLoading(true);
    try {
      const stored = await getDomains();
      const reconciled = await refreshGrantedFlags(stored);
      setDomains(reconciled);
      if (reconciled.some((d) => d.granted !== d.enabled)) {
        await saveDomains(reconciled);
      }
    } catch (err) {
      console.warn('[Options] Failed to load domains:', err);
    } finally {
      setIsDomainsLoading(false);
    }
  };

  const handleAddDomain = async () => {
    const trimmed = newDomain.trim().toLowerCase();
    if (!trimmed) return;
    if (!/^([a-z0-9-]+\.)+[a-z]{2,}$/.test(trimmed)) {
      setDomainError('Invalid domain format (e.g. example.com)');
      return;
    }
    if (domains.some((d) => d.domain === trimmed)) {
      setDomainError('Domain already exists');
      return;
    }
    setDomainError(null);
    // Ask the browser to grant the optional host permission. If the user
    // rejects, we don't add to the local list — the prompt is the source
    // of truth.
    let granted = false;
    try {
      granted = await browser.permissions.request({ origins: [toOriginPattern(trimmed)] });
    } catch (err) {
      console.warn('[Options] permissions.request failed:', err);
      granted = false;
    }
    if (!granted) {
      setDomainError(`Permission denied for ${trimmed}`);
      return;
    }
    const next: PersistedDomain = {
      id: Date.now().toString(),
      domain: trimmed,
      enabled: true,
      granted: true,
      updatedAt: Date.now(),
    };
    const updated = [...domains, next];
    setDomains(updated);
    await saveDomains(updated);
    setNewDomain('');
  };

  const handleRemoveDomain = async (id: string) => {
    const target = domains.find((d) => d.id === id);
    if (target) {
      try {
        await browser.permissions.remove({ origins: [toOriginPattern(target.domain)] });
      } catch (err) {
        console.warn('[Options] permissions.remove failed:', err);
      }
    }
    const updated = domains.filter((d) => d.id !== id);
    setDomains(updated);
    await saveDomains(updated);
  };

  const handleToggleDomain = async (id: string) => {
    const target = domains.find((d) => d.id === id);
    if (!target) return;
    let granted = target.granted;
    if (!target.enabled) {
      // Turning ON: ask the browser. Roll back on rejection.
      try {
        granted = await browser.permissions.request({ origins: [toOriginPattern(target.domain)] });
      } catch (err) {
        console.warn('[Options] permissions.request failed:', err);
        granted = false;
      }
      if (!granted) {
        setDomainError(`Permission denied for ${target.domain}`);
        return;
      }
    } else {
      // Turning OFF: revoke via permissions.remove.
      try {
        await browser.permissions.remove({ origins: [toOriginPattern(target.domain)] });
      } catch (err) {
        console.warn('[Options] permissions.remove failed:', err);
      }
      granted = false;
    }
    const updated = domains.map((d) =>
      d.id === id ? { ...d, enabled: !d.enabled, granted, updatedAt: Date.now() } : d
    );
    setDomains(updated);
    await saveDomains(updated);
  };

  const handleExportFunnel = async () => {
    setIsExportingFunnel(true);
    try {
      const result = await sendToBackground<{ success: boolean; filename?: string; error?: string }>({
        type: MESSAGE_TYPES.EXPORT_FUNNEL,
        windowDays: 7,
      });
      if (result?.success) {
        setSaved(true);
        setTimeout(() => setSaved(false), 3000);
      } else {
        setError(result?.error ?? 'Funnel export failed');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Funnel export failed');
    } finally {
      setIsExportingFunnel(false);
    }
  };

  if (!settings) {
    return (
      <div className="min-h-screen bg-gray-50 dark:bg-gray-900 flex items-center justify-center">
        <div className="flex items-center gap-2 text-sm text-gray-500">
          <svg className="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
            <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
            <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
          {t('loading')}
        </div>
      </div>
    );
  }

  const version = browser.runtime.getManifest().version;

  const tabItems = [
    { id: 'general', label: t('settingsTabs.general') },
    { id: 'conversion', label: t('settingsTabs.conversion') },
    { id: 'permissions', label: t('settingsTabs.permissions') },
    { id: 'about', label: t('settingsTabs.about') },
  ];

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900">
      <div className="max-w-3xl mx-auto p-6 space-y-6">
        {/* Toolbar */}
        <div className="flex items-center justify-between bg-white dark:bg-gray-800 rounded-lg p-4 shadow-sm border border-gray-200 dark:border-gray-700">
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 rounded-lg bg-gradient-to-br from-primary-500 to-primary-700 flex items-center justify-center shadow-sm">
              <span className="text-white font-bold text-sm">T2D</span>
            </div>
            <div className="leading-tight">
              <h1 className="text-lg font-semibold text-gray-900 dark:text-white">{t('appName')}</h1>
              <p className="text-xs text-gray-500">{t('settings')}</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <select
              value={locale}
              onChange={(e) => {
                setLocale(e.target.value as typeof locale);
                setSettings({ ...settings, language: e.target.value as 'en' | 'zh' });
              }}
              className="text-xs bg-transparent border border-gray-200 dark:border-gray-700 rounded-md px-2 py-1 text-gray-600 dark:text-gray-300"
            >
              <option value="en">English</option>
              <option value="zh">中文</option>
            </select>
          </div>
        </div>

        <Card>
          <Tabs tabs={tabItems} activeTab={activeTab} onChange={(id) => setActiveTab(id as SettingsTab)} variant="underline" />
        </Card>

        {/* General Tab */}
        {activeTab === 'general' && (
          <Card>
            <h2 className="text-sm font-semibold text-gray-900 dark:text-white mb-4">
              {t('settingsTabs.general')}
            </h2>
            <div className="space-y-4">
              <Input
                label={t('apiBaseUrl')}
                value={settings.api_base_url}
                onChange={(e) => setSettings({ ...settings, api_base_url: e.target.value })}
                placeholder="https://api.tex2doc.cn"
              />
              <Select
                label={t('theme')}
                value={settings.theme}
                onChange={(v) => setSettings({ ...settings, theme: v as 'light' | 'dark' | 'system' })}
                options={[
                  { value: 'light', label: t('themeSettings.light') },
                  { value: 'dark', label: t('themeSettings.dark') },
                  { value: 'system', label: t('themeSettings.system') },
                ]}
              />
            </div>
          </Card>
        )}

        {/* Conversion Defaults Tab */}
        {activeTab === 'conversion' && (
          <Card>
            <h2 className="text-sm font-semibold text-gray-900 dark:text-white mb-4">
              {t('settingsTabs.conversion')}
            </h2>
            <div className="grid grid-cols-2 gap-4">
              <Select
                label={t('defaultMode')}
                value={settings.default_mode}
                onChange={(v) => setSettings({ ...settings, default_mode: v as 'auto' | 'local' | 'cloud' })}
                options={[
                  { value: 'auto', label: t('autoMode') },
                  { value: 'local', label: t('localMode') },
                  { value: 'cloud', label: t('cloudMode') },
                ]}
              />
              <Select
                label={t('defaultProfile')}
                value={settings.default_profile}
                onChange={(v) => setSettings({ ...settings, default_profile: v })}
                options={[
                  { value: 'standard', label: t('profiles.standard') },
                  { value: 'academic', label: t('profiles.academic') },
                  { value: 'publication', label: t('profiles.publication') },
                ]}
              />
              <Select
                label={t('defaultQuality')}
                value={settings.default_quality}
                onChange={(v) => setSettings({ ...settings, default_quality: v })}
                options={[
                  { value: 'preview', label: t('qualities.preview') },
                  { value: 'balanced', label: t('qualities.balanced') },
                  { value: 'strict', label: t('qualities.strict') },
                ]}
              />
              <Input
                label={t('fileSizeLimit')}
                type="number"
                value={(settings.wasm_file_size_limit / (1024 * 1024)).toString()}
                onChange={(e) =>
                  setSettings({ ...settings, wasm_file_size_limit: parseInt(e.target.value) * 1024 * 1024 })
                }
                helperText="Size in MB"
              />
            </div>
          </Card>
        )}

        {/* Permissions Tab */}
        {activeTab === 'permissions' && (
          <Card>
            <h2 className="text-sm font-semibold text-gray-900 dark:text-white mb-1">
              {t('domainPermissions')}
            </h2>
            <p className="text-xs text-gray-500 mb-4">{t('domainPermissionsDescription')}</p>

            <div className="flex gap-2 mb-4">
              <Input
                value={newDomain}
                onChange={(e) => setNewDomain(e.target.value)}
                placeholder={t('domainPlaceholder')}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    void handleAddDomain();
                  }
                }}
              />
              <Button onClick={() => void handleAddDomain()}>{t('domainAdd')}</Button>
            </div>
            {domainError && <p className="text-xs text-red-600 dark:text-red-400 mb-3">{domainError}</p>}

            {isDomainsLoading ? (
              <div className="text-center py-8 text-sm text-gray-500">{t('loading')}</div>
            ) : domains.length === 0 ? (
              <div className="text-center py-8 text-sm text-gray-500">No domains configured yet.</div>
            ) : (
              <ul className="divide-y divide-gray-200 dark:divide-gray-700 border border-gray-200 dark:border-gray-700 rounded-lg">
                {domains.map((d) => (
                  <li key={d.id} className="flex items-center justify-between px-3 py-2">
                    <div className="flex items-center gap-2">
                      <Badge variant={d.enabled && d.granted ? 'success' : d.granted ? 'warning' : 'default'}>
                        {d.domain}
                      </Badge>
                      {d.enabled && !d.granted && (
                        <span className="text-[10px] text-amber-600 dark:text-amber-400">
                          Browser grant missing
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-2">
                      <button
                        onClick={() => void handleToggleDomain(d.id)}
                        className="text-xs text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
                      >
                        {d.enabled ? 'Disable' : 'Enable'}
                      </button>
                      <button
                        onClick={() => void handleRemoveDomain(d.id)}
                        className="text-xs text-red-600 hover:text-red-700 dark:text-red-400"
                      >
                        {t('delete')}
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </Card>
        )}

        {/* About Tab */}
        {activeTab === 'about' && (
          <Card>
            <h2 className="text-sm font-semibold text-gray-900 dark:text-white mb-4">
              {t('settingsTabs.about')}
            </h2>
            <div className="space-y-3 text-sm">
              <div className="flex justify-between">
                <span className="text-gray-500">{t('aboutVersion')}</span>
                <span className="text-gray-900 dark:text-white">v{version}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-500">{t('aboutLinks')}</span>
                <a href="https://tex2doc.cn" target="_blank" rel="noreferrer" className="text-primary-600 hover:underline">
                  tex2doc.cn
                </a>
              </div>
              <div className="pt-3 border-t border-gray-200 dark:border-gray-700 text-xs text-gray-500">
                {t('aboutCopyright')} (c) Tex2Doc
              </div>
              <div className="pt-3 border-t border-gray-200 dark:border-gray-700">
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <p className="text-xs font-medium text-gray-900 dark:text-white">
                      {t('funnel.title')}
                    </p>
                    <p className="text-[11px] text-gray-500 mt-0.5 leading-snug">
                      {t('funnel.description')}
                    </p>
                  </div>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => void handleExportFunnel()}
                    disabled={isExportingFunnel}
                  >
                    {isExportingFunnel ? t('loading') : t('funnel.export')}
                  </Button>
                </div>
              </div>
            </div>
          </Card>
        )}

        {/* Footer actions */}
        <div className="flex gap-3 justify-end pt-2">
          <Button variant="secondary" onClick={handleReset}>
            Reset to Defaults
          </Button>
          <Button onClick={handleSave}>{t('save')}</Button>
        </div>

        {saved && (
          <Toast type="success" title={t('success')} onClose={() => setSaved(false)}>
            Settings saved successfully
          </Toast>
        )}

        {error && (
          <Toast type="error" title={t('error')} onClose={() => setError(null)}>
            {error}
          </Toast>
        )}
      </div>
    </div>
  );
}