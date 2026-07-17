# SPECTRAL MODE MATRIX — shipped 2026-07-17

Raw signed-symmetric spectrum verb for Bee's P-vs-NP signature (fraction of
NEGATIVE eigenvalues of the SAT Hessian). Built TDD in an isolated worktree off
`origin/main` `c0370e3`.

## What shipped

```
SPECTRAL <bundle> ON FIBER (<h_field>) MODE MATRIX [DIAGONAL <diag_field>] [FULL [LIMIT k]];
```

Assembles the **raw** signed symmetric matrix M from edge-endpoint records and
returns its spectrum via the dense real `SymmetricEigen<f64>` path — the ORIGINAL
pre-magnetic FULL solver, applied to the raw adjacency instead of a Laplacian.
This is deliberately **not** `SPECTRAL`'s normalized Laplacian (`L = I −
D^{−1/2}WD^{−1/2}`, PSD, loses the negatives) and **not** MODE MAGNETIC's
Hermitian Laplacian (degree diagonal + U(1) phases). The negatives are the whole
signal, so PNP needs the raw matrix.

One row out: `eigenvalues` (ascending), `n_records_used`, `mode_used` =
`"matrix"`, `n_negative`, `instability_fraction`.

## TDD receipts

- RED  `294482f` — `test(spectral-matrix): RED — M1..M7`. 8 integration tests,
  all failing on the pre-ship tree (the ON FIBER branch parsed `MODES k`, not
  `MODE MATRIX`: *"Expected positive integer, got 'MATRIX'"*).
- GREEN `81eef3f` — `impl(spectral-matrix): GREEN`. 8/8 integration + 4 lib unit
  tests pass.

## Math anchors (all green)

| # | pin | result |
|---|-----|--------|
| M1 | `[[2,−1],[−1,2]]` | eigenvalues {1, 3} to 1e-12, n_negative 0 |
| M2 | `[[0,1],[1,0]]`   | {−1, +1}, n_negative 1, instability 0.5 — negatives survive |
| M3 | tridiagonal diag=2 vs diag=0 | {2−√2, 2, 2+√2} shifts to {−√2, 0, √2} by exactly −2 |
| M4 | mirror `(i,j)` + `(j,i)` equal h | assembles to ±3, **not** ±6 (no double-count) |
| M5 | no-GROUP + MATRIX ≠ Laplacian | GROUP ignored; raw min −1 vs D−W Laplacian min 0 (PSD) |
| M6 | PNP plumbing | 2-SAT-like instability 0.25 < 3-SAT-like 0.75, ratio 3 |
| M7 | k>V clamps; k=0 / empty / V>4096 typed errors | all typed, V>4096 names Phase 2.1 |

Plus a `DIAGONAL <field>` override-column test.

## Design decisions

- **Diagonal schema: Option S (self-loop).** The bundle is edge-oriented; a
  self-loop record (`vertex_a == vertex_b`) carries `M[v][v]` in the `h` field.
  Optional `DIAGONAL <field>` names an override column. Absent → `M[v][v] = 0`.
- **NEG_TOL = 1e-9**, reused from the gauge gap-extraction so the whole spectral
  surface shares one zero-threshold. `n_negative = #{λ < −1e-9}`.
- **`n_negative` / `instability_fraction` over the FULL spectrum always** — never
  windowed by `LIMIT` (it's the signal); the returned `eigenvalues` still honor
  `LIMIT`.
- **Groupless.** MODE MATRIX takes scalar real weights; a stray `GROUP` token is
  swallowed and ignored, never required (the key contrast with MODE MAGNETIC).
- **Off-diagonals assigned, not accumulated** — a mirrored `(j,i)` record does
  not double-count.
- **`mode_used = "matrix"`** (names the mode; self-documenting).
- **Dense ceiling V ≤ 4096**, ungated const so the plain-SPECTRAL path compiles
  without `gauge`; V > 4096 → typed error naming the Phase 2.1 sparse arm.

## Files

- `src/spectral.rs` — `spectral_matrix_raw` + `SpectralMatrixResult` +
  `SPECTRAL_MATRIX_NEG_TOL` / `SPECTRAL_MATRIX_DENSE_MAX_V` (all ungated); 4 lib
  unit tests.
- `src/parser.rs` — MODE (singular) vs MODES (plural) disambiguation in
  `parse_spectral`; `skip_optional_group_ignored`; `Statement::Spectral` gains
  `fiber_field` / `matrix` / `diagonal` (additive); executor arm.
- `src/bin/gigi_stream.rs` — matching HTTP executor arm on the pre-resolved store.
- `tests/spectral_matrix_basic.rs` — M1..M7 + DIAGONAL override.

`src/gauge/*`, `src/curvature.rs`, `src/topology.rs`, `src/obstruction.rs`,
`src/imagine/*`, `compute_record_k`, WAL/.dhoom formats — untouched.

## Gates (green)

- `cargo check --features "kahler imagine sharded transactions patterns causal_states wish halcyon" --bin gigi-stream` — OK
- `cargo test --no-default-features --lib` — 920 passed
- `cargo test --features halcyon --test spectral_matrix_basic` — 8 passed
- RH fences (must stay green): `spectral_gauge_basic`, `spectral_gauge_where_basic`,
  `spectral_full_basic`, `spectral_magnetic_basic`, `u1_flux_basic`
- topology / ingest / imagine / part-iv / aurora / lattice / davis-conjecture /
  patterns / default-feature suites per the worktree gate subset

MODE MAGNETIC / FULL complex path in `spectral.rs` was read-for-reuse only and is
not regressed — the branch is a new sibling function, not an edit to the gauge
solver.

## Scope / honesty

Restages Bee's documented geometric-complexity signature; the evidence lives in
her framework. **Not** a P≠NP separation — the verb computes a spectrum.

## Ship handoff

Implementation, tests, letter, and report are committed on the worktree branch
`spectral-matrix-pnp` with all gates green. Merge onto main, deploy
(`flyctl deploy -a gigi-stream`, honoring any held deploy lease), the substrate
drill, and the live `Q1`–`Q6` probe are the coordinated ship step — the branch
is not yet on merged main, and production deploy is lease-coordinated across the
concurrent sessions on/around main.
