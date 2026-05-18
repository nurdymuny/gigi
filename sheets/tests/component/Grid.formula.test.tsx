import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { Grid } from "../../src/components/Grid";
import type { BundleSchema } from "../../src/lib/gigi-client";

/**
 * Phase 1.F · inline cell formulas in Grid.
 *
 * Contract (FORMULAS_SPEC §"Formula cell semantics"):
 *   - The bundle row holds the *evaluated value*.
 *   - The formula text lives in a sidecar; the Grid receives it through
 *     a `getFormulaText(rowKey, field)` callback.
 *   - Display: the cell shows the evaluated value (already in `value`),
 *     with a marker (`data-has-formula="true"`) so styles + tests can
 *     detect that this cell is a formula.
 *   - Edit: clicking the cell opens the editor with the *formula text*
 *     as the initial value, not the displayed result. This is the only
 *     way for the user to inspect or change the formula.
 *   - Commit: the Grid stays pure — it passes the raw string up via
 *     onCellEdit. The parent (App.tsx) is responsible for parsing `=`,
 *     evaluating, writing the result to the bundle, and updating the
 *     sidecar. The Grid does NOT evaluate or persist formulas.
 */

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "temp", type: "numeric" },
    { name: "humidity", type: "numeric" },
  ],
  indexed_fields: ["sensor_id"],
  records: 1,
  storage_mode: "mmap",
};

const ROWS = [
  // `humidity` holds the *evaluated* result of `=temp*2.5`; the formula
  // text lives in the sidecar that the test injects below.
  { sensor_id: "S-001", temp: 22.5, humidity: 56.25 },
];

describe("Grid · inline formulas", () => {
  it("renders the evaluated value (not the formula text) in the cell", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => {}}
        getFormulaText={(rk, f) =>
          rk === "S-001" && f === "humidity" ? "=temp*2.5" : null
        }
      />,
    );
    // The display is the resolved number, not "=temp*2.5".
    expect(screen.getByTestId("grid-row")).toHaveTextContent("56.25");
    expect(screen.getByTestId("grid-row")).not.toHaveTextContent("=temp");
  });

  it("marks formula cells with data-has-formula", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => {}}
        getFormulaText={(rk, f) =>
          rk === "S-001" && f === "humidity" ? "=temp*2.5" : null
        }
      />,
    );
    const cells = screen.getAllByTestId("editable-cell");
    const humidity = cells.find((c) => c.getAttribute("data-field") === "humidity");
    expect(humidity).toBeDefined();
    expect(humidity!.getAttribute("data-has-formula")).toBe("true");

    const temp = cells.find((c) => c.getAttribute("data-field") === "temp");
    expect(temp).toBeDefined();
    expect(temp!.getAttribute("data-has-formula")).toBeNull();
  });

  it("clicking a formula cell opens the editor with the formula text", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => {}}
        getFormulaText={(rk, f) =>
          rk === "S-001" && f === "humidity" ? "=temp*2.5" : null
        }
      />,
    );
    const cells = screen.getAllByTestId("editable-cell");
    const humidity = cells.find((c) => c.getAttribute("data-field") === "humidity")!;
    fireEvent.click(humidity);
    const input = screen.getByTestId("cell-editor-input") as HTMLInputElement;
    // The editor shows the *raw formula*, not the evaluated number.
    expect(input.value).toBe("=temp*2.5");
    // And it's a plain text input so we don't lose the `=` to number coercion.
    expect(input.getAttribute("type")).toBe("text");
  });

  it("clicking a non-formula numeric cell still opens with the displayed value", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => {}}
        getFormulaText={() => null}
      />,
    );
    const cells = screen.getAllByTestId("editable-cell");
    const temp = cells.find((c) => c.getAttribute("data-field") === "temp")!;
    fireEvent.click(temp);
    const input = screen.getByTestId("cell-editor-input") as HTMLInputElement;
    expect(input.value).toBe("22.5");
    // Numeric cells use type="text" with inputmode="decimal" so a `=`
    // typed into a value-only cell can convert it into a formula. The
    // <input type="number"> path strips the `=` silently.
    expect(input.getAttribute("type")).toBe("text");
    expect(input.getAttribute("inputmode")).toBe("decimal");
  });

  it("Enter commits a formula edit, passing the raw `=…` string to onCellEdit", () => {
    const onCellEdit = vi.fn();
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={onCellEdit}
        getFormulaText={(rk, f) =>
          rk === "S-001" && f === "humidity" ? "=temp*2.5" : null
        }
      />,
    );
    const cells = screen.getAllByTestId("editable-cell");
    const humidity = cells.find((c) => c.getAttribute("data-field") === "humidity")!;
    fireEvent.click(humidity);
    const input = screen.getByTestId("cell-editor-input") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "=temp*3" } });
    fireEvent.keyDown(input, { key: "Enter" });
    // The Grid hands the raw `=…` string back; parsing/eval is the parent's job.
    expect(onCellEdit).toHaveBeenCalledWith("S-001", "humidity", "=temp*3");
  });

  it("typing a leading `=` on a numeric cell promotes the commit to a formula string", () => {
    // Without this, a numeric input's onCommit would coerce "=A1+B1" via
    // parseValue (Number("=A1+B1") = NaN). Detecting the leading `=` and
    // committing the raw text is what enables turning a value into a formula.
    const onCellEdit = vi.fn();
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={onCellEdit}
      />,
    );
    const cells = screen.getAllByTestId("editable-cell");
    const temp = cells.find((c) => c.getAttribute("data-field") === "temp")!;
    fireEvent.click(temp);
    const input = screen.getByTestId("cell-editor-input") as HTMLInputElement;
    // The user types a formula into what was a numeric cell. The browser's
    // <input type=number> won't accept the `=`, so the grid must switch to
    // text mode the moment the draft starts with `=`. We simulate by
    // directly setting the value — the assertion is on the commit payload.
    fireEvent.change(input, { target: { value: "=10+5" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onCellEdit).toHaveBeenCalledWith("S-001", "temp", "=10+5");
  });
});
