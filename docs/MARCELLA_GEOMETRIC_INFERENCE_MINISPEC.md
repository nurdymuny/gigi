# Marcella: Verified Geometric Inference

**Version:** 0.1 | **Date:** 2026-09-15 | **Status:** Proposed, not implemented

**Purpose:** Let GIGI derive previously unstored answers from explicit relationships,
and let Marcella explain those answers without asking her small language model to
invent or certify the mathematics. This is a bounded first step toward a geometric
reasoning system, not a claim of general intelligence or arbitrary theorem proving.

**Release rule:** Red test -> implementation -> independent verification -> useful
multi-domain live example -> commit. No implementation gate is closed by this document.

## 1. First Useful Product

Marcella can answer: "Given these records and these relationships, what follows?"
She can also explain dependencies, identify insufficient evidence, and recompute
an explicitly hypothetical change without rewriting measured facts.

V1 supports finite graphs of numeric scalar variables, identity/scaling restriction
maps, and either exact relationships or an explicitly selected harmonic model.
Variables have stable record/field IDs, units, scope, and evidence provenance.
The graph can contain paths, cycles, branches, and multiple observed boundaries.
No domain-specific solver branches are permitted.

V1 does **not** infer arbitrary laws from prose, average embeddings into factual
claims, silently relax constraints, or support nonlinear/discrete constraints.
Affine offsets, sums across several variables, vector stalks, and mixed hard/soft
models require later typed operators and their own gates. Existing SUDOKU is not
assumed equivalent to the linear solver.

## 2. Existing Components and Gaps

These are source observations, not a live deployment certification:

| Component | Reuse and required change |
|---|---|
| [GIGI sheaf solver](../src/sheaf/laplacian.rs) | Builds a scalar sheaf Laplacian and solves missing values. Add verified status, rank/nullspace, residual, and conditioning contracts; reuse the existing numerical library. |
| [GIGI COMPLETE](../src/sheaf/mod.rs) | Supplies schema adjacency and provenance patterns. Its per-target local completion is not automatically a joint, multi-step derivation API. |
| Marcella retrieval and grounding (`staircase/marcella_cli.py`, in the Marcella repository) | Reuse `gigi_attend`, stable IDs, hydration, claim planning, and deterministic rendering. Insert a checked derivation stage before factual composition. |
| Qwen mouth (`staircase/modal_mouth.py`, in the Marcella repository) | Consumes messages; no weight changes required. It may propose plans and wording, but cannot authorize a mathematical result. |

Important distinctions before reuse:

- `check_h1_local` currently uses median/MAD outlier screening. It is not a computed
  cohomological obstruction certificate. An exact derivation must not discard a
  contradictory observation because it is an outlier.
- The singular solver path can report both pseudoinverse completions and
  undetermined directions. A returned number alone is not evidence of uniqueness.
- The current `1 / (1 + inverse_diagonal)` confidence is a structural score, not a
  calibrated probability that a fact is true. Do not verbalize it as one.
- Current Marcella guards expect numbers/formulas to occur in cited evidence.
  A new numeric result therefore needs a checked derivation record, not a bypass.

Older specifications, including [GIGI's completion proposal](specs/gigi_sheaf_completion_spec.md)
and Marcella's meaning-mapping experiment (`geometric_meaning_mapping_spec.md`, in the Marcella repository),
provide research context. Their broader claims are not acceptance evidence here.

## 3. Mathematical Contract

For an oriented edge from variable `u` to `v`, an approved rule supplies a scalar
restriction coefficient `a_e`, with residual

\[
r_e(x)=a_e x_u-x_v,\qquad E(x)=\tfrac12\sum_e w_e r_e(x)^2
          =\tfrac12 x^T L x,\quad L=\delta^T W\delta.
\]

Require finite coefficients, positive finite weights, and declared unit conversions.
Weights affect the modeled penalty, not the truth or applicability of a rule.
Zero coefficients are allowed only when explicitly meaningful; they can leave
directions unconstrained. Never infer uniqueness merely from graph connectivity.

Partition observed values `y` and missing values `z`. Write
`delta x = A z - b`, where `A = delta_m` and `b = -delta_o y`.

**Exact mode:** Solve `A z = b`. Report `derived` only when the equations are
consistent and the requested target is determined. For a target `c^T z`, this means

\[
c^T n=0\quad\text{for every }n\in\ker A.
\]

Full column rank is sufficient for all missing variables to be determined.
If a relevant component is inconsistent, return `inconsistent`; if the target
varies along a null direction, return `underdetermined`. A pseudoinverse's preferred
representative must not masquerade as a uniquely implied value.

**Estimate mode:** Solve the weighted minimum-energy problem. When `L_mm` is
positive definite,

\[
L_{mm}\hat z=-L_{mo}y.
\]

This is the existing Schur/block-solve mechanism. A unique minimum is a unique
answer **under the harmonic model**, not proof of the unmeasured physical value.
Even a zero-residual fit remains `estimated` when the relationships are only
modeling assumptions. Do not add a ridge or switch modes silently to force a result.

Use rank-revealing QR/SVD for rank and nullspace, and stable solves rather than
forming an inverse for the answer. After unit/scale normalization, record the
rank threshold and normalized residual

\[
\eta=\frac{\|A\hat z-b\|_2}{\|A\|_2\|\hat z\|_2+\|b\|_2+\epsilon}.
\]

Register numerical tolerances before evaluating; return `numerically_unresolved`
for tolerance-sensitive rank or unacceptable conditioning, not a confident number.
Rational toy fixtures use an independent exact oracle; production float results
are numerical derivations within stated tolerances, not formal proofs.

If measured input covariance `Sigma_y` is supplied and maps are treated as fixed,
the linear estimate `z = J y` has propagated covariance `J Sigma_y J^T`, with
`J = -L_mm^{-1} L_mo`. Otherwise return uncertainty unavailable with a reason.
This formula excludes uncertainty in the rules and model mismatch. Preserve shared
source dependencies when composing results; derived values are not new independent
measurements.

The sheaf gluing axiom alone does not guarantee recovery across arbitrary holes.
The harmonic-extension assumptions above are consistent with the established
[cellular-sheaf treatment, Section 4.1](https://arxiv.org/html/1808.01513#S4.SS1).

## 4. Question-to-Answer Pipeline

```text
Question + visible conversation
  -> existing intent/permission gate and canonical evidence retrieval
  -> candidate typed plan from a bounded rule registry
  -> deterministic validation and explicit clarification when needed
  -> GIGI joint solve + independently checked derivation receipt
  -> typed derived evidence + existing factual evidence
  -> verified claim plan -> deterministic factual rendering
```

The **rule registry** is reviewed data: rule ID/version, source citation, input and
output types/units, applicability conditions, map direction, exact/model status,
and permitted scenario parameters. Similarity can find candidate rules; it cannot
authorize their use. Domain changes belong in registry/schema data, never solver code.

The planner selects IDs and binds entities. It cannot submit executable code,
invent coefficients, promote a conjecture, or label its own output verified.
Every binding must have an evidence field or explicit question span. Initially
use controlled request templates; ambiguous free-form mappings require the user's
confirmation. Type correctness alone does not prove faithful interpretation.

V1 composition collects all applicable constraints into one joint solve instead
of treating intermediate answers as independent evidence. Across turns, retain a
versioned plan and its dependency graph. A hypothetical edit gets a separate
scenario ID and explicitly identified overridden rules/observations; never silently
remove conflicting descendants or change the canonical bundle.

## 5. Engineering Boundaries

Proposed modules: GIGI `src/sheaf/derivation.rs`; Marcella
`staircase/marcella_derivation.py`; focused tests beside existing grounding tests.
Names are planned, not existing APIs. Begin with a library interface; the planned
HTTP surface is `POST /v1/brain/derive`, introduced only with route/auth/wire gates.

| Contract | Required content |
|---|---|
| `DerivationPlan` | Schema version, request/scenario ID, mode, targets as stable record/field refs, pinned source versions, registry rule IDs/versions, typed bindings, explicit overrides, resource limits. |
| Server validation | Re-read authorized evidence and rules. Reject unknown fields, missing observed inputs, nonfinite numbers, bad units/directions, expired snapshots, inapplicable rules, and forged derived inputs. Missing targets are expected. Qwen does not supply trusted matrices. |
| `DerivationResult` | Per-target `derived`, `estimated`, `underdetermined`, `inconsistent`, or `numerically_unresolved`; value/unit only when justified; source/rule dependencies, residual, rank/nullity, conditioning, numerical tolerances, uncertainty disclosures. |
| `DerivationReceipt` | Canonical problem hash, snapshot and registry versions, algorithm version, relevant equations, solution/witness, status evidence, and replay instructions. Numerical certificate, not proof of the source premises. |
| Other outcomes | `needs_clarification`, `unsupported`, and `resource_limit` are controller outcomes, not factual answers. Malformed input is HTTP 400, stale versions 409, invalid numerical/type requirements 422. Valid underdetermination/inconsistency returns a structured result. |

Read a consistent bounded snapshot. For several bundles, capture a version vector
under appropriate read locks or immutable snapshots; never mix revisions. Cache
only by canonical problem, scenario, registry, and snapshot versions. Any dependency
change invalidates the result. A hash identifies content; it is not authorization.

The trusted adapter creates `derived_numeric` evidence only after replay validation.
Its deterministic text includes the result, units, assumptions, result kind, and
receipt ID. Extend role validation explicitly; never relabel it `measured` or
`validated_math`. Persist separately only when requested; do not auto-ingest
generated answers as training facts or independent future evidence.

Extend `verify_claim_plan` to check the receipt, exact numeric/unit payload,
scenario, and required qualification. An estimate must remain an estimate in the
answer; an NLI pass cannot upgrade it. Initially render the derived factual clauses
deterministically. Qwen may compose surrounding language, but any changed fact,
unit, negation, or qualification fails verification. Existing factual guards stay.

Opt-in `MARCELLA_GEOMETRIC_INFERENCE=off|shadow|on`, default `off`. Shadow records
proposals without changing answers. Keep existing routes and unrelated behavior
unchanged; do not weaken the intent gate to increase answer counts. Source-role
isolation, prompt-injection defenses, authorization, and read-only defaults apply.

## 6. Resource-Conscious Gates

Suggested initial caps: 128 scalar variables, 512 edges, at most 4 source bundles,
one plan per turn, one clarification/replan, and a configurable wall-time limit.
Dense linear algebra is acceptable at this bounded scale; benchmark before choosing
a production timeout. No world-scale ingestion, new embeddings, or training run
is a prerequisite. Tests use local records and a stub mouth until the final gate.

| Gate | Evidence required before closing |
|---|---|
| G0: Contracts | Exact, inconsistent, singular, and harmonic fixtures with predetermined outcomes. Receipt tampering and invalid plans must fail. All initially red. |
| G1: Solver | Independent rational/QR reference; multi-unknown graphs, nonidentity direction checks, cycles, determined targets inside singular systems, zero/negative values, unit changes, near-singularity. Every valid exact fixture correct; every invalid claim refused. |
| G2: Service | Fresh local binary on `127.0.0.1`, scratch data, actual endpoint round trip. Check auth, refusal bodies, bounded execution, snapshot races, receipt replay, and mutation invalidation. Do not infer success from an empty HTTP 200. |
| G3: Marcella | Binding/paraphrase/negation/ambiguity tests; derived-number guards, conditional scenarios, source roles, multi-turn revisions, unchanged legacy grounding. Same Qwen in all comparisons. |
| G4: Product | Complex worked example below, independently labeled held-out cases, live Qwen and GIGI results, raw receipts and costs. No-go if usefulness is absent, even when unit tests pass. |

For every claimed mechanism, deliberately remove or corrupt it and confirm its
gate fails. Do not mutate the working engine destructively: use fixture controls
or an isolated test variant. Two permissible outcomes require two distinct fixtures.
Before an implementation commit run all-target `cargo test`, also the supported
`kahler` configuration, and Marcella's full relevant pytest suite. Rebuild/restart
the scratch server before live probes. This document does not report those runs.

## 7. Final Worked Example: Missing Operational Quantities

First, an exact, inspectable inventory chain whose result is absent from storage:

```text
Observed: 12 pallets; independent manifest: 288 cases.
Approved: 24 cases/pallet, 6 kits/case, 2 units/kit, 48 units/dispatch carton.
Missing: kits, units, dispatch cartons.
Derived: 1,728 kits -> 3,456 units -> 72 dispatch cartons.
```

Ask for cartons and supporting records, then ask what changes if each kit contains
3 units: the separate scenario yields 108 cartons. The rule change must be
explicitly hypothetical, with no changes to the recorded 12 pallets or 288 cases.
A contradictory 3,408-unit audit must produce `inconsistent`, not an average.
Removing the kits-to-units relationship, with no alternate path, must make cartons
`underdetermined`. Renaming records or reordering them must not change the answer.

The release example expands to 12 facilities x 30 daily snapshots across packaging,
manifest, calibration, and rule bundles, with conversions varying by facility/date,
redundant paths, missing links, conflicting audits, and stale revisions. Run bounded
local solves, not one artificially unlimited dense graph. Report a per-facility
decision table: derived result, explanation, or the exact missing/conflicting premise.

Prove generality using the **same planner/compiler/solver/receipt code** on separately
authored instrument-calibration and data-transfer accounting fixtures. Use differing
graph structures and rule factors, not merely relabeled inventory. Fixtures must
declare measurement models and units honestly; do not invent physical facts to
make a desired answer appear. Small planted fixtures verify mechanisms; the larger
example demonstrates an operational benefit, not a claim of field validation.

Pre-register a held-out 300-turn suite: 100 per domain, including at least 20
refusal/clarification cases per domain and held-out entity names, graph structures,
values, and human-authored paraphrases. Freeze thresholds before seeing results:

- At least 95% correct end-to-end answers on answerable turns, including units,
  provenance, and required qualifications; refusals count as misses here.
- Correct refusal/clarification on every designated negative case, no unsupported
  numerical claims, and replayable receipts for every emitted derivation.
- At least 10 percentage points more verified answer coverage than the existing
  retrieval-plus-same-Qwen baseline. Report paired results and uncertainty, not just
  a total score; these targets are release choices, not established performance.
- Separate gold-plan solver accuracy from natural-language plan accuracy. Include
  a same-Qwen oracle-evidence control to isolate retrieval from composition errors,
  and a no-completion ablation to prove that the mechanism causes the improvement.

Publish latency, memory, model calls/tokens, refusals, and error categories. Model
judging alone cannot close the gate: numerical truth comes from independent oracles,
and language bindings/qualifications from reviewed labels. Zero observed failures
in this finite suite is not a universal reliability guarantee.

## 8. Later Research, Not V1 Dependencies

Only after the bounded feature proves useful: introduce richer typed operators,
learned proposal/rule discovery with approval, and open-ended question-to-sheaf
construction. Evaluate new operators on novel compositions, not only interpolation.
This is the unresolved general-reasoning research boundary.

A portable condensed bundle can follow separately. For a fixed Laplacian with an
invertible interior block, eliminating interior variables gives a Schur-complement
boundary operator preserving the minimized boundary energy/response; interior
recovery needs the extension map. It does not
preserve arbitrary queries, and a reduced PSD operator need not remain representable
by the current scalar-edge schema. Preserve premises and receipt dependencies;
validate condensation under its own equivalence gate. It is not needed to begin.

**First action when resources allow:** implement G0 and G1 locally with a stub mouth.
Do not spend on training or a large corpus until the typed derivation and refusal
contracts work. The initial product claim is narrow and testable: Marcella can state
a new, checked consequence of explicit records and rules without neural retraining.
