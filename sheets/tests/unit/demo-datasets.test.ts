import { describe, expect, it } from "vitest";
import { DEMO_DATASETS, findDemo } from "../../src/lib/demo-datasets";
import { parseCsv } from "../../src/lib/csv";

describe("DEMO_DATASETS — structural integrity", () => {
  it("every demo has a slug-safe id", () => {
    for (const d of DEMO_DATASETS) {
      expect(d.id, `${d.title} id`).toMatch(/^[A-Za-z_][A-Za-z0-9_]*$/);
    }
  });

  it("every demo's record/field counts match the embedded CSV", () => {
    for (const d of DEMO_DATASETS) {
      const parsed = parseCsv(d.csv);
      expect(parsed.rows.length, `${d.id} row count`).toBe(d.records);
      expect(parsed.headers.length, `${d.id} field count`).toBe(d.fields);
    }
  });

  it("every demo names a key column that actually exists in the CSV header", () => {
    for (const d of DEMO_DATASETS) {
      const parsed = parseCsv(d.csv);
      expect(parsed.headers, `${d.id} key field`).toContain(d.suggestedKey);
    }
  });

  it("every demo names a cover column that exists and infers as categorical/text", () => {
    for (const d of DEMO_DATASETS) {
      const parsed = parseCsv(d.csv);
      const idx = parsed.headers.indexOf(d.suggestedCover);
      expect(idx, `${d.id} cover field index`).toBeGreaterThanOrEqual(0);
      const type = parsed.types[idx];
      expect(["categorical", "text"], `${d.id} cover field type`).toContain(type);
    }
  });

  it("every demo has at least 2 numeric columns (so Geometry renders)", () => {
    for (const d of DEMO_DATASETS) {
      const parsed = parseCsv(d.csv);
      const numerics = parsed.types.filter((t) => t === "numeric").length;
      expect(numerics, `${d.id} numeric column count`).toBeGreaterThanOrEqual(2);
    }
  });

  it("every demo key column has unique values across all rows", () => {
    for (const d of DEMO_DATASETS) {
      const parsed = parseCsv(d.csv);
      const keys = parsed.rows.map((r) => String(r[d.suggestedKey]));
      const unique = new Set(keys);
      expect(unique.size, `${d.id} duplicate keys: ${keys.length - unique.size}`).toBe(keys.length);
    }
  });
});

describe("DEMO_DATASETS — encryption metadata (where present)", () => {
  it("hospital_records ships with a PHI-shaped encryption map", () => {
    const hospital = DEMO_DATASETS.find((d) => d.id === "hospital_records");
    expect(hospital, "hospital_records demo").toBeDefined();
    expect(hospital?.encryption).toBeDefined();
    // Spot-check the three encryption modes are represented.
    const modes = new Set(Object.values(hospital!.encryption!));
    expect(modes.has("opaque")).toBe(true);
    expect(modes.has("indexed")).toBe(true);
    expect(modes.has("affine")).toBe(true);
  });

  it("every encryption-tagged field actually exists in the CSV header", () => {
    for (const d of DEMO_DATASETS) {
      if (!d.encryption) continue;
      const parsed = parseCsv(d.csv);
      for (const field of Object.keys(d.encryption)) {
        expect(parsed.headers, `${d.id}/${field}`).toContain(field);
      }
    }
  });

  it("every encryption value is one of opaque/indexed/affine", () => {
    for (const d of DEMO_DATASETS) {
      if (!d.encryption) continue;
      for (const [field, mode] of Object.entries(d.encryption)) {
        expect(["opaque", "indexed", "affine"], `${d.id}/${field}`).toContain(mode);
      }
    }
  });

  it("never marks the suggested cover field as encrypted (it'd break cohort grouping)", () => {
    for (const d of DEMO_DATASETS) {
      if (!d.encryption) continue;
      expect(
        d.encryption[d.suggestedCover],
        `${d.id} cover field "${d.suggestedCover}" is encrypted`,
      ).toBeUndefined();
    }
  });
});

describe("findDemo", () => {
  it("returns the matching dataset by id", () => {
    const d = findDemo("iris");
    expect(d).not.toBeNull();
    expect(d?.title).toMatch(/Iris/);
  });

  it("returns null for an unknown id", () => {
    expect(findDemo("not-a-demo")).toBeNull();
  });
});
