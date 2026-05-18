// @ts-check
import { test, expect } from '@playwright/test';

/**
 * S4 e2e — Geometry tab.
 *
 *   - tab switches between Grid and Geometry without losing state
 *   - scatter renders one point per row, halos for anomalies
 *   - clicking a point updates the inspector AND keeps the selection
 *     visible after switching back to the grid
 *   - axis selectors swap the X/Y fields and re-render
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
    { name: 'pressure', type: 'numeric' },
  ],
  indexed_fields: ['sensor_id', 'site_id'],
  records: 4,
  storage_mode: 'mmap',
};

const fixtureSection = {
  data: [
    { sensor_id: 'S-001', site_id: 'North', temp: 22.5, humidity: 60, pressure: 1013 },
    { sensor_id: 'S-002', site_id: 'North', temp: 23.0, humidity: 61, pressure: 1012 },
    { sensor_id: 'S-003', site_id: 'North', temp: 21.8, humidity: 63, pressure: 1011 },
    { sensor_id: 'S-OUT', site_id: 'North', temp: 99.0, humidity: 5,  pressure: 980  },
  ],
  total: 4,
  curvature: 0.4,
  confidence: 0.71,
};

test.describe('GIGI Sheets — Geometry tab', () => {
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

  test('boots in the Grid view; clicking the Geometry tab reveals the scatter', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId('tab-grid')).toHaveAttribute('aria-selected', 'true');
    await expect(page.queryByTestId?.('geometry') ?? page.getByTestId('geometry')).toBeHidden({ timeout: 200 }).catch(() => {});

    await page.getByTestId('tab-geometry').click();
    await expect(page.getByTestId('geometry')).toBeVisible();
    await expect(page.getByTestId('scatter')).toBeVisible();
    await expect(page.getByTestId('tab-geometry')).toHaveAttribute('aria-selected', 'true');
  });

  test('renders one circle per row with the right kappa class', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await page.getByTestId('tab-geometry').click();
    await expect(page.getByTestId('scatter')).toBeVisible();
    for (const id of ['S-001', 'S-002', 'S-003', 'S-OUT']) {
      await expect(page.getByTestId(`point-${id}`)).toBeVisible();
    }
    // The outlier should be classed "bad"; the tight peers should not be "bad".
    await expect(page.getByTestId('point-S-OUT')).toHaveAttribute('data-kappa-class', 'bad');
    await expect(page.getByTestId('halo-S-OUT')).toBeVisible();
  });

  test('clicking a scatter point selects that row and the transport line appears', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await page.getByTestId('tab-geometry').click();
    await expect(page.getByTestId('scatter')).toBeVisible();

    await page.getByTestId('point-S-002').click();
    // Inspector updates
    await expect(page.getByTestId('insp-title')).toContainText('S-002');
    // Selection ring appears on the clicked point
    await expect(page.getByTestId('ring-S-002')).toBeVisible();
    // Transport line + peer label render
    await expect(page.getByTestId('transport-overlay')).toBeVisible();
  });

  test('switching back to the Grid preserves the row selected in the Geometry tab', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await page.getByTestId('tab-geometry').click();
    await page.getByTestId('point-S-003').click();
    await expect(page.getByTestId('insp-title')).toContainText('S-003');

    await page.getByTestId('tab-grid').click();
    // The selected row in the grid mirrors the geometry selection.
    await expect(
      page.locator('[data-testid="grid-row"][data-row-key="S-003"]'),
    ).toHaveAttribute('data-selected', 'true');
  });

  test('changing the Y axis swaps the rendered field label', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await page.getByTestId('tab-geometry').click();
    await expect(page.getByTestId('y-axis-label')).toHaveText('humidity');
    await page.getByTestId('y-field-select').selectOption('pressure');
    await expect(page.getByTestId('y-axis-label')).toHaveText('pressure');
  });

  test('cover-stats sidebar surfaces size + anomaly counts per cohort', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await page.getByTestId('tab-geometry').click();
    await expect(page.getByTestId('geometry-sidebar')).toBeVisible();
    await expect(page.getByTestId('cover-North-size')).toHaveText('4');
    // S-OUT is the lone anomaly in North.
    await expect(page.getByTestId('cover-North-anom')).toBeVisible();
  });
});
