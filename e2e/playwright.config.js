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
});
