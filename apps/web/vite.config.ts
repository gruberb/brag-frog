/// <reference types="vitest/config" />
import { rmSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

// Cookies and OAuth stay same-origin in dev: every backend path is proxied to the Axum server.
const backend = process.env.BRAGFROG_API_URL ?? 'http://localhost:8080';
const proxy = Object.fromEntries(
  ['/api', '/auth', '/custom', '/healthz'].map((p) => [p, { target: backend, changeOrigin: false }]),
);

export default defineConfig({
  plugins: [
    react(),
    tailwindcss(),
    {
      // The MSW service worker is only for `npm run dev:mock`; it must not ship in the production bundle.
      name: 'drop-msw-worker',
      apply: 'build',
      closeBundle() {
        rmSync(fileURLToPath(new URL('./dist/mockServiceWorker.js', import.meta.url)), { force: true });
      },
    },
  ],
  resolve: { alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) } },
  server: { port: 5173, proxy },
  preview: { proxy },
  build: {
    chunkSizeWarningLimit: 1500,
    rollupOptions: {
      output: {
        // Editor and export libraries are dynamic imports and get their own chunks automatically.
        manualChunks(id: string) {
          if (!id.includes('node_modules')) return undefined;
          if (/node_modules\/(react|react-dom|scheduler)\//.test(id)) return 'react';
          if (/node_modules\/(react-router|@remix-run)\//.test(id)) return 'router';
          if (/node_modules\/@tanstack\/(react-)?query/.test(id)) return 'query';
          return undefined;
        },
      },
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/testing/setup.ts'],
    css: false,
    include: ['src/**/*.test.{ts,tsx}'],
    testTimeout: 15000,
  },
});
