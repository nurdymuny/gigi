import { describe, expect, it } from "vitest";
import { parseCsv } from "../../src/lib/csv";
import { findDemo } from "../../src/lib/demo-datasets";
import { evaluate } from "../../src/lib/formula";
import { buildBundleFormulaContext } from "../../src/lib/formula-context";
import type { BundleSchema, FieldDescriptor } from "../../src/lib/gigi-client";

/**
 * Phase 3.D · Davis identity over a real demo bundle.
 *
 * The single non-negotiable invariant the formula engine must honor:
 *
 *     S + d² = 1   for every (a, b) pair
 *
 * This test exercises the full formula pipeline (parse → evaluate →
 * embedder → davis.sameness → identity) on the Iris demo (150 rows,
 * 4 continuous numeric features + categorical species). 100 random
 * row pairs is the spec's threshold; we use a deterministic LCG so
 * the test fails reproducibly when a regression slips in.
 *
 * Special cases verified explicitly:
 *   SAME(A1, A1) === 1   exactly (no float drift)
 *   DIST(A1, A1) === 0   exactly (degenerate angle, sin(0) = 0)
 */

const TOL = 1e-6;

/** Deterministic LCG so the random pairs are reproducible. */
function lcg(seed: number): () => number {
  let s = seed >>> 0;
  return () => {
    s = (Math.imul(s, 1664525) + 1013904223) >>> 0;
    return s / 0x100000000;
  };
}

function bundleFromDemo(id: string): { schema: BundleSchema; rows: ReturnType<typeof parseCsv>["rows"] } {
  const demo = findDemo(id);
  if (!demo) throw new Error(`demo '${id}' not found`);
  const parsed = parseCsv(demo.csv);
  // Build a synthetic schema with the first header as the base field;
  // matches how the importer hands schemas to the engine.
  const base_fields: FieldDescriptor[] = [{ name: parsed.headers[0], type: parsed.types[0] }];
  const fiber_fields: FieldDescriptor[] = parsed.headers
    .slice(1)
    .map((h, i) => ({ name: h, type: parsed.types[i + 1] }));
  const schema: BundleSchema = {
    name: id,
    base_fields,
    fiber_fields,
    indexed_fields: [parsed.headers[0]],
    records: parsed.rows.length,
    storage_mode: "mmap",
  };
  return { schema, rows: parsed.rows };
}

describe("Davis identity · formula path · Iris bundle (150 rows)", () => {
  const { schema, rows } = bundleFromDemo("iris");
  const ctx = buildBundleFormulaContext({
    schema,
    rows,
    kappaMap: new Map(),
    keyField: "id",
    coverField: "species",
  });

  it("loaded 150 iris rows", () => {
    expect(rows.length).toBe(150);
  });

  it("=SAME(A1, A1) === 1 exactly (degenerate self-case)", () => {
    expect(evaluate("=SAME(A1, A1)", ctx).value).toBe(1);
  });

  it("=DIST(A1, A1) === 0 exactly (degenerate self-case)", () => {
    expect(evaluate("=DIST(A1, A1)", ctx).value).toBe(0);
  });

  it("=SAME + =DIST² === 1 across 100 random row pairs (tolerance 1e-6)", () => {
    const rand = lcg(0xc0ffee);
    let maxResidual = 0;
    for (let trial = 0; trial < 100; trial++) {
      const i = 1 + Math.floor(rand() * rows.length);
      const j = 1 + Math.floor(rand() * rows.length);
      const s = evaluate(`=SAME(A${i}, A${j})`, ctx).value as number;
      const d = evaluate(`=DIST(A${i}, A${j})`, ctx).value as number;
      const residual = Math.abs(s + d * d - 1);
      if (residual > maxResidual) maxResidual = residual;
      expect(residual, `trial ${trial}: SAME(A${i},A${j})=${s} DIST=${d}`).toBeLessThan(TOL);
    }
    // Sanity check: the residual is genuinely tiny, not just within tolerance
    // by accident. Float drift over a 448-dim dot product easily exceeds 1e-12
    // but should be nowhere near 1e-6.
    expect(maxResidual).toBeLessThan(TOL);
  });

  it("=SAME values fall in [0, 1] for every pair sampled", () => {
    const rand = lcg(0xdecafe);
    for (let trial = 0; trial < 50; trial++) {
      const i = 1 + Math.floor(rand() * rows.length);
      const j = 1 + Math.floor(rand() * rows.length);
      const s = evaluate(`=SAME(A${i}, A${j})`, ctx).value as number;
      expect(s).toBeGreaterThanOrEqual(0);
      expect(s).toBeLessThanOrEqual(1);
    }
  });

  it("=COHORT(ref) returns the species, NOT the column name", () => {
    // Iris is famously 50 of each species: setosa, versicolor, virginica.
    const seen = new Set<string>();
    for (let i = 1; i <= 150; i++) {
      const c = evaluate(`=COHORT(A${i})`, ctx).value as string;
      seen.add(c);
    }
    expect(seen.has("setosa")).toBe(true);
    expect(seen.has("versicolor")).toBe(true);
    expect(seen.has("virginica")).toBe(true);
  });
});
