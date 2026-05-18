import { describe, expect, it } from "vitest";
import { evaluate, type FormulaContext } from "../../src/lib/formula";

function makeCtx(overrides: Partial<FormulaContext> = {}): FormulaContext {
  return {
    cell: (ref) => {
      const map: Record<string, number | string | null> = {
        A1: 1,
        A2: 2,
        A3: 3,
        A4: 4,
        B1: 10,
        B2: 20,
        B3: 30,
        C1: "hello",
        C2: "world",
      };
      return ref in map ? map[ref] : null;
    },
    sameness: (a, b) => {
      // Mock pair: sameness depends on whether refs match.
      if (a === b) return 1;
      if ((a === "A1" && b === "A2") || (a === "A2" && b === "A1")) return 0.8;
      return 0.5;
    },
    kappa: (ref) => {
      const map: Record<string, number> = { A1: 0.05, A2: 0.12, A3: 0.42 };
      return ref in map ? map[ref] : 0;
    },
    cohort: (col) => `cohort:${col}`,
    ...overrides,
  };
}

describe("formula · arithmetic", () => {
  it("evaluates literals", () => {
    expect(evaluate("=1", makeCtx()).value).toBe(1);
    expect(evaluate("=1.5", makeCtx()).value).toBe(1.5);
    expect(evaluate("=-3", makeCtx()).value).toBe(-3);
  });

  it("evaluates + - * / with precedence", () => {
    expect(evaluate("=1+2*3", makeCtx()).value).toBe(7);
    expect(evaluate("=(1+2)*3", makeCtx()).value).toBe(9);
    expect(evaluate("=10/4", makeCtx()).value).toBe(2.5);
    expect(evaluate("=10-3-2", makeCtx()).value).toBe(5);
  });

  it("evaluates unary minus", () => {
    expect(evaluate("=-2*3", makeCtx()).value).toBe(-6);
    expect(evaluate("=5+-3", makeCtx()).value).toBe(2);
  });

  it("returns #DIV0! on division by zero", () => {
    const r = evaluate("=1/0", makeCtx());
    expect(r.error).toBe("#DIV0!");
  });
});

describe("formula · cell references", () => {
  it("resolves a single cell ref", () => {
    expect(evaluate("=A1", makeCtx()).value).toBe(1);
    expect(evaluate("=B1+B2", makeCtx()).value).toBe(30);
  });

  it("returns 0 for unknown cells (Excel convention)", () => {
    expect(evaluate("=Z99", makeCtx()).value).toBe(0);
  });

  it("mixed cell-ref + literal expressions", () => {
    expect(evaluate("=A1*10+B1", makeCtx()).value).toBe(20);
  });
});

describe("formula · functions", () => {
  it("=SUM with a range", () => {
    expect(evaluate("=SUM(A1:A4)", makeCtx()).value).toBe(10);
  });

  it("=SUM with discrete args", () => {
    expect(evaluate("=SUM(A1, A2, A3)", makeCtx()).value).toBe(6);
  });

  it("=AVG / =AVERAGE alias", () => {
    expect(evaluate("=AVG(A1:A4)", makeCtx()).value).toBe(2.5);
    expect(evaluate("=AVERAGE(A1:A4)", makeCtx()).value).toBe(2.5);
  });

  it("=MIN / =MAX", () => {
    expect(evaluate("=MIN(A1:A4)", makeCtx()).value).toBe(1);
    expect(evaluate("=MAX(A1:A4)", makeCtx()).value).toBe(4);
  });

  it("=COUNT counts numeric cells, skips text", () => {
    expect(evaluate("=COUNT(A1, A2, C1)", makeCtx()).value).toBe(2);
  });

  it("=IF returns the true branch for truthy", () => {
    expect(evaluate('=IF(1, "yes", "no")', makeCtx()).value).toBe("yes");
  });

  it("=IF returns the false branch for falsy", () => {
    expect(evaluate('=IF(0, "yes", "no")', makeCtx()).value).toBe("no");
  });
});

describe("formula · GIGI primitives", () => {
  it("=SAME(A1, A2) returns Davis sameness", () => {
    expect(evaluate("=SAME(A1, A2)", makeCtx()).value).toBe(0.8);
  });

  it("=SAME(A1, A1) returns 1", () => {
    expect(evaluate("=SAME(A1, A1)", makeCtx()).value).toBe(1);
  });

  it("=K(A1) returns the row's curvature", () => {
    expect(evaluate("=K(A1)", makeCtx()).value).toBe(0.05);
  });

  it("=DIST(A1, A2) satisfies the Davis double-cover identity SAME + DIST² = 1", () => {
    const S = evaluate("=SAME(A1, A2)", makeCtx()).value as number;
    const D = evaluate("=DIST(A1, A2)", makeCtx()).value as number;
    expect(S + D * D).toBeCloseTo(1, 6);
  });

  it("=DIST(x, x) = 0", () => {
    expect(evaluate("=DIST(A1, A1)", makeCtx()).value).toBe(0);
  });

  it("=COHORT(col) returns the cohort name for a column", () => {
    expect(evaluate('=COHORT("region")', makeCtx()).value).toBe("cohort:region");
  });
});

describe("formula · errors", () => {
  it("unknown function returns #NAME!", () => {
    const r = evaluate("=NOPE(1)", makeCtx());
    expect(r.error).toBe("#NAME!");
  });

  it("syntax error returns #ERROR!", () => {
    const r = evaluate("=1+", makeCtx());
    expect(r.error).toBe("#ERROR!");
  });

  it("non-formula input returns the raw string as value", () => {
    expect(evaluate("hello", makeCtx()).value).toBe("hello");
  });

  it("empty string returns null", () => {
    expect(evaluate("", makeCtx()).value).toBeNull();
  });
});
