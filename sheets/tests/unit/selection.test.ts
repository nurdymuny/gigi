import { describe, expect, it } from "vitest";
import {
  cellRange,
  emptySelection,
  extendByKappaNeighborhood,
  isCellSelected,
  isRowSelected,
  normalizeRect,
  selectionStats,
  toggleRow,
} from "../../src/lib/selection";

describe("selection · normalizeRect", () => {
  it("orients top-left to bottom-right regardless of input direction", () => {
    expect(normalizeRect({ r1: 5, c1: 4, r2: 2, c2: 1 })).toEqual({
      r1: 2,
      c1: 1,
      r2: 5,
      c2: 4,
    });
    expect(normalizeRect({ r1: 0, c1: 0, r2: 0, c2: 0 })).toEqual({
      r1: 0,
      c1: 0,
      r2: 0,
      c2: 0,
    });
  });
});

describe("selection · cellRange", () => {
  it("returns (r2-r1+1) × (c2-c1+1) cells", () => {
    const cells = cellRange({ r1: 0, c1: 0, r2: 2, c2: 1 });
    expect(cells).toHaveLength(6);
  });

  it("handles a single cell as a 1-element list", () => {
    expect(cellRange({ r1: 3, c1: 4, r2: 3, c2: 4 })).toEqual([{ row: 3, col: 4 }]);
  });

  it("normalizes inverted input before expansion", () => {
    expect(cellRange({ r1: 2, c1: 2, r2: 0, c2: 0 })).toHaveLength(9);
  });
});

describe("selection · isCellSelected / isRowSelected", () => {
  it("returns true for cells inside the rect", () => {
    const sel = { rect: { r1: 1, c1: 1, r2: 3, c2: 3 }, rowKeys: new Set<string>() };
    expect(isCellSelected(sel, 2, 2)).toBe(true);
    expect(isCellSelected(sel, 1, 1)).toBe(true);
    expect(isCellSelected(sel, 3, 3)).toBe(true);
  });

  it("returns false for cells outside the rect", () => {
    const sel = { rect: { r1: 1, c1: 1, r2: 3, c2: 3 }, rowKeys: new Set<string>() };
    expect(isCellSelected(sel, 0, 2)).toBe(false);
    expect(isCellSelected(sel, 4, 4)).toBe(false);
  });

  it("returns false for any cell when rect is null", () => {
    expect(isCellSelected({ rect: null, rowKeys: new Set() }, 0, 0)).toBe(false);
  });

  it("isRowSelected checks the rowKeys set", () => {
    const sel = { rect: null, rowKeys: new Set(["row-1", "row-2"]) };
    expect(isRowSelected(sel, "row-1")).toBe(true);
    expect(isRowSelected(sel, "row-3")).toBe(false);
  });
});

describe("selection · toggleRow", () => {
  it("adds a key not present", () => {
    const sel = toggleRow(emptySelection(), "row-1");
    expect(sel.rowKeys.has("row-1")).toBe(true);
  });

  it("removes a key already present", () => {
    let sel = toggleRow(emptySelection(), "row-1");
    sel = toggleRow(sel, "row-1");
    expect(sel.rowKeys.has("row-1")).toBe(false);
  });

  it("does not mutate the input selection", () => {
    const before = emptySelection();
    const after = toggleRow(before, "row-1");
    expect(before.rowKeys.size).toBe(0);
    expect(after).not.toBe(before);
  });
});

describe("selection · extendByKappaNeighborhood", () => {
  function v(...xs: number[]): Float32Array {
    return new Float32Array(xs);
  }
  function normalize(a: Float32Array): Float32Array {
    let n = 0;
    for (let i = 0; i < a.length; i++) n += a[i] * a[i];
    n = Math.sqrt(n);
    if (n > 0) for (let i = 0; i < a.length; i++) a[i] /= n;
    return a;
  }

  it("includes rows whose sameness to centroid is ≥ τ", () => {
    // Set up: three "close" rows and two "far" rows.
    const close1 = normalize(v(1, 0.05, 0));
    const close2 = normalize(v(1, 0.10, 0));
    const close3 = normalize(v(1, 0.08, 0));
    const far1 = normalize(v(0, 1, 0));
    const far2 = normalize(v(0, 0, 1));
    const embeddings = new Map<string, Float32Array>([
      ["r1", close1],
      ["r2", close2],
      ["r3", close3],
      ["r4", far1],
      ["r5", far2],
    ]);
    // Start by selecting r1 and r2. Extend should pull in r3 (also close)
    // but not r4 or r5.
    const seed = new Set(["r1", "r2"]);
    const extended = extendByKappaNeighborhood(seed, embeddings, 0.85);
    expect(extended.has("r3")).toBe(true);
    expect(extended.has("r4")).toBe(false);
    expect(extended.has("r5")).toBe(false);
    // Seed rows must remain in the extension.
    expect(extended.has("r1")).toBe(true);
    expect(extended.has("r2")).toBe(true);
  });

  it("returns the seed unchanged if no other row clears the threshold", () => {
    const a = normalize(v(1, 0));
    const b = normalize(v(0, 1));
    const c = normalize(v(-1, 0));
    const seed = new Set(["a"]);
    const extended = extendByKappaNeighborhood(
      seed,
      new Map([["a", a], ["b", b], ["c", c]]),
      0.99, // ultra-strict
    );
    expect(extended.size).toBe(1);
    expect(extended.has("a")).toBe(true);
  });

  it("ignores rows with missing embeddings instead of throwing", () => {
    const a = normalize(v(1, 0));
    const seed = new Set(["a", "ghost"]);
    expect(() =>
      extendByKappaNeighborhood(seed, new Map([["a", a]]), 0.5),
    ).not.toThrow();
  });
});

describe("selection · selectionStats", () => {
  it("counts, sums, and averages numeric cells", () => {
    const stats = selectionStats([1, 2, 3, 4]);
    expect(stats.count).toBe(4);
    expect(stats.sum).toBeCloseTo(10);
    expect(stats.mean).toBeCloseTo(2.5);
  });

  it("returns null mean for empty selection", () => {
    const stats = selectionStats([]);
    expect(stats.count).toBe(0);
    expect(stats.sum).toBe(0);
    expect(stats.mean).toBeNull();
  });

  it("skips non-finite values without crashing", () => {
    const stats = selectionStats([1, NaN, 2, Infinity, 3]);
    expect(stats.count).toBe(3);
    expect(stats.sum).toBeCloseTo(6);
    expect(stats.mean).toBeCloseTo(2);
  });
});
