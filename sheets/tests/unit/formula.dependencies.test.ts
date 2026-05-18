import { describe, expect, it } from "vitest";
import { collectDeps } from "../../src/lib/formula";

/**
 * Phase 2.B · static dependency extraction.
 *
 * `collectDeps(formula, ctx)` returns the **set of A1 refs** that a
 * formula reads. The result is computed statically at parse time and
 * stored alongside the formula; when any source cell changes, the
 * recompute engine uses the reverse-index of this set to figure out
 * which formulas need re-evaluation.
 *
 * Per FORMULAS_SPEC §"Recompute model": "For v1, the dependency graph
 * is computed at parse time and stored alongside the formula cell. No
 * incremental graph updates — when a formula changes, its old
 * dependencies are removed and the new set inserted."
 */

const resolveField = (name: string) => {
  if (name === "temperature") return ["A1", "A2", "A3", "A4", "A5"];
  if (name === "status") return ["B1", "B2", "B3", "B4", "B5"];
  return null;
};

function depsOf(formula: string): string[] {
  const r = collectDeps(formula, { resolveField });
  if (r.error) throw new Error(`collectDeps error: ${r.error} for ${formula}`);
  // Tests assert against sorted arrays for determinism. The Set itself
  // is order-irrelevant.
  return [...r.deps].sort();
}

describe("collectDeps · cell + range refs", () => {
  it("=A1 → {A1}", () => {
    expect(depsOf("=A1")).toEqual(["A1"]);
  });

  it("=A1 + B2 → {A1, B2}", () => {
    expect(depsOf("=A1 + B2")).toEqual(["A1", "B2"]);
  });

  it("=SUM(A1:A3) → {A1, A2, A3}", () => {
    expect(depsOf("=SUM(A1:A3)")).toEqual(["A1", "A2", "A3"]);
  });

  it("=SUM(A1:A3) + B1 → {A1, A2, A3, B1}", () => {
    expect(depsOf("=SUM(A1:A3) + B1")).toEqual(["A1", "A2", "A3", "B1"]);
  });

  it("a literal-only formula has no deps", () => {
    expect(depsOf("=1 + 2 * 3")).toEqual([]);
  });

  it("repeated refs are de-duped", () => {
    expect(depsOf("=A1 + A1 + A1")).toEqual(["A1"]);
  });
});

describe("collectDeps · field refs (named-column)", () => {
  it("=SUM(temperature) → A1..A5", () => {
    expect(depsOf("=SUM(temperature)")).toEqual(["A1", "A2", "A3", "A4", "A5"]);
  });

  it("=temperature[3] → A3", () => {
    expect(depsOf("=temperature[3]")).toEqual(["A3"]);
  });

  it("=SUM(temperature[2:4]) → A2, A3, A4", () => {
    expect(depsOf("=SUM(temperature[2:4])")).toEqual(["A2", "A3", "A4"]);
  });

  it("=temperature[3] + status[3] → A3, B3", () => {
    expect(depsOf("=temperature[3] + status[3]")).toEqual(["A3", "B3"]);
  });
});

describe("collectDeps · GIGI primitives", () => {
  it("=SAME(A1, A2) → A1, A2", () => {
    expect(depsOf("=SAME(A1, A2)")).toEqual(["A1", "A2"]);
  });

  it("=K(A1) → A1", () => {
    expect(depsOf("=K(A1)")).toEqual(["A1"]);
  });
});

describe("collectDeps · IF / *IF family", () => {
  it("=IF(A1 > 0, B1, B2) → A1, B1, B2", () => {
    // Both branches are tracked — recompute fires whichever path the
    // current value takes.
    expect(depsOf("=IF(A1 > 0, B1, B2)")).toEqual(["A1", "B1", "B2"]);
  });

  it('=SUMIF(A1:A3, ">5", B1:B3) → A1..A3, B1..B3', () => {
    expect(depsOf('=SUMIF(A1:A3, ">5", B1:B3)')).toEqual([
      "A1", "A2", "A3", "B1", "B2", "B3",
    ]);
  });
});

describe("collectDeps · errors", () => {
  it("syntax error → #ERROR! and an empty dep set", () => {
    const r = collectDeps("=A1 +", { resolveField });
    expect(r.error).toBe("#ERROR!");
    expect(r.deps.size).toBe(0);
  });

  it("unknown field → #NAME!", () => {
    const r = collectDeps("=SUM(missing)", { resolveField });
    expect(r.error).toBe("#NAME!");
  });

  it("not a formula → empty (a plain value has no deps)", () => {
    const r = collectDeps("plain value", { resolveField });
    expect(r.deps.size).toBe(0);
    expect(r.error).toBeNull();
  });
});

describe("collectDeps · dynamic row index falls back to full-column dep", () => {
  it("=temperature[A1] depends on A1 AND the full temperature column", () => {
    // The row index is itself a cell ref → unsolvable statically. The
    // safe (over-)approximation is to treat it as the full field range
    // (the row could be any of them), plus the index's own deps.
    const deps = depsOf("=temperature[A1]");
    expect(deps).toContain("A1");
    expect(deps).toContain("A3");
    expect(deps).toContain("A5");
  });
});
