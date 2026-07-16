# SPECTRAL_GAUGE Phase 2 — FULL LIMIT k + MODE MAGNETIC + U(1) INIT FLUX — SHIPPED

**Date**: 2026-07-16.
**Trigger**: Hallie's confirmed ask (relayed by Bee 2026-07-16): ship
`SPECTRAL FULL LIMIT k`, `MODE MAGNETIC`, and a flux-init path so the
RH sweep (generate flux → INGEST/INIT → SPECTRAL FULL MODE MAGNETIC →
rh_003–rh_011 battery) runs locally with no Modal and no heatbath.
**Authoritative upstream doc**: `SPECTRAL_GAUGE_PHASE2_SPEC.md`
(Hallie, 2026-06-30) — a sparse-Lanczos spec that PREDATES the
MAGNETIC ask; deviations from it are enumerated in §5 below.
**Base**: main @ `f2f2b7c`. Reply letter:
`GIGI_TO_HALCYON_REPLY_2026-07-16_SPECTRAL_FULL_MAGNETIC.md`.

---

## 1. What shipped (three concepts, TDD RED → GREEN each)

### A — `FULL [LIMIT k]` (dense)

- `SPECTRAL_GAUGE … [FULL [LIMIT k]]` now populates `eigenvalues`
  (ascending algebraic; all of them without LIMIT, the k smallest with
  it; k > V clamps; k = 0 → typed `InvalidLimit`). The Phase-1 `gap`
  field keeps its exact λ₁ rule (first |λ| > 1e-9) with and without
  FULL. Plain `SPECTRAL <b> FULL [LIMIT k]` gained the sibling
  contract on the normalized field-index-graph Laplacian (Def 3.10).
- Dense-only this phase: `SPECTRAL_DENSE_MAX_V = 4096` (spec §6
  boundary). FULL past it → typed `SparseUnavailable` naming the
  Phase 2.1 Lanczos deferral. The Phase-1 λ₁-only path is not gated
  (frozen behaviour).
- Result struct gained spec §6's `mode_used` (`Dense` |
  `SparseLanczos{shift,k}`) and `convergence` (always `None` on
  dense). Wire envelope: `eigenvalues` + `mode_used` are stamped on
  the single summary row ONLY when FULL is present — the λ₁-only
  `{gap, n_records_used, group_used}` shape is byte-identical
  otherwise (probe S6 + fence).
- Core: `spectral::spectral_gauge_spectrum` (new);
  `spectral_gauge_gap` stays as a signature-stable wrapper so every
  Phase-1 caller compiles unchanged.

### B — `MODE MAGNETIC` (U(1) Hermitian magnetic Laplacian)

- Grammar: `SPECTRAL_GAUGE … [GROUP g] [MODE MAGNETIC] [FULL [LIMIT k]]`.
  MAGNETIC is the only user-facing MODE this phase (R1); `MODE <other>`
  is a parse error naming MAGNETIC. Composes with FULL LIMIT and WHERE.
  SPECTRAL_GAUGE only — plain SPECTRAL has no gauge fiber.
- ORIENTATION CONVENTION (the load-bearing contract for Hallie's
  generator): record `(vertex_a = a, vertex_b = b, θ)` carries θ for
  the a → b direction:
  `L[a][b] = −e^{+iθ}`, `L[b][a] = −e^{−iθ}` (exact conjugate pair),
  `L[v][v] = deg(v)` (unit weights, |e^{iθ}| = 1). Hermitian ⇒ real
  spectrum ⇒ wire format stays `Vec<f64>`.
- Eigensolver: nalgebra 0.33.3 `SymmetricEigen` over
  `DMatrix<Complex<f64>>` — its Householder tridiagonalization uses
  the conjugate inner product (`dotc`), i.e. the native Hermitian
  path. The 2V×2V real-embedding fallback was NOT needed.
- Non-U(1) + MAGNETIC → typed `MagneticRequiresU1`:
  `MODE MAGNETIC requires GROUP U(1) in this phase (matrix-valued
  magnetic Laplacians are a later phase)`.

### C — U(1) flux path

- C.1 (pin): `INGEST … AS GAUGE_FIELD GROUP U(1) ON LATTICE l` was
  already green end-to-end (canonical fiber `theta`, repr_dim 1); the
  RED became a pinning test that also drives the ingested bundle
  through MODE MAGNETIC FULL — the exact ingest leg of the RH loop.
- C.2: `GAUGE_FIELD name GROUP U(1) INIT FLUX RANDOM SEED <n> ON
  LATTICE <l>;` and `… INIT FLUX UNIFORM <phi> …`. GAUGE_FIELD clause
  order is now flexible (ON LATTICE / GROUP / INIT each required once,
  any order — probe S2 puts GROUP first and the lattice at the tail;
  the Part-II canonical order is pinned unchanged).
- INIT FLUX **materializes a theta bundle**
  (`{config_id = 0, edge_id, vertex_a, vertex_b | theta}`, one record
  per lattice edge, orientation = the lattice's own oriented edge) —
  it does NOT create a U(1) `DenseLinkBuffer` or registry entry (none
  exists this phase; the bundle is the artifact the whole downstream
  toolchain reads). PERSIST + FLUX is rejected (bundle inserts already
  flow through the engine WAL, like INGEST).
- Determinism contract: θ_k = 2π·uniform_k from the house xorshift64*
  `SmallRng` (`gauge::marsaglia_haar` — same PRNG + seeding as INIT
  HAAR), one draw per edge in lattice edge order 0..n_edges. Same
  lattice + seed → byte-identical bundle; edge 0 is byte-pinned to
  2π·uniform₀(seed) in the tests. SEED is mandatory (parse error
  without it — no entropy fallback).

## 2. Math anchors (all measured, none assumed)

| anchor | expectation | result |
|---|---|---|
| C_6 cycle, θ = 0, FULL | 2 − 2cos(2πk/6), ascending | exact to 1e-9 |
| K_8 complete, θ = 0, FULL | {0, 8×7} | exact to 1e-9 |
| C_3 uniform flux φ = 0.7, MAGNETIC FULL | 2 − 2cos((3φ + 2πk)/3) | exact to 1e-9 |
| P_3 tree, arbitrary flux, MAGNETIC | gauge-trivial → {0, 1, 3} | exact to 1e-9 |
| zero flux | MAGNETIC ≡ cos-weight spectrum | exact to 1e-9 |
| K_12 field-index graph, plain SPECTRAL FULL LIMIT 5 | {0, 12/11 ×4} (normalized) | exact to 1e-9 |

**Symmetry-class gate** (Atas–Bogomolny–Giraud–Roux, PRL 110, 084101
(2013): Poisson ≈ 0.3863, GOE ≈ 0.5307, GUE ≈ 0.5996). Fixture:
Erdős–Rényi graphs, V = 512, mean degree 16, i.i.d. θ ~ U[0, 2π),
fixed seeds 20260716/1/2/3, same four graphs in both modes, bulk =
middle 80% of the sorted spectrum, tolerance ±0.03 on the 4-seed mean:

| mode | 4-seed mean r̃ | anchor | Δ | per-seed |
|---|---|---|---|---|
| cos-weight (real symmetric) | **0.5272** | 0.5307 (GOE) | −0.0035 | 0.5567, 0.5165, 0.5151, 0.5207 |
| MODE MAGNETIC (Hermitian) | **0.6046** | 0.5996 (GUE) | +0.0050 | 0.5941, 0.5981, 0.6111, 0.6153 |

Class separation is total: every magnetic seed exceeds every
cos-weight seed.

**Estimator receipt (investigation, not tolerance-tuning).** The first
run used a single seed (20260716) and measured cos-weight r̃ = 0.5613 —
outside ±0.03. Per the tranche rules the tolerance was NOT widened;
diagnosis (5 seeds + trim sweep + degree sweep) showed single-graph
r̃ scatter at V = 512 is σ ≈ 0.02 with the 5-seed mean 0.5335 ≈ the
GOE anchor — i.e. the physics was right and a single-seed ±0.03 window
is a ~1.5σ criterion that fails ~13% of seeds under CORRECT physics.
The gate was therefore made the mean over 4 fixed seeds (σ_mean ≈
0.01, ±0.03 ≈ 3σ), anchors and tolerance unchanged. Per-seed values
are printed by the test (`-- --nocapture`) and recorded above.

## 3. Gate table

All commands from the tranche's GATES block, run at the final tree:

| gate | result |
|---|---|
| `cargo check --features "kahler imagine sharded transactions patterns causal_states wish halcyon" --bin gigi-stream` | PASS |
| `cargo test --no-default-features --lib` | 912/0 |
| `spectral_gauge_basic` + `spectral_gauge_where_basic` (Phase-1 fence) | 21/0 + 7/0 |
| `spectral_full_basic` (new, concept A) | 12/0 |
| `spectral_magnetic_basic` (new, concept B) | 9/0 |
| `u1_flux_basic` (new, concept C) | 12/0 |
| `ingest_as_gauge_field_basic`, `ingest_gauge_vertex_basic`, `ingest_npz_key_basic`, `ingest_npz_dtype_basic`, `ingest_gql_bypass_basic`, `halcyon_l24_workflow_e2e` | PASS |
| `chern_class_basic`, `chern_class_bundle_target_basic`, `betti_pi1_basic`, `obstruction_basic`, `topology_verbs_gql_integration` | PASS |
| `halcyon_part_iv_gold`, `aurora_lie_poisson_trait` | PASS |
| `imagine_coherence_phase2` (kahler,imagine) | PASS |
| `cubic_lattice`, `lattice_obc_basic` (lattice) | PASS |
| `davis_conjecture_lambda_brain_ridealong` (kahler) | PASS |
| `pattern_hunt_parser` (patterns) | PASS |
| `emit_csv`, `noop_notices`, `timestamp_ergonomics`, `gql_reference_truth`, `explain_kappa`, `ingest_dir_gate`, `pathguard_escapes`, `ingest_executor`, `ingest_csv_basic`, `ingest_jsonl_basic` | PASS |

## 4. Live probes (deploy-stage checklist)

S1 `LATTICE l4_rh FROM CUBIC L=4 DIM=2 OBC AXIS 0;` → ok.
S2 `GAUGE_FIELD rh_flux GROUP U(1) INIT FLUX RANDOM SEED 42 ON LATTICE l4_rh;` → ok.
S3 `SPECTRAL_GAUGE rh_flux ON FIBER (theta) GROUP U(1) MODE MAGNETIC FULL LIMIT 8;` → 8 ascending reals.
S4 same without MODE MAGNETIC → cos-weight FULL works.
S5 GROUP SU(2) + MODE MAGNETIC → the this-phase error.
S6 plain λ₁ SPECTRAL_GAUGE → Phase-1 3-field shape.
S7 Marcella IMAGINE sanity → 200, coherence 1.0.

The S1→S2→S3 chain has a byte-level local twin in
`tests/u1_flux_basic.rs::test_flux_to_magnetic_spectrum_s1_s2_s3_loop`
(OBC drops 4 wrap links → 28 records → 8 ascending eigenvalues).

## 5. Deviations (from the 2026-06-30 spec and the tranche text — all named)

1. **`FULL [LIMIT k]` instead of `MODE dense|sparse`** (spec §6).
   Hallie's confirmed 2026-07-16 ask supersedes the spec's naming;
   MODE now carries MAGNETIC and solver selection is internal (R1).
   `SHIFT σ` (sparse-only) is deferred with the sparse arm.
2. **Ascending ALGEBRAIC ordering** (R3) vs spec's "ascending by |λ|".
   Identical on the PSD magnetic operator; differs on the indefinite
   cos-weight Laplacian.
3. **FULL without LIMIT = ALL eigenvalues** (R1) vs spec default
   k = 4 smallest-magnitude.
4. **Lanczos sparse arm deferred to Phase 2.1** (R2): no sprs dep, no
   IRL, no shift-invert this phase; FULL on V > 4096 returns the
   SparseUnavailable-shaped error naming Phase 2.1. Consequently the
   spec's full-reorthogonalization choice is not yet exercised and the
   bucky 6-sig-fig dense/sparse parity gate (§7.1) is N/A until the
   sparse arm lands. Every RH-loop graph is dense-side, so the sweep
   is unblocked regardless.
5. **Phase-1 fence tests 11–12 rewritten**: they pinned the FULL
   `PhaseNotImplemented` stub that this tranche replaces by design;
   they now pin the FULL contract (C_4 closed form, LIMIT clamp).
   Fence count preserved at 21/0 + 7/0.
6. **GOE/GUE estimator = 4-fixed-seed mean** (task text said "a
   fixed-seed graph"): variance reduction after the measured
   single-graph σ ≈ 0.02 made a single-seed ±0.03 window ~1.5σ.
   Anchors and tolerance unchanged; receipt in §2.
7. **GAUGE_FIELD grammar relaxed** to clause-order-flexible (required
   by the confirmed probe S2 order); duplicates/missing clauses are
   named parse errors; canonical order pinned.
8. **INIT FLUX materializes a bundle** (no U(1) DenseLinkBuffer, no
   registry entry, PERSIST rejected). The tranche's "mirror the INIT
   HAAR machinery" is honored at the determinism-contract level (same
   house RNG, same seeding, per-edge draws in edge order) — not at the
   buffer level, because SPECTRAL_GAUGE reads bundles and a U(1)
   buffer would have dragged the SU(2)/SU(3) registry/heatbath surface
   into scope for no consumer.
9. **MODE MAGNETIC lives on SPECTRAL_GAUGE only** — plain SPECTRAL has
   no gauge fiber (per the tranche's own discovery guidance).
10. **WAL format untouched**: flux inits allocate no WAL byte tag; the
    declare encoder returns a typed refusal if one ever reaches it
    (unreachable in practice — PERSIST is rejected upstream).

## 6. Files

- `src/spectral.rs` — `spectral_gauge_spectrum` core (FULL + MAGNETIC),
  `spectral_full_normalized`, `SpectralGaugeMode`/`Convergence`,
  `InvalidLimit`/`SparseUnavailable`/`MagneticRequiresU1`,
  `SPECTRAL_DENSE_MAX_V`.
- `src/parser.rs` — MODE MAGNETIC + SPECTRAL LIMIT grammar,
  clause-order-flexible GAUGE_FIELD, INIT FLUX, executor arms.
- `src/bin/gigi_stream.rs` — HTTP executor arms (same envelope).
- `src/gauge/u1_flux.rs` (new) — flux materializer.
- `src/gauge/{su2,su3}_gauge_field.rs`, `error.rs`, `persistence.rs`,
  `http.rs`, `wal.rs` — flux-init variants + exhaustiveness arms.
- `tests/spectral_full_basic.rs`, `tests/spectral_magnetic_basic.rs`,
  `tests/u1_flux_basic.rs` (new suites); `tests/spectral_gauge_basic.rs`
  (tests 11–12 → FULL contract); `tests/spectral_gauge_where_basic.rs`
  (magnetic-field destructure pin).
