import { describe, expect, it } from "vitest";
import { buildGqlSamples } from "../../src/lib/gql-samples";
import type { BundleSchema } from "../../src/lib/gigi-client";

/**
 * GQL sample-query builder — populates the chips above the GQL editor
 * with ready-to-run queries targeted at the current bundle. Tests pin
 * the substitution logic so the chips never produce queries the engine
 * would reject (missing bundle name, no key field, etc.).
 */

const SCHEMA: BundleSchema = {
  name: "nba_2024",
  base_fields: [{ name: "team", type: "text" }],
  fiber_fields: [
    { name: "conference", type: "categorical" },
    { name: "wins", type: "numeric" },
    { name: "points_scored", type: "numeric" },
    { name: "points_allowed", type: "numeric" },
  ],
  indexed_fields: ["team"],
  records: 30,
  storage_mode: "mmap",
} as unknown as BundleSchema;

describe("buildGqlSamples · query substitution", () => {
  it("returns the canonical 8 sample queries", () => {
    const samples = buildGqlSamples({
      schema: SCHEMA,
      coverField: "conference",
      sampleRowKey: "BOS",
      secondRowKey: "LAL",
    });
    expect(samples.map((s) => s.id)).toEqual([
      "curvature",
      "curvature-cover",
      "betti",
      "spectral",
      "section",
      "integrate",
      "holonomy",
      "transport",
    ]);
  });

  it("CURVATURE chip targets the active bundle", () => {
    const samples = buildGqlSamples({
      schema: SCHEMA,
      coverField: "conference",
      sampleRowKey: "BOS",
      secondRowKey: "LAL",
    });
    const curv = samples.find((s) => s.id === "curvature");
    expect(curv?.query).toBe("CURVATURE nba_2024;");
  });

  it("per-cohort CURVATURE uses BY (engine grammar — NOT 'COVER')", () => {
    const samples = buildGqlSamples({
      schema: SCHEMA,
      coverField: "conference",
      sampleRowKey: "BOS",
      secondRowKey: "LAL",
    });
    expect(samples.find((s) => s.id === "curvature-cover")?.query).toBe(
      "CURVATURE nba_2024 BY conference;",
    );
  });

  it("SECTION uses bare key=val (no parens — engine's parse_kv_pairs)", () => {
    const samples = buildGqlSamples({
      schema: SCHEMA,
      coverField: "conference",
      sampleRowKey: "BOS",
      secondRowKey: "LAL",
    });
    expect(samples.find((s) => s.id === "section")?.query).toBe(
      "SECTION nba_2024 AT team='BOS';",
    );
  });

  it("INTEGRATE uses bundle-first + MEASURE(field) (NOT 'INTEGRATE field OVER bundle')", () => {
    // With a cover field present, group by it; otherwise emit a global aggregate.
    const withCover = buildGqlSamples({
      schema: SCHEMA,
      coverField: "conference",
      sampleRowKey: "BOS",
      secondRowKey: "LAL",
    });
    expect(withCover.find((s) => s.id === "integrate")?.query).toBe(
      "INTEGRATE nba_2024 OVER conference MEASURE AVG(wins);",
    );

    const noCover = buildGqlSamples({
      schema: SCHEMA,
      coverField: "",
      sampleRowKey: "BOS",
      secondRowKey: "LAL",
    });
    expect(noCover.find((s) => s.id === "integrate")?.query).toBe(
      "INTEGRATE nba_2024 MEASURE AVG(wins);",
    );
  });

  it("HOLONOMY uses the first two numeric fields + cover", () => {
    const samples = buildGqlSamples({
      schema: SCHEMA,
      coverField: "conference",
      sampleRowKey: "BOS",
      secondRowKey: "LAL",
    });
    expect(samples.find((s) => s.id === "holonomy")?.query).toBe(
      "HOLONOMY nba_2024 ON FIBER (wins, points_scored) AROUND conference;",
    );
  });

  it("TRANSPORT uses two sample row keys + two fiber fields", () => {
    const samples = buildGqlSamples({
      schema: SCHEMA,
      coverField: "conference",
      sampleRowKey: "BOS",
      secondRowKey: "LAL",
    });
    expect(samples.find((s) => s.id === "transport")?.query).toBe(
      "TRANSPORT nba_2024 FROM (team='BOS') TO (team='LAL') ON FIBER (wins, points_scored);",
    );
  });
});

describe("buildGqlSamples · degraded inputs", () => {
  it("null schema returns no samples", () => {
    expect(
      buildGqlSamples({
        schema: null,
        coverField: "",
        sampleRowKey: null,
        secondRowKey: null,
      }),
    ).toEqual([]);
  });

  it("missing cover field disables the per-cohort + holonomy samples", () => {
    const samples = buildGqlSamples({
      schema: SCHEMA,
      coverField: "",
      sampleRowKey: "BOS",
      secondRowKey: "LAL",
    });
    const ids = samples.map((s) => s.id);
    expect(ids).not.toContain("curvature-cover");
    expect(ids).not.toContain("holonomy");
    // The bundle-wide ones still ship.
    expect(ids).toContain("curvature");
    expect(ids).toContain("betti");
    expect(ids).toContain("spectral");
  });

  it("missing sample row key disables SECTION + TRANSPORT", () => {
    const samples = buildGqlSamples({
      schema: SCHEMA,
      coverField: "conference",
      sampleRowKey: null,
      secondRowKey: null,
    });
    const ids = samples.map((s) => s.id);
    expect(ids).not.toContain("section");
    expect(ids).not.toContain("transport");
  });

  it("with only one row key (no second), TRANSPORT is disabled but SECTION ships", () => {
    const samples = buildGqlSamples({
      schema: SCHEMA,
      coverField: "conference",
      sampleRowKey: "BOS",
      secondRowKey: null,
    });
    expect(samples.find((s) => s.id === "section")).toBeDefined();
    expect(samples.find((s) => s.id === "transport")).toBeUndefined();
  });

  it("schema with no numeric fields disables INTEGRATE + HOLONOMY", () => {
    const noNumerics: BundleSchema = {
      ...SCHEMA,
      fiber_fields: [{ name: "tag", type: "categorical" }],
    } as unknown as BundleSchema;
    const samples = buildGqlSamples({
      schema: noNumerics,
      coverField: "tag",
      sampleRowKey: "BOS",
      secondRowKey: "LAL",
    });
    const ids = samples.map((s) => s.id);
    expect(ids).not.toContain("integrate");
    expect(ids).not.toContain("holonomy");
  });

  it("escapes single quotes in row keys", () => {
    const samples = buildGqlSamples({
      schema: SCHEMA,
      coverField: "conference",
      sampleRowKey: "O'Brien",
      secondRowKey: "LAL",
    });
    const section = samples.find((s) => s.id === "section");
    expect(section?.query).toBe("SECTION nba_2024 AT team='O''Brien';");
  });
});

describe("buildGqlSamples · metadata for each sample", () => {
  it("every sample has a short label and a description", () => {
    const samples = buildGqlSamples({
      schema: SCHEMA,
      coverField: "conference",
      sampleRowKey: "BOS",
      secondRowKey: "LAL",
    });
    for (const s of samples) {
      expect(s.label.length, `${s.id} label`).toBeGreaterThan(0);
      expect(s.label.length, `${s.id} label terse`).toBeLessThan(30);
      expect(s.description.length, `${s.id} description`).toBeGreaterThan(0);
      expect(s.query.endsWith(";"), `${s.id} ends with ;`).toBe(true);
    }
  });
});
