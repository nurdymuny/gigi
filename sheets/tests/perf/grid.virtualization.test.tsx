import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { Grid } from "../../src/components/Grid";
import type { BundleSchema, RowMap } from "../../src/lib/gigi-client";

/**
 * S0 perf gate (per GIGI_SHEETS_SPRINT_SPEC.md §6 + addendum §Q6).
 *
 * The Grid uses @tanstack/react-virtual for windowing. Without
 * virtualization, a 5,000-row bundle would mount 5,000 row divs and
 * destroy scroll/edit perf. The gate: render 5,000 rows, count the
 * actual grid-row DOM nodes, assert we stay under a tight ceiling.
 *
 * jsdom layout is stubbed in tests/setup.ts to a 1024×768 viewport.
 * With ROW_HEIGHT=34px that's ~22 visible rows + 8 overscan on either
 * side, so the natural ceiling is ~38. We assert ≤ 80 to leave room
 * for incremental changes to overscan / row height without flaking,
 * while still catching regressions to a non-virtualized renderer.
 */

const PERF_SCHEMA: BundleSchema = {
  name: "sensors_perf",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "temp", type: "numeric" },
    { name: "humidity", type: "numeric" },
    { name: "site_id", type: "categorical" },
    { name: "status", type: "categorical" },
  ],
  indexed_fields: ["sensor_id", "site_id"],
  records: 5000,
  storage_mode: "mmap",
};

function makeRows(n: number): RowMap[] {
  const sites = ["North-3", "East-1", "South-2", "West-4"];
  const statuses = ["ok", "ok", "ok", "warning", "offline"];
  const rows: RowMap[] = new Array(n);
  for (let i = 0; i < n; i++) {
    rows[i] = {
      sensor_id: `S-${String(i).padStart(5, "0")}`,
      temp: 18 + ((i * 7) % 24),
      humidity: 30 + ((i * 13) % 50),
      site_id: sites[i % sites.length],
      status: statuses[i % statuses.length],
    };
  }
  return rows;
}

describe("Grid — virtualization perf gate", () => {
  it("mounts ≤ 80 DOM rows when rendering 5,000 rows", () => {
    const rows = makeRows(5000);
    render(<Grid schema={PERF_SCHEMA} rows={rows} loading={false} />);

    const mounted = screen.getAllByTestId("grid-row");

    // Sanity: at least one row mounted, proving virtualization didn't
    // collapse to nothing (the failure mode we previously fixed with
    // jsdom layout stubs).
    expect(mounted.length).toBeGreaterThan(0);

    // The real assertion: don't mount the whole bundle.
    expect(mounted.length).toBeLessThanOrEqual(80);
  });

  it("renders large datasets without throwing", () => {
    const rows = makeRows(5000);
    expect(() => {
      render(<Grid schema={PERF_SCHEMA} rows={rows} loading={false} />);
    }).not.toThrow();
  });

  it("does not collapse to zero rows for medium datasets (200 rows)", () => {
    // Guard: a too-aggressive virtualizer that windows away everything
    // would silently break the grid. Catch that here.
    const rows = makeRows(200);
    render(<Grid schema={PERF_SCHEMA} rows={rows} loading={false} />);
    expect(screen.getAllByTestId("grid-row").length).toBeGreaterThan(0);
  });
});
