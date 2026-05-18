// @ts-check
import { test, expect } from '@playwright/test';

/**
 * S3 e2e — inspector + verb cards.
 *
 * Confirms:
 *   - row selection drives the inspector
 *   - clicking a verb button POSTs the expected query / GETs the right
 *     endpoint
 *   - the typed result card renders (matrix for TRANSPORT, b₀/b₁/χ for
 *     BETTI, bars for SPECTRAL, δφ for HOLONOMY)
 *   - engine errors show up in the alert region without breaking
 */

const SHEETS_URL =
  process.env.SHEETS_URL || 'http://localhost:5177/gigi/sheets/sensors';

const fixtureSchema = {
  name: 'sensors',
  base_fields: [{ name: 'sensor_id', type: 'text' }],
  fiber_fields: [
    { name: 'site_id', type: 'categorical' },
    { name: 'temp', type: 'numeric' },
    { name: 'humidity', type: 'numeric' },
  ],
  indexed_fields: ['sensor_id', 'site_id'],
  records: 4,
  storage_mode: 'mmap',
};

const fixtureSection = {
  data: [
    { sensor_id: 'S-0142', site_id: 'North-3', temp: 38.7, humidity: 18.2 },
    { sensor_id: 'S-0143', site_id: 'North-3', temp: 21.9, humidity: 62.4 },
    { sensor_id: 'S-0144', site_id: 'North-3', temp: 21.8, humidity: 63.1 },
    { sensor_id: 'S-0145', site_id: 'North-3', temp: 24.1, humidity: 55.4 },
  ],
  total: 4,
  curvature: 1.2,
  confidence: 0.45,
};

test.describe('GIGI Sheets — verbs', () => {
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

  test('row selection drives the inspector + 4 gauges visible', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    // App auto-selects the first row (S-0142).
    await expect(page.getByTestId('inspector')).toBeVisible();
    await expect(page.getByTestId('insp-title')).toContainText('S-0142');
    // All 4 gauges present
    await expect(page.getByTestId('gauge-kappa')).toBeVisible();
    await expect(page.getByTestId('gauge-conf')).toBeVisible();
    await expect(page.getByTestId('gauge-capacity')).toBeVisible();
    await expect(page.getByTestId('gauge-lambda1')).toBeVisible();

    // Click row 2 → inspector swaps
    await page.locator('[data-row-key="S-0144"]').click();
    await expect(page.getByTestId('insp-title')).toContainText('S-0144');
  });

  test('SPECTRAL button calls /spectral and renders bars', async ({ page }) => {
    let calls = 0;
    await page.route(/\/v1\/bundles\/sensors\/spectral/, async (route) => {
      calls++;
      expect(route.request().method()).toBe('GET');
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          lambda1: 0.182,
          diameter: 5,
          spectral_capacity: 0.612,
        }),
      });
    });
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('verb-spectral').click();
    await expect(page.getByTestId('result-spectral')).toBeVisible({ timeout: 5_000 });
    await expect(page.getByTestId('bar-λ₁')).toHaveText('0.182');
    await expect(page.getByTestId('bar-diam')).toHaveText('5');
    await expect(page.getByTestId('bar-C')).toHaveText('0.612');
    expect(calls).toBeGreaterThan(0);
  });

  test('BETTI button calls /betti and renders b₀, b₁, χ', async ({ page }) => {
    await page.route(/\/v1\/bundles\/sensors\/betti/, async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ beta_0: 4, beta_1: 2 }),
      });
    });
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('verb-betti').click();
    await expect(page.getByTestId('result-betti')).toBeVisible({ timeout: 5_000 });
    await expect(page.getByTestId('betti-chi')).toHaveText('2');
  });

  test('TRANSPORT button POSTs the expected GQL query and renders a 2×2 matrix', async ({ page }) => {
    let gqlBody = null;
    await page.route(/\/v1\/gql/, async (route) => {
      gqlBody = JSON.parse(route.request().postData() || '{}');
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          rows: [
            {
              dim: 2,
              angle: 0.523,
              matrix: [0.866, -0.5, 0.5, 0.866],
            },
          ],
          count: 1,
        }),
      });
    });
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    await page.getByTestId('verb-transport').click();
    await expect(page.getByTestId('result-transport')).toBeVisible({ timeout: 5_000 });

    // The matrix has exactly 4 cells (2x2).
    const matrix = page.getByTestId('matrix');
    await expect(matrix.locator('.mv')).toHaveCount(4);

    // GQL query shape
    expect(gqlBody?.query).toContain('TRANSPORT sensors FROM');
    expect(gqlBody?.query).toContain("sensor_id='S-0142'");
    expect(gqlBody?.query).toContain('ON FIBER (temp, humidity)');
    // Auto-peer is S-0143 by the prefix+1 heuristic
    expect(gqlBody?.query).toContain("sensor_id='S-0143'");
  });

  test('HOLONOMY button POSTs a HOLONOMY GQL query around the cover field', async ({ page }) => {
    let gqlBody = null;
    await page.route(/\/v1\/gql/, async (route) => {
      gqlBody = JSON.parse(route.request().postData() || '{}');
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          rows: [
            { site_id: 'N', temp: 22, humidity: 60, transport_angle: 0.1 },
            { site_id: 'S', temp: 25, humidity: 55, transport_angle: 0.4 },
            { _type: 'summary', holonomy_angle: 0.5, holonomy_trivial: false },
          ],
          count: 3,
        }),
      });
    });
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    await page.getByTestId('verb-holonomy').click();
    await expect(page.getByTestId('result-holonomy')).toBeVisible({ timeout: 5_000 });
    await expect(page.getByTestId('result-holonomy')).toContainText('2 cohorts');

    expect(gqlBody?.query).toContain('HOLONOMY sensors ON FIBER (temp, humidity)');
    expect(gqlBody?.query).toContain('AROUND site_id');
  });

  test('engine 500 on a verb surfaces in the alert region; grid keeps working', async ({ page }) => {
    await page.route(/\/v1\/bundles\/sensors\/spectral/, async (route) => {
      await route.fulfill({ status: 500, body: 'boom' });
    });
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    await page.getByTestId('verb-spectral').click();
    await expect(page.getByTestId('verb-error')).toBeVisible({ timeout: 5_000 });

    // Grid still functional — can pick another row
    await page.locator('[data-row-key="S-0143"]').click();
    await expect(page.getByTestId('insp-title')).toContainText('S-0143');
  });
});
