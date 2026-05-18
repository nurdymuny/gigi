import { describe, expect, it } from "vitest";
import {
  axisTicks,
  computeProjectionDomain,
  coverColor,
  nearestNeighbor,
  projectPoint,
  projectRows,
  type ProjectionDomain,
} from "../../src/lib/projection";

const ROWS = [
  { id: "A", temp: 22, hum: 60, site: "N" },
  { id: "B", temp: 23, hum: 61, site: "N" },
  { id: "C", temp: 50, hum: 10, site: "S" },
];

describe("computeProjectionDomain", () => {
  it("returns null when no row has finite (x, y) values", () => {
    const d = computeProjectionDomain(
      [{ id: "A", temp: "broken", hum: "broken" }],
      "temp",
      "hum",
    );
    expect(d).toBeNull();
  });

  it("returns null on empty input", () => {
    expect(computeProjectionDomain([], "temp", "hum")).toBeNull();
  });

  it("computes [min, max] with padding for both axes", () => {
    const d = computeProjectionDomain(ROWS, "temp", "hum", 0)!;
    expect(d.x).toEqual({ field: "temp", min: 22, max: 50 });
    expect(d.y).toEqual({ field: "hum", min: 10, max: 61 });
  });

  it("adds proportional padding (default 8%)", () => {
    const d = computeProjectionDomain(ROWS, "temp", "hum")!;
    // x range = 28, pad = 28*0.08 = 2.24
    expect(d.x.min).toBeCloseTo(22 - 2.24, 2);
    expect(d.x.max).toBeCloseTo(50 + 2.24, 2);
  });

  it("uses a unit pad for collapsed axes (all values identical)", () => {
    const d = computeProjectionDomain(
      [
        { id: "A", temp: 22, hum: 60 },
        { id: "B", temp: 22, hum: 60 },
      ],
      "temp",
      "hum",
    )!;
    expect(d.x.max - d.x.min).toBe(2);
    expect(d.y.max - d.y.min).toBe(2);
  });

  it("skips rows with NaN values without poisoning the domain", () => {
    const d = computeProjectionDomain(
      [
        { id: "A", temp: 22, hum: 60 },
        { id: "B", temp: "not-a-number", hum: 99 },
        { id: "C", temp: 50, hum: 10 },
      ],
      "temp",
      "hum",
      0,
    )!;
    expect(d.x).toEqual({ field: "temp", min: 22, max: 50 });
    expect(d.y).toEqual({ field: "hum", min: 10, max: 60 });
  });
});

describe("projectPoint", () => {
  const domain: ProjectionDomain = {
    x: { field: "temp", min: 0, max: 100 },
    y: { field: "hum", min: 0, max: 100 },
  };
  const view = { width: 200, height: 200, margin: { l: 0, r: 0, t: 0, b: 0 } };

  it("maps the corner of the domain to pixel space", () => {
    expect(projectPoint({ x: 0, y: 0 }, domain, view)).toEqual({ px: 0, py: 200 });
    expect(projectPoint({ x: 100, y: 100 }, domain, view)).toEqual({
      px: 200,
      py: 0,
    });
  });

  it("flips the y axis (top-down SVG coords)", () => {
    expect(projectPoint({ x: 0, y: 100 }, domain, view).py).toBe(0);
    expect(projectPoint({ x: 0, y: 0 }, domain, view).py).toBe(200);
  });

  it("applies margins symmetrically", () => {
    const margined = {
      width: 200,
      height: 200,
      margin: { l: 20, r: 20, t: 20, b: 20 },
    };
    const p = projectPoint({ x: 50, y: 50 }, domain, margined);
    expect(p.px).toBe(100);
    expect(p.py).toBe(100);
  });
});

describe("projectRows", () => {
  const domain: ProjectionDomain = {
    x: { field: "temp", min: 20, max: 60 },
    y: { field: "hum", min: 0, max: 100 },
  };
  const view = { width: 200, height: 200, margin: { l: 0, r: 0, t: 0, b: 0 } };

  it("returns one ProjectedPoint per row with finite values", () => {
    const points = projectRows(ROWS, "id", "temp", "hum", domain, view);
    expect(points).toHaveLength(3);
    expect(points[0]).toMatchObject({ key: "A", x: 22, y: 60 });
  });

  it("drops rows with non-finite values", () => {
    const points = projectRows(
      [
        ...ROWS,
        { id: "D", temp: NaN, hum: 50 },
        { id: "E", temp: 30, hum: "garbage" },
      ],
      "id",
      "temp",
      "hum",
      domain,
      view,
    );
    expect(points.map((p) => p.key)).toEqual(["A", "B", "C"]);
  });
});

describe("nearestNeighbor", () => {
  const points = [
    { key: "A", x: 0, y: 0, px: 0, py: 0 },
    { key: "B", x: 1, y: 1, px: 0, py: 0 },
    { key: "C", x: 10, y: 10, px: 0, py: 0 },
    { key: "D", x: 100, y: 100, px: 0, py: 0 },
  ];

  it("returns the nearest point by fiber-space distance", () => {
    const target = { key: "A", x: 0, y: 0, px: 0, py: 0 };
    const r = nearestNeighbor(target, points, "A");
    expect(r?.key).toBe("B");
  });

  it("never returns the excluded key (no self-loops)", () => {
    const target = { key: "B", x: 1, y: 1, px: 0, py: 0 };
    const r = nearestNeighbor(target, points, "B");
    expect(r?.key).not.toBe("B");
    expect(r?.key).toBe("A");
  });

  it("returns null if no candidates remain after exclusion", () => {
    const only = [{ key: "A", x: 0, y: 0, px: 0, py: 0 }];
    expect(nearestNeighbor(only[0], only, "A")).toBeNull();
  });
});

describe("coverColor", () => {
  it("is deterministic — same input → same color", () => {
    expect(coverColor("North-3")).toBe(coverColor("North-3"));
    expect(coverColor("")).toBe(coverColor(""));
  });

  it("returns a valid hex color", () => {
    expect(coverColor("anything")).toMatch(/^#[0-9a-f]{6}$/i);
  });

  it("distinguishes typical cover values", () => {
    const colors = new Set([
      coverColor("North-3"),
      coverColor("East-1"),
      coverColor("South-2"),
      coverColor("West-4"),
    ]);
    // At least 3 of the 4 distinct site labels should produce distinct colors.
    expect(colors.size).toBeGreaterThanOrEqual(3);
  });
});

describe("axisTicks", () => {
  it("produces evenly spaced ticks across [min, max]", () => {
    expect(axisTicks(0, 100, 5)).toEqual([0, 25, 50, 75, 100]);
  });

  it("handles single-tick case", () => {
    expect(axisTicks(5, 10, 1)).toEqual([5]);
  });

  it("handles inverted range gracefully", () => {
    // We don't promise correctness on inverted, but it shouldn't throw.
    expect(() => axisTicks(10, 0, 5)).not.toThrow();
  });
});
