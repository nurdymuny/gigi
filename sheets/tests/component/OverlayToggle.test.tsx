import { describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { Toolbar } from "../../src/components/Toolbar";
import type { BundleSchema } from "../../src/lib/gigi-client";

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "site_id", type: "categorical" },
    { name: "region", type: "categorical" },
    { name: "temp", type: "numeric" },
    { name: "operator", type: "text", encryption: "indexed" },
  ],
  indexed_fields: ["sensor_id"],
  records: 0,
  storage_mode: "mmap",
};

describe("Toolbar — overlay toggle + cover field selector", () => {
  it("calls onOverlayChange when toggled on then off", () => {
    const onOverlayChange = vi.fn();
    const { rerender } = render(
      <Toolbar
        schema={SCHEMA}
        coverField="site_id"
        onCoverFieldChange={() => {}}
        overlayOn={false}
        onOverlayChange={onOverlayChange}
      />,
    );
    const toggle = screen.getByTestId("overlay-toggle");
    expect(toggle).toHaveAttribute("aria-pressed", "false");

    fireEvent.click(toggle);
    expect(onOverlayChange).toHaveBeenCalledWith(true);

    rerender(
      <Toolbar
        schema={SCHEMA}
        coverField="site_id"
        onCoverFieldChange={() => {}}
        overlayOn={true}
        onOverlayChange={onOverlayChange}
      />,
    );
    const toggleOn = screen.getByTestId("overlay-toggle");
    expect(toggleOn).toHaveAttribute("aria-pressed", "true");
    expect(toggleOn.className).toContain("toolbar-toggle-on");

    fireEvent.click(toggleOn);
    expect(onOverlayChange).toHaveBeenLastCalledWith(false);
  });

  it("offers categorical + text fiber fields and the primary key as cover choices", () => {
    render(
      <Toolbar
        schema={SCHEMA}
        coverField="site_id"
        onCoverFieldChange={() => {}}
        overlayOn={false}
        onOverlayChange={() => {}}
      />,
    );
    const select = screen.getByTestId("cover-field-select") as HTMLSelectElement;
    const options = Array.from(select.options).map((o) => o.value);
    // site_id + region (categorical), operator excluded (encrypted), sensor_id (key)
    expect(options).toEqual(["site_id", "region", "sensor_id"]);
  });

  it("calls onCoverFieldChange when the select changes", () => {
    const onCoverFieldChange = vi.fn();
    render(
      <Toolbar
        schema={SCHEMA}
        coverField="site_id"
        onCoverFieldChange={onCoverFieldChange}
        overlayOn={false}
        onOverlayChange={() => {}}
      />,
    );
    const select = screen.getByTestId("cover-field-select");
    fireEvent.change(select, { target: { value: "region" } });
    expect(onCoverFieldChange).toHaveBeenCalledWith("region");
  });

  it("renders an anomaly count chip when anomalyCount > 0", () => {
    render(
      <Toolbar
        schema={SCHEMA}
        coverField="site_id"
        onCoverFieldChange={() => {}}
        overlayOn={true}
        onOverlayChange={() => {}}
        anomalyCount={3}
        driftCount={2}
      />,
    );
    expect(screen.getByTestId("anom-count")).toHaveTextContent("3 anomalies");
    expect(screen.getByTestId("drift-count")).toHaveTextContent("2 drift");
  });

  it("does not render the stats row when both counts are 0", () => {
    render(
      <Toolbar
        schema={SCHEMA}
        coverField="site_id"
        onCoverFieldChange={() => {}}
        overlayOn={true}
        onOverlayChange={() => {}}
        anomalyCount={0}
        driftCount={0}
      />,
    );
    expect(screen.queryByTestId("toolbar-stats")).toBeNull();
  });

  it("disables the select when no cover choices exist (no schema)", () => {
    render(
      <Toolbar
        schema={null}
        coverField=""
        onCoverFieldChange={() => {}}
        overlayOn={false}
        onOverlayChange={() => {}}
      />,
    );
    const select = screen.getByTestId("cover-field-select") as HTMLSelectElement;
    expect(select).toBeDisabled();
  });
});
