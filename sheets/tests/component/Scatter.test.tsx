import { describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { Scatter } from "../../src/components/Scatter";

const ROWS = [
  { sensor_id: "S-001", site_id: "N", temp: 22, humidity: 60 },
  { sensor_id: "S-002", site_id: "N", temp: 23, humidity: 61 },
  { sensor_id: "S-003", site_id: "S", temp: 50, humidity: 10 },
];

const KAPPA = new Map<string, number>([
  ["S-001", 0.1],
  ["S-002", 0.1],
  ["S-003", 4.2],
]);

describe("Scatter — render contract", () => {
  it("renders one circle per row with a finite (x, y) pair", () => {
    render(
      <Scatter
        rows={ROWS}
        keyField="sensor_id"
        coverField="site_id"
        xField="temp"
        yField="humidity"
        kappaMap={new Map()}
        selectedRowKey={null}
        onRowSelect={() => {}}
      />,
    );
    expect(screen.getByTestId("point-S-001")).toBeInTheDocument();
    expect(screen.getByTestId("point-S-002")).toBeInTheDocument();
    expect(screen.getByTestId("point-S-003")).toBeInTheDocument();
  });

  it("renders the empty state when no row has finite values", () => {
    render(
      <Scatter
        rows={[{ sensor_id: "A", temp: "x", humidity: "y", site_id: "N" }]}
        keyField="sensor_id"
        coverField="site_id"
        xField="temp"
        yField="humidity"
        kappaMap={new Map()}
        selectedRowKey={null}
        onRowSelect={() => {}}
      />,
    );
    expect(screen.getByTestId("scatter-empty")).toBeInTheDocument();
  });

  it("labels axes with the chosen field names", () => {
    render(
      <Scatter
        rows={ROWS}
        keyField="sensor_id"
        coverField="site_id"
        xField="temp"
        yField="humidity"
        kappaMap={new Map()}
        selectedRowKey={null}
        onRowSelect={() => {}}
      />,
    );
    expect(screen.getByTestId("x-axis-label")).toHaveTextContent("temp");
    expect(screen.getByTestId("y-axis-label")).toHaveTextContent("humidity");
  });

  it("draws a halo for anomaly rows and skips halos for healthy ones", () => {
    render(
      <Scatter
        rows={ROWS}
        keyField="sensor_id"
        coverField="site_id"
        xField="temp"
        yField="humidity"
        kappaMap={KAPPA}
        selectedRowKey={null}
        onRowSelect={() => {}}
      />,
    );
    expect(screen.getByTestId("halo-S-003")).toBeInTheDocument();
    expect(screen.queryByTestId("halo-S-001")).toBeNull();
  });

  it("annotates each point with its kappa class data-attribute", () => {
    render(
      <Scatter
        rows={ROWS}
        keyField="sensor_id"
        coverField="site_id"
        xField="temp"
        yField="humidity"
        kappaMap={KAPPA}
        selectedRowKey={null}
        onRowSelect={() => {}}
      />,
    );
    expect(screen.getByTestId("point-S-003")).toHaveAttribute(
      "data-kappa-class",
      "bad",
    );
    expect(screen.getByTestId("point-S-001")).toHaveAttribute(
      "data-kappa-class",
      "ok",
    );
  });

  it("colors points deterministically per cover value", () => {
    render(
      <Scatter
        rows={ROWS}
        keyField="sensor_id"
        coverField="site_id"
        xField="temp"
        yField="humidity"
        kappaMap={new Map()}
        selectedRowKey={null}
        onRowSelect={() => {}}
      />,
    );
    // S-001 and S-002 share site_id="N" → identical fill
    const a = screen.getByTestId("point-S-001").getAttribute("fill");
    const b = screen.getByTestId("point-S-002").getAttribute("fill");
    const c = screen.getByTestId("point-S-003").getAttribute("fill");
    expect(a).toBe(b);
    expect(a).not.toBe(c);
  });
});

describe("Scatter — selection + transport overlay", () => {
  it("renders a selection ring around the selected point", () => {
    render(
      <Scatter
        rows={ROWS}
        keyField="sensor_id"
        coverField="site_id"
        xField="temp"
        yField="humidity"
        kappaMap={new Map()}
        selectedRowKey="S-002"
        onRowSelect={() => {}}
      />,
    );
    expect(screen.getByTestId("ring-S-002")).toBeInTheDocument();
    expect(screen.queryByTestId("ring-S-001")).toBeNull();
  });

  it("draws the transport overlay between the selected point and its nearest peer", () => {
    render(
      <Scatter
        rows={ROWS}
        keyField="sensor_id"
        coverField="site_id"
        xField="temp"
        yField="humidity"
        kappaMap={new Map()}
        selectedRowKey="S-001"
        onRowSelect={() => {}}
      />,
    );
    expect(screen.getByTestId("transport-overlay")).toBeInTheDocument();
    // Nearest neighbor of S-001 (22, 60) is S-002 (23, 61), not S-003 (50, 10).
    expect(screen.getByTestId("peer-label")).toHaveTextContent("S-002");
  });

  it("omits the transport overlay when nothing is selected", () => {
    render(
      <Scatter
        rows={ROWS}
        keyField="sensor_id"
        coverField="site_id"
        xField="temp"
        yField="humidity"
        kappaMap={new Map()}
        selectedRowKey={null}
        onRowSelect={() => {}}
      />,
    );
    expect(screen.queryByTestId("transport-overlay")).toBeNull();
  });

  it("clicking a point fires onRowSelect with that row's key", () => {
    const onRowSelect = vi.fn();
    render(
      <Scatter
        rows={ROWS}
        keyField="sensor_id"
        coverField="site_id"
        xField="temp"
        yField="humidity"
        kappaMap={new Map()}
        selectedRowKey={null}
        onRowSelect={onRowSelect}
      />,
    );
    fireEvent.click(screen.getByTestId("point-S-003"));
    expect(onRowSelect).toHaveBeenCalledWith("S-003");
  });
});
