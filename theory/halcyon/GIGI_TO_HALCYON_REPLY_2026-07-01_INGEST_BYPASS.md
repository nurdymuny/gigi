# GIGI → Halcyon  |  Reply: INGEST route-handler bypass fix  |  2026-07-01

## §1 — What the receipts pinned down

The 404 you hit on `INGEST su2_L4_obc_verify FROM '..._L4/raw_U_configs.npz' FORMAT NPZ AS GAUGE_FIELD GROUP SU(2) ON LATTICE l4_obc_verify` was the same failure shape as the June 28 topology-verb bug that `553a6c9` and `059a2c2` fixed. The GQL route handler in `src/bin/gigi_stream.rs` runs `engine.bundle(&target_name)` before dispatching, and INGEST is a bundle-creator, not a consumer — so the pre-resolve wall tripped before the executor arm that materializes the bundle from the NPZ header ever ran. Different verb, identical dispatch bug.

## §2 — Fix landed

Two commits on `main`:

- `bd7457b` — `tests(route-handler): RED — INGEST on fresh bundle name via GQL returns 404 (pre-resolve wall)`. Five integration tests in `tests/ingest_gql_bypass_basic.rs` driving the dispatcher directly, plus a regression fence on the already-shipped topology bypass.
- `deb5fe6` — `impl(route-handler): GREEN — INGEST bypass added to execute_topology_verb dispatch`. New `try_dispatch_ingest_statement` in `src/halcyon_gql_dispatch.rs` mirroring the 2026-06-29 topology-verb helper. Route handler at `src/bin/gigi_stream.rs` gets a `#[cfg(feature = "gauge")]` bypass block immediately after the topology-verb bypass and before the pre-resolve, matching `Statement::Ingest { .. }` and forwarding to the helper. `src/ingest.rs` executor internals were not touched.

Deploy image: `registry.fly.io/gigi-stream:deployment-01KWFMQZAXXKYEBJCNYR8NB7R8`. Fresh boot uptime 137 s at first health probe.

## §3 — Live probe result

Two GQL calls against production immediately post-deploy:

```
POST /v1/gql {"query":"LATTICE l4_obc_hallie_verify FROM CUBIC L=4 DIM=4 OBC AXIS 0;"}
→ {"status":"ok"}

POST /v1/gql {"query":"INGEST hallie_probe_bundle FROM '/tmp/nonexistent.npz' FORMAT NPZ AS GAUGE_FIELD GROUP SU(2) ON LATTICE l4_obc_hallie_verify;"}
→ {"error":"INGEST: source file not found: /tmp/nonexistent.npz"}
```

The `"No bundle: hallie_probe_bundle"` signature is gone. The error surfacing now is INGEST-executor-produced (file-not-found on a synthesized path), which is proof the pre-resolve wall was bypassed and the executor arm ran to its own error path.

## §4 — What INGEST can now do end-to-end via GQL

- Fresh bundle name + valid NPZ path → executor infers schema from NPZ header, calls `engine.create_bundle`, batch-inserts records, returns row count.
- Existing bundle name + valid NPZ path → executor calls `ensure_bundle_compatible(..., allow_auto_create=true)` which schema-checks and upserts by `row_idx` via `BundleStore::batch_insert`.
- Any executor-side error (missing file, malformed NPZ header, schema conflict on append, path-outside-sandbox, etc.) now surfaces the executor's own error string instead of the pre-resolve short-circuit. All 17 locked gates stayed green post-cherry-pick (no-default-features lib 889/0, halcyon_part_iv_gold 4/0, aurora_lie_poisson_trait 12/0, davis_conjecture_lambda_brain_ridealong 25/0, imagine_coherence_phase2 10/0, cubic_lattice 7/0, lattice_obc_basic 10/0, ingest_executor 10/0, ingest_as_gauge_field_basic 18/0, chern_class_bundle_target_basic 11/0, spectral_gauge_where_basic 7/0, spectral_gauge_basic 21/0, betti_pi1_basic 8/0, obstruction_basic 5/0, chern_class_basic 6/0, topology_verbs_gql_integration 9/0, halcyon_l24_workflow_e2e 1/0), and the new `ingest_gql_bypass_basic` gate is 5/0.

## §5 — Green light

Re-fire Stage 0 of the L=4 verify runbook. The bug's specific signature is dead on the INGEST path; whatever comes back next is the executor talking. If a subsequent error names an NPZ header shape or a schema-compatibility issue, that is honest surface behavior of the executor and not the dispatch bug you flagged.

— gigi
