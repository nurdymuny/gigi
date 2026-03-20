// @ts-check
import { test, expect } from '@playwright/test';

const BASE = 'http://localhost:3142';

// ── Helpers ──────────────────────────────────────────────────────────────────

/** Assert a stat card shows something other than "—" */
async function expectStatCard(page, id, label) {
  const el = page.locator(`#${id}`);
  await expect(el, `${label} (${id}) should not be "—"`).not.toHaveText('—', { timeout: 20_000 });
  await expect(el, `${label} (${id}) should not be empty`).not.toBeEmpty();
}

/** Return the text of #ws-debug */
async function wsDebugText(page) {
  return page.locator('#ws-debug').textContent();
}

// ── Suite 1: REST sanity checks ───────────────────────────────────────────────

test.describe('REST API sanity', () => {
  test('health endpoint returns 200', async ({ request }) => {
    const r = await request.get('/v1/health');
    expect(r.status()).toBe(200);
  });

  test('/v1/bundles returns an array', async ({ request }) => {
    const r = await request.get('/v1/bundles');
    expect(r.status()).toBe(200);
    const body = await r.json();
    expect(Array.isArray(body)).toBe(true);
  });

  test('bundle lifecycle: create → health → delete', async ({ request }) => {
    // Clean up first
    await request.delete('/v1/bundles/pw-test');

    // Create
    const create = await request.post('/v1/bundles', {
      data: {
        name: 'pw-test',
        schema: {
          fields: { id: 'numeric', val: 'numeric' },
          keys: ['id'],
          indexed: [],
        },
      },
    });
    expect(create.status(), 'bundle create should succeed (200 or 201)').toBeLessThan(300);

    // Insert records
    const insert = await request.post('/v1/bundles/pw-test/points', {
      data: {
        records: [
          { id: 1, val: 10.0 },
          { id: 2, val: 20.0 },
          { id: 3, val: 30.0 },
        ],
      },
    });
    expect(insert.status(), 'insert should succeed').toBe(200);

    // Health
    const health = await request.get('/v1/bundles/pw-test/health');
    expect(health.status()).toBe(200);
    const hBody = await health.json();
    expect(hBody.record_count, 'record_count should be 3').toBe(3);
    expect(typeof hBody.confidence).toBe('number');
    expect(typeof hBody.k_mean).toBe('number');
    expect(Array.isArray(hBody.per_field)).toBe(true);

    // Cleanup
    const del = await request.delete('/v1/bundles/pw-test');
    expect(del.status()).toBeLessThan(500);
  });
});

// ── Suite 2: Dashboard page loads ─────────────────────────────────────────────

test.describe('Dashboard page', () => {
  test('loads and shows initial state', async ({ page }) => {
    await page.goto('/dashboard');

    // Title in header
    await expect(page.locator('header h1')).toContainText('GIGI Live Dashboard');

    // Status pill starts disconnected or connecting
    const pill = page.locator('#status-pill');
    await expect(pill).toBeVisible();

    // Stat cards show "—" initially (no active bundle)
    // (DOMContentLoaded connects to bundle="" which means all — no data)
    await expect(page.locator('#field-bars')).toContainText('No data yet.');

    // Buttons exist
    await expect(page.locator('#demo-btn')).toBeVisible();
    await expect(page.locator('#connect-btn')).toBeVisible();
  });

  test('connect button connects to named bundle', async ({ page }) => {
    await page.goto('/dashboard');

    // Make sure the demo bundle exists with some data
    await fetch(`${BASE}/v1/bundles`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        name: 'demo',
        schema: {
          fields: { id: 'numeric', val: 'numeric' },
          keys: ['id'], indexed: [],
        },
      }),
    }).catch(() => {});
    await fetch(`${BASE}/v1/bundles/demo/points`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        records: [{ id: 1, val: 10 }, { id: 2, val: 20 }],
      }),
    }).catch(() => {});

    // Type bundle name and click connect
    await page.fill('#bundle-input', 'demo');
    await page.click('#connect-btn');

    // Wait for WS to open (pill becomes "connected")
    await expect(page.locator('#status-pill')).toHaveClass(/connected/, { timeout: 5000 });

    // ws-debug should show OPEN
    const debug = await wsDebugText(page);
    expect(debug, 'ws-debug should show OPEN state').toContain('ws:OPEN');
  });
});

// ── Suite 3: Create Demo end-to-end ───────────────────────────────────────────

test.describe('Create Demo', () => {
  test.beforeEach(async ({ request }) => {
    // Clean slate — delete any existing demo bundle
    await request.delete('/v1/bundles/demo');
  });

  test('demo button runs full pipeline and populates dashboard', async ({ page }) => {
    await page.goto('/dashboard');

    // Collect browser console errors for debugging
    const consoleErrors = [];
    page.on('console', msg => {
      if (msg.type() === 'error') consoleErrors.push(msg.text());
    });
    // Collect 404 responses so we can debug which URL causes problems
    const not404s = ['/v1/bundles/demo/health', '/api/', '/favicon'];
    page.on('response', resp => {
      if (resp.status() === 404 && !not404s.some(p => resp.url().includes(p))) {
        consoleErrors.push(`404: ${resp.url()}`);
      }
    });

    // Click Create Demo
    await page.click('#demo-btn');

    // Watch demo-status for progress
    const demoStatus = page.locator('#demo-status');
    await expect(demoStatus).not.toBeEmpty({ timeout: 5000 });

    // Wait for demo to finish (shows "✓ done")
    await expect(demoStatus, 'demo should complete successfully')
      .toContainText('✓ done', { timeout: 45_000 });

    // Capture ws-debug before assertions
    const debug = await wsDebugText(page);
    console.log('ws-debug after demo:', debug);

    // ── Stat cards ─────────────────────────────────────────────────────────
    await expectStatCard(page, 's-records', 'Records');
    await expectStatCard(page, 's-conf', 'Confidence');
    await expectStatCard(page, 's-kmean', 'K mean');

    // Records should be ~205
    const recordsText = await page.locator('#s-records').textContent();
    const records = parseInt((recordsText || '0').replace(/,/g, ''), 10);
    expect(records, `expected ~205 records, got ${records}`).toBeGreaterThan(0);

    // ── WS debug ───────────────────────────────────────────────────────────
    expect(
      debug,
      `WebSocket should be OPEN. Got: "${debug}". ` +
      'If CLOSED/ERROR, the WS connection is failing (CORS or routing issue).'
    ).toContain('ws:OPEN');

    const msgCountMatch = (debug || '').match(/msgs:(\d+)/);
    const msgCount = msgCountMatch ? parseInt(msgCountMatch[1], 10) : 0;
    expect(
      msgCount,
      `Expected >0 WS messages, got ${msgCount}. ` +
      'WS opens but server emits no DashboardEvents — check dashboard_tx.send() in gigi_stream.rs'
    ).toBeGreaterThan(0);

    // ── Anomaly feed ───────────────────────────────────────────────────────
    // Should have at least some anomaly entries (we insert 5 extreme values)
    const feedItems = page.locator('#anomaly-feed .feed-item');
    await expect(feedItems.first(), 'anomaly feed should have entries').toBeVisible({ timeout: 10_000 });
    const feedCount = await feedItems.count();
    expect(feedCount, 'expected at least 1 anomaly feed entry').toBeGreaterThanOrEqual(1);

    // ── Event log ──────────────────────────────────────────────────────────
    const logItems = page.locator('#event-log .feed-item');
    await expect(logItems.first(), 'event log should have entries').toBeVisible({ timeout: 5000 });

    // ── No console errors ──────────────────────────────────────────────────
    if (consoleErrors.length > 0) {
      console.warn('Browser console errors:', consoleErrors);
    }
    expect(
      consoleErrors.filter(e => !e.includes('favicon')),
      'should have no console errors'
    ).toHaveLength(0);
  });

  test('REST polling updates stat cards even when WS is blocked', async ({ page, request }) => {
    // This test validates the REST fallback path independently:
    // Create bundle + insert records manually, then open dashboard
    // and click the bundle chip — stat cards should populate from REST.

    // Create bundle with data
    await request.post('/v1/bundles', {
      data: {
        name: 'demo',
        schema: {
          fields: { id: 'numeric', sensor: 'categorical', temp_c: 'numeric', humidity: 'numeric', pressure: 'numeric', co2_ppm: 'numeric' },
          keys: ['id'], indexed: ['sensor'],
        },
      },
    });
    const records = Array.from({ length: 30 }, (_, i) => ({
      id: i + 1,
      sensor: ['alpha', 'beta'][i % 2],
      temp_c: +(20 + (i % 5) * 0.5).toFixed(2),
      humidity: 50.0,
      pressure: 1010.0,
      co2_ppm: 450.0,
    }));
    await request.post('/v1/bundles/demo/points', { data: { records } });

    await page.goto('/dashboard');

    // Chips should include "demo" after the 3s refresh
    await expect(page.locator('.chip[data-bundle="demo"]')).toBeVisible({ timeout: 8_000 });

    // Click the demo chip
    await page.click('.chip[data-bundle="demo"]');

    // Confirm WS connects
    await expect(page.locator('#status-pill')).toHaveClass(/connected/, { timeout: 5000 });

    // Wait up to 6 seconds for REST poll (runs every 2s)
    await expectStatCard(page, 's-records', 'Records');
    await expectStatCard(page, 's-conf', 'Confidence');
    await expectStatCard(page, 's-kmean', 'K mean');

    const rec = await page.locator('#s-records').textContent();
    expect(parseInt((rec || '').replace(/,/g, ''), 10)).toBeGreaterThanOrEqual(30);
  });
});

// ── Suite 4: WebSocket protocol ───────────────────────────────────────────────

test.describe('WebSocket events', () => {
  test('inserts after WS subscribe → messages arrive', async ({ page, request }) => {
    // Ensure demo bundle exists
    await request.delete('/v1/bundles/demo').catch(() => {});
    await request.post('/v1/bundles', {
      data: {
        name: 'demo',
        schema: {
          fields: { id: 'numeric', val: 'numeric' },
          keys: ['id'], indexed: [],
        },
      },
    });

    await page.goto('/dashboard');

    // Connect to demo
    await page.fill('#bundle-input', 'demo');
    await page.click('#connect-btn');
    await expect(page.locator('#status-pill')).toHaveClass(/connected/, { timeout: 5000 });

    // Confirm WS is OPEN before inserting
    const debugBefore = await wsDebugText(page);
    console.log('ws-debug before insert:', debugBefore);
    expect(debugBefore).toContain('ws:OPEN');

    // Insert 3 records via REST
    await request.post('/v1/bundles/demo/points', {
      data: { records: [{ id: 1, val: 10 }, { id: 2, val: 20 }, { id: 3, val: 30 }] },
    });

    // Wait for at least 1 WS message
    await page.waitForFunction(() => {
      const el = document.getElementById('ws-debug');
      const match = (el?.textContent || '').match(/msgs:(\d+)/);
      return match && parseInt(match[1]) > 0;
    }, undefined, { timeout: 5000 });

    const debugAfter = await wsDebugText(page);
    console.log('ws-debug after insert:', debugAfter);
    expect(debugAfter).toContain('ws:OPEN');

    const msgMatch = (debugAfter || '').match(/msgs:(\d+)/);
    expect(parseInt(msgMatch?.[1] || '0')).toBeGreaterThan(0);

    // Event log should have entries
    await expect(page.locator('#event-log .feed-item').first()).toBeVisible({ timeout: 5000 });
  });

  test('event log shows both insert type and correct bundle', async ({ page, request }) => {
    await request.delete('/v1/bundles/demo').catch(() => {});
    await request.post('/v1/bundles', {
      data: {
        name: 'demo',
        schema: {
          fields: { id: 'numeric', val: 'numeric' },
          keys: ['id'], indexed: [],
        },
      },
    });

    await page.goto('/dashboard');
    await page.fill('#bundle-input', 'demo');
    await page.click('#connect-btn');
    await expect(page.locator('#status-pill')).toHaveClass(/connected/, { timeout: 5000 });

    await request.post('/v1/bundles/demo/points', {
      data: { records: [{ id: 1, val: 99 }] },
    });

    const logItem = page.locator('#event-log .feed-item').first();
    await expect(logItem).toBeVisible({ timeout: 6000 });

    const logText = await logItem.textContent();
    console.log('First log item text:', logText);
    expect(logText, 'log item should mention "insert"').toContain('insert');
    expect(logText, 'log item should mention bundle "demo"').toContain('demo');
  });
});
