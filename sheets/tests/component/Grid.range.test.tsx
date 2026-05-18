import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { Grid } from "../../src/components/Grid";
import type { BundleSchema } from "../../src/lib/gigi-client";

/**
 * Range-select drag in the Grid.
 *
 *   mousedown anchors → range is { anchor: A, focus: A }
 *   mouseenter on B while held → range extends to { anchor: A, focus: B }
 *   mouseup ends the drag. A drag that moved suppresses the click-to-edit
 *   that would normally open the inline editor.
 */

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "temp", type: "numeric" },
    { name: "humidity", type: "numeric" },
  ],
  indexed_fields: ["sensor_id"],
  records: 3,
  storage_mode: "mmap",
};

const ROWS = [
  { sensor_id: "S-1", temp: 22.5, humidity: 60.1 },
  { sensor_id: "S-2", temp: 18.2, humidity: 45.0 },
  { sensor_id: "S-3", temp: 30.1, humidity: 80.0 },
];

function cellOf(field: string, idx = 0): HTMLElement {
  // The first cell in the row (sticky key) has no testid; we always
  // match through editable-cell + data-field.
  return screen.getAllByTestId("editable-cell").filter((c) => c.getAttribute("data-field") === field)[idx];
}

describe("Grid · range select", () => {
  it("mousedown on a cell sets a single-cell range", () => {
    const onCellRangeChange = vi.fn();
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => undefined}
        cellRange={null}
        onCellRangeChange={onCellRangeChange}
      />,
    );
    fireEvent.mouseDown(cellOf("temp", 0));
    expect(onCellRangeChange).toHaveBeenCalledTimes(1);
    expect(onCellRangeChange).toHaveBeenLastCalledWith({
      anchorRowKey: "S-1",
      anchorField: "temp",
      focusRowKey: "S-1",
      focusField: "temp",
    });
  });

  it("mouseenter on another cell during drag extends the range", () => {
    const onCellRangeChange = vi.fn();
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => undefined}
        cellRange={null}
        onCellRangeChange={onCellRangeChange}
      />,
    );
    fireEvent.mouseDown(cellOf("temp", 0));
    fireEvent.mouseEnter(cellOf("temp", 2));
    expect(onCellRangeChange).toHaveBeenLastCalledWith({
      anchorRowKey: "S-1",
      anchorField: "temp",
      focusRowKey: "S-3",
      focusField: "temp",
    });
  });

  it("rectangular drag spans columns AND rows", () => {
    const onCellRangeChange = vi.fn();
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => undefined}
        cellRange={null}
        onCellRangeChange={onCellRangeChange}
      />,
    );
    fireEvent.mouseDown(cellOf("temp", 0));
    fireEvent.mouseEnter(cellOf("humidity", 2));
    expect(onCellRangeChange).toHaveBeenLastCalledWith({
      anchorRowKey: "S-1",
      anchorField: "temp",
      focusRowKey: "S-3",
      focusField: "humidity",
    });
  });

  it("mouseenter without a prior mousedown does NOT change the range", () => {
    const onCellRangeChange = vi.fn();
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => undefined}
        cellRange={null}
        onCellRangeChange={onCellRangeChange}
      />,
    );
    fireEvent.mouseEnter(cellOf("temp", 0));
    fireEvent.mouseEnter(cellOf("temp", 1));
    expect(onCellRangeChange).not.toHaveBeenCalled();
  });

  it("a drag that moved suppresses the click-to-edit on mouseup", () => {
    // The editor would normally open on click. After a drag, the cell's
    // onClick should NOT open the editor (the drag was the user's intent).
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => undefined}
        cellRange={null}
        onCellRangeChange={() => undefined}
      />,
    );
    const start = cellOf("temp", 0);
    const end = cellOf("temp", 2);
    fireEvent.mouseDown(start);
    fireEvent.mouseEnter(end);
    // Mouseup happens on the same cell as drag-end.
    fireEvent.mouseUp(end);
    fireEvent.click(end);
    // No editor opened — the click was suppressed.
    expect(screen.queryByTestId("cell-editor-input")).toBeNull();
  });

  it("a plain click (no drag) still opens the editor", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => undefined}
        cellRange={null}
        onCellRangeChange={() => undefined}
      />,
    );
    const cell = cellOf("temp", 0);
    fireEvent.mouseDown(cell);
    fireEvent.mouseUp(cell);
    fireEvent.click(cell);
    expect(screen.getByTestId("cell-editor-input")).toBeInTheDocument();
  });

  it("renders the in-range class on cells inside the range prop", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => undefined}
        cellRange={{
          anchorRowKey: "S-1",
          anchorField: "temp",
          focusRowKey: "S-2",
          focusField: "humidity",
        }}
        onCellRangeChange={() => undefined}
      />,
    );
    // Cells inside the bbox: S-1/temp, S-1/humidity, S-2/temp, S-2/humidity.
    const inRange = screen
      .getAllByTestId("editable-cell")
      .filter((c) => c.getAttribute("data-in-range") === "true");
    expect(inRange.length).toBeGreaterThanOrEqual(4);
    // S-3 should be outside.
    expect(
      cellOf("temp", 2).getAttribute("data-in-range"),
    ).toBeNull();
  });

  it("Shift+click extends the existing range from its anchor", () => {
    const onCellRangeChange = vi.fn();
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => undefined}
        cellRange={{
          anchorRowKey: "S-1",
          anchorField: "temp",
          focusRowKey: "S-1",
          focusField: "temp",
        }}
        onCellRangeChange={onCellRangeChange}
      />,
    );
    fireEvent.mouseDown(cellOf("humidity", 2), { shiftKey: true });
    expect(onCellRangeChange).toHaveBeenLastCalledWith({
      anchorRowKey: "S-1",
      anchorField: "temp",
      focusRowKey: "S-3",
      focusField: "humidity",
    });
  });
});
