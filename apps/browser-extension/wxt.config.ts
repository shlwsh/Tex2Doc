import { defineConfig } from 'wxt';
import react from '@vitejs/plugin-react';
import { resolve } from 'path';

export default defineConfig({
  plugins: [react()],

  alias: {
    '@': resolve(__dirname, 'src'),
    '@shared': resolve(__dirname, 'src/shared'),
    '@api': resolve(__dirname, 'src/api'),
    '@browser': resolve(__dirname, 'src/browser'),
    '@state': resolve(__dirname, 'src/state'),
    '@ui': resolve(__dirname, 'src/ui'),
  },

  entrypointsDir: './src/entrypoints',

  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          'vendor-react': ['react', 'react-dom'],
          'vendor-polyfill': ['webextension-polyfill'],
          'vendor-state': ['zustand', 'idb'],
        },
      },
    },
  },

  tailwind: {
    darkMode: 'class',
    content: ['./src/**/*.{js,jsx,ts,tsx}', './src/**/*.html'],
  },

  manifest: {
    name: 'Tex2Doc - LaTeX to Word',
    version: '0.1.0',
    description: 'Convert LaTeX documents to Word (.docx) directly in your browser',
    icons: {
      16: '/icons/icon16.png',
      32: '/icons/icon32.png',
      48: '/icons/icon48.png',
      128: '/icons/icon128.png',
    },
    action: {
      default_popup: 'popup/index.html',
      default_title: 'Tex2Doc',
    },
    permissions: ['storage', 'downloads', 'contextMenus', 'notifications'],
    host_permissions: ['https://api.tex2doc.cn/*'],
    optional_host_permissions: [
      'https://www.overleaf.com/*',
      'https://*.overleaf.com/*',
      'https://arxiv.org/*',
      'https://*.arxiv.org/*',
    ],
  },
});
