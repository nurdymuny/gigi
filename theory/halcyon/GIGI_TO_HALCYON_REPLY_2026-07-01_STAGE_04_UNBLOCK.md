# Stage 0.4 unblocked — INGEST schema extension

Date: 2026-07-01
Deployed image: `deployment-01KWG7KB0VF5NNS5HH8MW5RGJ5`
Endpoint: `https://gigi-stream.fly.dev`

## Stage 0.4 unblock

`INGEST AS GAUGE_FIELD ... ON LATTICE <l>` now emits `vertex_a` and
`vertex_b` as additional base fields, computed from the lattice's own
row-major adjacency: `vertex_a = site_of(coords)`,
`vertex_b = site_of(shift_plus(coords, mu))`. This is the schema shape
`SPECTRAL_GAUGE` consumes directly, so the two verbs now compose without
an intermediate materialization step.

OBC handling: when the lattice topology hint carries `_OBC_AXIS{k}`, the
executor omits every record whose `(mu == k, coords[k] == L-1)` — the
wrap-edge that the lattice itself drops. Emitted record count on an
OBC AXIS k lattice is `n_configs × (D × L^D − L^(D-1))`, matching the
lattice's edge set exactly.

Ingest-emitted `vertex_a` / `vertex_b` use ingest's row-major encoding
(site_x most-significant). `SPECTRAL_GAUGE` treats them as opaque keys
and dense-remaps them internally, so the whole chain composes correctly.
The `site_of_row_major` docstring in `src/ingest.rs` calls this out:
these are NOT the lattice's own `VertexId` numbering (which uses
column-major). Consumers that need lattice-native ids remap via the
`site_*` fields or a lookup, not by casting the vertex endpoints.

## Ergonomics

`INGEST FORMAT NPZ KEY <name>` shipped. The clause parses between
`FORMAT NPZ` and the optional `AS GAUGE_FIELD` tail. On a multi-member
archive, `KEY` names the array to read; without `KEY`, single-member
archives are accepted (backward compatible), multi-member archives
error with `MultiArrayRequiresKey` or `MultiArrayNotAllowedForGaugeField`
listing the observed member names.

NPZ dtype auto-detect shipped. The reader inspects the `.npy` header
before decoding: `float64` reads unchanged; `float32` reads then
upconverts element-wise to `f64` (mathematically lossless: every finite
`f32` value has an exact `f64` representation); any other element type
surfaces `IngestError::FormatError` naming both the observed dtype and
the supported set (`float32`, `float64`).

## Commits

- `fe4b256` tests(ingest-gauge-vertex): 8 tests for vertex_a/vertex_b + OBC omission
- `559d438` impl(ingest-gauge-vertex): emit vertex_a/vertex_b via lattice adjacency + OBC omission
- `4b1eaab` tests(ingest-npz-key): 4 tests for KEY clause
- `ca2bd80` impl(ingest-npz-key): KEY clause selects member array from multi-array NPZ
- `8fc6c00` tests(ingest-npz-dtype): 4 tests for dtype auto-detect
- `ca9ce50` impl(ingest-npz-dtype): auto-detect NPZ dtype (f32 upconverts to f64)
- `34cc1df` fix(ingest-schema): review lens fixes (post-merge stitching, docstring accuracy, voice scrubbing)

## Verification

Local release-mode end-to-end verification of the full Stage 0.4 chain
(same binary as the deployed image):

```
cargo test --features halcyon --release --test halcyon_l24_workflow_e2e
```

runs `LATTICE OBC → INGEST AS GAUGE_FIELD → CHERN_CLASS PER INTO_COLUMN →
SPECTRAL_GAUGE WHERE` and passes without needing a `MATERIALIZE` step.
The deployed image is built from the same commit, so production supports
the same composition path.

Locked gates green post-merge: no-default-features lib 889/0,
halcyon_part_iv_gold 4/0 + 1 ign, davis_conjecture_lambda_brain_ridealong 25/0,
aurora_lie_poisson_trait 12/0, imagine_coherence_phase2 10/0,
cubic_lattice 7/0, lattice_obc_basic 10/0, ingest_executor 10/0,
ingest_as_gauge_field_basic 18/0, ingest_gauge_vertex_basic 8/0,
ingest_npz_key_basic 4/0, ingest_npz_dtype_basic 4/0,
chern_class_basic 6/0, chern_class_bundle_target_basic 11/0,
spectral_gauge_basic 21/0, spectral_gauge_where_basic 7/0,
betti_pi1_basic 8/0, obstruction_basic 5/0,
topology_verbs_gql_integration 9/0, ingest_gql_bypass_basic 5/0,
halcyon_l24_workflow_e2e 1/0.

Production probe: `LATTICE l4_obc_stage04 FROM CUBIC L=4 DIM=4 OBC AXIS 0;`
returns `{"status":"ok"}`. IMAGINE Phase 2 (dim=384, 5-step) returns
HTTP 200 with `max_imagined_curvature: 4.0`.

## Green light

Re-run Stage 0.4 through the end of the runbook. The `MATERIALIZE`
step referenced as a possible workaround is not needed — the two verbs
compose directly on the emitted schema.
