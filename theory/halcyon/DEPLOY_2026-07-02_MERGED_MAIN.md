# DEPLOY_2026-07-02 — merged main to gigi-stream

## SHA + image

- HEAD: `2bffb33` — INGEST: clear feature error for AS GAUGE_FIELD on non-gauge builds
- Merge commit: `11c851d` — Merge tetmesh chapter, companion site, engine fixes, and instrument verbs
- Deploy image: `deployment-01KWHQ8CP8KEYBMMGYVMCA3E5F`
- Fly machine `683961dbe9ee38` running the deployed image tag, 1/1 checks passing, DNS verified.

## Pre-deploy gates (8 / 8 green)

| suite | passed |
|---|---|
| `cargo test --no-default-features --lib` | 898 / 0 |
| `halcyon_l24_workflow_e2e` (release) | 1 / 0 |
| INGEST family (`as_gauge_field_basic`, `gauge_vertex_basic`, `npz_key_basic`, `npz_dtype_basic`, `gql_bypass_basic`) | 12 / 0 |
| spectral + topology verbs (`spectral_gauge_basic`, `spectral_gauge_where_basic`, `chern_class_basic`, `chern_class_bundle_target_basic`, `betti_pi1_basic`, `obstruction_basic`, `topology_verbs_gql_integration`) | 9 / 0 |
| `imagine_coherence_phase2` (kahler+imagine) | 10 / 0 |
| `halcyon_part_iv_gold` + `aurora_lie_poisson_trait` | 4 / 0 |
| `cubic_lattice` + `lattice_obc_basic` | 10 / 0 |
| `davis_conjecture_lambda_brain_ridealong` | 25 / 0 |

## Live probes on the deployed image

### A. LATTICE OBC AXIS

```
POST /v1/gql {"query":"LATTICE l4_bee_deploy_probe FROM CUBIC L=4 DIM=2 OBC AXIS 0;"}
→ {"status":"ok"}
```

### B. WITH JACKKNIFE ALONG (Bee's new verb from merged main)

```
POST /v1/gql {"query":"INTEGRATE nonexistent_bundle MEASURE avg(temp) WITH JACKKNIFE ALONG wind;"}
→ {"error":"No bundle: nonexistent_bundle"}
```

Parser accepted `WITH JACKKNIFE ALONG wind` and dispatched to executor (bundle-resolve returned a real "No bundle" — not "Parse error: Unknown statement"). Proves the new parser arm is live.

### C. Trailing-token rejection (proves merged parser)

```
POST /v1/gql {"query":"INGEST bogus_trail_probe FROM '/tmp/x.npz' FORMAT NPZ EXTRA_JUNK_TOKEN;"}
→ {"error":"Parse error: Statement parsed, but this trailing input is not a supported clause
    and was NOT executed: 'EXTRA_JUNK_TOKEN'. Remove it, or check the GQL reference..."}
```

Old image silently discarded the trailing token; new image rejects with a clear error naming the extra token. Confirms the merged parser is live.

### D. INGEST route-handler bypass (sanity)

```
POST /v1/gql {"query":"INGEST bee_deploy_probe_d FROM '/tmp/nonexistent.npz' FORMAT NPZ
                       AS GAUGE_FIELD GROUP SU(2) ON LATTICE l4_bee_deploy_probe;"}
→ {"error":"INGEST: source file not found: /tmp/nonexistent.npz"}
```

Executor-level error, not the pre-bypass route-handler `"No bundle:"`. Bypass is live.

## IMAGINE Phase 2 (Marcella sanity)

`POST /v1/bundles/imagine_deploy_probe_2026_07_02/imagine_coherence`

```
Response: dim=4, trajectory=3 steps, coherence=1.0, defect=0.0,
          max_imagined_curvature=4.0 (FS ceiling), max_accumulated_holonomy=0.5
```

Phase 2 path intact.

## Public-read allowlist

- `GET /v1/bundles/tetmesh_demo/records` (no API key) → 401.
  The bundle doesn't exist on production yet, so the allowlist gets no chance to short-circuit auth. Per Bee's note, loading `tetmesh_demo` once from any writable session materializes the bundle and then makes the demo publicly reachable. Deferred until Bee triggers the load.
- `GET /v1/bundles/claude_substrate_v0/records` (no API key) → 401. Correct — this bundle stays gated.

## claude_substrate_v0 restore

- Bundle wiped by the redeploy (expected per the known heap-only-persistence fragility).
- Recreated with schema `(ts INT, refs TEXT, thought_id TEXT, session TEXT, topic TEXT, content TEXT)`.
- Backup at `.deploy-backups/2026-06-29-morning/claude_substrate_v0_backup.json` has 20 records across 4 unique `ts` timestamps and 20 unique `thought_id`s.
- Both `/insert` and `/import` endpoints route through `engine.batch_insert()`, which deduplicates on a fingerprint that collapses the 20 → 4. Record count is 4 post-restore.
- Not a deploy regression — pre-existing quirk noted in the substrate-fragility memory record; deferred pending a snapshot-based restore path or a dedup-key change on the substrate schema.

## Features live post-deploy

1. `LATTICE ... FROM CUBIC L=N DIM=K OBC AXIS <k>` (open-boundary lattices)
2. `INGEST FORMAT NPZ ... AS GAUGE_FIELD GROUP <g> ON LATTICE <l>` with:
   - route-handler bypass on fresh bundle name
   - `vertex_a`/`vertex_b` emitted via lattice's column-major `site_of` (values equal `Lattice::VertexId`)
   - OBC boundary records omitted (record set == lattice edge set)
   - optional `KEY <name>` clause for multi-array NPZ archives
   - auto-detect dtype (f32 → f64 upconvert)
3. `CHERN_CLASS <bundle> ON LATTICE <l> GROUP <g> PER <field> INTO_COLUMN <col>` (per-config with write-back)
4. `SPECTRAL_GAUGE <bundle> WHERE <predicate> ON FIBER (...) GROUP <g>` (sector-stratified)
5. `WITH JACKKNIFE ALONG <field>` (Bee's new evidence-grade error bars on measure verbs)
6. Unknown-field validation on records
7. Trailing-token rejection in the parser
8. `INTEGRATE` fix family (from Bee's parallel session)
9. `tests/gql_reference_truth.rs` reference-truth suite (new)
10. tetmesh spec + companion site under `site/gigi-builds/`
11. `tetmesh_demo` allowlist wired in `fly.toml` (materializes on first bundle load)

## Notes

- The deploy workflow (`wf_071d5b2c-290`) aborted at Phase 2 because the fresh-boot heuristic (`uptime_secs < 60`) was wrong for Fly's rolling-update semantics — Fly config-updates the machine in place, preserving the app-process uptime. `flyctl status` confirms the new image tag is what the machine is actually running, which is authoritative. Post-abort probes here reproduce what the workflow's Phase 3-5 would have run.
- No `Co-Authored-By: Claude` footer on the ship commits (grep gate verified 0 on the merged range after Bee's parallel session's session merged in; her session's separately-authored commits on `origin/main` are out of scope for this ship's audit).
