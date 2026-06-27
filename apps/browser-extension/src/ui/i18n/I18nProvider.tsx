/**
 * I18nProvider - React Context for internationalization
 */

import React, { createContext, useContext, useState, useEffect, useCallback } from 'react';
import { translations, Locale, t as translate } from './index';
import { getSettings, saveSettings } from '@/state/settings-store';

export interface I18nContextValue {
  locale: Locale;
  setLocale: (locale: Locale) => Promise<void>;
  t: (key: string, params?: Record<string, string | number>) => string;
}

const I18nContext = createContext<I18nContextValue | null>(null);

interface I18nProviderProps {
  children: React.ReactNode;
  defaultLocale?: Locale;
}

export const I18nProvider: React.FC<I18nProviderProps> = ({
  children,
  defaultLocale = 'en',
}) => {
  const [locale, setLocaleState] = useState<Locale>(defaultLocale);
  const [isInitialized, setIsInitialized] = useState(false);

  useEffect(() => {
    const loadLocale = async () => {
      try {
        const settings = await getSettings();
        const savedLocale = settings.language || defaultLocale;
        if (savedLocale !== locale) {
          setLocaleState(savedLocale as Locale);
        }
      } catch (error) {
        console.warn('[I18nProvider] Failed to load locale from settings:', error);
      } finally {
        setIsInitialized(true);
      }
    };
    loadLocale();
  }, [defaultLocale]);

  const setLocale = useCallback(async (newLocale: Locale) => {
    setLocaleState(newLocale);
    try {
      await saveSettings({ language: newLocale });
    } catch (error) {
      console.warn('[I18nProvider] Failed to save locale to settings:', error);
    }
  }, []);

  const t = useCallback(
    (key: string, params?: Record<string, string | number>): string => {
      return translate(locale, key, params);
    },
    [locale]
  );

  const value: I18nContextValue = {
    locale,
    setLocale,
    t,
  };

  if (!isInitialized) {
    return (
      <I18nContext.Provider value={value}>
        {children}
      </I18nContext.Provider>
    );
  }

  return (
    <I18nContext.Provider value={value}>
      {children}
    </I18nContext.Provider>
  );
};

export const useI18n = (): I18nContextValue => {
  const context = useContext(I18nContext);
  if (!context) {
    throw new Error('useI18n must be used within an I18nProvider');
  }
  return context;
};

export default I18nProvider;
