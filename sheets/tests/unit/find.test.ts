import { describe, expect, it } from "vitest";
import {
  findInRows,
  replaceInRows,
  type FindSpec,
} from "../../src/lib/find";

interface Row {
  id: string;
  ref: string;
  status: string;
  [key: string]: unknown;
}

const rows: Row[] = [
  { id: "r1", ref: "INV-2026-04823", status: "settled" },
  { id: "r2", ref: "INV 2026 04823", status: "settled" },
  { id: "r3", ref: "INV2026-04823",  status: "pending" },
  { id: "r4", ref: "INV-2026-04824", status: "settled" },
  { id: "r5", ref: "Q1 dividend",    status: "settled" },
];

const ids = (rs: Row[]): string[] => rs.map((r) => r.id);

describe("find · exact", () => {
  it("substring match (case-insensitive)", () => {
    const spec: FindSpec = { mode: "exact", query: "INV-2026" };
    const out = findInRows(rows, spec, ["ref"]);
    expect(ids(out)).toEqual(["r1", "r4"]);
  });

  it("case-insensitive substring matches mixed casing", () => {
    const spec: FindSpec = { mode: "exact", query: "dividend" };
    const out = findInRows(rows, spec, ["ref"]);
    expect(ids(out)).toEqual(["r5"]);
  });

  it("empty query returns no rows", () => {
    const spec: FindSpec = { mode: "exact", query: "" };
    expect(findInRows(rows, spec, ["ref"])).toEqual([]);
  });
});

describe("find · canonical", () => {
  it("matches references that differ only in punctuation / casing", () => {
    const spec: FindSpec = { mode: "canonical", query: "INV-2026-04823" };
    const out = findInRows(rows, spec, ["ref"]);
    // r1, r2, r3 all canonicalize to "INV202604823".
    expect(ids(out)).toEqual(["r1", "r2", "r3"]);
  });

  it("does not match references that differ in content (different number)", () => {
    const spec: FindSpec = { mode: "canonical", query: "INV-2026-04823" };
    const out = findInRows(rows, spec, ["ref"]);
    expect(ids(out)).not.toContain("r4"); // 04824 ≠ 04823
  });
});

describe("find · sameness", () => {
  it("matches rows whose sameness to pivot clears τ", () => {
    const sim = new Map<string, number>([
      ["r1", 0.99],
      ["r2", 0.98],
      ["r3", 0.91],
      ["r4", 0.60],
      ["r5", 0.20],
    ]);
    const spec: FindSpec = { mode: "sameness", pivot: "r1", threshold: 0.85 };
    const out = findInRows(rows, spec, ["ref"], {
      keyField: "id",
      samenessTo: (k) => sim.get(k) ?? 0,
    });
    expect(ids(out)).toEqual(["r1", "r2", "r3"]);
  });

  it("returns [] when no samenessTo lookup is available", () => {
    const spec: FindSpec = { mode: "sameness", pivot: "r1", threshold: 0.85 };
    expect(findInRows(rows, spec, ["ref"], { keyField: "id" })).toEqual([]);
  });
});

describe("find · replace", () => {
  it("replaces exact substring across all matching cells", () => {
    const { rows: out, count } = replaceInRows(
      rows,
      { mode: "exact", query: "settled" },
      "complete",
      ["status"],
    );
    expect(count).toBe(4);
    expect(out.filter((r) => r.status === "complete")).toHaveLength(4);
    expect(out.find((r) => r.id === "r3")?.status).toBe("pending");
  });

  it("returns count 0 + identical row references when no matches", () => {
    const { rows: out, count } = replaceInRows(
      rows,
      { mode: "exact", query: "nope" },
      "x",
      ["status"],
    );
    expect(count).toBe(0);
    expect(out).toEqual(rows);
  });

  it("does not mutate the input rows", () => {
    const before = JSON.parse(JSON.stringify(rows));
    replaceInRows(rows, { mode: "exact", query: "settled" }, "complete", ["status"]);
    expect(rows).toEqual(before);
  });

  it("canonical replace updates the whole cell when canonical form matches", () => {
    // r1.ref = "INV-2026-04823"; canonical query matches → replace whole cell.
    const { rows: out, count } = replaceInRows(
      rows,
      { mode: "canonical", query: "INV-2026-04823" },
      "INV-NEW",
      ["ref"],
    );
    expect(count).toBe(3); // r1, r2, r3
    expect(out.find((r) => r.id === "r1")?.ref).toBe("INV-NEW");
    expect(out.find((r) => r.id === "r4")?.ref).toBe("INV-2026-04824"); // unchanged
  });
});
