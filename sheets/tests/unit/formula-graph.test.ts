import { beforeEach, describe, expect, it } from "vitest";
import { FormulaGraph } from "../../src/lib/formula-graph";

/**
 * Phase 2.C · formula dependency graph + topological recompute order.
 *
 * The graph tracks `(formulaRef → deps)` and the reverse `(ref →
 * dependents)`. Given one or more *source cell changes*, it returns
 * the topologically ordered list of formula refs that need
 * re-evaluation — sources of a formula always evaluated before the
 * formula itself.
 *
 * Cycles are detected at this layer (Phase 2.D wires them into eval).
 */

let g: FormulaGraph;
beforeEach(() => {
  g = new FormulaGraph();
});

describe("FormulaGraph · register and forget", () => {
  it("has no formulas by default", () => {
    expect(g.has("A1")).toBe(false);
  });

  it("setFormula registers a formula at a cell", () => {
    g.setFormula("B1", new Set(["A1"]));
    expect(g.has("B1")).toBe(true);
  });

  it("removeFormula unregisters", () => {
    g.setFormula("B1", new Set(["A1"]));
    g.removeFormula("B1");
    expect(g.has("B1")).toBe(false);
    // And the reverse index drops the back-edge.
    expect(g.dependents("A1").size).toBe(0);
  });

  it("setFormula on an existing key replaces deps cleanly", () => {
    g.setFormula("C1", new Set(["A1"]));
    g.setFormula("C1", new Set(["B1"])); // swap dep
    expect(g.dependents("A1").size).toBe(0); // A1 no longer feeds C1
    expect(g.dependents("B1").has("C1")).toBe(true);
  });
});

describe("FormulaGraph · dependents (direct)", () => {
  it("returns formulas that read a given ref", () => {
    g.setFormula("B1", new Set(["A1"]));
    g.setFormula("B2", new Set(["A1", "A2"]));
    expect([...g.dependents("A1")].sort()).toEqual(["B1", "B2"]);
    expect([...g.dependents("A2")]).toEqual(["B2"]);
  });
});

describe("FormulaGraph · affected (transitive + topological)", () => {
  it("single dependent chain: A1 ← B1 ← C1", () => {
    g.setFormula("B1", new Set(["A1"]));
    g.setFormula("C1", new Set(["B1"]));
    const r = g.affected(["A1"]);
    expect(r.cycle).toBeNull();
    // B1 must come before C1 (C1 reads B1).
    expect(r.order).toEqual(["B1", "C1"]);
  });

  it("diamond: A1 ← B1, A1 ← B2, B1+B2 ← C1", () => {
    g.setFormula("B1", new Set(["A1"]));
    g.setFormula("B2", new Set(["A1"]));
    g.setFormula("C1", new Set(["B1", "B2"]));
    const r = g.affected(["A1"]);
    expect(r.cycle).toBeNull();
    // B1 and B2 can be in any order, but both before C1.
    const b1Idx = r.order.indexOf("B1");
    const b2Idx = r.order.indexOf("B2");
    const c1Idx = r.order.indexOf("C1");
    expect(b1Idx).toBeGreaterThanOrEqual(0);
    expect(b2Idx).toBeGreaterThanOrEqual(0);
    expect(c1Idx).toBeGreaterThan(b1Idx);
    expect(c1Idx).toBeGreaterThan(b2Idx);
  });

  it("a ref with no dependents → empty order", () => {
    g.setFormula("B1", new Set(["A1"]));
    expect(g.affected(["Z9"]).order).toEqual([]);
  });

  it("multiple changed refs → union of their dependents in topo order", () => {
    g.setFormula("C1", new Set(["A1"]));
    g.setFormula("C2", new Set(["A2"]));
    g.setFormula("D1", new Set(["C1", "C2"]));
    const r = g.affected(["A1", "A2"]);
    expect(r.cycle).toBeNull();
    // D1 must come after both C1 and C2.
    const idxC1 = r.order.indexOf("C1");
    const idxC2 = r.order.indexOf("C2");
    const idxD1 = r.order.indexOf("D1");
    expect(idxD1).toBeGreaterThan(idxC1);
    expect(idxD1).toBeGreaterThan(idxC2);
  });

  it("source refs themselves are NOT in the affected order", () => {
    // The graph reports what needs RECOMPUTE — the source is whatever
    // the caller just wrote, so re-evaluating it would be redundant.
    g.setFormula("B1", new Set(["A1"]));
    const r = g.affected(["A1"]);
    expect(r.order).not.toContain("A1");
  });
});

describe("FormulaGraph · cycle detection (#CIRC! seeds)", () => {
  it("direct self-reference: A1 → A1", () => {
    g.setFormula("A1", new Set(["A1"]));
    const r = g.affected(["A1"]);
    expect(r.cycle).not.toBeNull();
    expect(r.cycle).toContain("A1");
  });

  it("2-node cycle: A1 ↔ B1", () => {
    g.setFormula("A1", new Set(["B1"]));
    g.setFormula("B1", new Set(["A1"]));
    const r = g.affected(["A1"]);
    expect(r.cycle).not.toBeNull();
    expect(r.cycle!.sort()).toEqual(["A1", "B1"]);
  });

  it("3-node cycle: A1 → B1 → C1 → A1", () => {
    g.setFormula("A1", new Set(["C1"]));
    g.setFormula("B1", new Set(["A1"]));
    g.setFormula("C1", new Set(["B1"]));
    const r = g.affected(["A1"]);
    expect(r.cycle).not.toBeNull();
    expect(r.cycle!.sort()).toEqual(["A1", "B1", "C1"]);
  });

  it("a healthy chain with an unrelated cycle elsewhere: cycle is reported but order still returned for non-cycle paths", () => {
    g.setFormula("B1", new Set(["A1"]));      // healthy
    g.setFormula("X1", new Set(["Y1"]));      // cycle leg 1
    g.setFormula("Y1", new Set(["X1"]));      // cycle leg 2
    const r = g.affected(["A1"]);
    // The cycle wasn't reachable from A1 → no cycle on this recompute.
    expect(r.cycle).toBeNull();
    expect(r.order).toEqual(["B1"]);
    // But following the cycle leg directly should detect it.
    const rCycle = g.affected(["X1"]);
    expect(rCycle.cycle).not.toBeNull();
    expect(rCycle.cycle!.sort()).toEqual(["X1", "Y1"]);
  });
});

describe("FormulaGraph · isolation", () => {
  it("formulas in separate graph instances don't see each other", () => {
    const g2 = new FormulaGraph();
    g.setFormula("A1", new Set(["B1"]));
    expect(g2.has("A1")).toBe(false);
    expect(g2.dependents("B1").size).toBe(0);
  });
});
