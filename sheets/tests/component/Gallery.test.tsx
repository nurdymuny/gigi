import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { Gallery } from "../../src/components/Gallery";
import type { BundleSchema } from "../../src/lib/gigi-client";

const schema: BundleSchema = {
  name: "demo",
  base_fields: [{ name: "id", type: "text" }],
  fiber_fields: [
    { name: "category", type: "categorical" },
    { name: "score", type: "numeric" },
    { name: "status", type: "categorical" },
  ],
  indexed_fields: ["id"],
  records: 3,
  storage_mode: "mmap",
} as unknown as BundleSchema;

const rows = [
  { id: "R1", category: "alpha", score: 8.5, status: "active" },
  { id: "R2", category: "beta", score: 4.2, status: "active" },
  { id: "R3", category: "alpha", score: 9.1, status: "review" },
];

// kappa.ts thresholds default to {warn: 0.8, bad: 2.0}.
const kappaMap = new Map<string, number>([
  ["R1", 0.05], // healthy → kappa-ok
  ["R2", 3.0],  // bad → kappa-bad
  ["R3", 1.2],  // warn → kappa-warn
]);

// Helper: find a card by its row key. Gallery now sorts by κ desc by
// default, so addressing cards via array index is brittle — go through
// the data-row-key attribute instead.
function cardByKey(k: string): HTMLElement {
  const card = screen
    .getAllByTestId("gallery-card")
    .find((c) => c.getAttribute("data-row-key") === k);
  if (!card) throw new Error(`no card for ${k}`);
  return card;
}

describe("Gallery", () => {
  it("renders one card per row", () => {
    render(
      <Gallery
        schema={schema}
        rows={rows}
        kappaMap={kappaMap}
        coverField="category"
        onRowSelect={() => undefined}
      />,
    );
    const cards = screen.getAllByTestId("gallery-card");
    expect(cards).toHaveLength(3);
  });

  it("shows the row key and cover value on each card", () => {
    render(
      <Gallery
        schema={schema}
        rows={rows}
        kappaMap={kappaMap}
        coverField="category"
        onRowSelect={() => undefined}
      />,
    );
    expect(screen.getByText("R1")).toBeInTheDocument();
    expect(screen.getByText("R2")).toBeInTheDocument();
    expect(screen.getByText("R3")).toBeInTheDocument();
    // Cover values render as chip text. "alpha" appears twice (R1 and R3).
    expect(screen.getAllByText("alpha")).toHaveLength(2);
  });

  it("tints cards by κ-class", () => {
    render(
      <Gallery
        schema={schema}
        rows={rows}
        kappaMap={kappaMap}
        coverField="category"
        onRowSelect={() => undefined}
      />,
    );
    expect(cardByKey("R1").className).toMatch(/kappa-ok/);
    expect(cardByKey("R2").className).toMatch(/kappa-bad/);
    expect(cardByKey("R3").className).toMatch(/kappa-warn/);
  });

  it("calls onRowSelect with the key when a card is clicked", () => {
    const onRowSelect = vi.fn();
    render(
      <Gallery
        schema={schema}
        rows={rows}
        kappaMap={kappaMap}
        coverField="category"
        onRowSelect={onRowSelect}
      />,
    );
    fireEvent.click(cardByKey("R2"));
    expect(onRowSelect).toHaveBeenCalledWith("R2");
  });

  it("highlights the selected card", () => {
    render(
      <Gallery
        schema={schema}
        rows={rows}
        kappaMap={kappaMap}
        coverField="category"
        selectedRowKey="R2"
        onRowSelect={() => undefined}
      />,
    );
    expect(cardByKey("R2").className).toMatch(/gallery-card-selected/);
    expect(cardByKey("R1").className).not.toMatch(/gallery-card-selected/);
  });

  it("renders an empty state when there are no rows", () => {
    render(
      <Gallery
        schema={schema}
        rows={[]}
        kappaMap={new Map()}
        coverField="category"
        onRowSelect={() => undefined}
      />,
    );
    expect(screen.getByTestId("gallery-empty")).toBeInTheDocument();
  });
});
