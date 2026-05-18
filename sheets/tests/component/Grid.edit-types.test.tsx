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
    { name: "active", type: "boolean" },
    { name: "due_date", type: "timestamp" },
    { name: "hours", type: "numeric" },
  ],
  indexed_fields: ["task_id"],
  records: 3,
  storage_mode: "mmap",
} as unknown as BundleSchema;

const rows = [
  { task_id: "T-001", title: "Build login", status: "in-progress", active: true, due_date: "2026-05-22", hours: 8 },
  { task_id: "T-002", title: "Pricing copy", status: "review", active: true, due_date: "2026-05-25", hours: 4 },
  { task_id: "T-003", title: "Compliance", status: "done", active: false, due_date: "2026-05-15", hours: 12 },
];

describe("Grid · expanded cell-edit types", () => {
  it("categorical cell opens an editor (was previously not editable)", () => {
    const onCellEdit = vi.fn();
    render(
      <Grid
        schema={schema}
        rows={rows}
        loading={false}
        kappaMap={new Map()}
        onCellEdit={onCellEdit}
      />,
    );
    // Find the status cell on the first row.
    const cells = screen.getAllByTestId("editable-cell");
    const statusCell = cells.find(
      (c) => c.getAttribute("data-field") === "status",
    );
    expect(statusCell).toBeDefined();
    fireEvent.click(statusCell!);
    expect(screen.getByTestId("cell-editor")).toBeInTheDocument();
  });

  it("boolean cell renders a select editor with true/false/—", () => {
    const onCellEdit = vi.fn();
    render(
      <Grid
        schema={schema}
        rows={rows}
        loading={false}
        kappaMap={new Map()}
        onCellEdit={onCellEdit}
      />,
    );
    const cells = screen.getAllByTestId("editable-cell");
    const activeCell = cells.find(
      (c) => c.getAttribute("data-field") === "active",
    );
    expect(activeCell).toBeDefined();
    fireEvent.click(activeCell!);
    const editor = screen.getByTestId("cell-editor-input");
    expect(editor.tagName).toBe("SELECT");
    // Three options: blank, true, false.
    expect(editor.querySelectorAll("option")).toHaveLength(3);
  });

  it("timestamp cell renders a date input", () => {
    const onCellEdit = vi.fn();
    render(
      <Grid
        schema={schema}
        rows={rows}
        loading={false}
        kappaMap={new Map()}
        onCellEdit={onCellEdit}
      />,
    );
    const cells = screen.getAllByTestId("editable-cell");
    const dateCell = cells.find(
      (c) => c.getAttribute("data-field") === "due_date",
    );
    expect(dateCell).toBeDefined();
    fireEvent.click(dateCell!);
    const editor = screen.getByTestId("cell-editor-input") as HTMLInputElement;
    expect(editor.type).toBe("date");
    expect(editor.value).toBe("2026-05-22");
  });

  it("commits a boolean edit as a real boolean (not a string)", () => {
    const onCellEdit = vi.fn();
    render(
      <Grid
        schema={schema}
        rows={rows}
        loading={false}
        kappaMap={new Map()}
        onCellEdit={onCellEdit}
      />,
    );
    const cells = screen.getAllByTestId("editable-cell");
    const activeCell = cells.find(
      (c) => c.getAttribute("data-field") === "active",
    );
    fireEvent.click(activeCell!);
    const editor = screen.getByTestId("cell-editor-input");
    fireEvent.change(editor, { target: { value: "false" } });
    fireEvent.keyDown(editor, { key: "Enter" });
    expect(onCellEdit).toHaveBeenCalledTimes(1);
    expect(onCellEdit.mock.calls[0]).toEqual(["T-001", "active", false]);
  });

  it("commits a categorical edit as a string", () => {
    const onCellEdit = vi.fn();
    render(
      <Grid
        schema={schema}
        rows={rows}
        loading={false}
        kappaMap={new Map()}
        onCellEdit={onCellEdit}
      />,
    );
    const cells = screen.getAllByTestId("editable-cell");
    const statusCell = cells.find(
      (c) => c.getAttribute("data-field") === "status",
    );
    fireEvent.click(statusCell!);
    const editor = screen.getByTestId("cell-editor-input");
    fireEvent.change(editor, { target: { value: "backlog" } });
    fireEvent.keyDown(editor, { key: "Enter" });
    expect(onCellEdit).toHaveBeenCalledTimes(1);
    expect(onCellEdit.mock.calls[0]).toEqual(["T-001", "status", "backlog"]);
  });

  it("primary key IS editable, with a rename-warning tooltip", () => {
    const onCellEdit = vi.fn();
    render(
      <Grid
        schema={schema}
        rows={rows}
        loading={false}
        kappaMap={new Map()}
        onCellEdit={onCellEdit}
      />,
    );
    const cells = screen.getAllByTestId("editable-cell");
    const idCell = cells.find(
      (c) => c.getAttribute("data-field") === "task_id",
    );
    expect(idCell, "key cell should be in editable-cell set").toBeDefined();
    // Tooltip flags the special semantics so the user isn't surprised
    // by the confirmation dialog when they commit.
    const title = idCell!.getAttribute("title") ?? "";
    expect(title.toLowerCase()).toMatch(/rename/);
    expect(title.toLowerCase()).toMatch(/delete/);
  });

  it("key column gets the .grid-cell-key-editable class (orange hover)", () => {
    render(
      <Grid
        schema={schema}
        rows={rows}
        loading={false}
        kappaMap={new Map()}
        onCellEdit={() => undefined}
      />,
    );
    const cells = screen.getAllByTestId("editable-cell");
    const idCell = cells.find(
      (c) => c.getAttribute("data-field") === "task_id",
    );
    expect(idCell!.className).toMatch(/grid-cell-key-editable/);
    // Non-key editable cells don't get the warning class.
    const titleCell = cells.find(
      (c) => c.getAttribute("data-field") === "title",
    );
    expect(titleCell!.className).not.toMatch(/grid-cell-key-editable/);
  });

  it("clicking the key cell opens the editor (so rename can happen)", () => {
    render(
      <Grid
        schema={schema}
        rows={rows}
        loading={false}
        kappaMap={new Map()}
        onCellEdit={() => undefined}
      />,
    );
    const cells = screen.getAllByTestId("editable-cell");
    const idCell = cells.find(
      (c) => c.getAttribute("data-field") === "task_id",
    );
    fireEvent.click(idCell!);
    expect(screen.getByTestId("cell-editor")).toBeInTheDocument();
    const editor = screen.getByTestId("cell-editor-input") as HTMLInputElement;
    expect(editor.value).toBe("T-001");
  });
});
