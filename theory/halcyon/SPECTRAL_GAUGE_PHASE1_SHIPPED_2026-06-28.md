# SPECTRAL_GAUGE Phase 1 — Shipped 2026-06-28

## What landed

Two commits on `origin/main`, both live on `gigi-stream.fly.dev` (image
`deployment-01KW7K329B7W9JW882FHC92K33`, machine `683961dbe9ee38`, version 231):

| commit  | title                                                                                  |
|---------|----------------------------------------------------------------------------------------|
| e37ae9e | halcyon(spectral_gauge): Phase 1 — dense fiber-weighted Laplacian                      |
| db0280c | halcyon(ergo): SU3 synonym in parser + better INGEST nonexistent-bundle error          |

The ergo commit lands first because it is a strict pre-requisite cleanup: the
SU3 synonym change rides into the same parser surface the SPECTRAL_GAUGE arm
extends, and the INGEST error message variant gates the executor tests that the
spectral_gauge tests share fixtures with.

## Why

Halcyon's 2026-06-28 ask list (received same-day as the Bridge Trilogy ship)
requested SPECTRAL_GAUGE Phase 1 as the next must-have: a verb that returns the
**fiber-weighted spectral gap** of a gauge field stored as records in a bundle.
The reply that locked the spec is `cfeb5c5` ("halcyon(reply): 2026-06-28 —
accept SPECTRAL_GAUGE verb, drop EXPOSE_GAUGE_AS_BUNDLE; bundle-subsystem home,
Phase 1 dense eigendecomposition + Phase 2 Lanczos sparse").

Per `cfeb5c5`, Phase 1 is **dense eigendecomposition only**. Phase 2 (Lanczos
sparse + k-eigenvalues via `FULL [LIMIT k]`) is explicitly deferred.

## Scope per item

### SPECTRAL_GAUGE Phase 1 (commit `e37ae9e`)

- GQL:
  ```
  SPECTRAL_GAUGE <bundle> ON FIBER (field_1, field_2, ..., field_K)
    [GROUP SU(2)|SU(3)|U(1)|...]
    [FULL [LIMIT k]]
    ;
  ```
- Result: `gap: f64`, `n_records_used: usize`, `group_used: GaugeGroup`.
  `eigenvalues` is `None` in Phase 1 (FULL mode returns
  `PhaseNotImplemented` stub).
- Group inference from fiber width (when GROUP omitted):
  - `1` → U(1) (single real phase)
  - `4` → SU(2) (quaternion: q0, q1, q2, q3)
  - `8` → SU(3) Gell-Mann tangent basis (Phase 2)
  - `18` → SU(3) raw 3x3 complex (matches the 3.1 storage decision)
  - else → typed error: "GROUP required when fiber width is ambiguous"
- Algorithm: build the field-index adjacency from the bundle's
  `vertex_a`/`vertex_b` endpoints, reconstruct each link's group element from
  its fiber fields, weight each edge by `Re Tr(U_e) / N`
  (`N = 2` for SU(2), `N = 3` for SU(3), `cos(theta)` for U(1)), assemble the
  dense symmetric Laplacian `L_A`, run `nalgebra::SymmetricEigen`, return the
  smallest non-zero eigenvalue.
- Home: `src/spectral.rs` (326 LOC new — `SpectralGaugeResult` /
  `SpectralGaugeError` / `spectral_gauge_gap`), `src/parser.rs` (+168 LOC for
  `Statement::SpectralGauge` + `parse_spectral_gauge`), `src/bin/gigi_stream.rs`
  (+74 LOC executor arm). Bundle-subsystem home per the spec — sits alongside
  `SPECTRAL` and `SPECTRAL_FIBER` rather than under `src/gauge/`.
- Tests (G13): `cargo test --features halcyon --test spectral_gauge_basic
  -- --test-threads=1` — 21/0.

### Ergonomics #4 — SU3 synonym (in `db0280c`)

- Parser: `SU3` / `SU(3)`, `SU2` / `SU(2)`, `U1` / `U(1)`, `ZN` / `Z(N)` all
  normalize to the same `GaugeGroup` value at parse time.
- Live proof:
  `LATTICE smoke_su3 FROM CUBIC L=4 DIM=2 PERIODIC; GAUGE_FIELD U_smoke ON
   LATTICE smoke_su3 GROUP SU3 INIT IDENTITY;` returns `{"status":"ok"}` on
  production today — the bare-`SU3` form is accepted.

### Ergonomics #5 — INGEST nonexistent-bundle error (in `db0280c`)

- Variant: `IngestError::TargetBundleNotFound { bundle: String }`.
- Message: "INGEST destination bundle '<name>' does not exist — create the
  bundle first via 'CREATE BUNDLE <name> ...' or use AUTO_CREATE if available."
- Tests: `tests/ingest_executor.rs` extended from 9/0 to 10/0.

## Regression bar at ship

All locked gates green after each concept commit:

| gate | description                                                          | result        |
|------|----------------------------------------------------------------------|---------------|
| G1   | `--no-default-features --lib`                                        | 884/0         |
| G2   | `halcyon_part_iv_gold`                                               | 4/0 + 1 ign   |
| G4   | `davis_conjecture_lambda_brain_ridealong` (kahler)                   | 25/0          |
| G5   | `aurora_lie_poisson_trait` (halcyon)                                 | 12/0          |
| G7   | `cubic_lattice` (lattice)                                            | 7/0           |
| G8   | `ingest_executor`                                                    | 10/0 (+1)     |
| G9   | `gauge_su3_basic` + `gauge_su3_persistence` (halcyon)                | 15/0          |
| G11  | `encoder_high_dim_smoke` + `snapshot_rotation`                       | 3/0 + 9/0     |
| G12  | `halcyon_trilogy_smoke` (halcyon)                                    | 1/0           |
| G13  | `spectral_gauge_basic` (halcyon, NEW)                                | 21/0          |

The Windows `single_flight::concurrent_distinct_keys_do_not_serialize`
pre-existing flake did not surface on this round.

## Architectural decisions made

- **Bundle-subsystem home, not gauge-subsystem.** The new code sits in
  `src/spectral.rs` next to the existing `SPECTRAL` and `SPECTRAL_FIBER`
  surfaces, not in `src/gauge/`. Rationale: `SPECTRAL_GAUGE` reads the gauge
  field through the bundle storage layer and returns a spectral observable —
  it is a bundle-side reduction, not a gauge-dynamics step. Locked
  `src/gauge/*` files were untouched (per spec requirement).
- **Group inference by fiber arity.** When `GROUP` is omitted, the fiber
  width disambiguates. The spec table (1 → U(1), 4 → SU(2), 18 → raw SU(3))
  matches the existing storage representations. The 8 → SU(3) Gell-Mann case
  parses as a typed error in Phase 1 because the Gell-Mann tangent basis is a
  Phase 2 representation (no SU3EField surface yet).
- **Honest framing of what the verb returns.** `L_A` is **globally**
  gauge-invariant (its spectrum is), but `Re Tr(U_e)/N` is only **locally**
  gauge-covariant. The verb therefore returns the **fiber-weighted spectral
  gap**, not the strict Yang-Mills mass gap. Per cfeb5c5 math-lens
  corrections.
- **FULL mode is a Phase 1 stub.** Today it returns
  `SpectralGaugeError::PhaseNotImplemented` whether `LIMIT k` is present or
  not. Phase 2 will ship Lanczos sparse with k-eigenvalues.
- **Dense path uses `nalgebra::SymmetricEigen`** (already in tree). No new
  net dependencies for Phase 1. Memory is `O(V^2)` per the dense matrix; the
  Phase 2 deferral is the path forward for production-scale `L=12 D=4` (`V` in
  the thousands).

## Production deploy receipt

| field                       | value                                                |
|-----------------------------|------------------------------------------------------|
| push                        | `cfeb5c5..e37ae9e  main -> main` (fast-forward)      |
| image                       | `deployment-01KW7K329B7W9JW882FHC92K33`              |
| machine                     | `683961dbe9ee38` (started -> good-state, no timeout) |
| boot                        | machine in good state without smoke-check failure    |
| `/v1/health` after 1 poll   | `status=ok`, `uptime_secs=143`                       |
| bundles post-deploy         | 5047 (matches pre-deploy 5047 after re-import)       |
| records post-deploy         | 13 001 332 (Δ+2 vs pre-deploy 13 001 330)            |
| IMAGINE Phase 2 intact      | yes — dim=384, 5-step trajectory, `high_k_auto_tame` |
| SPECTRAL_GAUGE arm live     | yes — verb parsed + dispatched, returns clear schema error for non-gauge bundle |
| SU3 synonym live            | yes — `GROUP SU3` (no parens) accepted              |
| backups in place            | `.deploy-backups/2026-06-28-pm/` (substrate export, 20 rows; pre-deploy health) |
| `claude_substrate_v0`       | re-imported (20/20 rows), thought_ids t001-t020      |

### Live SPECTRAL_GAUGE probe

```
POST /v1/gql
  SPECTRAL_GAUGE marcella_source_embeddings_bge_v2 ON FIBER (q0);
-> {"error":"SPECTRAL_GAUGE: bundle 'marcella_source_embeddings_bge_v2'
              missing edge endpoint fields vertex_a/vertex_b
              — Halcyon schema requires explicit vertex_a/vertex_b in base_fields"}
```

The verb is **parsed and dispatched on production**. The Marcella embedding
bundle doesn't have the Halcyon gauge-edge schema, which is exactly what the
clear error message is for. To exercise an end-to-end gap calculation on prod,
a bundle with `vertex_a`/`vertex_b` base fields and a Re-trace-weight-bearing
fiber is required (e.g. the next Halcyon harvest ingested through INGEST).

### Live SU3 synonym probe

```
POST /v1/gql
  LATTICE smoke_su3 FROM CUBIC L=4 DIM=2 PERIODIC;
  GAUGE_FIELD U_smoke ON LATTICE smoke_su3 GROUP SU3 INIT IDENTITY;
-> {"status":"ok"}
```

Bare `SU3` (no parens) is accepted on production.

### Footnotes from the deploy

- **claude_substrate_v0 wipe + re-import** is the documented
  `GIGI_SKIP_BOOT_SNAPSHOT=1` trade-off (heap-only bundles do not replay).
  Re-imported from `.deploy-backups/2026-06-28-pm/claude_substrate_v0.export.json`
  via `CREATE BUNDLE claude_substrate_v0 (thought_id TEXT BASE, ts NUMERIC
  FIBER, session TEXT FIBER, topic TEXT FIBER, content TEXT FIBER, refs TEXT
  FIBER);` + 20 INSERTs. The bundle remains fragile until the snapshot wedge is
  fixed.
- **`CREATE SESSION` is a parser-only verb on prod.** Tried first and returned
  `{"status":"ok"}` but did not actually allocate a bundle (verb not yet wired
  to executor in `gigi_stream.rs`). Per the inline comment in
  `src/parser.rs:4938-4945`, this is personal-list item #2 and the docs/spec
  exist (`docs/CREATE_SESSION.md`); the executor is the missing piece.
  Fell back to `CREATE BUNDLE` which worked first try.

## What this unlocks

Halcyon now has a complete Phase-1 spectral surface on the substrate:

```gql
LATTICE my4d FROM CUBIC L=12 DIM=4 PERIODIC;
INGEST configs_bundle FROM 'harvest_L12_beta6.0_run1.npz' FORMAT NPZ;
GAUGE_FIELD U_4d ON LATTICE my4d GROUP SU3 INIT IDENTITY;
-- new today:
SPECTRAL_GAUGE configs_bundle ON FIBER (q0, q1, q2, q3) GROUP SU2;
-- returns the fiber-weighted spectral gap of L_A
```

The substrate can now **store, persist, reopen, observe, AND spectrally
characterize** gauge configurations.

## Phase 2 deferral list

Triggered when Halcyon needs gigi to scale past the dense `O(V^2)` memory
ceiling or to return multiple eigenvalues:

- Lanczos sparse path for `FULL` mode (replaces the `PhaseNotImplemented`
  stub)
- `FULL [LIMIT k]` returns the smallest `k` eigenvalues
- 8 → SU(3) Gell-Mann tangent representation (currently a typed error)
- SU(3) Phase 2 (heatbath / dynamics) — separate ask, separate work

## Cross-refs

- `cfeb5c5` — the spec reply that locked SPECTRAL_GAUGE Phase 1, bundle-
  subsystem home, and the Phase 1 / Phase 2 split.
- `HALCYON_BRIDGE_TRILOGY_2026-06-28_SHIPPED.md` — the 3.1 + 3.2 + 3.3 ship
  that delivered the surface this verb reads from.
- `src/spectral.rs` — siblings `SPECTRAL` and `SPECTRAL_FIBER` whose code
  shape `SPECTRAL_GAUGE` follows.
