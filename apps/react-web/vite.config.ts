import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

declare const process: {
  env: Record<string, string | undefined>;
};

const apiTarget = process.env.TEX2DOC_REACT_API_TARGET ?? 'http://127.0.0.1:2624';

export default defineConfig({
  plugins: [react()],
  server: {
    port: 2630,
    proxy: {
      '/v1': {
        target: apiTarget,
        changeOrigin: true,
      },
      '/api': {
        target: apiTarget,
        changeOrigin: true,
      },
      '/admin/v1': {
        target: apiTarget,
        changeOrigin: true,
      },
    },
  },
  build: {
    outDir: 'dist',
    sourcemap: false,
  },
});
