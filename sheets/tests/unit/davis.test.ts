import { describe, expect, it } from "vitest";
import {
  cohortCentroid,
  davisDistance,
  davisIdentityResidual,
  embedNumericRow,
  sameness,
} from "../../src/lib/davis";

function v(...xs: number[]): Float32Array {
  return new Float32Array(xs);
}

describe("davis · sameness", () => {
  it("returns 1 for identical unit vectors", () => {
    const a = v(1, 0, 0);
    expect(sameness(a, a)).toBeCloseTo(1, 6);
  });

  it("returns 0 for opposite unit vectors", () => {
    expect(sameness(v(1, 0), v(-1, 0))).toBeCloseTo(0, 6);
  });

  it("returns 0.5 for orthogonal unit vectors", () => {
    expect(sameness(v(1, 0), v(0, 1))).toBeCloseTo(0.5, 6);
  });

  it("clamps to [0, 1] under floating-point rounding", () => {
    // Vectors that are nearly identical but, after float arithmetic, can
    // yield a dot product slightly above 1. sameness must still be in [0,1].
    const a = v(0.6, 0.8);
    const b = v(0.6 + 1e-9, 0.8);
    const s = sameness(a, b);
    expect(s).toBeGreaterThanOrEqual(0);
    expect(s).toBeLessThanOrEqual(1);
  });

  it("is symmetric: S(a,b) === S(b,a)", () => {
    const a = v(0.3, 0.4, 0.5).slice();
    normalize(a);
    const b = v(-0.1, 0.7, 0.6).slice();
    normalize(b);
    expect(sameness(a, b)).toBeCloseTo(sameness(b, a), 10);
  });
});

describe("davis · davisDistance", () => {
  it("returns 0 for identical vectors", () => {
    const a = v(1, 0);
    expect(davisDistance(a, a)).toBeCloseTo(0, 6);
  });

  it("returns 1 for opposite vectors (S=0, d=√1=1)", () => {
    expect(davisDistance(v(1, 0), v(-1, 0))).toBeCloseTo(1, 6);
  });

  it("returns √0.5 for orthogonal vectors (S=0.5, d=sin(45°))", () => {
    // S = (1 + cos 90°)/2 = 0.5; d = sin(45°) = √0.5.
    const d = davisDistance(v(1, 0), v(0, 1));
    expect(d).toBeCloseTo(Math.SQRT1_2, 6);
  });

  it("satisfies S + d² = 1 for orthogonal vectors", () => {
    const S = sameness(v(1, 0), v(0, 1));
    const d = davisDistance(v(1, 0), v(0, 1));
    expect(S + d * d).toBeCloseTo(1, 6);
  });
});

describe("davis · IDENTITY (the load-bearing test)", () => {
  it("|S + d² − 1| < 1e-6 across 1000 random unit-vector pairs (Davis double-cover)", () => {
    // The Davis double-cover identity is the math the entire app depends on:
    //   S = cos²(θ/2), d = sin(θ/2), so S + d² = cos² + sin² = 1.
    // If this fails, every "sort by sameness," "filter by κ," "drag-fill by
    // OLS" is operating on incoherent geometry.
    const N = 1000;
    const DIM = 16;
    let worst = 0;
    for (let i = 0; i < N; i++) {
      const a = randomUnit(DIM);
      const b = randomUnit(DIM);
      const residual = Math.abs(davisIdentityResidual(a, b));
      if (residual > worst) worst = residual;
    }
    expect(worst).toBeLessThan(1e-6);
  });
});

describe("davis · cohortCentroid", () => {
  it("returns the input vector for a 1-row cohort", () => {
    const r = v(0.6, 0.8);
    const c = cohortCentroid([r]);
    expect(Array.from(c)).toEqual(Array.from(r));
  });

  it("returns a unit vector", () => {
    const c = cohortCentroid([v(0.3, 0.4, 0.5), v(0.1, 0.2, 0.6), v(0.8, 0.1, 0.2)]);
    const len = Math.sqrt(c.reduce((s, x) => s + x * x, 0));
    expect(len).toBeCloseTo(1, 6);
  });

  it("returns the midpoint direction for symmetric pairs", () => {
    // mean of (1,0) and (0,1) is (0.5, 0.5); normalize → (√½, √½)
    const c = cohortCentroid([v(1, 0), v(0, 1)]);
    expect(c[0]).toBeCloseTo(Math.SQRT1_2, 6);
    expect(c[1]).toBeCloseTo(Math.SQRT1_2, 6);
  });

  it("handles a zero-sum cohort by returning a zero vector (no NaN)", () => {
    const c = cohortCentroid([v(1, 0), v(-1, 0)]);
    expect(c[0]).toBe(0);
    expect(c[1]).toBe(0);
    expect(Number.isNaN(c[0])).toBe(false);
  });
});

describe("davis · embedNumericRow", () => {
  it("returns a unit vector for any non-empty input", () => {
    const e = embedNumericRow([1, 2, 3, 4]);
    const len = Math.sqrt(e.reduce((s, x) => s + x * x, 0));
    expect(len).toBeCloseTo(1, 6);
  });

  it("returns zero vector for empty input (no NaN)", () => {
    const e = embedNumericRow([]);
    expect(Array.from(e)).toEqual([]);
  });

  it("treats two identical numeric rows as sameness=1", () => {
    const a = embedNumericRow([10, 20, 30]);
    const b = embedNumericRow([10, 20, 30]);
    expect(sameness(a, b)).toBeCloseTo(1, 6);
  });

  it("treats one-zero rows and one-positive rows correctly", () => {
    // [1,0] and [0,1] should be orthogonal in this simple embed.
    const a = embedNumericRow([1, 0]);
    const b = embedNumericRow([0, 1]);
    expect(sameness(a, b)).toBeCloseTo(0.5, 6);
  });
});

// ── helpers ────────────────────────────────────────────────────────────
function normalize(a: Float32Array): void {
  let n = 0;
  for (let i = 0; i < a.length; i++) n += a[i] * a[i];
  n = Math.sqrt(n);
  if (n > 0) for (let i = 0; i < a.length; i++) a[i] /= n;
}

function randomUnit(dim: number): Float32Array {
  const v = new Float32Array(dim);
  let n = 0;
  for (let i = 0; i < dim; i++) {
    // Box-Muller for a normal distribution; sum-of-normals on a sphere.
    const u1 = Math.random() || 1e-12;
    const u2 = Math.random();
    v[i] = Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * u2);
    n += v[i] * v[i];
  }
  n = Math.sqrt(n);
  if (n > 0) for (let i = 0; i < dim; i++) v[i] /= n;
  return v;
}
