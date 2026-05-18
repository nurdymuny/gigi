import { describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { Grid } from "../../src/components/Grid";
import type { BundleSchema } from "../../src/lib/gigi-client";

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "temp", type: "numeric" },
    { name: "humidity", type: "numeric" },
    { name: "operator", type: "text", encryption: "indexed" },
  ],
  indexed_fields: ["sensor_id"],
  records: 1,
  storage_mode: "mmap",
};

const ROWS = [
  { sensor_id: "S-001", temp: 22.5, humidity: 60.1, operator: "opr_d2a8" },
];

describe("Grid — inline edit", () => {
  it("renders editable cells with data-testid='editable-cell' for editable fields", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => {}}
      />,
    );
    const editables = screen.getAllByTestId("editable-cell");
    // sensor_id (key — editable via rename flow) + temp + humidity.
    // operator is encrypted, so it's not editable.
    expect(editables).toHaveLength(3);
    const fields = editables.map((e) => e.getAttribute("data-field"));
    expect(fields).toEqual(["sensor_id", "temp", "humidity"]);
  });

  it("does NOT render editable affordance when onCellEdit is omitted", () => {
    render(<Grid schema={SCHEMA} rows={ROWS} loading={false} />);
    expect(screen.queryByTestId("editable-cell")).toBeNull();
  });

  it("marks the primary key column editable with a rename-warning tooltip", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => {}}
      />,
    );
    const editables = screen.getAllByTestId("editable-cell");
    const keyCell = editables.find(
      (e) => e.getAttribute("data-field") === "sensor_id",
    );
    expect(keyCell, "key column should be editable").toBeDefined();
    // Tooltip warns the user about the rename-via-delete-then-insert flow.
    expect(keyCell!.getAttribute("title")?.toLowerCase()).toMatch(/rename/);
  });

  it("does NOT mark encrypted fields as editable", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => {}}
      />,
    );
    const editables = screen.getAllByTestId("editable-cell");
    const fields = editables.map((e) => e.getAttribute("data-field"));
    expect(fields).not.toContain("operator");
  });

  it("clicking an editable cell opens the inline editor with the current value", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => {}}
      />,
    );
    const tempCell = screen
      .getAllByTestId("editable-cell")
      .find((c) => c.getAttribute("data-field") === "temp")!;
    fireEvent.click(tempCell);

    const input = screen.getByTestId("cell-editor-input") as HTMLInputElement;
    expect(input).toBeInTheDocument();
    // Numeric cells render as text+inputMode="decimal" so a leading `=`
    // can survive (a number input would strip it). parseValue still
    // coerces digit-only commits back to numbers on the way out.
    expect(input).toHaveAttribute("type", "text");
    expect(input).toHaveAttribute("inputmode", "decimal");
    expect(input.value).toBe("22.5");
  });

  it("Enter commits the edit, calls onCellEdit with parsed numeric value", () => {
    const onCellEdit = vi.fn();
    render(
      <Grid schema={SCHEMA} rows={ROWS} loading={false} onCellEdit={onCellEdit} />,
    );
    const tempCell = screen
      .getAllByTestId("editable-cell")
      .find((c) => c.getAttribute("data-field") === "temp")!;
    fireEvent.click(tempCell);
    const input = screen.getByTestId("cell-editor-input") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "45.7" } });
    fireEvent.keyDown(input, { key: "Enter" });

    expect(onCellEdit).toHaveBeenCalledOnce();
    expect(onCellEdit).toHaveBeenCalledWith("S-001", "temp", 45.7);
    // Editor closed.
    expect(screen.queryByTestId("cell-editor-input")).toBeNull();
  });

  it("Escape cancels the edit, does NOT call onCellEdit", () => {
    const onCellEdit = vi.fn();
    render(
      <Grid schema={SCHEMA} rows={ROWS} loading={false} onCellEdit={onCellEdit} />,
    );
    const tempCell = screen
      .getAllByTestId("editable-cell")
      .find((c) => c.getAttribute("data-field") === "temp")!;
    fireEvent.click(tempCell);
    const input = screen.getByTestId("cell-editor-input") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "999" } });
    fireEvent.keyDown(input, { key: "Escape" });

    expect(onCellEdit).not.toHaveBeenCalled();
    expect(screen.queryByTestId("cell-editor-input")).toBeNull();
  });

  it("blur commits the edit (Excel-style implicit commit)", () => {
    const onCellEdit = vi.fn();
    render(
      <Grid schema={SCHEMA} rows={ROWS} loading={false} onCellEdit={onCellEdit} />,
    );
    const tempCell = screen
      .getAllByTestId("editable-cell")
      .find((c) => c.getAttribute("data-field") === "temp")!;
    fireEvent.click(tempCell);
    const input = screen.getByTestId("cell-editor-input") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "30.0" } });
    fireEvent.blur(input);

    expect(onCellEdit).toHaveBeenCalledWith("S-001", "temp", 30.0);
  });

  it("renders null cells as editable when onCellEdit is provided", () => {
    const rowsWithNull = [
      { sensor_id: "S-001", temp: null, humidity: 60.1, operator: "opr_d2a8" },
    ];
    render(
      <Grid
        schema={SCHEMA}
        rows={rowsWithNull}
        loading={false}
        onCellEdit={() => {}}
      />,
    );
    const editables = screen.getAllByTestId("editable-cell");
    const tempCell = editables.find((c) => c.getAttribute("data-field") === "temp");
    expect(tempCell).toBeDefined();
    // The placeholder em-dash still renders.
    expect(tempCell).toHaveTextContent("—");
    // And clicking it opens the editor with an empty initial value.
    fireEvent.click(tempCell!);
    const input = screen.getByTestId("cell-editor-input") as HTMLInputElement;
    expect(input.value).toBe("");
  });

  it("primary key column is editable even when the row has null neighbors", () => {
    const rowsWithNulls = [
      { sensor_id: "S-001", temp: null, humidity: null, operator: "opr_d2a8" },
    ];
    render(
      <Grid
        schema={SCHEMA}
        rows={rowsWithNulls}
        loading={false}
        onCellEdit={() => {}}
      />,
    );
    const editables = screen.getAllByTestId("editable-cell");
    expect(editables.map((e) => e.getAttribute("data-field"))).toContain(
      "sensor_id",
    );
  });

  it("commits an empty edit on a null cell as null (clears the cell)", () => {
    const onCellEdit = vi.fn();
    const rowsWithNull = [
      { sensor_id: "S-001", temp: 22.5, humidity: 60.1, operator: "opr_d2a8" },
    ];
    render(
      <Grid
        schema={SCHEMA}
        rows={rowsWithNull}
        loading={false}
        onCellEdit={onCellEdit}
      />,
    );
    const tempCell = screen
      .getAllByTestId("editable-cell")
      .find((c) => c.getAttribute("data-field") === "temp")!;
    fireEvent.click(tempCell);
    const input = screen.getByTestId("cell-editor-input") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onCellEdit).toHaveBeenCalledWith("S-001", "temp", null);
  });

  it("re-renders new value after parent updates the rows prop (optimistic UI from above)", () => {
    const { rerender } = render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        onCellEdit={() => {}}
      />,
    );
    expect(screen.getByTestId("grid-row")).toHaveTextContent("22.5");

    rerender(
      <Grid
        schema={SCHEMA}
        rows={[{ ...ROWS[0], temp: 45.7 }]}
        loading={false}
        onCellEdit={() => {}}
      />,
    );
    expect(screen.getByTestId("grid-row")).toHaveTextContent("45.7");
  });
});
