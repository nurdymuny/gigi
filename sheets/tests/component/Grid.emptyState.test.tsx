import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { Grid } from "../../src/components/Grid";
import type { BundleSchema } from "../../src/lib/gigi-client";

const SCHEMA: BundleSchema = {
  name: "bees_demo",
  base_fields: [{ name: "id", type: "text" }],
  fiber_fields: [{ name: "value", type: "text" }],
  indexed_fields: ["id"],
  records: 0,
  storage_mode: "mmap",
};

describe("Grid — actionable empty state", () => {
  it('shows "This bundle is empty" copy when schema.records === 0', () => {
    render(<Grid schema={SCHEMA} rows={[]} loading={false} />);
    expect(screen.getByTestId("grid-empty")).toHaveTextContent(
      /this bundle is empty/i,
    );
  });

  it('shows "No rows match" copy when records > 0 but query returned 0', () => {
    render(
      <Grid
        schema={{ ...SCHEMA, records: 250 }}
        rows={[]}
        loading={false}
      />,
    );
    expect(screen.getByTestId("grid-empty")).toHaveTextContent(
      /no rows match/i,
    );
  });

  it("renders Add row / Import / Schema buttons when callbacks are provided", () => {
    const onInsertRow = vi.fn();
    const onImportCsv = vi.fn();
    const onOpenSchema = vi.fn();
    render(
      <Grid
        schema={SCHEMA}
        rows={[]}
        loading={false}
        emptyActions={{ onInsertRow, onImportCsv, onOpenSchema }}
      />,
    );
    fireEvent.click(screen.getByTestId("grid-empty-insert"));
    expect(onInsertRow).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByTestId("grid-empty-import"));
    expect(onImportCsv).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByTestId("grid-empty-schema"));
    expect(onOpenSchema).toHaveBeenCalledOnce();
  });

  it("omits action buttons whose callbacks are absent", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={[]}
        loading={false}
        emptyActions={{}}
      />,
    );
    expect(screen.queryByTestId("grid-empty-insert")).toBeNull();
    expect(screen.queryByTestId("grid-empty-import")).toBeNull();
    expect(screen.queryByTestId("grid-empty-schema")).toBeNull();
  });

  it('right-clicking the empty area dispatches onRowContextMenu with rowKey=""', () => {
    const onRowContextMenu = vi.fn();
    render(
      <Grid
        schema={SCHEMA}
        rows={[]}
        loading={false}
        onRowContextMenu={onRowContextMenu}
      />,
    );
    fireEvent.contextMenu(screen.getByTestId("grid-empty"), {
      clientX: 320,
      clientY: 200,
    });
    expect(onRowContextMenu).toHaveBeenCalledWith("", 320, 200);
  });
});
