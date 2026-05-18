import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { Charts } from "../../src/components/Charts";
import type { BundleSchema } from "../../src/lib/gigi-client";

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "site", type: "categorical" },
    { name: "temp", type: "numeric" },
    { name: "humidity", type: "numeric" },
  ],
  indexed_fields: ["sensor_id"],
  records: 4,
  storage_mode: "mmap",
};

const ROWS = [
  { sensor_id: "S-1", site: "N", temp: 22.5, humidity: 60 },
  { sensor_id: "S-2", site: "N", temp: 23.1, humidity: 61 },
  { sensor_id: "S-3", site: "S", temp: 38.7, humidity: 18 },
  { sensor_id: "S-4", site: "S", temp: 22.0, humidity: 59 },
];

describe("Charts view", () => {
  it("renders four chart cards when schema is present", () => {
    render(
      <Charts schema={SCHEMA} rows={ROWS} kappaMap={new Map()} coverField="site" />,
    );
    expect(screen.getByTestId("charts")).toBeInTheDocument();
    expect(screen.getByTestId("charts-cover-counts")).toBeInTheDocument();
    expect(screen.getByTestId("charts-histogram")).toBeInTheDocument();
    expect(screen.getByTestId("charts-kappa-by-row")).toBeInTheDocument();
    expect(screen.getByTestId("charts-conf-kappa")).toBeInTheDocument();
  });

  it("groups rows by cover field with correct counts", () => {
    render(
      <Charts schema={SCHEMA} rows={ROWS} kappaMap={new Map()} coverField="site" />,
    );
    const card = screen.getByTestId("charts-cover-counts");
    expect(card).toHaveTextContent(/Rows by site/);
    expect(screen.getByTestId("charts-bar-N")).toHaveTextContent("2");
    expect(screen.getByTestId("charts-bar-S")).toHaveTextContent("2");
  });

  it("renders the histogram only when a numeric field is selected", () => {
    render(
      <Charts schema={SCHEMA} rows={ROWS} kappaMap={new Map()} coverField="site" />,
    );
    const select = screen.getByTestId("charts-hist-field") as HTMLSelectElement;
    expect(["temp", "humidity"]).toContain(select.value);
    // At least one bin should be rendered.
    expect(screen.getByTestId("charts-hist-bin-0")).toBeInTheDocument();
  });

  it("falls back to an empty-state message when schema is null", () => {
    render(
      <Charts schema={null} rows={[]} kappaMap={new Map()} coverField="" />,
    );
    expect(screen.getByTestId("charts-empty")).toBeInTheDocument();
  });

  it("renders κ-by-row bars when kappa data exists", () => {
    const k = new Map<string, number>([
      ["S-1", 0.1],
      ["S-2", 0.2],
      ["S-3", 4.5],
      ["S-4", 0.3],
    ]);
    render(<Charts schema={SCHEMA} rows={ROWS} kappaMap={k} coverField="site" />);
    expect(screen.getByTestId("charts-kappa-bar-S-3")).toBeInTheDocument();
    expect(screen.getByTestId("charts-conf-point-S-3")).toBeInTheDocument();
  });
});
