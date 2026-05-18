// @ts-check
import { test, expect } from '@playwright/test';

/**
 * Spreadsheet-style interactions:
 *   - Cmd/Ctrl-click toggles a row's selection
 *   - Shift-click extends the selection range from the anchor
 *   - Right-click opens the row context menu with useful actions
 *   - Inspector + Geometry sidebar can be collapsed and re-opened
 *   - Multi-select count appears in the topbar
 */

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
    { sensor_id: 'S-001', site_id: 'North', temp: 22.5, humidity: 60.1 },
    { sensor_id: 'S-002', site_id: 'North', temp: 23.0, humidity: 61.0 },
    { sensor_id: 'S-003', site_id: 'North', temp: 21.8, humidity: 63.1 },
    { sensor_id: 'S-004', site_id: 'South', temp: 50.0, humidity: 10.0 },
  ],
  total: 4,
  curvature: 0.3,
  confidence: 0.8,
};

const SHEETS_URL =
  process.env.SHEETS_URL || 'http://localhost:5177/gigi/sheets/sensors';

async function muteWebSocket(page) {
  await page.addInitScript(() => {
    const w = /** @type {any} */ (window);
    class NullWS {
      constructor() { /** @type {any} */ (this).readyState = 0; }
      addEventListener() {}
      removeEventListener() {}
      send() {}
      close() {}
    }
    w.WebSocket = /** @type {any} */ (NullWS);
  });
}

test.describe('GIGI Sheets — spreadsheet interactions', () => {
  test.beforeEach(async ({ page }) => {
    await muteWebSocket(page);
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

  test('Cmd/Ctrl-click toggles row selection (multi-select)', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    // App auto-selects S-001 on load.
    await expect(
      page.locator('[data-testid="grid-row"][data-row-key="S-001"]'),
    ).toHaveAttribute('data-selected', 'true');

    // Ctrl-click S-003: adds it to selection.
    await page
      .locator('[data-testid="grid-row"][data-row-key="S-003"]')
      .click({ modifiers: ['Control'] });

    await expect(
      page.locator('[data-testid="grid-row"][data-row-key="S-001"]'),
    ).toHaveAttribute('data-selected', 'true');
    await expect(
      page.locator('[data-testid="grid-row"][data-row-key="S-003"]'),
    ).toHaveAttribute('data-selected', 'true');
    // Multi-select count appears.
    await expect(page.getByTestId('multiselect-count')).toContainText('2 selected');

    // Ctrl-click S-003 again: removes it.
    await page
      .locator('[data-testid="grid-row"][data-row-key="S-003"]')
      .click({ modifiers: ['Control'] });
    await expect(
      page.locator('[data-testid="grid-row"][data-row-key="S-003"]'),
    ).toHaveAttribute('data-selected', 'false');
  });

  test('Shift-click extends the range from the anchor', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    // S-001 is anchor (auto-selected). Shift-click S-003 → S-001..S-003 selected.
    await page
      .locator('[data-testid="grid-row"][data-row-key="S-003"]')
      .click({ modifiers: ['Shift'] });

    for (const id of ['S-001', 'S-002', 'S-003']) {
      await expect(
        page.locator(`[data-testid="grid-row"][data-row-key="${id}"]`),
      ).toHaveAttribute('data-selected', 'true');
    }
    // S-004 stays unselected.
    await expect(
      page.locator('[data-testid="grid-row"][data-row-key="S-004"]'),
    ).toHaveAttribute('data-selected', 'false');
    await expect(page.getByTestId('multiselect-count')).toContainText('3 selected');
  });

  test('plain click clears the multi-selection back to one row', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await page
      .locator('[data-testid="grid-row"][data-row-key="S-003"]')
      .click({ modifiers: ['Control'] });
    await expect(page.getByTestId('multiselect-count')).toContainText('2 selected');

    // Plain click on S-002.
    await page
      .locator('[data-testid="grid-row"][data-row-key="S-002"]')
      .click();
    await expect(
      page.locator('[data-testid="grid-row"][data-row-key="S-002"]'),
    ).toHaveAttribute('data-selected', 'true');
    await expect(
      page.locator('[data-testid="grid-row"][data-row-key="S-001"]'),
    ).toHaveAttribute('data-selected', 'false');
    await expect(
      page.locator('[data-testid="grid-row"][data-row-key="S-003"]'),
    ).toHaveAttribute('data-selected', 'false');
    // Multi-select chip gone (only 1 selected).
    await expect(page.getByTestId('multiselect-count')).toBeHidden();
  });

  test('right-click opens a context menu with the row key in the header', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    await page
      .locator('[data-testid="grid-row"][data-row-key="S-002"]')
      .click({ button: 'right' });

    const menu = page.getByTestId('context-menu');
    await expect(menu).toBeVisible();
    await expect(page.getByTestId('context-menu-header')).toContainText('S-002');
    await expect(page.getByTestId('context-menu-copy-id')).toBeVisible();
    await expect(page.getByTestId('context-menu-copy-json')).toBeVisible();
    await expect(page.getByTestId('context-menu-copy-gql')).toBeVisible();
    await expect(page.getByTestId('context-menu-open')).toBeVisible();
  });

  test('right-clicking on an unselected row makes IT the target (Excel-style)', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    // Right-click on S-003 (not currently selected).
    await page
      .locator('[data-testid="grid-row"][data-row-key="S-003"]')
      .click({ button: 'right' });

    // Selection moves to S-003.
    await expect(
      page.locator('[data-testid="grid-row"][data-row-key="S-003"]'),
    ).toHaveAttribute('data-selected', 'true');
    await expect(
      page.locator('[data-testid="grid-row"][data-row-key="S-001"]'),
    ).toHaveAttribute('data-selected', 'false');
    await expect(page.getByTestId('context-menu-header')).toContainText('S-003');
  });

  test('right-clicking inside a multi-selection keeps it intact and shows plural labels', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    // Build a 3-row selection via shift-click.
    await page
      .locator('[data-testid="grid-row"][data-row-key="S-003"]')
      .click({ modifiers: ['Shift'] });
    await expect(page.getByTestId('multiselect-count')).toContainText('3 selected');

    // Right-click any of the selected rows.
    await page
      .locator('[data-testid="grid-row"][data-row-key="S-002"]')
      .click({ button: 'right' });

    await expect(page.getByTestId('context-menu-header')).toContainText('3 rows');
    await expect(page.getByTestId('context-menu-copy-id')).toContainText('Copy 3 row keys');
    await expect(page.getByTestId('context-menu-copy-gql')).toContainText('Copy SECTION query for 3 rows');
  });

  test('Escape closes an open context menu', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await page
      .locator('[data-testid="grid-row"][data-row-key="S-001"]')
      .click({ button: 'right' });
    await expect(page.getByTestId('context-menu')).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.getByTestId('context-menu')).toBeHidden();
  });

  test('inspector toggle hides + shows the panel', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('inspector')).toBeVisible({ timeout: 10_000 });
    const toggle = page.getByTestId('inspector-toggle');
    await toggle.click();
    await expect(page.getByTestId('inspector')).toBeHidden();
    await expect(toggle).toContainText('Show inspector');
    await toggle.click();
    await expect(page.getByTestId('inspector')).toBeVisible();
    await expect(toggle).toContainText('Hide inspector');
  });

  test('geometry sidebar can be collapsed and the scatter takes the full width', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('tab-geometry').click();
    await expect(page.getByTestId('geometry-sidebar')).toBeVisible();

    await page.getByTestId('geometry-sidebar-toggle').click();
    await expect(page.getByTestId('geometry-sidebar')).toBeHidden();
    await page.getByTestId('geometry-sidebar-toggle').click();
    await expect(page.getByTestId('geometry-sidebar')).toBeVisible();
  });
});
