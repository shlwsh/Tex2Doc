import { defineConfig } from 'wxt';
import react from '@vitejs/plugin-react';
import path from 'path';
import { fileURLToPath } from 'url';
import fs from 'fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = __dirname;
const srcDir = path.resolve(rootDir, 'src');

export default defineConfig({
  srcDir,

  outDirTemplate: "{{browser}}-mv{{manifestVersion}}{{modeSuffix}}",

  modules: ['@wxt-dev/module-react'],

  vite: () => ({
    plugins: [react()],
    build: {
      modulePreload: false,
    },
  }),

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

  manifest: () => {
    // API base URL from env (supports localhost for dev, api.tex2doc.cn for prod)
    const apiBaseUrl = import.meta.env.VITE_API_BASE_URL || 'https://api.tex2doc.cn';
    const apiHost = apiBaseUrl.replace(/^https?:\/\//, '').replace(/\/.*$/, '');
    return {
      name: 'Tex2Doc - LaTeX to Word',
      short_name: 'Tex2Doc',
      version: '0.1.0',
      description: 'Convert LaTeX documents to Word (.docx) directly in your browser',
      author: 'Tex2Doc Project',
      homepage_url: 'https://tex2doc.cn',
      support_url: 'https://tex2doc.cn/support',
      privacy_policy_url: 'https://tex2doc.cn/privacy',
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
      permissions: ['storage', 'downloads', 'contextMenus', 'notifications', 'alarms'],
      content_security_policy: {
        extension_pages: "script-src 'self' 'wasm-unsafe-eval'; worker-src 'self'; object-src 'self'",
      },
      host_permissions: [`https://${apiHost}/*`, `http://${apiHost}/*`],
      optional_host_permissions: [
        'https://www.overleaf.com/*',
        'https://*.overleaf.com/*',
        'https://arxiv.org/*',
        'https://*.arxiv.org/*',
      ],
    };
  },

});
