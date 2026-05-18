import { describe, expect, it } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { Grid } from "../../src/components/Grid";
import type { BundleSchema } from "../../src/lib/gigi-client";

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "temp", type: "numeric" },
    { name: "operator", type: "text", encryption: "opaque" },
  ],
  indexed_fields: ["sensor_id"],
  records: 3,
  storage_mode: "mmap",
};

const ROWS = [
  { sensor_id: "S-002", temp: 30.1, operator: "x" },
  { sensor_id: "S-001", temp: 12.5, operator: "y" },
  { sensor_id: "S-003", temp: 22.0, operator: "z" },
];

function visibleRowKeys(): string[] {
  return Array.from(document.querySelectorAll("[data-row-key]")).map(
    (el) => el.getAttribute("data-row-key") || "",
  );
}

describe("Grid — column sort", () => {
  it("renders rows in their original order when no sort is active", () => {
    render(<Grid schema={SCHEMA} rows={ROWS} loading={false} />);
    expect(visibleRowKeys()).toEqual(["S-002", "S-001", "S-003"]);
  });

  it("marks headers with aria-sort='none' when no sort is active", () => {
    render(<Grid schema={SCHEMA} rows={ROWS} loading={false} />);
    expect(screen.getByTestId("header-sensor_id")).toHaveAttribute("aria-sort", "none");
    expect(screen.getByTestId("header-temp")).toHaveAttribute("aria-sort", "none");
  });

  it("clicking a header sorts that column ascending and shows a chevron", () => {
    render(<Grid schema={SCHEMA} rows={ROWS} loading={false} />);
    fireEvent.click(screen.getByTestId("header-temp"));
    expect(visibleRowKeys()).toEqual(["S-001", "S-003", "S-002"]);
    expect(screen.getByTestId("header-temp")).toHaveAttribute("aria-sort", "ascending");
    expect(screen.getByTestId("sort-indicator")).toHaveAttribute("data-sort-dir", "asc");
  });

  it("clicking the same header again flips to descending", () => {
    render(<Grid schema={SCHEMA} rows={ROWS} loading={false} />);
    fireEvent.click(screen.getByTestId("header-temp"));
    fireEvent.click(screen.getByTestId("header-temp"));
    expect(visibleRowKeys()).toEqual(["S-002", "S-003", "S-001"]);
    expect(screen.getByTestId("header-temp")).toHaveAttribute("aria-sort", "descending");
    expect(screen.getByTestId("sort-indicator")).toHaveAttribute("data-sort-dir", "desc");
  });

  it("clicking a third time clears the sort", () => {
    render(<Grid schema={SCHEMA} rows={ROWS} loading={false} />);
    fireEvent.click(screen.getByTestId("header-temp"));
    fireEvent.click(screen.getByTestId("header-temp"));
    fireEvent.click(screen.getByTestId("header-temp"));
    expect(visibleRowKeys()).toEqual(["S-002", "S-001", "S-003"]);
    expect(screen.getByTestId("header-temp")).toHaveAttribute("aria-sort", "none");
    expect(screen.queryByTestId("sort-indicator")).toBeNull();
  });

  it("clicking a different column switches the sort to that column (ascending)", () => {
    render(<Grid schema={SCHEMA} rows={ROWS} loading={false} />);
    fireEvent.click(screen.getByTestId("header-temp")); // temp asc
    fireEvent.click(screen.getByTestId("header-sensor_id")); // switch → sensor_id asc
    expect(visibleRowKeys()).toEqual(["S-001", "S-002", "S-003"]);
    expect(screen.getByTestId("header-sensor_id")).toHaveAttribute("aria-sort", "ascending");
    expect(screen.getByTestId("header-temp")).toHaveAttribute("aria-sort", "none");
  });

  it("sorts text columns alphabetically (locale-aware, numeric-aware)", () => {
    const rows = [
      { sensor_id: "S-10", temp: 1, operator: "" },
      { sensor_id: "S-2", temp: 2, operator: "" },
      { sensor_id: "S-1", temp: 3, operator: "" },
    ];
    render(<Grid schema={SCHEMA} rows={rows} loading={false} />);
    fireEvent.click(screen.getByTestId("header-sensor_id"));
    // localeCompare with numeric:true treats "S-2" < "S-10".
    expect(visibleRowKeys()).toEqual(["S-1", "S-2", "S-10"]);
  });

  it("pushes nulls to the end when sorting ascending", () => {
    const rows = [
      { sensor_id: "A", temp: 5, operator: "" },
      { sensor_id: "B", temp: null, operator: "" },
      { sensor_id: "C", temp: 1, operator: "" },
    ];
    render(<Grid schema={SCHEMA} rows={rows} loading={false} />);
    fireEvent.click(screen.getByTestId("header-temp"));
    expect(visibleRowKeys()).toEqual(["C", "A", "B"]);
  });

  it("does NOT make OPAQUE-encrypted column headers sortable", () => {
    render(<Grid schema={SCHEMA} rows={ROWS} loading={false} />);
    const opHeader = screen.getByTestId("header-operator");
    expect(opHeader).not.toHaveClass("grid-hcell-sortable");
    fireEvent.click(opHeader);
    // No sort applied — order unchanged.
    expect(visibleRowKeys()).toEqual(["S-002", "S-001", "S-003"]);
  });

  it("κ column header is clickable and sorts by computed κ value", () => {
    const k = new Map<string, number>([
      ["S-001", 4.5],
      ["S-002", 0.1],
      ["S-003", 1.2],
    ]);
    const rows = [
      { sensor_id: "S-001", temp: 22, operator: "" },
      { sensor_id: "S-002", temp: 23, operator: "" },
      { sensor_id: "S-003", temp: 24, operator: "" },
    ];
    render(
      <Grid schema={SCHEMA} rows={rows} loading={false} kappaMap={k} />,
    );
    const kappaHeader = screen.getByTestId("header-kappa");
    expect(kappaHeader).toHaveAttribute("aria-sort", "none");
    fireEvent.click(kappaHeader);
    // Ascending: 0.1 (S-002), 1.2 (S-003), 4.5 (S-001).
    expect(visibleRowKeys()).toEqual(["S-002", "S-003", "S-001"]);
    expect(kappaHeader).toHaveAttribute("aria-sort", "ascending");
    fireEvent.click(kappaHeader);
    expect(visibleRowKeys()).toEqual(["S-001", "S-003", "S-002"]);
    expect(kappaHeader).toHaveAttribute("aria-sort", "descending");
  });

  it("clicking the resize handle does NOT trigger a sort", () => {
    render(<Grid schema={SCHEMA} rows={ROWS} loading={false} />);
    fireEvent.click(screen.getByTestId("resize-temp"));
    expect(screen.getByTestId("header-temp")).toHaveAttribute("aria-sort", "none");
    expect(visibleRowKeys()).toEqual(["S-002", "S-001", "S-003"]);
  });
});
