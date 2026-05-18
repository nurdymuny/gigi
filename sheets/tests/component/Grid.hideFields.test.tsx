import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { Grid } from "../../src/components/Grid";
import type { BundleSchema } from "../../src/lib/gigi-client";

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "site_id", type: "categorical" },
    { name: "temp", type: "numeric" },
    { name: "humidity", type: "numeric" },
    { name: "operator", type: "text", encryption: "indexed" },
  ],
  indexed_fields: ["sensor_id"],
  records: 1,
  storage_mode: "mmap",
};

const ROWS = [
  { sensor_id: "S-001", site_id: "N", temp: 22.5, humidity: 60.1, operator: "opr" },
];

describe("Grid — hiddenFields", () => {
  it("renders every column when hiddenFields is omitted", () => {
    render(<Grid schema={SCHEMA} rows={ROWS} loading={false} />);
    expect(screen.getByTestId("header-sensor_id")).toBeInTheDocument();
    expect(screen.getByTestId("header-site_id")).toBeInTheDocument();
    expect(screen.getByTestId("header-temp")).toBeInTheDocument();
    expect(screen.getByTestId("header-humidity")).toBeInTheDocument();
    expect(screen.getByTestId("header-operator")).toBeInTheDocument();
  });

  it("hides exactly the fiber fields named in the set", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        hiddenFields={new Set(["humidity", "operator"])}
      />,
    );
    expect(screen.getByTestId("header-sensor_id")).toBeInTheDocument();
    expect(screen.getByTestId("header-site_id")).toBeInTheDocument();
    expect(screen.getByTestId("header-temp")).toBeInTheDocument();
    expect(screen.queryByTestId("header-humidity")).toBeNull();
    expect(screen.queryByTestId("header-operator")).toBeNull();
  });

  it("never hides the primary key (base_fields[0]) even when listed", () => {
    render(
      <Grid
        schema={SCHEMA}
        rows={ROWS}
        loading={false}
        hiddenFields={new Set(["sensor_id", "temp"])}
      />,
    );
    // Key column stays.
    expect(screen.getByTestId("header-sensor_id")).toBeInTheDocument();
    // Temp is hidden.
    expect(screen.queryByTestId("header-temp")).toBeNull();
  });

  it("first column (key) carries the sticky-left class so it stays during horizontal scroll", () => {
    render(<Grid schema={SCHEMA} rows={ROWS} loading={false} />);
    const keyHeader = screen.getByTestId("header-sensor_id");
    expect(keyHeader.className).toContain("grid-cell-sticky-key");
  });
});
