import { describe, expect, it } from "vitest";
import { parseCsv } from "../../src/lib/csv";
import { DEMO_DATASETS } from "../../src/lib/demo-datasets";
import { findWorkflow } from "../../src/lib/prism-workflows";

/**
 * Smoke test: after switching Books to canonical sameness-join, the
 * 3 planted amount conflicts + 2 orphans still surface in the Chase ↔
 * QuickBooks demo bundles.
 */
describe("Books · planted-breaks smoke", () => {
  it("finds 3 amount conflicts and 2 orphans on the Chase/QB demo", () => {
    const chase = DEMO_DATASETS.find((d) => d.id === "chase_statements")!;
    const qb = DEMO_DATASETS.find((d) => d.id === "quickbooks_ledger")!;
    const parsedA = parseCsv(chase.csv);
    const parsedB = parseCsv(qb.csv);

    const schemaA = {
      name: chase.id,
      base_fields: [{ name: parsedA.headers[0], type: parsedA.types[0] }],
      fiber_fields: parsedA.headers.slice(1).map((h, i) => ({
        name: h,
        type: parsedA.types[i + 1],
      })),
      indexed_fields: [parsedA.headers[0]],
      records: parsedA.rows.length,
      storage_mode: "mmap",
    } as const;

    const books = findWorkflow("books")!;
    const result = books.run({
      schema: schemaA as never,
      rows: parsedA.rows,
      kappaMap: new Map(),
      secondaryRows: parsedB.rows,
      secondaryName: qb.id,
    });

    // 3 planted amount conflicts: refs 003, 010, 020 (28000↔28500, 45000↔44500, 15800↔15900)
    const conflictStat = result.stats.find((s) => s.label === "Conflicts");
    expect(conflictStat?.value).toBe("3");

    // 2 planted orphans: CHK-202604-035 only in Chase, CHK-202604-036 only in QB.
    const onlyA = result.stats.find((s) => s.label.startsWith("Only in chase_"));
    const onlyB = result.stats.find((s) => s.label.startsWith("Only in quickbooks_"));
    expect(onlyA?.value).toBe("1");
    expect(onlyB?.value).toBe("1");
  });
});
