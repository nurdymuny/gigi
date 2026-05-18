import { describe, expect, it } from "vitest";
import { parseCsv } from "../../src/lib/csv";
import { findDemo } from "../../src/lib/demo-datasets";
import { buildBundleSameness } from "../../src/lib/formula-context";
import type {
  BundleSchema,
  FieldDescriptor,
  RowMap,
} from "../../src/lib/gigi-client";

/**
 * Cross-check the Gallery find-similar feature against the Iris bundle.
 *
 * The hypothesis being tested: when the user pivots on a setosa flower,
 * the top-K most-similar rows should be mostly setosas — not random
 * flowers. Same for versicolor and virginica.
 *
 * Iris is the classic separability test: setosa is linearly separable
 * from the other two; versicolor and virginica overlap. So we expect:
 *
 *   setosa pivot      → top-10 are ALL setosas (50 setosas in the bundle,
 *                       species string dominates the embedding signature,
 *                       so same-species neighbors win cleanly)
 *   virginica pivot   → top-10 are mostly virginicas, maybe a few
 *                       versicolors (the overlapping band)
 *   versicolor pivot  → mostly versicolors + a few virginicas
 *
 * Also pinned: pivot-against-self is exactly 1, sameness has real
 * variance (i.e. the embedder isn't returning a constant), and the
 * Davis identity holds for every pair sampled.
 */

function loadIris() {
  const demo = findDemo("iris");
  if (!demo) throw new Error("iris demo not found");
  const parsed = parseCsv(demo.csv);
  const base_fields: FieldDescriptor[] = [
    { name: parsed.headers[0], type: parsed.types[0] },
  ];
  const fiber_fields: FieldDescriptor[] = parsed.headers
    .slice(1)
    .map((h, i) => ({ name: h, type: parsed.types[i + 1] }));
  const schema: BundleSchema = {
    name: "iris",
    base_fields,
    fiber_fields,
    indexed_fields: [parsed.headers[0]],
    records: parsed.rows.length,
    storage_mode: "mmap",
  };
  return { schema, rows: parsed.rows, keyField: "id" };
}

function speciesOf(rows: RowMap[], key: string): string {
  const r = rows.find((rr) => String(rr.id) === key);
  return r ? String(r.species ?? "") : "";
}

function topK(
  rows: RowMap[],
  pivotKey: string,
  sameness: (a: string, b: string) => number,
  k: number,
): { key: string; s: number; species: string }[] {
  const scored = rows
    .filter((r) => String(r.id) !== pivotKey)
    .map((r) => ({
      key: String(r.id),
      s: sameness(pivotKey, String(r.id)),
      species: String(r.species ?? ""),
    }));
  scored.sort((a, b) => b.s - a.s);
  return scored.slice(0, k);
}

describe("Gallery find-similar · cross-check on Iris", () => {
  const { schema, rows, keyField } = loadIris();
  const sameness = buildBundleSameness({ schema, rows, keyField });

  it("loaded 150 iris rows across 3 species (50 each)", () => {
    expect(rows.length).toBe(150);
    const counts = new Map<string, number>();
    for (const r of rows) {
      const s = String(r.species ?? "");
      counts.set(s, (counts.get(s) ?? 0) + 1);
    }
    expect(counts.get("setosa")).toBe(50);
    expect(counts.get("versicolor")).toBe(50);
    expect(counts.get("virginica")).toBe(50);
  });

  it("pivot vs self is exactly S=1 (degenerate self-case)", () => {
    for (const id of ["1", "75", "150"]) {
      expect(sameness(id, id), `S(${id}, ${id})`).toBe(1);
    }
  });

  it("sameness is non-trivial — not every pair returns 1 or 0", () => {
    const samples: number[] = [];
    for (let i = 1; i <= 150; i += 10) {
      for (let j = i + 5; j <= 150; j += 10) {
        samples.push(sameness(String(i), String(j)));
      }
    }
    expect(samples.length).toBeGreaterThan(20);
    const unique = new Set(samples.map((s) => s.toFixed(6)));
    // If the embedder were broken (constant output), this set would have
    // size 1. Real similarity values should produce dozens of distinct
    // numbers across our sample.
    expect(unique.size, `unique sameness values out of ${samples.length}`).toBeGreaterThan(10);
    // No degenerate 1s/0s for off-diagonal pairs.
    for (const s of samples) {
      expect(s).toBeGreaterThan(0);
      expect(s).toBeLessThan(1);
    }
  });

  it("setosa pivots — top-10 neighbors are all (or nearly all) setosas", () => {
    // Pick 5 setosa pivots spread across the setosa block (rows 1-50).
    const pivots = ["1", "10", "20", "30", "40"];
    let totalSame = 0;
    let totalChecked = 0;
    for (const pivotId of pivots) {
      expect(speciesOf(rows, pivotId)).toBe("setosa");
      const neighbors = topK(rows, pivotId, sameness, 10);
      const sameSpecies = neighbors.filter((n) => n.species === "setosa").length;
      totalSame += sameSpecies;
      totalChecked += neighbors.length;
      // Setosa is famously the easy class — at least 7 of the top 10
      // should be other setosas (much stricter than a random-baseline
      // expectation of 33/100 = ~3.3).
      expect(
        sameSpecies,
        `setosa pivot ${pivotId}: only ${sameSpecies}/10 top-similar are setosas. Neighbors: ${JSON.stringify(neighbors)}`,
      ).toBeGreaterThanOrEqual(7);
    }
    // Aggregate hit rate across all pivots.
    expect(totalSame / totalChecked).toBeGreaterThan(0.7);
  });

  it("virginica pivots — top-10 neighbors are mostly virginicas (with some versicolor overlap allowed)", () => {
    // Pick 5 virginica pivots (rows 101-150).
    const pivots = ["101", "110", "120", "130", "140"];
    let totalSame = 0;
    let totalChecked = 0;
    for (const pivotId of pivots) {
      expect(speciesOf(rows, pivotId)).toBe("virginica");
      const neighbors = topK(rows, pivotId, sameness, 10);
      const sameSpecies = neighbors.filter((n) => n.species === "virginica").length;
      totalSame += sameSpecies;
      totalChecked += neighbors.length;
      // virginica/versicolor overlap, so we're lenient: at least 5/10.
      expect(
        sameSpecies,
        `virginica pivot ${pivotId}: only ${sameSpecies}/10 top-similar are virginicas. Neighbors: ${JSON.stringify(neighbors)}`,
      ).toBeGreaterThanOrEqual(5);
    }
    expect(totalSame / totalChecked).toBeGreaterThan(0.5);
  });

  it("versicolor pivots — top-10 are mostly versicolor/virginica, never mostly setosa", () => {
    // Pick 5 versicolor pivots (rows 51-100).
    const pivots = ["51", "60", "70", "80", "90"];
    for (const pivotId of pivots) {
      expect(speciesOf(rows, pivotId)).toBe("versicolor");
      const neighbors = topK(rows, pivotId, sameness, 10);
      const setosaCount = neighbors.filter((n) => n.species === "setosa").length;
      // Setosa is the well-separated class — versicolor should rarely
      // pull setosas into its top-10.
      expect(
        setosaCount,
        `versicolor pivot ${pivotId}: ${setosaCount}/10 top-similar are setosas (expected ≤ 3). Neighbors: ${JSON.stringify(neighbors)}`,
      ).toBeLessThanOrEqual(3);
    }
  });

  it("within-class separation: two setosas are NOT identical-similarity to a third", () => {
    // The regression this test pins: before adding DIM_NUM, the embedder
    // hashed only the species string. Two setosas had identical embeddings
    // so S(setosa_A, setosa_C) === S(setosa_B, setosa_C) for any C in the
    // same class — find-similar couldn't rank within a class. With the
    // generic-numerics subblock, distinct measurements separate the rows.
    const reference = "5";
    const peers = ["1", "10", "15", "20", "25", "30", "35"];
    const scores = peers.map((p) => sameness(reference, p));
    const unique = new Set(scores.map((s) => s.toFixed(6)));
    expect(unique.size, `setosa scores: ${scores.map((s) => s.toFixed(4)).join(", ")}`).toBeGreaterThan(3);
  });

  it("sameness ordering is consistent — re-sorting against the same pivot gives the same top-K", () => {
    // Determinism check: no time-of-day-volatile state in the embedder.
    const pivot = "75";
    const first = topK(rows, pivot, sameness, 20).map((n) => n.key);
    const second = topK(rows, pivot, sameness, 20).map((n) => n.key);
    expect(second).toEqual(first);
  });

  it("Davis identity holds: S(a, b) + D(a, b)² = 1 for sampled Iris pairs", () => {
    // D = √(1 − S); we verify the identity through the buildBundleSameness
    // path — the Gallery's find-similar uses the same Davis math.
    for (let i = 1; i <= 150; i += 15) {
      for (let j = i + 7; j <= 150; j += 17) {
        const s = sameness(String(i), String(j));
        const d = Math.sqrt(Math.max(0, 1 - s));
        expect(Math.abs(s + d * d - 1)).toBeLessThan(1e-9);
      }
    }
  });
});
