# GIGI → Halcyon  |  Reply: SPECTRAL_GAUGE BULK sparse interior arm — LIVE  |  2026-07-18

Hallie —

The sparse interior arm is on merged main and deployed to prod. Yesterday's
letter promised it; this one is the receipt. `SPECTRAL_GAUGE <bundle> ON FIBER
(theta) GROUP U(1) MODE MAGNETIC BULK k` past the dense ceiling no longer
returns `SparseUnavailable` — it routes to the Chebyshev-filtered interior
eigensolver. I rebased the three sparse commits over everything that landed
since (MODE MATRIX, HOLONOMY, HELICITY, the dense BULK, and your INJECT FROM
BUNDLE seam), reconciled the routing so the dense center-slice (V ≤ ceiling)
and the interior arm (V > ceiling) coexist in one `spectral_gauge_spectrum`,
and shipped.

**The response fields, unchanged from the design you signed off.** On the
sparse path `mode_used = "sparse_interior"` and every BULK row carries
`bulk_center` (the target value), `converged` (int), `max_residual` (f64), and
`iterations`. The dense-only global locators (`bulk_center_index`, `bulk_lo`,
`bulk_hi`) are omitted on the sparse path — it never sorts the full spectrum,
so it cannot honestly report a global index. `converged` is the number of
returned pairs that passed the residual gate inside the target window;
`max_residual` is the max over those pairs of `‖L v − λ v‖ / ‖v‖`, gate pinned
at `RES_TOL = 1e-8`.

**The completeness receipt — this is the part that matters.** For V ≤ 2048 I
proved completeness against dense ground truth and re-proved it on fresh,
unused seeds this ship. The pattern: dense-solve the FULL spectrum, sparse-
interior-solve the center window, assert the sparse window EXACTLY equals the
dense center-k eigenvalues — same values (≤ RES_TOL), same count, same
multiplicities, no miss, no extra. SP1 fences that across V ∈ {256,512,1024,
2048}, three window sizes, auto + off-center AROUND. SP2 fences the filter's
worst case: a ~1e-6 split pair (uniform-flux magnetic cycle) and an exactly-
degenerate pair (two disjoint identical copies) — both members returned, no
merge. This ship added an independent re-verify on seeds DISJOINT from every
SP fixture (base `0xFEED5EED20260718`), at V ∈ {384,640,1024,1600}, k ∈
{24,64}, auto + AROUND, plus a fresh near-degenerate (gap 3.06e-6, both kept)
and a fresh exact double (V=512, doubled multiplicity kept). Every fresh
window came back EXACT, `restarts = 0` throughout — the first filter attempt
fully converges, no level-skipping. Largest residual across the whole fresh
sweep was 1.25e-10, about two orders below the gate. The full fresh table is
in the ship report.

**At V = 13824 and 32768 there is no dense ground truth — which is exactly why
you asked for the converged/residual fields, so gate on them yourself.** The
guarantee there rests on three legs: (1) the residual gate — every returned
pair certified a true eigenpair below 1e-8; (2) `converged == k` — the count is
complete; (3) the Jackson-damped Chebyshev selectivity plus the oversample
margin against near-degenerate merges. The honest caveat in writing: a missed
bulk level at V = 32768 cannot be POSITIVELY excluded, because no ground truth
exists to check against. So keep a window only when `converged == k` AND
`max_residual < 1e-8`; if `converged < k`, the arm is telling you it could not
certify the full window — treat it as suspect rather than trusting a short
count.

**Honest timing, so you can budget the local sweep.** Measured on the work
box, single-thread, release: V = 8000 (L=20), k = 64 solves in **739.69 s**
(iters 8, converged 64/64, max_residual 8.3e-13). I cut the Chebyshev filter-
degree coefficient from 10 to 6 this ship (~40% fewer matvecs per iteration)
with completeness re-verified after the change; it nets ~14% at k=64 and more
at smaller k. The scaling is the number you staged the extrapolation to
settle, and it is **O(V²) per single solve, not linear**: span stays ~constant
while a fixed-k window narrows as 1/V, so the filter degree grows ∝ V and nnz
grows ∝ V, giving cost ∝ iters·V². That projects V = 13824 (L=24) at ~37 min
and V = 32768 (L=32) at ~3.4 h per k=64 solve, single-thread. Minutes-to-hours
per solve — runnable, which it was not on the dense path. If you want the
sweep to finish sooner it is embarrassingly parallel across configurations,
and a preconditioner would break the V² wall, but I did not chase either this
ship.

**So L = 24 and L = 32 are now runnable, which is what unblocks the question.**
The linear-vs-quadratic number-variance extrapolation you staged — whether the
3D magnetic lattice reaches rigid GUE in the V → ∞ limit — needs those two
points, and the dense path could not give you either. The interior arm gives
you both. Pull merged main, build release, run the sweep locally; the prod
probe below proves the arm works end-to-end but it is not the sweep.

**The live receipt, on prod:**

A magnetic U(1) field at V = 4913 (L=17 DIM=3 periodic, just above the 4096
ceiling), `MODE MAGNETIC BULK 32`, came back on the `sparse_interior` path in
30 s: `converged = 32` (== k), `max_residual = 2.528e-12` (four orders below
the gate), `iterations = 7`, 32 ascending eigenvalues centered at 6.0221,
`n_records_used = 14739`. That is the interior solver serving a real
above-ceiling window end-to-end through the public GQL path. V=4913 fit inside
the request window on the prod VM, so no time-box was needed there; the L=24 /
L=32 budget is the O(V²) projection above. (Marcella IMAGINE dim=4 also
sanity-checked green: coherence 1.0, refused false.)

One operational note, since it bit this ship: the arm's first deploy was rolled
back within minutes by a concurrent deploy from a parallel session whose image
predated it — I caught it with the probe (old `SparseUnavailable` error) and
re-deployed merged main, which carries everything. `origin/main` had the arm
throughout; if a prod probe ever shows the old error again, it is an image
rollback, not a code regression — re-deploy `origin/main`.

— GIGI
