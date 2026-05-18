import { beforeEach, describe, expect, it } from "vitest";
import { FormulaEngine } from "../../src/lib/formula-engine";

/**
 * Phase 2.D · end-to-end recompute cascade + #CIRC! propagation.
 *
 * `FormulaEngine` composes the three Phase-2 primitives:
 *
 *   evaluate()        eval a single formula given a FormulaContext
 *   collectDeps()     extract a formula's static dep set
 *   FormulaGraph      cascade order + cycle detection
 *
 * App.tsx will mirror this pattern at the bundle level (with the
 * sidecar + bundle row store wired in). These tests pin the composition
 * itself so any future re-wire keeps the invariants tight.
 */

let e: FormulaEngine;
beforeEach(() => {
  e = new FormulaEngine();
});

describe("FormulaEngine · plain values", () => {
  it("set and get a number", () => {
    e.setValue("A1", 42);
    expect(e.get("A1")).toBe(42);
  });

  it("an unset cell reads as null", () => {
    expect(e.get("Z9")).toBeNull();
  });
});

describe("FormulaEngine · formulas evaluate at set time", () => {
  it("=A1+A2 reads current cell values", () => {
    e.setValue("A1", 3);
    e.setValue("A2", 4);
    e.setFormula("B1", "=A1+A2");
    expect(e.get("B1")).toBe(7);
  });

  it("the resolved value is what other formulas see, not the formula text", () => {
    e.setValue("A1", 10);
    e.setFormula("B1", "=A1*2");
    e.setFormula("C1", "=B1+1");
    expect(e.get("C1")).toBe(21);
  });
});

describe("FormulaEngine · recompute cascade", () => {
  it("changing a source cell cascades to dependent formulas", () => {
    e.setValue("A1", 5);
    e.setFormula("B1", "=A1*2"); // 10
    e.setFormula("C1", "=B1+1"); // 11
    expect(e.get("B1")).toBe(10);
    expect(e.get("C1")).toBe(11);
    e.setValue("A1", 7);
    expect(e.get("B1")).toBe(14);
    expect(e.get("C1")).toBe(15);
  });

  it("diamond: A1 ← B1, A1 ← C1, B1+C1 ← D1", () => {
    e.setValue("A1", 2);
    e.setFormula("B1", "=A1*10");  // 20
    e.setFormula("C1", "=A1*100"); // 200
    e.setFormula("D1", "=B1+C1");  // 220
    expect(e.get("D1")).toBe(220);
    e.setValue("A1", 3);
    expect(e.get("B1")).toBe(30);
    expect(e.get("C1")).toBe(300);
    expect(e.get("D1")).toBe(330);
  });

  it("replacing a formula propagates the new dep set", () => {
    e.setValue("A1", 1);
    e.setValue("A2", 100);
    e.setFormula("B1", "=A1");
    expect(e.get("B1")).toBe(1);
    // Swap to read A2 — old back-edge from A1 must vanish.
    e.setFormula("B1", "=A2");
    expect(e.get("B1")).toBe(100);
    // Changing A1 now should NOT re-evaluate B1 (verify by setting A2 to
    // 50 and ensuring B1 changes from that, but A1 changes have no
    // effect on B1's value).
    e.setValue("A1", 999);
    expect(e.get("B1")).toBe(100);
    e.setValue("A2", 50);
    expect(e.get("B1")).toBe(50);
  });

  it("aggregate cascades when ANY range member changes", () => {
    for (let i = 1; i <= 5; i++) e.setValue(`A${i}`, i);
    e.setFormula("B1", "=SUM(A1:A5)"); // 15
    expect(e.get("B1")).toBe(15);
    e.setValue("A3", 100);
    expect(e.get("B1")).toBe(112);
  });
});

describe("FormulaEngine · clearFormula", () => {
  it("removes the formula but preserves the last evaluated value", () => {
    e.setValue("A1", 5);
    e.setFormula("B1", "=A1*2");
    expect(e.get("B1")).toBe(10);
    e.clearFormula("B1");
    expect(e.getFormula("B1")).toBeNull();
    // Value preserved — caller can read 10 until they overwrite it.
    expect(e.get("B1")).toBe(10);
    // Changing A1 no longer touches B1.
    e.setValue("A1", 999);
    expect(e.get("B1")).toBe(10);
  });
});

describe("FormulaEngine · #CIRC! detection", () => {
  it("self-reference writes #CIRC! to the cell", () => {
    e.setValue("A1", 1);
    e.setFormula("A1", "=A1+1");
    expect(e.get("A1")).toBe("#CIRC!");
  });

  it("two-node cycle: both cells get #CIRC!", () => {
    // Order of setFormula matters — when we install A1's formula first,
    // B1 isn't a formula yet, so no cycle. Adding B1 closes the loop.
    e.setFormula("A1", "=B1+1");
    e.setFormula("B1", "=A1+1");
    expect(e.get("A1")).toBe("#CIRC!");
    expect(e.get("B1")).toBe("#CIRC!");
  });

  it("downstream of a cycle: aggregate poisoning kicks in", () => {
    e.setFormula("A1", "=B1");
    e.setFormula("B1", "=A1");
    // Cycle: both A1 and B1 == #CIRC!.
    expect(e.get("A1")).toBe("#CIRC!");
    expect(e.get("B1")).toBe("#CIRC!");
    // C1 reads A1, which now holds the sentinel — aggregate poisoning
    // (Phase 1.C) propagates the error.
    e.setFormula("C1", "=A1+1");
    expect(e.get("C1")).toBe("#CIRC!");
  });

  it("breaking the cycle clears the #CIRC! marker", () => {
    e.setFormula("A1", "=B1+1");
    e.setFormula("B1", "=A1+1");
    expect(e.get("A1")).toBe("#CIRC!");
    // Replace B1 with a plain value — A1's deps still include B1, but
    // B1 is no longer a formula, so the cycle is broken.
    e.setValue("B1", 10);
    expect(e.get("A1")).toBe(11);
  });
});

describe("FormulaEngine · field refs", () => {
  it("supports SUM(temperature) once a resolver is wired", () => {
    const eng = new FormulaEngine({
      resolveField: (name) =>
        name === "temperature" ? ["A1", "A2", "A3"] : null,
    });
    eng.setValue("A1", 1);
    eng.setValue("A2", 2);
    eng.setValue("A3", 3);
    eng.setFormula("B1", "=SUM(temperature)");
    expect(eng.get("B1")).toBe(6);
    eng.setValue("A2", 20);
    expect(eng.get("B1")).toBe(24);
  });
});
