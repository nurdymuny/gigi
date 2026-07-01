# SPECTRAL_GAUGE Phase 2 — Sparse Lanczos Spec

**Status**: SPEC (not yet implemented). Successor to Phase 1 (shipped 2026-06-28,
commit `e37ae9e`, doc `SPECTRAL_GAUGE_PHASE1_SHIPPED_2026-06-28.md`).

**Author**: Hallie, Principal Halcyon Engineer, 2026-06-30.

**Framing carried forward from Phase 1**: L_A's spectrum is globally
gauge-invariant, but the per-edge trace weight `Re Tr(U_e)/N` is only locally
gauge-covariant. This verb returns the **fiber-weighted spectral gap** — NOT
the strict Yang-Mills mass gap. Phase 2 does not change this framing; it only
changes how the eigenvalues are extracted.

---

## 1. Motivation — why dense stops working

Phase 1 assembles the full V×V Laplacian as a `nalgebra::DMatrix<f64>` and calls
`SymmetricEigen`. Memory is O(V²), compute is O(V³). This is fine on the
buckyball (V = 60 vertices, 90 edges) and up to about V ~ 10 000, but it walls
off the regime real Yang-Mills lattice studies live in:

| lattice                             | V (edges = LinkCount)  | dense L_A size |
|-------------------------------------|------------------------|----------------|
| buckyball                           | 90                     | 65 KB          |
| 4D SU(2), L = 8, OBC                | 4 · 8⁴ = 16 384        | 2.1 GB         |
| 4D SU(2), L = 16, OBC               | 4 · 16⁴ = 262 144      | 549 GB         |
| 4D SU(2), L = 24, OBC               | 4 · 24⁴ = 1 327 104    | 14 TB          |

At SU(2) β = 2.3 with OBC — the working point Gigi's YM mass-gap line-up needs
for Q ≠ 0 sector occupancy — the smallest lattice that admits topological
sectors and clean plateau extraction is L ≥ 16. Phase 1 cannot reach it. That
is what Phase 2 exists to fix.

Note the axis matters: dense is memory-bound at L ≥ 12 well before it becomes
compute-bound. A single 549 GB matrix does not fit anywhere on the RTX 5070
Laptop that Gigi runs local heatbath on, nor on the Modal machines the cross-
seed L = 24 OBC ensemble is being pulled to. Sparse is the only way in.

---

## 2. Algorithm — Implicitly Restarted Lanczos over signed L_A

The Laplacian's structure is the whole reason a Krylov method wins here.

**Matrix-vector product is local.** Each edge `e = (i, j)` with weight `w_e`
contributes at most four nonzero updates to `L_A · v`:

```
(L_A v)_i +=  w_e (v_i - v_j)
(L_A v)_j +=  w_e (v_j - v_i)
```

So the per-vertex row of L_A has exactly `deg(i) + 1` nonzeros: one diagonal
entry plus one off-diagonal per incident edge. On a 4D cubic lattice with OBC
each interior lattice vertex has degree 8 (up to 4 neighbours in each of ±
directions), and each link vertex — the row index in L_A per the Phase 1
construction — has exactly two lattice endpoints, giving `z ≈ 5` nonzeros per
row on average. That is the payoff:

* **Sparse storage**: O(V · z) = O(V), not O(V²).
* **Matvec**: O(V · z) = O(V), not O(V²).
* **Krylov iteration** finds k extreme eigenpairs in O(k · V · z) work per
  restart, with the constants set by the number of Lanczos vectors kept.

The eigensolver of choice is **Implicitly Restarted Lanczos (IRL)**, i.e. the
Lanczos analog of IRAM. IRL keeps a bounded Krylov subspace (dim ~ 2k or 3k),
periodically compresses it via QR steps on the tridiagonal T_m matrix keyed
to unwanted Ritz values, and iterates until the k target Ritz pairs converge.
That is what ARPACK's `dsaupd`/`dseupd` implement.

---

## 3. Sparse representation — CSR over V × V

Phase 1 already indexes vertices densely (each unique vertex id gets a compact
0..V row on first sight — see `spectral_gauge_gap` step 4 in `src/spectral.rs`
lines 1175–1197). Phase 2 reuses that index and rewrites the assembly step:

1. **First pass** over records: fill `vertex_idx`, count `deg[i]` for each
   vertex (via `+= 1` on both endpoints of every edge), and stage
   `(i, j, w_e)` triples.
2. **Allocate CSR**: `row_ptr` of length V+1 by exclusive-scan of a per-row
   nonzero count = `deg[i] + 1` (one diagonal + `deg[i]` off-diagonals);
   `col_idx` and `values` of length `sum(row_ptr[V])`.
3. **Second pass**: for each staged triple `(i, j, w_e)` with `i < j`, write
   the two off-diagonals `L[i,j] = L[j,i] = -w_e` and accumulate into the
   diagonals `L[i,i] += w_e`, `L[j,j] += w_e`. Skip self-loops (i == j).
4. **Symmetry**: the matrix is symmetric by construction — the assembly can
   build only the upper triangle and let the Lanczos wrapper treat it as
   symmetric, or store the full matrix (both triangles). Full-matrix storage
   trades ~2× memory for simpler matvec and is the recommended default at
   Phase 2 sizes because V · z memory is small enough not to matter.

CSR is the right choice over COO or CSC here because Lanczos matvec is
strictly `L · v` (never `Lᵀ · v` — L is symmetric, so `Lᵀ = L`), and CSR gives
optimal row-streaming matvec.

**Memory footprint at L = 16, OBC**: V = 262 144, average z = 5, so ~1.3M
nonzeros × 16 B per nonzero (f64 value + u32 col_idx + row_ptr overhead) ≈
20 MB for the matrix. Krylov vectors at k = 4 with restart dim m = 20 add
another 20 · V · 8 B ≈ 40 MB. Total peak in the low hundreds of MB. Well
inside the 4 GB acceptance ceiling below.

---

## 4. Indefiniteness — the SU(2) sign problem

SU(2)'s trace weight `w_e = q_0` ranges over [-1, +1]. At the working β = 2.3,
OBC configurations routinely produce edges with `q_0 < 0` — anti-aligned
holonomy. This makes L_A **indefinite** (not positive semidefinite). Phase 1
notes this in its rustdoc: "the 'smallest nonzero' eigenvalue may legitimately
be negative in heavily anti-correlated regimes — this is the honest physics,
surfaced verbatim rather than clamped" (`src/spectral.rs:1113–1117`).

Lanczos on an indefinite symmetric matrix works, but with caveats:

1. **Loss of orthogonality**. Roundoff drifts the Lanczos basis off-orthogonal
   over ~O(√ε_machine) iterations; on indefinite spectra this happens faster
   because near-cancelling contributions amplify roundoff. **Full orthogonalization**
   (Gram–Schmidt against every previous basis vector at every step) or
   **selective orthogonalization** (Simon 1984 — only re-orthogonalize against
   converged Ritz vectors) is required. IRL implementations in ARPACK use
   partial reorthogonalization by default; for Phase 2's small k (≤ 4) full
   reorthogonalization is affordable and safer.
2. **Convergence order**. Lanczos naturally converges to extremal eigenvalues
   fastest — the largest and smallest of the spectrum. For SPECTRAL_GAUGE we
   want the k **smallest-magnitude** eigenvalues, i.e. the ones nearest zero.
   For a positive semidefinite Laplacian these are the smallest algebraic; for
   an indefinite one they are neither the largest nor the smallest algebraic
   but a middle band. Two paths:

   **Path A — shift-invert**. Solve `(L_A - σI)⁻¹ v = w` at each matvec,
   with σ = 0 (or a small shift to avoid singularity). This maps the desired
   near-zero eigenvalues to the largest-magnitude eigenvalues of `(L_A - σI)⁻¹`,
   which Lanczos converges to fastest. Cost: one sparse LU factorization
   up front plus one triangular solve per matvec. Sparse LU on a 4D lattice
   L_A fills in aggressively (bandwidth ~ V^{3/4} = 22 000 at L = 16); the
   factorization dominates memory and can push peak past the 4 GB budget.
   Use SuiteSparse via `suitesparse-sys` or the `sprs` crate's LU if the
   factorization stays sparse; fall back to an iterative inner solve
   (MINRES on `L_A - σI`) if not.

   **Path B — |A|-Lanczos (absolute value)**. Run Lanczos on `L_Aᵀ L_A = L_A²`
   (which IS PSD) and extract sqrt(λ) as the target. Squaring doubles the
   condition number and roughly halves the achievable relative accuracy; for
   the fiber-weighted gap where we already report signed eigenvalues this
   loses the sign. Only use this if shift-invert is infeasible.

   **Recommendation**: Path A (shift-invert) with σ = 0 as the default;
   fall back to unshifted Lanczos with **explicit sign check** on the top-of-
   sequence Ritz pair (the "signed absolute Lanczos" pattern). The API
   exposes `SHIFT σ` so a caller can move σ away from 0 when the factor at
   0 is nearly singular.

3. **Empirical bucky sanity check** gates the whole choice. If unshifted
   Lanczos on the 90-edge buckyball reproduces Phase 1's four smallest
   eigenvalues to 6 sig figs, the sign handling is fine at bucky scale and
   the question of whether it survives at L = 16 becomes an empirical one
   for the acceptance suite.

---

## 5. Crate choice

Current gigi `Cargo.toml` has only `nalgebra = "0.33"` for linear algebra —
no sparse deps in tree. Options:

**Option 1 — `sprs` + hand-rolled Lanczos** (recommended).
`sprs` is a pure-Rust sparse matrix crate (CSR, CSC, COO). It has no
eigensolver of its own but provides matvec, LU (via a companion `sprs-ldl`
for symmetric factorization), and integrates with `ndarray`. Writing IRL on
top of `sprs`'s matvec is ~500 LOC and stays inside the pure-Rust build
guarantee. No C/Fortran dependency, no arpack-ng install pain, cross-compiles
cleanly to Fly's build image (which is the current gigi-stream deploy target
per Phase 1's deploy receipt).

**Option 2 — `arpack-ng` via `sparse-eigen` or `arpack-rs`**.
Wraps the battle-tested Fortran ARPACK library. Robust, but adds a native
build-dep chain (Fortran + BLAS + LAPACK) that gigi does not currently pull.
The Fly image would need `apt install libarpack2-dev`. This is the fast path
for correctness confidence, at the cost of build-system complexity.

**Option 3 — `nalgebra-sparse` + iterative eigensolver from `argmin`**.
`nalgebra-sparse` matches the existing `nalgebra` dep and is the natural
extension. It has CSR/CSC/COO but no built-in Lanczos. Combined with `argmin`
for the iteration loop this stays in the nalgebra family but is more
plumbing than option 1.

**Recommendation**: **Option 1 (`sprs` + hand-rolled Lanczos)** as Phase 2 v1.
Rationale: preserves the pure-Rust build, keeps the Fly deploy simple,
matvec cost dominates iteration count for the sizes we care about so a
Fortran-optimized inner loop is not on the critical path. Keep Option 2 in
reserve as Phase 2.1 if the hand-rolled convergence rate proves inadequate
on the L = 16+ acceptance target.

---

## 6. API surface

Extend the existing `SPECTRAL_GAUGE` GQL verb:

```
SPECTRAL_GAUGE <bundle> ON FIBER (q0, q1, q2, q3)
  [GROUP SU(2) | SU(3) | U(1) | ...]
  [MODE dense | sparse]
  [LIMIT k]
  [SHIFT σ]
  ;
```

**New parameters**:

* `MODE sparse` — triggers IRL over CSR. `MODE dense` — Phase 1 fallback,
  which is what happens today when `MODE` is omitted; Phase 2 keeps this as
  the default until the sparse path is soaked, at which point the default
  flips to `sparse` when V > 4096.
* `LIMIT k` — number of smallest-magnitude eigenvalues to return.
  Default k = 4. Populates the `eigenvalues: Vec<f64>` field on the result
  (Phase 1 leaves this `None`). The `gap` field remains the smallest-
  magnitude eigenvalue above tolerance for continuity with Phase 1
  consumers.
* `SHIFT σ` — shift-invert center. Default `σ = 0` (near-zero targeting).
  Ignored when `MODE dense`.

**Result type** (extends Phase 1):

```rust
pub struct SpectralGaugeResult {
    pub gap: f64,                        // smallest-magnitude eigenvalue above tol
    pub eigenvalues: Option<Vec<f64>>,   // Some(vec) when MODE sparse; ascending by |λ|
    pub n_records_used: usize,           // as Phase 1
    pub group_used: crate::gauge::Group, // as Phase 1
    pub mode_used: SpectralGaugeMode,    // NEW: Dense | SparseLanczos { shift, k }
    pub convergence: Option<Convergence>,// NEW: iters, final residual — sparse only
}

pub enum SpectralGaugeMode {
    Dense,
    SparseLanczos { shift: f64, k: usize },
}

pub struct Convergence {
    pub iterations: usize,
    pub restarts: usize,
    pub final_residual: f64,  // max over the k returned pairs of ||L v - λ v|| / ||v||
}
```

**Errors** (new variants added to `SpectralGaugeError`):

* `LanczosDidNotConverge { iters, residual }` — hit the iteration cap with
  residual above tolerance.
* `ShiftFactorizationSingular { shift }` — sparse LU on `(L_A - σI)` returned
  a zero pivot; caller should try a small σ perturbation.
* `SparseUnavailable` — the build was compiled without the `sparse-lanczos`
  feature flag (Phase 2 lands behind an off-by-default cargo feature until
  soaked; the flag becomes default in Phase 2.1).

---

## 7. Acceptance criteria

All four gates must be green before Phase 2 lands on `main`:

1. **Bucky parity**. `SPECTRAL_GAUGE ... MODE sparse LIMIT 4` on the buckyball
   (V = 60, 90 edges, SU(2) fibers from the existing Phase 1 test fixture)
   returns the same 4 smallest-magnitude eigenvalues as
   `SPECTRAL_GAUGE ... MODE dense LIMIT 4` (the Phase 1 dense path extended
   to return `k=4`) to **6 significant figures**. Test lives in a new
   `tests/spectral_gauge_sparse.rs` alongside the Phase 1
   `spectral_gauge_basic` gate.

2. **L = 8 OBC latency**. `SPECTRAL_GAUGE` `MODE sparse LIMIT 4` on a 4D SU(2)
   L = 8 OBC gauge bundle (V = 16 384 edges) completes in **< 30 s** wall
   clock on a single Fly.io shared-cpu-4x machine (2.6 GHz Intel Cascade
   Lake baseline). Test: benchmark in `benches/spectral_gauge_sparse_bench.rs`.

3. **L = 16 OBC latency**. Same verb on a 4D SU(2) L = 16 OBC bundle
   (V = 262 144 edges) completes in **< 300 s** on the same target.

4. **L = 16 OBC memory**. Peak RSS during the L = 16 run stays **< 4 GB**.
   Measured via `procfs::Process::stat().rss` sampled at 100 ms and reported
   in the bench output.

5. **Regression**. All Phase 1 gates (G1, G2, G4, G5, G7, G8, G9, G11, G12,
   G13) stay green. `spectral_gauge_basic` in particular MUST NOT change —
   the Phase 1 dense path is not touched by Phase 2; the sparse path is a
   parallel arm dispatched on `MODE sparse`.

---

## 8. Open questions to flag

These are the questions that this spec has not resolved and that need
empirical answers or design decisions before / during implementation:

* **Does the signed-Laplacian indefiniteness break unshifted Lanczos?** The
  bucky parity gate answers this at V = 60, but the L = 8 OBC gate is
  where it actually gets tested at scale. If unshifted Lanczos fails
  convergence at L = 8, shift-invert becomes non-optional and the sparse
  LU dep-chain question moves to the critical path. Empirical answer
  required; do not pre-commit to Path A vs Path B without measuring.

* **How to preserve orthogonality across iterations when q0 sign-flips?**
  Full reorthogonalization at every step is the safe default and is
  affordable at k = 4, but at k = 20+ (e.g. if we later want the full
  low-frequency spectrum) it becomes O(m² · V) and eats the sparse win.
  Selective reorthogonalization (Simon 1984) is the correct answer for
  k > 4; deferred until Phase 2.1 unless the k = 4 gate reveals it early.

* **Should the API expose the full small-magnitude spectrum or just the gap?**
  Phase 1 returns only `gap`. Phase 2's `LIMIT k` returns the k smallest-
  magnitude eigenvalues, but Halcyon's downstream Wilson mass-gap pipeline
  may want ALL eigenvalues below a magnitude cutoff (`WHERE |λ| < ε`)
  rather than a fixed count. Add an alternate parameter `WITHIN ε` in Phase
  2.1? Or leave that shape for a future SPECTRAL_GAUGE_BAND verb? Not
  decided.

* **Cross-seed L = 24 OBC ensemble follow-up**. The v4 paper's downsampling
  preview (path 2 of the current workflow) reports a per-sector gap
  structure. Once Phase 2 sparse lands, run
  `SPECTRAL_GAUGE ... MODE sparse LIMIT 20` on the full-volume cross-seed
  L = 24 ensemble (V = 1 327 104) and check whether the sector-dependent
  gap plateau the downsampling reported survives at the full-volume
  extraction. If it does not — if the plateau was an artifact of
  downsampling — the v4 conclusion needs revisiting. This is the first
  scientific measurement Phase 2 unlocks and it belongs on the gate list
  for Phase 2's ship report, not on this spec's acceptance criteria.

* **Shift-invert vs sparse LU memory blowup at L = 24**. Fill-in on a 4D
  cubic-lattice Laplacian scales as ~V^{4/3} for nested dissection; at
  L = 24, V = 1.3M this is ~1.6M × the nonzero count per row of the factor.
  A rough estimate puts the LU factor near 30 GB. If shift-invert is
  needed at L = 24 the answer is likely iterative-inner-solve (MINRES) or
  a lower-fidelity Jacobi-Davidson variant. Deferred to L = 24 scaling
  work.

* **U(1) and SU(3) — does the same sparse path work unchanged?** The
  weight `w_e = Re Tr(U_e) / N` is a scalar; the Laplacian structure is
  group-independent. The sparse path should work verbatim for U(1) and
  SU(3) fibers. Add coverage in the acceptance suite for at least one
  U(1) case (small-lattice sanity) once the SU(2) L = 16 gate is green.

---

## 9. Non-goals for Phase 2

Explicitly out of scope:

* SU(3) Gell-Mann tangent basis (width 8) group inference — still a typed
  error, deferred to Phase 3.
* GPU/wgpu matvec backend — the `E_BACKEND_BOUND_SPECTRAL` slot in
  `src/spectral.rs` reserves budget for it but the Phase 2 sparse path is
  CPU-only.
* SPECTRAL_GAUGE_BAND (magnitude-cutoff variant) — see open questions.
* Rewriting the Phase 1 dense path. Phase 2 is an additive `MODE sparse`
  arm. `MODE dense` and the current test suite are frozen.

---

## 10. Cross-refs

* `SPECTRAL_GAUGE_PHASE1_SHIPPED_2026-06-28.md` — Phase 1 ship report,
  including the "Phase 2 deferral list" that lines up with this spec.
* `src/spectral.rs` §§ 921–1245 — Phase 1 dense implementation, including
  the `SpectralGaugeResult`, `SpectralGaugeError`, and `spectral_gauge_gap`
  surfaces Phase 2 extends.
* `HALCYON_BRIDGE_TRILOGY_2026-06-28_SHIPPED.md` — the surface upstream of
  SPECTRAL_GAUGE (INGEST + gauge-field bundle schema) that Phase 2 does
  not touch.
* `cfeb5c5` — the reply that locked the Phase 1 / Phase 2 split and the
  bundle-subsystem home.

---

_Ship blocker: cargo dep choice (Option 1 `sprs` per §5). Everything else
follows._
