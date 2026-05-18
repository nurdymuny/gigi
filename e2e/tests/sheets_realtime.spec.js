// @ts-check
import { test, expect } from '@playwright/test';

/**
 * S2.5 e2e — realtime subscription.
 *
 * We mock both:
 *   - the HTTP API via page.route (schema + section)
 *   - window.WebSocket via addInitScript, so the app's subscription is
 *     intercepted at the JS level. The mock exposes window.__mockWS.emit()
 *     for the test to push frames in.
 *
 * The test asserts:
 *   - sheet sends SUBSCRIBE <bundle> on open
 *   - realtime pill flips to "live"
 *   - an injected EVENT frame updates a row without user gesture
 *   - NOTICE lagged=N surfaces the lag pill
 *   - URL path /gigi/sheets/<bundle> determines which bundle loads
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

/**
 * Replace window.WebSocket with a stub that:
 *   - records every URL it's constructed for, and every send() payload
 *   - exposes window.__mockWS.emit(line) / .close() / .sent / .url
 *   - fires onopen synchronously on next microtask
 */
async function installMockWebSocket(page) {
  await page.addInitScript(() => {
    const w = /** @type {any} */ (window);
    w.__mockWS = {
      url: '',
      sent: [],
      instance: null,
    };
    class MockWebSocket {
      constructor(url) {
        const self = /** @type {any} */ (this);
        self.url = url;
        self.readyState = 0;
        self._listeners = { open: [], message: [], close: [], error: [] };
        w.__mockWS.url = url;
        w.__mockWS.instance = self;
        // Open on a microtask so addEventListener can register first.
        Promise.resolve().then(() => {
          self.readyState = 1;
          for (const fn of self._listeners.open) fn({});
        });
        // Helpers exposed via the global mock.
        w.__mockWS.emit = (line) => {
          for (const fn of self._listeners.message) fn({ data: line });
        };
        w.__mockWS.closeFromServer = () => {
          self.readyState = 3;
          for (const fn of self._listeners.close) fn({});
        };
      }
      addEventListener(type, fn) {
        const self = /** @type {any} */ (this);
        (self._listeners[type] ?? []).push(fn);
      }
      removeEventListener(type, fn) {
        const self = /** @type {any} */ (this);
        const arr = self._listeners[type] ?? [];
        const i = arr.indexOf(fn);
        if (i >= 0) arr.splice(i, 1);
      }
      send(data) {
        w.__mockWS.sent.push(String(data));
      }
      close() {
        const self = /** @type {any} */ (this);
        self.readyState = 3;
        for (const fn of self._listeners.close) fn({});
      }
    }
    w.WebSocket = /** @type {any} */ (MockWebSocket);
  });
}

test.describe('GIGI Sheets — realtime', () => {
  test.beforeEach(async ({ page }) => {
    await installMockWebSocket(page);
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

  test('URL routing — /gigi/sheets/sensors loads the "sensors" bundle', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await expect(page.locator('.crumbs')).toContainText('sensors');
  });

  test('opens a WebSocket subscription and shows "live" with SUBSCRIBE sent', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId('realtime-pill')).toHaveAttribute(
      'data-status',
      'open',
      { timeout: 5_000 },
    );
    // Mock recorded SUBSCRIBE sensors.
    const sent = await page.evaluate(() => /** @type {any} */ (window).__mockWS.sent);
    expect(sent).toContain('SUBSCRIBE sensors');
    // URL the app dialed.
    const wsUrl = await page.evaluate(() => /** @type {any} */ (window).__mockWS.url);
    expect(wsUrl).toBe('ws://localhost:3142/ws');
  });

  test('an injected EVENT frame updates a row without any user gesture', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId('realtime-pill')).toHaveAttribute(
      'data-status',
      'open',
      { timeout: 5_000 },
    );

    const tempCell = page.locator(
      '[data-testid="grid-row"][data-row-key="S-001"] [data-field="temp"]',
    );
    await expect(tempCell).toContainText('22.5');

    // Inject the event from outside the React tree.
    await page.evaluate(() => {
      /** @type {any} */ (window).__mockWS.emit(
        'EVENT sensors update {"sensor_id":"S-001","temp":99.9} K=2.4 C=0.29',
      );
    });

    await expect(tempCell).toContainText('99.9', { timeout: 5_000 });
    // κ̄ in the topbar follows the event's K=.
    await expect(page.locator('.stat-value').nth(1)).toHaveText('2.40');
  });

  test('NOTICE lagged=N surfaces and accumulates the lag pill', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId('realtime-pill')).toHaveAttribute(
      'data-status',
      'open',
      { timeout: 5_000 },
    );

    await page.evaluate(() => {
      /** @type {any} */ (window).__mockWS.emit('NOTICE sensors lagged=7');
    });
    await expect(page.getByTestId('realtime-lag')).toContainText('+7 behind', {
      timeout: 5_000,
    });

    await page.evaluate(() => {
      /** @type {any} */ (window).__mockWS.emit('NOTICE sensors lagged=3');
    });
    await expect(page.getByTestId('realtime-lag')).toContainText('+10 behind');
  });

  test('an INSERT event appends a row that did not exist', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId('realtime-pill')).toHaveAttribute(
      'data-status',
      'open',
      { timeout: 5_000 },
    );

    await expect(page.getByTestId('grid-row')).toHaveCount(2);

    await page.evaluate(() => {
      /** @type {any} */ (window).__mockWS.emit(
        'EVENT sensors insert {"sensor_id":"S-003","site_id":"North","temp":24.0,"humidity":61.0} K=0.1 C=0.9',
      );
    });

    await expect(page.getByTestId('grid-row')).toHaveCount(3, { timeout: 5_000 });
    await expect(
      page.locator('[data-testid="grid-row"][data-row-key="S-003"]'),
    ).toBeVisible();
  });

  test('a server-initiated close flips the pill to "closed"', async ({ page }) => {
    await page.goto(SHEETS_BASE + 'sensors');
    await expect(page.getByTestId('realtime-pill')).toHaveAttribute(
      'data-status',
      'open',
      { timeout: 5_000 },
    );
    await page.evaluate(() => {
      /** @type {any} */ (window).__mockWS.closeFromServer();
    });
    await expect(page.getByTestId('realtime-pill')).toHaveAttribute(
      'data-status',
      'closed',
    );
  });
});
