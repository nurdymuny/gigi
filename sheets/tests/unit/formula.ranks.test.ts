import { describe, expect, it } from "vitest";
import { evaluate } from "../../src/lib/formula";
import { buildBundleFormulaContext } from "../../src/lib/formula-context";
import type { BundleSchema, RowMap } from "../../src/lib/gigi-client";

/**
 * Phase 3.C · KAPPA_RANK and SAMENESS_RANK with dense-rank tie-breaking.
 *
 * Dense rank: ties share a rank, the next distinct value gets +1, not
 * +count. Values [5, 5, 3, 1] sorted desc → ranks [1, 1, 2, 3].
 *
 *   =KAPPA_RANK(ref)              rank of ref's row by κ descending
 *   =SAMENESS_RANK(pivot, ref)    rank of ref's row by S(pivot, row) desc
 *
 * Per FORMULAS_SPEC: "Rank 1 is the highest" — anomaly leaderboards put
 * the worst row first; sameness leaderboards put the most-similar row
 * first.
 */

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [{ name: "temp", type: "numeric" }],
  indexed_fields: ["sensor_id"],
  records: 5,
  storage_mode: "mmap",
};

const ROWS: RowMap[] = [
  { sensor_id: "S1", temp: 10 },
  { sensor_id: "S2", temp: 20 },
  { sensor_id: "S3", temp: 30 },
  { sensor_id: "S4", temp: 40 },
  { sensor_id: "S5", temp: 50 },
];

const KAPPA: Map<string, number> = new Map([
  ["S1", 0.1],
  ["S2", 2.5],
  ["S3", 0.4],
  ["S4", 2.5], // tie with S2
  ["S5", 0.7],
]);

function makeCtx() {
  return buildBundleFormulaContext({
    schema: SCHEMA,
    rows: ROWS,
    kappaMap: KAPPA,
    keyField: "sensor_id",
    coverField: undefined,
  });
}

describe("formula · KAPPA_RANK (dense, descending)", () => {
  it("highest κ → rank 1 (ties also rank 1)", () => {
    // Sorted desc: S2=2.5, S4=2.5, S5=0.7, S3=0.4, S1=0.1
    // Dense ranks:  1,     1,     2,     3,     4
    expect(evaluate("=KAPPA_RANK(A2)", makeCtx()).value).toBe(1);
    expect(evaluate("=KAPPA_RANK(A4)", makeCtx()).value).toBe(1);
    expect(evaluate("=KAPPA_RANK(A5)", makeCtx()).value).toBe(2);
    expect(evaluate("=KAPPA_RANK(A3)", makeCtx()).value).toBe(3);
    expect(evaluate("=KAPPA_RANK(A1)", makeCtx()).value).toBe(4);
  });

  it("=KAPPA_RANK of a non-ref arg → #REF!", () => {
    expect(evaluate("=KAPPA_RANK(1)", makeCtx()).error).toBe("#REF!");
  });

  it("=KAPPA_RANK of a ref past the row count → #REF!", () => {
    expect(evaluate("=KAPPA_RANK(A99)", makeCtx()).error).toBe("#REF!");
  });

  it("works with named field refs (KAPPA_RANK(temp[2]))", () => {
    expect(evaluate("=KAPPA_RANK(temp[2])", makeCtx()).value).toBe(1);
    expect(evaluate("=KAPPA_RANK(temp[1])", makeCtx()).value).toBe(4);
  });
});

describe("formula · SAMENESS_RANK (dense, descending against a pivot)", () => {
  it("the pivot row itself ranks 1 (S=1 against itself)", () => {
    // Every row has S(row, row) = 1, which is the max possible. So
    // SAMENESS_RANK(pivot, pivot) is always 1.
    expect(evaluate("=SAMENESS_RANK(A1, A1)", makeCtx()).value).toBe(1);
    expect(evaluate("=SAMENESS_RANK(A3, A3)", makeCtx()).value).toBe(1);
  });

  it("other rows rank by sameness to the pivot descending", () => {
    // With distinct rows, no ties → ranks are 1..5. Pivot is rank 1.
    // The exact ranks of S2..S5 against A1 depend on the embedder, but
    // the pivot must rank 1 AND each other row's rank must be in 2..5.
    const ctx = makeCtx();
    expect(evaluate("=SAMENESS_RANK(A1, A1)", ctx).value).toBe(1);
    for (let i = 2; i <= 5; i++) {
      const r = evaluate(`=SAMENESS_RANK(A1, A${i})`, ctx).value as number;
      expect(r).toBeGreaterThanOrEqual(2);
      expect(r).toBeLessThanOrEqual(5);
    }
  });

  it("=SAMENESS_RANK with non-ref args → #REF!", () => {
    expect(evaluate("=SAMENESS_RANK(1, 2)", makeCtx()).error).toBe("#REF!");
  });
});

describe("formula · KAPPA_RANK reserved-name + parsing", () => {
  it("KAPPA_RANK is reserved (cannot be shadowed by a field)", () => {
    const ctx = buildBundleFormulaContext({
      schema: { ...SCHEMA, fiber_fields: [{ name: "kappa_rank", type: "numeric" }] },
      rows: [{ sensor_id: "S1", kappa_rank: 0 }],
      kappaMap: new Map([["S1", 0]]),
      keyField: "sensor_id",
      coverField: undefined,
    });
    // The function wins; bare `=kappa_rank` (no parens) is "function called wrong" → #NAME!.
    expect(evaluate("=kappa_rank", ctx).error).toBe("#NAME!");
  });
});
