// @ts-check
import { test, expect } from '@playwright/test';

/**
 * S1 e2e for GIGI Sheets — inline edit with optimistic UI + rollback.
 *
 * Engine calls are mocked with page.route() so this runs without
 * gigi-stream. The contract under test:
 *
 *   1. Click a numeric cell → editor opens with current value.
 *   2. Type a new value, press Enter → cell shows new value
 *      *before* the update response settles (optimistic).
 *   3. On success, toast says "Updated …" and the κ̄ stat updates.
 *   4. On failure, cell reverts to the original value and an error
 *      toast is shown.
 *   5. Esc cancels without committing.
 */

const SHEETS_URL =
  process.env.SHEETS_URL || 'http://localhost:5177/gigi/sheets/sensors';

const fixtureSchema = {
  name: 'sensors',
  base_fields: [{ name: 'sensor_id', type: 'text' }],
  fiber_fields: [
    { name: 'temp', type: 'numeric' },
    { name: 'humidity', type: 'numeric' },
  ],
  indexed_fields: ['sensor_id'],
  records: 2,
  storage_mode: 'mmap',
};

const fixtureSection = {
  data: [
    { sensor_id: 'S-001', temp: 22.5, humidity: 60.1 },
    { sensor_id: 'S-002', temp: 19.3, humidity: 71.4 },
  ],
  total: 2,
  curvature: 0.1,
  confidence: 0.9,
};

test.describe('GIGI Sheets — inline edit', () => {
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

  test('optimistic commit: cell updates immediately and toast confirms', async ({ page }) => {
    // Hold the update response so we can observe the optimistic UI.
    let resolveUpdate;
    /** @type {Promise<void>} */
    const gate = new Promise((r) => {
      resolveUpdate = r;
    });
    await page.route(/\/v1\/bundles\/sensors\/update/, async (route) => {
      const reqBody = JSON.parse(route.request().postData() || '{}');
      // Sanity: client sent key + fields properly
      expect(reqBody.key).toEqual({ sensor_id: 'S-001' });
      expect(reqBody.fields).toEqual({ temp: 45.7 });
      await gate;
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          status: 'updated',
          data: { sensor_id: 'S-001', temp: 45.7, humidity: 60.1 },
          total: 2,
          curvature: 1.42,
          confidence: 0.41,
        }),
      });
    });

    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    // Click the temp cell on the first row
    const tempCell = page
      .getByTestId('grid-row')
      .first()
      .locator('[data-field="temp"]');
    await tempCell.click();

    // Editor opens with current value
    const editor = page.getByTestId('cell-editor-input');
    await expect(editor).toBeVisible();
    await expect(editor).toHaveValue('22.5');

    // Type the new value + commit
    await editor.fill('45.7');
    await editor.press('Enter');

    // Optimistic: editor closes, cell shows 45.7 BEFORE response settles
    await expect(page.getByTestId('cell-editor-input')).toBeHidden();
    await expect(
      page.getByTestId('grid-row').first().locator('[data-field="temp"]'),
    ).toContainText('45.7');

    // Release the engine response
    resolveUpdate();

    // Success toast + κ̄ updates from response
    await expect(page.getByTestId('toast')).toHaveAttribute('data-kind', 'success');
    await expect(page.getByTestId('toast')).toContainText('Updated S-001.temp → 45.7');
    await expect(page.locator('.stat-value').nth(1)).toHaveText('1.42');
  });

  test('rollback on engine 400: cell reverts and error toast appears', async ({ page }) => {
    await page.route(/\/v1\/bundles\/sensors\/update/, async (route) => {
      await route.fulfill({
        status: 400,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'invalid temp range' }),
      });
    });

    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    const tempCell = page
      .getByTestId('grid-row')
      .first()
      .locator('[data-field="temp"]');
    await tempCell.click();

    const editor = page.getByTestId('cell-editor-input');
    await editor.fill('999');
    await editor.press('Enter');

    // Error toast eventually appears
    const toast = page.getByTestId('toast');
    await expect(toast).toHaveAttribute('data-kind', 'error', { timeout: 5_000 });
    await expect(toast).toContainText('http_error');

    // Cell reverted to original value
    await expect(
      page.getByTestId('grid-row').first().locator('[data-field="temp"]'),
    ).toContainText('22.5');
  });

  test('Escape cancels without sending a request', async ({ page }) => {
    let updateRequests = 0;
    await page.route(/\/v1\/bundles\/sensors\/update/, async (route) => {
      updateRequests++;
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ status: 'updated', total: 2, curvature: 0, confidence: 0 }),
      });
    });

    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    const tempCell = page
      .getByTestId('grid-row')
      .first()
      .locator('[data-field="temp"]');
    await tempCell.click();

    const editor = page.getByTestId('cell-editor-input');
    await editor.fill('999');
    await editor.press('Escape');

    await expect(page.getByTestId('cell-editor-input')).toBeHidden();
    // Cell unchanged
    await expect(
      page.getByTestId('grid-row').first().locator('[data-field="temp"]'),
    ).toContainText('22.5');
    // No network call made
    expect(updateRequests).toBe(0);
  });

  test('primary key column does not show inline editor', async ({ page }) => {
    await page.goto(SHEETS_URL);
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    // sensor_id cell — not marked as editable, so click does nothing
    const idCell = page
      .getByTestId('grid-row')
      .first()
      .locator('[data-field="sensor_id"]');
    // The key cell renders as `td.key`, not `.grid-cell-editable`, so it has no
    // editable-cell testid.
    await expect(idCell).not.toHaveAttribute('data-testid', 'editable-cell');
  });
});
