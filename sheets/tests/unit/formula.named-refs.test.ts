import { describe, expect, it } from "vitest";
import { evaluate, type FormulaContext } from "../../src/lib/formula";

/**
 * Phase 1.D · named-field refs + identifier collision rule.
 *
 *   temperature        → the entire column (acts as a range in aggregates)
 *   temperature[5]     → row 5 of the temperature column
 *
 * Identifier collision: reserved function names (MEDIAN, SUM, etc.)
 * always parse as functions, never as field refs. A bundle with a
 * field literally named `median` must access it via A1 notation.
 */

function makeCtx(): FormulaContext {
  // Three "columns": temperature (5 rows), status (5 rows), nope (absent).
  const tempByRow: Record<number, number> = { 1: 22.0, 2: 23.5, 3: 21.8, 4: 25.1, 5: 24.0 };
  const statusByRow: Record<number, string> = { 1: "ok", 2: "ok", 3: "warn", 4: "ok", 5: "bad" };
  // Simulate the A1 cell mapping: column A = temperature, column B = status.
  const cellMap: Record<string, number | string | null> = {};
  for (let r = 1; r <= 5; r++) {
    cellMap[`A${r}`] = tempByRow[r];
    cellMap[`B${r}`] = statusByRow[r];
  }
  return {
    cell: (ref) => (ref in cellMap ? cellMap[ref] : null),
    sameness: () => 0.5,
    kappa: () => 0,
    cohort: () => "",
    resolveField: (name) => {
      if (name === "temperature") return ["A1", "A2", "A3", "A4", "A5"];
      if (name === "status") return ["B1", "B2", "B3", "B4", "B5"];
      return null;
    },
    fieldRowRef: (name, row) => {
      if (name === "temperature") {
        return row >= 1 && row <= 5 ? `A${row}` : null;
      }
      if (name === "status") {
        return row >= 1 && row <= 5 ? `B${row}` : null;
      }
      return null;
    },
  };
}

describe("formula · named-field refs (whole column)", () => {
  it("=SUM(temperature) sums the whole column", () => {
    // 22 + 23.5 + 21.8 + 25.1 + 24 = 116.4
    const v = evaluate("=SUM(temperature)", makeCtx()).value as number;
    expect(v).toBeCloseTo(116.4, 5);
  });
  it("=AVG(temperature) averages the whole column", () => {
    const v = evaluate("=AVG(temperature)", makeCtx()).value as number;
    expect(v).toBeCloseTo(23.28, 5);
  });
  it("=COUNT(temperature) returns 5", () => {
    expect(evaluate("=COUNT(temperature)", makeCtx()).value).toBe(5);
  });
  it("=MEDIAN(temperature) returns 23.5", () => {
    expect(evaluate("=MEDIAN(temperature)", makeCtx()).value).toBe(23.5);
  });
  it("=MAX(temperature) returns 25.1", () => {
    expect(evaluate("=MAX(temperature)", makeCtx()).value).toBe(25.1);
  });
});

describe("formula · named-field refs with row index", () => {
  it("=temperature[3] returns 21.8", () => {
    expect(evaluate("=temperature[3]", makeCtx()).value).toBe(21.8);
  });
  it("=temperature[1] + temperature[2] returns 45.5", () => {
    expect(evaluate("=temperature[1] + temperature[2]", makeCtx()).value).toBe(
      45.5,
    );
  });
  it("=temperature[6] (out of bounds) returns #REF!", () => {
    expect(evaluate("=temperature[6]", makeCtx()).error).toBe("#REF!");
  });
  it("=temperature[0] (1-based; zero is invalid) returns #REF!", () => {
    expect(evaluate("=temperature[0]", makeCtx()).error).toBe("#REF!");
  });
  it("=temperature[-1] returns #REF!", () => {
    expect(evaluate("=temperature[-1]", makeCtx()).error).toBe("#REF!");
  });
});

describe("formula · unknown field name", () => {
  it("=nope returns #NAME!", () => {
    expect(evaluate("=nope", makeCtx()).error).toBe("#NAME!");
  });
  it("=SUM(nope) returns #NAME!", () => {
    expect(evaluate("=SUM(nope)", makeCtx()).error).toBe("#NAME!");
  });
  it("=nope[3] returns #NAME!", () => {
    expect(evaluate("=nope[3]", makeCtx()).error).toBe("#NAME!");
  });
});

describe("formula · identifier collision rule (reserved beats field)", () => {
  it("a bundle with a field named 'median' does NOT match =MEDIAN(…)", () => {
    // Set up a ctx that has a `median` field — should NOT shadow the function.
    const ctx: FormulaContext = {
      cell: () => null,
      sameness: () => 0.5,
      kappa: () => 0,
      cohort: () => "",
      resolveField: (name) =>
        name === "median" ? ["A1", "A2", "A3"] : null,
    };
    // `=MEDIAN(...)` should be the function call, not a field ref.
    const r = evaluate("=MEDIAN(1, 2, 3)", ctx);
    expect(r.error).toBeNull();
    expect(r.value).toBe(2);
  });
  it("=MEDIAN alone (no parens) returns #NAME! — function called wrong", () => {
    // Reserved function names always parse as functions. `=MEDIAN` alone
    // is "function MEDIAN called without parens" — #NAME!.
    const ctx: FormulaContext = {
      cell: () => null,
      sameness: () => 0.5,
      kappa: () => 0,
      cohort: () => "",
      resolveField: (name) =>
        name === "median" ? ["A1", "A2", "A3"] : null,
    };
    expect(evaluate("=MEDIAN", ctx).error).toBe("#NAME!");
  });
  it("reserved names are case-insensitive (median === MEDIAN)", () => {
    const ctx: FormulaContext = {
      cell: () => null,
      sameness: () => 0.5,
      kappa: () => 0,
      cohort: () => "",
      resolveField: () => null,
    };
    expect(evaluate("=median(1, 2, 3)", ctx).value).toBe(2);
    expect(evaluate("=Median(1, 2, 3)", ctx).value).toBe(2);
  });
});

describe("formula · field names are case-sensitive", () => {
  it("Temperature (capital T) does not match temperature", () => {
    const ctx: FormulaContext = {
      cell: () => null,
      sameness: () => 0.5,
      kappa: () => 0,
      cohort: () => "",
      resolveField: (name) =>
        name === "temperature" ? ["A1"] : null,
    };
    expect(evaluate("=Temperature", ctx).error).toBe("#NAME!");
  });
});
