import { describe, expect, it } from "vitest";
import { evaluate, type FormulaContext } from "../../src/lib/formula";

/**
 * Phase 1.5.B · *IF / *IFS family.
 *
 * Functions covered: SUMIF, COUNTIF, AVERAGEIF, SUMIFS, COUNTIFS,
 * AVERAGEIFS, MINIFS, MAXIFS. Predicate semantics live in
 * formula-predicate.ts; these tests pin the wiring.
 *
 * Bundle shape used here:
 *   A1..A5   numeric        [10, 20, 30, 40, 50]
 *   B1..B5   text/category  ["INV-1", "INV-2", "PO-3", "INV-4", "PO-5"]
 *   C1..C5   numeric         [1.5, 2.5, 3.5, 4.5, 5.5]   (sum_range for IFS)
 */

function makeCtx(): FormulaContext {
  const cells: Record<string, number | string | null> = {
    A1: 10, A2: 20, A3: 30, A4: 40, A5: 50,
    B1: "INV-1", B2: "INV-2", B3: "PO-3", B4: "INV-4", B5: "PO-5",
    C1: 1.5, C2: 2.5, C3: 3.5, C4: 4.5, C5: 5.5,
  };
  return {
    cell: (ref) => (ref in cells ? cells[ref] : null),
    sameness: () => 0.5,
    kappa: () => 0,
    cohort: () => "",
  };
}

describe("formula · SUMIF (single criterion)", () => {
  it('=SUMIF(A1:A5, ">25") sums values > 25 → 30+40+50 = 120', () => {
    expect(evaluate('=SUMIF(A1:A5, ">25")', makeCtx()).value).toBe(120);
  });

  it("=SUMIF(A1:A5, 30) sums values equal to 30", () => {
    expect(evaluate("=SUMIF(A1:A5, 30)", makeCtx()).value).toBe(30);
  });

  it('=SUMIF(A1:A5, "<>20") sums everything except 20 → 10+30+40+50 = 130', () => {
    expect(evaluate('=SUMIF(A1:A5, "<>20")', makeCtx()).value).toBe(130);
  });

  it('=SUMIF(B1:B5, "INV*", A1:A5) sums A where B starts with INV → 10+20+40 = 70', () => {
    expect(
      evaluate('=SUMIF(B1:B5, "INV*", A1:A5)', makeCtx()).value,
    ).toBe(70);
  });

  it("returns 0 when no value matches", () => {
    expect(evaluate('=SUMIF(A1:A5, ">999")', makeCtx()).value).toBe(0);
  });
});

describe("formula · COUNTIF", () => {
  it('=COUNTIF(A1:A5, ">=30") counts 30, 40, 50 → 3', () => {
    expect(evaluate('=COUNTIF(A1:A5, ">=30")', makeCtx()).value).toBe(3);
  });

  it('=COUNTIF(B1:B5, "INV*") counts 3 invoice rows', () => {
    expect(evaluate('=COUNTIF(B1:B5, "INV*")', makeCtx()).value).toBe(3);
  });

  it("=COUNTIF(A1:A5, 999) → 0 (no matches)", () => {
    expect(evaluate("=COUNTIF(A1:A5, 999)", makeCtx()).value).toBe(0);
  });
});

describe("formula · AVERAGEIF", () => {
  it('=AVERAGEIF(A1:A5, ">20") averages 30,40,50 → 40', () => {
    expect(evaluate('=AVERAGEIF(A1:A5, ">20")', makeCtx()).value).toBe(40);
  });

  it('=AVERAGEIF(B1:B5, "INV*", A1:A5) averages A where B is invoice → (10+20+40)/3', () => {
    const v = evaluate('=AVERAGEIF(B1:B5, "INV*", A1:A5)', makeCtx()).value as number;
    expect(v).toBeCloseTo(70 / 3, 9);
  });

  it("returns #DIV0! when nothing matches", () => {
    expect(evaluate('=AVERAGEIF(A1:A5, ">999")', makeCtx()).error).toBe("#DIV0!");
  });
});

describe("formula · SUMIFS (multi-criterion, all ANDed)", () => {
  it('=SUMIFS(C1:C5, A1:A5, ">15", B1:B5, "INV*") → C2+C4 = 2.5+4.5 = 7', () => {
    // Rows where A>15 AND B starts with INV: rows 2 (A=20, B=INV-2) and 4 (A=40, B=INV-4).
    expect(
      evaluate('=SUMIFS(C1:C5, A1:A5, ">15", B1:B5, "INV*")', makeCtx()).value,
    ).toBe(7);
  });

  it('=SUMIFS with a single criterion behaves like SUMIF(sum, range, pred)', () => {
    // SUMIFS arg order is (sum_range, crit_range1, pred1, …), whereas
    // SUMIF puts the predicate after the criteria range. Easy to confuse.
    expect(
      evaluate('=SUMIFS(C1:C5, A1:A5, ">=30")', makeCtx()).value,
    ).toBe(3.5 + 4.5 + 5.5);
  });

  it("returns 0 when no row matches every criterion", () => {
    expect(
      evaluate('=SUMIFS(C1:C5, A1:A5, ">15", B1:B5, "FOO*")', makeCtx()).value,
    ).toBe(0);
  });

  it("returns #VALUE! when ranges have different lengths", () => {
    // Different-length criteria ranges are a user error; we surface it
    // rather than silently truncating.
    expect(
      evaluate('=SUMIFS(C1:C5, A1:A3, ">0", B1:B5, "INV*")', makeCtx()).error,
    ).toBe("#VALUE!");
  });
});

describe("formula · COUNTIFS", () => {
  it('=COUNTIFS(A1:A5, ">15", B1:B5, "INV*") → 2', () => {
    expect(
      evaluate('=COUNTIFS(A1:A5, ">15", B1:B5, "INV*")', makeCtx()).value,
    ).toBe(2);
  });

  it("single-pair COUNTIFS == COUNTIF", () => {
    expect(
      evaluate('=COUNTIFS(A1:A5, ">=30")', makeCtx()).value,
    ).toBe(3);
  });
});

describe("formula · AVERAGEIFS / MINIFS / MAXIFS", () => {
  it('=AVERAGEIFS(C1:C5, A1:A5, ">15", B1:B5, "INV*") → mean(C2,C4) = (2.5+4.5)/2 = 3.5', () => {
    expect(
      evaluate('=AVERAGEIFS(C1:C5, A1:A5, ">15", B1:B5, "INV*")', makeCtx()).value,
    ).toBe(3.5);
  });

  it('=MINIFS(C1:C5, A1:A5, ">15") → min(C2..C5) = 2.5', () => {
    expect(
      evaluate('=MINIFS(C1:C5, A1:A5, ">15")', makeCtx()).value,
    ).toBe(2.5);
  });

  it('=MAXIFS(C1:C5, B1:B5, "INV*") → max(C1, C2, C4) = 4.5', () => {
    expect(
      evaluate('=MAXIFS(C1:C5, B1:B5, "INV*")', makeCtx()).value,
    ).toBe(4.5);
  });

  it("AVERAGEIFS with no matches → #DIV0!", () => {
    expect(
      evaluate('=AVERAGEIFS(C1:C5, A1:A5, ">999")', makeCtx()).error,
    ).toBe("#DIV0!");
  });

  it("MINIFS / MAXIFS with no matches → 0 (Excel parity)", () => {
    expect(
      evaluate('=MINIFS(C1:C5, A1:A5, ">999")', makeCtx()).value,
    ).toBe(0);
    expect(
      evaluate('=MAXIFS(C1:C5, A1:A5, ">999")', makeCtx()).value,
    ).toBe(0);
  });
});

describe("formula · *IF error propagation", () => {
  it("error sentinel in the criteria range poisons SUMIF", () => {
    const ctx: FormulaContext = {
      cell: (ref) => (ref === "A2" ? "#REF!" : ref === "A1" ? 10 : ref === "A3" ? 30 : null),
      sameness: () => 0.5,
      kappa: () => 0,
      cohort: () => "",
    };
    expect(evaluate('=SUMIF(A1:A3, ">0")', ctx).error).toBe("#REF!");
  });

  it("error sentinel in the sum range poisons SUMIF", () => {
    const ctx: FormulaContext = {
      cell: (ref) => {
        if (ref === "A1") return 10;
        if (ref === "A2") return 20;
        if (ref === "A3") return 30;
        if (ref === "C2") return "#NAME!";
        if (ref === "C1") return 1;
        if (ref === "C3") return 3;
        return null;
      },
      sameness: () => 0.5,
      kappa: () => 0,
      cohort: () => "",
    };
    expect(
      evaluate('=SUMIF(A1:A3, ">0", C1:C3)', ctx).error,
    ).toBe("#NAME!");
  });
});
