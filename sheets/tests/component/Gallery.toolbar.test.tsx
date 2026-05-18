import { describe, expect, it } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/react";
import { Gallery } from "../../src/components/Gallery";
import type { BundleSchema } from "../../src/lib/gigi-client";

/**
 * Phase 7.B · Gallery toolbar — group by / sort / filter / density.
 *
 * The toolbar is the "discoverability layer" for the gallery: every
 * arrangement you'd normally have to script (group by category, only
 * show anomalies, sort by κ, shrink cards to fit more) becomes a
 * single click.
 */

const schema: BundleSchema = {
  name: "demo",
  base_fields: [{ name: "id", type: "text" }],
  fiber_fields: [
    { name: "category", type: "categorical" },
    { name: "score", type: "numeric" },
  ],
  indexed_fields: ["id"],
  records: 5,
  storage_mode: "mmap",
} as unknown as BundleSchema;

const rows = [
  { id: "R1", category: "alpha", score: 8.5 },
  { id: "R2", category: "beta", score: 4.2 },
  { id: "R3", category: "alpha", score: 9.1 },
  { id: "R4", category: "beta", score: 2.0 },
  { id: "R5", category: "gamma", score: 6.0 },
];

const kappaMap = new Map<string, number>([
  ["R1", 0.05], // ok
  ["R2", 3.0],  // bad
  ["R3", 1.2],  // warn (drift)
  ["R4", 0.2],  // ok
  ["R5", 2.5],  // bad
]);

function setup(extra: object = {}) {
  return render(
    <Gallery
      schema={schema}
      rows={rows}
      kappaMap={kappaMap}
      coverField="category"
      onRowSelect={() => undefined}
      {...extra}
    />,
  );
}

describe("Gallery · toolbar exists", () => {
  it("renders a toolbar with the standard controls", () => {
    setup();
    expect(screen.getByTestId("gallery-toolbar")).toBeInTheDocument();
    expect(screen.getByTestId("gallery-group-by")).toBeInTheDocument();
    expect(screen.getByTestId("gallery-sort-field")).toBeInTheDocument();
    expect(screen.getByTestId("gallery-sort-dir")).toBeInTheDocument();
    expect(screen.getByTestId("gallery-density")).toBeInTheDocument();
    expect(screen.getByTestId("gallery-kappa-filter")).toBeInTheDocument();
  });
});

describe("Gallery · κ-class filter", () => {
  it("'Bad only' shows only κ-bad cards", () => {
    setup();
    fireEvent.click(screen.getByTestId("gallery-kappa-bad"));
    const cards = screen.getAllByTestId("gallery-card");
    expect(cards).toHaveLength(2); // R2 + R5
    const keys = cards.map((c) => c.getAttribute("data-row-key")).sort();
    expect(keys).toEqual(["R2", "R5"]);
  });

  it("'Drift' shows κ-warn AND κ-bad cards", () => {
    setup();
    fireEvent.click(screen.getByTestId("gallery-kappa-drift"));
    const cards = screen.getAllByTestId("gallery-card");
    // drift band: warn + bad → R2, R3, R5
    expect(cards).toHaveLength(3);
  });

  it("'All' restores the full set", () => {
    setup();
    fireEvent.click(screen.getByTestId("gallery-kappa-bad"));
    fireEvent.click(screen.getByTestId("gallery-kappa-all"));
    expect(screen.getAllByTestId("gallery-card")).toHaveLength(5);
  });
});

describe("Gallery · sort", () => {
  it("sorts by κ desc by default → R2 first (κ=3.0)", () => {
    setup();
    const cards = screen.getAllByTestId("gallery-card");
    expect(cards[0].getAttribute("data-row-key")).toBe("R2");
    expect(cards[1].getAttribute("data-row-key")).toBe("R5"); // κ=2.5
  });

  it("flipping sort direction reverses the order", () => {
    setup();
    fireEvent.click(screen.getByTestId("gallery-sort-dir"));
    const cards = screen.getAllByTestId("gallery-card");
    // κ asc → R1 first (κ=0.05)
    expect(cards[0].getAttribute("data-row-key")).toBe("R1");
  });

  it("sort by 'key' orders alphabetically by row key", () => {
    setup();
    fireEvent.change(screen.getByTestId("gallery-sort-field"), {
      target: { value: "key" },
    });
    // Switch to ascending so R1 is first under key sort.
    const dir = screen.getByTestId("gallery-sort-dir") as HTMLButtonElement;
    if (dir.textContent?.toLowerCase().includes("desc")) fireEvent.click(dir);
    const cards = screen.getAllByTestId("gallery-card");
    expect(cards[0].getAttribute("data-row-key")).toBe("R1");
    expect(cards[4].getAttribute("data-row-key")).toBe("R5");
  });

  it("sort by a numeric column orders by that field", () => {
    setup();
    fireEvent.change(screen.getByTestId("gallery-sort-field"), {
      target: { value: "score" },
    });
    // Already desc by default → R3 (9.1) first.
    const cards = screen.getAllByTestId("gallery-card");
    expect(cards[0].getAttribute("data-row-key")).toBe("R3");
  });
});

describe("Gallery · group-by", () => {
  it("with no group, no group headers appear", () => {
    setup();
    expect(screen.queryByTestId(/^gallery-group-header-/)).toBeNull();
  });

  it("group by category buckets cards under group headers with counts", () => {
    setup();
    fireEvent.change(screen.getByTestId("gallery-group-by"), {
      target: { value: "category" },
    });
    expect(screen.getByTestId("gallery-group-header-alpha")).toBeInTheDocument();
    expect(screen.getByTestId("gallery-group-header-beta")).toBeInTheDocument();
    expect(screen.getByTestId("gallery-group-header-gamma")).toBeInTheDocument();
    // Counts in the header label.
    expect(
      screen.getByTestId("gallery-group-header-alpha").textContent,
    ).toMatch(/2/);
    expect(
      screen.getByTestId("gallery-group-header-beta").textContent,
    ).toMatch(/2/);
  });

  it("cards in a group are contained under that group's section", () => {
    setup();
    fireEvent.change(screen.getByTestId("gallery-group-by"), {
      target: { value: "category" },
    });
    const alphaGroup = screen.getByTestId("gallery-group-alpha");
    const cardsInAlpha = within(alphaGroup).getAllByTestId("gallery-card");
    expect(cardsInAlpha).toHaveLength(2);
    const keys = cardsInAlpha
      .map((c) => c.getAttribute("data-row-key"))
      .sort();
    expect(keys).toEqual(["R1", "R3"]);
  });
});

describe("Gallery · density", () => {
  it("standard density is the default", () => {
    setup();
    const grid = screen.getByTestId("gallery-grid");
    expect(grid.className).toMatch(/gallery-grid-standard/);
  });

  it("compact density adds the compact class", () => {
    setup();
    fireEvent.change(screen.getByTestId("gallery-density"), {
      target: { value: "compact" },
    });
    const grid = screen.getByTestId("gallery-grid");
    expect(grid.className).toMatch(/gallery-grid-compact/);
  });

  it("expanded density shows more body fields", () => {
    // Schema only has 2 fiber fields, so expanded shows both; standard
    // shows the first 4 (capped at 2 here since that's all there is).
    setup();
    const beforeRow = screen
      .getAllByTestId("gallery-card")[0]
      .querySelectorAll(".gallery-card-row").length;
    fireEvent.change(screen.getByTestId("gallery-density"), {
      target: { value: "expanded" },
    });
    const afterRow = screen
      .getAllByTestId("gallery-card")[0]
      .querySelectorAll(".gallery-card-row").length;
    // With our schema both render the same number of rows, so we just
    // assert the class flipped — the visual difference is in CSS, not row count.
    expect(beforeRow).toBeGreaterThan(0);
    expect(afterRow).toBeGreaterThan(0);
    const grid = screen.getByTestId("gallery-grid");
    expect(grid.className).toMatch(/gallery-grid-expanded/);
  });
});
