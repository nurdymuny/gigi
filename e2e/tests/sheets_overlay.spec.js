// @ts-check
import { test, expect } from '@playwright/test';

/**
 * S2 e2e for GIGI Sheets — geometry overlay.
 *
 * Confirms the end-to-end flow:
 *   - κ is computed client-side from the response (no engine support
 *     needed for per-row κ yet)
 *   - anomaly rows get a kappa-bad class + tinted background
 *   - toggling the overlay updates body[data-overlay]
 *   - changing the cover field recomputes κ on the fly
 *   - editing a cell flips its κ class (optimistic recompute via the
 *     React render cycle)
 */

const SHEETS_URL =
  process.env.SHEETS_URL || 'http://localhost:5177/gigi/sheets/sensors';

// Schema with a categorical cover field + two numeric fiber fields,
// matching the mockup's `sensors` shape.
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

// One outlier (S-0142) in site North-3; three peers tight in (temp, humidity).
const fixtureSection = {
  data: [
    { sensor_id: 'S-0142', site_id: 'North-3', temp: 38.7, humidity: 18.2 },
    { sensor_id: 'S-0117', site_id: 'North-3', temp: 21.9, humidity: 62.4 },
    { sensor_id: 'S-0201', site_id: 'North-3', temp: 21.8, humidity: 63.1 },
    { sensor_id: 'S-0210', site_id: 'North-3', temp: 24.1, humidity: 55.4 },
  ],
  total: 4,
  curvature: 1.2,
  confidence: 0.45,
};

test.describe('GIGI Sheets — geometry overlay', () => {
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

  test('marks the outlier with data-kappa-class="bad" and tints its row when overlay is on', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    const outlierRow = page.locator('[data-testid="grid-row"][data-row-key="S-0142"]');
    await expect(outlierRow).toHaveAttribute('data-kappa-class', 'bad');
    await expect(outlierRow).toHaveClass(/kappa-bad/);

    // Peers in a tight cohort-of-4 with one outlier will drift to "warn"
    // because the outlier inflates their leave-one-out centroid distance.
    // The strong claim we make is: the outlier is the ONLY "bad" row.
    for (const id of ['S-0117', 'S-0201', 'S-0210']) {
      const row = page.locator(`[data-testid="grid-row"][data-row-key="${id}"]`);
      await expect(row).not.toHaveAttribute('data-kappa-class', 'bad');
    }

    // Toolbar surfaces the anomaly count
    await expect(page.getByTestId('anom-count')).toContainText('1 anomaly');
  });

  test('overlay toggle flips body[data-overlay] and updates aria-pressed', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    // App boots with overlay on.
    const body = page.locator('body');
    await expect(body).toHaveAttribute('data-overlay', 'on');
    const toggle = page.getByTestId('overlay-toggle');
    await expect(toggle).toHaveAttribute('aria-pressed', 'true');

    await toggle.click();
    await expect(body).toHaveAttribute('data-overlay', 'off');
    await expect(toggle).toHaveAttribute('aria-pressed', 'false');

    await toggle.click();
    await expect(body).toHaveAttribute('data-overlay', 'on');
    await expect(toggle).toHaveAttribute('aria-pressed', 'true');
  });

  test('changing the cover field recomputes κ', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    // Default cover field is the first categorical: site_id.
    const select = page.getByTestId('cover-field-select');
    await expect(select).toHaveValue('site_id');

    // With cover=site_id, S-0142 is bad.
    const outlier = page.locator('[data-testid="grid-row"][data-row-key="S-0142"]');
    await expect(outlier).toHaveAttribute('data-kappa-class', 'bad');

    // Switching to the primary key makes every row its own cohort →
    // κ = 0 for everything, all rows become "ok".
    await select.selectOption('sensor_id');
    await expect(outlier).toHaveAttribute('data-kappa-class', 'ok');
    // Anomaly count drops to 0 → chip disappears
    await expect(page.getByTestId('anom-count')).toBeHidden();
  });

  test('editing the outlier into healthy range flips it from bad to ok (optimistic κ recompute)', async ({ page }) => {
    await page.route(/\/v1\/bundles\/sensors\/update/, async (route) => {
      const body = JSON.parse(route.request().postData() || '{}');
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          status: 'updated',
          data: { sensor_id: body.key.sensor_id, ...body.fields, site_id: 'North-3', humidity: 60 },
          total: 4,
          curvature: 0.1,
          confidence: 0.9,
        }),
      });
    });

    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    const outlier = page.locator('[data-testid="grid-row"][data-row-key="S-0142"]');
    await expect(outlier).toHaveAttribute('data-kappa-class', 'bad');

    // Bring temp into cohort range
    await outlier.locator('[data-field="temp"]').click();
    const input = page.getByTestId('cell-editor-input');
    await input.fill('22.0');
    await input.press('Enter');

    // Optimistic κ recompute via the React render cycle: should become ok.
    await expect(outlier).toHaveAttribute('data-kappa-class', 'ok');
    await expect(page.getByTestId('anom-count')).toBeHidden();
  });
});
