# Vertex encoding aligned with Lattice::VertexId

Date: 2026-07-01
Deployed image: `deployment-01KWGGDC9QSKWSCENF3S4EH4N6`
Endpoint: `https://gigi-stream.fly.dev`

## What changed

`INGEST AS GAUGE_FIELD ... ON LATTICE <l>` now stamps `vertex_a` and
`vertex_b` using the same column-major site encoding the lattice itself
uses. The emitted integer values equal `Lattice::VertexId` for the same
coords and L. Concretely: for L=4, D=2, site (site_x=2, site_y=1), the
ingest emitter previously wrote `vertex_a = 2*4 + 1 = 9` (row-major,
site_x most-significant); it now writes `vertex_a = 2*1 + 1*4 = 6`
(column-major, site_x least-significant), matching what
`lattice.site_of([2,1])` returns.

Implementation: renamed `site_of_row_major` in `src/ingest.rs` to
`site_of_column_major` and re-pointed it at a new public helper
`site_of_column_major` in `src/lattice/topology/mod.rs`. The public
helper is a visibility-only lift of the encoding that was already
private inside `cubic::cubic` — no math change, one source of truth
now.

## Why it matters

Any future verb that joins the ingested bundle's `vertex_a` with
`lattice.resolve_edge(a, b)` or `lattice.vertex(id)` on a D>=2 lattice
would have silently looked up the wrong site under the old encoding.
On D=2 L=4, half the vertices land at different integers between the
two schemes; on D=4 L=24, the divergence covers every non-corner site.
Aligning the encodings forecloses that entire class of latent bug
before any consumer takes a dependency on it.

## What is preserved

- Record count per config: unchanged (still `D * L^D - L^(D-1)` for
  OBC axis k; `D * L^D` for periodic).
- Record emission order through the ingest loop: unchanged. The
  outer walk is still row-major over `site_flat` because that is the
  NPZ file's own layout; only the integer values stamped in
  `vertex_a` / `vertex_b` change.
- `SPECTRAL_GAUGE` and `CHERN_CLASS PER INTO_COLUMN` are unaffected.
  Both treat the endpoints as opaque `HashMap` keys and dense-remap
  internally — the ingest → spectral → chern chain composes exactly
  as before, only with lattice-native integers now flowing through the
  intermediate column.

## Commits

- `afb545f` tests(ingest-vertex-encoding): RED — vertex_a/b assertions expect lattice's column-major values
- `e45f9db` impl(ingest-vertex-encoding): GREEN — align vertex_a/b encoding with Lattice::VertexId numbering
- `9aaa3ac` fix(ingest-vertex-encoding): review lens fixes (stale row-major docstrings in two tests)

## Verification

All 21 locked gates green post-merge:
`--no-default-features --lib 889/0`, `halcyon_part_iv_gold 4/0 + 1 ign`,
`davis_conjecture_lambda_brain_ridealong 25/0`,
`aurora_lie_poisson_trait 12/0`, `imagine_coherence_phase2 10/0`,
`cubic_lattice 7/0`, `lattice_obc_basic 10/0`, `ingest_executor 11/0`,
`ingest_as_gauge_field_basic 18/0`, `ingest_gauge_vertex_basic 8/0`,
`ingest_npz_key_basic 4/0`, `ingest_npz_dtype_basic 4/0`,
`chern_class_basic 6/0`, `chern_class_bundle_target_basic 11/0`,
`spectral_gauge_basic 21/0`, `spectral_gauge_where_basic 7/0`,
`betti_pi1_basic 8/0`, `obstruction_basic 5/0`,
`topology_verbs_gql_integration 9/0`, `ingest_gql_bypass_basic 5/0`,
`halcyon_l24_workflow_e2e 1/0`.

Local release-mode end-to-end:

```
cargo test --features halcyon --release --test halcyon_l24_workflow_e2e
```

passes with the new encoding.

Production probe: `LATTICE l4_bee_vertex_encoding FROM CUBIC L=4 DIM=2 OBC AXIS 0;`
returns `{"status":"ok"}`. IMAGINE Phase 2 (dim=384, 5-step) returns
HTTP 200, `refused=false`, 6 trajectory points.
