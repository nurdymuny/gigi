# SPECTRAL_GAUGE BULK — dense arm shipped, sparse-interior arm scored

**From**: GIGI substrate
**To**: Bee (dependency decision) + Hallie (RH sweep)
**Date**: 2026-07-17
**Re**: Hallie's 2026-07-17 ask + correction — interior center-window (BULK) for the 3D magnetic-lattice RH statistics
**Status**: Part 1 (dense BULK + opt-in 8192) is on a worktree branch, tests green, NOT merged. Part 2 (sparse interior) is a design spike — report only, no code, no dependency added.

---

## 0. The correction that reframes everything

The June-30 Phase-2 sparse arm (spec `SPECTRAL_GAUGE_PHASE2_SPEC.md`) did shift-invert
at **σ = 0** → the smallest-|λ| eigenvalues = the **bottom** of the spectrum = the
Yang-Mills **mass gap**. That is the right object for the mass-gap pipeline and the
wrong object for Riemann-Hypothesis / number-variance statistics.

RH statistics live in the **BULK**: a contiguous window of *consecutive* levels at the
spectral **center**, unfolded, where the local spacing statistics are universal (GUE for
the magnetic/Hermitian operator). Phase 2.1-for-RH therefore needs an **interior**
eigensolver — a window around the center — **not** the σ = 0 bottom-of-spectrum solver
the spec designed. This document splits the answer into the arm that is *already done*
(dense BULK, this run) and the arm that is a *real build needing your ratify* (sparse
interior).

---

## PART 1 — Dense BULK: shipped, ready, unblocks two of three 3D points now

### Grammar (live on the branch)

```
SPECTRAL_GAUGE <bundle> [WHERE …] ON FIBER (theta) GROUP U(1) MODE MAGNETIC
    BULK <k> [AROUND <σ>] [IN [<a>,<b>]] ;
```

BULK is a peer of FULL (mutually exclusive; both parse-time-rejected together). It is a
**re-centering slice on the already-sorted dense magnetic spectrum** the FULL path
computes — *no re-solve*. It requires `MODE MAGNETIC` this phase (the complex-Hermitian
spectrum is the RH object; the real cos-weight bulk is out of scope). The response row
carries the window plus its locator:

```
{ gap, n_records_used, group_used,
  eigenvalues,            // the k-level window, ascending
  mode_used = "dense",
  bulk = true,
  bulk_center,            // the center VALUE used
  bulk_center_index,      // its index in the full spectrum
  bulk_lo, bulk_hi }      // [lo, hi) window range (len = hi − lo)
```

### Three centerings

| clause | semantics | center reported |
|---|---|---|
| `BULK k` | the k **centermost** levels, contiguous | positional-median eigenvalue `vals[⌊V/2⌋]` |
| `BULK k AROUND σ` | the k levels **nearest σ** by value, contiguous | σ |
| `BULK k IN [a,b]` | **ALL** levels in the closed interval `[a,b]`; k is a safety clamp | interval midpoint `(a+b)/2` |

### Two definitions pinned (load-bearing — read these)

**Center := positional MEDIAN, not midrange.** Auto-center is the eigenvalue at index
`c = ⌊V/2⌋` of the ascending spectrum, and the window is the k consecutive levels
straddling `c` (`lo = clamp(c − ⌊k/2⌋)`, `hi = lo + k`). It is **not** the midrange
`(λ_min + λ_max)/2`. Two reasons, both about what "bulk" means:

1. **Robustness to an asymmetric DOS.** The magnetic-Laplacian density of states is
   semicircle-ish but skewed; the midrange is dragged by a single extreme `λ_max`/`λ_min`
   into a low-density tail, whereas the median tracks where levels are *densest by count*
   — the true bulk center. The shipped test uses the star `K_{1,5}` (spectrum
   `{0,1,1,1,1,6}`) where median = 1 and midrange = 3 diverge hard, and pins the center to
   the median.
2. **Contiguity for free.** An index-based center makes the returned window *exactly* the
   k consecutive levels at the middle-by-count — precisely the object number variance /
   level-spacing ratios require (k consecutive *unfolded* levels straddling the center),
   with no "nearest"-tie ambiguity.

`AROUND σ` is by *value* (k nearest σ, then contiguity-checked); auto is by *index*.

**`IN [a,b]` := all eigenvalues in the CLOSED interval.** An energy-bounded interval is
inherently a contiguous consecutive-level window — exactly what number variance in a
fixed energy band wants. `k` is only a safety clamp: if `[a,b]` holds more than k levels,
we return the k nearest the interval midpoint (still contiguous, still inside `[a,b]`).
`a > b` is a typed `InvalidInterval` error. This is cleaner than "k nearest the midpoint"
as the primary semantics, and it matches the completeness intent: you name the band, you
get every level in it.

### Opt-in 8192 ceiling (the L=20 unblock)

Dense complex-Hermitian eigensolve is `16·V²` bytes for the matrix alone, plus a same-size
eigenvector matrix, plus tridiagonalization workspace, at `O(V³)` compute:

| L | V | matrix (16·V²) | realistic peak RSS | dense verdict |
|---|---|---|---|---|
| — | 4096 | 0.27 GB | 0.5–0.75 GB | safe (current default ceiling) |
| **20** | **8000** | **1.02 GB** | **2–3 GB** | laptop-risky → **opt-in only** |
| — | 8192 | 1.07 GB | 2–3 GB | edge of laptop-safe (opt-in max) |
| 24 | 13824 | 3.06 GB | 6–9 GB | over a laptop; tight on Fly |
| 32 | 32768 | 17.2 GB | 34–51 GB | flatly infeasible dense |

So the dense ceiling stays **4096 by default** and can be raised to **8192** by setting
`GIGI_DENSE_CEIL` — a machine-safety knob (laptop vs Fly), so it lives in the environment,
not the query. The value is clamped to the safe band `[4096, 8192]`: it can only *raise*
the ceiling, never lower the 4096 floor, never exceed 8192. Refused-by-default: a request
in `(4096, 8192]` without the opt-in returns a typed error that names the memory cost
**and** the knob:

> SPECTRAL_GAUGE: FULL/BULK on V = 4201 vertices exceeds the dense eigensolver ceiling
> (in force: V = 4096, spec §6 boundary 4096). Opt in to a higher dense ceiling up to 8192
> by setting GIGI_DENSE_CEIL — but note the memory cost: a V ≈ 8000 complex-Hermitian
> Laplacian is ~1 GB for the matrix plus ~1 GB for eigenvectors (~2–3 GB peak RSS, O(V³)
> work) and can OOM a laptop, which is why the default stays 4096. For V beyond 8192 the
> sparse interior Lanczos arm ships in Phase 2.1 …

`GIGI_DENSE_CEIL=8192` makes `dense_full_allowed(8000) = Ok` — **L=20 (V=8000 < 8192) is
unblocked.**

### What Part 1 unblocks *right now*

**Yes — dense + opt-in 8192 gives Hallie two of the three 3D points immediately:**

- **V = 4096** — already dense-side, no opt-in needed.
- **L = 20, V = 8000** — dense with `GIGI_DENSE_CEIL=8192` set (2–3 GB peak; run it on Fly
  or a ≥16 GB box, not a thin laptop).

That is enough to *start* the linear-vs-quadratic extrapolation with two points while the
interior sparse arm for **L = 24 (V = 13824)** and **L = 32 (V = 32768)** is decided. Those
two remaining points cannot be reached by the dense complex-Hermitian path at all (see the
table) — they are the entire reason the sparse arm exists.

---

## PART 2 — Sparse interior eigensolver: the real question, scored

**The problem:** k contiguous **interior** (bulk-center) eigenvalues of a large sparse
**complex Hermitian** magnetic Laplacian at V = 13824 and 32768, with a **completeness
guarantee** (Hallie's ask #3: no missed levels in the window — number variance is a count,
so a single skipped level corrupts it), inside gigi's constraint: a **pure-Rust build**
for the Fly Docker image. Adding a C/Fortran dependency (ARPACK-ng, SuiteSparse, MKL)
changes that build and is a posture decision, not a code decision.

### The dependency wall (the single fact that decides the ranking)

There is **no production-quality pure-Rust complex sparse indefinite LDLᵀ/LU** today.
- `sprs`/`sprs-ldl`: real-valued, (quasi-)SPD-oriented — not complex, not robust for the
  indefinite `(A − σI)`.
- `nalgebra-sparse`: CSR/CSC/COO + spmv + a real SPD `CscCholesky` — no complex, no
  indefinite LDLᵀ, no pivoted LU for interior shift-invert.
- `faer`: the only pure-Rust crate with genuinely production-grade sparse factorization
  and `c64` support — but its **complex sparse *indefinite* symmetric LDLᵀ + signed-inertia**
  path is not a soaked, guaranteed feature for this use. It is a research-grade bet, not a
  drop-in, and it would still be a **new dependency**.

Every method that must factorize the complex Hermitian indefinite `(A − σI)` — shift-invert
Lanczos (A), inertia/spectrum-slicing (C), FEAST node solves (D) — hits this wall. The one
method that does **not** need a factorization is polynomial-filtered subspace iteration (B):
it needs only sparse matvec + a dense small-matrix eigensolve, both of which gigi already
has, pure-Rust, in-tree.

### Scoring table

| | A. Shift-invert Lanczos (σ in bulk) | **B. Chebyshev-filtered subspace** | C. Spectrum-slice / inertia | D. FEAST / contour |
|---|---|---|---|---|
| **Pure-Rust buildable?** | ✗ (needs complex indefinite LDLᵀ/LU) | ✅ **yes — spmv + dense Ritz + reorth, all in-tree** | ✗ (needs signed complex LDLᵀ) | ✗ (needs complex node solves) |
| **New dependency?** | faer (unproven) or ARPACK-ng+SuiteSparse (C/Fortran) | **none** | faer (unproven) or SuiteSparse/MKL PARDISO | MKL FEAST (C) or a stack of complex solves |
| **Completeness mechanism** | shift-invert clusters near σ, but interior gaps *between* Ritz values can hide missed levels — needs care | **count Ritz values in the target window vs a subspace dim > window count; residual-gate; inertia cross-check if a factorization is ever added** | **gold standard — literally *counts* levels below σ via signed pivots (Sylvester)** | stochastic subspace estimate + count; robust but node-solve-bound |
| **Effort** | high (factorization + IRL + sign handling) | **medium-high (complex CSR spmv ~30 LOC + fixture-validated filter design + block subspace iteration + reorth + count validation; several hundred LOC)** | high (factorization + bisection driver) | high (contour + node solves + subspace) |
| **V=32768, k=4000 on Fly** | LU fill-in on the 3D Laplacian is the memory driver (spec §8 flags ~30 GB LU at L=24 in 4D; 3D is milder but large) | **feasible; cost driven by filter degree × block width. k=4000 at the exact center is the heavy end (block subspace ~2–4 GB, heavy spmv over minutes–hours); narrower windows are much cheaper** | count is cheap per shift, but each shift needs a factorization (fill-in memory) | powerful but each quadrature node is a full complex solve — the same wall ×(#nodes) |
| **Verdict** | dep wall; iterative inner solve (GMRES/MINRES, pure-Rust) dodges it but needs a preconditioner and loses clean completeness | **RECOMMENDED — the constraint-satisfying path** | best *count* guarantee, but dep wall | powerful, pure-Rust-hard, dep wall |

### Recommendation: **Option B (Chebyshev-filtered subspace iteration)**

It is the pure-Rust answer and it reuses machinery already in the repo:
- **complex sparse matvec** — build a CSR/COO of `Complex<f64>` from the same edge list the
  executor already assembles (`src/spectral.rs`, the magnetic branch); spmv is ~30 LOC;
- **dense Rayleigh-Ritz** on the projected subspace — `nalgebra::SymmetricEigen` already
  does the dense complex-Hermitian case *live* in the magnetic FULL branch;
- **full/selective reorthogonalization** — the exact twice-is-enough Gram-Schmidt pattern
  already exists in `src/sharded/spectral.rs` (the T7 Lanczos port).

**Completeness for B** is the whole point of spectrum-slicing methods: with a *validated*
filter (a Chebyshev polynomial that passes the bulk energy window and damps the rest) and a
subspace dimension larger than the window count, you **prove no misses by counting** — the
number of converged Ritz values inside the window must match, and stay stable as you widen
the subspace, with a residual gate on each pair. The bulletproof cross-check is Option C's
inertia count (levels below σ via signed LDLᵀ pivots); if we ever add the factorization,
B + inertia is airtight. Without it, B's count + residual + subspace-headroom is the honest
completeness story, and it is the standard one for FEAST/filter methods.

**Fallback if a C dependency becomes acceptable:** ARPACK-ng shift-invert (Option A) or MKL
FEAST (Option D). Both are mature and would be faster to *correctness* than hand-rolling B —
but both **change the Fly Docker build** from pure-Rust to +Fortran/BLAS/LAPACK (ARPACK-ng)
or +MKL (FEAST), and that is a dependency-posture decision only Bee makes.

### Honest framing — this is a real build, not a wiring change

The sparse interior arm is **several hundred lines of new numerics** — complex CSR spmv,
Chebyshev filter design (degree vs window-width tradeoff, validated on a fixture with a
known interior spectrum), block subspace iteration, reorthogonalization, and a count/inertia
completeness validator — **plus** a fixture-validated filter. It is *not* a re-wiring of the
dense path. Before it starts it needs Bee to ratify:

1. **Approach** — pure-Rust Option B (no dep, more of our LOC, completeness by counting), or
   accept a C/Fortran dependency (ARPACK-ng / MKL FEAST) that changes the Docker build.
2. **Dependency posture** — is the Fly image allowed to stop being pure-Rust? That is the
   single decision that flips the whole ranking. If pure-Rust is a hard constraint (it has
   been), Option B is the *only* constraint-satisfying path and the answer is "we build B."

Nothing about the sparse arm blocks Hallie today: **the dense path with the opt-in 8192
ceiling already delivers V = 4096 and L = 20 (V = 8000)** — two of the three 3D points — so
the linear-vs-quadratic extrapolation can start now, in parallel with this decision.

---

## Cross-refs

- `src/spectral.rs` — dense BULK (`compute_bulk_window`, centering rules), the opt-in
  ceiling (`dense_ceiling` / `dense_full_allowed` / `DENSE_CEIL_OPTIN_MAX`), and the
  `BulkSpec`/`BulkCenter`/`BulkWindow` types.
- `tests/spectral_bulk_basic.rs` — the 16-test dense-bulk suite (anchors a–f + opt-in).
- `SPECTRAL_GAUGE_PHASE2_SPEC.md` §4/§8 — the σ = 0 shift-invert design (the *bottom*-of-
  spectrum arm) and the open questions (full-vs-selective reorthogonalization; the ~30 GB
  LU-factor memory blow-up at L = 24) — consistent with this survey's dep-wall conclusion.
- `SPECTRAL_PHASE2_MAGNETIC_SHIPPED_2026-07-16.md` — the FULL + MODE MAGNETIC ship the
  dense BULK extends.
