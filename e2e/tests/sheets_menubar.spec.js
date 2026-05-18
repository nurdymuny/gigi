// @ts-check
import { test, expect } from '@playwright/test';

/**
 * MenuBar (File/Edit/View/Insert/Format/Data/Geometry/Tools/Help) +
 * column hide affordance + sticky-left key column behaviour.
 */

const fixtureSchema = {
  name: 'sensors',
  base_fields: [{ name: 'sensor_id', type: 'text' }],
  fiber_fields: [
    { name: 'site_id', type: 'categorical' },
    { name: 'drug_name', type: 'categorical' },
    { name: 'year', type: 'numeric' },
    { name: 'value', type: 'numeric' },
    { name: 'units', type: 'categorical' },
    { name: 'citation', type: 'categorical' },
    { name: 'standard', type: 'categorical' },
    { name: 'doi', type: 'categorical' },
  ],
  indexed_fields: ['sensor_id'],
  records: 4,
  storage_mode: 'mmap',
};

const fixtureSection = {
  data: [
    {
      sensor_id: 'S-001',
      site_id: 'N',
      drug_name: 'CRO',
      year: 2024,
      value: 1000,
      units: 'ug_hr_mL',
      citation: 'Patel IH',
      standard: 'FDA',
      doi: '',
    },
  ],
  total: 1,
  curvature: 0.1,
  confidence: 0.9,
};

const SHEETS_BASE =
  process.env.SHEETS_URL?.replace(/\/sensors$/, '/') ||
  'http://localhost:5177/gigi/sheets/';

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

test.describe('MenuBar + column hiding + sticky columns', () => {
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

  test('all 9 top-level menus appear between the topbar and the tabs', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    for (const name of [
      'file',
      'edit',
      'view',
      'insert',
      'format',
      'data',
      'geometry',
      'tools',
      'help',
    ]) {
      await expect(page.getByTestId(`menu-${name}`)).toBeVisible();
    }
  });

  test('Tools › Schema editor opens the schema modal', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('menu-tools').click();
    await page.getByTestId('menu-item-tools:schema').click();
    await expect(page.getByTestId('schema-modal')).toBeVisible();
  });

  test('View › Geometry overlay shows a ✓ when it is on; toggling flips it', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('menu-view').click();
    // App boots with overlayOn=true.
    await expect(page.getByTestId('menu-item-view:overlay')).toContainText('✓');
    await page.getByTestId('menu-item-view:overlay').click();
    // Re-open the menu and confirm the check is gone.
    await page.getByTestId('menu-view').click();
    await expect(page.getByTestId('menu-item-view:overlay')).not.toContainText('✓');
  });

  test('View › Hide fields… opens the modal and applying hides those fields from the grid', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    // Before: doi and citation are visible.
    await expect(page.getByTestId('header-doi')).toBeVisible();
    await expect(page.getByTestId('header-citation')).toBeVisible();

    await page.getByTestId('menu-view').click();
    await page.getByTestId('menu-item-view:hide-fields').click();
    await expect(page.getByTestId('hide-fields-modal')).toBeVisible();

    // Hide both doi and citation.
    await page.getByTestId('hide-fields-check-doi').click();
    await page.getByTestId('hide-fields-check-citation').click();
    await page.getByTestId('hide-fields-apply').click();

    // Grid drops them.
    await expect(page.getByTestId('header-doi')).toHaveCount(0);
    await expect(page.getByTestId('header-citation')).toHaveCount(0);
    // Other columns still there.
    await expect(page.getByTestId('header-value')).toBeVisible();
  });

  test('the primary key column is marked sticky-left in the header', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    const keyHeader = page.getByTestId('header-sensor_id');
    // Confirms the sticky-left class is on the header cell.
    await expect(keyHeader).toHaveClass(/grid-cell-sticky-key/);
  });

  test('Help › Keyboard shortcuts surfaces a toast with the keymap', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('menu-help').click();
    await page.getByTestId('menu-item-help:shortcuts').click();
    await expect(page.getByTestId('toast')).toBeVisible();
    await expect(page.getByTestId('toast')).toContainText(/⌘/);
  });
});
