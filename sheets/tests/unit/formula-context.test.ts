import { describe, expect, it } from "vitest";
import { evaluate } from "../../src/lib/formula";
import { buildBundleFormulaContext } from "../../src/lib/formula-context";
import type { BundleSchema, RowMap } from "../../src/lib/gigi-client";

/**
 * Phase 3 · GIGI primitives wired against the real embedder.
 *
 * `buildBundleFormulaContext` is what App.tsx uses to translate a
 * (schema, rows, kappaMap, …) tuple into a FormulaContext the formula
 * engine can evaluate against. It is the only place the formula layer
 * touches the embedder + cohort logic; tests here pin its behavior.
 *
 * Invariants this test file pins:
 *   - `=SAME(A, A) === 1`  exactly (degenerate self-comparison)
 *   - `=DIST(A, A) === 0`  exactly
 *   - `=SAME + =DIST²  === 1`  within 1e-6 (Davis double-cover identity)
 *   - `=K(ref)` reads kappaMap
 *   - `=COHORT(ref)` returns the row's cover-field value, NOT the column name
 *   - Named-field refs work over real schema columns
 */

const SCHEMA: BundleSchema = {
  name: "payments",
  base_fields: [{ name: "payment_id", type: "text" }],
  fiber_fields: [
    { name: "amount_usd", type: "numeric" },
    { name: "currency", type: "text" },
    { name: "counterparty", type: "text" },
    { name: "rail", type: "categorical" },
    { name: "value_date", type: "timestamp" },
  ],
  indexed_fields: ["payment_id"],
  records: 4,
  storage_mode: "mmap",
};

const ROWS: RowMap[] = [
  { payment_id: "P1", amount_usd: 100, currency: "USD", counterparty: "ACME Corp", rail: "ACH",  value_date: "2026-03-01" },
  { payment_id: "P2", amount_usd: 200, currency: "USD", counterparty: "ACME Corp", rail: "ACH",  value_date: "2026-03-02" },
  { payment_id: "P3", amount_usd: 100, currency: "EUR", counterparty: "Globex",    rail: "SEPA", value_date: "2026-03-01" },
  { payment_id: "P4", amount_usd: 300, currency: "USD", counterparty: "ACME Corp", rail: "ACH",  value_date: "2026-03-03" },
];

const KAPPA_MAP = new Map<string, number>([
  ["P1", 0.10],
  ["P2", 1.50],
  ["P3", 0.40],
  ["P4", 2.20],
]);

function makeCtx() {
  return buildBundleFormulaContext({
    schema: SCHEMA,
    rows: ROWS,
    kappaMap: KAPPA_MAP,
    keyField: "payment_id",
    coverField: "counterparty",
  });
}

describe("formula-context · cell + field refs", () => {
  it("=A1 reads the first column (payment_id) of row 1", () => {
    expect(evaluate("=A1", makeCtx()).value).toBe("P1");
  });

  it("=B2 reads amount_usd of row 2 → 200", () => {
    expect(evaluate("=B2", makeCtx()).value).toBe(200);
  });

  it("=SUM(amount_usd) over the column → 700", () => {
    expect(evaluate("=SUM(amount_usd)", makeCtx()).value).toBe(700);
  });

  it("=amount_usd[3] → 100 (row 3)", () => {
    expect(evaluate("=amount_usd[3]", makeCtx()).value).toBe(100);
  });
});

describe("formula-context · =K reads kappaMap", () => {
  it("=K(A1) returns κ for row 1 (P1)", () => {
    expect(evaluate("=K(A1)", makeCtx()).value).toBe(0.10);
  });

  it("=K(A4) returns κ for row 4 (P4)", () => {
    expect(evaluate("=K(A4)", makeCtx()).value).toBe(2.20);
  });
});

describe("formula-context · =COHORT returns the cover-field value", () => {
  it("=COHORT(A1) returns 'ACME Corp' (P1's counterparty)", () => {
    // The Phase 1 stub returned the column name. Phase 3 fixes this.
    expect(evaluate("=COHORT(A1)", makeCtx()).value).toBe("ACME Corp");
  });

  it("=COHORT(A3) returns 'Globex'", () => {
    expect(evaluate("=COHORT(A3)", makeCtx()).value).toBe("Globex");
  });
});

describe("formula-context · =SAME / =DIST via real embedder", () => {
  it("self-sameness is exactly 1", () => {
    expect(evaluate("=SAME(A1, A1)", makeCtx()).value).toBe(1);
    expect(evaluate("=SAME(A4, A4)", makeCtx()).value).toBe(1);
  });

  it("self-distance is exactly 0", () => {
    expect(evaluate("=DIST(A1, A1)", makeCtx()).value).toBe(0);
    expect(evaluate("=DIST(A3, A3)", makeCtx()).value).toBe(0);
  });

  it("S + d² = 1 holds for every cross-row pair", () => {
    const ctx = makeCtx();
    for (let i = 1; i <= 4; i++) {
      for (let j = 1; j <= 4; j++) {
        const s = evaluate(`=SAME(A${i}, A${j})`, ctx).value as number;
        const d = evaluate(`=DIST(A${i}, A${j})`, ctx).value as number;
        expect(Math.abs(s + d * d - 1)).toBeLessThan(1e-6);
      }
    }
  });

  it("more-similar rows have higher sameness than dissimilar rows", () => {
    const ctx = makeCtx();
    // P1 ↔ P2 (same counterparty / rail / currency, different date + amount)
    // should be MORE similar than P1 ↔ P3 (different counterparty / rail / currency).
    const s12 = evaluate("=SAME(A1, A2)", ctx).value as number;
    const s13 = evaluate("=SAME(A1, A3)", ctx).value as number;
    expect(s12).toBeGreaterThan(s13);
  });

  it("=DIST argument coercion — non-ref args return #REF!", () => {
    expect(evaluate("=SAME(1, 2)", makeCtx()).error).toBe("#REF!");
    expect(evaluate("=DIST(1, 2)", makeCtx()).error).toBe("#REF!");
  });
});

describe("formula-context · resilient to absent schema / empty rows", () => {
  it("with null schema, =SAME falls back to orthogonal (S=0)", () => {
    const empty = buildBundleFormulaContext({
      schema: null,
      rows: [],
      kappaMap: new Map(),
      keyField: undefined,
      coverField: undefined,
    });
    // No schema → no embedding possible → return the orthogonal value 0
    // (the conservative "we don't know how similar these are" answer).
    // S=0 + d²=1 still satisfies the Davis identity, so downstream
    // formulas don't break.
    expect(evaluate("=SAME(A1, A2)", empty).value).toBe(0);
    expect(evaluate("=DIST(A1, A2)", empty).value).toBe(1);
  });

  it("with no coverField, =COHORT returns empty string", () => {
    const noCover = buildBundleFormulaContext({
      schema: SCHEMA,
      rows: ROWS,
      kappaMap: new Map(),
      keyField: "payment_id",
      coverField: undefined,
    });
    expect(evaluate("=COHORT(A1)", noCover).value).toBe("");
  });
});
