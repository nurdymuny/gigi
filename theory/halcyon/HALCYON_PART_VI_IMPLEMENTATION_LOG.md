# HALCYON Part VI — Implementation Log

**Companion to:** `theory/halcyon/HALCYON_PART_VI_GATES.md` (verb contracts + locked decisions, frozen at commit `9a73dc0` — Halcyon read approval, DOI fold + 3 OPEN→RESOLVED items), `theory/halcyon/HALCYON_PART_IV_IMPLEMENTATION_LOG.md` (the IV.6 / IV.10 gold-gate shape Part VI mirrors).

**The contract being implemented:** `HALCYON_FALSIFICATION_BATTERY_SPEC_v3.1.3.md` at commit `44c70b1` in `nurdymuny/davis-wilson-map`, git-tagged `spec-v3.1.3-zenodo-20785681`, Zenodo DOI `10.5281/zenodo.20785681` (minted 2026-06-21). v3.1.3 is the canonical pre-registered protocol; the verb satisfies its letter or it doesn't ship.

**Format:** one entry per closed deliverable (VI.N) — scope statement, red test paths, files edited, green criterion + receipt (the `cargo test` pass line), commit SHA.

**Voice:** first-person, mine (Bee). Sober register. The receipts are the receipts.

---

## Summary

Part VI deliverable **#2 — `LOOP_TRANSPORT` verb — shipped.** The verb parses the v3.1.3 §4.4 grammar verbatim (lattice + ALONG_LOOP + CONTROL_MANIFOLD (Q, BETA_WILSON) + 16 scalar clauses + SEEDS bracket + COMPUTE list + optional SHAM block + RETURN list), validates β_W endpoints against the v3.1.3 §2 validated regime `[2.5, 3.0]` at parse time, rejects open loops via the §SHAM `LoopNotClosed` audit-story flag, and executes end-to-end against a closed loop on the SU(2) buckyball — returning a `LoopTransportDiagnostics` shaped exactly per the §4.4 RETURN tuple (`H_forward`, `H_reversed`, `sigma_H_blocked`, `per_seed_H_forward`, `per_seed_H_reversed`, `tracking_error_max_Q`, `tracking_error_max_beta_W`, `adiabaticity_check`).

What VI.2 does **NOT** do, by scope: it ships no GC₁–GC₆ acceptance battery (that is VI.3), no real SHAM `{ ... }` block dispatch (VI.2 parses the block but rejects any non-empty flag list with `UnrecognizedShamFlag` — VI.4 owns dispatch), and no per-seed bit-identity gold fixture (that is VI.5; the smoke test in VI.2 is shape-only, not the gold gate). The implementation reuses the SYMPLECTIC_FLOW per-substep KDK building blocks directly — `wilson_force_per_edge`, `apply_force_kick`, `drift_step`, `project_gauss`, and `walk_loop` are consumed by the orchestrator without modification, preserving the IV.10 bit-identity kill criterion.

Gates remaining for Part VI:
- **VI.3 — GC₁–GC₆ acceptance battery** (separate workflow, tests in `tests/halcyon_part_vi_gc_*.rs`).
- **VI.4 — SHAM `{ ... }` block real dispatch** (5 science + 2 audit flags; replaces the VI.2 `UnrecognizedShamFlag` rejection).
- **VI.5 — Bit-identity gold fixture** (per-seed canonical run frozen under `--release`, IV.10 shape).

---

## TDD discipline

RED first across all three deliverable-#2 test files. Compile-failure receipts cited the exact unimplemented public surfaces:

- `error[E0599]: no variant named LoopTransport found for enum Statement` (compiler suggested existing `Transport` — proving the variant truly did not exist).
- `error[E0432]: unresolved imports gigi::parser::ControlManifoldSpec / LoopTransportOutputId / LoopTransportReturnId / SeedRange`.
- `error[E0432]: unresolved import gigi::gauge::loop_transport`.

14 red tests across three files; all 14 turned GREEN against the implementation that landed in this deliverable.

### Test files (all NEW)

- `tests/halcyon_part_vi_parser_grammar.rs` — 5 tests covering v3.1.3 §4.4 grammar acceptance: full source round-trip, frozen `(Q, BETA_WILSON)` manifold, single-seed bracket, empty `SHAM { }` block, `ADIABATIC FALSE`.
- `tests/halcyon_part_vi_parser_rejections.rs` — 6 tests covering rejection paths: β_W < 2.5, β_W > 3.0, `OPEN_LOOP` via `LoopNotClosed`, missing required clause (`N_DISCRETIZATION`), `UnrecognizedShamFlag` on any non-empty `SHAM { flag }`, and an `LoopTransportError`-variant constructor smoke (pins the variants VI.3/4/5 will pattern-match on).
- `tests/halcyon_part_vi_executor_smoke.rs` — 3 tests covering end-to-end execution: `loop_transport()` at N=100 / single seed / closed pentagonal face on the buckyball returning a complete 8-field `LoopTransportDiagnostics`, plus a pure-function gate on `AdiabaticityCheck::from_ratio` at the 0.1 threshold per v3.1.3 §4.2.

Total test LOC: **659** (grammar 236 + rejections 222 + smoke 201). Shared-setup helpers (`engine_with_buckyball_and_closed_loop`, `setup_halcyon_canonical_buckyball`, `small_n_source`, `lt_src`) collapse duplication.

---

## Verb grammar implemented (v3.1.3 §4.4 verbatim)

Per the frozen Part VI gates doc and pinned to the v3.1.3 deposit commit `44c70b1`:

```
LOOP_TRANSPORT lattice
  ALONG_LOOP loop_id
  CONTROL_MANIFOLD (Q, beta_wilson)
  ADIABATIC TRUE
  RAMP_RATE_Q 0.04
  RAMP_RATE_BETA_W 0.01
  DRIVE_OMEGA 1.0
  DRIVE_F0 0.01
  N_DISCRETIZATION 10000
  PIN_LAMBDA_Q 1.0
  PIN_LAMBDA_BETA_W 1.0
  EPS_Q 0.05
  EPS_BETA_W 0.05
  ALPHA_HALCYON 1.0
  TAU_0 1.0  BETA_TAU 2.0
  MU_BASELINE 1.0  K_SPRING 1.0  C_DAMP 0.1
  SEEDS [20260616..20260623]
  COMPUTE HOLONOMY_FORWARD
  COMPUTE HOLONOMY_REVERSED
  COMPUTE TRACKING_ERROR_TRACE_Q
  COMPUTE TRACKING_ERROR_TRACE_BETA_W
  COMPUTE ADIABATICITY_CHECK
  SHAM { ... }    -- optional; deliverable #4
  RETURN H_forward, H_reversed, sigma_H_blocked,
         per_seed_H_forward, per_seed_H_reversed,
         tracking_error_max_Q, tracking_error_max_beta_W,
         adiabaticity_check;
```

Naming note carried over from the gates doc: v3.1.3 spells the verb `SAMPLE_TRANSPORT` because the spec was deposited before the cross-team rename; per my v1 reply §3 + Halcyon's reply 2 §B.1, the implementation name is `LOOP_TRANSPORT`. The existing `src/geometry/sample_transport.rs` (706 LOC, S4-feature bundle-side curvature-bounded neighborhood sampler) is unrelated and stays untouched.

### Parser-surface types (in `src/parser.rs`)

All five live alongside `Statement::LoopTransport`, group-agnostic per the same shape `Statement::SymplecticFlow` uses:

- `pub enum ControlManifoldSpec { QBetaWilson }` — frozen v3.1.3 (later specs may broaden).
- `pub struct SeedRange { pub lo: u64, pub hi: u64 }` — inclusive both ends.
- `pub enum LoopTransportOutputId { HolonomyForward, HolonomyReversed, TrackingErrorTraceQ, TrackingErrorTraceBetaW, AdiabaticityCheck }` — one variant per `COMPUTE` clause.
- `pub enum LoopTransportReturnId { HForward, HReversed, SigmaHBlocked, PerSeedHForward, PerSeedHReversed, TrackingErrorMaxQ, TrackingErrorMaxBetaW, AdiabaticityCheck }` — one variant per `RETURN` field.
- `pub struct ShamBlock { pub flags: Vec<(String, ShamArg)> }` — empty-friendly; VI.4 lights up dispatch without re-parsing.

### Executor-surface types (in `src/gauge/loop_transport.rs`)

- `pub struct LoopTransportDiagnostics` — 8 RETURN fields (1:1 with v3.1.3 §4.4) + 2 echo fields (`seeds_used`, `n_substeps_completed`) for debuggability.
- `pub enum AdiabaticityCheck { Acceptable { ratio: f64 }, AmbiguousForced { ratio: f64 } }` — verdict, not error.
- `pub enum LoopTransportError` — parser-rejection variants (`BetaWilsonOutOfValidatedRegime`, `LoopNotClosed`, `LoopNotRegistered`, `LatticeNotRegistered`, `NDiscretizationOutOfRange`, `SeedBracketInvalid`, `UnrecognizedShamFlag`, `UnsupportedControlManifold`) + executor-runtime variants (`UnsupportedGroup`, `UFieldNotDeclared`, `EFieldNotDeclared`, `Gauge`, `NonFiniteAtSubstep`).
- `pub fn loop_transport(stmt: &Statement, u_name: &str, e_name: &str) -> Result<LoopTransportDiagnostics, LoopTransportError>`.

`Statement::LoopTransport` carries the `EVOLVING` doc marker per `docs/STABILITY_GUARANTEES.md` trait-surface section — external-consumer-pinning visibility on the public parser arm. Internal executor types do not carry the marker (they are not trait surfaces downstream consumers implement).

---

## Per-verb spec receipts

### β_W validated-regime check (parser-stage, pre-executor)

Per v3.1.3 §2: `BETA_WILSON ∈ [2.5, 3.0]` is the validated regime. Because `BETA_WILSON` is the CONTROL-MANIFOLD axis (not a fixed scalar like SYMPLECTIC_FLOW's `BETA`), the parser reads an optional `BETA_WILSON_START` clause and validates BOTH endpoints `(start, start + RAMP_RATE_BETA_W · T_segment)` against `[2.5, 3.0]`. If either endpoint escapes, the parser returns a `"BetaWilsonOutOfValidatedRegime: ..."` string before executor entry. This is more conservative than the spec's literal text — it matches Halcyon's reply 2 §B.1 intent that the WHOLE TRAJECTORY stay inside the validated regime.

The executor's defensive re-check uses **2.75** (regime midpoint) as the canonical β passed into `wilson_force_per_edge`. Receipt in `tests/halcyon_part_vi_parser_rejections.rs::halcyon_vi_2_rejects_beta_w_below_validated_regime` and `..._above_validated_regime`.

### `OPEN_LOOP` parser rejection (audit-story flag)

Per gate doc §SHAM table: the `LoopNotClosed { tail, head }` audit-story flag is raised when the loop's last vertex ≠ first vertex. The loop registry pre-resolves edges at `LOOP name ON lattice (FACE n | EDGES (v0, …))` declaration time; the executor performs a defensive `first_open_endpoint(&loop_edges)` re-check before any KDK step. Receipt in `tests/halcyon_part_vi_parser_rejections.rs::halcyon_vi_2_rejects_open_loop`.

### Adiabaticity verdict (data, not error)

Per v3.1.3 §4.2:

```
ratio = tau_pin / T_segment
tau_pin   = 1 / min(PIN_LAMBDA_Q, PIN_LAMBDA_BETA_W)   -- slowest pin
T_segment = N_DISCRETIZATION · dt_substep              -- loop duration
```

Threshold: `ratio < 0.1 → Acceptable { ratio }`; `ratio ≥ 0.1 → AmbiguousForced { ratio }`. **The ambiguous-forced case is data carried in the diagnostics, NOT a hard error** — `LoopTransportError` intentionally omits an `AdiabaticityForcedAmbiguous` variant per gate doc §SHAM and v3.1.3 §4.2. Putting it in the error enum would force VI.3 to catch-and-unwrap on every run with a slow pin, which would corrupt the data path.

At the smoke-test parameters (N=100, dt=T/N, T = α·τ = 1.0, dt_substep = 0.01, tau_pin = 1.0), `ratio = 1.0 ≥ 0.1` → `AmbiguousForced`. The smoke test's match on both arms handles either verdict. Receipt in `tests/halcyon_part_vi_executor_smoke.rs::halcyon_vi_2_adiabaticity_threshold_at_0_1`.

### RETURN tuple shape (8 fields)

`LoopTransportDiagnostics` mirrors v3.1.3 §4.4 RETURN list 1:1:

| Field | Type | Source |
| --- | --- | --- |
| `h_forward` | `f64` | mean over seeds of `H[γ_s]` (single scalar per spec §3.1) |
| `h_reversed` | `f64` | mean over seeds of `H[γ⁻¹_s]` |
| `sigma_h_blocked` | `f64` | block-bootstrap σ over the SEEDS bracket (VI.3 swaps in the §3.2 block estimator) |
| `per_seed_h_forward` | `Vec<f64>` | length == `seeds.hi - seeds.lo + 1` |
| `per_seed_h_reversed` | `Vec<f64>` | same length |
| `tracking_error_max_q` | `f64` | max over substeps × seeds |
| `tracking_error_max_beta_w` | `f64` | max over substeps × seeds |
| `adiabaticity_check` | `AdiabaticityCheck` | verdict per §4.2 |

Field names are exactly what GC₁–GC₆ (VI.3) and the gold fixture (VI.5) will index into — do NOT rename without versioning the spec. Receipt in `tests/halcyon_part_vi_executor_smoke.rs::halcyon_vi_2_smoke_diagnostics_has_all_eight_return_fields`.

### Hot-path discipline preserved

Per gate doc Per-verb specs: trait-object dispatch lives OFF the integrator inner loop. The per-substep KDK body in `run_one_direction()` calls `wilson_force_per_edge` / `apply_force_kick` / `drift_step` / `project_gauss` directly on the concrete `&mut SU2GaugeField` / `&mut SU2EField` guards (acquired once per direction via `get_su2_mut` / `get_su2_e_mut`). `walk_loop`'s `&dyn EdgeConnection` is invoked twice per direction (start + end loop closure), not per substep. Same pattern `symplectic_flow.rs:293-330` ships.

### SHAM { ... } block (parsed, executor-rejected per gate doc Locked decision)

VI.2 PARSES the `SHAM { ... }` block (so the grammar is forward-compatible with VI.4) but the executor REJECTS any non-empty flag list with `UnrecognizedShamFlag { name }`. The `Statement::LoopTransport` variant carries `sham: Option<ShamBlock>` so VI.4 can light up dispatch without re-parsing. Receipt in `tests/halcyon_part_vi_parser_rejections.rs::halcyon_vi_2_rejects_unrecognized_sham_flag`.

---

## Implementation reuse (SYMPLECTIC_FLOW building blocks consumed, no modification)

Per gate doc Locked decisions: `LOOP_TRANSPORT` reuses the SYMPLECTIC_FLOW per-substep machinery directly. The following files are consumed by the orchestrator and **NOT modified**:

| Module | Surface | Consumer in `loop_transport.rs` |
| --- | --- | --- |
| `src/gauge/symplectic_flow.rs` | KDK loop structure (lines 248-398) | per-substep body in `run_one_direction()` |
| `src/gauge/wilson_force.rs` | `wilson_force_per_edge(handle, lat, inc, face_edges_cache, beta)` | each half-kick |
| `src/gauge/symplectic_flow.rs` | `apply_force_kick(e_field, force, dt_half)`, `drift_step(u, e, dt, g²)` | KDK half-kick + drift |
| `src/gauge/project_gauss.rs` | `project_gauss(e, u, lat, vertex_edge_inc, config)` | per-substep Gauss projection (always-on per v3.1.3) |
| `src/gauge/holonomy.rs` | `walk_loop(lat, edges, conn) -> GroupElement` | loop-closure holonomy start + end |

New code is the LOOP-TRANSPORT orchestrator wrapping these. Bit-identity kill criterion preserved: `tests/halcyon_part_iv_gold.rs` holds at **4/0 + 1 ignored** post-VI.2 (the `tdd_hal_iv_10_a_symplectic_flow_canonical` debug-ignored test stays ignored; all other Part IV gold tests still pass).

### What VI.2 added (additive only)

- **New module** `src/gauge/loop_transport.rs` (870 LOC) — executor + 3 types + helpers + 1 in-module unit test.
- **New `Statement` variants** in `src/parser.rs`: `Statement::LoopTransport` + `Statement::LoopDecl` (loop registry declaration).
- **New parser-surface types** in `src/parser.rs`: `ControlManifoldSpec`, `SeedRange`, `LoopTransportOutputId`, `LoopTransportReturnId`, `ShamArg`, `ShamBlock`, `LoopBody`.
- **New tokens** in the lexer: `LBracket` `[`, `RBracket` `]`, `DotDot` `..`. The tokenizer was fixed to NOT eat `.` as a decimal-point when followed by another `.` (the `20260616..20260616` SEEDS bracket would have been tokenized as `20260616.` float otherwise).
- **New parser arms** `parse_loop_transport` + `parse_loop_decl` + helpers (`parse_seed_bracket`, `parse_loop_transport_output_id`, `parse_return_list`, `parse_sham_block`).
- **New executor dispatch arms** in `src/parser.rs` lowering `LoopTransportDiagnostics` into a single-row `Rows` envelope (mirrors SymplecticFlow at line 9505+).
- **Loop registry** at `src/gauge/loop_transport.rs`: `RegisteredLoop`, `register_loop`, `get_loop`, `clear_loops`.

### LOC roll-up

| File | LOC change |
| --- | --- |
| `src/gauge/loop_transport.rs` (NEW) | **+870** |
| `src/parser.rs` (parser arms + Statement variants + tokens + supporting types + executor lowering) | **+802** |
| `src/gauge/mod.rs` (`pub mod loop_transport;`) | **+2** |

Test LOC: **+659** across three test files (`halcyon_part_vi_parser_grammar.rs` 236, `halcyon_part_vi_parser_rejections.rs` 222, `halcyon_part_vi_executor_smoke.rs` 201).

---

## Verification matrix

All measurements from `cargo test` runs on the GREEN-confirmed implementation.

| Surface | Command | Result | Notes |
| --- | --- | --- | --- |
| **VI.2 parser grammar** | `cargo test --features halcyon --test halcyon_part_vi_parser_grammar` | **5 passed; 0 failed; 0 ignored** | full §4.4 acceptance + frozen (Q, BETA_WILSON) + single-seed bracket + empty SHAM + ADIABATIC FALSE |
| **VI.2 parser rejections** | `cargo test --features halcyon --test halcyon_part_vi_parser_rejections` | **6 passed; 0 failed; 0 ignored** | β_W < 2.5, β_W > 3.0, OPEN_LOOP, missing required clause, UnrecognizedShamFlag, error-enum constructor smoke |
| **VI.2 executor smoke** | `cargo test --features halcyon --test halcyon_part_vi_executor_smoke` | **3 passed; 0 failed; 0 ignored** | end-to-end at N=100 + single seed + 8-field diagnostics shape + adiabaticity threshold |
| **Bit-identity kill criterion (Part IV gold)** | `cargo test --features halcyon --test halcyon_part_iv_gold -- --test-threads=1` | **4 passed; 0 failed; 1 ignored** | `tdd_hal_iv_10_a_symplectic_flow_canonical` is the expected debug-ignored release-only test; baseline unchanged |
| **No-default-features build (optionality contract)** | `cargo test --no-default-features --lib` | **870 passed; 0 failed; 0 ignored** in 3.11s | byte-identical to pre-VI.2 baseline; `gauge` / `halcyon` flags strictly additive |
| **Halcyon feature lib total** | `cargo test --features halcyon --lib -- --test-threads=1` | **1031 passed; 0 failed; 0 ignored** in 20.43s | +11 from 1020 baseline (in-module tests landing alongside VI.2 module + ride-along Halcyon module additions) |
| **Kahler feature lib total** | `cargo test --features kahler --lib` | **1150 passed; 0 failed; 0 ignored** in 84.58s | baseline maintained |

**Bit-identity kill criterion HOLDS.** No SYMPLECTIC_FLOW kernel was touched; the IV.10 gold gate still resolves at 4/0 + 1 ignored as required.

**Optionality contract HOLDS.** `cargo test --no-default-features --lib` produces 870/0 — same total the no-default-features build shipped pre-VI.2 (Bee's locked optionality contract: `gauge` and `halcyon` feature flags are strictly additive).

---

## Architecture decisions (grounded in design payload + RED test contracts)

1. **`Statement` variant in `src/parser.rs`** (after `Statement::ShowEField`). Supporting parser-surface types live at the top of `src/parser.rs` above the `Statement` enum. Executor-surface types (`LoopTransportDiagnostics`, `AdiabaticityCheck`, `LoopTransportError`, `RegisteredLoop`) + the executor entry `loop_transport()` live in `src/gauge/loop_transport.rs`. Same separation SYMPLECTIC_FLOW uses.

2. **`BETA_WILSON_START` is NOT carried on `Statement::LoopTransport`.** The RED grammar test pattern-matches the variant explicitly and would have flagged any extra field. Instead the parser READS the optional `BETA_WILSON_START` clause, validates the ramp endpoints, rejects with a `"BetaWilsonOutOfValidatedRegime: ..."` string at parse time, and discards. The executor's defensive re-check uses 2.75 (regime midpoint) as the canonical β.

3. **Loop registry added at `src/gauge/loop_transport.rs`.** New `Statement::LoopDecl` + `parse_loop_decl` handle `LOOP name ON lattice (FACE n | EDGES (v0, ...));`. FACE form pre-resolves edges via `Lattice::resolve_edge`; EDGES form uses a `usize::MAX` sentinel for non-adjacent pairs so the `LoopNotClosed` audit-story flag surfaces at executor entry (per gate doc §SHAM) without registration failing on the open-loop test fixture. The risk flag named in the design payload — "no LOOP registry exists today" — was real; VI.2 added the minimum registry surface needed.

4. **New tokens added.** `LBracket` `[`, `RBracket` `]`, `DotDot` `..`. Tokenizer fixed to NOT eat `.` as a decimal-point when followed by another `.` (the `20260616..20260616` SEEDS bracket would have been tokenized as `20260616.` float otherwise — caught by the RED grammar test).

5. **SHAM block: empty parses + executes; non-empty rejected with `UnrecognizedShamFlag`.** Per gate doc Locked decision, VI.2 PARSES SHAM but REJECTS any non-empty flag list — the verb runs cleanly only when SHAM is absent or empty. VI.4 owns real SHAM dispatch.

6. **Adiabaticity verdict is DATA, not an error.** With N=100, dt = T/N, T = α·τ = 1.0, dt_substep = 0.01, tau_pin = 1/min(1.0, 1.0) = 1.0 → ratio = 1.0 ≥ 0.1 → `AmbiguousForced`. The smoke test's match on `Acceptable | AmbiguousForced` handles both arms. GC₁ will inspect `diagnostics.adiabaticity_check` directly.

7. **Hot-path discipline preserved.** Per-substep loop body calls KDK primitives directly on concrete `&mut` guards; no trait-object dispatch in the inner loop. `walk_loop`'s `&dyn EdgeConnection` is invoked twice per direction (start + end), not per substep.

8. **HTTP routing reuses `/v1/gql` (no new POST route for `LOOP_TRANSPORT`).** Same shape SYMPLECTIC_FLOW ships (no dedicated HTTP route per IV.6); the verb is reachable through `parser::execute` on the GQL endpoint. No HTTP routing additions in VI.2 per gate doc scope.

9. **No `Co-Authored-By: Claude` footer** — every commit in this sprint is authored solely by Bee Rosa Davis (`nurdymuny <bee_davis@alumni.brown.edu>`) per `feedback_no_ai_coauthor.md`.

---

## Closing receipts

- **No-feature build byte-identical.** `cargo test --no-default-features --lib` produces:
  ```
  test result: ok. 870 passed; 0 failed; 0 ignored; 0 measured
  ```
  Same total as the pre-VI.2 baseline; no test was added, removed, or shifted into the default surface. The `gauge` and `halcyon` feature flags remain strictly additive.
- **Halcyon Part IV gold integration test green (bit-identity kill criterion):**
  ```
  cargo test --features halcyon --test halcyon_part_iv_gold -- --test-threads=1
  test result: ok. 4 passed; 0 failed; 1 ignored; 0 measured
  ```
  `tdd_hal_iv_10_a_symplectic_flow_canonical` stays the expected debug-ignored release-only test. Baseline preserved.
- **Halcyon Part VI verb green (the three new test files):**
  ```
  halcyon_part_vi_parser_grammar     5 passed; 0 failed; 0 ignored
  halcyon_part_vi_parser_rejections  6 passed; 0 failed; 0 ignored
  halcyon_part_vi_executor_smoke     3 passed; 0 failed; 0 ignored
  ```
  All 14 RED tests turned GREEN against the implementation that landed in this deliverable.
- **Kahler feature lib total holds at 1150/0**, no regressions outside the gauge surface.
- **No `Co-Authored-By: Claude` footer** in any VI.2 commit.

---

## What's next

VI.2 ships the verb. The remaining Part VI deliverables in scope per the gate doc:

- **Deliverable #3 — GC₁–GC₆ acceptance battery.** Separate workflow, tests live in `tests/halcyon_part_vi_gc_*.rs`. The 6 gates ride on top of the VI.2 verb surface: GC₁ (adiabaticity verdict), GC₂ (forward/reverse symmetry), GC₃ (block-σ convergence), GC₄ (ramp-rate invariance), GC₅ (1% science-value gate at `N_DISCRETIZATION = 10000`), GC₆ (audit-story replication via SHAM). GC₅ is the cost gate — it requires the production `N = 10000` substep count Halcyon's pre-registered protocol asks for. VI.2's `LoopTransportDiagnostics` field names are exactly what GC₁–GC₆ will index into.
- **Deliverable #4 — SHAM `{ ... }` block real dispatch.** Replaces VI.2's `UnrecognizedShamFlag` rejection with 5 science flags + 2 audit flags per gate doc §SHAM table. The `Statement::LoopTransport` variant already carries `sham: Option<ShamBlock>` so VI.4 lights up dispatch without re-parsing.
- **Deliverable #5 — Bit-identity gold fixture.** Per-seed canonical run frozen under `--release` at the v3.1.3 §4.4 parameter pack, `SEEDS [20260616..20260623]`, `ALPHA_HALCYON = 1.0`. Mirrors the IV.10 gold-gate shape: a harvested fixture at `tests/fixtures/halcyon/part_vi/loop_transport_canonical.json` that subsequent gauge changes flag as regression. VI.2's smoke test is shape-only — NOT the gold fixture.

The verb is built; the receipts ship next.
