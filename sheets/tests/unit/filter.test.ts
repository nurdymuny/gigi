import { describe, expect, it } from "vitest";
import {
  applyFilters,
  kappaClass,
  type Filter,
} from "../../src/lib/filter";

interface Row {
  id: string;
  name: string;
  amount: number;
  rail: string;
  [key: string]: unknown;
}

const rows: Row[] = [
  { id: "r1", name: "Alice", amount: 100, rail: "SWIFT" },
  { id: "r2", name: "BOB",   amount: 250, rail: "ACH" },
  { id: "r3", name: "Carol", amount: 150, rail: "SWIFT" },
  { id: "r4", name: "Dave",  amount: 999, rail: "RTP" },
];

const ids = (rs: Row[]): string[] => rs.map((r) => r.id);

describe("filter · text", () => {
  it("contains (case-insensitive)", () => {
    const f: Filter = { kind: "text", column: "name", op: "contains", value: "a" };
    expect(ids(applyFilters(rows, [f]))).toEqual(["r1", "r3", "r4"]);
  });

  it("equals (case-insensitive)", () => {
    const f: Filter = { kind: "text", column: "rail", op: "equals", value: "swift" };
    expect(ids(applyFilters(rows, [f]))).toEqual(["r1", "r3"]);
  });

  it("startsWith / endsWith", () => {
    const sw: Filter = { kind: "text", column: "name", op: "startsWith", value: "B" };
    expect(ids(applyFilters(rows, [sw]))).toEqual(["r2"]);
    const ew: Filter = { kind: "text", column: "name", op: "endsWith", value: "e" };
    expect(ids(applyFilters(rows, [ew]))).toEqual(["r1", "r4"]);
  });
});

describe("filter · numeric range", () => {
  it("inclusive on both ends", () => {
    const f: Filter = { kind: "range", column: "amount", min: 100, max: 250 };
    expect(ids(applyFilters(rows, [f]))).toEqual(["r1", "r2", "r3"]);
  });

  it("greater than", () => {
    const f: Filter = { kind: "range", column: "amount", min: 200 };
    expect(ids(applyFilters(rows, [f]))).toEqual(["r2", "r4"]);
  });

  it("less than", () => {
    const f: Filter = { kind: "range", column: "amount", max: 150 };
    expect(ids(applyFilters(rows, [f]))).toEqual(["r1", "r3"]);
  });

  it("excludes non-numeric values", () => {
    const rs = [...rows, { id: "r5", name: "weird", amount: NaN, rail: "X" }];
    const f: Filter = { kind: "range", column: "amount", min: 0 };
    expect(applyFilters(rs, [f]).find((r) => r.id === "r5")).toBeUndefined();
  });
});

describe("filter · sameness", () => {
  it("keeps rows with sameness ≥ τ to pivot", () => {
    const sim = new Map<string, number>([
      ["r1", 0.95],
      ["r2", 0.40],
      ["r3", 0.88],
      ["r4", 0.20],
    ]);
    const f: Filter = { kind: "sameness", pivot: "r1", threshold: 0.85 };
    const out = applyFilters(rows, [f], {
      keyField: "id",
      samenessTo: (k) => sim.get(k) ?? 0,
    });
    expect(ids(out)).toEqual(["r1", "r3"]);
  });

  it("is inclusive at the threshold value (S = τ stays in)", () => {
    const sim = new Map<string, number>([
      ["r1", 0.85],
      ["r2", 0.849],
    ]);
    const f: Filter = { kind: "sameness", pivot: "anything", threshold: 0.85 };
    const out = applyFilters([rows[0], rows[1]], [f], {
      keyField: "id",
      samenessTo: (k) => sim.get(k) ?? 0,
    });
    expect(ids(out)).toEqual(["r1"]);
  });

  it("becomes a no-op when no samenessTo is wired", () => {
    const f: Filter = { kind: "sameness", pivot: "r1", threshold: 0.85 };
    expect(ids(applyFilters(rows, [f], { keyField: "id" }))).toEqual([
      "r1",
      "r2",
      "r3",
      "r4",
    ]);
  });
});

describe("filter · κ-class", () => {
  it("kappaClass bucket boundaries", () => {
    expect(kappaClass(0.05)).toBe("healthy");
    expect(kappaClass(0.099)).toBe("healthy");
    expect(kappaClass(0.10)).toBe("drift");
    expect(kappaClass(0.29)).toBe("drift");
    expect(kappaClass(0.30)).toBe("anomaly");
    expect(kappaClass(0.95)).toBe("anomaly");
  });

  it("filter by κ-class keeps only matching rows", () => {
    const kappa = new Map<string, number>([
      ["r1", 0.02],
      ["r2", 0.15],
      ["r3", 0.40],
      ["r4", 0.50],
    ]);
    const f: Filter = { kind: "kappa", classes: ["anomaly"] };
    const out = applyFilters(rows, [f], {
      keyField: "id",
      kappa: (k) => kappa.get(k) ?? 0,
    });
    expect(ids(out)).toEqual(["r3", "r4"]);
  });

  it("filter accepts multiple classes (union)", () => {
    const kappa = new Map<string, number>([
      ["r1", 0.02],
      ["r2", 0.15],
      ["r3", 0.40],
      ["r4", 0.50],
    ]);
    const f: Filter = { kind: "kappa", classes: ["drift", "anomaly"] };
    const out = applyFilters(rows, [f], {
      keyField: "id",
      kappa: (k) => kappa.get(k) ?? 0,
    });
    expect(ids(out)).toEqual(["r2", "r3", "r4"]);
  });
});

describe("filter · stacking", () => {
  it("AND-combines multiple filters", () => {
    const swift: Filter = { kind: "text", column: "rail", op: "equals", value: "swift" };
    const big: Filter = { kind: "range", column: "amount", min: 120 };
    const out = applyFilters(rows, [swift, big]);
    expect(ids(out)).toEqual(["r3"]);
  });

  it("empty filter list is identity", () => {
    expect(ids(applyFilters(rows, []))).toEqual(["r1", "r2", "r3", "r4"]);
  });
});
