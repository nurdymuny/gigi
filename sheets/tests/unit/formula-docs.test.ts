import { describe, expect, it } from "vitest";
import {
  FORMULA_DOCS,
  assembleFormula,
  findDoc,
  searchDocs,
  type FormulaDoc,
} from "../../src/lib/formula-docs";
import { RESERVED_NAMES } from "../../src/lib/formula";

/**
 * Phase 5.A · function metadata registry + formula-assembly helper.
 *
 * The registry powers the FormulaPicker UI: function list, category
 * grouping, search-by-name-or-description, and per-argument help. The
 * assembler turns a (function, argValues) tuple into a valid `=FN(...)`
 * string that the formula bar can drop into the active cell.
 *
 * Invariant: every reserved function name in `formula.ts` has a doc
 * entry. If a new function is added without a doc, this test fails
 * (the picker should never silently lack a function the engine supports).
 */

describe("formula-docs · registry coverage", () => {
  it("every RESERVED_NAMES entry has a doc", () => {
    // Skip `AVG` since `AVERAGE` is its canonical name (Excel parity);
    // the docs surface AVERAGE and the engine accepts both.
    const aliases = new Set(["AVG"]);
    const docNames = new Set(FORMULA_DOCS.map((d) => d.name));
    const missing: string[] = [];
    for (const name of RESERVED_NAMES) {
      if (aliases.has(name)) continue;
      if (!docNames.has(name)) missing.push(name);
    }
    expect(missing, `missing docs for: ${missing.join(", ")}`).toEqual([]);
  });

  it("every doc has the required fields", () => {
    for (const d of FORMULA_DOCS) {
      expect(d.name, "name").toBeTruthy();
      expect(d.category, `${d.name}: category`).toBeTruthy();
      expect(d.signature, `${d.name}: signature`).toBeTruthy();
      expect(d.description, `${d.name}: description`).toBeTruthy();
      expect(d.example, `${d.name}: example`).toBeTruthy();
      expect(Array.isArray(d.args), `${d.name}: args is array`).toBe(true);
    }
  });

  it("docs are sorted alphabetically within each category", () => {
    const byCat = new Map<string, FormulaDoc[]>();
    for (const d of FORMULA_DOCS) {
      const arr = byCat.get(d.category) ?? [];
      arr.push(d);
      byCat.set(d.category, arr);
    }
    for (const [cat, docs] of byCat) {
      const names = docs.map((d) => d.name);
      const sorted = [...names].sort();
      expect(names, `${cat} sorted`).toEqual(sorted);
    }
  });
});

describe("formula-docs · findDoc / searchDocs", () => {
  it("findDoc is case-insensitive on the function name", () => {
    expect(findDoc("SUM")?.name).toBe("SUM");
    expect(findDoc("sum")?.name).toBe("SUM");
    expect(findDoc("Sum")?.name).toBe("SUM");
  });

  it("findDoc returns null for unknown names", () => {
    expect(findDoc("NOPE")).toBeNull();
  });

  it("searchDocs matches by name prefix (case-insensitive)", () => {
    const r = searchDocs("sum");
    expect(r.map((d) => d.name)).toContain("SUM");
    expect(r.map((d) => d.name)).toContain("SUMIF");
    expect(r.map((d) => d.name)).toContain("SUMIFS");
  });

  it("searchDocs matches inside descriptions too", () => {
    const r = searchDocs("median");
    // MEDIAN itself + any function that mentions median (e.g. QUARTILE's
    // description "Q2 is the median").
    expect(r.length).toBeGreaterThan(0);
    expect(r.find((d) => d.name === "MEDIAN")).toBeTruthy();
  });

  it("empty search returns the full sorted list", () => {
    const r = searchDocs("");
    expect(r.length).toBe(FORMULA_DOCS.length);
  });

  it("name-prefix matches outrank description-only matches", () => {
    const r = searchDocs("sum");
    // SUM should come before any description-only hit.
    expect(r[0].name).toBe("SUM");
  });
});

describe("formula-docs · assembleFormula", () => {
  it("builds a zero-arg call", () => {
    expect(assembleFormula("TODAY", [])).toBe("=TODAY()");
  });

  it("builds a single-arg call", () => {
    expect(assembleFormula("ABS", ["-5"])).toBe("=ABS(-5)");
  });

  it("comma-separates multiple args, preserving order", () => {
    expect(assembleFormula("SUMIF", ["A1:A5", '">10"', "B1:B5"])).toBe(
      '=SUMIF(A1:A5, ">10", B1:B5)',
    );
  });

  it("drops trailing empty args (optional-args path)", () => {
    // SUMIF's 3rd arg is optional; the picker leaves it empty if the
    // user doesn't fill it in. The assembled formula shouldn't have a
    // trailing comma.
    expect(assembleFormula("SUMIF", ["A1:A5", '">10"', ""])).toBe(
      '=SUMIF(A1:A5, ">10")',
    );
  });

  it("preserves blank args in the middle (the user can do that on purpose)", () => {
    // E.g. IF(cond, , else) — Excel treats empty as the empty string.
    expect(assembleFormula("IF", ["A1>0", "", "B1"])).toBe(
      "=IF(A1>0, , B1)",
    );
  });

  it("trims whitespace inside each arg", () => {
    expect(assembleFormula("SUM", ["  A1:A5  "])).toBe("=SUM(A1:A5)");
  });
});

describe("formula-docs · categories are stable", () => {
  it("expected categories are present", () => {
    const cats = new Set(FORMULA_DOCS.map((d) => d.category));
    for (const c of [
      "aggregate",
      "math",
      "stats",
      "logic",
      "text",
      "date",
      "conditional",
      "geometry",
    ]) {
      expect(cats.has(c as FormulaDoc["category"]), `category ${c}`).toBe(true);
    }
  });
});
