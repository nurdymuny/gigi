import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { FindModal } from "../../src/components/FindModal";
import type { BundleSchema } from "../../src/lib/gigi-client";

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "site", type: "categorical" },
    { name: "operator", type: "text", encryption: "opaque" },
    { name: "temp", type: "numeric" },
  ],
  indexed_fields: ["sensor_id"],
  records: 4,
  storage_mode: "mmap",
};

const ROWS = [
  { sensor_id: "S-100", site: "North", operator: "alice_42", temp: 22.5 },
  { sensor_id: "S-200", site: "South", operator: "bob_99", temp: 18.2 },
  { sensor_id: "S-300", site: "North", operator: "carol_7", temp: 30.1 },
  { sensor_id: "S-400", site: "East", operator: "alice_42", temp: 25.0 },
];

describe("FindModal", () => {
  it("renders nothing when closed", () => {
    render(
      <FindModal
        open={false}
        schema={SCHEMA}
        rows={ROWS}
        onClose={() => {}}
        onSelectRow={() => {}}
      />,
    );
    expect(screen.queryByTestId("find-modal")).toBeNull();
  });

  it("renders an autofocused input and 'type to search' hint when open with no query", () => {
    render(
      <FindModal
        open
        schema={SCHEMA}
        rows={ROWS}
        onClose={() => {}}
        onSelectRow={() => {}}
      />,
    );
    const input = screen.getByTestId("find-input") as HTMLInputElement;
    expect(input).toBeInTheDocument();
    expect(document.activeElement).toBe(input);
    expect(screen.getByTestId("find-empty-hint")).toHaveTextContent(/type/i);
  });

  it("filters rows by case-insensitive substring across any field", () => {
    render(
      <FindModal
        open
        schema={SCHEMA}
        rows={ROWS}
        onClose={() => {}}
        onSelectRow={() => {}}
      />,
    );
    fireEvent.change(screen.getByTestId("find-input"), {
      target: { value: "NORTH" },
    });
    const results = screen.getAllByTestId("find-result");
    expect(results).toHaveLength(2);
    expect(results[0]).toHaveTextContent("S-100");
    expect(results[1]).toHaveTextContent("S-300");
  });

  it("excludes OPAQUE-encrypted fields from the searchable surface", () => {
    // 'operator' is opaque; searching its plaintext should NOT match.
    render(
      <FindModal
        open
        schema={SCHEMA}
        rows={ROWS}
        onClose={() => {}}
        onSelectRow={() => {}}
      />,
    );
    fireEvent.change(screen.getByTestId("find-input"), {
      target: { value: "alice" },
    });
    expect(screen.queryAllByTestId("find-result")).toHaveLength(0);
    expect(screen.getByTestId("find-empty-results")).toBeInTheDocument();
  });

  it("shows a result count", () => {
    render(
      <FindModal
        open
        schema={SCHEMA}
        rows={ROWS}
        onClose={() => {}}
        onSelectRow={() => {}}
      />,
    );
    fireEvent.change(screen.getByTestId("find-input"), {
      target: { value: "north" },
    });
    expect(screen.getByTestId("find-count")).toHaveTextContent("2");
  });

  it("clicking a result fires onSelectRow with that row's key and closes", () => {
    const onSelectRow = vi.fn();
    const onClose = vi.fn();
    render(
      <FindModal
        open
        schema={SCHEMA}
        rows={ROWS}
        onClose={onClose}
        onSelectRow={onSelectRow}
      />,
    );
    fireEvent.change(screen.getByTestId("find-input"), {
      target: { value: "South" },
    });
    fireEvent.click(screen.getByTestId("find-result"));
    expect(onSelectRow).toHaveBeenCalledWith("S-200");
    expect(onClose).toHaveBeenCalled();
  });

  it("Enter on the input fires onSelectRow with the first result's key", () => {
    const onSelectRow = vi.fn();
    render(
      <FindModal
        open
        schema={SCHEMA}
        rows={ROWS}
        onClose={() => {}}
        onSelectRow={onSelectRow}
      />,
    );
    fireEvent.change(screen.getByTestId("find-input"), {
      target: { value: "north" },
    });
    fireEvent.keyDown(screen.getByTestId("find-input"), { key: "Enter" });
    expect(onSelectRow).toHaveBeenCalledWith("S-100");
  });

  it("Escape closes the modal", () => {
    const onClose = vi.fn();
    render(
      <FindModal
        open
        schema={SCHEMA}
        rows={ROWS}
        onClose={onClose}
        onSelectRow={() => {}}
      />,
    );
    fireEvent.keyDown(screen.getByTestId("find-input"), { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
  });

  it("caps the result list at a reasonable maximum (50)", () => {
    const big: typeof ROWS = Array.from({ length: 200 }, (_, i) => ({
      sensor_id: `S-${1000 + i}`,
      site: "North",
      operator: "x",
      temp: i,
    }));
    render(
      <FindModal
        open
        schema={SCHEMA}
        rows={big}
        onClose={() => {}}
        onSelectRow={() => {}}
      />,
    );
    fireEvent.change(screen.getByTestId("find-input"), {
      target: { value: "S-" },
    });
    expect(screen.getAllByTestId("find-result").length).toBeLessThanOrEqual(50);
    // Total count still reflects the real total.
    expect(screen.getByTestId("find-count")).toHaveTextContent("200");
  });
});
