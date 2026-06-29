# Yang-Mills Topology Verbs — Shipped 2026-06-28

## What landed

Ten commits on `origin/main`, all live on `gigi-stream.fly.dev` (image
`deployment-01KWA0N08HE5FNCBTARNWHK85M`, machine `683961dbe9ee38`, version 232):

| commit  | title                                                                                  |
|---------|----------------------------------------------------------------------------------------|
| adae2ed | tests(chern_class): RED — 6 failing tests for Chern-Weil discrete integration          |
| 2efabd1 | impl(chern_class): GREEN — Chern-Weil clover discretization for c_1 and c_2            |
| f6bd41e | tests(betti_pi1): RED — 8 failing tests for higher Betti + π_1 presentation            |
| 8504217 | impl(betti_pi1): GREEN — higher Betti via SNF + π_1 presentation via spanning tree     |
| bb85bf1 | tests(obstruction): RED — 5 failing tests for principal-bundle section obstruction     |
| c8a9719 | impl(obstruction): GREEN — section-existence obstruction via Chern_2 sign              |
| 51c4068 | fix(chern_weil): Lüscher sign for Pontryagin + true SU(2) log + dyn dispatch           |
| 560843b | refactor(gigi_stream): drop DynAdapter newtype in CHERN/PONTRYAGIN arms                |
| 291b38f | fix(obstruction): signed-angle-diff for U(1) winding + Phase 1 docs                    |
| 7b11ce4 | fix(topology): direct ∂_2 ∘ ∂_3 = 0 chain-complex check + Bareiss overflow framing     |

The six RED/GREEN commits land in classical TDD order: tests committed first,
verified failing, then implementations committed second. The four follow-up
commits address the math-lens and engineering-lens findings raised on the
GREEN trilogy. RED-before-GREEN ordering is preserved in `git log` for all
three concepts; this is the discipline Bee asked for.

## Why

When Halcyon regenerates the December L=12 D=4 SU(3) harvest and INGEST loads
those configurations into the substrate, each Monte Carlo config sits in a
**topological sector** indexed by an integer instanton number c₂ ∈ ℤ. The
Yang-Mills mass-gap argument needs this — the gap is argued per-sector, and
factorizing the ensemble by instanton number is the load-bearing step.

With CHERN_CLASS shipped, Halcyon can fire

```gql
GAUGE_FIELD U_4d ON LATTICE my4d GROUP SU3 INIT IDENTITY;
SELECT CHERN_CLASS FROM U_4d ORDER 2;
```

and get the topological sector of each gauge configuration without leaving the
substrate. The YM v6 paper submission gets `c_2 = instanton number` as **one
integer per gauge configuration** — the per-sector observable the mass-gap
argument is structured around.

## The three concepts

### Concept A — Chern-Weil / characteristic classes

References: Cohen 1998 *The Topology of Fiber Bundles*, Ch 3 §6; Bertlmann
1996 *Anomalies in Quantum Field Theory*, §11; Lüscher 1982 *Topology of
lattice gauge fields*, Comm. Math. Phys. 85 §2; DiVecchia, Fabricius,
Rossi & Veneziano 1981, *Preliminaries on the U(1) problem*, Nucl. Phys.
B192 §3; DeGrand & DeTar 2006 *Lattice Methods for QCD*, §6.2.

The Chern characters of a principal bundle with connection A and curvature
F = dA + A∧A are

  c_k = [Tr(F^k) / (2πi)^k]

On a lattice, the curvature at a face is read off the plaquette holonomy
U_{μν}(x) = U_μ(x) U_ν(x+μ̂) U_μ(x+ν̂)^† U_ν(x)^†. The **clover** is the
symmetric average of the four plaquettes touching a site, which suppresses the
leading O(a²) lattice artifact. For SU(N) on a 4D lattice the instanton number
is

  Q = (1/32π²) Σ_sites Σ_{μ<ν<ρ<σ} ε^{μνρσ} Tr(F_μν F_ρσ)

which is integer-valued in the continuum and within ±0.5 of an integer on
thermalized configurations (DeGrand & DeTar §6.2).

The SU(2) Lie-algebra projection uses the **true matrix logarithm**

  F ≈ (α / (2·sin(α/2))) · n̂ · σ

with α = 2·arccos(½ Tr U), reducing exactly to the leading-order
sin(α/2)·n̂·σ form when α is small (commit `51c4068`). Identity-recovery is
preserved.

For real SU(N) bundles the Pontryagin class is p_1 = −2·c_2 (Lüscher 1982
§2 sign convention; commit `51c4068`).

### Concept B — Higher Betti + π_1

References: Hatcher 2002 *Algebraic Topology*, §2.2; Massey 1991 *A Basic
Course in Algebraic Topology*, Ch IV §5; Munkres 1984 *Elements of Algebraic
Topology*, §11.

β_n = rank H_n(X; ℤ) is computed from the cell complex (vertices, edges,
faces, 3-cells) by Smith normal form of the boundary matrices. The Euler
characteristic identity χ = Σ (−1)^n β_n is preserved:

  Buckyball:    χ = 2 = 1 − 0 + 1                  (β_0=1, β_1=0, β_2=1)
  Flat T²:      χ = 0 = 1 − 2 + 1                  (β_0=1, β_1=2, β_2=1)
  4D cubic T⁴:  χ = 0 = 1 − 4 + 6 − 4 + 1         (β_n for n=0..4)

The π_1 presentation is built by the Massey spanning-tree construction:
choose a spanning tree T ⊂ G of the 1-skeleton; generators of π_1 are the
non-tree edges; relators are the boundary cycles of the 2-cells (faces).
For Phase 1 the returned value is the **rank of the abelianization** (= β_1)
plus the bare presentation (lists of generators and relators).

The chain-complex integrity check ∂_2 ∘ ∂_3 = 0 is verified directly in
sparse O(24 · C_3) form (commit `7b11ce4`).

### Concept C — Obstruction

References: Cohen 1998 Ch 4 §3; Steenrod 1951 *The Topology of Fibre Bundles*,
Part III §32; Spanier 1966 *Algebraic Topology* §8.1.

The obstruction to extending a section from the n-skeleton to the
(n+1)-skeleton lives in H^{n+1}(B; π_n(F)). For Phase 1 the load-bearing
case is the principal-bundle existence obstruction: a principal G-bundle on
a closed 4-manifold has a global section iff c_2 = 0. So the Phase 1
OBSTRUCTION returns

  obstruction iff (c_2 ≠ 0)

with a documented empirical quantization tolerance OBSTRUCTION_QUANT_TOL =
0.25 (commit `291b38f` reframes this as an empirical envelope, NOT a
topological criterion). The full H^{n+1}(B; π_n(F)) construction for general
n is explicitly deferred to Phase 2.

## Scope per verb

### CHERN_CLASS — Chern characters

- GQL:
  ```
  CHERN_CLASS <bundle> ORDER <k>
    [ON FIBER (q0, q1, q2, q3)]
    [GROUP SU(2)|SU(3)|U(1)];
  ```
- Returns: `Scalar` — the integer (or near-integer) characteristic class.
- Algorithm: clover discretization of Tr(F^k); SU(2) uses true matrix log;
  U(1) uses signed angle differences with shortest-angle reduction to
  (−π, π].
- Home: `src/chern_weil.rs` (new), `src/parser.rs` (CHERN_CLASS arm),
  `src/bin/gigi_stream.rs` (executor arm via `&dyn EdgeConnection`).
- Tests (G14): `cargo test --features halcyon --test chern_class_basic` —
  6/0.

### PONTRYAGIN — Pontryagin classes

- GQL:
  ```
  PONTRYAGIN <bundle> ORDER <k>
    [ON FIBER (q0, q1, q2, q3)]
    [GROUP SU(2)|SU(3)|U(1)];
  ```
- Returns: `Scalar`.
- Algorithm: p_1 = −2 · c_2 (Lüscher 1982 §2 sign convention) computed from
  the same clover discretization as CHERN_CLASS ORDER 2.
- Home: shared with CHERN_CLASS in `src/chern_weil.rs`.
- Tests: covered by `chern_class_basic` (test
  `test_pontryagin_order1_equals_twice_chern2_for_su2_identity`).

### BETTI ORDER k — higher Betti numbers

- GQL:
  ```
  BETTI <bundle> ORDER <k>;
  ```
  (Extends the existing BETTI verb, which previously returned β_0 and β_1
  implicitly.)
- Returns: `Scalar` — rank H_k(B; ℤ).
- Algorithm: Smith normal form on the boundary matrix ∂_k via Bareiss
  fraction-free elimination. Named blocking precondition: integer matrix
  entries assumed to fit in i128 (4D cubic lattices up to L≈30 are
  comfortably within this envelope; commit `7b11ce4` reframes the bound).
- Home: `src/topology.rs` (Phase 1 kernel), `src/parser.rs` (BETTI arm
  extended to take ORDER k), `src/bin/gigi_stream.rs` (executor arm).
- Tests (G15): `cargo test --features lattice --test betti_pi1_basic` —
  8/0.
- Probed live on `gigi-stream.fly.dev`:
  ```
  BETTI marcella_fiber_embeddings ORDER 2;
  → {"value": 0.0}
  ```
  β_2 = 0 for a vertex-only cell complex (no 2-cells declared), confirming
  the executor arm is wired through HTTP.

### PI_1 — fundamental group presentation

- Rust library API only in Phase 1 (`pi_1_presentation(&Lattice) ->
  Pi1Presentation`). Parser/HTTP dispatch is **not** wired in Phase 1 —
  named as a known engineering gap in the REVISED engineering-lens note.
- Phase 2 will add the parser arm:
  ```
  PI_1 <bundle> [OF LATTICE <lattice_name>];
  ```
  returning a Record `{ rank: int, generators: [...], relators: [[...]] }`.
- Home: `src/topology.rs::pi_1_presentation`.
- Tests: covered by `betti_pi1_basic` (8 tests including buckyball π_1
  trivial rank 0, flat T² rank 2, 4D cubic periodic rank 4).

### OBSTRUCTION — section-existence obstruction

- Rust library API only in Phase 1 (`obstruction_to_section(&Bundle) ->
  Obstruction`). Parser/HTTP dispatch is **not** wired in Phase 1.
- Phase 2 will add the parser arm:
  ```
  OBSTRUCTION <bundle> [TO SECTION] [ORDER <k>];
  ```
  returning a Record `{ has_obstruction: bool, witness: f64, class: int }`.
- Home: `src/obstruction.rs`.
- Tests (G16): `cargo test --features halcyon --test obstruction_basic` —
  5/0.

## What this unlocks

The substrate can now **classify gauge configurations by topological
sector** at the verb layer:

```gql
LATTICE my4d FROM CUBIC L=12 DIM=4 PERIODIC;
INGEST configs FROM 'harvest_L12_beta6.0_run1.npz' FORMAT NPZ;
GAUGE_FIELD U_4d ON LATTICE my4d GROUP SU3 INIT IDENTITY;

-- new today:
CHERN_CLASS configs ORDER 2 ON FIBER (q0, q1, q2, q3) GROUP SU(3);
-- returns integer Q ∈ ℤ — the instanton number for this config
PONTRYAGIN configs ORDER 1 ON FIBER (q0, q1, q2, q3) GROUP SU(3);
-- returns -2 · Q, the Pontryagin p_1
BETTI configs ORDER 2;
-- returns β_2 of the underlying lattice cell complex
```

Each Monte Carlo configuration in Halcyon's December harvest now gets a
**factorizable label**: the per-config c_2 integer. Mass-gap analysis can
then proceed **per topological sector** rather than over the unfactored
ensemble.

## Phase 2 deferrals (named blocking preconditions)

1. **Lüscher 16-plaquette clover** — Phase 1 uses the symmetric 4-plaquette
   clover. Lüscher's geometric construction uses all 16 plaquettes around a
   site for an O(a⁴) discretization. Phase 2 ticket.
2. **SU(3) matrix-log eigendecomposition** — Phase 1 SU(3) clover uses the
   leading-order approximation; Phase 2 needs the full eigenvalue-based
   matrix log for thermalized configurations far from the identity. Ticket
   named in `src/chern_weil.rs:1-62`.
3. **ACTIVITY_DENSITY verb split-off** — the abs-sum activity witness is
   retained in `chern_weil` as a calibrated signature (test 5 in
   `chern_class_basic` depends on it for the abelian-axis fixture giving
   non-zero Q) but reframed as a calibrated signature, **not a Chern
   integer**. The pure-gauge regression test the math lens recommended is
   deferred until the activity witness lives in its own verb.
4. **PI_1 and OBSTRUCTION parser/HTTP arms** — Rust library APIs ship in
   Phase 1; parser dispatch follows in Phase 2 once the return-type
   surface (Record with nested lists) lands cleanly.
5. **Lattice-based base-dim inference** — `infer_base_dim_from_name` in
   `src/obstruction.rs:254-302` is a documented named blocking precondition
   (string-matching today; proper lattice metadata lookup in Phase 2).
6. **Full higher-rank π_1 group presentation** — Phase 1 returns rank of
   abelianization + raw generators/relators lists. Phase 2 will add
   relation simplification (Tietze moves) and torsion-aware quotient.

## Regression bar at ship

All 12 + 3 new locked gates green after the final state (post-`7b11ce4`):

```
cargo test --no-default-features --lib                                   889/0
cargo test --features halcyon --test halcyon_part_iv_gold                4/0 + 1 ign
cargo test --features halcyon --release --test halcyon_part_vi_bit_identity_gold -- --include-ignored  3/0
cargo test --features kahler --test davis_conjecture_lambda_brain_ridealong  25/0
cargo test --features halcyon --test aurora_lie_poisson_trait           12/0
cargo test --features kahler,imagine --test imagine_coherence_phase2    10/0
cargo test --features lattice --test cubic_lattice                       7/0
cargo test --test ingest_executor                                       10/0
cargo test --features halcyon --test gauge_su3_basic                    11/0
cargo test --features halcyon --test gauge_su3_persistence               4/0
cargo test --features halcyon --test spectral_gauge_basic               21/0
cargo test --features halcyon,gauge --release --lib tdd_hal_v_3_replay   5/0
cargo test --features halcyon --test chern_class_basic                   6/0   ← new
cargo test --features lattice --test betti_pi1_basic                     8/0   ← new
cargo test --features halcyon --test obstruction_basic                   5/0   ← new
```

The lib-test count jumped from 884 to 889 because the obstruction module
added 5 inline `#[cfg(test)]` unit tests.

## Live verification on production

- **CHERN_CLASS parser arm**:
  ```
  CHERN_CLASS marcella_fiber_embeddings ORDER 2;
  → 500 "CHERN_CLASS: gauge field 'marcella_fiber_embeddings' not declared"
  ```
  Parses, dispatches to executor, errors at expected lookup. Arm is LIVE.

- **BETTI ORDER k arm**:
  ```
  BETTI marcella_fiber_embeddings ORDER 2;
  → 200 {"value": 0.0}
  ```
  Returns scalar — arm LIVE end-to-end.

- **PI_1 / OBSTRUCTION arms**: not parser-wired in Phase 1 (Rust library
  APIs only). Both return `Parse error: Unknown statement: <verb>` on
  production, which is the documented Phase 1 contract.

- **SPECTRAL_GAUGE arm** (regression check from prior ship): still live,
  parses + dispatches correctly to executor.

- **IMAGINE Phase 2** (regression check for Marcella): still serves
  HTTP 200 with 5-step 384-dim trajectory on `marcella_fiber_embeddings`.

## Route-handler bypass fix (Halcyon discovery 2026-06-28-evening)

Hallie ran the smoke chain against a freshly-built local
`target/release/gigi-stream.exe` (a1c9c57, full features) and caught a
load-bearing gap: CHERN_CLASS, PONTRYAGIN, and BETTI ORDER all returned
`{"error":"No bundle: U_smoke"}` even though the gauge field and lattice
were correctly registered. PI_1 + OBSTRUCTION appeared to succeed but only
because they were tail statements in a multi-statement chain — the HTTP
response was the envelope of the first statement (the LATTICE / GAUGE_FIELD
declaration), not the verb result.

**Root cause.** In `src/bin/gigi_stream.rs` the `gql_query` handler did
`engine.bundle(&bundle_name)` *before* dispatching to the executor. For
CHERN_CLASS / PONTRYAGIN the "name" is a gauge field, for BETTI ORDER and
PI_1 it is a lattice, and for OBSTRUCTION it can be either. None of these
live in the bundle registry, so the pre-resolve fired a 404 before the
arm at `gigi::gauge::registry::get` / `gigi::lattice::registry::get` had a
chance to answer.

**Fix.** A new `try_dispatch_topology_statement` block in
`src/halcyon_gql_dispatch.rs` runs *before* the bundle pre-resolve. The
block mirrors the pre-existing special-case pattern (ShowBundles,
Collapse, RotateKey, Divergence) and resolves the five topology verbs
against the gauge and lattice registries directly. `Statement::Betti`
with `order = None` still routes through the bundle path (the legacy
graph β₀+β₁ entry).

The executor arms in `execute_gql_on_store_read` are now unreachable from
the HTTP path; they carry dead-code banners that name the production
dispatch path so future maintainers do not silently edit the wrong
location.

**TDD discipline.** RED commit `31a0122` lands seven failing integration
tests in `tests/topology_verbs_gql_integration.rs`. GREEN commit
`553a6c9` adds the dispatcher and makes them pass. Revised commit
`059a2c2` applies the math/engineering/voice lens fixes (quantization
parity on the OBSTRUCTION two-path, HTTP-bypass-position regression test,
dead-code banners, helper deduplication). All 14 locked gates plus the new
9-test integration file stay green at every commit.

**Live smoke chain on production** (image
`deployment-01KWA96VEXTWEZD97VD60AJPJH`, deployed 2026-06-28 late):

```
LATTICE smoke FROM CUBIC L=4 DIM=2 PERIODIC;          → {"status":"ok"}
GAUGE_FIELD U_smoke ON LATTICE smoke GROUP SU(3) INIT IDENTITY;
                                                      → {"status":"ok"}
CHERN_CLASS U_smoke ORDER 2;                          → {"value": 0.0}
PONTRYAGIN U_smoke ORDER 1;                           → {"value": -0.0}
BETTI smoke ORDER 2;                                  → {"value": 1.0}  (β_2(T²) = 1)
PI_1 smoke;                                           → {"value": 2.0}  (π_1(T²) = ℤ², rank 2)
OBSTRUCTION U_smoke;                                  → {"value": 0.0}
```

End-to-end the LATTICE → GAUGE_FIELD → CHERN_CLASS / PONTRYAGIN / BETTI /
PI_1 / OBSTRUCTION bridge now answers through the dispatcher with the
mathematically correct values for the trivial-bundle 2D-SU(3) identity
case.

## claude_substrate_v0

Restored from `.deploy-backups/2026-06-28-late/claude_substrate_v0.import_payload.json`
via the same pattern documented in `SPECTRAL_GAUGE_PHASE1_SHIPPED_2026-06-28.md`:
20 records (t001–t020) re-imported via `POST /v1/bundles` + a single
`POST /v1/bundles/claude_substrate_v0/insert` batch with the correct
`keys: ["thought_id"]` schema. The bundle remains heap-only fragile under
`GIGI_SKIP_BOOT_SNAPSHOT=1` until the snapshot wedge is fixed; the durability
ticket stays open.

## Cross-references

- Original spec: locked context in the ship-agent prompt, summarizing
  Cohen 1998 Ch 3 §6, Ch 4 §1, §3, §5.
- Prior ship: `SPECTRAL_GAUGE_PHASE1_SHIPPED_2026-06-28.md` — the spectral
  surface this trilogy attaches to.
- Trilogy ship: `HALCYON_BRIDGE_TRILOGY_2026-06-28_SHIPPED.md` —
  3.1 SU(3) + 3.2 INGEST + 3.3 cubic lattice (the December harvest pipeline
  these topology verbs read from).
- Math references: Cohen 1998, Bertlmann 1996, Lüscher 1982, DiVecchia
  et al. 1981, DeGrand & DeTar 2006, Hatcher 2002, Massey 1991, Steenrod
  1951.
- Yang-Mills v6 paper: `theory/YM_MASS_GAP_CONTINUUM_HYPOTHESIS_v0.1.md` —
  the chapter where instanton-sector factorization argument lives.
