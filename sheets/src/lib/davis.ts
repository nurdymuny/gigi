/**
 * Davis geometric primitives — the load-bearing math layer for Sheets.
 *
 * Every "smart" feature (sort by sameness, filter by κ, sameness-join,
 * drag-fill OLS) ultimately reduces to one of these functions. Keep them
 * tight, well-tested, and free of side effects.
 *
 *   sameness        S(a, b) = (1 + cosθ)/2 = cos²(θ/2) ∈ [0, 1]
 *   davisDistance   d(a, b) = √(1 − S) = sin(θ/2)      ∈ [0, 1]
 *   cohortCentroid  μ = normalize(Σ rᵢ)
 *   deviation       1 − S(r, μ) = d²                   ∈ [0, 1]
 *
 * The Davis double-cover identity **S + d² = 1** is the single
 * non-negotiable invariant — note `S` is linear here, not squared.
 * Geometrically, S is `cos²(θ/2)` and d is `sin(θ/2)`, so the Pythagorean
 * identity `cos²(θ/2) + sin²(θ/2) = 1` gives the identity directly.
 *
 * `davisIdentityResidual` is what the load-bearing test in
 * `tests/unit/davis.test.ts` measures.
 *
 * ── References ────────────────────────────────────────────────────
 * Davis, B. R. (2026). *The Double Cover Principle.* Theorem 3.1
 *   (Pythagorean Bridge). The identity `S + d² = 1` for all θ.
 * Davis, B. R. (2026). *Zero Does Not Exist.* §2.4. The identity is
 *   vacuous at n = 1 (S = 1, d² = 0 trivially); becomes a genuine
 *   constraint at n ≥ 2.
 * Davis, B. R. (2026). *The Davis Duality of Approximation and
 *   Obstruction.* Davis invariant C = τ/K.
 *
 * ── Class note ───────────────────────────────────────────────────
 * This is the **quadratic sameness class** S_Q = cos²(θ/2) from
 * Theorem 4.4 (Quadratic sameness) in the Double Cover Principle —
 * the class associated with Fubini-Study geometry on CP¹ (the Bloch
 * sphere). The alternative **arcsin sameness** S_A = 1 − (2/π)·arcsin(K/2)
 * is documented in Theorem 4.7 of the same paper but is NOT used
 * here. We pick S_Q because (1) it has quadratic protection at the
 * identity (`|dS/dθ| → 0` as θ → 0, so small noise barely moves
 * sameness), and (2) it's the form that drops out of any inner-product
 * embedding via the dot-product cosine.
 *
 * ── Vocabulary note: "deviation" / "κ" / "K" ─────────────────────
 * Three quantities show up in the codebase that are easy to confuse:
 *  - `1 − S` returned by `deviation(row, centroid)` below — bounded
 *    [0, 1]. This is d², the **deviation** (Zero-paper §2.4 vocab).
 *  - The engine's per-row κ (in `kappa.ts`, thresholded at warn 0.8 /
 *    bad 2.0) — this is the chord K = |1 − e^iθ| = 2 sin(θ/2) from
 *    the Double Cover paper, bounded [0, 2], **not** the same as
 *    deviation. Related by K² = 4·d².
 *  - The Riemannian κ in the Davis Duality paper — different again
 *    (sectional curvature on a manifold). Out of scope here.
 */

/** Cosine sameness in [0, 1]. Inputs are assumed unit-length. */
export function sameness(a: Float32Array, b: Float32Array): number {
  const n = a.length;
  if (n !== b.length) {
    throw new Error(
      `sameness: dimension mismatch (${n} vs ${b.length}). Embeddings must agree.`,
    );
  }
  let dot = 0;
  for (let i = 0; i < n; i++) dot += a[i] * b[i];
  // Float-arithmetic guard: dot can drift outside [-1, 1] by ε.
  if (dot > 1) dot = 1;
  if (dot < -1) dot = -1;
  return (1 + dot) / 2;
}

/**
 * Davis distance derived from the double-cover identity: d = √(1 − S).
 *
 * Note this is `1 − S`, not `1 − S²` — see the file header. With
 * `S = cos²(θ/2)`, the natural geometric pair is `d = sin(θ/2)`, and
 * `cos² + sin² = 1` gives `S + d² = 1` directly.
 */
export function davisDistance(a: Float32Array, b: Float32Array): number {
  const S = sameness(a, b);
  const sq = 1 - S;
  return sq <= 0 ? 0 : Math.sqrt(sq);
}

/**
 * Residual of the Davis double-cover identity for a pair of vectors:
 *   |S + d² − 1|
 *
 * For unit vectors this is identically 0 by construction (since d² is
 * *defined* as 1 − S). The test exists to catch any future
 * implementation that derives S and d independently and lets them drift.
 */
export function davisIdentityResidual(a: Float32Array, b: Float32Array): number {
  const S = sameness(a, b);
  const d = davisDistance(a, b);
  return Math.abs(S + d * d - 1);
}

/**
 * Deviation `d² = 1 − S`: how far a row sits from a reference centroid,
 * in the [0, 1] range. By the Davis double-cover identity this equals
 * `d²` where `d` is the half-angle distance.
 *
 * Naming follows *Zero Does Not Exist* §2.4: the loss-of-sameness
 * quantity is called the **deviation**, distinct from:
 *
 * - the engine's per-row κ (in `kappa.ts`), which is the chord
 *   K = 2 sin(θ/2) bounded [0, 2] — related to deviation by K² = 4·d²
 * - the Riemannian κ in the *Davis Duality* paper, sectional
 *   curvature on a manifold — out of scope here
 *
 * Use this function whenever you want `1 − S` as a single value.
 */
export function deviation(row: Float32Array, centroid: Float32Array): number {
  return 1 - sameness(row, centroid);
}

/**
 * Cohort centroid: take the (unnormalized) mean of a group of unit vectors,
 * then re-normalize. If the rows form a perfectly balanced opposite pair the
 * mean is the zero vector — return that explicitly so callers can detect it
 * rather than receiving NaN from a divide-by-zero.
 */
export function cohortCentroid(rows: Float32Array[]): Float32Array {
  if (rows.length === 0) return new Float32Array(0);
  const dim = rows[0].length;
  const sum = new Float32Array(dim);
  for (const r of rows) {
    if (r.length !== dim) {
      throw new Error(
        `cohortCentroid: dimension mismatch (${r.length} vs ${dim}).`,
      );
    }
    for (let i = 0; i < dim; i++) sum[i] += r[i];
  }
  for (let i = 0; i < dim; i++) sum[i] /= rows.length;
  // Renormalize the mean. If it's a zero vector, leave it — callers must
  // handle this case explicitly (it means the cohort has no preferred
  // direction).
  let n = 0;
  for (let i = 0; i < dim; i++) n += sum[i] * sum[i];
  n = Math.sqrt(n);
  if (n > 0) for (let i = 0; i < dim; i++) sum[i] /= n;
  return sum;
}

/**
 * Embed an array of numeric values into a unit vector by direct L2
 * normalization. Useful for "is this row roughly the same as that row?"
 * over numeric columns. For mixed-type rows use the heavier hashing-trick
 * embedder in prism-workflows.ts.
 */
export function embedNumericRow(values: number[]): Float32Array {
  const out = new Float32Array(values.length);
  let n = 0;
  for (let i = 0; i < values.length; i++) {
    const v = Number.isFinite(values[i]) ? values[i] : 0;
    out[i] = v;
    n += v * v;
  }
  n = Math.sqrt(n);
  if (n > 0) for (let i = 0; i < values.length; i++) out[i] /= n;
  return out;
}
