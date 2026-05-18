// @ts-check
import { test, expect } from '@playwright/test';

/**
 * E2E for the remaining sprints:
 *   S5 — Schema modal (add/drop fields)
 *   S6 — Saved views (URL share + localStorage)
 *   S8 — Encrypted fields surface
 *   S9 — Insights drawer
 */

const fixtureSchema = {
  name: 'sensors',
  base_fields: [{ name: 'sensor_id', type: 'text' }],
  fiber_fields: [
    { name: 'site_id', type: 'categorical' },
    { name: 'temp', type: 'numeric' },
    { name: 'humidity', type: 'numeric' },
    { name: 'operator', type: 'text', encryption: 'indexed' },
    { name: 'secret', type: 'text', encryption: 'opaque' },
  ],
  indexed_fields: ['sensor_id', 'site_id'],
  records: 4,
  storage_mode: 'mmap',
};

const fixtureSection = {
  data: [
    { sensor_id: 'S-001', site_id: 'N', temp: 22.5, humidity: 60.1, operator: 'opr_d2a8', secret: 'classified' },
    { sensor_id: 'S-002', site_id: 'N', temp: 23.0, humidity: 61.0, operator: 'opr_d2a8', secret: 'top-secret' },
    { sensor_id: 'S-003', site_id: 'N', temp: 24.1, humidity: 59.0, operator: 'opr_d2a8', secret: 'hush' },
    { sensor_id: 'S-OUT', site_id: 'N', temp: 99.0, humidity: 5.0, operator: 'opr_d2a8', secret: 'should-never-leak' },
  ],
  total: 4,
  curvature: 1.2,
  confidence: 0.45,
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

test.describe('S5 — Schema modal', () => {
  test.beforeEach(async ({ page }) => {
    await muteWebSocket(page);
    await page.addInitScript(() => localStorage.clear());
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

  test('opens, lists every field, drop is disabled on the primary key', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('schema-open').click();
    await expect(page.getByTestId('schema-modal')).toBeVisible();

    await expect(page.getByTestId('schema-field-sensor_id')).toBeVisible();
    await expect(page.getByTestId('schema-field-temp')).toBeVisible();
    await expect(page.getByTestId('schema-drop-sensor_id')).toBeDisabled();
  });

  test('+ Add field opens the form and POSTs to /add-field on submit', async ({ page }) => {
    let added = null;
    await page.route(/\/v1\/bundles\/sensors\/add-field/, async (route) => {
      added = JSON.parse(route.request().postData() || '{}');
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ status: 'ok' }),
      });
    });

    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('schema-open').click();
    await page.getByTestId('schema-add').click();
    await page.getByTestId('schema-form-name').fill('pressure_hpa');
    await page.getByTestId('schema-form-type').selectOption('numeric');
    await page.getByTestId('schema-form-submit').click();

    await expect.poll(() => added).not.toBeNull();
    expect(added).toMatchObject({ name: 'pressure_hpa', type: 'numeric' });
  });
});

test.describe('S6 — Saved views', () => {
  test.beforeEach(async ({ page }) => {
    await muteWebSocket(page);
    await page.addInitScript(() => localStorage.clear());
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

  test('saving a view persists it to the drawer list', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('views-open').click();
    await page.getByTestId('views-drawer-name').fill('North-3 anomalies');
    await page.getByTestId('views-drawer-save').click();
    await expect(page.getByTestId('views-drawer-list')).toBeVisible();
    await expect(page.getByText('North-3 anomalies')).toBeVisible();
  });

  test('URL ?view= hydrates the activeView on load', async ({ page }) => {
    // Pre-built ?view= encoding for { v:1, activeView: "geometry" } …
    // We just build it on the fly by saving a view, copying the URL, then visiting it fresh.
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    // Switch to geometry tab so the saved spec captures it.
    await page.getByTestId('tab-geometry').click();
    await page.getByTestId('views-open').click();

    // Stub the clipboard so we can read what 'Copy share link' wrote.
    /** @type {string|null} */
    let copied = null;
    await page.exposeFunction('__captureCopy', (s) => { copied = s; });
    await page.addInitScript(() => {
      const w = /** @type {any} */ (window);
      // Re-stub clipboard after the page boots.
      Object.defineProperty(navigator, 'clipboard', {
        configurable: true,
        value: { writeText: async (s) => w.__captureCopy(s) },
      });
    });
    // After addInitScript, reload so the override applies.
    await page.reload();
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('tab-geometry').click();
    await page.getByTestId('views-open').click();
    await page.getByTestId('views-drawer-copy-link').click();
    await expect.poll(() => copied).toContain('view=');

    // Visit the share URL in a fresh tab and confirm Geometry is selected.
    await page.goto(/** @type {string} */ (copied));
    await expect(page.getByTestId('tab-geometry')).toHaveAttribute('aria-selected', 'true');
  });
});

test.describe('S8 — Encrypted fields', () => {
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

  test('OPAQUE fields never leak plaintext into the DOM textContent', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    // The 'secret' field is OPAQUE in the fixture. None of the fixture values
    // (classified, top-secret, hush, should-never-leak) should appear anywhere
    // in the page text.
    const body = await page.locator('body').textContent();
    for (const leak of ['classified', 'top-secret', 'hush', 'should-never-leak']) {
      expect(body).not.toContain(leak);
    }

    // The OPAQUE cells render with data-encryption="opaque" and block chars.
    const opaqueCells = page.locator('[data-encryption="opaque"]');
    await expect(opaqueCells.first()).toBeVisible();
    await expect(opaqueCells.first()).toContainText('▒');
  });

  test('INDEXED fields render the value with a lock affordance', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    const idxCell = page.locator('[data-encryption="indexed"]').first();
    await expect(idxCell).toBeVisible();
    await expect(idxCell.locator('svg')).toBeVisible();
    await expect(idxCell).toContainText('opr_d2a8');
  });
});

test.describe('S9 — Insights drawer', () => {
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

  test('opens with at least one anomaly insight surfaced', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('insights-open').click();
    await expect(page.getByTestId('insights-drawer')).toBeVisible();
    // S-OUT has κ ≈ 4.x given the fixture; the cohort + top-κ rules should both fire.
    await expect(page.getByTestId('insight-cohort-top-anomalies')).toBeVisible();
    await expect(page.getByTestId('insight-top-kappa')).toBeVisible();
  });

  test('Copy on an insight writes its GQL to the clipboard', async ({ page }) => {
    // Stub navigator.clipboard so headless browsers don't reject writeText.
    await page.addInitScript(() => {
      const w = /** @type {any} */ (window);
      w.__clipboard = '';
      Object.defineProperty(navigator, 'clipboard', {
        configurable: true,
        value: { writeText: async (s) => { w.__clipboard = s; } },
      });
    });
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('insights-open').click();
    await page.getByTestId('insight-copy-cohort-top-anomalies').click();
    await expect(page.getByTestId('toast')).toBeVisible({ timeout: 5_000 });
    await expect(page.getByTestId('toast')).toContainText(/copied/i);
    const clip = await page.evaluate(() => /** @type {any} */ (window).__clipboard);
    expect(clip).toContain('SECTION sensors WHERE');
  });
});
