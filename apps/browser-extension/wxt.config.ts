import { defineConfig } from 'wxt';
import react from '@vitejs/plugin-react';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = __dirname;
const srcDir = path.resolve(rootDir, 'src');

export default defineConfig({
  srcDir,

  outBaseDir: '.output',

  outDirTemplate: "{{browser}}-mv{{manifestVersion}}{{modeSuffix}}",

  modules: ['@wxt-dev/module-react'],

  plugins: [react()],

  alias: {
    '@': srcDir,
    '@shared': path.join(srcDir, 'shared'),
    '@api': path.join(srcDir, 'api'),
    '@browser': path.join(srcDir, 'browser'),
    '@state': path.join(srcDir, 'state'),
    '@ui': path.join(srcDir, 'ui'),
  },

  entrypointsDir: path.join(srcDir, 'entrypoints'),

  publicDir: path.join(rootDir, 'public'),

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

  manifest: ({ mode }) => ({
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
      default_popup: 'popup.html',
      default_title: 'Tex2Doc',
    },
    permissions: ['storage', 'downloads', 'contextMenus', 'notifications'],
    content_security_policy: {
      extension_pages: "script-src 'self' 'wasm-unsafe-eval'; object-src 'self' blob:; worker-src 'self' blob:",
    },
    ...(mode === 'edge' ? { side_panel: { default_path: 'sidepanel.html' } } : {}),
    host_permissions: ['https://api.tex2doc.cn/*'],
    optional_host_permissions: [
      'https://www.overleaf.com/*',
      'https://*.overleaf.com/*',
      'https://arxiv.org/*',
      'https://*.arxiv.org/*',
    ],
  }),
});
