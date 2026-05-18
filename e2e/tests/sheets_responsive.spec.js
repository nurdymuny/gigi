// @ts-check
import { test, expect } from '@playwright/test';

/**
 * Responsiveness — the body must never overflow horizontally,
 * even with a wide bundle. The grid owns its own horizontal scroll;
 * everything outside the grid stays clipped to the viewport.
 */

// A wide schema modeled after marcella_source_claims — 9 fields, several long
// text columns. Without proper containment the page horizontally overflows.
const wideSchema = {
  name: 'marcella_source_claims',
  base_fields: [{ name: 'claim_id', type: 'text' }],
  fiber_fields: [
    { name: 'claim_type', type: 'categorical' },
    { name: 'line_start', type: 'numeric' },
    { name: 'line_end', type: 'numeric' },
    { name: 'n_chars', type: 'numeric' },
    { name: 'section_id', type: 'categorical' },
    { name: 'doc_id', type: 'categorical' },
    { name: 'label', type: 'categorical' },
    { name: 'content', type: 'text' },
  ],
  indexed_fields: ['claim_id', 'doc_id'],
  records: 250,
  storage_mode: 'mmap',
};

const wideSection = {
  data: Array.from({ length: 40 }, (_, i) => ({
    claim_id: `claim_${String(i).padStart(4, '0')}`,
    claim_type: 'definition',
    line_start: 200 + i,
    line_end: 250 + i,
    n_chars: 80 + i,
    section_id: `section_${String(i).padStart(3, '0')}`,
    doc_id: `davis_yang_mills_mass_gap_v${(i % 5) + 1}`,
    label: i % 3 === 0 ? `eq:label_${i}` : '',
    content:
      'This is some long content text that exceeds a normal column width and forces truncation in the cell when the column is narrow enough that it has to use ellipsis.',
  })),
  total: 250,
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

test.describe('Responsiveness — wide schemas stay within the viewport', () => {
  test.beforeEach(async ({ page }) => {
    await muteWebSocket(page);
    await page.route(/\/v1\/bundles\/marcella_source_claims\/schema/, async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(wideSchema),
      });
    });
    await page.route(/\/v1\/bundles\/marcella_source_claims\/query/, async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(wideSection),
      });
    });
  });

  test('the body never has a horizontal scrollbar — the grid owns its own scroll', async ({ page }) => {
    // Pick a typical laptop viewport, smaller than the grid's natural width.
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto(SHEETS_BASE + 'marcella_source_claims');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    const bodyScrolls = await page.evaluate(() => ({
      scrollWidth: document.documentElement.scrollWidth,
      clientWidth: document.documentElement.clientWidth,
    }));
    // scrollWidth === clientWidth ⇔ no body-level horizontal overflow.
    expect(bodyScrolls.scrollWidth).toBe(bodyScrolls.clientWidth);
  });

  test('the grid has its own internal horizontal scrollbar (the columns overflow it)', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto(SHEETS_BASE + 'marcella_source_claims');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    // The grid-scroll container's scrollWidth should exceed its clientWidth
    // when there are too many columns to fit.
    const gridScrolls = await page.evaluate(() => {
      const grid = document.querySelector('.grid-scroll');
      if (!grid) return null;
      return { scrollWidth: grid.scrollWidth, clientWidth: grid.clientWidth };
    });
    expect(gridScrolls).not.toBeNull();
    expect(gridScrolls.scrollWidth).toBeGreaterThan(gridScrolls.clientWidth);
  });

  test('the inspector stays inside the viewport (never clipped off the right edge)', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto(SHEETS_BASE + 'marcella_source_claims');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    const ok = await page.evaluate(() => {
      const insp = document.querySelector('.inspector');
      if (!insp) return false;
      const r = insp.getBoundingClientRect();
      return r.right <= window.innerWidth + 1; // 1px tolerance for borders
    });
    expect(ok).toBe(true);
  });

  test('dragging a column resize handle changes that column width', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto(SHEETS_BASE + 'marcella_source_claims');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    const handle = page.getByTestId('resize-content');
    const measure = () =>
      page.evaluate(() => {
        const grid = document.querySelector('.grid-header');
        if (!grid) return 0;
        const cells = grid.querySelectorAll('[data-testid^="header-"]');
        const last = cells[cells.length - 1];
        return last.getBoundingClientRect().width;
      });
    const before = await measure();

    // Drag right ~120px. Hover the handle first so the mouse cursor is on it
    // before mouse.down (otherwise the down event fires at the wrong target).
    await handle.scrollIntoViewIfNeeded();
    await handle.hover();
    const box = await handle.boundingBox();
    if (!box) throw new Error('no handle box');
    const startX = box.x + box.width / 2;
    const startY = box.y + box.height / 2;
    await page.mouse.down();
    await page.mouse.move(startX + 120, startY, { steps: 6 });
    await page.mouse.up();

    const after = await measure();
    expect(after).toBeGreaterThan(before + 80);
  });

  test('Hide fields shrinks the grid to fit when the bundle has too many columns', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto(SHEETS_BASE + 'marcella_source_claims');
    await expect(page.getByTestId('grid')).toBeVisible({ timeout: 10_000 });

    await page.getByTestId('menu-view').click();
    await page.getByTestId('menu-item-view:hide-fields').click();
    // Hide everything but the key.
    await page.getByTestId('hide-fields-hide-non-key').click();
    await page.getByTestId('hide-fields-apply').click();

    // After hiding most fields, the grid no longer needs to scroll.
    const gridScrolls = await page.evaluate(() => {
      const grid = document.querySelector('.grid-scroll');
      if (!grid) return null;
      return { scrollWidth: grid.scrollWidth, clientWidth: grid.clientWidth };
    });
    expect(gridScrolls).not.toBeNull();
    expect(gridScrolls.scrollWidth).toBeLessThanOrEqual(gridScrolls.clientWidth + 1);
  });
});
