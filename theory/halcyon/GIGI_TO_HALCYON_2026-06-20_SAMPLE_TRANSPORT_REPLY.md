# GIGI -> HALCYON reply, SAMPLE_TRANSPORT loop-holonomy verb scope review (2026-06-20)

**From:** GIGI engine team (Bee + Claude)
**To:** Halcyon team
**Subject:** Five-piece ask received. Verb scoped as `LOOP_TRANSPORT`, a peer to `SYMPLECTIC_FLOW`. Per-ask scope review + cross-cutting design questions to pin before any LOC lands. Pre-registration at `0fe654d` honored.
**Companion to:** `HALCYON_TO_GIGI_2026-06-20_HOLONOMY_VERB_REQUEST.md` (relayed; not yet on disk under `theory/halcyon/`).

---

## Letter

Halcyon —

The five-piece ask reads cleanly. The verb (1), six sham flags (2), adiabaticity self-check (3), per-seed independence (4), and regression test (5) are all substrate-shaped work; we accept them as posed with one rename, one disambiguation, and three cross-cutting design questions worth pinning before any line of Rust lands.

The methodological commitment is the part of your letter we want to acknowledge first. The v3 SPEC's falsification criteria are locked at commit `0fe654d` independent of how we spec the verb. We will not negotiate against the pre-registration; the substrate timeline does not move what you accept as a result. That stance is exactly the discipline we want every cross-team verb introduction to inherit, and we are mirroring it here: this letter pins design questions, not promises.

One framing decision up front. The verb you described — `SAMPLE_TRANSPORT ... ALONG_LOOP ... ADIABATIC` driving a programmed parameter surface and computing holonomy at loop closure — is not a Halcyon-specific shape. It is the substrate primitive any future apparatus needs once it does parameter-space transport: staggered fermions, shidoku-quotient holonomy, AURORA-style atmospheric driven loops. We will ship it as a peer to `SYMPLECTIC_FLOW` under the gauge module, named by what it returns, with Halcyon's parameter pack as the first registered variant. The pre-registered falsification criteria at `0fe654d` sit on top of the generic verb — they constrain your use of it with the Halcyon parameter variant, not the verb's substrate semantics.

One naming note before the scope review: `SAMPLE_TRANSPORT` is already the name of a bundle-side curvature-bounded neighborhood sampler at `src/geometry/sample_transport.rs` (696 LOC, S4 feature, GQL verb at `src/parser.rs:5242`). It does configuration-space draws on a fiber, not parameter-space loop transport on a gauge field. We are proposing `LOOP_TRANSPORT` as the gauge-side verb name throughout this letter. If you intended to overload the existing name with an `ALONG_LOOP` modifier and a target-type dispatcher, flag it in the reply and we will work the parser surgery; if `LOOP_TRANSPORT` reads correctly to you, we adopt it. Either is fine; the rename is mechanical, the disambiguation must land before the parser arm.

One transparency item. The substrate had a parallel-WAL-replay regression today that lost a production bundle (`claude_substrate_v0`). Reverted at HEAD (the revert of commit `8912e3c`). Your bit-identity contracts (IV.10, III.8b, V.*) were not affected because gauge primitives go through a separate code path; we re-ran the Part IV gold gates against HEAD and they pass byte-identically. Surfacing because we owe transparency on substrate health, not because anything in your work is in question.

Below: scope per ask (§1–§5), cross-cutting design questions worth pinning before implementation (§6), generalization framing (§7), what we are not asking back (§8), pre-registration acknowledgment (§9). Pushback welcome on every clause.

—Bee + Claude

---

## 1. Ask 1 — the verb

### Shape

**Register a verb. Light extension; not refactor-first.** The integrator skeleton already exists in `SYMPLECTIC_FLOW`; `LOOP_TRANSPORT` wraps an outer parameter-march around a duplicate of the per-substep body.

### Scope

- New module `src/gauge/loop_transport.rs` (~280–350 LOC) defining `LoopTransportConfig { loop_id, ramp_rate, drive_omega, parameter_pack: ParameterPackKind, seeds }`, `LoopTransportDiagnostics { run_id, seed_count, steps_completed, adiabaticity_warnings, holonomy }`, and `loop_transport(u_name, e_name, config) -> Result<LoopTransportResponse>`. The function holds the outer parameter loop; the inner per-substep body duplicates the KDK kernel from `src/gauge/symplectic_flow.rs:294-330` and threads the ramp-rate / parameter-pack hooks at the F0/F1 and drift sites.
- New `Statement::LoopTransport` variant + `parse_loop_transport` function in `src/parser.rs` (~50–70 LOC) mirroring the `SYMPLECTIC_FLOW` parse shape at `src/parser.rs:2941-3003`. Dispatch arm at `src/parser.rs:1530`.
- `src/gauge/mod.rs` re-export of the new module + types (~5 LOC).
- New `Statement::LoopTransport` match arm in the executor (~15–25 LOC).
- New gate doc `theory/halcyon/HALCYON_PART_VI_GATES.md` locking bit-identity, holonomy correctness, and adiabaticity semantics. **Gate doc lands before code lands**, per the Sprint B revert lesson.

Total: ~350–450 LOC across three Rust files + one gate doc, not counting tests (covered in §5).

### What does not get touched

`src/gauge/symplectic_flow.rs` is not refactored on this commit. The per-substep KDK body at lines 294–330 stays where it is; `LOOP_TRANSPORT` duplicates the ~40 LOC of inner orchestration rather than extracting a shared helper. Rationale: the IV.6 gold-gate test is bit-identity-locked against the current symplectic_flow body, and the Sprint B parallel-WAL-replay revert is fresh evidence that touching production hot paths during verb introduction conflates two changes. Extraction is a follow-up commit gated by a third consumer (see CC-LT-4 in §6).

### Holonomy measurement

`COMPUTE HOLONOMY` at loop closure calls `walk_loop(lattice, edges, conn)` at `src/gauge/holonomy.rs:27-37` directly. The walker is general-purpose, group-erased, 11 LOC, zero state. No new holonomy machinery is needed — full reuse. The returned `GroupElement` lands in `LoopTransportDiagnostics.holonomy`.

### Design questions worth pinning before LOC lands

- **Loop declarability.** Your `loop_id` parameter implies a first-class loop object exists; today loops are implicit edge lists on faces (`src/gauge/holonomy.rs:44-57`). Two paths: (a) add a `DECLARE LOOP foo FROM faces(...) ON lattice bar` statement plus a `LoopRegistry` mirroring the `GaugeFieldRegistry` pattern (~60–80 LOC), or (b) keep `loop_id` as an opaque string handle resolved at execution time against a hashmap parked in the verb's own registry. Path (a) is the cleaner long-term shape and the right choice if you plan to reuse the same loop across multiple `LOOP_TRANSPORT` calls; path (b) ships faster if every call constructs its own loop inline. Pin this before the parser arm. See CC-LT-1.
- **Parameter-pack registry.** Your surface — `alpha_halcyon, tau_0, beta_tau, mu_baseline, K, c` — is Halcyon-domain-specific. Per the AURORA v0.1 reply's CC-1 (`HamiltonianKind` open registry), we are proposing `ParameterPackKind::Halcyon { alpha, tau_0, beta_tau, mu_baseline, K, c }` as the first registered variant, with `loop_id / ramp_rate / drive_omega / seeds` as kind-agnostic outer fields. `alpha_halcyon` becomes `alpha` inside the Halcyon variant. Future apparatuses register their own variants. See CC-LT-2.
- **Outer-loop shape.** Discrete (parameter takes N waypoints; KDK marches between waypoints) or continuous (parameter evolves smoothly with `ramp_rate`; KDK substep updates parameter each step)? Your adiabaticity ask (#3) implies the latter — `ramp_rate` is a continuous `d_param/dt` that must stay slower than the field-relaxation rate. We need confirmation; the executor shape differs (nested-for vs single-for with per-substep parameter update). Recommendation: continuous, with `n_substeps` derived from `ramp_rate` and the total loop arc length.

---

## 2. Ask 2 — six sham flags

### Shape

**Register an extension on the verb skeleton.** Five of six are clean 1–5 LOC at the per-substep insertion sites you identified; one needs disambiguation before code lands.

### Per-flag scope

| Flag | LOC | Insertion site | Status |
| --- | --- | --- | --- |
| `SHAM_FLAT_FIELD` | ~3 | F0 + F1 force computations (equivalents of `symplectic_flow.rs:297, 302`) — bypass `wilson_force_per_edge`, return zero-force vector | clean |
| `SHAM_ALPHA_ZERO` | ~2 | Parameter-pack entry — override `config.alpha` (and any equivalent coupling parameter inside the Halcyon variant) to 0.0 before the per-substep loop starts | clean |
| `SHAM_MASS_SCALED <float>` | ~4 | Drift step (equivalent of `symplectic_flow.rs:300`) — multiply exponent argument by scale factor: `drift_step(&mut e, dt * sham_mass_scale.unwrap_or(1.0), g2)` | clean |
| `SHAM_REVERSED_LOOP` | ~3 | Loop edge enumeration — reverse the edge list before passing to `walk_loop`, flip every orientation. Composes with existing `EdgeOrientation::{Forward, Reverse}` flip | clean |
| `SHAM_DEGENERATE_LOOP` | ~3–8 | TBD (depends on disambiguation) | **needs disambiguation; see below** |
| `SHAM_FROZEN_FIELD` | ~3 | Drift step — when flag set, skip the U update entirely. `drift_step` is the only mutation site for U, so bypassing it freezes the field | clean |

Total for the five clean flags: ~15–20 LOC.

### The disambiguation ask: `SHAM_DEGENERATE_LOOP`

"Degenerate" reads three ways. Each is a different test target:

1. **Out-and-back.** Loop traverses an edge and immediately backtracks. Holonomy is identity by construction (because `walk_loop` composes `U · U^{-1} = I` for the back-edge). ~3 LOC at edge enumeration.
2. **Zero-length.** Loop has no edges. Holonomy is `GroupElement::su2_identity()` by `walk_loop`'s initial value (`src/gauge/holonomy.rs:32`). ~1 LOC.
3. **Non-closing.** Loop's last vertex does not match its first. This is a parser-rejection question, not a runtime sham; the loop is malformed at declaration time.

Pick one and we build that one. If you want all three as separate flags (`SHAM_BACKTRACK_LOOP`, `SHAM_EMPTY_LOOP`, `SHAM_OPEN_LOOP`), that is ~10 LOC total and is the more honest shape — each tests a different invariant. Recommendation: split into three.

### Structural ask on the API surface

Six sham flags as top-level grammar keywords, or nested in a `SHAM { flat_field: true, alpha_zero: true, ... }` clause? Top-level is more visible (sham presence shouts at the operator reading the GQL); nested is more grammar-stable (adding a seventh flag doesn't add a new top-level keyword). Recommendation: nested. The verb's `SHAM` clause is itself a load-bearing observability surface and deserves a single declarative block. Pushback welcome. See CC-LT-6.

---

## 3. Ask 3 — adiabaticity self-check

### Shape

**New observable + new diagnostic field.** Does not piggyback on existing energy-drift or Gauss-residual measurement.

### What's already there, what's missing

The existing diagnostics in `src/gauge/symplectic_flow.rs` are `max_energy_drift_rel` (symplecticity check, IV.10, derived from H_total chain history not instantaneous rate) and `gauss_residual_max` (covariant Gauss residual at end-of-flow only, `src/gauge/symplectic_flow.rs:359-368`). Neither is an instantaneous gauge-relaxation rate.

Your ask requires three new things:

1. Compute the field's instantaneous response rate during each substep.
2. Compare that to `ramp_rate` (the parameter `d_param/dt`).
3. Emit a warning when `ramp_rate > relaxation_rate` (strict, see threshold question below).

You are correct that the substrate is the right place for this. We hold `(U, E)` at every substep and can compute the norm in `O(n_edges)` per step without exposing the full field state to Python. Python only sees summary measurements; it cannot diagnose adiabaticity violations from the outside.

### Proposed shape

- New `Observable::AdiabaticityRate` variant — per-substep `f64` chain (one sample per substep, stored in the same measurement history map as `PlaquetteMean`, etc.).
- New diagnostic field `adiabaticity_warnings_count: usize` in `LoopTransportDiagnostics`.
- New diagnostic field `adiabaticity_warning_steps: Vec<usize>` recording substep indices where the warning fired.
- WAL record per warning so the violation is durable and queryable post-hoc, not just stderr-logged.

### Three design questions that must settle before code lands

These are the questions where getting the answer wrong costs a second LOC pass:

- **What is the relaxation-rate formula?** Your letter says "instantaneous gauge-relaxation rate" but does not pin the analytical expression. Three candidates:
  - `||dU/dt||_op = g² · sup_edges ||E_edge||` (operator norm of the drift; cheap, reads what the integrator already computes)
  - `1 / τ_min`, where `τ_min` is the inverse of the largest eigenvalue of the local curvature operator (theoretically correct, expensive — requires a per-step eigensolve)
  - `||F||_op` (force operator norm; action-curvature in field space; cheap, reads what `wilson_force_per_edge` already returns)

  Per the t013 three-constraint definition, this is the no-tunable-tolerance analytical target. The formula must come from theory, not be an operator epsilon. Pin one formula in the v3 SPEC (or commit a primary with a documented fallback). Without it, the warning is parametrized on something we picked, and the test (ask 5) cannot be pre-registered against a specific bound. Recommendation: candidate (a) for cheapness if your theory permits it; (b) if not.

- **Warning threshold.** Strict (`ramp_rate > relaxation_rate` fires when equality is violated) or `ramp_rate > κ · relaxation_rate` for some safety factor `κ < 1`? If `κ` exists, it is a tunable tolerance and violates the three-constraint contract that the AURORA v0.1 reply locked in. Push back if you want `κ` — demand `κ = 1` (strict) or `κ` derived from theory (e.g., from the adiabatic theorem's gap-squared bound).

- **Emission cadence.** Every substep (expensive — `O(n_edges)` per step) or every `measure_every` substeps (cheap, may miss short violation windows). Recommendation: every substep but only computing the norm — the comparison is one `f64` compare. If the chosen formula reads a quantity that an existing observable already computes (`HTotal` or `PlaquetteMean` reads `||E||`), we reuse it for free.

---

## 4. Ask 4 — per-seed independence

### Shape

**Register a verb.** Inherits the seed-propagation pattern from `SYMPLECTIC_FLOW` and `GIBBS_SAMPLE` with one shape question.

### What inherits cleanly

The seed-from-u64 pattern at `src/gauge/gibbs_sample.rs:226` (`SmallRng::seed_from_u64(seed)`) and the seed-or-entropy pattern at `src/geometry/sample_transport.rs:252` are both available. Sub-kernel RNG threading (per-substep stochastic kernels, holonomy measurement if stochastic) lands as a direct port. No new design needed for the threading mechanism.

### The shape question

`SYMPLECTIC_FLOW` takes `seed: Option<u64>` (singular, echo-only per `src/gauge/symplectic_flow.rs:239-247` — the flow has no stochastic kernel today). Your surface specifies `seeds: Vec<u64>` (plural). Two readings:

1. **Ensemble replication.** Each seed in the `Vec` maps to one replica run. The verb runs `len(seeds)` independent trajectories and accumulates ensemble statistics. The trajectories are independent in the parallel-runs sense.
2. **Per-substep seed thread.** One trajectory; `seed_i` is consumed at substep `i`. The trajectory has different RNG state at each step but is a single run.

These are entirely different executor shapes. Confirmation needed. Recommendation: ensemble replication (reading 1). It is the cleaner shape, parallelizable across cores, and matches how Monte-Carlo studies usually want to aggregate per-seed results.

### Reproducibility gate (for the Part VI gate doc)

A bit-identity contract along the lines of "same `seeds` vec + same config → byte-identical `measurement_history` and byte-identical holonomy `GroupElement` components" needs to land in `theory/halcyon/HALCYON_PART_VI_GATES.md`. This mirrors the IV.6 gate. Flagged here so it doesn't get lost between the design phase and the test phase.

---

## 5. Ask 5 — regression test placement

### Shape

**Register a verb.** Test infrastructure exists; one new file following the `halcyon_part_*.rs` convention.

### File placement

`tests/halcyon_part_vi_loop_transport.rs` (new file). Rationale: the existing pattern is `tests/halcyon_part_<numeral>_<purpose>.rs` (e.g., `halcyon_part_iv_gold.rs`, `halcyon_part_v_snapshot.rs`, `halcyon_part_v_p1_gql_dispatch.rs`). `LOOP_TRANSPORT` is a new Part-VI scope, so it gets its own file. Gate doc lands at `theory/halcyon/HALCYON_PART_VI_GATES.md` alongside. Every test uses `#[cfg(feature = "halcyon")]`.

**Do not mix with `halcyon_part_iv_gold.rs`.** Those tests are bit-identity-locked, and any churn in that file risks the v3 SPEC's pre-registered falsification criteria at `0fe654d`. Part VI is its own file, isolated from Part IV.

### Three assertion blocks

- **(a) Sham flags return identity holonomy.** Six tests (one per flag, possibly more after the `SHAM_DEGENERATE_LOOP` disambiguation lands). Each declares a known-non-trivial gauge field (e.g., the buckyball with planted half-turn from `src/gauge/holonomy.rs` TDD-HAL-I.5 test fixture, or a Part-IV thermalized field), runs `LOOP_TRANSPORT` with the sham flag set, asserts the returned holonomy `GroupElement` compares-equal to `GroupElement::su2_identity()` within FP64 tolerance. Reuses the `compare_su2_identity` helper at `src/gauge/holonomy.rs:65-75`. ~30–40 LOC per test. Total: ~180–240 LOC.

- **(b) Trivial-bundle holonomy = identity.** One test. Declares a gauge field initialized to the identity connection everywhere (use the `FixedEdgeConnection::identity_everywhere()` pattern from `src/gauge/holonomy.rs:82`, or add a `gauge_field_set_identity` constructor if one is not wired). Runs `LOOP_TRANSPORT` with no sham flags, asserts holonomy = identity on every loop in a set of test loops (face loops on the buckyball, same pattern as TDD-HAL-I.4). Essentially TDD-HAL-I.4 wrapped in a `LOOP_TRANSPORT` call. ~40–60 LOC.

- **(c) Known-non-trivial holonomy = analytic value.** One test. The recommendation for v1 is the planted-half-turn buckyball edge from TDD-HAL-I.5 — analytic value is `half_turn_z = SU2 { q0: 0, q1: 0, q2: 0, q3: 1.0 }`, gives an exact FP64 compare, reuses existing test fixtures. The test frames as "LOOP_TRANSPORT recovers the `walk_loop` answer when the parameter loop is trivial (no ramping)" which is the regression target. ~50–80 LOC.

  Your v3 SPEC pre-registered targets may demand a "real" physics test instead (e.g., a strong-coupling-limit Wilson loop where holonomy is computable in closed form). That test is larger (~150 LOC + numerical-stability section) and should be a **separate** test, not bundled into the three-assertion gate suite. Push back if you want it in the `cargo test --features halcyon` gate suite versus being a longer-running `#[ignore = "physics regression, runs in nightly CI"]` test. Our recommendation: keep the gate suite at the three-assertion shape and run the physics regression separately.

Total test file: ~300–400 LOC.

---

## 6. Cross-cutting design questions to pin before implementation

These are the questions where the answer affects the parser surface, the executor shape, and every future consumer of the verb. Each one is worth pinning in the v3 SPEC before LOC lands. Per Sprint B revert lesson, design questions get answered first.

### CC-LT-1 — Loop declarability

First-class declarable loop object (`DECLARE LOOP foo FROM faces(0, 3, 7, 12) ON lattice bar;` plus a `LoopRegistry` mirroring `GaugeFieldRegistry`) or opaque string handle (`loop_id` resolves at execute time against an inline cache)? Mirrors AURORA CC-2 (`LatticeTopology` registry). Recommend first-class if Halcyon plans to reference the same loop across multiple verb invocations; opaque-string-handle otherwise. Pin before the parser arm lands.

### CC-LT-2 — Parameter-pack registry

`ParameterPackKind` as an enum variant (variants: `Halcyon { alpha, tau_0, beta_tau, mu_baseline, K, c }`, future: `StaggeredFermion { ... }`, `ShidokuQuotient { ... }`) with kind-agnostic outer fields (`loop_id, ramp_rate, drive_omega, seeds`). Mirrors `HamiltonianKind` exactly. Halcyon registers as the first variant; future apparatuses plug in their own. Recommendation: yes. Push back if you want the verb signature to carry Halcyon-domain identifiers directly.

### CC-LT-3 — Adiabaticity threshold

Per the t013 three-constraint definition (no tunable tolerance, analytical target), the relaxation-rate formula must come from theory. The three candidates listed in §3 each cost different things to compute. Need a commit to one (or a primary with documented fallback). If the v3 SPEC instead asks for an operator-tunable `κ` safety factor, push back — `κ` is a tunable tolerance and violates the three-constraint contract that the AURORA v0.1 reply locked in.

### CC-LT-4 — Integrator reuse vs duplication

Extract `per_substep_kdk` helper from `src/gauge/symplectic_flow.rs:294-330` (clean code; risks IV.6 gate) or duplicate the body in `src/gauge/loop_transport.rs` (~40 LOC duplication; zero risk to existing gates). Recommendation: duplicate for v1, per Sprint B revert lesson. Touching the IV.6-gated hot path on a verb-introduction commit conflates two changes. Extraction is a follow-up commit gated by a third consumer materializing (e.g., a staggered-fermion `LOOP_TRANSPORT` peer).

### CC-LT-5 — Name collision resolution

`src/geometry/sample_transport.rs` already exists as the bundle-side `SAMPLE_TRANSPORT` verb (S4 feature, GQL verb at `src/parser.rs:5242`). The Halcyon verb is a gauge-side peer that does not exist today. Three resolutions:

- (a) Rename the gauge-side verb to `LOOP_TRANSPORT`. Clean, no parser disambiguation, name accurately describes what it computes.
- (b) Overload `SAMPLE_TRANSPORT` with an `ALONG_LOOP` modifier that triggers target-type dispatch (bundle vs gauge field). Requires parser surgery.
- (c) Namespace as `GAUGE.SAMPLE_TRANSPORT` in the grammar. Breaks the flat-verb convention.

Recommendation: (a). Pin with Halcyon before the parser arm.

### CC-LT-6 — Sham-flag API shape

Six `SHAM_*` flags as top-level grammar keywords or nested in a `SHAM { flat_field: true, alpha_zero: true, ... }` clause? Top-level shouts at the operator reading the GQL; nested is more grammar-stable for adding a seventh flag without a new keyword. Recommendation: nested.

---

## 7. Generalization framing

`LOOP_TRANSPORT` is a peer to `SYMPLECTIC_FLOW` under the gauge module. The substrate primitive is "march a configuration-space integrator along a programmed parameter-space path and measure a gauge-invariant observable at loop closure." Halcyon is the first consumer — mass-gap probe with six sham flags for false-positive testing — but the verb is not Halcyon-specific.

Downstream consumers that plug in as `ParameterPackKind` variants (CC-LT-2):

- Staggered fermions.
- Shidoku-quotient holonomy.
- AURORA-style atmospheric driven loops (any apparatus that ramps a parameter and reads a gauge-invariant observable at loop closure).
- Any future Halcyon-style apparatus that drives a programmed parameter through a loop.

The verb signature stays clean of Halcyon-domain identifiers: `alpha_halcyon` becomes `alpha` inside the Halcyon variant; `loop_id, ramp_rate, drive_omega, seeds` are kind-agnostic outer fields. The adiabaticity self-check (ask 3) is similarly general — every future consumer that ramps a parameter inherits the no-tunable-tolerance relaxation-rate gate from the t013 three-constraint definition. This matches the AURORA v0.1 reply discipline: name the verb by what it returns, register the consumer's parameters as a variant, lock the generic gate.

The pre-registered falsification criteria at `0fe654d` sit on top of the generic verb. They constrain your use of `LOOP_TRANSPORT` with the Halcyon parameter variant, not the verb's substrate semantics. If a future letter starts describing `LOOP_TRANSPORT` as "the Halcyon verb," we have drifted; pushback welcome on that drift the same way you would push back on a parser bug.

---

## 8. What we are not asking back

Per your letter's stance, and reciprocating it:

- **No turnaround date.** The verb ships when the design questions in §6 settle. We are not committing to a week or a milestone.
- **No v3 outcome guarantee.** Substrate ships a verb; what Halcyon's mass-gap probe returns when it runs against the verb is your falsification result, not ours to predict.
- **No guarantee the verb will pass your v3 SPEC.** We will build it to the substrate's three-constraint definition (gauge-invariant observable, local per-step updates, no-tunable-tolerance analytical target). Whether your physics passes through that verb is the experiment.

What we are committing to:

- The verb gets shipped with the gate doc on disk before the implementation commit lands.
- The bit-identity contracts on IV.10 / III.8b / V.* stay locked. Any LOC that touches their hot paths gets its own commit, separate from the `LOOP_TRANSPORT` introduction.
- The cross-cutting questions in §6 get answered in writing (here, or in your next letter, or in the v3 SPEC) before the parser arm or executor body lands.

---

## 9. Pre-registration acknowledgment

Commit `0fe654d` is the independent referee. Whatever the v3 SPEC's falsification criteria say at that hash is what Halcyon will accept as a result — independent of when the substrate ships the verb, independent of the verb's parser surface, independent of which of the design questions in §6 settle which way.

The substrate timeline does not move the pre-registration. The pre-registration does not move the substrate timeline either. They are two clocks, locked separately. That is the methodological commitment we want to reciprocate, and it is the reason every design question in this letter is "pin this before LOC lands" rather than "we will iterate on this in the implementation log." Pre-registered work earns up-front design discipline.

Pushback welcome on every clause in this letter. The cross-cutting questions in §6 in particular are decisions, not announcements — we want Halcyon's read before they land.

—Bee + Claude
