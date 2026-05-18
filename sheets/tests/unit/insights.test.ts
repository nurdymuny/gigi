import { describe, expect, it } from "vitest";
import { computeInsights } from "../../src/lib/insights";
import type { BundleSchema } from "../../src/lib/gigi-client";

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "site_id", type: "categorical" },
    { name: "temp", type: "numeric" },
    { name: "humidity", type: "numeric" },
  ],
  indexed_fields: ["sensor_id"],
  records: 0,
  storage_mode: "mmap",
};

const ROWS = [
  { sensor_id: "S-001", site_id: "N", temp: 22, humidity: 60 },
  { sensor_id: "S-002", site_id: "N", temp: 23, humidity: 61 },
  { sensor_id: "S-003", site_id: "N", temp: 24, humidity: 59 },
  { sensor_id: "S-OUT", site_id: "N", temp: 99, humidity: 5 },
  { sensor_id: "S-004", site_id: "S", temp: 22, humidity: 60 },
];

const KAPPA = new Map<string, number>([
  ["S-001", 0.2],
  ["S-002", 0.2],
  ["S-003", 0.3],
  ["S-OUT", 4.2],
  ["S-004", 0.0],
]);

describe("computeInsights", () => {
  it("returns [] for empty rows", () => {
    expect(
      computeInsights({
        bundle: "sensors",
        schema: SCHEMA,
        rows: [],
        kappaMap: new Map(),
        coverField: "site_id",
        meanCurvature: 0,
      }),
    ).toEqual([]);
  });

  it("returns [] when schema is null", () => {
    expect(
      computeInsights({
        bundle: "sensors",
        schema: null,
        rows: ROWS,
        kappaMap: KAPPA,
        coverField: "site_id",
        meanCurvature: 0,
      }),
    ).toEqual([]);
  });

  it("surfaces the cohort that holds the most anomalies", () => {
    const insights = computeInsights({
      bundle: "sensors",
      schema: SCHEMA,
      rows: ROWS,
      kappaMap: KAPPA,
      coverField: "site_id",
      meanCurvature: 1.0,
    });
    const top = insights.find((i) => i.id === "cohort-top-anomalies");
    expect(top).toBeDefined();
    expect(top?.tag).toBe("bad");
    expect(top?.body).toContain("N");
    expect(top?.gql).toContain("SECTION sensors WHERE site_id='N'");
  });

  it("surfaces the highest-κ row when it crosses the warn threshold", () => {
    const insights = computeInsights({
      bundle: "sensors",
      schema: SCHEMA,
      rows: ROWS,
      kappaMap: KAPPA,
      coverField: "site_id",
      meanCurvature: 1.0,
    });
    const top = insights.find((i) => i.id === "top-kappa");
    expect(top).toBeDefined();
    expect(top?.body).toContain("S-OUT");
    expect(top?.body).toContain("4.20");
  });

  it("does NOT surface a top-κ insight when everything is healthy", () => {
    const insights = computeInsights({
      bundle: "sensors",
      schema: SCHEMA,
      rows: ROWS,
      kappaMap: new Map(ROWS.map((r) => [r.sensor_id, 0.1])),
      coverField: "site_id",
      meanCurvature: 0.1,
    });
    expect(insights.find((i) => i.id === "top-kappa")).toBeUndefined();
  });

  it("calls out a loose cohort separately from the top-anomaly cohort", () => {
    const looseRows = [
      ...ROWS,
      { sensor_id: "S-100", site_id: "Z", temp: 22, humidity: 60 },
      { sensor_id: "S-101", site_id: "Z", temp: 23, humidity: 61 },
      { sensor_id: "S-102", site_id: "Z", temp: 22, humidity: 60 },
    ];
    const kappa = new Map<string, number>([
      ...KAPPA,
      ["S-100", 1.1],
      ["S-101", 1.3],
      ["S-102", 0.9],
    ]);
    const insights = computeInsights({
      bundle: "sensors",
      schema: SCHEMA,
      rows: looseRows,
      kappaMap: kappa,
      coverField: "site_id",
      meanCurvature: 1.0,
    });
    const loose = insights.find((i) => i.id === "loose-cohort");
    expect(loose).toBeDefined();
    expect(loose?.body).toContain("Z");
  });

  it("always emits the bundle-wide κ̄ summary", () => {
    const insights = computeInsights({
      bundle: "sensors",
      schema: SCHEMA,
      rows: ROWS,
      kappaMap: KAPPA,
      coverField: "site_id",
      meanCurvature: 0.6,
    });
    expect(insights.find((i) => i.id === "mean-kappa")).toBeDefined();
  });

  it("calls out encrypted fields when present in the schema", () => {
    const enc: BundleSchema = {
      ...SCHEMA,
      fiber_fields: [
        ...SCHEMA.fiber_fields,
        { name: "operator", type: "text", encryption: "indexed" },
        { name: "secret", type: "numeric", encryption: "opaque" },
      ],
    };
    const insights = computeInsights({
      bundle: "sensors",
      schema: enc,
      rows: ROWS,
      kappaMap: KAPPA,
      coverField: "site_id",
      meanCurvature: 0.5,
    });
    const card = insights.find((i) => i.id === "encrypted-fields");
    expect(card).toBeDefined();
    expect(card?.body).toContain("operator");
    expect(card?.body).toContain("secret");
  });

  it("ranks insights by score (highest first)", () => {
    const insights = computeInsights({
      bundle: "sensors",
      schema: SCHEMA,
      rows: ROWS,
      kappaMap: KAPPA,
      coverField: "site_id",
      meanCurvature: 1.0,
    });
    for (let i = 1; i < insights.length; i++) {
      expect(insights[i - 1].score).toBeGreaterThanOrEqual(insights[i].score);
    }
  });

  it("escapes single quotes in cohort labels for safe GQL", () => {
    const rows = [
      { sensor_id: "A", site_id: "O'Brien", temp: 22, humidity: 60 },
      { sensor_id: "B", site_id: "O'Brien", temp: 99, humidity: 1 },
    ];
    const kappa = new Map<string, number>([["A", 0.1], ["B", 4.0]]);
    const insights = computeInsights({
      bundle: "sensors",
      schema: SCHEMA,
      rows,
      kappaMap: kappa,
      coverField: "site_id",
      meanCurvature: 1,
    });
    const top = insights.find((i) => i.id === "cohort-top-anomalies");
    expect(top?.gql).toContain("'O''Brien'");
  });
});
