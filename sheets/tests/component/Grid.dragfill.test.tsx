import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { Grid } from "../../src/components/Grid";
import type { BundleSchema } from "../../src/lib/gigi-client";

/**
 * Drag-fill handle on the Grid.
 *
 * When a `cellRange` is active and `onDragFill` is provided, the
 * bottom-right cell of the range renders a small square handle.
 * Mousedown on the handle starts a fill-drag; mouseenter on cells
 * updates the projected target; mouseup fires `onDragFill` with the
 * source range + target cell so the host can extrapolate values.
 */

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "temp", type: "numeric" },
    { name: "humidity", type: "numeric" },
  ],
  indexed_fields: ["sensor_id"],
  records: 5,
  storage_mode: "mmap",
};

const ROWS = [
  { sensor_id: "S-1", temp: 1, humidity: 10 },
  { sensor_id: "S-2", temp: 2, humidity: 20 },
  { sensor_id: "S-3", temp: 3, humidity: 30 },
  { sensor_id: "S-4", temp: null, humidity: null },
  { sensor_id: "S-5", temp: null, humidity: null },
];

function cellOf(field: string, idx = 0): HTMLElement {
  return screen
    .getAllByTestId("editable-cell")
    .filter((c) => c.getAttribute("data-field") === field)[idx];
}

describe("Grid · drag-fill handle", () => {
  it("renders the handle on the bottom-right cell of the range", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => undefined}
        cellRange={{
          anchorRowKey: "S-1",
          anchorField: "temp",
          focusRowKey: "S-3",
          focusField: "temp",
        }}
        onCellRangeChange={() => undefined}
        onDragFill={() => undefined}
      />,
    );
    const handles = screen.getAllByTestId("grid-fill-handle");
    // Exactly one handle — at the range's bottom-right (S-3 / temp).
    expect(handles).toHaveLength(1);
  });

  it("no handle when there's no range", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => undefined}
        cellRange={null}
        onCellRangeChange={() => undefined}
        onDragFill={() => undefined}
      />,
    );
    expect(screen.queryByTestId("grid-fill-handle")).toBeNull();
  });

  it("no handle when onDragFill isn't provided (the host hasn't opted in)", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => undefined}
        cellRange={{
          anchorRowKey: "S-1",
          anchorField: "temp",
          focusRowKey: "S-3",
          focusField: "temp",
        }}
        onCellRangeChange={() => undefined}
      />,
    );
    expect(screen.queryByTestId("grid-fill-handle")).toBeNull();
  });

  it("mousedown on handle → mouseenter target → mouseup fires onDragFill", () => {
    const onDragFill = vi.fn();
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => undefined}
        cellRange={{
          anchorRowKey: "S-1",
          anchorField: "temp",
          focusRowKey: "S-3",
          focusField: "temp",
        }}
        onCellRangeChange={() => undefined}
        onDragFill={onDragFill}
      />,
    );
    const handle = screen.getByTestId("grid-fill-handle");
    fireEvent.mouseDown(handle);
    // Drag down to S-5.
    fireEvent.mouseEnter(cellOf("temp", 4));
    fireEvent.mouseUp(document);
    expect(onDragFill).toHaveBeenCalledTimes(1);
    const call = onDragFill.mock.calls[0][0];
    expect(call.source).toEqual({
      anchorRowKey: "S-1",
      anchorField: "temp",
      focusRowKey: "S-3",
      focusField: "temp",
    });
    expect(call.target).toEqual({ rowKey: "S-5", field: "temp" });
    expect(call.rowOrder).toEqual(["S-1", "S-2", "S-3", "S-4", "S-5"]);
    expect(call.fieldOrder).toEqual(["sensor_id", "temp", "humidity"]);
  });

  it("mouseup without a fill-drag does not fire onDragFill", () => {
    const onDragFill = vi.fn();
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => undefined}
        cellRange={{
          anchorRowKey: "S-1",
          anchorField: "temp",
          focusRowKey: "S-3",
          focusField: "temp",
        }}
        onCellRangeChange={() => undefined}
        onDragFill={onDragFill}
      />,
    );
    fireEvent.mouseUp(document);
    expect(onDragFill).not.toHaveBeenCalled();
  });
});
