import { describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { Geometry } from "../../src/components/Geometry";
import type { BundleSchema } from "../../src/lib/gigi-client";

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "site_id", type: "categorical" },
    { name: "temp", type: "numeric" },
    { name: "humidity", type: "numeric" },
    { name: "pressure", type: "numeric" },
  ],
  indexed_fields: ["sensor_id", "site_id"],
  records: 3,
  storage_mode: "mmap",
};

const ROWS = [
  { sensor_id: "S-001", site_id: "N", temp: 22, humidity: 60, pressure: 1013 },
  { sensor_id: "S-002", site_id: "N", temp: 23, humidity: 61, pressure: 1012 },
  { sensor_id: "S-003", site_id: "S", temp: 50, humidity: 10, pressure: 990 },
];

describe("Geometry — empty/loading states", () => {
  it("renders an empty state when schema is null", () => {
    render(
      <Geometry
        schema={null}
        rows={[]}
        kappaMap={new Map()}
        coverField="site_id"
        selectedRowKey={null}
        onRowSelect={() => {}}
      />,
    );
    expect(screen.getByTestId("geometry-empty")).toBeInTheDocument();
  });

  it("explains when the bundle has zero numeric fiber fields and points the user to Schema", () => {
    const noNumeric: BundleSchema = {
      ...SCHEMA,
      fiber_fields: [{ name: "site_id", type: "categorical" }],
    };
    render(
      <Geometry
        schema={noNumeric}
        rows={[]}
        kappaMap={new Map()}
        coverField="site_id"
        selectedRowKey={null}
        onRowSelect={() => {}}
      />,
    );
    const empty = screen.getByTestId("geometry-empty");
    expect(empty).toHaveTextContent(/no numeric fields/i);
    expect(empty).toHaveTextContent(/Schema/);
  });

  it("renders the scatter with a single numeric field (uses it for both axes)", () => {
    const oneNumeric: BundleSchema = {
      ...SCHEMA,
      fiber_fields: [
        { name: "site_id", type: "categorical" },
        { name: "temp", type: "numeric" },
      ],
    };
    render(
      <Geometry
        schema={oneNumeric}
        rows={ROWS}
        kappaMap={new Map()}
        coverField="site_id"
        selectedRowKey={null}
        onRowSelect={() => {}}
      />,
    );
    expect(screen.queryByTestId("geometry-empty")).toBeNull();
    expect(screen.getByTestId("geometry")).toBeInTheDocument();
    const xSelect = screen.getByTestId("x-field-select") as HTMLSelectElement;
    const ySelect = screen.getByTestId("y-field-select") as HTMLSelectElement;
    expect(xSelect.value).toBe("temp");
    expect(ySelect.value).toBe("temp");
  });
});

describe("Geometry — axis selectors + cover stats", () => {
  it("offers every numeric fiber field on both axes and defaults to the first two", () => {
    render(
      <Geometry
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={new Map()}
        coverField="site_id"
        selectedRowKey={null}
        onRowSelect={() => {}}
      />,
    );
    const xSelect = screen.getByTestId("x-field-select") as HTMLSelectElement;
    const ySelect = screen.getByTestId("y-field-select") as HTMLSelectElement;
    expect(Array.from(xSelect.options).map((o) => o.value)).toEqual([
      "temp",
      "humidity",
      "pressure",
    ]);
    expect(xSelect.value).toBe("temp");
    expect(ySelect.value).toBe("humidity");
  });

  it("switching the X axis swaps which field renders on the X axis label", () => {
    render(
      <Geometry
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={new Map()}
        coverField="site_id"
        selectedRowKey={null}
        onRowSelect={() => {}}
      />,
    );
    expect(screen.getByTestId("x-axis-label")).toHaveTextContent("temp");
    fireEvent.change(screen.getByTestId("x-field-select"), {
      target: { value: "pressure" },
    });
    expect(screen.getByTestId("x-axis-label")).toHaveTextContent("pressure");
  });

  it("renders one cover-stats row per distinct cover value, sorted by size", () => {
    render(
      <Geometry
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={new Map()}
        coverField="site_id"
        selectedRowKey={null}
        onRowSelect={() => {}}
      />,
    );
    expect(screen.getByTestId("cover-N-size")).toHaveTextContent("2");
    expect(screen.getByTestId("cover-S-size")).toHaveTextContent("1");
  });

  it("surfaces per-cover anomaly + drift counts", () => {
    const kappa = new Map<string, number>([
      ["S-001", 0.1],
      ["S-002", 1.2], // warn
      ["S-003", 4.2], // bad
    ]);
    render(
      <Geometry
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={kappa}
        coverField="site_id"
        selectedRowKey={null}
        onRowSelect={() => {}}
      />,
    );
    expect(screen.getByTestId("cover-N-drift")).toHaveTextContent("1");
    expect(screen.getByTestId("cover-S-anom")).toHaveTextContent("1");
  });

  it("propagates onRowSelect through the embedded Scatter", () => {
    const onRowSelect = vi.fn();
    render(
      <Geometry
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={new Map()}
        coverField="site_id"
        selectedRowKey={null}
        onRowSelect={onRowSelect}
      />,
    );
    fireEvent.click(screen.getByTestId("point-S-003"));
    expect(onRowSelect).toHaveBeenCalledWith("S-003");
  });
});
