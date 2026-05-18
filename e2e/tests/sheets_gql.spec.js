// @ts-check
import { test, expect } from '@playwright/test';

/**
 * S7 e2e — GQL editor tab.
 *
 *   - third tab "GQL" appears alongside Grid and Geometry
 *   - typing in the textarea is preserved across tab swaps
 *   - clicking Run POSTs to /v1/gql with the right body
 *   - row-returning queries render as a table
 *   - parse errors from the engine render inline with status 400
 *   - Format collapses + re-breaks a sloppy query
 *   - ⌘↵ inside the editor runs the query
 */

const fixtureSchema = {
  name: 'sensors',
  base_fields: [{ name: 'sensor_id', type: 'text' }],
  fiber_fields: [
    { name: 'site_id', type: 'categorical' },
    { name: 'temp', type: 'numeric' },
    { name: 'humidity', type: 'numeric' },
  ],
  indexed_fields: ['sensor_id'],
  records: 2,
  storage_mode: 'mmap',
};

const fixtureSection = {
  data: [
    { sensor_id: 'S-001', site_id: 'North', temp: 22.5, humidity: 60.1 },
    { sensor_id: 'S-002', site_id: 'North', temp: 19.3, humidity: 71.4 },
  ],
  total: 2,
  curvature: 0.1,
  confidence: 0.9,
};

const SHEETS_BASE =
  process.env.SHEETS_URL || 'http://localhost:5177/gigi/sheets/';

/** Replace window.WebSocket with a no-op so the page boot doesn't dial the engine. */
async function muteWebSocket(page) {
  await page.addInitScript(() => {
    const w = /** @type {any} */ (window);
    class NullWS {
      constructor() {
        // Quiet, never opens, never errors.
        /** @type {any} */ (this).readyState = 0;
      }
      addEventListener() {}
      removeEventListener() {}
      send() {}
      close() {}
    }
    w.WebSocket = /** @type {any} */ (NullWS);
  });
}

test.describe('GIGI Sheets — GQL editor', () => {
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

  test('GQL tab is visible alongside Grid and Geometry', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId('tab-grid')).toBeVisible();
    await expect(page.getByTestId('tab-geometry')).toBeVisible();
    await expect(page.getByTestId('tab-gql')).toBeVisible();

    await page.getByTestId('tab-gql').click();
    await expect(page.getByTestId('gql-view')).toBeVisible();
    await expect(page.getByTestId('gql-editor')).toBeVisible();
  });

  test('the editor seeds with a valid CURVATURE query for the current bundle', async ({ page }) => {
    // The old default `SECTION <bundle> LIMIT 25;` is NOT valid GQL —
    // the parser requires `AT (key=val)` after SECTION. CURVATURE is the
    // simplest bundle-wide query that always parses.
    await page.goto(SHEETS_BASE + 'sensors');
    await page.getByTestId('tab-gql').click();
    const editor = page.getByTestId('gql-editor');
    await expect(editor).toHaveValue('CURVATURE sensors;');
  });

  test('clicking Run POSTs to /v1/gql and renders the result rows', async ({ page }) => {
    let gqlBody = null;
    await page.route(/\/v1\/gql/, async (route) => {
      gqlBody = JSON.parse(route.request().postData() || '{}');
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          rows: [
            { sensor_id: 'S-001', temp: 22.5 },
            { sensor_id: 'S-002', temp: 19.3 },
          ],
          count: 2,
          curvature: 0.42,
          confidence: 0.71,
        }),
      });
    });

    await page.goto(SHEETS_BASE + 'sensors');
    await page.getByTestId('tab-gql').click();
    await page.getByTestId('gql-run').click();

    await expect(page.getByTestId('gql-table')).toBeVisible({ timeout: 5_000 });
    await expect(page.getByTestId('gql-tr')).toHaveCount(2);
    await expect(page.getByTestId('meta-status')).toContainText('200');
    await expect(page.getByTestId('meta-rows')).toContainText('2');
    await expect(page.getByTestId('meta-kappa')).toContainText('0.420');

    expect(gqlBody?.query).toBe('CURVATURE sensors;');
  });

  test('typed query persists when switching tabs', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await page.getByTestId('tab-gql').click();
    const editor = page.getByTestId('gql-editor');
    await editor.fill('BETTI sensors;');
    await page.getByTestId('tab-grid').click();
    await page.getByTestId('tab-gql').click();
    await expect(page.getByTestId('gql-editor')).toHaveValue('BETTI sensors;');
  });

  test('engine parse error renders inline with status 400', async ({ page }) => {
    await page.route(/\/v1\/gql/, async (route) => {
      await route.fulfill({
        status: 400,
        contentType: 'application/json',
        body: JSON.stringify({ error: "Parse error: unexpected token at 'BAD'" }),
      });
    });
    await page.goto(SHEETS_BASE + 'sensors');
    await page.getByTestId('tab-gql').click();
    await page.getByTestId('gql-editor').fill('BAD GQL;');
    await page.getByTestId('gql-run').click();
    await expect(page.getByTestId('gql-result-engine-msg')).toBeVisible({
      timeout: 5_000,
    });
    await expect(page.getByTestId('meta-status')).toContainText('400');
    await expect(page.getByRole('alert')).toContainText(/Parse error/);
  });

  test('Format reformats sloppy input into one clause per line', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await page.getByTestId('tab-gql').click();
    await page.getByTestId('gql-editor').fill(
      "SECTION sensors WHERE site_id='N' ORDER BY κ DESC LIMIT 5;",
    );
    await page.getByTestId('gql-format').click();
    await expect(page.getByTestId('gql-editor')).toHaveValue(
      "SECTION sensors\nWHERE site_id='N'\nORDER BY κ DESC\nLIMIT 5;",
    );
  });

  test('⌘↵ inside the editor runs the query', async ({ page }) => {
    let posted = false;
    await page.route(/\/v1\/gql/, async (route) => {
      posted = true;
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ rows: [{ a: 1 }], count: 1 }),
      });
    });
    await page.goto(SHEETS_BASE + 'sensors');
    await page.getByTestId('tab-gql').click();
    const editor = page.getByTestId('gql-editor');
    await editor.click();
    await editor.press('Meta+Enter');
    await expect(page.getByTestId('gql-table')).toBeVisible({ timeout: 5_000 });
    expect(posted).toBe(true);
  });
});
