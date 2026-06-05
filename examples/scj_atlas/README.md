# SCJ Windows Atlas — substrate landing zone

This directory is the landing zone for the **Shadow Clone Jutsu (SCJ)** team's
**Windows Atlas on Gigi v0.1** ingest target. SCJ is Gigi's third major
downstream consumer (after Marcella and KRAKEN) and is about to ingest the
full Windows binary corpus — every function in ntoskrnl, win32k\*, the
Hyper-V family, system drivers, system DLLs — as three Gigi bundles keyed
by `(module, rva)`, with the SCJ feature ontology and the GASM map scalar
fields as fiber data.

## Correspondence

This work was negotiated across three letters (2026-06-05 → 2026-06-06):

| Letter | Path | Author |
|---|---|---|
| SCJ heads-up letter | `theory/scj/REPLY_FROM_SCJ_*.md` (the 2026-06-05 inbound) | SCJ |
| Gigi reply (asks A–F resolved) | `theory/scj/REPLY_TO_LETTER_2026-06-05.md` | Gigi engine team |
| SCJ commitment + decisions | `theory/scj/REPLY_FROM_SCJ_2026-06-06.md` | SCJ |
| Gigi ack + audit + engine prep | `theory/scj/REPLY_TO_REPLY_2026-06-06.md` | Gigi engine team |

Read the second exchange first; the first exchange establishes context for
the six asks, the second locks the contract.

## What lands here

Once SCJ ships deliverable 2A (target: ready against `scj-v0.1-substrate`),
this directory will hold:

```
examples/scj_atlas/
  ├── README.md                  (this file)
  ├── windows_fns.gql            (BUNDLE DDL — function ontology)
  ├── windows_calls.gql          (BUNDLE DDL — call-graph topology)
  ├── windows_sinks.gql          (BUNDLE DDL — sink ontology)
  ├── scj_vid_smoke.py           (orchestrator: ingest vid.sys, run SUSANOO top-10)
  ├── evidence_pack_context_schema_v0.1.md  (frozen geometric_context schema)
  ├── candidate_pack.md.j2       (stock minijinja template for evidence packs)
  ├── vid_sys_derived/           (derived artifacts under PolyForm NC)
  │   ├── gasm_map.h5            (heat/taint/curvature/near_trust_boundary scalars)
  │   ├── decompiled.jsonl       (function bodies + CFG hashes)
  │   ├── embeddings.npy         (128-d IR2Vec+GraphSAGE+PCA, unit-normalized)
  │   ├── sinks_reached.jsonl    (per-function sink ontology hits)
  │   └── confirmed_bugs.csv     (SUSANOO + 7 others — calibration anchors)
  └── calibration/
      ├── cross_version_identity_v01.csv  (23H2 ↔ 24H2, ~500 hand-labeled pairs)
      └── ood_baseline_v01.csv             (500 vid.sys ID + 500 pwsh/notepad OOD)
```

`vid.sys` itself is Microsoft-owned and is **not** shipped here. The
derived-work artifacts under `vid_sys_derived/` are sufficient to run the
smoke test on Gigi CI without the binary, under PolyForm NC.

## What the contract test covers

`tests/scj_atlas_contract.rs` asserts (when the DDLs land + the smoke
artifacts populate):

1. All three BUNDLE DDLs parse against the frozen `scj-v0.1-substrate`
   grammar without hand-edits.
2. `schema → DHOOM emit → re-ingest` is byte-identical on vid.sys-scale
   synthetic data.
3. `SIMILAR TOP 10` is run-to-run deterministic — critical for SUSANOO
   top-10 reproducibility (acceptance gate: SUSANOO at rank 1,
   TSUKUYOMI in {2,3,4}, geodesic distance within ±5% of 2.018).
4. TAGSET shadow-encoding equivalence (gated on Ask A engine-side).
5. `instant-distance` version pin (currently `0.6`). Bumping this is a
   substrate contract change and forces a deliberate SCJ-notification step.

The tests are currently `#[ignore]`'d. When deliverable 2A lands, flip the
ignore attribute off and the contract goes live.

## The pinned substrate

SCJ pins to the `scj-v0.1-substrate` branch (cut off `gigi-stream` main at
the commit that includes this scaffold). The branch is **frozen** for the
duration of SCJ's v0.1 ingest — no force-pushes, no rebases. When SCJ hits
issues, we cherry-pick fixes onto the frozen branch; SCJ re-pins a tag, not
a moving head. Same discipline used for Marcella's IMAGINE_COHERENCE
Phase 1.

The pinned substrate ships with:
- Sharding T1–T13 green (Poincaré-to-sharding lineage)
- Transactions Phase 1–4 (atomic sheaf commits + snapshot isolation + MVCC
  + geometric coherence)
- IMAGINE/WALK with the Marcella trust envelope
- Brain primitives L9–L13 (12 cognitive primitives via Friston substrate)
- `instant-distance = "0.6"` as the HNSW backend

See `theory/scj/REPLY_TO_REPLY_2026-06-06.md` §4 for the full engine-side
prep manifest and §1A for the HNSW recall contract.

## Roadmap mapping

The four engine-side asks that will land downstream:

| Ask | Engine ship | SCJ v0.1 workaround |
|---|---|---|
| A — TAGSET fiber type | `FieldType::TagSet` + Roaring transpose index | 17-boolean shadow encoding (`reaches_<sink>` BOOLEAN INDEX per sink for top-17) |
| B — ALTERNATE KEY identity | `ALTERNATE KEY identity (...)` + `DIFF BY identity` | client-side `identity_hash TEXT` (SHA-256 over cfg_shape, decompiled_sha256, pdb_symbol), JOIN-via-COVER on the text field |
| C — HNSW for single-field vectors | finish the plumbing — `instant-distance` already in tree | ingest as `VECTOR DIM 128` as if it's HNSW-backed; it will be |
| D — Chebyshev-as-a-primitive | `POST /v1/bundles/{name}/heat_kernel` + future `wgpu-spectral` flag | `gasm/heat.py` wrapped behind `compute_heat_kernel(adj, source, t)`, swap to HTTP when shipped |
| E — `EMIT TEMPLATED` | template engine + `TEMPLATED FROM template.{md,html,txt}` | client-side minijinja against `EMIT JSON` |
| F — §10 post-Kähler entry | review + merge SCJ PR to `theory/post_kahler_directions/` | drafted by SCJ in parallel with 2A |

Until each engine-side ship lands, SCJ runs the v0.1 workaround. Migration
to the engine-side path is a one-line schema/call-site change because the
contract was negotiated end-to-end before SCJ wrote any ingest code.

— Gigi engine team · 2026-06-06
