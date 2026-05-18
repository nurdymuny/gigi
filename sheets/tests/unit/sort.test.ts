import { describe, expect, it } from "vitest";
import { sortRows, type SortSpec } from "../../src/lib/sort";

interface Row {
  id: string;
  name: string;
  amount: number;
  [key: string]: unknown;
}

const rows: Row[] = [
  { id: "r1", name: "Charlie", amount: 250 },
  { id: "r2", name: "alpha", amount: 100 },
  { id: "r3", name: "Bravo", amount: 300 },
  { id: "r4", name: "delta", amount: 50 },
];

function ids(rs: Row[]): string[] {
  return rs.map((r) => r.id);
}

describe("sort · column asc/desc", () => {
  it("ascending lexicographic (case-insensitive)", () => {
    const spec: SortSpec = { mode: "column", column: "name", direction: "asc" };
    const out = sortRows(rows, spec, () => ({ row: () => null, kappa: () => 0 }));
    expect(ids(out)).toEqual(["r2", "r3", "r1", "r4"]);
  });

  it("descending lexicographic (case-insensitive)", () => {
    const spec: SortSpec = { mode: "column", column: "name", direction: "desc" };
    const out = sortRows(rows, spec, () => ({ row: () => null, kappa: () => 0 }));
    expect(ids(out)).toEqual(["r4", "r1", "r3", "r2"]);
  });

  it("numeric ascending", () => {
    const spec: SortSpec = { mode: "column", column: "amount", direction: "asc" };
    const out = sortRows(rows, spec, () => ({ row: () => null, kappa: () => 0 }));
    expect(ids(out)).toEqual(["r4", "r2", "r1", "r3"]);
  });

  it("numeric descending", () => {
    const spec: SortSpec = { mode: "column", column: "amount", direction: "desc" };
    const out = sortRows(rows, spec, () => ({ row: () => null, kappa: () => 0 }));
    expect(ids(out)).toEqual(["r3", "r1", "r2", "r4"]);
  });

  it("handles null / undefined by sinking them to the end", () => {
    const rs = [
      { id: "a", name: null as unknown as string, amount: 1 },
      { id: "b", name: "x", amount: 2 },
    ];
    const spec: SortSpec = { mode: "column", column: "name", direction: "asc" };
    const out = sortRows(rs, spec, () => ({ row: () => null, kappa: () => 0 }));
    expect(ids(out)).toEqual(["b", "a"]);
  });

  it("is stable: rows with equal keys retain their input order", () => {
    const rs = [
      { id: "1", name: "x", amount: 1 },
      { id: "2", name: "x", amount: 2 },
      { id: "3", name: "x", amount: 3 },
    ];
    const spec: SortSpec = { mode: "column", column: "name", direction: "asc" };
    const out = sortRows(rs, spec, () => ({ row: () => null, kappa: () => 0 }));
    expect(ids(out)).toEqual(["1", "2", "3"]);
  });
});

describe("sort · κ-rank", () => {
  it("orders rows by κ desc (highest curvature first)", () => {
    const kappa = new Map<string, number>([
      ["r1", 0.1],
      ["r2", 0.5],
      ["r3", 0.03],
      ["r4", 0.25],
    ]);
    const spec: SortSpec = { mode: "kappa", direction: "desc" };
    const out = sortRows(
      rows,
      spec,
      () => ({ row: () => null, kappa: (key) => kappa.get(key) ?? 0 }),
      "id",
    );
    expect(ids(out)).toEqual(["r2", "r4", "r1", "r3"]);
  });

  it("orders rows by κ asc (most-typical first)", () => {
    const kappa = new Map<string, number>([
      ["r1", 0.1],
      ["r2", 0.5],
      ["r3", 0.03],
      ["r4", 0.25],
    ]);
    const spec: SortSpec = { mode: "kappa", direction: "asc" };
    const out = sortRows(
      rows,
      spec,
      () => ({ row: () => null, kappa: (key) => kappa.get(key) ?? 0 }),
      "id",
    );
    expect(ids(out)).toEqual(["r3", "r1", "r4", "r2"]);
  });
});

describe("sort · sameness-pivot", () => {
  it("orders by sameness to pivot desc (most-similar first)", () => {
    // Pivot = r2. Set sameness so r3 is closest after pivot, then r1, then r4.
    const sims = new Map<string, number>([
      ["r1", 0.7],
      ["r2", 1.0],
      ["r3", 0.9],
      ["r4", 0.3],
    ]);
    const spec: SortSpec = {
      mode: "sameness",
      pivot: "r2",
      direction: "desc",
    };
    const out = sortRows(
      rows,
      spec,
      () => ({
        row: () => null,
        kappa: () => 0,
        samenessTo: (key) => sims.get(key) ?? 0,
      }),
      "id",
    );
    expect(ids(out)).toEqual(["r2", "r3", "r1", "r4"]);
  });

  it("returns the input order unchanged when pivot not provided", () => {
    const spec: SortSpec = {
      mode: "sameness",
      pivot: "",
      direction: "desc",
    };
    const out = sortRows(rows, spec, () => ({ row: () => null, kappa: () => 0 }), "id");
    expect(ids(out)).toEqual(["r1", "r2", "r3", "r4"]);
  });
});

describe("sort · null spec (no-op)", () => {
  it("returns the input array unchanged", () => {
    const out = sortRows(rows, null, () => ({ row: () => null, kappa: () => 0 }));
    expect(ids(out)).toEqual(["r1", "r2", "r3", "r4"]);
  });

  it("does not mutate the input array", () => {
    const copy = rows.slice();
    sortRows(rows, { mode: "column", column: "amount", direction: "desc" }, () => ({
      row: () => null,
      kappa: () => 0,
    }));
    expect(rows).toEqual(copy);
  });
});
