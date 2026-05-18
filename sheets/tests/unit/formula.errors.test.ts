import { describe, expect, it } from "vitest";
import { evaluate, type FormulaContext } from "../../src/lib/formula";

/**
 * Phase 1.C · error propagation through operators AND aggregates.
 *
 * Cells can hold error-sentinel strings (e.g. `"#REF!"`) — this happens
 * when a formula upstream evaluated to an error and the bundle row
 * captured it. Aggregates must POISON on these (Excel parity): if any
 * input is an error, the result is that same error.
 */

function ctxWith(cells: Record<string, number | string | null>): FormulaContext {
  return {
    cell: (ref) => (ref in cells ? cells[ref] : null),
    sameness: () => 0.5,
    kappa: () => 0,
    cohort: () => "",
  };
}

describe("formula · error sentinels parse as themselves", () => {
  it("explicit error string in a cell propagates through SUM", () => {
    const ctx = ctxWith({ A1: 1, A2: "#REF!", A3: 3 });
    expect(evaluate("=SUM(A1:A3)", ctx).error).toBe("#REF!");
  });
});

describe("formula · aggregate poisoning", () => {
  it("SUM poisons on cell-sentinel #REF!", () => {
    const ctx = ctxWith({ A1: 1, A2: "#REF!", A3: 3 });
    expect(evaluate("=SUM(A1:A3)", ctx).error).toBe("#REF!");
  });
  it("AVG poisons on cell-sentinel #DIV0!", () => {
    const ctx = ctxWith({ A1: 1, A2: "#DIV0!", A3: 3 });
    expect(evaluate("=AVG(A1:A3)", ctx).error).toBe("#DIV0!");
  });
  it("MIN poisons on cell-sentinel", () => {
    const ctx = ctxWith({ A1: 1, A2: "#VALUE!" });
    expect(evaluate("=MIN(A1:A2)", ctx).error).toBe("#VALUE!");
  });
  it("MAX poisons on cell-sentinel", () => {
    const ctx = ctxWith({ A1: 1, A2: "#NAME!" });
    expect(evaluate("=MAX(A1:A2)", ctx).error).toBe("#NAME!");
  });
  it("MEDIAN poisons on cell-sentinel", () => {
    const ctx = ctxWith({ A1: 1, A2: "#REF!", A3: 3, A4: 4 });
    expect(evaluate("=MEDIAN(A1:A4)", ctx).error).toBe("#REF!");
  });
  it("STDEV poisons on cell-sentinel", () => {
    const ctx = ctxWith({ A1: 1, A2: 2, A3: "#REF!" });
    expect(evaluate("=STDEV(A1:A3)", ctx).error).toBe("#REF!");
  });
  it("COUNT poisons on cell-sentinel (any error in input)", () => {
    const ctx = ctxWith({ A1: 1, A2: "#REF!" });
    expect(evaluate("=COUNT(A1:A2)", ctx).error).toBe("#REF!");
  });
  it("COUNTA poisons on cell-sentinel", () => {
    const ctx = ctxWith({ A1: 1, A2: "#REF!" });
    expect(evaluate("=COUNTA(A1:A2)", ctx).error).toBe("#REF!");
  });
  it("PERCENTILE poisons on cell-sentinel", () => {
    const ctx = ctxWith({ A1: 1, A2: 2, A3: "#REF!" });
    expect(evaluate("=PERCENTILE(A1:A3, 0.5)", ctx).error).toBe("#REF!");
  });
});

describe("formula · binary operator error propagation", () => {
  it("plus with error operand poisons", () => {
    const ctx = ctxWith({ A1: 1, A2: "#REF!" });
    expect(evaluate("=A1 + A2", ctx).error).toBe("#REF!");
  });
  it("comparison with error operand poisons", () => {
    const ctx = ctxWith({ A1: 1, A2: "#REF!" });
    expect(evaluate("=A1 > A2", ctx).error).toBe("#REF!");
  });
  it("concat with error operand poisons", () => {
    const ctx = ctxWith({ A1: "x", A2: "#REF!" });
    expect(evaluate("=A1 & A2", ctx).error).toBe("#REF!");
  });
  it("IF poisons when condition is an error", () => {
    const ctx = ctxWith({ A1: "#REF!" });
    expect(evaluate('=IF(A1, "yes", "no")', ctx).error).toBe("#REF!");
  });
});

describe("formula · error precedence (first error wins)", () => {
  it("SUM with two errors returns the first encountered", () => {
    const ctx = ctxWith({ A1: 1, A2: "#REF!", A3: "#DIV0!" });
    expect(evaluate("=SUM(A1:A3)", ctx).error).toBe("#REF!");
  });
});

describe("formula · parse-depth DoS guard", () => {
  // A 5000-deep `=(((((…)))))` would otherwise blow the JS call stack
  // and hang the tab — the sidecar lets attacker-controlled formulas
  // load on page reload, so this gate has to live at the parser layer.
  // The evaluator catches FormulaParseError and returns #ERROR!.
  it("returns #ERROR! when a formula nests past MAX_PARSE_DEPTH", () => {
    const ctx: FormulaContext = {
      cell: () => null,
      sameness: () => 0,
      kappa: () => 0,
      cohort: () => "",
    };
    const huge = "=" + "(".repeat(5000) + "1" + ")".repeat(5000);
    const r = evaluate(huge, ctx);
    expect(r.error).toBe("#ERROR!");
  });

  it("a normal multi-level nested formula still parses fine", () => {
    const ctx: FormulaContext = {
      cell: () => null,
      sameness: () => 0,
      kappa: () => 0,
      cohort: () => "",
    };
    expect(evaluate("=((((1+2))*3)+(4*(5+6)))", ctx).value).toBe(53);
  });
});
