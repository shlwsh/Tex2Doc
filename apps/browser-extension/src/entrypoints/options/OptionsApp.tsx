/**
 * Options Settings Page
 */

import React, { useState, useEffect } from 'react';
import Button from '@/ui/components/Button';
import Input from '@/ui/components/Input';
import Select from '@/ui/components/Select';
import Card from '@/ui/components/Card';
import Toast from '@/ui/components/Toast';
import { getSettings, saveSettings, type ExtensionSettings } from '@/state/settings-store';
import { t, type Locale } from '@/ui/i18n';

export default function OptionsApp() {
  const [locale, setLocale] = useState<Locale>('en');
  const [settings, setSettings] = useState<ExtensionSettings | null>(null);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load settings on mount
  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const s = await getSettings();
      setSettings(s);
      setLocale(s.language);
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
    if (!confirm('Reset all settings to defaults?')) return;

    try {
      const defaults: ExtensionSettings = {
        api_base_url: 'https://api.tex2doc.cn',
        default_profile: 'standard',
        default_quality: 'balanced',
        default_mode: 'auto',
        wasm_file_size_limit: 10 * 1024 * 1024,
        language: 'en',
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

  if (!settings) {
    return (
      <div className="min-h-screen bg-gray-50 dark:bg-gray-900 flex items-center justify-center">
        <p className="text-gray-500">Loading...</p>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900">
      <div className="max-w-2xl mx-auto p-6 space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
            {t(locale, 'settings')}
          </h1>
          <div className="flex gap-2">
            <Select
              value={locale}
              onChange={(v) => {
                setLocale(v as Locale);
                setSettings({ ...settings, language: v as 'en' | 'zh' });
              }}
              options={[
                { value: 'en', label: 'English' },
                { value: 'zh', label: '中文' },
              ]}
            />
          </div>
        </div>

        {/* API Settings */}
        <Card>
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
            API {t(locale, 'settings')}
          </h2>
          <div className="space-y-4">
            <Input
              label={t(locale, 'apiBaseUrl')}
              value={settings.api_base_url}
              onChange={(e) =>
                setSettings({ ...settings, api_base_url: e.target.value })
              }
              placeholder="https://api.tex2doc.cn"
            />
          </div>
        </Card>

        {/* Default Conversion Settings */}
        <Card>
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
            {t(locale, 'defaultSettings')}
          </h2>
          <div className="grid grid-cols-2 gap-4">
            <Select
              label={t(locale, 'profile')}
              value={settings.default_profile}
              onChange={(v) =>
                setSettings({ ...settings, default_profile: v })
              }
              options={[
                { value: 'standard', label: t(locale, 'profiles.standard') },
                { value: 'academic', label: t(locale, 'profiles.academic') },
                { value: 'publication', label: t(locale, 'profiles.publication') },
              ]}
            />
            <Select
              label={t(locale, 'quality')}
              value={settings.default_quality}
              onChange={(v) =>
                setSettings({ ...settings, default_quality: v })
              }
              options={[
                { value: 'preview', label: t(locale, 'qualities.preview') },
                { value: 'balanced', label: t(locale, 'qualities.balanced') },
                { value: 'strict', label: t(locale, 'qualities.strict') },
              ]}
            />
            <Select
              label={t(locale, 'mode')}
              value={settings.default_mode}
              onChange={(v) =>
                setSettings({ ...settings, default_mode: v as 'auto' | 'local' | 'cloud' })
              }
              options={[
                { value: 'auto', label: t(locale, 'autoMode') },
                { value: 'local', label: t(locale, 'localMode') },
                { value: 'cloud', label: t(locale, 'cloudMode') },
              ]}
            />
            <Select
              label={t(locale, 'theme')}
              value={settings.theme}
              onChange={(v) =>
                setSettings({ ...settings, theme: v as 'light' | 'dark' | 'system' })
              }
              options={[
                { value: 'light', label: t(locale, 'themes.light') },
                { value: 'dark', label: t(locale, 'themes.dark') },
                { value: 'system', label: t(locale, 'themes.system') },
              ]}
            />
          </div>
        </Card>

        {/* WASM Settings */}
        <Card>
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
            {t(locale, 'localMode')}
          </h2>
          <div className="space-y-4">
            <Input
              label={t(locale, 'fileSizeLimit')}
              type="number"
              value={(settings.wasm_file_size_limit / (1024 * 1024)).toString()}
              onChange={(e) =>
                setSettings({
                  ...settings,
                  wasm_file_size_limit: parseInt(e.target.value) * 1024 * 1024,
                })
              }
              helperText="Size in MB"
            />
          </div>
        </Card>

        {/* Advanced Settings */}
        <Card>
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
            Advanced
          </h2>
          <div className="space-y-4">
            <Input
              label="Polling Interval (ms)"
              type="number"
              value={settings.polling_interval.toString()}
              onChange={(e) =>
                setSettings({
                  ...settings,
                  polling_interval: parseInt(e.target.value),
                })
              }
              helperText="Cloud conversion polling interval in milliseconds"
            />
          </div>
        </Card>

        {/* Actions */}
        <div className="flex gap-3">
          <Button onClick={handleSave}>{t(locale, 'save')}</Button>
          <Button variant="secondary" onClick={handleReset}>
            Reset to Defaults
          </Button>
        </div>

        {/* Notifications */}
        {saved && (
          <Toast type="success" title={t(locale, 'success')}>
            Settings saved successfully
          </Toast>
        )}

        {error && (
          <Toast type="error" title={t(locale, 'error')} onClose={() => setError(null)}>
            {error}
          </Toast>
        )}

        {/* Version Info */}
        <div className="text-center text-sm text-gray-500">
          <p>Tex2Doc Extension v{browser.runtime.getManifest().version}</p>
        </div>
      </div>
    </div>
  );
}
