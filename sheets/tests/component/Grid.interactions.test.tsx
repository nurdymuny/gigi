import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { Grid } from "../../src/components/Grid";
import type { BundleSchema } from "../../src/lib/gigi-client";

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "site_id", type: "categorical" },
    { name: "temp", type: "numeric" },
  ],
  indexed_fields: ["sensor_id"],
  records: 3,
  storage_mode: "mmap",
};

const ROWS = [
  { sensor_id: "S-001", site_id: "N", temp: 22 },
  { sensor_id: "S-002", site_id: "N", temp: 23 },
  { sensor_id: "S-003", site_id: "S", temp: 50 },
];

describe("Grid — multi-select highlighting", () => {
  it("highlights every row in selectedKeys, not just selectedRowKey", () => {
    const keys = new Set(["S-001", "S-003"]);
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        selectedRowKey="S-001"
        selectedKeys={keys}
        onRowClick={() => {}}
      />,
    );
    const selected = screen
      .getAllByTestId("grid-row")
      .filter((r) => r.getAttribute("data-selected") === "true");
    expect(selected).toHaveLength(2);
    // selectedRowKey is the "focus" row.
    const focused = screen.getAllByTestId("grid-row").filter(
      (r) => r.getAttribute("data-focused") === "true",
    );
    expect(focused).toHaveLength(1);
    expect(focused[0]).toHaveAttribute("data-row-key", "S-001");
  });

  it("falls back to selectedRowKey when selectedKeys is omitted (back-compat)", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        selectedRowKey="S-002"
        onRowSelect={() => {}}
      />,
    );
    const selected = screen.getAllByTestId("grid-row").filter(
      (r) => r.getAttribute("data-selected") === "true",
    );
    expect(selected).toHaveLength(1);
    expect(selected[0]).toHaveAttribute("data-row-key", "S-002");
  });
});

describe("Grid — click + modifier handling", () => {
  it("plain click fires onRowClick with no modifiers", () => {
    const onRowClick = vi.fn();
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        selectedKeys={new Set()}
        onRowClick={onRowClick}
      />,
    );
    fireEvent.click(screen.getAllByTestId("grid-row")[0]);
    expect(onRowClick).toHaveBeenCalledWith("S-001", {
      meta: false,
      shift: false,
      alt: false,
    });
  });

  it("Cmd/Ctrl-click sets mods.meta", () => {
    const onRowClick = vi.fn();
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        selectedKeys={new Set()}
        onRowClick={onRowClick}
      />,
    );
    fireEvent.click(screen.getAllByTestId("grid-row")[1], { metaKey: true });
    expect(onRowClick).toHaveBeenCalledWith(
      "S-002",
      expect.objectContaining({ meta: true }),
    );
  });

  it("Shift-click sets mods.shift", () => {
    const onRowClick = vi.fn();
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        selectedKeys={new Set()}
        onRowClick={onRowClick}
      />,
    );
    fireEvent.click(screen.getAllByTestId("grid-row")[2], { shiftKey: true });
    expect(onRowClick).toHaveBeenCalledWith(
      "S-003",
      expect.objectContaining({ shift: true }),
    );
  });

  it("falls back to legacy onRowSelect when onRowClick is not provided", () => {
    const onRowSelect = vi.fn();
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        selectedKeys={new Set()}
        onRowSelect={onRowSelect}
      />,
    );
    fireEvent.click(screen.getAllByTestId("grid-row")[0]);
    expect(onRowSelect).toHaveBeenCalledWith("S-001");
  });

  it("plain click on an editable cell ALSO selects the row (spreadsheet convention)", () => {
    // Click selects the row (so the inspector updates) AND the cell's own
    // onClick opens the editor. Both behaviors are intentional.
    const onRowClick = vi.fn();
    const onCellEdit = vi.fn();
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        selectedKeys={new Set()}
        onRowClick={onRowClick}
        onCellEdit={onCellEdit}
      />,
    );
    const tempCell = screen
      .getAllByTestId("editable-cell")
      .find((c) => c.getAttribute("data-field") === "temp")!;
    fireEvent.click(tempCell);
    expect(onRowClick).toHaveBeenCalledWith("S-001", {
      meta: false,
      shift: false,
      alt: false,
    });
  });
});

describe("Grid — right-click context menu", () => {
  it("right-click fires onRowContextMenu with viewport coordinates + the row key", () => {
    const onRowContextMenu = vi.fn();
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        selectedKeys={new Set()}
        onRowContextMenu={onRowContextMenu}
      />,
    );
    fireEvent.contextMenu(screen.getAllByTestId("grid-row")[1], {
      clientX: 250,
      clientY: 400,
    });
    expect(onRowContextMenu).toHaveBeenCalledWith("S-002", 250, 400);
  });

  it("right-click without an onRowContextMenu prop does not throw", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        selectedKeys={new Set()}
      />,
    );
    expect(() => {
      fireEvent.contextMenu(screen.getAllByTestId("grid-row")[0]);
    }).not.toThrow();
  });
});
