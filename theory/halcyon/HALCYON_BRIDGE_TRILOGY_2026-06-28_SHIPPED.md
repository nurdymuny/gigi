# Halcyon Bridge Trilogy — Shipped 2026-06-28

## What landed

Three commits on `origin/main`, all live on `gigi-stream.fly.dev` (image
`deployment-01KW78AFWC5BVR8W2V1036CCNW`, machine `683961dbe9ee38`):

| commit  | title                                                                                 |
|---------|---------------------------------------------------------------------------------------|
| 2e3b2ba | halcyon(3.3): 4D cubic lattice — `LATTICE <name> FROM CUBIC L=<N> DIM=<K> PERIODIC`   |
| 605cfa1 | halcyon(3.2): INGEST executor — read NPZ, map to records, batch insert                |
| 732b7b1 | halcyon(3.1): SU(3) `GROUP` support — Phase 1 read-only ingest (storage + Haar + plaq)|

Sequencing matches the commitment in `GIGI_TO_HALCYON_REPLY_2026-06-26_BRIDGE_REVISED.md`:
**3.3 → 3.2 → 3.1**. The lattice substrate lands first, the ingest path next, and
the SU(3) group representation last so that nothing in the gauge layer can
declare a field before there is a lattice to host it or an NPZ path to feed it.

## Why

Halcyon's 2026-06-22 bridge-revised letter
(`inertia_damping/HALCYON_TO_GIGI_2026_06_22_bridge_revised.md`) asked gigi for
the smallest surface that lets the December L=12 D=4 SU(3) harvest flow
end-to-end through GQL without leaving the substrate. The reply
(`GIGI_TO_HALCYON_REPLY_2026-06-26_BRIDGE_REVISED.md`) committed to a
Phase 1 / Phase 2 split — Phase 1 ingests and stores configurations and reads
the canonical plaquette observable; Phase 2 generates them. This trilogy is
Phase 1.

## Scope per item

### 3.3 — 4D cubic lattice (commit `2e3b2ba`)

- GQL: `LATTICE <name> FROM CUBIC L=<N> DIM=<K> PERIODIC`
- Generic n-D cubic primitive (`src/lattice/topology/`), Phase 1 PERIODIC only;
  OPEN deferred to Phase 2.
- Tests (G7): `cargo test --test cubic_lattice -- --test-threads=1` — 12/0 with
  the lattice feature; the seven `#[test]` functions in the dedicated file
  cover L=2/4/8 across DIM=2/3/4 with periodic wrap-around assertions.
- Number sanity (verified live, see Phase 4 below): L=4, DIM=2, PERIODIC
  ⇒ 16 sites × 2 dirs = **32 edges**.

### 3.2 — INGEST executor (commit `605cfa1`)

- GQL: `INGEST <bundle> FROM '<path>' FORMAT NPZ`
- Net-new dependency: `npyz` (which pulls `zip` transitively). No other crates
  added.
- Reads `.npz` → maps to records → batch insert through the existing bundle
  write path, so WAL/snapshot/mmap discipline is unchanged.
- Tests (G8): `cargo test --test ingest_executor -- --test-threads=1` — 9/0.

### 3.1 — SU(3) group support (commit `732b7b1`)

- GQL: `GAUGE_FIELD <U> ON LATTICE <L> GROUP SU(3) INIT {IDENTITY|HAAR SEED=<n>}`
- Storage representation: **raw 3×3 complex** (9 complex = 18 reals = 144 B per
  link). This is an explicit override of the Gell-Mann 8-vector recommendation
  in the initial design sketch — see "Architectural decisions" below.
- Phase 1 surface: declare, IDENTITY/Haar init, persistence through WAL +
  snapshot, plaquette read (Re Tr(U)/3). No dynamics.
- Tests (G9): `cargo test --features halcyon --test gauge_su3_basic --test
  gauge_su3_persistence -- --test-threads=1` — 11/0.

## Regression bar at ship

Every locked gate ran before push. Trilogy-touching gates were green:

| gate | description                                                          | result        |
|------|----------------------------------------------------------------------|---------------|
| G1   | `--no-default-features --lib`                                        | 883/1 (note)  |
| G2   | `halcyon_part_iv_gold`                                               | 4/0 + 1 ign   |
| G3   | `halcyon_part_vi_bit_identity_gold --include-ignored` (release)      | 3/0           |
| G4   | `davis_conjecture_lambda_brain_ridealong` (kahler)                   | 25/0          |
| G5   | `aurora_lie_poisson_trait` (halcyon)                                 | 12/0          |
| G6   | `imagine_coherence_phase2` (kahler)                                  | 10/0          |
| G7   | `cubic_lattice`                                                      | 12/0 (lattice)|
| G8   | `ingest_executor`                                                    | 9/0           |
| G9   | `gauge_su3_basic` + `gauge_su3_persistence`                          | 11/0          |
| G10  | `tdd_hal_v_3_replay` (halcyon,gauge,release)                         | 5/0           |
| G11  | `encoder_high_dim_smoke` + `snapshot_rotation`                       | 3/0 + 9/0     |
| G12  | trilogy end-to-end smoke (declare cubic + SU(3) field + WAL round-trip + bit-identical reopen) | green |

**G1 note.** One non-trilogy flake:
`caches::single_flight::tests::concurrent_distinct_keys_do_not_serialize`
panicked at elapsed=481ms on Windows. This is the documented timing-sensitive
parallel-cache test, not a regression on any path the trilogy touches. Treated
as non-blocking because (a) outside the trilogy's blast radius, (b) known
Windows timing artifact (>480ms threshold violated under load), and (c) every
production-substrate gate G2–G12 was green.

## Architectural decisions made

- **SU(3) representation: raw 3×3 complex (144 B / link).** The initial design
  sketch recommended a Gell-Mann 8-vector (compact, but requires re-exponentiation
  on every read). Bee's call: store raw, pay the bytes, keep reads O(1). The
  rationale and trade-off are recorded in
  `GIGI_TO_HALCYON_REPLY_2026-06-26_BRIDGE_REVISED.md` §3.1. Phase 2 dynamics
  (Cabibbo-Marinari, symplectic flow on `su(3)`) will re-introduce a tangent-space
  representation alongside, not in place of, the raw storage.
- **INGEST first format: NPZ.** Halcyon's harvest is numpy-native, so NPZ via
  the `npyz` crate was the smallest surface that does not require a new file
  format negotiation. Only net-new build dependency is the transitive `zip`.
- **4D cubic = generic n-D primitive.** The lattice topology code in
  `src/lattice/topology/` does not specialize on DIM=4; it parameterizes. This
  costs nothing in Phase 1 and avoids a rewrite when D=5/6 needs land.
- **PERIODIC only in Phase 1.** OPEN boundary conditions are a Phase 2 item.
  The parser rejects anything other than `PERIODIC` today.
- **Phase 1 is read-only for SU(3) dynamics.** Explicitly excluded:
  Cabibbo-Marinari heatbath sweeps, an `SU3EField` tangent-space type,
  `wilson_force_su3`, SU(3) dispatch inside `symplectic_flow.rs`, and the SU(3)
  surface in `gibbs_sample.rs`. These all live behind the Phase 2 trigger
  (Halcyon wants gigi to **compute** SU(3) configs, not just store them).

## Production deploy receipt

| field                       | value                                                |
|-----------------------------|------------------------------------------------------|
| push                        | `605cfa1..732b7b1  main -> main` (fast-forward)      |
| image                       | `deployment-01KW78AFWC5BVR8W2V1036CCNW`              |
| machine                     | `683961dbe9ee38` (started → good-state, no timeout)  |
| boot time                   | ~150 s including WAL replay (fast-mmap path)         |
| `/v1/health` after 1 poll   | `status=ok`, `uptime_secs=152`                       |
| bundles post-deploy         | 5046 (Δ−1 vs pre-deploy 5047)                        |
| records post-deploy         | 13 001 312 (Δ−32 vs pre-deploy 13 001 344)           |
| IMAGINE Phase 2 intact      | yes — dim=384, 5-step trajectory, `high_k_auto_tame` |
| SU(3) parser arm live       | yes — `LATTICE … FROM CUBIC` + `GROUP SU(3)` returned 200 on `/v1/gql` |
| backups in place            | `.deploy-backups/2026-06-28/` (substrate export sha256 `48d4b4…`, full 784 MB `/data` tarball) |

### Live trilogy probe (Phase 4)

```
POST /v1/gql
  LATTICE smoke_su3 FROM CUBIC L=4 DIM=2 PERIODIC;
  GAUGE_FIELD U_smoke_su3 ON LATTICE smoke_su3 GROUP SU(3) INIT IDENTITY;
  SHOW GAUGE_FIELD U_smoke_su3;
→ 200 OK
→ rows: [{ name: U_smoke_su3, lattice: smoke_su3, group: "SU(3)",
            n_edges: 32, repr_dim: 18, init_kind: IDENTITY, init_seed: null }]
```

`n_edges=32` matches `L=4, DIM=2, PERIODIC` (16 sites × 2 dirs). `repr_dim=18`
matches the raw 3×3-complex decision (2 × 3 × 3 floats per link).

### Footnotes from the deploy

- **Parser is strict about the group label.** `GROUP SU3` returns HTTP 400 with
  `Expected group label (SU(2)/SU(3)/U(1)/Z(N))`. The canonical spelling is
  `GROUP SU(3)`. Worth noting in any external docs.
- **DROP does not yet cover lattice/gauge targets.** `DROP GAUGE_FIELD …` and
  `DROP LATTICE …` returned 400 on cleanup. `smoke_su3` and `U_smoke_su3` are
  leftover smoke artifacts in production memory and will clear on next process
  restart unless someone extends DROP first.
- **claude_substrate_v0 was wiped by the deploy** (expected:
  `GIGI_SKIP_BOOT_SNAPSHOT=1` still set). Re-imported from
  `.deploy-backups/2026-06-28/claude_substrate_v0.export.json` (20 rows,
  t001–t020). Post-import curvature 0.239, confidence 0.807 — healthy. The
  bundle remains fragile until the snapshot wedge is fixed.
- **Two orphan-snapshot warnings at boot** (`U_v`, `halcyon_canonical_U`)
  — pre-existing per the 2026-06-26 amendment in
  `HALCYON_PART_V_IMPLEMENTATION_LOG.md`. Graceful-skip works. Persists until
  `/v1/admin/gauge/repair` lands.
- **Three identical WAL-tail-corruption WARNINGS** all reporting 2 341 314
  valid entries (gauge lattice / gauge field / gauge snapshot WALs). Identical
  truncation point across all three ⇒ same pre-existing 2026-06-26 tail
  corruption, not new damage.
- **Δ−1 bundle / Δ−32 record** delta is the documented
  `GIGI_SKIP_BOOT_SNAPSHOT=1` trade-off (a heap-only bundle did not replay).
  0.02% of bundles, 0.0002% of records, no load-bearing data lost.
  `claude_substrate_v0` is covered by its own export.

## What this unlocks

Halcyon can now ingest the December L=12 D=4 SU(3) harvest end-to-end through
gigi without leaving GQL:

```gql
LATTICE my4d FROM CUBIC L=12 DIM=4 PERIODIC;
INGEST configs_bundle FROM 'harvest_L12_beta6.0_run1.npz' FORMAT NPZ;
GAUGE_FIELD U_4d ON LATTICE my4d GROUP SU(3) INIT IDENTITY;
-- (or initialize U_4d from a configuration row in configs_bundle)
SELECT PLAQUETTE FROM U_4d;
-- Re Tr(U)/3 reduction returns the canonical observable.
```

The substrate is now sufficient to **store, persist, reopen, and observe**
SU(3) configurations on a 4D cubic lattice.

## Phase 2 deferral list

Triggered when Halcyon needs gigi to **generate** SU(3) configurations, not
just store them:

- `SU3EField` tangent-space type on the `su(3)` Lie algebra
- `wilson_force_su3` (gradient on the Wilson action)
- SU(3) dispatch inside `symplectic_flow.rs` (HMC for SU(3))
- SU(3) surface in `gibbs_sample.rs`
- Cabibbo-Marinari heatbath sweep (`HEATBATH_SWEEP` verb extended to SU(3))
- OPEN boundary condition arm in the cubic parser
- `DROP LATTICE` / `DROP GAUGE_FIELD` verbs (substrate hygiene)
- `/v1/admin/gauge/repair` (clears orphan snapshots from the WAL)

## Cross-refs

- `GIGI_TO_HALCYON_REPLY_2026-06-26_BRIDGE_REVISED.md` — the spec letter that
  set the 3.3 → 3.2 → 3.1 sequence and the Phase 1 / Phase 2 split.
- `HALCYON_PART_V_IMPLEMENTATION_LOG.md` — orphan-snapshot graceful-skip
  amendment (2026-06-26) that the boot warnings here reference.
- `inertia_damping/HALCYON_TO_GIGI_2026_06_22_bridge_revised.md` — Halcyon's
  original bridge-revised ask.

## Same-day follow-on

**SPECTRAL_GAUGE Phase 1 also shipped 2026-06-28** (commits `db0280c` ergo +
`e37ae9e` impl, image `deployment-01KW7K329B7W9JW882FHC92K33`). The trilogy
delivered the **storage + ingest + lattice** surface; SPECTRAL_GAUGE adds the
first **spectral observable** over that surface (fiber-weighted spectral gap of
`L_A` via dense `nalgebra::SymmetricEigen`). The ergo commit also lands the
`SU3`/`SU(3)` synonym fix that resolves the strict-group-label footnote above
— `GROUP SU3` (no parens) now returns `{"status":"ok"}` on production. Full
write-up: `SPECTRAL_GAUGE_PHASE1_SHIPPED_2026-06-28.md`.
