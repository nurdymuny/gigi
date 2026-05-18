import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { Grid } from "../../src/components/Grid";
import {
  SheetsClientError,
  type BundleSchema,
} from "../../src/lib/gigi-client";

const schema: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "temp", type: "numeric" },
    { name: "humidity", type: "numeric" },
    { name: "operator", type: "text", encryption: "indexed" },
  ],
  indexed_fields: ["sensor_id"],
  records: 2,
  storage_mode: "mmap",
};

describe("Grid — boot states", () => {
  it("renders a skeleton while loading", () => {
    render(<Grid schema={null} rows={[]} loading={true} />);
    expect(screen.getByTestId("grid-skeleton")).toBeInTheDocument();
    expect(screen.getByTestId("grid-skeleton")).toHaveAttribute("aria-busy", "true");
  });

  it("renders the error state with code + status when error is present", () => {
    const error = new SheetsClientError("Bundle missing", "http_error", 404);
    render(<Grid schema={null} rows={[]} loading={false} error={error} />);
    const alert = screen.getByRole("alert");
    expect(alert).toHaveTextContent(/Bundle missing/);
    expect(alert).toHaveTextContent(/http_error/);
    expect(alert).toHaveTextContent(/404/);
  });

  it("renders header cells in schema order (base, then fiber)", () => {
    render(<Grid schema={schema} rows={[]} loading={false} />);
    const headers = Array.from(
      document.querySelectorAll('[data-testid^="header-"]'),
    )
      .map((h) => (h as HTMLElement).dataset.testid)
      // Ignore the gutter headers (row-number, κ) — those aren't part of
      // the field column list and live at fixed positions on the left.
      .filter((id) => id !== "header-row-number" && id !== "header-kappa");
    expect(headers).toEqual([
      "header-sensor_id",
      "header-temp",
      "header-humidity",
      "header-operator",
    ]);
  });

  it("renders one grid row per response item", () => {
    const rows = [
      { sensor_id: "S-001", temp: 22.5, humidity: 60.1, operator: "opr_d2a8" },
      { sensor_id: "S-002", temp: 19.3, humidity: 71.4, operator: "opr_77bc" },
    ];
    render(<Grid schema={schema} rows={rows} loading={false} />);
    expect(screen.getAllByTestId("grid-row")).toHaveLength(2);
  });

  it("uses the first base_field as the row key (stable identity)", () => {
    const rows = [
      { sensor_id: "S-001", temp: 22.5, humidity: 60.1, operator: "x" },
    ];
    render(<Grid schema={schema} rows={rows} loading={false} />);
    const row = screen.getByTestId("grid-row");
    expect(row).toHaveAttribute("data-row-key", "S-001");
  });

  it("renders the empty state when rows is [] but schema is present", () => {
    render(<Grid schema={schema} rows={[]} loading={false} />);
    expect(screen.getByTestId("grid-empty")).toBeInTheDocument();
  });

  it("renders encrypted fields with a lock affordance + INDEXED mode shows the value", () => {
    const rows = [
      { sensor_id: "S-001", temp: 22.5, humidity: 60.1, operator: "opr_secret" },
    ];
    render(<Grid schema={schema} rows={rows} loading={false} />);
    const row = screen.getByTestId("grid-row");
    const encCell = row.querySelector(".grid-cell-enc");
    expect(encCell).not.toBeNull();
    expect(encCell?.querySelector("svg")).not.toBeNull();
    // INDEXED encryption surfaces the ciphertext (so equality reads stay readable).
    expect(encCell).toHaveAttribute("data-encryption", "indexed");
    expect(encCell).toHaveTextContent("opr_secret");
  });

  it("renders an OPAQUE field with no plaintext in textContent", () => {
    const opaqueSchema: BundleSchema = {
      ...schema,
      fiber_fields: [
        ...schema.fiber_fields.slice(0, 2),
        { name: "operator", type: "text", encryption: "opaque" },
      ],
    };
    const rows = [
      { sensor_id: "S-001", temp: 22.5, humidity: 60.1, operator: "this should be hidden" },
    ];
    render(<Grid schema={opaqueSchema} rows={rows} loading={false} />);
    const opaqueCell = screen.getByTestId("encrypted-cell");
    expect(opaqueCell).toHaveAttribute("data-encryption", "opaque");
    // No plaintext leak — the only visible text is the block-char placeholder.
    expect(opaqueCell.textContent).not.toContain("this should be hidden");
    expect(opaqueCell.textContent).toMatch(/[▒]+/);
  });
});
