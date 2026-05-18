import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { Grid } from "../../src/components/Grid";
import type { BundleSchema } from "../../src/lib/gigi-client";

const schema = {
  name: "tasks",
  base_fields: [{ name: "task_id", type: "text" }],
  fiber_fields: [
    { name: "title", type: "text" },
    { name: "status", type: "categorical" },
    { name: "hours", type: "numeric" },
  ],
  indexed_fields: ["task_id"],
  records: 2,
  storage_mode: "mmap",
} as unknown as BundleSchema;

const rows = [
  { task_id: "T-001", title: "First", status: "open", hours: 4 },
  { task_id: "T-002", title: "Second", status: "done", hours: 8 },
];

describe("Grid · column letter header row", () => {
  it("renders a letters row with A, B, C, D for the 4 columns", () => {
    render(
      <Grid
        schema={schema}
        rows={rows}
        loading={false}
        kappaMap={new Map()}
      />,
    );
    expect(screen.getByTestId("grid-letters-row")).toBeInTheDocument();
    expect(screen.getByTestId("grid-letter-task_id")).toHaveTextContent("A");
    expect(screen.getByTestId("grid-letter-title")).toHaveTextContent("B");
    expect(screen.getByTestId("grid-letter-status")).toHaveTextContent("C");
    expect(screen.getByTestId("grid-letter-hours")).toHaveTextContent("D");
  });

  it("each letter cell carries data-column for the field name", () => {
    render(
      <Grid
        schema={schema}
        rows={rows}
        loading={false}
        kappaMap={new Map()}
      />,
    );
    expect(screen.getByTestId("grid-letter-title")).toHaveAttribute(
      "data-column",
      "title",
    );
  });

  it("clicking a letter calls onColumnSelect with that column", () => {
    const onColumnSelect = vi.fn();
    render(
      <Grid
        schema={schema}
        rows={rows}
        loading={false}
        kappaMap={new Map()}
        onColumnSelect={onColumnSelect}
      />,
    );
    fireEvent.click(screen.getByTestId("grid-letter-status"));
    expect(onColumnSelect).toHaveBeenCalledWith("status");
  });

  it("clicking an already-selected letter toggles to null (clear)", () => {
    const onColumnSelect = vi.fn();
    render(
      <Grid
        schema={schema}
        rows={rows}
        loading={false}
        kappaMap={new Map()}
        selectedColumn="status"
        onColumnSelect={onColumnSelect}
      />,
    );
    fireEvent.click(screen.getByTestId("grid-letter-status"));
    expect(onColumnSelect).toHaveBeenCalledWith(null);
  });

  it("selected column gets the active class on its letter cell", () => {
    render(
      <Grid
        schema={schema}
        rows={rows}
        loading={false}
        kappaMap={new Map()}
        selectedColumn="title"
      />,
    );
    expect(screen.getByTestId("grid-letter-title").className).toMatch(
      /grid-letter-cell-active/,
    );
    expect(screen.getByTestId("grid-letter-status").className).not.toMatch(
      /grid-letter-cell-active/,
    );
  });

  it("right-click on a letter fires onColumnContextMenu with coords", () => {
    const onColumnContextMenu = vi.fn();
    render(
      <Grid
        schema={schema}
        rows={rows}
        loading={false}
        kappaMap={new Map()}
        onColumnContextMenu={onColumnContextMenu}
      />,
    );
    fireEvent.contextMenu(screen.getByTestId("grid-letter-hours"), {
      clientX: 200,
      clientY: 30,
    });
    expect(onColumnContextMenu).toHaveBeenCalledTimes(1);
    expect(onColumnContextMenu.mock.calls[0][0]).toBe("hours");
    expect(onColumnContextMenu.mock.calls[0][1]).toBe(200);
    expect(onColumnContextMenu.mock.calls[0][2]).toBe(30);
  });
});
