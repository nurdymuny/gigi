import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { Grid } from "../../src/components/Grid";
import type { BundleSchema } from "../../src/lib/gigi-client";

/**
 * Phase 4.C · grid renders formula-error sentinels as a red badge.
 *
 * When a formula evaluates to `#REF!` / `#DIV0!` / `#NAME!` / `#CIRC!`,
 * App.tsx writes that sentinel string into the bundle row. The grid
 * detects it (via `asError`) and renders a red pill badge instead of
 * the raw string, plus a tooltip with the original formula text.
 */

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [{ name: "ratio", type: "numeric" }],
  indexed_fields: ["sensor_id"],
  records: 2,
  storage_mode: "mmap",
};

describe("Grid · formula error badge", () => {
  it("renders an error badge for a cell containing #DIV0!", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={[
          { sensor_id: "S1", ratio: "#DIV0!" },
          { sensor_id: "S2", ratio: 1.5 },
        ]}
        loading={false}
        onCellEdit={() => {}}
        getFormulaText={(rk, f) =>
          rk === "S1" && f === "ratio" ? "=A1/0" : null
        }
      />,
    );
    const badges = screen.getAllByTestId("cell-error-badge");
    expect(badges).toHaveLength(1);
    expect(badges[0]).toHaveTextContent("#DIV0!");
  });

  it("renders no badge when the value is a plain string that isn't a sentinel", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={[{ sensor_id: "S1", ratio: 1.5 }]}
        loading={false}
        onCellEdit={() => {}}
      />,
    );
    expect(screen.queryByTestId("cell-error-badge")).toBeNull();
  });

  it("error cell carries data-error attribute + a tooltip including the formula text", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={[{ sensor_id: "S1", ratio: "#REF!" }]}
        loading={false}
        onCellEdit={() => {}}
        getFormulaText={(rk, f) =>
          rk === "S1" && f === "ratio" ? "=Z99" : null
        }
      />,
    );
    const cells = screen.getAllByTestId("editable-cell");
    const ratioCell = cells.find((c) => c.getAttribute("data-field") === "ratio");
    expect(ratioCell).toBeDefined();
    expect(ratioCell!.getAttribute("data-error")).toBe("#REF!");
    expect(ratioCell!.getAttribute("title")).toMatch(/#REF!/);
    expect(ratioCell!.getAttribute("title")).toMatch(/=Z99/);
  });
});
