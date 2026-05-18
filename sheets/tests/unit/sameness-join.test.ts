import { describe, expect, it } from "vitest";
import { samenessJoin } from "../../src/lib/sameness-join";

interface Row {
  key: string;
  data: string;
  [k: string]: unknown;
}

describe("sameness-join · exact-key parity", () => {
  it("recovers all matches when keys are clean (parity with INNER JOIN)", () => {
    const left: Row[] = [
      { key: "A1", data: "left-A" },
      { key: "B2", data: "left-B" },
    ];
    const right: Row[] = [
      { key: "A1", data: "right-A" },
      { key: "B2", data: "right-B" },
    ];
    const out = samenessJoin(left, right, "key", {
      threshold: 0.85,
      // For this baseline test, sameness is 1 iff canonical keys match.
      samenessOf: (a, b) => (a === b ? 1 : 0),
    });
    expect(out).toHaveLength(2);
    expect(out.map((p) => p.left.data).sort()).toEqual(["left-A", "left-B"]);
  });
});

describe("sameness-join · canonical refs", () => {
  it("matches keys that differ only in punctuation / case", () => {
    const left: Row[] = [
      { key: "INV-2026-04823", data: "chase-1" },
      { key: "INV-2026-04824", data: "chase-2" },
    ];
    const right: Row[] = [
      { key: "INV 2026 04823", data: "qb-1" },
      { key: "INV2026-04824",  data: "qb-2" },
    ];
    const out = samenessJoin(left, right, "key", {
      threshold: 0.85,
      // Use the canonical-match semantics directly.
      useCanonical: true,
    });
    expect(out).toHaveLength(2);
    const pair1 = out.find((p) => p.left.data === "chase-1");
    expect(pair1?.right.data).toBe("qb-1");
  });

  it("survives typos that don't change canonical form", () => {
    const left: Row[] = [{ key: "supplier payment", data: "L" }];
    const right: Row[] = [{ key: "supplier-payment", data: "R" }];
    const out = samenessJoin(left, right, "key", {
      threshold: 0.85,
      useCanonical: true,
    });
    expect(out).toHaveLength(1);
    expect(out[0].sameness).toBe(1);
  });
});

describe("sameness-join · threshold", () => {
  it("does not match below threshold", () => {
    const left: Row[] = [{ key: "K1", data: "L" }];
    const right: Row[] = [{ key: "K2", data: "R" }];
    const out = samenessJoin(left, right, "key", {
      threshold: 0.85,
      samenessOf: () => 0.5,
    });
    expect(out).toHaveLength(0);
  });

  it("at the threshold, is inclusive (S = τ matches)", () => {
    const left: Row[] = [{ key: "K", data: "L" }];
    const right: Row[] = [{ key: "K", data: "R" }];
    const out = samenessJoin(left, right, "key", {
      threshold: 0.85,
      samenessOf: () => 0.85,
    });
    expect(out).toHaveLength(1);
  });
});

describe("sameness-join · orphans", () => {
  it("emits left-only orphans when requested", () => {
    const left: Row[] = [
      { key: "A", data: "L-A" },
      { key: "B", data: "L-B" },
    ];
    const right: Row[] = [{ key: "A", data: "R-A" }];
    const result = samenessJoin(left, right, "key", {
      threshold: 0.85,
      samenessOf: (a, b) => (a === b ? 1 : 0),
      includeOrphans: true,
    });
    // result becomes array of pairs PLUS arrays of orphans on .orphansLeft / .orphansRight
    expect(result).toHaveLength(1);
    expect(result.orphansLeft).toEqual([{ key: "B", data: "L-B" }]);
    expect(result.orphansRight).toEqual([]);
  });
});
