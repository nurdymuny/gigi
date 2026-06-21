# HALCYON Part VI — Gates

**Companion to:** `theory/halcyon/HALCYON_PART_I_GATES.md`, `theory/halcyon/HALCYON_PART_IV_GATES.md` (the IV.6 gold-gate shape Part VI mirrors), `theory/halcyon/GIGI_TO_HALCYON_2026-06-20_SAMPLE_TRANSPORT_REPLY.md` (v1 scope review, commit `302ce1a`), and `theory/halcyon/GIGI_TO_HALCYON_2026-06-21_SAMPLE_TRANSPORT_REPLY_2.md` (v2 against v3.1.3, commit `baac7f2`).

**The contract being implemented:** `HALCYON_FALSIFICATION_BATTERY_SPEC_v3.1.3.md` at commit `44c70b1` in `nurdymuny/davis-wilson-map` (Zenodo DOI imminent). v3.1.3 is the canonical pre-registered protocol; this gate doc says exactly what gigi must ship to satisfy it.

**Voice:** first-person, mine (Bee). Sober register. I spec the algorithm here, not the prose around it.

This document fixes the verb contracts and the locked decisions I'm carrying into the Part VI sprint. The Sprint B revert lesson says gates before code — this lands before any LOC of `LOOP_TRANSPORT` does. Receipts (per-gate red/green, commit SHAs, test counts) will live in `HALCYON_PART_VI_IMPLEMENTATION_LOG.md` after the work ships.

---

## Part VI pass criterion

`LOOP_TRANSPORT` satisfies the v3.1.3 §4.4 grammar verbatim, returns the §4.4 RETURN tuple, exposes the §5 sham flags as a nested `SHAM { ... }` block, and passes the §7.4 verb-acceptance battery `GC₁–GC₆` green — including GC₅'s 1% science-value gate at `N_discretization = 10000` (the convergence bracket required by Halcyon's pre-registered protocol).

Bit-identity per-seed mirrors the IV.6 gold-gate shape: the canonical `LOOP_TRANSPORT halcyon_canonical_buckyball` call at the §4.4 parameter pack, `SEEDS [20260616..20260623]`, `ALPHA_HALCYON = 1.0` reproduces a harvested gold fixture at `tests/fixtures/halcyon/part_vi/loop_transport_canonical.json` byte-for-byte under `--release`. The fixture is captured at the Part VI ship commit; any subsequent change to gauge code that perturbs the per-seed `H_forward` / `H_reversed` values flags as a regression, same shape as IV.10.

Halcyon's `run_holonomy_battery.py` orchestrator (per v3.1.3 §4.6) makes one substrate call per loop direction, computes `H_geom = ½(H_forward − H_reversed)` and `H_sys = ½(H_forward + H_reversed)` in Python, and applies the §3 gate thresholds. The substrate does the substrate-side computation; Halcyon does the protocol-side judgment. The §7.1 two-layer audit story stands.

---

## Per-verb specs

### `LOOP_TRANSPORT lattice ALONG_LOOP loop_id CONTROL_MANIFOLD (Q, beta_wilson) ADIABATIC TRUE ...` (Gate VI.1, parser-lift Gate VI.7)

The v3.1.3 §4.4 grammar verbatim. v3.1.3 spelled it `SAMPLE_TRANSPORT` because the spec was deposited before the cross-team rename; per my v1 reply §3 + Halcyon's reply 2 §B.1, the implementation name is `LOOP_TRANSPORT` (the existing `src/geometry/sample_transport.rs` is the unrelated bundle-side curvature-bounded neighborhood sampler at S4-feature, kept). The grammar is otherwise frozen at the v3.1.3 commit hash:

```
LOOP_TRANSPORT halcyon_canonical_buckyball
  ALONG_LOOP gamma_unit_in_Q_beta_W
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
  SHAM { ... }    -- optional; see §SHAM block below
  RETURN H_forward, H_reversed, sigma_H_blocked,
         per_seed_H_forward, per_seed_H_reversed,
         tracking_error_max_Q, tracking_error_max_beta_W,
         adiabaticity_check;
```

**Parser arm:** `Statement::LoopTransport` with explicit fields for every clause. Default-`None` for clauses left out; defaults for numeric parameters baked into the parser arm (the v3.1.3 §4.4 values are the defaults the spec pre-registers — overriding any of them turns this from a science call into something else, so the parser permits overrides but the GC₅ acceptance test gates against the canonical values).

**β_W validation:** the parser arm validates `BETA_WILSON ∈ [2.5, 3.0]` strictly per v3.1.3 §2 + my v2 reply §6. β = 2.5 inherits bit-identity from the locked Halcyon canonical thermalization β (Sprint A gold). Extension below β = 2.5 needs independent SU(2) Q-tracking validation first per v3.1.3 §2; the parser refuses out-of-range values with `LoopTransportError::BetaWilsonOutOfValidatedRegime { got, range: (2.5, 3.0) }`.

**Hot-path discipline (per v2 reply §6, inherited from AURORA CC-1):** trait-object dispatch (Hamiltonian factory, observable visitor) lives off the integrator inner loop. The per-substep KDK + measurement body is generic over a concrete `Hamiltonian: HamiltonianForce + HamiltonianDrift` and does not pay the v-table cost per substep. v3.1.3's `N_DISCRETIZATION = 10000` × 8 seeds × 2 directions = 160,000 substeps per science call; the loop must not allocate per-step.

**Return tuple shape:** the eight RETURN fields in v3.1.3 §4.4 map to a `LoopTransportDiagnostics` struct with `Vec<f64>` per-seed slices for `per_seed_H_forward` and `per_seed_H_reversed`, scalar `f64` for the means + tracking-error maxima + `sigma_H_blocked`, and a typed `AdiabaticityCheck` envelope (numerical `tau_pin_over_T_segment` plus a `verdict: Acceptable | AmbiguousForced`) per v3.1.3 §4.2's substrate-gated rule.

**HTTP routing:** the verb is HTTP-callable at `POST /v1/gql` via the existing parser dispatch. No dedicated `/v1/loop_transport` route at Gate VI ship; the GQL surface is the production interface Halcyon's orchestrator will use.

**Executor location:** `src/gauge/loop_transport.rs` (NEW); parser arm at `src/parser.rs` next to `SYMPLECTIC_FLOW` (the existing peer at `src/gauge/symplectic_flow.rs`, which `LOOP_TRANSPORT` reuses internally — see Locked decisions below).

---

## GC₁–GC₆ acceptance battery (Gate VI.2, all six green before any v3.1.3 science call)

Lifted verbatim from v3.1.3 §7.4 (which was Halcyon's §7.4 ask in their original letter). The substrate must pass these six contracts as `cargo test --features halcyon` integration tests at the verb's introduction. The existing 1373-assertion gigi test suite is necessary but not sufficient — GC₁–GC₆ are the **new** contracts the new verb introduces.

| # | Contract | Test |
|---|---|---|
| **GC₁** | **Flat connection returns zero.** | Construct a known-flat connection (`A ≡ 0` in synthetic mode); verify `H[any loop] = 0` to machine ε across at least 4 loop shapes. Test fixtures: γ_unit, γ_reversed, γ_small_area, γ_degenerate. |
| **GC₂** | **Known area law for an Abelian constant-curvature connection.** | Construct a connection with constant curvature `F₀` in `(Q, β_W)`; verify `H[γ] = F₀ · Area(γ)` to 1% across 3 loop sizes (small / unit / large). |
| **GC₃** | **Reversed loop inverts/sign-flips.** | For an arbitrary non-trivial connection, verify `H[γ⁻¹] = −H[γ]` (Abelian) or `H[γ]⁻¹` (non-Abelian) to 1% across at least 3 connections. This is the algebraic identity behind `H_geom = ½(H_forward − H_reversed)`. |
| **GC₄** | **Zero-size loop returns zero.** | Construct a degenerate loop bounding zero area (`γ_degenerate` per §5 S₅); verify `H = 0` to machine ε. |
| **GC₅** | **Discretization convergence + 1% science-value gate (v3.1.3 patch).** | Compute H at `N_discretization ∈ {1000, 2000, 4000, 8000, 16000}`; verify monotone convergence with relative change `< 1%` between 8000 and 16000 substeps. The v3.1.3 science call uses `N = 10000`, which lies inside this convergence bracket. **The substrate does not negotiate the 1% threshold.** If the verb cannot meet it, the verb is patched or `N_DISCRETIZATION` is moved by a v3.1.x amendment from Halcyon; the threshold doesn't move. |
| **GC₆** | **Gauge invariance.** | Apply a known gauge transformation to the substrate's connection; verify H is invariant to machine ε. The transformation surface uses the existing `GAUGE_FIELD` apply-gauge primitive from Part III; the test loop is γ_unit at C=4 (a non-trivial but small case). |

GC₁ + GC₃ + GC₄ + GC₆ verify to **machine ε** (no tolerance knob, `< 1e-14` typical). GC₂ verifies to **1%** (the area-law approximation accumulates discretization error). GC₅ verifies to **1% relative** between 8000-substep and 16000-substep runs (the convergence bracket itself).

A single GC failure blocks the v3.1.3 science calls. Substrate-side fix lands as a follow-up commit; the gate has to be green before Halcyon's `run_holonomy_battery.py` flips from mock to live client.

---

## SHAM block (Gate VI.3)

v3.1.3 §5 specifies five science-gate sham controls (with S₄ folded into the antisymmetric observable, not exposed as a flag). The gate doc names the four flag-bearing science shams + the §5 S₆ frozen field, totalling five flag-bearing controls inside the `SHAM { ... }` block.

| Sham flag | Implementation (verb-side) | v3.1.3 §5 gate |
|---|---|---|
| `FLAT_FIELD` | `SHAM_FLAT_FIELD = true` → `κ_Q ≡ 0` on all edges, all times | S₁: `|H_S₁| < 2σ_S₁` AND `|H_S₁| < 10⁻¹⁰` |
| `ALPHA_ZERO` | `ALPHA_HALCYON = 0` (overrides the call's value) | S₂: `|H_S₂| < 10⁻¹⁰` (load-bearing); 2σ check is sanity |
| `MASS_BASELINE_SCALED` | `MU_BASELINE ∈ {0.1, 1.0, 10.0}`; substrate fits baseline-subtracted H | S₃: POSITIVE branch — baseline-subtracted H invariant within 10%. NULL/AMBIGUOUS branches — `|H_S₃ at μ=1| < 2σ_S₃` AND `< 10⁻¹⁰` |
| `DEGENERATE_LOOP` | substitutes `γ_unit` with a zero-area loop in Λ | S₅: `|H_S₅| < 2σ_S₅` AND `|H_S₅| < 10⁻¹⁰` |
| `FROZEN_FIELD` | `SHAM_FROZEN_FIELD = true` → gauge field static across all substeps | S₆: `|H_S₆| < 2σ_S₆` AND `|H_S₆| < 10⁻¹⁰` |

S₄ (reversed-loop sign-flip) is **not a sham flag** — it's folded into the primary observable: `H_geom = ½(H[γ] − H[γ⁻¹])` is built into the verb's return tuple (`per_seed_H_forward` + `per_seed_H_reversed`) and Halcyon's orchestrator computes `H_geom` Python-side from those.

**Audit-story shams (OPEN — Halcyon to enumerate).** Halcyon's `PENDING_FROM_GIGI.md` references "5 science-gate + 2 audit-story" sham flags = 7 total. I count 5 science (the table above) but the 2 audit-story shams aren't enumerated in v3.1.3 §5 or §7 as I read them. Two possibilities:

1. They're implementation diagnostics that confirm the substrate isn't lying about its internals (per-substep state dump, deterministic-replay assertion, etc.). These wouldn't gate verdict but would gate auditor confidence.
2. They're in a v3.1.3 section I haven't found yet, or they're elaborated in a Halcyon-side companion doc not yet sent.

Resolution: Halcyon writes back with the 2 audit-story shams enumerated. They land in this gate doc as a follow-up Edit before the SHAM block implementation begins. Sprint B discipline says don't guess; this stays OPEN.

**SHAM block grammar.** The `SHAM { ... }` block is nested inside the `LOOP_TRANSPORT` clause list. Within the block, each flag is a `KEY = VALUE` line where keys are the flag names above; the parser arm validates against a closed enum of recognized flags and errors on unknown keys. Multiple flags in one block compose; the verb runs once per sham-flag combination Halcyon's orchestrator requests.

---

## Bit-identity contract per-seed (Gate VI.6, mirrors IV.6 gold-gate)

The Sprint B revert lesson + the v3.1.3 pre-registration discipline both require per-seed reproducibility. The contract:

1. **Gold fixture format.** `tests/fixtures/halcyon/part_vi/loop_transport_canonical.json` (NEW). Captures `per_seed_H_forward` + `per_seed_H_reversed` (each a `[f64; 8]` over the canonical seed range), the four scalar diagnostics (`H_forward_mean`, `H_reversed_mean`, `sigma_H_blocked`, `adiabaticity_check.tau_pin_over_T_segment`), and the SHA-256 of the v3.1.3 SPEC at execution time. Format follows v3.1.3 §7.2's `section_12_holonomy_battery_v3_1_3` sidecar shape.
2. **Capture mechanism.** Fixture is harvested at the Part VI ship commit by running `LOOP_TRANSPORT halcyon_canonical_buckyball` with the §4.4 parameter pack at `--release`. The fixture is then committed to the repo; the test asserts byte-for-byte match on every subsequent run.
3. **Acceptance arm (VI-F (a), debug-safe):** without `--release`, the test verifies the diagnostics are within `1e-10` of the gold fixture values, the adiabaticity verdict matches, and the GC₁–GC₆ green status is preserved. f64 reassociation differences between debug and release are tolerated within this bound (same shape as IV-F (a)).
4. **Regression arm (VI-F (b), release-only):** under `--release`, the per-seed values match byte-for-byte. Any change to gauge code, RNG path, or KDK/measurement order that perturbs the values flags as a regression. Path through `LOOP_TRANSPORT` is gated by the same `cargo test --features halcyon --release` discipline as IV.10.
5. **What this catches:** the kind of "passes the algebraic GC contracts but drifts the per-seed numerical outputs" failure mode that Sprint B taught us costs more than the perf win is worth. The IV.6 + IV.10 gold-gate shape is the proven defense.

CG iteration count + per-substep timing are **DIAGNOSTIC ONLY**, never compared across runs (same exclusion as IV.10).

---

## Locked decisions inherited from prior Halcyon work + the v3.1.3 chain

**Pre-registration anchor:** `HALCYON_FALSIFICATION_BATTERY_SPEC_v3.1.3.md` at commit `44c70b1` is the canonical contract. The five-round review chain (Gigi's methodological intervention establishing pre-registration discipline + four pre-deposit technical reviews catching mathematical / executability / validity-window / audit-tightness defects) is what makes v3.1.3 the deposit. I do not negotiate against the deposit; the verb satisfies the deposit's letter or it doesn't ship.

**Verb name:** `LOOP_TRANSPORT` (post-rename per my v1 reply §3 + Halcyon's reply 2 §B.1). The v3.1.3 spec spelled it `SAMPLE_TRANSPORT`; the rename resolves the name collision with the existing `src/geometry/sample_transport.rs` and was accepted cross-team before deposit. The spec's wording stays frozen at deposit; the implementation uses the agreed name.

**Implementation reuse:** `LOOP_TRANSPORT` shares the SYMPLECTIC_FLOW KDK leapfrog skeleton + the existing wilson_force_per_edge + drift_step + project_gauss building blocks (`src/gauge/symplectic_flow.rs`, `src/gauge/project_gauss.rs`, `src/gauge/staple_sum.rs`). The new code is the loop-transport orchestrator + parameter-space ramp logic + SHAM dispatch + holonomy-at-loop-closure measurement. Reuse keeps the per-substep loop body bit-identical to Halcyon's locked Part IV thermalization where it can be; the new code is in clearly-named auxiliary functions.

**Holonomy primitive:** `LOOP_TRANSPORT`'s holonomy-at-loop-closure uses `walk_loop()` from `src/gauge/holonomy.rs` (the existing read-only helper). The accumulated transport along the parameter-space loop discretization composes face holonomies through `walk_loop` at loop closure.

**β_W range:** validated to `[2.5, 3.0]` strictly per v3.1.3 §2 + v2 reply §6. β = 2.5 is the Halcyon canonical thermalization β (Sprint A gold value, `ea7b934c…` canonical SHA). Bit-identity contracts inherited.

**Adiabaticity check:** substrate-gated per v3.1.3 §4.2. Violation forces `verdict = AmbiguousForced` regardless of H values. The numerical `tau_pin / T_segment` ratio rides along in the return tuple. The Halcyon orchestrator's `apply_v3_1_3_gates(...)` reads the verdict and applies §3's AMBIGUOUS branch.

**CC questions from prior reply chain:**
- CC-LT-1 verb dispatch (closed enum vs open registry) — **still pending Halcyon's read**. Either resolves; closed-enum is simpler. Default to closed-enum if no preference arrives by VI.2 GREEN.
- CC-LT-2 adiabaticity-threshold source — **resolved** by v3.1.3 §4.2 substrate-gated ADIABATICITY_CHECK (theory-derived, not operator-tunable).
- CC-LT-3 sham namespacing — **superseded** by the S vs GC two-surface frame above.

---

## VI bit-identity matrix (the gates I will hold this against)

| Row | Contract | Source | Status before VI |
|---|---|---|---|
| **VI.A1** | Halcyon Part IV.10 gold values (canonical SYMPLECTIC_FLOW at β=2.5, dt=0.02, N_STEPS=1000, SEED=20260616) byte-for-byte under `--release` | tests/fixtures/halcyon/part_iv/symplectic_flow_canonical.json | Green at HEAD; VI shall not perturb |
| **VI.A2** | Halcyon Part III.8b face holonomy contract (face holonomies through `walk_loop` byte-for-byte) | tests/halcyon_part_iii_*.rs | Green at HEAD; VI shall not perturb |
| **VI.A3** | Halcyon Part V.* snapshot contracts (SNAPSHOT verb + WAL replay determinism) | tests/halcyon_part_v_*.rs | Green at HEAD; VI shall not perturb |
| **VI.B1** | LOOP_TRANSPORT per-seed gold fixture byte-for-byte under `--release` | tests/fixtures/halcyon/part_vi/loop_transport_canonical.json (NEW at VI ship) | Captured at VI ship; locked thereafter |
| **VI.B2** | GC₁–GC₆ green at every commit touching gauge code | tests/halcyon_part_vi_gc_acceptance.rs (NEW) | Captured at VI ship; locked thereafter |
| **VI.B3** | SHAM block 5 science flags pass their §5 gates on the canonical buckyball at β=2.5 | tests/halcyon_part_vi_sham_*.rs (NEW) | Captured at VI ship; locked thereafter |

Row VI.A* are the prior contracts Part VI inherits. VI.B* are the new contracts Part VI introduces. Any commit that breaks any A or B row reverts before push.

---

## Cross-binding bit-identity disposition

`LOOP_TRANSPORT` shares per-substep building blocks with `SYMPLECTIC_FLOW` (the IV.6 contract). If a Phase 1/1b/2 AURORA refactor or a Part V snapshot extension perturbs `wilson_force_per_edge`, `drift_step`, or `project_gauss`, both verbs feel it. VI.A1 is the regression detector for the SYMPLECTIC_FLOW side; VI.B1 is the regression detector for LOOP_TRANSPORT. Both run on every `cargo test --features halcyon --release` invocation.

The AURORA Phase 1–2 work (Phases 0/1/1b/2 at commits `ca589eb` / `f62e46c` / `1091dd5` / `17105ff`) was verified bit-identical against the existing IV.10 gold gate at each ship. The same discipline carries through Part VI: any LOC that touches `src/gauge/` hot paths gets its own commit, separate from the Part VI introduction, so the regression bisect surface stays clean.

---

## What is decided / what is not

### Decided (frozen at this gate doc commit)

- The v3.1.3 §4.4 grammar is the parser contract. No deviations.
- Verb name is `LOOP_TRANSPORT`. The spec's `SAMPLE_TRANSPORT` spelling stays frozen at deposit; the implementation uses the agreed name.
- GC₁–GC₆ as specified in v3.1.3 §7.4 are the acceptance battery. The 1% GC₅ threshold is non-negotiable.
- The 5 science-gate sham flags (FLAT_FIELD, ALPHA_ZERO, MASS_BASELINE_SCALED, DEGENERATE_LOOP, FROZEN_FIELD) ship in the `SHAM { ... }` block.
- S₄ is folded into the antisymmetric observable, not a flag.
- β_W range is `[2.5, 3.0]` strictly; parser errors on out-of-range.
- Bit-identity contract is per-seed; gold fixture format mirrors `section_12_holonomy_battery_v3_1_3` per v3.1.3 §7.2.
- HTTP routing through `POST /v1/gql`; no dedicated REST endpoint at Gate VI ship.
- Hot-path discipline: trait-object dispatch off the integrator inner loop.
- Implementation reuses `SYMPLECTIC_FLOW` per-substep building blocks (KDK skeleton, wilson_force_per_edge, drift_step, project_gauss). Reuse path keeps bit-identity inheritance from IV.10 clean.

### OPEN (resolution before code lands)

- **The 2 audit-story shams** referenced in Halcyon's `PENDING_FROM_GIGI.md` (`5 science-gate + 2 audit-story` = 7 total). v3.1.3 §5 lists 5 science + S₄-folded; I do not see the 2 audit-story shams enumerated. Halcyon enumerates them in a reply, or they get added to v3.1.4 as a deposit amendment. Either way they land in this gate doc as a follow-up Edit before VI.3 begins.
- **CC-LT-1 verb dispatch shape** — Halcyon's read on closed-enum vs open-registry for the parser arm. Closed enum is the default if no preference arrives by VI.2 GREEN.
- **GC₅ test fixture scope** — which loop sizes, which seeds for the convergence-bracket assertion. Default per v3.1.3 §7.4: γ_unit at the canonical 8 seeds, `N_discretization ∈ {1000, 2000, 4000, 8000, 16000}` (5-point bracket). Halcyon pushback welcome on widening the bracket if 1% is too tight a science-value gate for the canonical regime.

### Deferred (not in Part VI scope; later or never)

- A dedicated `/v1/loop_transport` REST endpoint. GQL via `/v1/gql` is the production interface at VI ship.
- LOOP_TRANSPORT result snapshotting (analogous to Part V SNAPSHOT). Out of scope unless Halcyon's `run_holonomy_battery.py` needs it.
- Other-group support beyond SU(2). The existing `gauge` feature's group-erased storage compiles for U(1) / Z_N but panics at use site; LOOP_TRANSPORT inherits that behavior.
- α_Halcyon derivation from the Davis Field Equations. Per Halcyon's PENDING note: "the protocol runs at α=1 and α=1000 without it. If it lands first, great; if not, the two pre-registered calibrations still produce a verdict." Not a Part VI blocker.

---

## Future-audit anchor

This gate doc commits at a date earlier than the LOOP_TRANSPORT implementation does. The implementation log (`HALCYON_PART_VI_IMPLEMENTATION_LOG.md`, written after VI ships) cross-references back here for the contract. A reviewer asking "what was supposed to happen" reads this file; a reviewer asking "what actually happened with which receipts" reads the impl log.

The Sprint B revert lesson is the meta-discipline: gates first, then code. The IV.10 gold gate caught a perf-win commit that would have invisibly broken the science. Part VI inherits the same defense via VI.B1.

The v3.1.3 pre-registration deposit at commit `44c70b1` (Zenodo DOI when minted) is the public commitment. Whatever Halcyon's `run_holonomy_battery.py` returns when it runs against this verb is the falsification result, regardless of whether the result is POSITIVE / NULL / AMBIGUOUS, and regardless of whether I prefer one outcome to another.

— Bee, 2026-06-21
