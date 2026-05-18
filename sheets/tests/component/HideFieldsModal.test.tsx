import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { HideFieldsModal } from "../../src/components/HideFieldsModal";
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
  records: 4,
  storage_mode: "mmap",
};

describe("HideFieldsModal", () => {
  it("renders nothing when closed", () => {
    render(
      <HideFieldsModal
        open={false}
        schema={SCHEMA}
        hiddenFields={new Set()}
        onClose={() => {}}
        onChange={() => {}}
      />,
    );
    expect(screen.queryByTestId("hide-fields-modal")).toBeNull();
  });

  it("lists every field with a checkbox; primary key is disabled", () => {
    render(
      <HideFieldsModal
        open
        schema={SCHEMA}
        hiddenFields={new Set()}
        onClose={() => {}}
        onChange={() => {}}
      />,
    );
    expect(screen.getByTestId("hide-fields-check-sensor_id")).toBeDisabled();
    expect(screen.getByTestId("hide-fields-check-temp")).not.toBeDisabled();
    expect(screen.getByTestId("hide-fields-check-humidity")).not.toBeDisabled();
  });

  it("Apply emits the hidden set; cancel does not call onChange", () => {
    const onChange = vi.fn();
    render(
      <HideFieldsModal
        open
        schema={SCHEMA}
        hiddenFields={new Set()}
        onClose={() => {}}
        onChange={onChange}
      />,
    );
    fireEvent.click(screen.getByTestId("hide-fields-check-temp"));
    fireEvent.click(screen.getByTestId("hide-fields-apply"));
    expect(onChange).toHaveBeenCalledOnce();
    const hidden: Set<string> = onChange.mock.calls[0][0];
    expect(hidden.has("temp")).toBe(true);
    expect(hidden.has("humidity")).toBe(false);
  });

  it("Show all clears the local set", () => {
    const onChange = vi.fn();
    render(
      <HideFieldsModal
        open
        schema={SCHEMA}
        hiddenFields={new Set(["temp", "humidity"])}
        onClose={() => {}}
        onChange={onChange}
      />,
    );
    fireEvent.click(screen.getByTestId("hide-fields-show-all"));
    fireEvent.click(screen.getByTestId("hide-fields-apply"));
    expect(onChange.mock.calls[0][0].size).toBe(0);
  });

  it("Hide-everything-but-the-key marks all fiber fields hidden", () => {
    const onChange = vi.fn();
    render(
      <HideFieldsModal
        open
        schema={SCHEMA}
        hiddenFields={new Set()}
        onClose={() => {}}
        onChange={onChange}
      />,
    );
    fireEvent.click(screen.getByTestId("hide-fields-hide-non-key"));
    fireEvent.click(screen.getByTestId("hide-fields-apply"));
    const hidden: Set<string> = onChange.mock.calls[0][0];
    expect(hidden.has("temp")).toBe(true);
    expect(hidden.has("humidity")).toBe(true);
    expect(hidden.has("site_id")).toBe(true);
    expect(hidden.has("sensor_id")).toBe(false);
  });
});
