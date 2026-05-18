import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { Kanban } from "../../src/components/Kanban";
import type { BundleSchema } from "../../src/lib/gigi-client";

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "site", type: "categorical" },
    { name: "temp", type: "numeric" },
  ],
  indexed_fields: ["sensor_id"],
  records: 5,
  storage_mode: "mmap",
};

const ROWS = [
  { sensor_id: "S-1", site: "North", temp: 22 },
  { sensor_id: "S-2", site: "North", temp: 23 },
  { sensor_id: "S-3", site: "South", temp: 40 }, // anomaly
  { sensor_id: "S-4", site: "South", temp: 22 },
  { sensor_id: "S-5", site: "East", temp: 22 },
];

// κ: S-3 = 4.5 (bad), S-2 = 1.2 (warn), others = 0.1 (ok)
const KAPPA = new Map<string, number>([
  ["S-1", 0.1],
  ["S-2", 1.2],
  ["S-3", 4.5],
  ["S-4", 0.1],
  ["S-5", 0.1],
]);

describe("Kanban view", () => {
  it("renders an empty state when schema is null", () => {
    render(
      <Kanban
        schema={null}
        rows={[]}
        kappaMap={new Map()}
        coverField=""
        onRowSelect={() => {}}
      />,
    );
    expect(screen.getByTestId("kanban-empty")).toBeInTheDocument();
  });

  it("groups rows by κ class with healthy / drift / anomaly columns by default", () => {
    render(
      <Kanban
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={KAPPA}
        coverField="site"
        onRowSelect={() => {}}
      />,
    );
    expect(screen.getByTestId("kanban-col-ok")).toBeInTheDocument();
    expect(screen.getByTestId("kanban-col-warn")).toBeInTheDocument();
    expect(screen.getByTestId("kanban-col-bad")).toBeInTheDocument();
    // Three healthy (S-1, S-4, S-5), one drift (S-2), one anomaly (S-3).
    expect(screen.getByTestId("kanban-col-ok-count")).toHaveTextContent("3");
    expect(screen.getByTestId("kanban-col-warn-count")).toHaveTextContent("1");
    expect(screen.getByTestId("kanban-col-bad-count")).toHaveTextContent("1");
  });

  it("renders one card per row, attached to the right column", () => {
    render(
      <Kanban
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={KAPPA}
        coverField="site"
        onRowSelect={() => {}}
      />,
    );
    expect(screen.getByTestId("kanban-card-S-3").closest('[data-testid="kanban-col-bad"]')).toBeTruthy();
    expect(screen.getByTestId("kanban-card-S-2").closest('[data-testid="kanban-col-warn"]')).toBeTruthy();
    expect(screen.getByTestId("kanban-card-S-1").closest('[data-testid="kanban-col-ok"]')).toBeTruthy();
  });

  it("clicking a card fires onRowSelect with the row's key", () => {
    const onRowSelect = vi.fn();
    render(
      <Kanban
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={KAPPA}
        coverField="site"
        onRowSelect={onRowSelect}
      />,
    );
    fireEvent.click(screen.getByTestId("kanban-card-S-3"));
    expect(onRowSelect).toHaveBeenCalledWith("S-3");
  });

  it("switches grouping to a categorical cover field when selected", () => {
    render(
      <Kanban
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={KAPPA}
        coverField="site"
        onRowSelect={() => {}}
      />,
    );
    fireEvent.change(screen.getByTestId("kanban-groupby"), {
      target: { value: "site" },
    });
    expect(screen.getByTestId("kanban-col-North")).toBeInTheDocument();
    expect(screen.getByTestId("kanban-col-South")).toBeInTheDocument();
    expect(screen.getByTestId("kanban-col-East")).toBeInTheDocument();
  });

  it("cards include the row key + a κ pill", () => {
    render(
      <Kanban
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={KAPPA}
        coverField="site"
        onRowSelect={() => {}}
      />,
    );
    const card = screen.getByTestId("kanban-card-S-3");
    expect(card).toHaveTextContent("S-3");
    expect(card).toHaveTextContent(/4\.5/);
  });
});
