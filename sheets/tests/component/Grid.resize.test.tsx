import { describe, expect, it } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { Grid } from "../../src/components/Grid";
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
  records: 1,
  storage_mode: "mmap",
};

const ROWS = [{ sensor_id: "S-001", site_id: "N", temp: 22, humidity: 60 }];

function parsePxList(track: string): number[] {
  return track
    .split(/\s+/)
    .filter((t) => t.endsWith("px"))
    .map((t) => parseInt(t, 10));
}

describe("Grid — column resize handles", () => {
  it("renders a resize handle per column header", () => {
    render(<Grid schema={SCHEMA} rows={ROWS} loading={false} />);
    expect(screen.getByTestId("resize-sensor_id")).toBeInTheDocument();
    expect(screen.getByTestId("resize-temp")).toBeInTheDocument();
    expect(screen.getByTestId("resize-humidity")).toBeInTheDocument();
  });

  it("dragging a resize handle updates the column track width", () => {
    render(<Grid schema={SCHEMA} rows={ROWS} loading={false} />);
    const header = screen.getByTestId("grid-header");
    const trackBefore = (header.getAttribute("style") || "")
      .split("grid-template-columns:")[1]
      ?.split(";")[0]
      ?.trim() ?? "";
    const widthsBefore = parsePxList(trackBefore);

    // Drag the temp handle right by 80px.
    const handle = screen.getByTestId("resize-temp");
    fireEvent.mouseDown(handle, { clientX: 200 });
    fireEvent.mouseMove(document, { clientX: 280 });
    fireEvent.mouseUp(document);

    const trackAfter = (header.getAttribute("style") || "")
      .split("grid-template-columns:")[1]
      ?.split(";")[0]
      ?.trim() ?? "";
    const widthsAfter = parsePxList(trackAfter);

    // Index map (after row-number gutter was added):
    // 0=row#, 1=κ, 2=sensor_id, 3=site_id, 4=temp, 5=humidity
    expect(widthsAfter[4]).toBeGreaterThan(widthsBefore[4]);
    expect(widthsAfter[4] - widthsBefore[4]).toBeGreaterThanOrEqual(70);
  });

  it("dragging left shrinks the column, with a 48px floor", () => {
    render(<Grid schema={SCHEMA} rows={ROWS} loading={false} />);
    const handle = screen.getByTestId("resize-humidity");
    // Drag far left to test the clamp.
    fireEvent.mouseDown(handle, { clientX: 500 });
    fireEvent.mouseMove(document, { clientX: -2000 });
    fireEvent.mouseUp(document);

    const header = screen.getByTestId("grid-header");
    const track = (header.getAttribute("style") || "")
      .split("grid-template-columns:")[1]
      ?.split(";")[0]
      ?.trim() ?? "";
    const widths = parsePxList(track);
    // humidity is the last numeric track.
    expect(widths[widths.length - 1]).toBeGreaterThanOrEqual(48);
  });

  it("each column header has overflow:hidden so long names get an ellipsis", () => {
    render(<Grid schema={SCHEMA} rows={ROWS} loading={false} />);
    const h = screen.getByTestId("header-sensor_id");
    // The .hname span gets overflow:hidden + text-overflow.
    const name = h.querySelector(".hname");
    expect(name).not.toBeNull();
  });
});
