/**
 * Design Tokens for Tex2Doc Extension UI
 */

export const tokens = {
  // Colors
  colors: {
    primary: {
      50: '#f0f9ff',
      100: '#e0f2fe',
      200: '#bae6fd',
      300: '#7dd3fc',
      400: '#38bdf8',
      500: '#0ea5e9',
      600: '#0284c7',
      700: '#0369a1',
      800: '#075985',
      900: '#0c4a6e',
      950: '#082f49',
    },
    accent: {
      50: '#fdf4ff',
      100: '#fae8ff',
      200: '#f5d0fe',
      300: '#f0abfc',
      400: '#e879f9',
      500: '#d946ef',
      600: '#c026d3',
      700: '#a21caf',
      800: '#86198f',
      900: '#701a75',
      950: '#4a044e',
    },
    gray: {
      50: '#f9fafb',
      100: '#f3f4f6',
      200: '#e5e7eb',
      300: '#d1d5db',
      400: '#9ca3af',
      500: '#6b7280',
      600: '#4b5563',
      700: '#374151',
      800: '#1f2937',
      900: '#111827',
      950: '#030712',
    },
    success: '#22c55e',
    warning: '#f59e0b',
    error: '#ef4444',
    info: '#3b82f6',
  },

  // Surface tokens (light + dark) for commercial-grade calm SaaS look
  surface: {
    light: {
      bg: '#ffffff',
      bgMuted: '#f9fafb',
      bgSubtle: '#f3f4f6',
      border: '#e5e7eb',
      borderStrong: '#d1d5db',
      text: '#111827',
      textMuted: '#6b7280',
      textSubtle: '#9ca3af',
    },
    dark: {
      bg: '#0b1220',
      bgMuted: '#111827',
      bgSubtle: '#1f2937',
      border: '#1f2937',
      borderStrong: '#374151',
      text: '#f9fafb',
      textMuted: '#9ca3af',
      textSubtle: '#6b7280',
    },
  },

  // Typography
  typography: {
    fontFamily: {
      sans: 'Inter, system-ui, -apple-system, sans-serif',
      mono: 'JetBrains Mono, Consolas, monospace',
    },
    fontSize: {
      xs: '0.75rem',
      sm: '0.875rem',
      base: '1rem',
      lg: '1.125rem',
      xl: '1.25rem',
      '2xl': '1.5rem',
      '3xl': '1.875rem',
    },
    fontWeight: {
      normal: '400',
      medium: '500',
      semibold: '600',
      bold: '700',
    },
    lineHeight: {
      tight: '1.25',
      normal: '1.5',
      relaxed: '1.75',
    },
    letterSpacing: {
      tight: '-0.01em',
      normal: '0',
      wide: '0.04em',
    },
  },

  // Spacing
  spacing: {
    px: '1px',
    0: '0',
    0.5: '0.125rem',
    1: '0.25rem',
    1.5: '0.375rem',
    2: '0.5rem',
    2.5: '0.625rem',
    3: '0.75rem',
    3.5: '0.875rem',
    4: '1rem',
    5: '1.25rem',
    6: '1.5rem',
    8: '2rem',
    10: '2.5rem',
    12: '3rem',
    16: '4rem',
  },

  // Border radius
  borderRadius: {
    none: '0',
    sm: '0.25rem',
    DEFAULT: '0.375rem',
    md: '0.5rem',
    lg: '0.75rem',
    xl: '1rem',
    '2xl': '1.5rem',
    full: '9999px',
  },

  // Shadows
  shadows: {
    none: 'none',
    sm: '0 1px 2px 0 rgb(0 0 0 / 0.05)',
    DEFAULT: '0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1)',
    md: '0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1)',
    lg: '0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1)',
    xl: '0 20px 25px -5px rgb(0 0 0 / 0.1), 0 8px 10px -6px rgb(0 0 0 / 0.1)',
    focus: '0 0 0 3px rgb(14 165 233 / 0.35)',
  },

  // Motion
  motion: {
    durationFast: '120ms',
    duration: '180ms',
    durationSlow: '240ms',
    easeStandard: 'cubic-bezier(0.2, 0, 0, 1)',
    easeOut: 'cubic-bezier(0, 0, 0.2, 1)',
  },

  // Transitions
  transitions: {
    fast: '150ms',
    DEFAULT: '200ms',
    slow: '300ms',
  },

  // Layout sizes
  layout: {
    popup: {
      width: '480px',
      minHeight: '560px',
      maxHeight: '600px',
    },
    sidepanel: {
      width: '100%',
      maxWidth: '420px',
    },
    options: {
      width: '100%',
      maxWidth: '880px',
    },
  },

  // Content script badge
  badge: {
    small: '20px',
    medium: '24px',
    large: '32px',
  },
} as const;

export type ColorToken = keyof typeof tokens.colors;
export type SpacingToken = keyof typeof tokens.spacing;
