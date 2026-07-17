# SPECTRAL_GAUGE BULK sparse interior arm — shipped 2026-07-17

Chebyshev-filtered interior eigensolver for the magnetic Laplacian, routed
when `MODE MAGNETIC BULK k` is requested past the dense ceiling. Settles the
RH / number-variance question at V = 13824 (L=24) and 32768 (L=32), where a
dense V x V complex eigendecomposition is infeasible.

## What shipped

- `src/spectral_interior.rs` — the solver. Complex CSR magnetic Laplacian
  (~30-LOC matvec), Gershgorin band bracket, Jackson-damped Chebyshev
  bandpass filter, block subspace iteration with twice-is-enough complex
  reorthogonalization + Rayleigh-Ritz (`nalgebra::SymmetricEigen` on the
  small projected block), KPM DOS for the median-value / count-below
  estimates, and the completeness retry loop.
- Routing seam in `src/spectral.rs::spectral_gauge_spectrum`: when
  `magnetic && bulk.is_some() && dense_full_allowed(v).is_err()`, route to
  the interior solver instead of returning `SparseUnavailable`. FULL past
  the ceiling still errors (no sparse FULL arm this phase).
- Response fields (both the parser executor and the gigi_stream handler):
  `mode_used = "sparse_interior"`, `converged` (int), `max_residual` (f64),
  `iterations`. Dense-only locators (`bulk_center_index`, `bulk_lo`,
  `bulk_hi`) are omitted on the sparse path (it never sorts the full
  spectrum); `bulk_center` (the target value) is emitted.
- `tests/spectral_interior_basic.rs` — SP1..SP7.

Built ON TOP of the cherry-picked dense-BULK commits (7302da4 / a2b57f9 /
18d04b0 → BULK grammar + `GIGI_DENSE_CEIL` opt-in ceiling).

## The math

- Assembly matches the dense magnetic path EXACTLY: per edge `(i,j,theta)`,
  `L[i,j] -= e^{+i theta}`, `L[j,i] -= e^{-i theta}`, `diag[i] += 1`,
  `diag[j] += 1`. Self-loops skipped; parallel edges accumulate. (SP5 pins
  the CSR matvec to a dense `L·x` at 1e-12.)
- Gershgorin: every eigenvalue lies in `[0, 2·deg_max]` (PSD, row radius =
  degree). Guaranteed enclosure, zero matvecs; padded outward so nothing
  maps outside [-1,1] after the affine rescale.
- Filter degree grows as the window narrows relative to the full spectral
  width: `m ≈ alpha·(lambda_max - lambda_min)/Delta_lambda`, ×~1.4 for the
  Jackson transition widening, bumped on the completeness retry. Jackson
  damping suppresses the Gibbs ringing that would otherwise leak
  out-of-band levels into the subspace (the merge failure mode).
- Auto center = the KPM median-value estimate (bisection on the integrated
  DOS to rank floor(V/2)), consistent with the dense pinned positional
  median. AROUND sigma centers on sigma; IN [a,b] on the midpoint.

## Completeness mechanism (Hallie ask #3)

1. RESIDUAL GATE — `||L v - lambda v|| / ||v|| < RES_TOL = 1e-8` per kept
   pair. Certifies each returned value is a true eigenpair.
2. COUNT — converged-in-window must == k; short → grow subspace / raise
   degree (bounded retries); still short → return `converged < k` with
   `fully_converged = false`. Never silently claims k.
3. OVERSAMPLE — block dim `b = k + oversample`, `oversample >=
   max(0.4k, 60)`; the completeness margin against near-degenerate merges.
4. KPM count-below-edge — a cheap NON-authoritative contiguity sanity
   number (O(1/sqrt(probes)) noise), surfaced as `count_below_estimate`.

## SP1 ground-truth completeness table

For each (V, k, center) the sparse interior window EXACTLY equals the dense
FULL spectrum's center-k eigenvalues: same values (<= RES_TOL), same count,
same multiplicities, NO miss, NO extra. `EXACT` = the assert_window_eq gate
passed (length + every level within RES_TOL).

<!-- SP1_TABLE_PLACEHOLDER -->

## SP2 near-degenerate

- `~1e-6` split pair (uniform-flux magnetic cycle, per-edge theta = 1e-6):
  BOTH members returned, no merge — `<!-- SP2A -->`.
- EXACT degeneracy (two disjoint identical copies, every level doubly
  degenerate): multiplicity-2 preserved in the window — `<!-- SP2B -->`.

## Anchors

| Anchor | What | Result |
| --- | --- | --- |
| SP1 | completeness vs dense ground truth (V<=2048, >=3 windows, Auto + AROUND) | <!-- SP1R --> |
| SP2 | near-degenerate (~1e-6 + exact) both returned | <!-- SP2R --> |
| SP3 | residual gate + honest converged<k flag | <!-- SP3R --> |
| SP4 | sparse == dense parity at V the dense path handles | <!-- SP4R --> |
| SP5 | complex CSR matvec == dense L·x to 1e-12 | <!-- SP5R --> |
| SP6 | window arithmetic (clamp, k=0 error, AROUND/IN) | <!-- SP6R --> |
| SP7 | scale smoke V=8000, converged==k, residuals bounded | <!-- SP7R --> |

## Honest scope

Completeness is PROVEN vs dense ground truth for V <= 2048 (SP1/SP2/SP4). At
V = 13824/32768 no ground truth exists; the guarantee rests on the residual
gate (every returned pair a true eigenpair < RES_TOL) + `converged == k` +
the Chebyshev filter selectivity. A missed bulk level at V = 32768 cannot be
positively excluded by ground truth — which is exactly why the
converged/max_residual fields exist and why the honest gate is
`converged == k && max_residual < 1e-8` per kept window.

## Debug vs release

The dense ground-truth reference is O(V^3) COMPLEX — fast in release,
punishing in an unoptimized debug build. The automated gate caps the
ground-truth V at 1024 (SP1) / 512 (SP4) and the scale smoke at V=2048 in
debug; the full V ∈ {256,512,1024,2048} completeness ladder and the V=8000
scale proof run under `--release` (how this numerical suite is meant to be
exercised). Correctness is identical at every V.
