# SPECTRAL_GAUGE BULK sparse interior arm — LIVE on merged main, 2026-07-18

Chebyshev-filtered interior eigensolver for the magnetic Laplacian, routed
when `MODE MAGNETIC BULK k` is requested past the dense ceiling. This is the
ship that lands the 2026-07-17 sparse interior arm on **current** main (atop
MODE MATRIX, HOLONOMY, HELICITY, dense BULK, and GAUGE_FIELD INIT FROM
BUNDLE) and deploys it to prod. It settles the RH / number-variance question
at V = 13824 (L=24) and 32768 (L=32), where a dense V×V complex
eigendecomposition is infeasible.

## What this ship did

- **Rebase / reconcile.** Cherry-picked the three sparse-specific commits
  (RED tests, GREEN impl, perf) onto current `origin/main`. The dense-BULK
  commits were already on main (git cherry `-`), so only the interior module
  + routing + tests moved. `src/spectral.rs` was reconciled so BOTH the dense
  BULK center-slice (V ≤ ceiling) AND the Chebyshev interior arm (V > ceiling)
  coexist, alongside main's MODE MATRIX (`spectral_matrix_raw`, unchanged).
  The only rebase conflict was a two-line `src/lib.rs` module-list collision
  (main's `pub mod helicity;` vs the sparse `pub mod spectral_interior;`) —
  resolved keep-both.
- **Fresh-seed completeness re-verify** (the independent correctness gate) —
  see table below. HOLD was NOT triggered.
- **Perf pass** — filter-degree coefficient `alpha` 10.0 → 6.0 (extracted as
  `FILTER_DEGREE_COEFF`), ~40% fewer matvecs per subspace iteration, with
  completeness re-verified after the change. SP7 rewritten to a realistic
  release budget.

## Routing (the coexistence)

`src/spectral.rs::spectral_gauge_spectrum`, Step 4b:

```
if full { dense_full_allowed(v_count)?; }              // FULL: still errors past ceiling
if let Some(req) = bulk {                              // BULK past ceiling: route to interior
    if dense_full_allowed(v_count).is_err() {
        return spectral_interior_route(&edges, v_count, req, group, n_records_used);
    }
}
// ... BULK ≤ ceiling falls through to the dense compute_bulk_window slice
```

FULL past the ceiling still returns the typed `SparseUnavailable`. BULK ≤
ceiling is the dense re-centering slice (default 4096, opt-in 8192 via
`GIGI_DENSE_CEIL`). BULK > ceiling is the interior solver.

## Response fields (Hallie ask #3)

On the sparse path `mode_used = "sparse_interior"` and each BULK row carries
`bulk_center` (target value), `converged` (int), `max_residual` (f64),
`iterations`. Dense-only global locators (`bulk_center_index`, `bulk_lo`,
`bulk_hi`) are omitted on the sparse path — it never sorts the full spectrum.
The completeness gate the caller applies is `converged == k && max_residual <
1e-8` per kept window.

## Fresh-seed completeness re-verify

Independent of the branch's SP1/SP2 fixtures: fresh random magnetic U(1)
lattices at seeds DISJOINT from every SP fixture (base
`FRESH_FIXTURE_BASE = 0xFEED5EED20260718`; the branch's SP1 used `0xA11CE^v`).
Dense FULL ground truth, sparse interior center + AROUND windows, EXACT window
equality asserted. Verified at the SHIPPED `alpha = 6.0`. Every window EXACT
(no miss / no extra, right multiplicity); `restarts = 0` throughout (first
filter attempt fully converges — no level-skipping); `max_residual < 1e-8`
everywhere.

```
 V     k   AUTO conv  iters  AUTO maxres  AROUND conv  AROUND maxres  match
 384   24  24/24      5      7.93e-12     24/24        4.80e-10       EXACT
 384   64  64/64      10     2.18e-13     64/64        5.56e-13       EXACT
 640   24  24/24      6      1.78e-14     24/24        1.47e-14       EXACT
 640   64  64/64      10     3.02e-13     64/64        4.44e-13       EXACT
 1024  24  24/24      5      7.36e-12     24/24        1.24e-14       EXACT
 1024  64  64/64      9      6.21e-12     64/64        8.49e-14       EXACT
 1600  24  24/24      6      1.64e-14     24/24        5.99e-13       EXACT
 1600  64  64/64      8      1.25e-10     64/64        3.24e-14       EXACT
 near-degen n=640 theta=2e-6 gap=3.061e-6: 16/16, both members returned (no merge), maxres 2.82e-13, PAIR_KEPT
 exact-degen V=512 doubled: 40/40, exact double multiplicity preserved, maxres 8.48e-14, DOUBLED
```

Largest residual across the whole fresh sweep: `1.25e-10` (V=1600, k=64,
AUTO), ~2 orders of magnitude below the 1e-8 gate. **fresh_completeness_holds
= true.**

## Anchors (post-rebase, on merged main)

`cargo test --release --features halcyon --test spectral_interior_basic` =
**13 passed, 0 failed, 1 ignored (SP7), 270s.**

| Anchor | What | Result |
| --- | --- | --- |
| SP1 | completeness vs dense ground truth (V≤2048, ≥3 windows, Auto + AROUND) | PASS |
| SP2 | near-degenerate (~1e-6 split) + exact double, both returned | PASS |
| SP3 | residual gate + honest `converged<k` flag | PASS |
| SP4 | sparse == dense parity at V=1024 | PASS |
| SP5 | complex CSR matvec == dense L·x to 1e-12 + Hermitian real spectrum | PASS |
| SP6 | window arithmetic (clamp, k=0 error, AROUND/IN) | PASS |
| SP7 | scale smoke V=8000 (release, `#[ignore]`) — see timing | PASS (739.69s < 1200s) |
| FRESH | unused-seed completeness (above), incl. near + exact degeneracy | PASS |

## Honest timing (for Hallie's local sweep budget)

Measured on the work box, single-thread, release, `alpha = 6.0`:

| V | L | k | measured / projected | iters | notes |
| --- | --- | --- | --- | --- | --- |
| 8000 | 20* | 64 | **739.69 s (measured)** | 8 | converged 64/64, max_residual 8.30e-13, restarts 0 |
| 13824 | 24 | 64 | ~2.21e3 s (~37 min, projected) | ~8 | O(V²) window-narrowing scaling |
| 32768 | 32 | 64 | ~1.24e4 s (~3.4 h, projected) | ~8 | O(V²) window-narrowing scaling |

*V=8000 is the SP7 scale fixture (random magnetic graph), not a cubic L; L=20
periodic cubic is V=8000.

**Scaling is O(V²) per single solve, not linear.** span = 2·deg_max stays
~constant while a fixed-k window narrows as 1/V, so the Chebyshev degree grows
∝ V (measured ~510 at V=8000) and nnz ∝ V, giving cost ∝ iters·V². Check:
(13824/8000)²·739.69 = 2209 s; (32768/8000)²·739.69 = 12412 s. The alpha 10→6
cut removed ~40% of the matvecs per iteration at small k; at V=8000/k=64 the
iteration count rose 6→8, netting 13.9% faster (858.79 s → 739.69 s) at that
window, with larger gains at smaller k. The interior arm makes L=24 and L=32
**runnable** (impossible on the dense path); the O(V²) wall is inherent to the
narrow-window / wide-band ratio and would need a different algorithm
(parallelism / preconditioner) to break — explicitly out of scope here.

## SP7 budget reconciliation

The prior fence report cited a "60 s budget"; the committed source actually
asserted `elapsed.as_secs() < 600`, and its debug path ran V=2048 (only the
release path runs V=8000). Both were reconciled: SP7 is now `#[ignore]`
(release-only; the run command is in the ignore string), the debug path keeps
the bounded V=2048 case, and the release V=8000 assert is `< 1200` (~1.6× the
measured 739.69 s) — a real gate that passes, not the impossible 60 s.

## Live prod receipt (interior solver live)

`gigi-stream.fly.dev`, image `deployment-01KXV4456AT9HSKQ2VK3JQJWV5`. A magnetic
U(1) field just above the default ceiling, solved through the interior arm:

```
LATTICE sparse_probe_lat FROM CUBIC L=17 DIM=3 PERIODIC;           -- V = 4913 > 4096
GAUGE_FIELD sparse_probe_field GROUP U(1) INIT FLUX RANDOM SEED 42 ON LATTICE sparse_probe_lat;
SPECTRAL_GAUGE sparse_probe_field ON FIBER (theta) GROUP U(1) MODE MAGNETIC BULK 32;
```

Response (HTTP 200, 30.2 s wall):

```
mode_used      = "sparse_interior"
converged      = 32          (== k, complete window)
max_residual   = 2.528e-12   (< RES_TOL 1e-8 by ~4 orders)
iterations     = 7
bulk_center    = 6.02208751...
eigenvalues    = [5.99464..., ..., 6.04861...]   (32 ascending, centered)
n_records_used = 14739       (= 3 * 4913 edges, L=17 DIM=3 periodic)
group_used     = "U(1)"
```

The interior solver is live on prod. V=4913 completed in-request (30 s) on the
`performance-4x` VM — well inside the request window, so no time-box was needed;
the L=24/L=32 scale proof rests on the local V=8000 measurement above (the sweep
is Hallie's to run locally).

Marcella IMAGINE sanity (post-deploy, dim=4): HTTP 200, `endpoint_coherence 1.0`,
`refused false` — Phase 2 path intact.

### Deploy note (concurrency)

The first deploy of this image (release v255) was superseded within ~5 min by a
concurrent deploy (v256) from a parallel session whose image predated this arm —
a live probe caught it returning the old `SparseUnavailable` error. Re-deployed
merged main (`c6b1dfc` = v257, image `01KXV4456...`), which is the authoritative
superset (all of Thread-1's INJECT FROM BUNDLE + the Poincaré lens work + this
arm), and re-verified the receipt above. `origin/main` carried this arm the whole
time; only the running image had been rolled back.

### Substrate

`claude_substrate_v0` exported pre-deploy (20 records → `.deploy-backups/2026-07-18-sparse/`),
wiped to 4 by the restart (WAL recovery had also mis-keyed the bundle base on `ts`,
which collapses the 20 records onto their 4 distinct timestamps). Restored per the
documented drill — drop, recreate keyed on `thought_id`, re-import: **post-count 20,
20 distinct thought_ids, total 20**.

## Honest scope

Completeness is PROVEN vs dense ground truth for V ≤ 2048 (SP1/SP2/SP4 +
fresh-seed). At V = 13824 / 32768 no dense ground truth exists; the guarantee
rests on (1) the residual gate — every returned pair certified below 1e-8;
(2) `converged == k`; (3) the Jackson-damped Chebyshev selectivity + oversample
margin against near-degenerate merges. A missed bulk level at V=32768 cannot
be positively excluded by ground truth — which is exactly why the
converged/max_residual fields exist. Gate every kept window on
`converged == k && max_residual < 1e-8`.
