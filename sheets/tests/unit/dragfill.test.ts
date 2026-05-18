import { describe, expect, it } from "vitest";
import {
  dragFillCategorical,
  dragFillDate,
  dragFillNumeric,
  ols,
} from "../../src/lib/dragfill";

describe("dragfill · ols", () => {
  it("returns slope 0 + intercept = mean for a constant series", () => {
    const { slope, intercept } = ols([5, 5, 5, 5]);
    expect(slope).toBeCloseTo(0, 6);
    expect(intercept).toBeCloseTo(5, 6);
  });

  it("returns slope 1 for [0,1,2,3]", () => {
    const { slope, intercept } = ols([0, 1, 2, 3]);
    expect(slope).toBeCloseTo(1, 6);
    expect(intercept).toBeCloseTo(0, 6);
  });

  it("fits a noisy line tightly (not last-pair)", () => {
    // [1, 1.9, 3.1, 4] — true slope ≈ 1.04. Naive last-pair would give 0.9.
    const { slope } = ols([1, 1.9, 3.1, 4]);
    expect(slope).toBeGreaterThan(0.95);
    expect(slope).toBeLessThan(1.10);
  });

  it("returns slope=0 for fewer than 2 points", () => {
    expect(ols([]).slope).toBe(0);
    expect(ols([7]).slope).toBe(0);
  });
});

describe("dragfill · dragFillNumeric", () => {
  it("extrapolates a clean linear sequence", () => {
    const out = dragFillNumeric([1, 2, 3], 3);
    expect(out).toEqual([4, 5, 6]);
  });

  it("extrapolates a noisy trend via OLS, not last-pair", () => {
    // Trend ≈ 1, last pair would say "3.9 + 0.0 = 3.9" — OLS goes higher.
    const out = dragFillNumeric([1, 1.9, 3.1, 4], 2);
    // We expect roughly 5.04 and 6.08, but tolerate OLS noise.
    expect(out[0]).toBeGreaterThan(4.5);
    expect(out[1]).toBeGreaterThan(5.5);
  });

  it("returns an empty array for empty seed", () => {
    expect(dragFillNumeric([], 5)).toEqual([]);
  });

  it("zero-count is a no-op", () => {
    expect(dragFillNumeric([1, 2, 3], 0)).toEqual([]);
  });
});

describe("dragfill · dragFillDate", () => {
  it("steps by day when seed is daily", () => {
    const out = dragFillDate(["2026-05-01", "2026-05-02"], 3);
    expect(out).toEqual(["2026-05-03", "2026-05-04", "2026-05-05"]);
  });

  it("steps across month boundary", () => {
    const out = dragFillDate(["2026-04-29", "2026-04-30"], 3);
    expect(out).toEqual(["2026-05-01", "2026-05-02", "2026-05-03"]);
  });

  it("infers a 7-day step for weekly seeds", () => {
    const out = dragFillDate(["2026-05-01", "2026-05-08"], 2);
    expect(out).toEqual(["2026-05-15", "2026-05-22"]);
  });

  it("empty seed returns []", () => {
    expect(dragFillDate([], 3)).toEqual([]);
  });
});

describe("dragfill · dragFillCategorical", () => {
  it("returns the most-common seed value", () => {
    expect(dragFillCategorical(["A", "A", "B"], 3)).toEqual(["A", "A", "A"]);
  });

  it("handles a single-value seed", () => {
    expect(dragFillCategorical(["X"], 4)).toEqual(["X", "X", "X", "X"]);
  });

  it("empty seed returns []", () => {
    expect(dragFillCategorical([], 5)).toEqual([]);
  });
});
