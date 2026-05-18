import { describe, expect, it } from "vitest";
import { evaluate, type FormulaContext } from "../../src/lib/formula";

/**
 * Phase 1.A · new functions: Excel-parity ROUND, MOD, ABS, COUNTA,
 * CONCAT, plus the stats functions (MEDIAN, STDEV, STDEVP, VAR, VARP,
 * PERCENTILE, QUARTILE).
 *
 * See FORMULAS_SPEC.md §"Required" and §"Stats" tables for the contract.
 */

function makeCtx(): FormulaContext {
  const cells: Record<string, number | string | null> = {
    A1: 1, A2: 2, A3: 3, A4: 4, A5: 5,
    B1: 10, B2: 20, B3: 30,
    C1: "alpha", C2: "beta", C3: "gamma",
    D1: 1.5, D2: 2.5, D3: -0.5, D4: -1.5, D5: 0.5,
    E1: null, E2: 5, E3: null, E4: 7,
  };
  return {
    cell: (ref) => (ref in cells ? cells[ref] : null),
    sameness: () => 0.5,
    kappa: () => 0,
    cohort: () => "",
  };
}

describe("formula · ROUND (Excel parity: round-half-away-from-zero)", () => {
  it("rounds 0.5 to 1 (not 0 like Math.round)", () => {
    expect(evaluate("=ROUND(0.5)", makeCtx()).value).toBe(1);
  });
  it("rounds -0.5 to -1 (not 0)", () => {
    expect(evaluate("=ROUND(-0.5)", makeCtx()).value).toBe(-1);
  });
  it("rounds 2.5 to 3 (not 2 like banker's rounding)", () => {
    expect(evaluate("=ROUND(2.5)", makeCtx()).value).toBe(3);
  });
  it("rounds -2.5 to -3", () => {
    expect(evaluate("=ROUND(-2.5)", makeCtx()).value).toBe(-3);
  });
  it("accepts a digits argument", () => {
    expect(evaluate("=ROUND(3.14159, 2)", makeCtx()).value).toBe(3.14);
    expect(evaluate("=ROUND(3.14159, 0)", makeCtx()).value).toBe(3);
  });
  it("rounds correctly with negative digits (tens, hundreds)", () => {
    expect(evaluate("=ROUND(123.45, -1)", makeCtx()).value).toBe(120);
    expect(evaluate("=ROUND(155, -1)", makeCtx()).value).toBe(160);
  });
  it("propagates a cell ref's value", () => {
    expect(evaluate("=ROUND(D2)", makeCtx()).value).toBe(3); // 2.5 → 3
    expect(evaluate("=ROUND(D3)", makeCtx()).value).toBe(-1); // -0.5 → -1
  });
});

describe("formula · MOD", () => {
  it("=MOD(7, 3) returns 1", () => {
    expect(evaluate("=MOD(7, 3)", makeCtx()).value).toBe(1);
  });
  it("=MOD(10, 4) returns 2", () => {
    expect(evaluate("=MOD(10, 4)", makeCtx()).value).toBe(2);
  });
  it("=MOD(a, 0) returns #DIV0!", () => {
    expect(evaluate("=MOD(5, 0)", makeCtx()).error).toBe("#DIV0!");
  });
  it("handles negative dividend (Excel: result has sign of divisor)", () => {
    // Excel MOD(-7, 3) = 2 (sign of divisor)
    expect(evaluate("=MOD(-7, 3)", makeCtx()).value).toBe(2);
  });
});

describe("formula · ABS", () => {
  it("=ABS(-5) returns 5", () => {
    expect(evaluate("=ABS(-5)", makeCtx()).value).toBe(5);
  });
  it("=ABS(5) returns 5", () => {
    expect(evaluate("=ABS(5)", makeCtx()).value).toBe(5);
  });
  it("=ABS(0) returns 0", () => {
    expect(evaluate("=ABS(0)", makeCtx()).value).toBe(0);
  });
  it("works with cell refs", () => {
    expect(evaluate("=ABS(D4)", makeCtx()).value).toBe(1.5); // -1.5 → 1.5
  });
});

describe("formula · COUNTA (counts non-empty cells)", () => {
  it("counts numerics + strings + booleans, skips empty", () => {
    // E1=null, E2=5, E3=null, E4=7 — 2 non-empty
    expect(evaluate("=COUNTA(E1, E2, E3, E4)", makeCtx()).value).toBe(2);
  });
  it("counts strings (which COUNT doesn't)", () => {
    expect(evaluate("=COUNTA(C1, C2, C3)", makeCtx()).value).toBe(3);
    expect(evaluate("=COUNT(C1, C2, C3)", makeCtx()).value).toBe(0);
  });
  it("counts a range", () => {
    expect(evaluate("=COUNTA(A1:A5)", makeCtx()).value).toBe(5);
  });
});

describe("formula · CONCAT (function form of &)", () => {
  it("concatenates two strings", () => {
    expect(evaluate('=CONCAT("hello", " world")', makeCtx()).value).toBe(
      "hello world",
    );
  });
  it("concatenates strings and numbers", () => {
    expect(evaluate('=CONCAT("x = ", 42)', makeCtx()).value).toBe("x = 42");
  });
  it("concatenates n args", () => {
    expect(evaluate('=CONCAT("a", "b", "c", "d")', makeCtx()).value).toBe(
      "abcd",
    );
  });
  it("treats null as empty", () => {
    expect(evaluate('=CONCAT("x", E1, "y")', makeCtx()).value).toBe("xy");
  });
});

describe("formula · MEDIAN", () => {
  it("returns the middle of an odd-length series", () => {
    expect(evaluate("=MEDIAN(1, 3, 5)", makeCtx()).value).toBe(3);
  });
  it("returns the mean of the two middles for even N", () => {
    expect(evaluate("=MEDIAN(1, 2, 3, 4)", makeCtx()).value).toBe(2.5);
  });
  it("works over a range", () => {
    expect(evaluate("=MEDIAN(A1:A5)", makeCtx()).value).toBe(3);
  });
  it("ignores non-numeric values", () => {
    expect(evaluate("=MEDIAN(1, 2, C1, 3)", makeCtx()).value).toBe(2);
  });
  it("returns #DIV0! on empty / all-non-numeric input", () => {
    expect(evaluate("=MEDIAN(C1, C2)", makeCtx()).error).toBe("#DIV0!");
  });
});

describe("formula · STDEV (sample) and STDEVP (population)", () => {
  it("=STDEV(1,2,3,4,5) ≈ 1.5811 (sample, n-1 denom)", () => {
    const v = evaluate("=STDEV(1, 2, 3, 4, 5)", makeCtx()).value as number;
    expect(v).toBeCloseTo(1.5811, 3);
  });
  it("=STDEVP(1,2,3,4,5) ≈ 1.4142 (population, n denom)", () => {
    const v = evaluate("=STDEVP(1, 2, 3, 4, 5)", makeCtx()).value as number;
    expect(v).toBeCloseTo(1.4142, 3);
  });
  it("=STDEV(single_value) returns #DIV0! (n < 2)", () => {
    expect(evaluate("=STDEV(5)", makeCtx()).error).toBe("#DIV0!");
  });
  it("=STDEVP(single_value) returns 0 (n=1 valid, deviation is 0)", () => {
    expect(evaluate("=STDEVP(5)", makeCtx()).value).toBe(0);
  });
  it("ignores non-numeric values", () => {
    const v = evaluate("=STDEV(1, 2, C1, 3, 4, 5)", makeCtx()).value as number;
    expect(v).toBeCloseTo(1.5811, 3);
  });
  it("returns 0 for all-equal samples", () => {
    expect(evaluate("=STDEV(3, 3, 3, 3)", makeCtx()).value).toBe(0);
  });
});

describe("formula · VAR (sample) and VARP (population)", () => {
  it("=VAR(1,2,3,4,5) = 2.5 (sample, n-1 denom)", () => {
    expect(evaluate("=VAR(1, 2, 3, 4, 5)", makeCtx()).value).toBeCloseTo(2.5, 6);
  });
  it("=VARP(1,2,3,4,5) = 2 (population, n denom)", () => {
    expect(evaluate("=VARP(1, 2, 3, 4, 5)", makeCtx()).value).toBeCloseTo(2, 6);
  });
  it("=VAR(single) returns #DIV0!", () => {
    expect(evaluate("=VAR(5)", makeCtx()).error).toBe("#DIV0!");
  });
  it("=VARP(single) returns 0", () => {
    expect(evaluate("=VARP(5)", makeCtx()).value).toBe(0);
  });
});

describe("formula · PERCENTILE (Excel PERCENTILE.INC convention)", () => {
  it("k=0 returns min", () => {
    expect(evaluate("=PERCENTILE(A1:A5, 0)", makeCtx()).value).toBe(1);
  });
  it("k=1 returns max", () => {
    expect(evaluate("=PERCENTILE(A1:A5, 1)", makeCtx()).value).toBe(5);
  });
  it("k=0.5 = MEDIAN", () => {
    const p = evaluate("=PERCENTILE(A1:A5, 0.5)", makeCtx()).value as number;
    const m = evaluate("=MEDIAN(A1:A5)", makeCtx()).value as number;
    expect(p).toBe(m);
  });
  it("linear interpolation between samples for non-quantile k", () => {
    // values [1,2,3,4,5], k=0.25 → position 1 (0-indexed), value 2
    expect(evaluate("=PERCENTILE(A1:A5, 0.25)", makeCtx()).value).toBe(2);
    // k=0.75 → position 3, value 4
    expect(evaluate("=PERCENTILE(A1:A5, 0.75)", makeCtx()).value).toBe(4);
  });
  it("returns #DIV0! on empty range", () => {
    expect(evaluate("=PERCENTILE(C1:C2, 0.5)", makeCtx()).error).toBe("#DIV0!");
  });
});

describe("formula · QUARTILE", () => {
  it("=QUARTILE(range, 2) === MEDIAN(range)", () => {
    const q2 = evaluate("=QUARTILE(A1:A5, 2)", makeCtx()).value as number;
    const m = evaluate("=MEDIAN(A1:A5)", makeCtx()).value as number;
    expect(q2).toBe(m);
  });
  it("=QUARTILE(range, 0) === min", () => {
    expect(evaluate("=QUARTILE(A1:A5, 0)", makeCtx()).value).toBe(1);
  });
  it("=QUARTILE(range, 4) === max", () => {
    expect(evaluate("=QUARTILE(A1:A5, 4)", makeCtx()).value).toBe(5);
  });
});
