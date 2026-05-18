// @ts-check
import { test, expect } from '@playwright/test';

/**
 * S0 e2e for GIGI Sheets.
 *
 * This test does NOT need gigi-stream running — engine calls are mocked
 * with page.route(). It does need the Sheets dev server up on
 * :5177/gigi/sheets/, which playwright.config.js auto-starts via the
 * `webServer` block.
 *
 * Spec reference: GIGI_SHEETS_SPRINT_SPEC.md §5/S0, addendum §Q1.
 */

const SHEETS_URL =
  process.env.SHEETS_URL || 'http://localhost:5177/gigi/sheets/sensors';

const fixtureSchema = {
  name: 'sensors',
  base_fields: [{ name: 'sensor_id', type: 'text' }],
  fiber_fields: [
    { name: 'temp', type: 'numeric' },
    { name: 'humidity', type: 'numeric' },
    { name: 'operator', type: 'text', encryption: 'indexed' },
  ],
  indexed_fields: ['sensor_id'],
  records: 3,
  storage_mode: 'mmap',
};

const fixtureSection = {
  data: [
    { sensor_id: 'S-001', temp: 22.5, humidity: 60.1, operator: 'opr_d2a8' },
    { sensor_id: 'S-002', temp: 19.3, humidity: 71.4, operator: 'opr_77bc' },
    { sensor_id: 'S-003', temp: 38.7, humidity: 18.2, operator: 'opr_d2a8' },
  ],
  total: 3,
  curvature: 0.42,
  confidence: 0.7,
};

test.describe('GIGI Sheets — boot', () => {
  test.beforeEach(async ({ page }) => {
    await page.route(/\/v1\/bundles\/sensors\/schema/, async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(fixtureSchema),
      });
    });
    await page.route(/\/v1\/bundles\/sensors\/query/, async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(fixtureSection),
      });
    });
  });

  test('renders the topbar with bundle name and server URL', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await expect(page.locator('.brand-name')).toContainText('GIGI Sheets');
    await expect(page.locator('.crumbs')).toContainText('sensors');
    await expect(page.locator('.crumbs')).toContainText('localhost:3142');
  });

  test('renders header cells in schema order (base, then fiber)', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    const headers = page.locator('[data-testid^="header-"]');
    await expect(headers).toHaveCount(4);
    await expect(headers.nth(0)).toHaveAttribute('data-testid', 'header-sensor_id');
    await expect(headers.nth(1)).toHaveAttribute('data-testid', 'header-temp');
    await expect(headers.nth(2)).toHaveAttribute('data-testid', 'header-humidity');
    await expect(headers.nth(3)).toHaveAttribute('data-testid', 'header-operator');
  });

  test('renders one row per section response item', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId('grid-row')).toHaveCount(3);

    // First row keyed by base_fields[0]
    const first = page.getByTestId('grid-row').first();
    await expect(first).toHaveAttribute('data-row-key', 'S-001');
  });

  test('promotes κ̄ and conf̄ from the section response into the topbar', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    const stats = page.locator('.stat-value');
    await expect(stats).toHaveCount(3);
    await expect(stats.nth(0)).toHaveText('3');     // rows
    await expect(stats.nth(1)).toHaveText('0.42');  // κ̄
    await expect(stats.nth(2)).toHaveText('0.70');  // conf̄
  });

  test('marks encrypted fields with a lock icon, never plaintext class', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    // The "operator" column is INDEXED-encrypted in the fixture schema.
    const encCells = page.locator('.grid-cell-enc');
    await expect(encCells.first()).toBeVisible();
    await expect(encCells.first().locator('svg')).toBeVisible();
  });

  test('falls through to the bundle picker when /schema 404s', async ({ page }) => {
    // Re-route schema to 404 — later registration wins.
    // Also mock /v1/bundles so the picker has something to show.
    await page.route(/\/v1\/bundles\/sensors\/schema/, async (route) => {
      await route.fulfill({ status: 404, body: 'not found' });
    });
    await page.route(/\/v1\/bundles$/, async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          { name: 'real_bundle', records: 42, fields: 3 },
        ]),
      });
    });
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('bundle-picker')).toBeVisible({ timeout: 10_000 });
    // The picker explains why we're here.
    const header = page.getByTestId('bundle-picker').locator('header');
    await expect(header).toContainText('sensors');
    await expect(header).toContainText('HTTP 404');
    // And surfaces the other available bundles.
    await expect(page.getByTestId('bundle-pick-real_bundle')).toBeVisible();
  });
});
