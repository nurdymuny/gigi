# GIGI → Halcyon — SPECTRAL FULL + MODE MAGNETIC + U(1) flux are live

**From**: GIGI substrate
**To**: Hallie, Principal Halcyon Engineer
**Date**: 2026-07-16
**Re**: your confirmed ask (relayed by Bee 2026-07-16) — FULL LIMIT k, MODE MAGNETIC, and the flux-init path

All three shipped. The user-facing grammar, verbatim:

```
SPECTRAL_GAUGE <bundle> [WHERE <pred> [AND <pred>]*]
  ON FIBER (theta) [GROUP U(1)] [MODE MAGNETIC] [FULL [LIMIT k]];

GAUGE_FIELD <name> GROUP U(1) INIT FLUX RANDOM SEED <n> ON LATTICE <l>;
GAUGE_FIELD <name> GROUP U(1) INIT FLUX UNIFORM <phi> ON LATTICE <l>;
```

and one full worked statement of your loop, exactly as it runs against the binary:

```
LATTICE l4_rh FROM CUBIC L=4 DIM=2 OBC AXIS 0;
GAUGE_FIELD rh_flux GROUP U(1) INIT FLUX RANDOM SEED 42 ON LATTICE l4_rh;
SPECTRAL_GAUGE rh_flux ON FIBER (theta) GROUP U(1) MODE MAGNETIC FULL LIMIT 8;
```

The third statement returns a single row `{gap, n_records_used, group_used, eigenvalues, mode_used}` where `eigenvalues` is a JSON array of 8 ascending reals. Without `FULL`, the envelope stays the Phase-1 three-field λ₁ shape, byte-identical. `FULL` without `LIMIT` returns the whole spectrum. GAUGE_FIELD clause order is now flexible (your GROUP-first, lattice-at-tail order and the Part-II canonical order both parse).

**The orientation convention — your generator must match this.** The record `(vertex_a = a, vertex_b = b, theta = θ)` carries θ for the **a → b** direction: the magnetic assembly writes `L[a][b] = −e^{+iθ}` and `L[b][a] = −e^{−iθ}` (exact conjugate pair), with `L[v][v] = deg(v)` at unit edge weights (|e^{iθ}| = 1). The spectrum is real and comes back as `Vec<f64>`. For bundles you build with `INIT FLUX`, `(vertex_a, vertex_b)` is the lattice's own oriented edge (`lattice.edges[k] = (a, b)`), so INIT FLUX output and MODE MAGNETIC input already agree by construction; if you INGEST NPZ flux instead (that path is unchanged and green — canonical fiber `theta`, repr_dim 1), stamp θ for the site → site+μ direction your `vertex_a → vertex_b` encodes. Determinism contract for `FLUX RANDOM SEED n`: θ_k = 2π · uniform_k from GIGI's house xorshift64* SmallRng (the same PRNG and seeding INIT HAAR uses), one draw per edge in lattice edge order k = 0..n_edges — same lattice + same seed is byte-identical, and edge 0 is pinned in the tests to 2π·uniform₀(seed).

**Symmetry-class receipts.** The triangle closed form passed exactly: C₃ with uniform flux φ per edge gives eigenvalues 2 − 2cos((3φ + 2πk)/3), verified to 1e-9 through the full parse → execute → assembly path (plus a tree-flux gauge-triviality anchor and a zero-flux cross-anchor against the cos-weight mode). The spacing-ratio gate, measured on fixed-seed Erdős–Rényi U(1) random-flux graphs (V = 512, mean degree 16, seeds 20260716/1/2/3, bulk = middle 80% of the sorted spectrum, published anchors from Atas–Bogomolny–Giraud–Roux PRL 110, 084101):

| mode | 4-seed mean r̃ | anchor | per-seed |
|---|---|---|---|
| default cos-weight (real symmetric) | **0.5272** | GOE 0.5307 | 0.5567, 0.5165, 0.5151, 0.5207 |
| MODE MAGNETIC (complex Hermitian) | **0.6046** | GUE 0.5996 | 0.5941, 0.5981, 0.6111, 0.6153 |

Both within ±0.03 of their anchors, and every magnetic seed sits above every cos-weight seed — time-reversal breaking is real, not noise. One honesty note on the estimator: the gate averages 4 fixed seeds because we measured the single-graph r̃ scatter at V = 512 to be σ ≈ 0.02, which makes a single-seed ±0.03 window a ~1.5σ criterion; the anchors and tolerance are untouched.

**Dense/Lanczos.** FULL runs on the dense SymmetricEigen path up to the spec §6 boundary V = 4096 — every graph in your RH loop is dense-side, so the sweep is fully unblocked. The sparse Lanczos arm (sprs + IRL + shift-invert per your spec §2–§5) is deferred to Phase 2.1 rather than rushed: FULL on V > 4096 returns a typed `SparseUnavailable` error naming Phase 2.1, and your bucky 6-sig-figure parity gate stays open until that arm lands. Named deviations from your 2026-06-30 spec, per your confirmed ask superseding it: the clause is `FULL [LIMIT k]` (not `MODE dense|sparse`; MODE now carries MAGNETIC, and solver choice is internal); ordering is ascending **algebraic** (your sparse spec said ascending by |λ| — identical on the PSD magnetic operator, different on the indefinite cos-weight one); `FULL` without LIMIT returns all eigenvalues (your spec's sparse default was k = 4). The result struct did gain your §6 `mode_used` + `convergence` fields (dense → `"dense"` + null).

**Error strings you will see.** Non-U(1) magnetic: `SPECTRAL_GAUGE: MODE MAGNETIC requires GROUP U(1) in this phase (matrix-valued magnetic Laplacians are a later phase); got GROUP SU(2)`. LIMIT bounds: `SPECTRAL_GAUGE: FULL LIMIT must be ≥ 1 (got 0) — omit LIMIT to return all eigenvalues` (LIMIT > V clamps silently). Over-threshold: `SPECTRAL_GAUGE: FULL on V = … exceeds the dense eigensolver threshold (V = 4096 …) — the sparse Lanczos arm ships in Phase 2.1 …`. Flux misuse: `gauge: INIT FLUX requires GROUP U(1) this phase (got …)`; `GAUGE_FIELD INIT FLUX RANDOM requires SEED <n>`; PERSIST with FLUX is rejected (the materialized theta bundle is already the durable artifact); re-initializing an existing bundle name errors with `already exists`.

Green light: the generate → INGEST (or INIT FLUX) → SPECTRAL_GAUGE MODE MAGNETIC FULL → rh_003–rh_011 loop is live on prod (gigi-stream). For the sweep itself I'd run the same binary locally — per-statement HTTP latency is the only difference, the math is identical. Ship report with the full gate table: `SPECTRAL_PHASE2_MAGNETIC_SHIPPED_2026-07-16.md`.
