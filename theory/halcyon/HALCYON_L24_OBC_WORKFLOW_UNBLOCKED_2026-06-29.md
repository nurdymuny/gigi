# Halcyon L=24 OBC Sectoral SPECTRAL_GAUGE Workflow — Unblocked 2026-06-29

## What landed

Nine commits on `origin/main`, all live on `gigi-stream.fly.dev`. The
4-concept extension chain unblocks Hallie's SU(2) 4D L=24 β=2.3 OBC
sectoral SPECTRAL_GAUGE workflow — every ask in her 2026-06-28-evening
letter has a shipped grammar answer.

Deployment: image `deployment-01KWFEDVNWJQBM1TBDMN6167XJ` (build 1),
followed by the ALTER-wiring fix in image built from HEAD `050b9f6`,
machine `683961dbe9ee38`.

| commit  | title                                                                  |
|---------|------------------------------------------------------------------------|
| 6a881f2 | tests(lattice-obc): RED — 9 failing tests for LATTICE ... OBC AXIS <k> |
| b911a1f | impl(lattice-obc): GREEN — LATTICE ... FROM CUBIC ... OBC AXIS <k>     |
| c8ccea4 | tests(ingest-as-gauge-field): RED — 18 failing tests                   |
| 4b18309 | impl(ingest-as-gauge-field): GREEN — INGEST AS GAUGE_FIELD             |
| f4bb832 | tests(chern-class-bundle): RED — 11 failing tests                      |
| d88f6b4 | impl(chern-class-bundle): GREEN — bundle target + PER + INTO_COLUMN    |
| e42ae08 | tests(spectral-gauge-where): RED — 7 failing tests                     |
| ac0be4d | impl(spectral-gauge-where): GREEN — WHERE clause                       |
| 2c0d68b | fix(halcyon-l24): MATH/ENGINEERING/VOICE lens fixes                    |
| b1e2ce7 | fix(route-handler): wire ALTER BUNDLE ADD BASE through gigi_stream     |
| 050b9f6 | fix(route-handler): delegate ALTER BUNDLE ADD BASE to parser executor  |

## Why

Hallie's 2026-06-28-evening letter enumerated five blockers on her
production SPECTRAL_GAUGE workflow at L=24, β=2.3, OBC. Each maps to a
concrete grammar extension:

- **Ask 1**: OBC on cubic lattices → Concept 1 (`OBC AXIS <k>`)
- **Ask 2**: CHERN_CLASS on ingested bundle targets → Concept 3, Path B
- **Ask 3**: Per-config chern → Concept 3, `PER <field>` + `INTO_COLUMN`
- **Ask 4**: Sector-filtered SPECTRAL_GAUGE → Concept 4 (`WHERE`)
- **Ask 5**: NPZ → gauge-field bundle in one step → Concept 2

The five asks compose into a single verb chain that runs Q-sector-by-
Q-sector Q=0..Q_max SPECTRAL_GAUGE sweeps over her Modal-computed
CHERN_CLASS integer projections.

## Concept 1 — LATTICE ... FROM CUBIC ... OBC AXIS `<k>`

Grammar (concise form only for Phase 1):

```gql
LATTICE l24 FROM CUBIC L=24 DIM=4 OBC AXIS 0;
LATTICE l24 FROM CUBIC L=24 DIM=4 PERIODIC;   -- unchanged, backwards compat
```

Semantics: with `OBC AXIS <k>`, plaquettes crossing the boundary at axis
`k` (index 0 and L−1) are omitted from `faces`. Edges wrapping the
boundary in axis `k` are omitted from `edges`. Vertex count stays L^D
(open boundary keeps sites; just removes wrap-around connectivity).

For L=24 D=4 OBC AXIS 0:

| quantity | count      | vs periodic                |
|----------|------------|----------------------------|
| V        | 331,776    | unchanged                  |
| E        | 1,313,280  | 1,327,104 − 13,824         |
| F        | 1,949,184  | 1,990,656 − 41,472         |

Explicit multi-axis form (`PERIODIC AXES (1,2,3) OBC AXIS 0`) is
grammar-reserved for Phase 2 — Phase 1 only needs the single-OBC-axis
case.

**Files**: `src/lattice/topology/cubic.rs`, `src/parser.rs`

## Concept 2 — INGEST ... AS GAUGE_FIELD GROUP `<g>` ON LATTICE `<l>`

Grammar (optional interpretation clause on INGEST):

```gql
INGEST su2_L24_seed702 FROM '/path/to/raw_U_configs.npz'
  FORMAT NPZ
  AS GAUGE_FIELD GROUP SU(2) ON LATTICE l24;
```

Without the interpretation clause, INGEST stays generic-array
(backwards compat).

Innermost axis width must match the group's `repr_dim`:

- SU(2) → 4  (q0, q1, q2, q3)
- SU(3) → 18 (re_00, im_00, ..., re_22, im_22)
- U(1)  → 1  (theta)

Axis interpretation (outermost to innermost):

- axis 0: `config_id` (INT record field)
- axis 1: `mu` (INT direction 0..D−1 where D = `lattice.dim`)
- axes 2..2+D−1: `site_x`, `site_y`, `site_z`, `site_t` (each 0..L−1)
- innermost: fiber (canonical names per group)

Total records = L^D × D × n_configs where `n_configs = shape[0]`.

Error cases surface with clear diagnostics:

- Fiber width mismatch → names expected vs got
- Axis count mismatch (shape ndim wrong for `lattice.dim`) → clear error
- Lattice not found → clear error

**Files**: `src/ingest.rs`, `src/parser.rs`

## Concept 3 — CHERN_CLASS bundle target + PER + INTO_COLUMN

Grammar (full):

```gql
CHERN_CLASS <name> ORDER <k>
  [ON LATTICE <lattice>]
  [ON FIBER (f1, f2, ...)]
  [GROUP <g>]
  [PER <field>]
  [INTO_COLUMN <col>];
```

Two-path resolver on `<name>`:

- **Path A — gauge field target** (backwards compat): resolves through
  `gauge::registry`. The handle carries its lattice binding, so
  `ON LATTICE` is a conflict error. `PER` / `INTO_COLUMN` are also
  errors on gauge-field targets.
- **Path B — bundle target** (new): resolves through `engine.bundle()`.
  `ON LATTICE <name>` is required (bundle records supply fiber;
  lattice supplies cell complex). Returns Rows — one row per PER
  group, or one row total when PER is omitted.

Ambiguity guard: if `<name>` resolves as BOTH a gauge field AND a
bundle, the error surfaces both matches so the caller renames one.

`PER <field>` groups bundle records by `<field>`, returns
`[{<field>, chern_class_<k>, q_rounded}, ...]` — one row per group in
ascending-key order.

`INTO_COLUMN <col>` writes `q_rounded` back to the source bundle as a
new BASE field. Requires the column to already exist (`ALTER BUNDLE ...
ADD BASE <col> INT` first). Idempotent under fixed input.

**Files**: `src/chern_weil.rs`, `src/halcyon_gql_dispatch.rs`,
`src/parser.rs`

## Concept 4 — SPECTRAL_GAUGE ... WHERE `<predicate>`

Grammar:

```gql
SPECTRAL_GAUGE <bundle>
  [WHERE <predicate>]
  ON FIBER (f1, f2, ...)
  [GROUP <g>]
  [FULL [LIMIT k]];
```

Predicate scope (Phase 1):

- Equality: `<field> = <literal>` (Integer, Float, or Text)
- Comparison: `<field> [<= >= < > !=] <literal>`
- AND / OR combinators (2-3 predicates max; no parens needed for Phase 1)

Semantics: records are pre-filtered by the predicate, the adjacency
graph is built from the filtered subset only (edges retained iff every
condition matches the record), then the Laplacian is constructed and
eigendecomposed on the reduced graph. `n_records_used` reports the
filtered count so callers can observe the filter's effect.

**Files**: `src/spectral.rs`, `src/parser.rs`,
`src/bin/gigi_stream.rs`

## The unblocked target workflow

Hallie's exact target chain (from her 2026-06-28-evening letter),
ready to fire against L=24 real data:

```gql
INGEST su2_L24_seed702 FROM '/results/.../raw_U_configs.npz'
  FORMAT NPZ AS GAUGE_FIELD GROUP SU(2) ON LATTICE l24;

LATTICE l24 FROM CUBIC L=24 DIM=4 OBC AXIS 0;

ALTER BUNDLE su2_L24_seed702 ADD BASE q_rounded INT;

CHERN_CLASS su2_L24_seed702 ORDER 2 ON LATTICE l24 GROUP SU(2)
  PER config_id INTO_COLUMN q_rounded;

SPECTRAL_GAUGE su2_L24_seed702 WHERE q_rounded = 0
  ON FIBER (q0, q1, q2, q3) GROUP SU(2);

SPECTRAL_GAUGE su2_L24_seed702 WHERE q_rounded = 1
  ON FIBER (q0, q1, q2, q3) GROUP SU(2);

-- etc per sector
```

## Live verification receipts (2026-06-29 post-deploy)

Reproduced on a synthesized L=4 D=2 SU(2) OBC AXIS 0 fixture (small
enough for inline probe, large enough to exercise all 4 concepts):

**Concept 1** — LATTICE OBC:

```json
POST /v1/gql {"query": "LATTICE l4_obc FROM CUBIC L=4 DIM=2 OBC AXIS 0;"}
→ {"status": "ok"}
```

**Concept 3** — CHERN_CLASS bundle target + PER + INTO_COLUMN, on
64 identity SU(2) records across 2 configs:

```json
POST /v1/gql {
  "query": "CHERN_CLASS test_su2_l4_obc ORDER 2 ON LATTICE l4_obc
            GROUP SU(2) PER config_id INTO_COLUMN q_rounded;"
}
→ {"rows": [
    {"config_id": 0, "chern_class_2": 0.0, "q_rounded": 0},
    {"config_id": 1, "chern_class_2": 0.0, "q_rounded": 0}
  ], "count": 2}
```

**Concept 4** — SPECTRAL_GAUGE WHERE on a proper 6-ring + 4-ring
graph-schema fixture (sector 0 = ring6, sector 1 = ring4):

```json
POST /v1/gql {
  "query": "SPECTRAL_GAUGE test_su2_sectors WHERE q_rounded = 0
            ON FIBER (q0, q1, q2, q3) GROUP SU(2);"
}
→ {"rows": [{"gap": 0.999999999999998, "n_records_used": 6, "group_used": "SU(2)"}]}

POST /v1/gql {
  "query": "SPECTRAL_GAUGE test_su2_sectors WHERE q_rounded = 1
            ON FIBER (q0, q1, q2, q3) GROUP SU(2);"
}
→ {"rows": [{"gap": 2.0, "n_records_used": 4, "group_used": "SU(2)"}]}
```

Both gaps match the exact algebraic connectivity of the ring on their
sector's surviving edges: `2·(1 − cos(2π/6)) = 1.0` for the 6-ring,
`2·(1 − cos(2π/4)) = 2.0` for the 4-ring. The WHERE clause filtered
the records to the correct sector, built the graph from the filtered
subset, and returned that sector's algebraic connectivity.

## Route-handler wiring notes

The initial 4-concept ship shipped the parser + kernel + `try_dispatch_*`
dispatchers, but not the `gigi_stream` route-handler wiring for
`ALTER BUNDLE ADD BASE`. Phase E post-deploy verification caught this:
ALTER returned 200 ok but SHOW BUNDLES confirmed the schema stayed at
8 fields (not 9). Two-line fix at `b1e2ce7` (get_bundle_name +
needs_write) + one-line executor delegation at `050b9f6` completed the
wiring. Same class of bug as `553a6c9` (topology-verb bypass), same
fix shape — the pattern is worth remembering.

## Locked gates (all green post-ship)

- `cargo test --no-default-features --lib`: 889 / 0
- `cargo test --features halcyon --test halcyon_part_iv_gold`: 4 / 0 + 1 ign
- `cargo test --features halcyon --release --test halcyon_part_vi_bit_identity_gold -- --include-ignored`: 3 / 0
- `cargo test --features kahler --test davis_conjecture_lambda_brain_ridealong`: 25 / 0
- `cargo test --features halcyon --test aurora_lie_poisson_trait`: 12 / 0
- `cargo test --features kahler,imagine --test imagine_coherence_phase2`: 10 / 0
- `cargo test --features lattice --test cubic_lattice`: 7 / 0
- `cargo test --features lattice --test lattice_obc_basic`: 10 / 0
- `cargo test --test ingest_executor`: 10 / 0
- `cargo test --features halcyon --test ingest_as_gauge_field_basic`: 18 / 0
- `cargo test --features halcyon --test gauge_su3_basic`: 4 / 0 + `gauge_su3_persistence`: 4 / 0
- `cargo test --features halcyon --test spectral_gauge_basic`: 21 / 0
- `cargo test --features halcyon --test spectral_gauge_where_basic`: 7 / 0
- `cargo test --features halcyon --test betti_pi1_basic`: 8 / 0
- `cargo test --features halcyon --test obstruction_basic`: 5 / 0
- `cargo test --features halcyon --test chern_class_basic`: 6 / 0
- `cargo test --features halcyon --test chern_class_bundle_target_basic`: 11 / 0
- `cargo test --features halcyon --test topology_verbs_gql_integration`: 9 / 0
- `cargo test --features halcyon --test halcyon_l24_workflow_e2e`: 1 / 0

The E2E test at `tests/halcyon_l24_workflow_e2e.rs` is the CI-locked
witness of the Phase 6 acceptance criterion: it chains all 4 concepts
end-to-end via the in-process executor + a synthesized identity NPZ
fixture. If any of the four concepts regresses, the test fails at HEAD.

## Deferred to Phase 2

- Explicit multi-axis OBC form (`PERIODIC AXES (...) OBC AXIS <k>`)
- Sparse Lanczos SPECTRAL_GAUGE for L≥16 (spec at `SPECTRAL_GAUGE_PHASE2_SPEC.md`)
- OR predicates in SPECTRAL_GAUGE WHERE
- Parenthesized boolean expressions in WHERE
