// @ts-check
import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  timeout: 60_000,       // demo takes ~3s, give plenty of room
  retries: 0,            // no retries — we want to see failures clearly
  reporter: [['list'], ['html', { open: 'never' }]],

  use: {
    baseURL: 'http://localhost:3142',
    // Capture traces on failure for debugging
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'on-first-retry',

    // Don't use headless by default so you can watch it run
    // Switch to headless: true for CI
    headless: true,
  },

  // Auto-start dev servers for tests that need them.
  // gigi-stream is NOT started here — Rust binary, user runs it manually.
  // Sheets is a Vite dev server we can boot on demand. reuseExistingServer
  // keeps it cheap if you already have `npm run dev` going.
  webServer: [
    {
      command: 'npm --prefix ../sheets run dev -- --no-open',
      url: 'http://localhost:5177/gigi/sheets/',
      reuseExistingServer: true,
      timeout: 60_000,
      stdout: 'pipe',
      stderr: 'pipe',
    },
  ],
});
