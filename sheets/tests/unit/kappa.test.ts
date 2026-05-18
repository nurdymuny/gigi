import { describe, expect, it } from "vitest";
import {
  DEFAULT_THRESHOLDS,
  computeCohortKappa,
  kappaClass,
  numericFiberFields,
  pickDefaultCoverField,
} from "../../src/lib/kappa";
import type { BundleSchema } from "../../src/lib/gigi-client";

/**
 * S2 acceptance tests for the κ kernel.
 *
 * These are pure-function tests — no DOM, no React. They pin both the
 * threshold table that drives the row-tinting overlay and the kernel
 * the engine will eventually serve over the wire (E-S1a).
 */

describe("kappaClass — threshold table", () => {
  const cases: Array<[number, string]> = [
    [0, "ok"],
    [0.5, "ok"],
    [0.79, "ok"],
    [0.8, "warn"],
    [1.5, "warn"],
    [1.99, "warn"],
    [2.0, "bad"],
    [4.2, "bad"],
    [100, "bad"],
  ];
  for (const [k, expected] of cases) {
    it(`κ=${k} → ${expected}`, () => {
      expect(kappaClass(k)).toBe(expected);
    });
  }

  it("negative κ falls back to 'ok'", () => {
    expect(kappaClass(-1)).toBe("ok");
  });

  it("NaN κ falls back to 'ok'", () => {
    expect(kappaClass(Number.NaN)).toBe("ok");
  });

  it("Infinity κ classifies as 'ok' (defensive — engine should never emit this)", () => {
    expect(kappaClass(Infinity)).toBe("ok");
  });

  it("custom thresholds shift the boundaries", () => {
    expect(kappaClass(1.0, { warn: 1.5, bad: 3.0 })).toBe("ok");
    expect(kappaClass(2.0, { warn: 1.5, bad: 3.0 })).toBe("warn");
    expect(kappaClass(3.0, { warn: 1.5, bad: 3.0 })).toBe("bad");
  });

  it("default thresholds are { warn: 0.8, bad: 2.0 }", () => {
    expect(DEFAULT_THRESHOLDS).toEqual({ warn: 0.8, bad: 2.0 });
  });
});

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "site_id", type: "categorical" },
    { name: "temp", type: "numeric" },
    { name: "humidity", type: "numeric" },
    { name: "operator", type: "text", encryption: "indexed" },
  ],
  indexed_fields: ["sensor_id", "site_id"],
  records: 4,
  storage_mode: "mmap",
};

describe("pickDefaultCoverField", () => {
  it("prefers the first categorical fiber field", () => {
    expect(pickDefaultCoverField(SCHEMA)).toBe("site_id");
  });

  it("falls back to the first non-encrypted text field if no categorical", () => {
    const schema: BundleSchema = {
      ...SCHEMA,
      fiber_fields: [
        { name: "temp", type: "numeric" },
        { name: "operator", type: "text", encryption: "indexed" }, // encrypted, skip
        { name: "label", type: "text" },
      ],
    };
    expect(pickDefaultCoverField(schema)).toBe("label");
  });

  it("falls back to the primary key when no usable fiber field exists", () => {
    const schema: BundleSchema = {
      ...SCHEMA,
      fiber_fields: [{ name: "temp", type: "numeric" }],
    };
    expect(pickDefaultCoverField(schema)).toBe("sensor_id");
  });
});

describe("numericFiberFields", () => {
  it("returns only non-encrypted numeric fiber fields", () => {
    expect(numericFiberFields(SCHEMA)).toEqual(["temp", "humidity"]);
  });
});

describe("computeCohortKappa", () => {
  it("returns empty map for empty rows", () => {
    const k = computeCohortKappa({
      rows: [],
      keyField: "id",
      coverField: "site",
      fiberFields: ["temp"],
    });
    expect(k.size).toBe(0);
  });

  it("returns κ=0 for singleton cohorts (no peers)", () => {
    const k = computeCohortKappa({
      rows: [{ id: "A", site: "X", temp: 99 }],
      keyField: "id",
      coverField: "site",
      fiberFields: ["temp"],
    });
    expect(k.get("A")).toBe(0);
  });

  it("returns κ=0 when all rows in a cohort have identical fiber values", () => {
    const k = computeCohortKappa({
      rows: [
        { id: "A", site: "X", temp: 22, hum: 60 },
        { id: "B", site: "X", temp: 22, hum: 60 },
        { id: "C", site: "X", temp: 22, hum: 60 },
      ],
      keyField: "id",
      coverField: "site",
      fiberFields: ["temp", "hum"],
    });
    expect(k.get("A")).toBeCloseTo(0);
    expect(k.get("B")).toBeCloseTo(0);
    expect(k.get("C")).toBeCloseTo(0);
  });

  it("flags the outlier with the highest κ in its cohort", () => {
    const k = computeCohortKappa({
      rows: [
        { id: "A", site: "N", temp: 22, hum: 60 },
        { id: "B", site: "N", temp: 21, hum: 61 },
        { id: "C", site: "N", temp: 23, hum: 59 },
        { id: "D", site: "N", temp: 50, hum: 20 }, // outlier
      ],
      keyField: "id",
      coverField: "site",
      fiberFields: ["temp", "hum"],
    });
    const kA = k.get("A")!;
    const kB = k.get("B")!;
    const kC = k.get("C")!;
    const kD = k.get("D")!;
    expect(kD).toBeGreaterThan(kA);
    expect(kD).toBeGreaterThan(kB);
    expect(kD).toBeGreaterThan(kC);
    // D is well above the "bad" threshold for a tight cohort like this.
    expect(kappaClass(kD)).toBe("bad");
  });

  it("computes cohorts independently per cover value", () => {
    // Cohort N is tight; cohort S has a single outlier.
    const k = computeCohortKappa({
      rows: [
        { id: "A", site: "N", temp: 22, hum: 60 },
        { id: "B", site: "N", temp: 22.1, hum: 60 },
        { id: "C", site: "S", temp: 22, hum: 60 },
        { id: "D", site: "S", temp: 80, hum: 10 },
      ],
      keyField: "id",
      coverField: "site",
      fiberFields: ["temp", "hum"],
    });
    expect(k.get("A")).toBeLessThan(0.1);
    expect(k.get("B")).toBeLessThan(0.1);
    expect(k.get("D")).toBeGreaterThan(2);
  });

  it("matches the mockup's leave-one-out formula on a known example", () => {
    // S-0142 from the mockup: site North-3 has 4 sensors, temp=38.7 vs
    // peers' mean of (21.9+21.8+24.1)/3 ≈ 22.6; humidity=18.2 vs
    // (62.4+63.1+55.4)/3 ≈ 60.3.
    // Δ = (16.1, -42.1) → ‖Δ‖ ≈ 45.07; κ = 45.07 / 10 ≈ 4.5
    const k = computeCohortKappa({
      rows: [
        { id: "S-0142", site: "North-3", temp: 38.7, humidity: 18.2 },
        { id: "S-0117", site: "North-3", temp: 21.9, humidity: 62.4 },
        { id: "S-0201", site: "North-3", temp: 21.8, humidity: 63.1 },
        { id: "S-0210", site: "North-3", temp: 24.1, humidity: 55.4 },
      ],
      keyField: "id",
      coverField: "site",
      fiberFields: ["temp", "humidity"],
    });
    expect(k.get("S-0142")).toBeGreaterThan(4.2);
    expect(k.get("S-0142")).toBeLessThan(4.8);
  });

  it("ignores non-numeric values gracefully", () => {
    const k = computeCohortKappa({
      rows: [
        { id: "A", site: "N", temp: 22, hum: "broken" },
        { id: "B", site: "N", temp: 23, hum: 60 },
      ],
      keyField: "id",
      coverField: "site",
      fiberFields: ["temp", "hum"],
    });
    // No throw; A's hum is dropped from its centroid contribution.
    expect(k.has("A")).toBe(true);
    expect(k.has("B")).toBe(true);
  });

  it("handles rows missing the cover field (groups them under '')", () => {
    const k = computeCohortKappa({
      rows: [
        { id: "A", temp: 22, hum: 60 },
        { id: "B", temp: 23, hum: 61 },
      ],
      keyField: "id",
      coverField: "site",
      fiberFields: ["temp", "hum"],
    });
    expect(k.size).toBe(2);
  });
});
