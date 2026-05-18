// @ts-check
import { test, expect } from '@playwright/test';

/**
 * Help › About GIGI Sheets opens a real modal — not a toast.
 */

const fixtureSchema = {
  name: 'sensors',
  base_fields: [{ name: 'sensor_id', type: 'text' }],
  fiber_fields: [{ name: 'temp', type: 'numeric' }],
  indexed_fields: ['sensor_id'],
  records: 1,
  storage_mode: 'mmap',
};

const fixtureSection = {
  data: [{ sensor_id: 'S-001', temp: 22.5 }],
  total: 1,
  curvature: 0.0,
  confidence: 1.0,
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

test.describe('Help › About modal', () => {
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

  test('opens the About modal from Help › About GIGI Sheets', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('menu-help').click();
    await page.getByTestId('menu-item-help:about').click();
    const modal = page.getByTestId('about-modal');
    await expect(modal).toBeVisible();
    await expect(modal).toContainText('GIGI');
    await expect(modal).toContainText('Bee Rosa Davis');
  });

  test('switches between engine and person tabs', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('menu-help').click();
    await page.getByTestId('menu-item-help:about').click();

    // Defaults to engine.
    await expect(page.getByTestId('about-engine')).toBeVisible();
    await expect(page.queryByTestId?.('about-person') ?? page.getByTestId('about-person')).toBeHidden({ timeout: 200 }).catch(() => {});

    await page.getByTestId('about-tab-person').click();
    await expect(page.getByTestId('about-person')).toBeVisible();
    await expect(page.getByTestId('about-person')).toContainText('KRAKEN');
    await expect(page.getByTestId('about-person')).toContainText('Marcella');
  });

  test('closes on Escape', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('menu-help').click();
    await page.getByTestId('menu-item-help:about').click();
    await expect(page.getByTestId('about-modal')).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.getByTestId('about-modal')).toBeHidden();
  });
});
