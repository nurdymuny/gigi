import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { Grid } from "../../src/components/Grid";
import type { BundleSchema } from "../../src/lib/gigi-client";

/**
 * S2 gutter — verifies the κ column reflects the kappaMap prop:
 *   - bar width tracks κ
 *   - kappa class (ok/warn/bad) propagates to row + cell data attrs
 *   - missing κ falls back to "—" without crashing
 */

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "site_id", type: "categorical" },
    { name: "temp", type: "numeric" },
    { name: "humidity", type: "numeric" },
  ],
  indexed_fields: ["sensor_id"],
  records: 3,
  storage_mode: "mmap",
};

const ROWS = [
  { sensor_id: "S-OK", site_id: "N", temp: 22, humidity: 60 },
  { sensor_id: "S-WARN", site_id: "N", temp: 23, humidity: 60 },
  { sensor_id: "S-BAD", site_id: "N", temp: 99, humidity: 10 },
];

function kappa(map: Record<string, number>): Map<string, number> {
  return new Map(Object.entries(map));
}

describe("Grid — κ gutter", () => {
  it("renders the κ-bar with width proportional to κ for healthy rows", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        kappaMap={kappa({ "S-OK": 0.1, "S-WARN": 0.1, "S-BAD": 0.1 })}
      />,
    );
    const cells = screen.getAllByTestId("kappa-cell");
    expect(cells).toHaveLength(3);
    for (const c of cells) {
      expect(c).toHaveAttribute("data-kappa-class", "ok");
    }
  });

  it("applies kappa-warn class on the row + bar for 0.8 ≤ κ < 2.0", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        kappaMap={kappa({ "S-OK": 0.1, "S-WARN": 1.2, "S-BAD": 0.1 })}
      />,
    );
    const warnRow = screen
      .getAllByTestId("grid-row")
      .find((r) => r.getAttribute("data-row-key") === "S-WARN")!;
    expect(warnRow).toHaveAttribute("data-kappa-class", "warn");
    expect(warnRow.className).toContain("kappa-warn");

    const cell = warnRow.querySelector('[data-testid="kappa-cell"]')!;
    expect(cell).toHaveAttribute("data-kappa-class", "warn");
  });

  it("applies kappa-bad class on the row + bar for κ ≥ 2.0", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        kappaMap={kappa({ "S-OK": 0.1, "S-WARN": 0.1, "S-BAD": 4.2 })}
      />,
    );
    const badRow = screen
      .getAllByTestId("grid-row")
      .find((r) => r.getAttribute("data-row-key") === "S-BAD")!;
    expect(badRow).toHaveAttribute("data-kappa-class", "bad");
    expect(badRow.className).toContain("kappa-bad");
    const cell = badRow.querySelector('[data-testid="kappa-cell"]')!;
    expect(cell).toHaveAttribute("data-kappa-class", "bad");
    expect(cell).toHaveAttribute("data-kappa", "4.200");
  });

  it("renders '—' when no κ entry exists for the row (no kappaMap given)", () => {
    render(<Grid schema={SCHEMA} rows={ROWS} loading={false} />);
    const cells = screen.getAllByTestId("kappa-cell");
    expect(cells).toHaveLength(3);
    for (const c of cells) {
      expect(c).toHaveTextContent("—");
      expect(c).toHaveAttribute("data-kappa-class", "ok");
    }
  });

  it("renders the κ value as a 2-decimal number with the kappa-class", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        kappaMap={kappa({ "S-OK": 0.1, "S-WARN": 0.9, "S-BAD": 50 })}
      />,
    );
    const cells = screen.getAllByTestId("kappa-cell");
    const get = (key: string) =>
      cells.find(
        (c) =>
          c.closest("[data-row-key]")?.getAttribute("data-row-key") === key,
      )!;
    expect(get("S-OK")).toHaveTextContent("0.10");
    expect(get("S-OK")).toHaveAttribute("data-kappa-class", "ok");
    expect(get("S-WARN")).toHaveTextContent("0.90");
    expect(get("S-WARN")).toHaveAttribute("data-kappa-class", "warn");
    // κ ≥ 10 switches to 1-decimal formatting to fit the gutter width.
    expect(get("S-BAD")).toHaveTextContent("50.0");
    expect(get("S-BAD")).toHaveAttribute("data-kappa-class", "bad");
  });
});
