import { describe, expect, it } from "vitest";
import { evaluate, type FormulaContext } from "../../src/lib/formula";

/**
 * Phase 2.A · sliced named-field ranges: `temperature[1:10]`.
 *
 * Phase 1.D shipped:
 *   temperature          → whole-column range
 *   temperature[5]       → single row
 *
 * Phase 2.A adds:
 *   temperature[1:5]     → rows 1 through 5 (1-based, inclusive)
 *   temperature[2:2]     → degenerate single-row range
 *
 * Open-ended slices (`temperature[3:]`, `temperature[:5]`) are NOT in
 * scope for v1 — the user can always use the whole-column form or be
 * explicit about both ends.
 */

function makeCtx(): FormulaContext {
  const temp: Record<number, number> = { 1: 22, 2: 23, 3: 21, 4: 25, 5: 24, 6: 26, 7: 27 };
  const cellMap: Record<string, number | null> = {};
  for (let r = 1; r <= 7; r++) cellMap[`A${r}`] = temp[r];
  return {
    cell: (ref) => (ref in cellMap ? cellMap[ref] : null),
    sameness: () => 0.5,
    kappa: () => 0,
    cohort: () => "",
    resolveField: (name) => (name === "temperature" ? ["A1", "A2", "A3", "A4", "A5", "A6", "A7"] : null),
    fieldRowRef: (name, row) =>
      name === "temperature" && row >= 1 && row <= 7 ? `A${row}` : null,
  };
}

describe("formula · sliced field ranges", () => {
  it("=SUM(temperature[1:5]) sums rows 1..5 → 22+23+21+25+24 = 115", () => {
    expect(evaluate("=SUM(temperature[1:5])", makeCtx()).value).toBe(115);
  });

  it("=AVG(temperature[1:5]) averages 5 rows → 23", () => {
    expect(evaluate("=AVG(temperature[1:5])", makeCtx()).value).toBe(23);
  });

  it("=COUNT(temperature[3:7]) counts 5 cells", () => {
    expect(evaluate("=COUNT(temperature[3:7])", makeCtx()).value).toBe(5);
  });

  it("=MAX(temperature[1:3]) → max of first 3 rows = 23", () => {
    expect(evaluate("=MAX(temperature[1:3])", makeCtx()).value).toBe(23);
  });

  it("degenerate slice =SUM(temperature[2:2]) returns row 2 value", () => {
    expect(evaluate("=SUM(temperature[2:2])", makeCtx()).value).toBe(23);
  });

  it("inverted bounds =SUM(temperature[5:1]) returns 0 (empty range, not an error)", () => {
    // Matches the existing A1:A0 behavior — an empty-resolved range
    // contributes no values rather than throwing.
    expect(evaluate("=SUM(temperature[5:1])", makeCtx()).value).toBe(0);
  });

  it("=SUM(temperature[1:99]) clamps to available rows", () => {
    // Out-of-bounds upper limit should clamp to the last available row
    // rather than emit #REF!; lets users write `temperature[1:1000]` to
    // mean "everything".
    expect(evaluate("=SUM(temperature[1:99])", makeCtx()).value).toBe(
      22 + 23 + 21 + 25 + 24 + 26 + 27,
    );
  });

  it("=SUM(temperature[0:3]) returns #REF! (1-based, zero invalid)", () => {
    expect(evaluate("=SUM(temperature[0:3])", makeCtx()).error).toBe("#REF!");
  });

  it("=SUM(nope[1:5]) on unknown field returns #NAME!", () => {
    expect(evaluate("=SUM(nope[1:5])", makeCtx()).error).toBe("#NAME!");
  });

  it("=temperature[1:3] outside an aggregate collapses to the first cell", () => {
    // Symmetric with `range` / bare `fieldRef`: outside an aggregate,
    // the leading cell wins.
    expect(evaluate("=temperature[1:3]", makeCtx()).value).toBe(22);
  });
});

describe("formula · sliced field ranges compose with SUMIF / IFS", () => {
  it("=SUMIF(temperature[1:5], '>22') → 23+25+24 = 72", () => {
    expect(evaluate("=SUMIF(temperature[1:5], \">22\")", makeCtx()).value).toBe(72);
  });

  it("=COUNTIF(temperature[3:7], '>=25') → 25,26,27 → 3", () => {
    expect(evaluate("=COUNTIF(temperature[3:7], \">=25\")", makeCtx()).value).toBe(3);
  });
});
