/**
 * ThemeProvider - React Context for theme management
 */

/* eslint-disable react-refresh/only-export-components */
import React, { createContext, useContext, useState, useEffect, useCallback, useMemo } from 'react';
import { getSettings, saveSettings } from '@/state/settings-store';

export type Theme = 'light' | 'dark' | 'system';
export type ResolvedTheme = 'light' | 'dark';

export interface ThemeContextValue {
  theme: Theme;
  resolvedTheme: ResolvedTheme;
  setTheme: (theme: Theme) => Promise<void>;
  isDark: boolean;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

interface ThemeProviderProps {
  children: React.ReactNode;
  defaultTheme?: Theme;
}

function getSystemTheme(): ResolvedTheme {
  if (typeof window !== 'undefined' && window.matchMedia) {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }
  return 'light';
}

function resolveTheme(theme: Theme): ResolvedTheme {
  if (theme === 'system') {
    return getSystemTheme();
  }
  return theme;
}

export const ThemeProvider: React.FC<ThemeProviderProps> = ({
  children,
  defaultTheme = 'system',
}) => {
  const [theme, setThemeState] = useState<Theme>(defaultTheme);
  const [resolvedTheme, setResolvedTheme] = useState<ResolvedTheme>(() => resolveTheme(defaultTheme));
  const [isInitialized, setIsInitialized] = useState(false);

  useEffect(() => {
    const loadTheme = async () => {
      try {
        const settings = await getSettings();
        const savedTheme = settings.theme || defaultTheme;
        const resolved = resolveTheme(savedTheme as Theme);
        setThemeState(savedTheme as Theme);
        setResolvedTheme(resolved);
        applyTheme(resolved);
      } catch (error) {
        console.warn('[ThemeProvider] Failed to load theme from settings:', error);
      } finally {
        setIsInitialized(true);
      }
    };
    loadTheme();
  }, [defaultTheme]);

  useEffect(() => {
    if (!isInitialized) return;

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handler = () => {
      if (theme === 'system') {
        const newResolved = getSystemTheme();
        setResolvedTheme(newResolved);
        applyTheme(newResolved);
      }
    };

    mediaQuery.addEventListener('change', handler);
    return () => mediaQuery.removeEventListener('change', handler);
  }, [theme, isInitialized]);

  const applyTheme = (resolved: ResolvedTheme) => {
    const root = document.documentElement;
    if (resolved === 'dark') {
      root.classList.add('dark');
    } else {
      root.classList.remove('dark');
    }
  };

  useEffect(() => {
    if (isInitialized) {
      applyTheme(resolvedTheme);
    }
  }, [resolvedTheme, isInitialized]);

  const setTheme = useCallback(async (newTheme: Theme) => {
    setThemeState(newTheme);
    const resolved = resolveTheme(newTheme);
    setResolvedTheme(resolved);
    applyTheme(resolved);

    try {
      await saveSettings({ theme: newTheme });
    } catch (error) {
      console.warn('[ThemeProvider] Failed to save theme to settings:', error);
    }
  }, []);

  const value = useMemo<ThemeContextValue>(() => ({
    theme,
    resolvedTheme,
    setTheme,
    isDark: resolvedTheme === 'dark',
  }), [theme, resolvedTheme, setTheme]);

  if (!isInitialized) {
    return (
      <ThemeContext.Provider value={value}>
        {children}
      </ThemeContext.Provider>
    );
  }

  return (
    <ThemeContext.Provider value={value}>
      {children}
    </ThemeContext.Provider>
  );
};

export const useTheme = (): ThemeContextValue => {
  const context = useContext(ThemeContext);
  if (!context) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }
  return context;
};

export default ThemeProvider;
