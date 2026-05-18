import { describe, expect, it } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { Grid } from "../../src/components/Grid";
import type { BundleSchema } from "../../src/lib/gigi-client";

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [{ name: "temp", type: "numeric" }],
  indexed_fields: ["sensor_id"],
  records: 3,
  storage_mode: "mmap",
};

const ROWS = [
  { sensor_id: "S-100", temp: 22.5 },
  { sensor_id: "S-200", temp: 18.2 },
  { sensor_id: "S-300", temp: 30.1 },
];

describe("Grid — row number gutter", () => {
  it("renders a row-number column header (#)", () => {
    render(<Grid schema={SCHEMA} rows={ROWS} loading={false} />);
    expect(screen.getByTestId("header-row-number")).toBeInTheDocument();
    expect(screen.getByTestId("header-row-number")).toHaveTextContent("#");
  });

  it("renders one row-number cell per row, numbered sequentially starting at 1", () => {
    render(<Grid schema={SCHEMA} rows={ROWS} loading={false} />);
    const nums = screen.getAllByTestId("row-number");
    expect(nums).toHaveLength(3);
    expect(nums[0]).toHaveTextContent("1");
    expect(nums[1]).toHaveTextContent("2");
    expect(nums[2]).toHaveTextContent("3");
  });

  it("re-numbers after sorting so the top row is always #1", () => {
    render(<Grid schema={SCHEMA} rows={ROWS} loading={false} />);
    // Original temp order: 22.5, 18.2, 30.1
    // After ascending sort on temp: 18.2 (S-200), 22.5 (S-100), 30.1 (S-300)
    fireEvent.click(screen.getByTestId("header-temp"));
    const nums = screen.getAllByTestId("row-number");
    expect(nums[0]).toHaveTextContent("1");
    expect(nums[1]).toHaveTextContent("2");
    expect(nums[2]).toHaveTextContent("3");
    // Verify the row order actually changed.
    const rows = document.querySelectorAll("[data-row-key]");
    expect(rows[0].getAttribute("data-row-key")).toBe("S-200");
  });

  it("the row-number cell carries row-number-sticky positioning class", () => {
    render(<Grid schema={SCHEMA} rows={ROWS} loading={false} />);
    const first = screen.getAllByTestId("row-number")[0];
    expect(first.className).toMatch(/grid-cell-sticky-row-number/);
  });
});
