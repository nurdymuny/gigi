import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { InsightsDrawer } from "../../src/components/InsightsDrawer";
import type { BundleSchema } from "../../src/lib/gigi-client";

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "site_id", type: "categorical" },
    { name: "temp", type: "numeric" },
    { name: "humidity", type: "numeric" },
  ],
  indexed_fields: ["sensor_id"],
  records: 0,
  storage_mode: "mmap",
};

const ROWS = [
  { sensor_id: "S-001", site_id: "N", temp: 22, humidity: 60 },
  { sensor_id: "S-002", site_id: "N", temp: 23, humidity: 61 },
  { sensor_id: "S-003", site_id: "N", temp: 24, humidity: 59 },
  { sensor_id: "S-OUT", site_id: "N", temp: 99, humidity: 5 },
];

const KAPPA = new Map<string, number>([
  ["S-001", 0.2],
  ["S-002", 0.2],
  ["S-003", 0.3],
  ["S-OUT", 4.2],
]);

describe("InsightsDrawer", () => {
  it("renders nothing when closed", () => {
    render(
      <InsightsDrawer
        open={false}
        bundle="sensors"
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={KAPPA}
        coverField="site_id"
        meanCurvature={1}
        onClose={() => {}}
      />,
    );
    expect(screen.queryByTestId("insights-drawer")).toBeNull();
  });

  it("renders a list of insights when open", () => {
    render(
      <InsightsDrawer
        open
        bundle="sensors"
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={KAPPA}
        coverField="site_id"
        meanCurvature={1}
        onClose={() => {}}
      />,
    );
    expect(screen.getByTestId("insights-drawer")).toBeInTheDocument();
    expect(screen.getByTestId("insights-list")).toBeInTheDocument();
    expect(screen.getByTestId("insight-cohort-top-anomalies")).toBeInTheDocument();
    expect(screen.getByTestId("insight-top-kappa")).toBeInTheDocument();
  });

  it("shows the empty state when no insights would be emitted", () => {
    render(
      <InsightsDrawer
        open
        bundle="sensors"
        schema={null}
        rows={[]}
        kappaMap={new Map()}
        coverField=""
        meanCurvature={0}
        onClose={() => {}}
      />,
    );
    expect(screen.getByTestId("insights-drawer-empty")).toBeInTheDocument();
  });

  it("Copy button writes the GQL to clipboard and calls onCopyGql", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    const onCopyGql = vi.fn();
    render(
      <InsightsDrawer
        open
        bundle="sensors"
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={KAPPA}
        coverField="site_id"
        meanCurvature={1}
        onClose={() => {}}
        onCopyGql={onCopyGql}
      />,
    );
    fireEvent.click(screen.getByTestId("insight-copy-cohort-top-anomalies"));
    await waitFor(() => expect(writeText).toHaveBeenCalled());
    await waitFor(() => expect(onCopyGql).toHaveBeenCalled());
    expect(writeText.mock.calls[0][0]).toContain("SECTION sensors WHERE site_id='N'");
  });

  it("Escape closes the drawer", () => {
    const onClose = vi.fn();
    render(
      <InsightsDrawer
        open
        bundle="sensors"
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={KAPPA}
        coverField="site_id"
        meanCurvature={1}
        onClose={onClose}
      />,
    );
    fireEvent.keyDown(document, { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
  });

  it("clicking the dimmed backdrop closes the drawer", () => {
    const onClose = vi.fn();
    render(
      <InsightsDrawer
        open
        bundle="sensors"
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={KAPPA}
        coverField="site_id"
        meanCurvature={1}
        onClose={onClose}
      />,
    );
    fireEvent.click(screen.getByTestId("insights-drawer-bg"));
    expect(onClose).toHaveBeenCalled();
  });

  it("tags each insight with its semantic kind via data-tag", () => {
    render(
      <InsightsDrawer
        open
        bundle="sensors"
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={KAPPA}
        coverField="site_id"
        meanCurvature={1}
        onClose={() => {}}
      />,
    );
    expect(screen.getByTestId("insight-cohort-top-anomalies")).toHaveAttribute(
      "data-tag",
      "bad",
    );
  });
});
