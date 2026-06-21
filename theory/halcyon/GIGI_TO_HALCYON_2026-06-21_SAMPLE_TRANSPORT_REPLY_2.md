# GIGI → HALCYON reply 2, LOOP_TRANSPORT scope refresh against v3.1.3 (2026-06-21)

**From:** GIGI engine team (Bee + Claude)
**To:** Halcyon team
**Subject:** v3.1.3 supersedes v3.0. Five updates to v1 commitments — pre-registration anchor, verb naming lock, S vs GC two-surface disambiguation, antisymmetric observable as return-shape, β_W ∈ [2.5, 3.0] validation.
**In reply to:** `HALCYON_FALSIFICATION_BATTERY_SPEC_v3.1.3.md` (commit `44c70b1`) and the Halcyon 2026-06-21 reply (commit `c70e86a`).
**Prior letter:** `theory/halcyon/GIGI_TO_HALCYON_2026-06-20_SAMPLE_TRANSPORT_REPLY.md` (commit `302ce1a`).
**Supersedes (where changed):** the v1 reply, in the five locations §1 names. Everywhere else v1 stands verbatim. v1 stays in git history; this letter is the operative substrate-side ask document.

---

## Letter

Halcyon —

v3.1.3 supersedes v3.0. The v1 reply pinned commit `0fe654d` as the independent referee; that hash is the first draft. The canonical pre-registration going to Zenodo is `44c70b1` (v3.1.3) plus the DOI when minted. The two-clocks framing stands; the clock I named on the Halcyon side is the wrong one. This letter moves it.

The chain from v3.0 to v3.1.3 was five rounds of review — Gigi's methodological intervention establishing the philosophical posture in v3.0, then four pre-deposit technical review rounds: round 1 caught two mathematical defects (scalar holonomy vanishing by the fundamental theorem of calculus, the adiabaticity inequality reversed), round 2 caught seven executability issues, round 3 caught the validity-window blocker (β_W traversal outside the validated SU(2) regime), round 4 caught three wording and audit-tightness issues including the GC₅ science-value gate and the substrate-gated τ_pin. Every round caught real defects that a one-pass pre-registration would have locked in. That is what pre-registration is supposed to do: each pass before the deposit increases the chance the deposited version is the one you want to be held to. v1's praise for the methodology gave it less credit than it deserved. The discipline is stronger than I described.

The substantive change set is small and well-bounded. Five places in v1 need updating; everywhere else v1 stands. The updates are:

1. Pre-registration citation moves from `0fe654d` to `44c70b1` + the imminent Zenodo DOI.
2. Verb name LOCKS to `LOOP_TRANSPORT`. Halcyon's §B.1 reply already adopted the rename; v2 confirms.
3. v1's "six sham flags" framing was a conflation. v3.1.3 has five sham controls S₁–S₅ (with S₄ absorbed into the antisymmetric primary observable, not shipped as an external flag) AND six verb-acceptance contracts GC₁–GC₆. These are two distinct surfaces. The substrate commits to GC₁–GC₆ as the verb's introduction battery before any science call fires.
4. The verb's `COMPUTE HOLONOMY` clause returns a tuple, not a scalar. Both `H_geom = ½(H[γ] − H[γ⁻¹])` (antisymmetric, primary, load-bearing) and `H_sys = ½(H[γ] + H[γ⁻¹])` (symmetric, systematic-offset diagnostic) are built into the verb's return shape. Not an option. Not a query-side derivation.
5. β_W is validated to `[2.5, 3.0]` strictly at the parser. The lower endpoint coincides with the locked Halcyon canonical thermalization β; the upper edge is the validated envelope's upper bound.

Everything else v1 said stands. The generalization framing (peer to `SYMPLECTIC_FLOW`, not Halcyon-specific) stands. The two-layer two-clocks methodological discipline stands — now pointing at the right reference. The cross-cutting CC questions stand, with refinements where v3.1.3 named what v1 was guessing at. The "what we are not asking back" stance stands. The transparency about the WAL revert stands (and Halcyon's §G acknowledged it without requiring relitigation; we don't reopen).

Decision summary up front, detail follows.

—Bee + Claude

---

## 1. Decision summary

| v1 commitment | v2 verdict | One-sentence rationale |
|---|---|---|
| Pre-registration anchor `0fe654d` | UPDATED to `44c70b1` + imminent Zenodo DOI | v3.0 was first draft; v3.1.3 is canonical after a five-round review chain. |
| Verb name `LOOP_TRANSPORT` (proposed in v1) | LOCKED per Halcyon §B.1 acceptance | Halcyon adopted the rename verbatim; collision with `src/geometry/sample_transport.rs` resolved by name, not overload. Path (b) (target-type dispatcher on the SPEC's `ALONG_LOOP CONTROL_MANIFOLD` suffix) documented as the alternative we would have built. |
| "Six sham flags" | DISAMBIGUATED into five sham controls S₁–S₅ + six GC contracts GC₁–GC₆ | v1 conflated two surfaces; v3.1.3 §5 and §7.4 split them; S₄ folds into the antisymmetric observable, not a separate flag. |
| Holonomy as scalar output | TUPLE return: `{ h_geom, h_sys, ... }` | v3.1.3 §3.1 and §4.4 make both observables load-bearing; the verb carries both at the response boundary. |
| Adiabaticity check threshold open | SPLIT into Observable A (τ_pin / T_segment, threshold lives in SPEC at 0.1, applied outside substrate) + Observable B (`ramp_rate / gauge_relaxation_rate`, diagnostic only, no threshold pre-registered) | Per Halcyon §C.3; substrate emits f64 ratios, Halcyon's Python applies the gate. v3.1.3 §4.2 makes the ADIABATICITY_CHECK gate substrate-side: violation forces AMBIGUOUS regardless of H. |
| β_W parameter surface unconstrained in v1 | VALIDATED to `[2.5, 3.0]` strictly at parser | v3.1.3 §4.1 locks the validity window inside the SU(2) Q-observable regime; extension below 2.5 requires independent validation + v3.1.x amendment. |
| GC contracts as test commitment | GC₁–GC₆ acceptance battery as substrate-side gate before any Halcyon science call | New `tests/halcyon_part_vi_loop_transport_gc.rs`; GC₅ carries the science-value gate (8000→16000 < 1%); substrate blocks science calls if the gate fails. |
| `ParameterPackKind::Halcyon` field list (v1 said "first registered variant") | LOCKED per Halcyon §B.2 canonical list | `{ alpha, tau_0, beta_tau, mu_baseline, K_spring, c_damp, drive_omega, drive_F0, pin_lambda_Q, pin_lambda_beta_W, eps_Q, eps_beta_W }`; `drive_omega` and `drive_F0` move INSIDE the Halcyon variant. |
| `loop_id` opaque-string-or-first-class question (CC-LT-1) | RESOLVED: first-class `DECLARE LOOP` + `LoopRegistry` per Halcyon §C.1 path (a) | Two declared loops in v3.1.3 consume it: `gamma_unit`, `gamma_degenerate`. `gamma_unit⁻¹` is NOT separately declared — substrate time-reverses in the executor. |
| Per-axis vs scalar `ramp_rate` (CC-LT-8 new) | PER-AXIS, four-segment rectangle in (Q, β_W) | v3.1.3 grammar uses `RAMP_RATE_Q 0.04`, `RAMP_RATE_BETA_W 0.01`; substrate ships `LoopShape::PiecewiseLinear { vertices, t_per_segment }` as v0.1. |

No new questions back to Halcyon. The 2026-06-21 reply resolved CC-LT-1 through CC-LT-6; v2 names CC-LT-7 (loop time-reversal mechanism) and CC-LT-8 (per-axis ramp_rate shape) as substrate-side pins that fall out of v3.1.3's structure, not as questions back.

---

## 2. Pre-registration citation update

v1 §9 named commit `0fe654d` as the independent referee. That commit is v3.0, the first draft. Replace with:

- **Canonical pre-registration:** `HALCYON_FALSIFICATION_BATTERY_SPEC_v3.1.3.md` at commit `44c70b1` plus the Zenodo DOI when minted.
- **Chain of custody (history, not authority):** v3.0 (`0fe654d`) → v3.1 (`7121094`) → v3.1.1 (`1165d63`) → v3.1.2 (`f4cfa14`) → v3.1.3 (`44c70b1`).

The five-round review chain is what should be acknowledged about v3.1.3, not as a marketing line but as a load-bearing methodological fact:

- **Round 0 (Gigi's intervention → v3.0).** Established the philosophical posture (gauge-invariant observable, local per-step updates, no-tunable-tolerance analytical target). Sets the frame that subsequent rounds enforce against.
- **Round 1 (→ v3.1).** Caught two mathematical defects: the scalar holonomy vanishing by FTC (the cumulative trace of a path-ordered exponential along a closed loop telescopes to identity in the scalar reading); the adiabaticity inequality reversed. Both load-bearing. A one-pass pre-registration would have locked these in.
- **Round 2 (→ v3.1.1).** Caught seven executability issues. Each was a real ambiguity in the protocol description that would have surfaced at substrate-implementation time and forced a v3.1.x patch under deposit.
- **Round 3 (→ v3.1.2).** Caught the validity-window blocker. β_W traversal outside the validated SU(2) regime would have produced numbers that interpreted as a substrate signal but were actually integrator artifacts at an un-validated coupling. Caught before deposit; the cost is a parser range check at substrate side.
- **Round 4 (→ v3.1.3).** Caught three wording and audit-tightness issues. Two of these are real protocol changes, not editorial: (a) GC₅ now carries an explicit science-value gate (N=10000 accepted only if 8000→16000 relative change < 1%) — this turns a "convergence check" annotation into a substrate-enforced gate that blocks science calls if it fails; (b) τ_pin moves from prose numerical fact to nominal design target with substrate-side ADIABATICITY_CHECK gate (violation forces AMBIGUOUS regardless of H values).

The pre-deposit technical review rounds are model-assisted reviews of the SPEC's mathematical content and protocol executability. They are not a substitute for external human peer review. v3.1.3 is the version that goes to deposit; future external review happens after.

**The two-clocks framing stands and now points at the right references.** Halcyon's clock: `44c70b1` + the Zenodo DOI when minted, pre-registered falsification criteria locked independent of substrate timeline. Gigi's clock: the substrate-side three-constraint contract (gauge-invariant observable, local per-step updates, no-tunable-tolerance analytical target), locked independent of which falsification criteria Halcyon registers. Both clocks ran during the five-round review chain; they did not interfere with each other.

The methodology — five passes of review before depositing — is the right posture for pre-registration. Each pass catching real defects is the property pre-registration is meant to enable, not a sign the process is broken. v1's praise was correct in direction but undersized; v2 corrects it.

---

## 3. Verb grammar disambiguation

v1 proposed renaming the gauge-side verb to `LOOP_TRANSPORT` to resolve the collision with `src/geometry/sample_transport.rs` (706 LOC, S4 feature, GQL verb at `src/parser.rs:5242`). v3.1.3's SPEC text uses the original grammar:

```
SAMPLE_TRANSPORT halcyon_canonical_buckyball
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
  RETURN H_forward, H_reversed, sigma_H_blocked,
         per_seed_H_forward, per_seed_H_reversed,
         tracking_error_max_Q, tracking_error_max_beta_W,
         adiabaticity_check
```

Two paths read this grammar:

- **(a) `LOOP_TRANSPORT` rename.** v1's proposal. Cleanest. No parser surgery beyond the new arm. Collision with the bundle-side `SAMPLE_TRANSPORT` resolved by separate keyword. The CONTROL_MANIFOLD keyword still introduces the 2D control surface Λ = (Q, β_W) — the rename only changes the verb name, not the clause structure.
- **(b) `SAMPLE_TRANSPORT` keyword preserved + target-type dispatcher on `ALONG_LOOP CONTROL_MANIFOLD` modifier suffix.** Implements v3.1.3 SPEC text verbatim; preserves Halcyon's naming. More parser work (target-type dispatch on the bundle-vs-gauge target type), but a clean rule.

Halcyon's 2026-06-21 reply (§B.1) accepted path (a). The substrate adopts `LOOP_TRANSPORT` as the gauge-side verb name. We read v3.1.3's grammar through the rename: `LOOP_TRANSPORT ... ALONG_LOOP ... CONTROL_MANIFOLD ... ADIABATIC` is the new arm; the SPEC text's `SAMPLE_TRANSPORT` keyword is read as `LOOP_TRANSPORT` per Halcyon's restatement. Path (b) is documented here for archival accuracy as the alternative we would have built if Halcyon had preferred to preserve the SPEC keyword; we do not re-open the question.

This is also the right place to pin the executor-side detail noted in §1: `γ_unit⁻¹` is NOT a separately declared loop. The substrate computes the reversed walk by traversing `gamma_unit` time-reversed in the executor. One `DECLARE LOOP` statement per logical loop in the registry; the reverse-traversal is an inline option on the `LoopTransport` WAL entry. This is what makes the H_geom return-shape cheap (single loop registration, two walks, two combinations).

---

## 4. Five sham controls S₁–S₅ + six GC contracts GC₁–GC₆

v1's framing collapsed two distinct surfaces into "six sham flags." v3.1.3 splits them, and the split is load-bearing. The substrate ships both.

### 4.1 Five sham controls S₁–S₅ (test-only insertions; gate Halcyon-protocol science-call interpretation)

These are the protocol-side falsifiers. They are inserted into the verb to verify that a non-trivial holonomy reading reflects the geometry of the connection rather than a substrate or orchestrator artifact. Per v3.1.3 §5:

| Control | Implementation flag | Gate |
|---|---|---|
| **S₁** flat field (κ_Q ≡ 0 on all edges, all times) | `SHAM_FLAT_FIELD = true` | `|H_S₁| < 2σ_S₁` AND `|H_S₁| < 10⁻¹⁰` |
| **S₂** α_Halcyon = 0 | `ALPHA_HALCYON = 0` | `|H_S₂| < 10⁻¹⁰` (load-bearing); `2σ` check is sanity |
| **S₃** μ_baseline scaled (×0.1, ×1, ×10) | `MU_BASELINE ∈ {0.1, 1.0, 10.0}`; substrate fits baseline-subtracted H | POSITIVE branch: baseline-subtracted H invariant within 10%. NULL/AMBIGUOUS branches: `|H_S₃ at μ_baseline=1| < 2σ_S₃` AND `< 10⁻¹⁰` |
| **S₄** reversed-loop | (no separate flag — absorbed into H_geom) | Folded into the antisymmetric primary observable per §3.1; satisfied by construction |
| **S₅** degenerate loop (zero area in Λ) | `LOOP gamma_degenerate` (declared loop) | `|H_S₅| < 2σ_S₅` AND `|H_S₅| < 10⁻¹⁰` |

S₄'s absorption into H_geom is the structural correction to v1. The reversed-loop sham assertion `H[γ⁻¹] = −H[γ]` is mathematically what the antisymmetric combination `½(H[γ] − H[γ⁻¹])` is testing for. Building the antisymmetric observable into the return tuple makes the reversed-loop sham unfakeable by construction: any reading that is not antisymmetric under loop reversal cannot survive the H_geom projection. There is no external flag for it because the assertion is the return-shape contract.

The four explicit sham flags ship under Halcyon's §C.6 nested `SHAM { ... }` block (CC-LT-6 accepted nested over top-level): `flat_field` (S₁), `alpha_zero` (S₂), `mass_scale: Option<f64>` (S₃), `degenerate_loop` (S₅, mapped to `BACKTRACK_LOOP` per Halcyon §D.1), plus the `frozen_field` flag noted in Halcyon's table. v1's `SHAM_DEGENERATE_LOOP` disambiguation discussion is resolved by Halcyon's §D.1: the substrate ships three companion flags — `SHAM_BACKTRACK_LOOP` (canonical S₅ mapping; out-and-back gives identity by `U · U⁻¹`), `SHAM_EMPTY_LOOP` (GC₄ companion; zero edges gives identity by walker initialization), `SHAM_OPEN_LOOP` (parser-rejection test; loop's last vertex does not match first). The required science-gate sham set uses only `BACKTRACK_LOOP`; the other two are substrate-internal audit-story flags.

Re-scoping v1's ask 2 explicitly: the substrate ships four flags inside `SHAM { ... }` (S₁, S₂, S₃, S₅), confirms S₄ folds correctly into the antisymmetric observable as a return-shape assertion, and ships the three companion degenerate-loop flags for audit. That is the correct shape; v1's "six sham flags" framing is superseded.

### 4.2 Six GC contracts GC₁–GC₆ (substrate-correctness contracts; gate verb introduction)

These are the verb-acceptance gates. They verify that `LOOP_TRANSPORT` computes a holonomy correctly before any science call against it is meaningful. Per v3.1.3 §7.4, in `tests/halcyon_part_vi_loop_transport_gc.rs` (new file, `#[cfg(feature = "halcyon")]`):

- **GC₁ — flat-connection-zero.** Construct a known-flat connection (A ≡ 0 in synthetic mode); verify H[any loop] = 0 to machine ε across at least 4 loop shapes.
- **GC₂ — Abelian constant-curvature area law.** Construct a connection with constant curvature F₀ in (Q, β_W); verify H[γ] = F₀ · Area(γ) to 1% across 3 loop sizes.
- **GC₃ — reversed-loop sign-flip.** For an arbitrary connection, verify H[γ⁻¹] = −H[γ] (Abelian) or H[γ]⁻¹ (non-Abelian) to 1% across at least 3 connections.
- **GC₄ — zero-size-zero.** Construct a degenerate loop bounding zero area; verify H = 0 to machine ε.
- **GC₅ — discretization convergence + science-value gate (v3.1.3 round-4 patch).** Compute H at N_discretization ∈ {1000, 2000, 4000, 8000, 16000}; verify monotone convergence with relative change < 1% between 8000 and 16000 substeps. The v3.1.3 science call uses N = 10000, which lies inside this convergence bracket. Science calls are accepted only if the 8000→16000 relative change is < 1%; otherwise science calls are blocked at the substrate side until the bracket is widened, the verb is patched, or a v3.1.x amendment moves N.
- **GC₆ — gauge invariance.** Apply a known gauge transformation to the substrate's connection; verify H is invariant to machine ε.

The substrate commits to GC₁–GC₆ as the verb's introduction battery. A 1373-assertion green run is necessary but not sufficient; GC₁–GC₆ green is the gate. The GC₅ science-value gate is treated as a real acceptance refinement, not as an annotation: substrate-side enforcement means a Halcyon science call at N = 10000 is rejected if the 8000→16000 relative change exceeds 1%. The substrate does NOT negotiate the 1% threshold; if the verb cannot meet it, the verb is patched (or N is moved by a v3.1.x amendment from Halcyon), not the threshold.

### 4.3 The two surfaces are independent

GC contracts gate verb introduction; sham controls gate Halcyon-protocol science-call interpretation. They live in two different test files (`tests/halcyon_part_vi_loop_transport_gc.rs` for GC₁–GC₆, `tests/halcyon_part_vi_loop_transport.rs` for the sham-flag assertions and the v1 §5 three-block test suite). They share no fixtures and are gate-checked independently. The gate doc `theory/halcyon/HALCYON_PART_VI_GATES.md` includes both tables side by side so they are never conflated again.

---

## 5. Antisymmetric observable built into the verb's return shape

Per v3.1.3 §3.1 and §4.4, the verb's `COMPUTE HOLONOMY` clause returns a tuple. Both observables are load-bearing:

- **H_geom = ½(H[γ_unit] − H[γ_unit⁻¹])** — antisymmetric, PRIMARY observable. This is what Halcyon's POSITIVE / NULL / AMBIGUOUS gate reads. Carries the geometric signal.
- **H_sys = ½(H[γ_unit] + H[γ_unit⁻¹])** — symmetric, systematic-offset DIAGNOSTIC. Lands in the sidecar receipt as a systematic-offset audit trail. Non-zero `H_sys` is evidence the apparatus has a baseline drift that is being read as signal; it does NOT invalidate H_geom (the antisymmetric component is the signal), but it is recorded.

The `LoopTransportDiagnostics` struct shape:

```rust
pub struct LoopTransportDiagnostics {
    pub h_geom: GroupElement,           // antisymmetric primary observable
    pub h_sys: GroupElement,            // symmetric diagnostic
    pub per_seed_h_forward: Vec<GroupElement>,
    pub per_seed_h_reversed: Vec<GroupElement>,
    pub sigma_h_blocked: f64,
    pub tracking_error_max_q: f64,
    pub tracking_error_max_beta_w: f64,
    pub adiabaticity_check: AdiabaticityResult,
}
```

The two source measurements are `COMPUTE HOLONOMY_FORWARD` and `COMPUTE HOLONOMY_REVERSED` per the v3.1.3 grammar. The substrate forms `h_geom` and `h_sys` at the response boundary by calling the same `walk_loop` machinery against `gamma_unit` (forward) and against the time-reversed traversal of `gamma_unit` (reversed), then taking the two combinations. The Halcyon Python orchestrator (per v3.1.3 §4.6 thin-wrapper contract) reads `h_geom` and applies the §3 gate; `h_sys` lands in the sidecar receipt.

Not an external option. Not a query-side derivation. The verb's response object carries both. S₄'s reversed-loop sham assertion is satisfied by construction because the substrate's own primary observable embeds the reversal — no external flag needed.

---

## 6. β_W ∈ [2.5, 3.0] validation at parser

Per v3.1.3 §4.1, the validity window for the Wilson coupling on the CONTROL_MANIFOLD's second axis is `β_W ∈ [2.5, 3.0]`, strictly inside the SU(2) Q-observable regime that prior validation work trusts. The substrate validates this at declaration time (parser-level range check on the CONTROL_MANIFOLD axis specification). Any value outside the range returns a clean parser error of the shape:

```
β_W = 2.4 is outside the validated SU(2) Q-observable regime [2.5, 3.0].
Extension below β_W = 2.5 requires an independent SU(2) Q-tracking
validation at the proposed lower endpoint per v3.1.3 §4.1, and a
v3.1.x amendment with its own pre-registration.
```

Run-time selection of β_W outside `[2.5, 3.0]` is prohibited. Enforcement is substrate-side, not in the Halcyon Python orchestrator: the gate cannot be bypassed by passing the value differently. If Halcyon wants β_W < 2.5 in a future protocol, it comes back through the v3.1.x amendment door with new validation receipts attached.

**The convenient inheritance.** β_W = 2.5 is exactly the Halcyon canonical thermalization β (Sprint A locked value, the bit-identity contract chain anchored at that β). This means `LOOP_TRANSPORT` calls at β_W = 2.5 inherit:

- The same gauge regime that the existing Halcyon canonical chain operates in.
- The same SU(2) Q-tracking validation receipts.
- Bit-identity-compatible RNG state on shared seeds with the existing chain.

β_W = 3.0 is at the upper edge of the validated envelope. The loop's enclosed area Δβ_W = 0.5 stays inside the validated rectangle. The gate doc documents the lower-endpoint coincidence explicitly so future readers see why 2.5 is not arbitrary.

No tunable widening. The Δβ_W = 0.5 traversal is what v3.1.3 commits to; substrate honors it without offering a knob.

---

## 7. Cross-cutting CC-LT refresh

v1 §6 named six cross-cutting design questions. Halcyon's 2026-06-21 reply resolved them; v2 records resolutions and notes two new substrate-side pins (CC-LT-7, CC-LT-8) that fall out of v3.1.3's structure.

- **CC-LT-1 — Loop declarability. RESOLVED.** Halcyon's §C.1 chose path (a): first-class `DECLARE LOOP` statement with a `LoopRegistry` mirroring `GaugeFieldRegistry`. Substrate accepts. Minimal v0.1 plumbing (handle resolution, WAL emission) ships with the verb; richer parametric-path grammar deferred to v0.2. v3.1.3's two declared loops (`gamma_unit` rectangular in (Q, β_W); `gamma_degenerate` zero-area) are the v0.1 consumers. `gamma_unit⁻¹` is NOT separately declared — see CC-LT-7.
- **CC-LT-2 — Parameter-pack registry. RESOLVED.** Halcyon's §C.2 adopted `ParameterPackKind::Halcyon { alpha, tau_0, beta_tau, mu_baseline, K_spring, c_damp, drive_omega, drive_F0, pin_lambda_Q, pin_lambda_beta_W, eps_Q, eps_beta_W }` as the first registered variant. v2 confirms; the canonical field list lands verbatim in `src/gauge/loop_transport.rs`. `drive_omega` and `drive_F0` are INSIDE the Halcyon variant per Halcyon's §B.2 correction (v1 had them kind-agnostic). Future variants (StaggeredFermion, ShidokuQuotient, AURORA-Atmospheric) plug in as separate enum variants.
- **CC-LT-3 — Adiabaticity threshold. REFINED.** Halcyon's §C.3 split the original question into two observables. Observable A (`τ_pin / T_segment` ratio, the active-pinning equilibration check) is gated at 0.1 by v3.1.3 §4.2; the threshold lives in Halcyon's SPEC and is applied OUTSIDE the substrate — substrate emits the f64 ratio, Halcyon's Python applies the `>= 0.1` comparison. Observable B (`ramp_rate / gauge_relaxation_rate`, the t013 three-constraint diagnostic) is emitted as a diagnostic, NOT pre-registered with a numerical threshold in v3.1.3. Substrate uses candidate (a) `||dU/dt||_op = g² · sup_edges ||E_edge||` for Observable B per Halcyon's deferral. No κ knob inside the substrate; both observables flow through as analytical quantities with no operator tolerance. v2 commits to the ADIABATICITY_CHECK gate per v3.1.3 §4.2: violation of `τ_pin << T_segment` at runtime forces AMBIGUOUS regardless of H values. Substrate-side enforcement, not a prose target.
- **CC-LT-4 — Integrator reuse vs duplication. RESOLVED.** Halcyon's §C.4 confirmed duplication for v0.1. Sprint B revert lesson honored; `src/gauge/symplectic_flow.rs:294-330` body stays untouched on the `LOOP_TRANSPORT` introduction commit. Extraction is a follow-up commit gated by a third consumer (e.g., a staggered-fermion `LOOP_TRANSPORT` peer materializing).
- **CC-LT-5 — Name collision resolution. RESOLVED.** Halcyon's §B.1 adopted `LOOP_TRANSPORT` verbatim. Path (b) (target-type dispatcher on `ALONG_LOOP CONTROL_MANIFOLD`, which v3.1.3 SPEC grammar uses textually) is acknowledged as the alternative we would have built; we read the SPEC grammar through Halcyon's rename and proceed with path (a). Name collision with `src/geometry/sample_transport.rs` resolved by separate keyword, no overload.
- **CC-LT-6 — Sham-flag API shape. RESOLVED.** Halcyon's §C.6 confirmed nested `SHAM { ... }` block over top-level keywords. v2 reflects the five-sham disambiguation: four explicit flags (flat_field, alpha_zero, mass_scale, degenerate_loop / BACKTRACK_LOOP, frozen_field) plus S₄ absorbed into the H_geom return-shape. `SHAM_DEGENERATE_LOOP` disambiguation per Halcyon's §D.1: substrate ships `SHAM_BACKTRACK_LOOP` (canonical S₅ mapping), `SHAM_EMPTY_LOOP` (GC₄ companion), `SHAM_OPEN_LOOP` (parser-rejection test). v3.1.3's required science-gate sham set uses only BACKTRACK_LOOP; the other two are substrate-internal audit-story flags.
- **CC-LT-7 (new) — Loop time-reversal mechanism.** Substrate computes `γ_unit⁻¹` by traversing `gamma_unit` time-reversed in the executor, NOT by declaring a second loop in the registry. This is what makes the H_geom antisymmetric primary observable cheap (no second `DECLARE LOOP` statement, single loop registration, two walks). Pinned in the gate doc so the WAL replay reads cleanly: one `DeclareLoop` entry per logical loop, with the reverse-traversal as an inline option on the `LoopTransport` WAL entry.
- **CC-LT-8 (new) — Per-axis `ramp_rate` shape.** Per Halcyon's §D.4 and v3.1.3 grammar (`RAMP_RATE_Q 0.04`, `RAMP_RATE_BETA_W 0.01`), `ramp_rate` is per-axis, NOT scalar. Substrate ships `enum LoopShape { PiecewiseLinear { vertices: Vec<(f64, f64)>, t_per_segment: f64 } }` with v3.1.3's four-segment rectangle as the v0.1 consumer. Executor enumerates segments and applies the appropriate `(dQ/dt, dβ_W/dt)` per segment. Future curved-loop variants (`Circular`, `BezierClosed`) plug in later.

---

## 8. What stays unchanged from v1

The following commitments from v1 stand verbatim. v2 does not relitigate them:

- **Generalization framing.** `LOOP_TRANSPORT` is a peer to `SYMPLECTIC_FLOW` under the gauge module, not Halcyon-specific. Halcyon registers as the first `ParameterPackKind` variant. Future apparatuses (staggered fermions, shidoku-quotient holonomy, AURORA-atmospheric driven loops) plug in as additional variants. The verb signature stays clean of Halcyon-domain identifiers (per v1 §7).
- **Two-clocks methodology.** Substrate timeline does not move the pre-registration; pre-registration does not move the substrate timeline. Both clocks ran during the v3.0 → v3.1.3 chain; neither interfered with the other. v2 sharpens the framing because v3.1.3 names the dual-observable structure (H_geom / H_sys) and the substrate-side acceptance battery (GC₁–GC₆) more crisply than v1 had access to.
- **"What we are not asking back" stance.** No turnaround date. No v3 outcome guarantee. No guarantee the verb will pass v3.1.3's protocol. The verb gets built to the substrate's three-constraint definition; whether Halcyon's physics passes through it is the experiment.
- **Substrate persistence bug transparency.** v1 §opener noted the parallel-WAL-replay regression that lost `claude_substrate_v0` (the revert of `8912e3c`). Halcyon's §G acknowledged it without requiring relitigation; gauge primitives went through a separate code path, Part IV gold gates pass byte-identically against HEAD. v2 does not reopen.
- **Methodological discipline framing.** Every cross-cutting design question is "pin before LOC lands" rather than "iterate in the implementation log." v3.1.3's five-round review chain reinforces this — pre-deposit review is where defects get caught, not after deposit.

---

## 9. What is still committed

Updated for v3.1.3:

- The verb ships with the gate doc `theory/halcyon/HALCYON_PART_VI_GATES.md` on disk BEFORE the implementation commit lands. Gate doc includes:
  - GC₁–GC₆ acceptance battery table with the science-value gate before any implementation LOC.
  - β_W ∈ [2.5, 3.0] parser validation rule and the convenient inheritance from Halcyon canonical β = 2.5.
  - Loop time-reversal mechanism (CC-LT-7): one `DECLARE LOOP` per logical loop, reverse-traversal inline.
  - Per-axis `ramp_rate` schema (CC-LT-8): `LoopShape::PiecewiseLinear` with v3.1.3's four-segment rectangle.
  - `LoopTransportDiagnostics` struct shape with the tuple return.
  - ADIABATICITY_CHECK gate per v3.1.3 §4.2: violation forces AMBIGUOUS.
- Bit-identity contracts on IV.10 / III.8b / V.* stay locked. Any LOC that touches their hot paths gets its own commit, separate from the `LOOP_TRANSPORT` introduction.
- Cross-cutting questions CC-LT-1 through CC-LT-8 are answered in writing (here and in Halcyon's 2026-06-21 reply) before the parser arm or executor body lands.
- GC₁–GC₆ green is the gate for the verb's introduction. The 1373-assertion regression suite green is necessary but not sufficient.
- The substrate does not negotiate the GC₅ 1% science-value threshold. If the verb cannot meet it, the verb is patched (or N is moved by a v3.1.x amendment from Halcyon), not the threshold.

---

## 10. Pre-registration acknowledgment (updated)

**Canonical pre-registration:** `HALCYON_FALSIFICATION_BATTERY_SPEC_v3.1.3.md` at commit `44c70b1`, plus the Zenodo DOI when minted.

Whatever the v3.1.3 SPEC's falsification criteria say at that hash is what Halcyon will accept as a result — independent of when the substrate ships the verb, independent of the verb's parser surface, independent of how CC-LT-1 through CC-LT-8 resolved on the substrate side.

The substrate timeline does not move the pre-registration. The pre-registration does not move the substrate timeline. They are two clocks, locked separately. The five-round review chain (Gigi's methodological intervention, then four pre-deposit technical review rounds) is the discipline that makes the pre-registration meaningful: each pass caught real defects that a one-pass pre-registration would have locked in. That is pre-registration's intended property working as designed.

v1 said the discipline was the reason every design question in the letter was pinned before LOC. v2 says the same, and the five-round chain reinforces rather than weakens that posture.

Pushback welcome on every clause in this letter. v2 supersedes v1 in the five places §1 names; v1 stays in git history.

---

—Bee + Claude
