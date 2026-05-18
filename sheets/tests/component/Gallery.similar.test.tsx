import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { Gallery } from "../../src/components/Gallery";
import type { BundleSchema } from "../../src/lib/gigi-client";

/**
 * Phase 7.C · find-similar mode.
 *
 * Right-click a card → "Find similar to this row" → Gallery sorts every
 * card by Davis sameness against the pivot (descending), pins the pivot
 * with a marker, and shows a per-card S-bar. The user can exit the mode
 * via the chip in the toolbar.
 */

const schema: BundleSchema = {
  name: "demo",
  base_fields: [{ name: "id", type: "text" }],
  fiber_fields: [{ name: "score", type: "numeric" }],
  indexed_fields: ["id"],
  records: 4,
  storage_mode: "mmap",
} as unknown as BundleSchema;

const rows = [
  { id: "R1", score: 1 },
  { id: "R2", score: 2 },
  { id: "R3", score: 3 },
  { id: "R4", score: 4 },
];

const kappaMap = new Map<string, number>([
  ["R1", 0.1],
  ["R2", 0.2],
  ["R3", 0.3],
  ["R4", 0.4],
]);

/** Stub sameness — closer scores → higher S. Pivot vs self is 1. */
function makeSameness() {
  return (a: string, b: string) => {
    if (a === b) return 1;
    const sa = Number(a.replace("R", ""));
    const sb = Number(b.replace("R", ""));
    return Math.max(0, 1 - Math.abs(sa - sb) * 0.2);
  };
}

function cardByKey(k: string): HTMLElement {
  const card = screen
    .getAllByTestId("gallery-card")
    .find((c) => c.getAttribute("data-row-key") === k);
  if (!card) throw new Error(`no card for ${k}`);
  return card;
}

describe("Gallery · find-similar mode", () => {
  it("with no similarPivot, the toolbar chip is hidden and cards aren't sameness-tagged", () => {
    render(
      <Gallery
        schema={schema}
        rows={rows}
        kappaMap={kappaMap}
        coverField="score"
        onRowSelect={() => undefined}
      />,
    );
    expect(screen.queryByTestId("gallery-similar-chip")).toBeNull();
    expect(screen.queryAllByTestId("gallery-card-sameness")).toHaveLength(0);
  });

  it("with a pivot, renders the chip and pins the pivot card at the top", () => {
    render(
      <Gallery
        schema={schema}
        rows={rows}
        kappaMap={kappaMap}
        coverField="score"
        onRowSelect={() => undefined}
        similarPivot="R2"
        sameness={makeSameness()}
      />,
    );
    const chip = screen.getByTestId("gallery-similar-chip");
    expect(chip).toHaveTextContent("R2");
    // First card is the pivot.
    const first = screen.getAllByTestId("gallery-card")[0];
    expect(first.getAttribute("data-row-key")).toBe("R2");
    expect(first.getAttribute("data-pivot")).toBe("true");
  });

  it("cards sort by sameness desc against the pivot", () => {
    render(
      <Gallery
        schema={schema}
        rows={rows}
        kappaMap={kappaMap}
        coverField="score"
        onRowSelect={() => undefined}
        similarPivot="R2"
        sameness={makeSameness()}
      />,
    );
    // sameness against R2 (per stub):
    //   R2→R2 = 1.0
    //   R2→R1 = 0.8 (Δ=1)
    //   R2→R3 = 0.8 (Δ=1)
    //   R2→R4 = 0.6 (Δ=2)
    // Expected order: R2, then R1 / R3 (tie), then R4.
    const order = screen
      .getAllByTestId("gallery-card")
      .map((c) => c.getAttribute("data-row-key"));
    expect(order[0]).toBe("R2");
    expect(order[3]).toBe("R4");
    expect(order.slice(1, 3).sort()).toEqual(["R1", "R3"]);
  });

  it("renders a per-card sameness bar with the numeric score", () => {
    render(
      <Gallery
        schema={schema}
        rows={rows}
        kappaMap={kappaMap}
        coverField="score"
        onRowSelect={() => undefined}
        similarPivot="R2"
        sameness={makeSameness()}
      />,
    );
    // The pivot card shows S=1.000.
    const pivot = cardByKey("R2");
    const pivotBar = pivot.querySelector('[data-testid="gallery-card-sameness"]');
    expect(pivotBar?.getAttribute("data-sameness")).toBe("1.0000");
    // A neighbor shows the stub's computed value.
    const r4 = cardByKey("R4");
    const r4Bar = r4.querySelector('[data-testid="gallery-card-sameness"]');
    expect(r4Bar?.getAttribute("data-sameness")).toBe("0.6000");
  });

  it("disables sort + dir controls while in similar mode", () => {
    render(
      <Gallery
        schema={schema}
        rows={rows}
        kappaMap={kappaMap}
        coverField="score"
        onRowSelect={() => undefined}
        similarPivot="R2"
        sameness={makeSameness()}
      />,
    );
    expect(screen.getByTestId("gallery-sort-field")).toBeDisabled();
    expect(screen.getByTestId("gallery-sort-dir")).toBeDisabled();
  });

  it("clicking the chip's ✕ fires onClearSimilar", () => {
    const onClearSimilar = vi.fn();
    render(
      <Gallery
        schema={schema}
        rows={rows}
        kappaMap={kappaMap}
        coverField="score"
        onRowSelect={() => undefined}
        similarPivot="R2"
        sameness={makeSameness()}
        onClearSimilar={onClearSimilar}
      />,
    );
    fireEvent.click(screen.getByTestId("gallery-similar-clear"));
    expect(onClearSimilar).toHaveBeenCalled();
  });
});
