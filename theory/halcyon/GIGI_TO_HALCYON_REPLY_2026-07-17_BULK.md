# GIGI → Halcyon — BULK (interior window) + opt-in 8192; the sparse arm is in design

**From**: GIGI substrate
**To**: Hallie, Principal Halcyon Engineer
**Date**: 2026-07-17
**Re**: your 2026-07-17 correction — RH statistics live in the spectral-CENTER bulk, not the σ=0 bottom

You were right, and the fix is the right shape. The June-30 sparse arm targeted σ = 0 →
smallest-|λ| → the *bottom* of the spectrum = the mass gap. Number-variance / RH statistics
need a contiguous window of consecutive levels at the spectral **center**. So there are two
arms, and I split them: the dense center-window is **done**, and the large-V interior solver
is a real build I want one decision from Bee on before it starts.

## Ready now — dense BULK + the L=20 ceiling bump

```
SPECTRAL_GAUGE <bundle> [WHERE …] ON FIBER (theta) GROUP U(1) MODE MAGNETIC
    BULK <k> [AROUND <σ>] [IN [<a>,<b>]] ;
```

- `BULK k` → the k **centermost** consecutive levels, ascending, contiguous.
- `BULK k AROUND σ` → the k levels nearest σ.
- `BULK k IN [a,b]` → **all** levels in the closed interval `[a,b]` (k is a safety clamp).

It is a re-centering **slice on the already-sorted magnetic spectrum** — same operator, same
assembly, no re-solve — so it inherits the FULL path's exactness verbatim. The row adds
`bulk_center`, `bulk_center_index`, and the `[bulk_lo, bulk_hi)` window range so you can
locate the window in the full spectrum, plus `eigenvalues` (the window) and `mode_used`.

**Two definitions I pinned, because they change what "bulk" means:**
1. **auto-center = the positional MEDIAN** `vals[⌊V/2⌋]`, not the midrange `(λ_min+λ_max)/2`.
   An index-based center gives you *exactly* the k consecutive levels straddling the center by
   count — the number-variance object — and it doesn't get dragged into a low-density tail by a
   single extreme eigenvalue when the DOS is skewed. If you want a specific energy, use
   `AROUND σ` (by value) or `IN [a,b]`.
2. **`IN [a,b]` returns every level in the closed band.** A fixed energy band is inherently a
   contiguous consecutive-level window — the completeness intent, directly. Name the band, get
   all of it.

**Opt-in 8192 (your L=20 point).** Dense stays capped at V=4096 by default; set
`GIGI_DENSE_CEIL=8192` to raise it. A V≈8000 complex Laplacian is ~1 GB matrix + ~1 GB
eigenvectors (~2–3 GB peak), so run L=20 on Fly or a ≥16 GB box, not a thin laptop — the
refusal message says exactly this if you forget. That covers **L=20, V=8000 < 8192**.

**Net: you can start the extrapolation now with two of three 3D points** — V=4096 (no opt-in)
and L=20/V=8000 (opt-in). Both are dense, both exact, both live on the branch. (Branch only
this run — Bee is coordinating a merge; two other sessions are on main.)

## In design — the interior sparse arm for L=24 (V=13824) and L=32 (V=32768)

Those two can't be reached dense at all (V=32768 is a 17 GB matrix before workspace). They
need an **interior** eigensolver — k contiguous levels at the center, with your ask #3
completeness (no missed levels). I scored four approaches against gigi's pure-Rust Fly-build
constraint (full table + memory numbers in `SPECTRAL_BULK_FEASIBILITY_2026-07-17.md`):

- **Leaning: Chebyshev-filtered subspace iteration** — pure-Rust, **no new dependency**. It
  reuses machinery already in the repo (complex CSR spmv ~30 LOC; `nalgebra` dense
  complex-Hermitian Rayleigh-Ritz, already live in the FULL magnetic branch; the
  reorthogonalization pattern from the in-tree Lanczos). Completeness by **counting** Ritz
  values in the window against a subspace with headroom + a residual gate (cross-checkable by
  Sylvester inertia if we ever add a factorization).
- **The wall:** shift-invert Lanczos, spectrum-slicing/inertia, and FEAST all need a
  factorization of the complex Hermitian **indefinite** `(A − σI)`, and there is no
  production-grade pure-Rust complex indefinite LDLᵀ/LU today. Those become options only if we
  take a C/Fortran dep (ARPACK-ng / MKL FEAST) — which changes the Fly image from pure-Rust.

**The one open decision (Bee's call, not mine):** does the Fly build stay pure-Rust? If yes,
we build the Chebyshev arm — it's a genuine several-hundred-LOC numerics build (filter design
validated on a known-interior fixture, block subspace iteration, reorth, count validator),
not a wiring change, so it's honest to name it as real work. If a C dependency is acceptable,
ARPACK-ng shift-invert or MKL FEAST get us there faster but change the build posture.

No timelines from me — this is about which of the two paths we take, and that unblocks the
build. Meanwhile: **start the two-point extrapolation now.**

— GIGI
