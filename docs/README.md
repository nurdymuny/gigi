# Documentation map

Where everything in this repository lives. The engine itself is `src/`; nothing in `docs/` is part of the build.

| folder | what is in it |
| --- | --- |
| [`research/`](research/README.md) | **Research Notes**: every note, pre-registration and result, by thread and date |
| [`../research/`](../research/README.md) | Receipts: the raw outputs behind the notes, one dated folder per run |
| [`papers/`](papers/) | Paper sources: berger-spectrum (the Floor paper), by-enough (the edge paper), two-ring-pg, chord-arc-tangent |
| [`reference/`](reference/) | Reference material for using GIGI (also `GQL_REFERENCE.md` and `GIGI_API.md` at the repository root) |
| [`specs/`](specs/) | Engine and product specifications, sprint plans and roadmaps |
| [`operations/`](operations/) | Deployment, incidents, security review, audits, migrations |
| [`correspondence/`](correspondence/) | Letters, replies, handoffs and briefings |
| [`patents/`](patents/) | The GIGI provisional patent draft |
| [`../theory/`](../theory/) | Theory papers and long-running programmes (Halcyon, Kähler, Aurora, ...); its own index is [`theory/SPECS_INDEX.md`](../theory/SPECS_INDEX.md) |

## Using GIGI

| file | title |
| --- | --- |
| [`GETTING_STARTED.md`](GETTING_STARTED.md) | Getting started with GIGI |
| [`HTTP_API_REFERENCE.md`](HTTP_API_REFERENCE.md) | GIGI HTTP API Reference |
| [`CONSUMER_PATTERNS.md`](CONSUMER_PATTERNS.md) | Consumer patterns — building an LLM/agent on top of GIGI |
| [`CREATE_SESSION.md`](CREATE_SESSION.md) | CREATE SESSION |
| [`SHAPE_VERBS.md`](SHAPE_VERBS.md) | The shape verbs — TEXTURE, PRECEDENCE, CADENCE |
| [`STABILITY_GUARANTEES.md`](STABILITY_GUARANTEES.md) | gigi Stability Guarantees |
| [`GIGI_HOSTING_ITSELF.md`](GIGI_HOSTING_ITSELF.md) | GIGI hosting itself — the `__bundles__` virtual bundle |
| [`GQL_REFERENCE.md`](../GQL_REFERENCE.md) | GQL reference (repository root) |
| [`GIGI_API.md`](../GIGI_API.md) | HTTP API (repository root) |
| [`EXTRACTION_MAP.md`](../EXTRACTION_MAP.md) | Engine module map (repository root; source files point at it) |
| [`BENCHMARKS_VS_STORES.md`](reference/BENCHMARKS_VS_STORES.md) | GIGI vs SQLite vs DuckDB — the vs_stores benchmark (2026-07-31) |
| [`BRAIN_PRIMITIVES_CONSUMER_GUIDE.md`](reference/BRAIN_PRIMITIVES_CONSUMER_GUIDE.md) | Brain primitives — consumer guide |
| [`GIGI_PERF_ANALYSIS.md`](reference/GIGI_PERF_ANALYSIS.md) | GQL Performance Analysis — Marcella Fiber Fidelity Run |
| [`GQL_ADDENDUM_v2.1.md`](reference/GQL_ADDENDUM_v2.1.md) | GQL v2.1 Addendum — PostgreSQL Parity Features |
| [`GQL_SPECIFICATION.md`](reference/GQL_SPECIFICATION.md) | GQL — Geometric Query Language |
| [`VALIDATION.md`](reference/VALIDATION.md) | VALIDATION — the numeric validation artifacts, indexed |

## Engine research specifications and results (in `docs/`)

| file | title |
| --- | --- |
| [`DAVIS_CONJECTURE_LAMBDA_RIDEALONG.md`](DAVIS_CONJECTURE_LAMBDA_RIDEALONG.md) | Davis Conjecture λ-Budget Ride-Along |
| [`MONODROMY_GROUP_ORDER_RESULTS.md`](MONODROMY_GROUP_ORDER_RESULTS.md) | MONODROMY_GROUP_ORDER: 120 versus 1 |
| [`MONODROMY_GROUP_ORDER_SPEC.md`](MONODROMY_GROUP_ORDER_SPEC.md) | MONODROMY_GROUP_ORDER: bounded SU(2) closure |
| [`RECOVER_CURVATURE_SPEC.md`](RECOVER_CURVATURE_SPEC.md) | RECOVER_CURVATURE: spec |
| [`RECOVER_CURVATURE_SU2_RESULTS.md`](RECOVER_CURVATURE_SU2_RESULTS.md) | RECOVER_CURVATURE for SU(2), step S1: results |
| [`RECOVER_CURVATURE_SU2_SPEC.md`](RECOVER_CURVATURE_SU2_SPEC.md) | RECOVER_CURVATURE for SU(2): the covariant two-ring recovery (spec) |
| [`RESEARCH_RELATIONSHIP_REGISTRY_PROPOSAL.md`](RESEARCH_RELATIONSHIP_REGISTRY_PROPOSAL.md) | Proposal: typed research relationships with evidence |
| [`SETTLING_APPLICABILITY_RESULTS.md`](SETTLING_APPLICABILITY_RESULTS.md) | Settling applicability: evidence without overclaiming |
| [`SETTLING_APPLICABILITY_SPEC.md`](SETTLING_APPLICABILITY_SPEC.md) | Settling applicability: smallest justified fix |
| [`SPHERE_TRANSPORT_RESULTS.md`](SPHERE_TRANSPORT_RESULTS.md) | Checked sphere transport: results and consumer handoff |
| [`SPHERE_TRANSPORT_SPEC.md`](SPHERE_TRANSPORT_SPEC.md) | Checked sphere transport: GIGI-side DTP follow-up |

## The Ricci reconstruction series

Paired specifications and results, in `docs/`. Published papers cite these paths, so they stay where they are.

| file | title |
| --- | --- |
| [`RICCI_ADJOINT_CONSISTENT_RECOVERY_SPEC.md`](RICCI_ADJOINT_CONSISTENT_RECOVERY_SPEC.md) | RF2H: adjoint-consistent curvature recovery (spec before implementation) |
| [`RICCI_BOUNDARY_EXTENSION_RESULTS.md`](RICCI_BOUNDARY_EXTENSION_RESULTS.md) | Boundary extension: verified incompatibility with the fixed exterior |
| [`RICCI_BOUNDARY_EXTENSION_SPEC.md`](RICCI_BOUNDARY_EXTENSION_SPEC.md) | Smallest boundary extension, fixed six-cell patch |
| [`RICCI_CURVATURE_RECONSTRUCTION_SPEC.md`](RICCI_CURVATURE_RECONSTRUCTION_SPEC.md) | RF2E: curvature reconstruction and its actual discrete adjoint |
| [`RICCI_FINAL_BOUNDED_ATTEMPT.md`](RICCI_FINAL_BOUNDED_ATTEMPT.md) | RF2G: research synthesis and one bounded final attempt |
| [`RICCI_FINAL_BOUNDED_RESULTS.md`](RICCI_FINAL_BOUNDED_RESULTS.md) | RF2G: final bounded attempt, verified NO-GO |
| [`RICCI_FIXED_EXTERIOR_ASSESSMENT_2026_11_17.md`](RICCI_FIXED_EXTERIOR_ASSESSMENT_2026_11_17.md) | Independent assessment: the fixed-exterior obstruction |
| [`RICCI_FLOW_SU2_RESULTS.md`](RICCI_FLOW_SU2_RESULTS.md) | Native Ricci flow: RF1 evidence |
| [`RICCI_FLOW_SU2_SPEC.md`](RICCI_FLOW_SU2_SPEC.md) | Ricci flow on homogeneous three-manifolds: RF1 |
| [`RICCI_HANDOFF.md`](RICCI_HANDOFF.md) | Ricci / Davis-Wilson lattice correspondence: PARKED |
| [`RICCI_HANDOFF_2026_11_17.md`](RICCI_HANDOFF_2026_11_17.md) | Claude handoff: GIGI / Davis-Wilson / Ricci correspondence |
| [`RICCI_INDEPENDENT_ASSESSMENT_2026_11_17.md`](RICCI_INDEPENDENT_ASSESSMENT_2026_11_17.md) | Independent assessment and one bounded spec |
| [`RICCI_LATTICE_CORRESPONDENCE_RESULTS.md`](RICCI_LATTICE_CORRESPONDENCE_RESULTS.md) | RF2B: actual lattice / Ricci correspondence experiment |
| [`RICCI_LATTICE_CORRESPONDENCE_SPEC.md`](RICCI_LATTICE_CORRESPONDENCE_SPEC.md) | RF2B: Davis-Wilson lattice / Ricci correspondence |
| [`RICCI_LATTICE_RESIDUAL_ANALYSIS.md`](RICCI_LATTICE_RESIDUAL_ANALYSIS.md) | RF2B residual diagnosis and next controlled experiment |
| [`RICCI_LOCAL_PUSH_AUDIT.md`](RICCI_LOCAL_PUSH_AUDIT.md) | Bounded local push audit |
| [`RICCI_LOCAL_PUSH_INTERFACE_ADDENDUM.md`](RICCI_LOCAL_PUSH_INTERFACE_ADDENDUM.md) | Final bounded separation: cell interiors and shared faces |
| [`RICCI_LOCAL_PUSH_RESULTS.md`](RICCI_LOCAL_PUSH_RESULTS.md) | Local push audit: shared-face imbalance located |
| [`RICCI_OPERATOR_CONSISTENCY_RESULTS.md`](RICCI_OPERATOR_CONSISTENCY_RESULTS.md) | RF2D: operator consistency diagnosis |
| [`RICCI_OPERATOR_CONSISTENCY_SPEC.md`](RICCI_OPERATOR_CONSISTENCY_SPEC.md) | RF2D: locating the shape-regular operator discrepancy |
| [`RICCI_OPERATOR_SPECTRAL_CONTROL_ADDENDUM.md`](RICCI_OPERATOR_SPECTRAL_CONTROL_ADDENDUM.md) | RF2D spectral accounting addendum |
| [`RICCI_PG_CURVATURE_RESULTS.md`](RICCI_PG_CURVATURE_RESULTS.md) | Results: one reconstruction, both force routes, Abelian control |
| [`RICCI_PG_CURVATURE_SPEC.md`](RICCI_PG_CURVATURE_SPEC.md) | S1–S3: Petrov-Galerkin curvature test, Abelian control |
| [`RICCI_PG_HYBRID_SPEC.md`](RICCI_PG_HYBRID_SPEC.md) | The hybrid three-ring Petrov–Galerkin force: BW05 at n = 1, 2, 3, 5, stability and numerical range (spec) |
| [`RICCI_PG_TWORING_RESULTS.md`](RICCI_PG_TWORING_RESULTS.md) | Two-ring Petrov-Galerkin curvature: BW05 and S3 both pass (Abelian control) |
| [`RICCI_PROJECTION_LAYER_RESULTS.md`](RICCI_PROJECTION_LAYER_RESULTS.md) | Bee's magnet idea: the geometry behind it, and two probes |
| [`RICCI_RECONSTRUCTION_EXPLORATORY_STATUS.md`](RICCI_RECONSTRUCTION_EXPLORATORY_STATUS.md) | RF2E exploratory regularization: excluded from the authorized repair gate |
| [`RICCI_SHAPE_REGULAR_RESULTS.md`](RICCI_SHAPE_REGULAR_RESULTS.md) | RF2C: shape-regular mesh experiment |
| [`RICCI_SHAPE_REGULAR_SPEC.md`](RICCI_SHAPE_REGULAR_SPEC.md) | RF2C: mesh-only shape-regular refinement experiment |
| [`RICCI_SHARED_CURVATURE_PATCH_RESULTS.md`](RICCI_SHARED_CURVATURE_PATCH_RESULTS.md) | Shared curvature on the original six-cell patch |
| [`RICCI_SHARED_CURVATURE_PATCH_SPEC.md`](RICCI_SHARED_CURVATURE_PATCH_SPEC.md) | Smallest shared-curvature patch construction |
| [`RICCI_SHARED_FACE_CONSTRAINT_RESULTS.md`](RICCI_SHARED_FACE_CONSTRAINT_RESULTS.md) | Shared-face constraint: checker verified, original patch incompatible |
| [`RICCI_SHARED_FACE_CONSTRAINT_SPEC.md`](RICCI_SHARED_FACE_CONSTRAINT_SPEC.md) | Shared-face constraint: bounded diagnostic specification |
| [`RICCI_SU2_BATCHED_FORCE_RESULTS.md`](RICCI_SU2_BATCHED_FORCE_RESULTS.md) | The SU(2) force, batched across cells (results, CPU) |
| [`RICCI_SU2_BATCHED_FORCE_SPEC.md`](RICCI_SU2_BATCHED_FORCE_SPEC.md) | The SU(2) force, batched across cells (spec) |
| [`RICCI_SU2_BATCHED_JACOBIAN_RESULTS.md`](RICCI_SU2_BATCHED_JACOBIAN_RESULTS.md) | B5: the linearised SU(2) force (the matrix K), batched (results) |
| [`RICCI_SU2_BATCHED_JACOBIAN_SPEC.md`](RICCI_SU2_BATCHED_JACOBIAN_SPEC.md) | B5: the linearised SU(2) force (the matrix K), batched (spec) |
| [`RICCI_SU2_EXACT_DEFECT_DERIVATIVE_RESULTS.md`](RICCI_SU2_EXACT_DEFECT_DERIVATIVE_RESULTS.md) | The SU(2) force's defect derivative: exact, not finite-difference (results) |
| [`RICCI_SU2_EXACT_DEFECT_DERIVATIVE_SPEC.md`](RICCI_SU2_EXACT_DEFECT_DERIVATIVE_SPEC.md) | The SU(2) force's defect derivative: exact, not finite-difference (spec) |
| [`RICCI_SU2_EXACT_STABILITY_SPEC.md`](RICCI_SU2_EXACT_STABILITY_SPEC.md) | B6: stability and numerical range of the SU(2) forces from the exact-derivative K (spec) |
| [`RICCI_SU2_PG_FLOW_CAYLEY_RESULTS.md`](RICCI_SU2_PG_FLOW_CAYLEY_RESULTS.md) | S3b: the Cayley time step (results, n = 1, 2 and 3) |
| [`RICCI_SU2_PG_FLOW_CAYLEY_SPEC.md`](RICCI_SU2_PG_FLOW_CAYLEY_SPEC.md) | S3b: the Cayley time step, and the SU(2) flow at n = 3 (spec) |
| [`RICCI_SU2_PG_FLOW_FILTER_RESULTS.md`](RICCI_SU2_PG_FLOW_FILTER_RESULTS.md) | S3a: why the native force does not stall in time (results) |
| [`RICCI_SU2_PG_FLOW_FILTER_SPEC.md`](RICCI_SU2_PG_FLOW_FILTER_SPEC.md) | S3a: why the native force does not stall in time (spec) |
| [`RICCI_SU2_PG_FLOW_RESULTS.md`](RICCI_SU2_PG_FLOW_RESULTS.md) | S3: time integration of the SU(2) two-ring force (results) |
| [`RICCI_SU2_PG_FLOW_SPEC.md`](RICCI_SU2_PG_FLOW_SPEC.md) | S3: the SU(2) two-ring force in time, against the Ricci-flowed links (spec) |
| [`RICCI_SU2_PG_FLOW_STARTUP_RESULTS.md`](RICCI_SU2_PG_FLOW_STARTUP_RESULTS.md) | S3d: is the Cayley step's N = 4 failure undamped stiff modes? (results) |
| [`RICCI_SU2_PG_FLOW_STARTUP_SDIRK_RESULTS.md`](RICCI_SU2_PG_FLOW_STARTUP_SDIRK_RESULTS.md) | S3e: a second-order L-stable start-up for the Cayley step (results) |
| [`RICCI_SU2_PG_FLOW_STARTUP_SDIRK_SPEC.md`](RICCI_SU2_PG_FLOW_STARTUP_SDIRK_SPEC.md) | S3e: a second-order L-stable start-up (SDIRK2) for the Cayley step (spec) |
| [`RICCI_SU2_PG_FLOW_STARTUP_SPEC.md`](RICCI_SU2_PG_FLOW_STARTUP_SPEC.md) | S3d: does the Cayley step fail at N = 4 because it does not damp stiff modes? (spec) |
| [`RICCI_SU2_PG_FORCE_RESULTS.md`](RICCI_SU2_PG_FORCE_RESULTS.md) | S2a: the two-ring Petrov–Galerkin force for SU(2): results |
| [`RICCI_SU2_PG_FORCE_SPEC.md`](RICCI_SU2_PG_FORCE_SPEC.md) | S2a: the two-ring Petrov–Galerkin force for SU(2) (spec) |
| [`RICCI_SU2_PG_NUMRANGE_RESULTS.md`](RICCI_SU2_PG_NUMRANGE_RESULTS.md) | S2c: numerical range of the SU(2) two-ring force (results) |
| [`RICCI_SU2_PG_NUMRANGE_SPEC.md`](RICCI_SU2_PG_NUMRANGE_SPEC.md) | S2c: numerical range of the SU(2) two-ring force (spec) |
| [`RICCI_SU2_PG_SPECTRUM_N3_RESULTS.md`](RICCI_SU2_PG_SPECTRUM_N3_RESULTS.md) | S2d: the linearised SU(2) force at n = 3: spectrum and numerical range (results) |
| [`RICCI_SU2_PG_SPECTRUM_N3_SPEC.md`](RICCI_SU2_PG_SPECTRUM_N3_SPEC.md) | S2d: the linearised SU(2) force at n = 3: spectrum and numerical range (spec) |
| [`RICCI_SU2_PG_STABILITY_RESULTS.md`](RICCI_SU2_PG_STABILITY_RESULTS.md) | S2b: stability of the SU(2) two-ring force: results |
| [`RICCI_SU2_PG_STABILITY_SPEC.md`](RICCI_SU2_PG_STABILITY_SPEC.md) | S2b: stability of the SU(2) two-ring force (spec) |
| [`RICCI_VARIATIONAL_RECOVERY_RESULTS.md`](RICCI_VARIATIONAL_RECOVERY_RESULTS.md) | RF2F result: verified NO-GO for the first variational recovery |
| [`RICCI_VARIATIONAL_RECOVERY_SPEC.md`](RICCI_VARIATIONAL_RECOVERY_SPEC.md) | RF2F: compatible variational curvature recovery |
| [`RICCI_WILSON_CORRESPONDENCE_RESULTS.md`](RICCI_WILSON_CORRESPONDENCE_RESULTS.md) | RF2A: a specific Ricci / Yang-Mills correspondence |
| [`RICCI_WILSON_CORRESPONDENCE_SPEC.md`](RICCI_WILSON_CORRESPONDENCE_SPEC.md) | RF2A: test the Levi-Civita / Yang-Mills correspondence |
| [`RICCI_ZERO_MEAN_RESULTS.md`](RICCI_ZERO_MEAN_RESULTS.md) | Option 1: Whitney plus a zero-mean correction — diagnostic result |
| [`RING_FOLD_FORCE_RESULTS.md`](RING_FOLD_FORCE_RESULTS.md) | Fold-force: the Abelian force on folded stencils (results) |
| [`RING_FOLD_FORCE_SPEC.md`](RING_FOLD_FORCE_SPEC.md) | Fold-force: the Abelian force on folded stencils (spec) |
| [`RING_FOLD_RESULTS.md`](RING_FOLD_RESULTS.md) | Fold-Λ: the smallest stable piece of rim (results) |
| [`RING_FOLD_SPEC.md`](RING_FOLD_SPEC.md) | Fold-Λ: the smallest stable piece of rim (spec) |

## Specifications

| file | title |
| --- | --- |
| [`DOCUMENTATION_PLAN.md`](specs/DOCUMENTATION_PLAN.md) | GIGI Documentation Plan |
| [`GIGI_ANOMALY_DASHBOARD_SPEC.md`](specs/GIGI_ANOMALY_DASHBOARD_SPEC.md) | GIGI Anomaly Detection API + Live Dashboard |
| [`GIGI_AUTOMATIC_ANALYTICS_API.md`](specs/GIGI_AUTOMATIC_ANALYTICS_API.md) | GIGI Automatic Geometric Analytics API |
| [`GIGI_COHERENCE_EXTENSIONS_v0.1.md`](specs/GIGI_COHERENCE_EXTENSIONS_v0.1.md) | GIGI Coherence Extensions |
| [`GIGI_CRUD_ROADMAP.md`](specs/GIGI_CRUD_ROADMAP.md) | GIGI CRUD & Database Feature Roadmap |
| [`GIGI_ENCRYPT_v0.2_SPRINT_SPEC.md`](specs/GIGI_ENCRYPT_v0.2_SPRINT_SPEC.md) | GIGI Encrypt v0.2 — Full Surface Sprint Spec |
| [`GIGI_ENCRYPT_v0.3_SPRINT_SPEC.md`](specs/GIGI_ENCRYPT_v0.3_SPRINT_SPEC.md) | GIGI Encrypt v0.3 — Band 2 Engineering Sprint Spec |
| [`GIGI_GEOMETRIC_ENCRYPTION_SPEC.md`](specs/GIGI_GEOMETRIC_ENCRYPTION_SPEC.md) | GIGI Geometric Encryption |
| [`GIGI_LANG_SPEC.md`](specs/GIGI_LANG_SPEC.md) | GIGI Lang — Specification (High-Level Goals) |
| [`GIGI_OBSERVABILITY_SPEC.md`](specs/GIGI_OBSERVABILITY_SPEC.md) | GIGI Observability & Logging Specification |
| [`GIGI_PERSISTENCE_UPGRADE_SPEC.md`](specs/GIGI_PERSISTENCE_UPGRADE_SPEC.md) | GIGI Engine Upgrade: Persistent Bundle Storage |
| [`GIGI_POST_KAHLER_SPEC_v0.1.md`](specs/GIGI_POST_KAHLER_SPEC_v0.1.md) | GIGI Post-Kähler Extensions (Phase 1) |
| [`GIGI_PRODUCT_SPECS.md`](specs/GIGI_PRODUCT_SPECS.md) | GIGI Product Suite — Build Specifications |
| [`GIGI_SAMPLE_TRANSPORT_SPRINT_SPEC.md`](specs/GIGI_SAMPLE_TRANSPORT_SPRINT_SPEC.md) | GIGI Sample-Transport Sprint Spec |
| [`GIGI_SCHEMA_INTROSPECTION_SPEC.md`](specs/GIGI_SCHEMA_INTROSPECTION_SPEC.md) | GIGI Schema Introspection — Specification |
| [`GIGI_SHEETS_SPRINT_SPEC.md`](specs/GIGI_SHEETS_SPRINT_SPEC.md) | GIGI Sheets — Sprint Spec (v0.1) |
| [`GIGI_SHEETS_SPRINT_SPEC_ADDENDUM_v0.1.md`](specs/GIGI_SHEETS_SPRINT_SPEC_ADDENDUM_v0.1.md) | GIGI Sheets — Sprint Spec Addendum v0.1 (audit answers) |
| [`GIGI_SPEC_v0.1.md`](specs/GIGI_SPEC_v0.1.md) | GIGI Database Engine |
| [`GIGI_SUDOKU_SPRINT_SPEC.md`](specs/GIGI_SUDOKU_SPRINT_SPEC.md) | GIGI Sudoku Sprint — Feature Gap Analysis & Implementation Spec |
| [`GIGI_TETMESH_SPEC_v0.6.md`](specs/GIGI_TETMESH_SPEC_v0.6.md) | GIGI_TETMESH_SPEC_v0.6.md |
| [`GIGI_V2_FEATURE_SPEC.md`](specs/GIGI_V2_FEATURE_SPEC.md) | GIGI v2 Feature Specification |
| [`gigi_sheaf_completion_spec.md`](specs/gigi_sheaf_completion_spec.md) | GIGI Sheaf Completion Engine |

## Operations

| file | title |
| --- | --- |
| [`DEPLOY_PLAN_2026-09-20.md`](operations/DEPLOY_PLAN_2026-09-20.md) | gigi-stream deploy — 2026-09-20 |
| [`DEPLOY_RUNBOOK_2026-08-11.md`](operations/DEPLOY_RUNBOOK_2026-08-11.md) | GIGI-STREAM DEPLOY RUNBOOK |
| [`GIGI_TOOL_AUDIT_2026-07-02.md`](operations/GIGI_TOOL_AUDIT_2026-07-02.md) | GIGI Tool Audit — 2026-07-02 |
| [`INCIDENT_2026-08-12_gworls_reply4.md`](operations/INCIDENT_2026-08-12_gworls_reply4.md) | gworls — the two open defects are closed |
| [`INCIDENT_2026-08-20_snapshot.md`](operations/INCIDENT_2026-08-20_snapshot.md) | Incident: pre-deploy snapshot failure, 2026-08-20 04:43 UTC |
| [`MARCELLA_INTENT_GATE_MIGRATION.md`](operations/MARCELLA_INTENT_GATE_MIGRATION.md) | Marcella → `/brain/intent_gate` migration recipe |
| [`SECURITY_REVIEW_SELF.md`](operations/SECURITY_REVIEW_SELF.md) | GIGI — Internal Security Review |
| [`TYPE_SEAM_AUDIT.md`](operations/TYPE_SEAM_AUDIT.md) | Type Seam Audit |

## Correspondence

| file | title |
| --- | --- |
| [`GIGI_LANG_REPLY_FROM_ENGINE_2026-05-24.md`](correspondence/GIGI_LANG_REPLY_FROM_ENGINE_2026-05-24.md) | GIGI Lang spec — reply from the engine side |
| [`HANDOFF_GIGI_BUILDS_SITE.md`](correspondence/HANDOFF_GIGI_BUILDS_SITE.md) | HANDOFF — GIGI Builds companion site + tetmesh chapter |
| [`MARCELLA_BRAIN_PRIMITIVE_REPLY_2026_05_27.md`](correspondence/MARCELLA_BRAIN_PRIMITIVE_REPLY_2026_05_27.md) | Gigi → Marcella: brain primitive clarifications |
| [`MORNING_BRIEFING_S1_STEPS_1_2.md`](correspondence/MORNING_BRIEFING_S1_STEPS_1_2.md) | S1 morning briefing — steps 1+2 done |
| [`answer-for-art-vector-brain.md`](correspondence/answer-for-art-vector-brain.md) | Answer — the brain endpoints did not intentionally ignore `Value::Vector` |
| [`fixlist-for-bee.md`](correspondence/fixlist-for-bee.md) | GIGI — Site & Numbers Fix Spec |

## Moved from the repository root (22 November 2026)

These files used to sit at the root. If an old note or a code comment names one of them, it is here now.

| old name | now at |
| --- | --- |
| `BENCHMARKS_VS_STORES.md` | [`docs/reference/BENCHMARKS_VS_STORES.md`](reference/BENCHMARKS_VS_STORES.md) |
| `BRAIN_PRIMITIVES_CONSUMER_GUIDE.md` | [`docs/reference/BRAIN_PRIMITIVES_CONSUMER_GUIDE.md`](reference/BRAIN_PRIMITIVES_CONSUMER_GUIDE.md) |
| `DEPLOY_PLAN_2026-09-20.md` | [`docs/operations/DEPLOY_PLAN_2026-09-20.md`](operations/DEPLOY_PLAN_2026-09-20.md) |
| `DEPLOY_RUNBOOK_2026-08-11.md` | [`docs/operations/DEPLOY_RUNBOOK_2026-08-11.md`](operations/DEPLOY_RUNBOOK_2026-08-11.md) |
| `DOCUMENTATION_PLAN.md` | [`docs/specs/DOCUMENTATION_PLAN.md`](specs/DOCUMENTATION_PLAN.md) |
| `GIGI_ANOMALY_DASHBOARD_SPEC.md` | [`docs/specs/GIGI_ANOMALY_DASHBOARD_SPEC.md`](specs/GIGI_ANOMALY_DASHBOARD_SPEC.md) |
| `GIGI_AUTOMATIC_ANALYTICS_API.md` | [`docs/specs/GIGI_AUTOMATIC_ANALYTICS_API.md`](specs/GIGI_AUTOMATIC_ANALYTICS_API.md) |
| `GIGI_COHERENCE_EXTENSIONS_v0.1.md` | [`docs/specs/GIGI_COHERENCE_EXTENSIONS_v0.1.md`](specs/GIGI_COHERENCE_EXTENSIONS_v0.1.md) |
| `GIGI_CRUD_ROADMAP.md` | [`docs/specs/GIGI_CRUD_ROADMAP.md`](specs/GIGI_CRUD_ROADMAP.md) |
| `GIGI_ENCRYPT_v0.2_SPRINT_SPEC.md` | [`docs/specs/GIGI_ENCRYPT_v0.2_SPRINT_SPEC.md`](specs/GIGI_ENCRYPT_v0.2_SPRINT_SPEC.md) |
| `GIGI_ENCRYPT_v0.3_SPRINT_SPEC.md` | [`docs/specs/GIGI_ENCRYPT_v0.3_SPRINT_SPEC.md`](specs/GIGI_ENCRYPT_v0.3_SPRINT_SPEC.md) |
| `GIGI_GEOMETRIC_ENCRYPTION_SPEC.md` | [`docs/specs/GIGI_GEOMETRIC_ENCRYPTION_SPEC.md`](specs/GIGI_GEOMETRIC_ENCRYPTION_SPEC.md) |
| `GIGI_LANG_REPLY_FROM_ENGINE_2026-05-24.md` | [`docs/correspondence/GIGI_LANG_REPLY_FROM_ENGINE_2026-05-24.md`](correspondence/GIGI_LANG_REPLY_FROM_ENGINE_2026-05-24.md) |
| `GIGI_LANG_SPEC.md` | [`docs/specs/GIGI_LANG_SPEC.md`](specs/GIGI_LANG_SPEC.md) |
| `GIGI_OBSERVABILITY_SPEC.md` | [`docs/specs/GIGI_OBSERVABILITY_SPEC.md`](specs/GIGI_OBSERVABILITY_SPEC.md) |
| `GIGI_PERF_ANALYSIS.md` | [`docs/reference/GIGI_PERF_ANALYSIS.md`](reference/GIGI_PERF_ANALYSIS.md) |
| `GIGI_PERSISTENCE_UPGRADE_SPEC.md` | [`docs/specs/GIGI_PERSISTENCE_UPGRADE_SPEC.md`](specs/GIGI_PERSISTENCE_UPGRADE_SPEC.md) |
| `GIGI_POST_KAHLER_SPEC_v0.1.md` | [`docs/specs/GIGI_POST_KAHLER_SPEC_v0.1.md`](specs/GIGI_POST_KAHLER_SPEC_v0.1.md) |
| `GIGI_PRODUCT_SPECS.md` | [`docs/specs/GIGI_PRODUCT_SPECS.md`](specs/GIGI_PRODUCT_SPECS.md) |
| `GIGI_Provisional_Patent.pdf` | [`docs/patents/GIGI_Provisional_Patent.pdf`](patents/GIGI_Provisional_Patent.pdf) |
| `GIGI_Provisional_Patent.tex` | [`docs/patents/GIGI_Provisional_Patent.tex`](patents/GIGI_Provisional_Patent.tex) |
| `GIGI_SAMPLE_TRANSPORT_SPRINT_SPEC.md` | [`docs/specs/GIGI_SAMPLE_TRANSPORT_SPRINT_SPEC.md`](specs/GIGI_SAMPLE_TRANSPORT_SPRINT_SPEC.md) |
| `GIGI_SCHEMA_INTROSPECTION_SPEC.md` | [`docs/specs/GIGI_SCHEMA_INTROSPECTION_SPEC.md`](specs/GIGI_SCHEMA_INTROSPECTION_SPEC.md) |
| `GIGI_SHEETS_SPRINT_SPEC.md` | [`docs/specs/GIGI_SHEETS_SPRINT_SPEC.md`](specs/GIGI_SHEETS_SPRINT_SPEC.md) |
| `GIGI_SHEETS_SPRINT_SPEC_ADDENDUM_v0.1.md` | [`docs/specs/GIGI_SHEETS_SPRINT_SPEC_ADDENDUM_v0.1.md`](specs/GIGI_SHEETS_SPRINT_SPEC_ADDENDUM_v0.1.md) |
| `GIGI_SPEC_v0.1.md` | [`docs/specs/GIGI_SPEC_v0.1.md`](specs/GIGI_SPEC_v0.1.md) |
| `GIGI_SUDOKU_SPRINT_SPEC.md` | [`docs/specs/GIGI_SUDOKU_SPRINT_SPEC.md`](specs/GIGI_SUDOKU_SPRINT_SPEC.md) |
| `GIGI_TETMESH_SPEC_v0.6.md` | [`docs/specs/GIGI_TETMESH_SPEC_v0.6.md`](specs/GIGI_TETMESH_SPEC_v0.6.md) |
| `GIGI_TOOL_AUDIT_2026-07-02.md` | [`docs/operations/GIGI_TOOL_AUDIT_2026-07-02.md`](operations/GIGI_TOOL_AUDIT_2026-07-02.md) |
| `GIGI_V2_FEATURE_SPEC.md` | [`docs/specs/GIGI_V2_FEATURE_SPEC.md`](specs/GIGI_V2_FEATURE_SPEC.md) |
| `GQL_ADDENDUM_v2.1.md` | [`docs/reference/GQL_ADDENDUM_v2.1.md`](reference/GQL_ADDENDUM_v2.1.md) |
| `GQL_SPECIFICATION.md` | [`docs/reference/GQL_SPECIFICATION.md`](reference/GQL_SPECIFICATION.md) |
| `HANDOFF_GIGI_BUILDS_SITE.md` | [`docs/correspondence/HANDOFF_GIGI_BUILDS_SITE.md`](correspondence/HANDOFF_GIGI_BUILDS_SITE.md) |
| `INCIDENT_2026-08-12_gworls_reply4.md` | [`docs/operations/INCIDENT_2026-08-12_gworls_reply4.md`](operations/INCIDENT_2026-08-12_gworls_reply4.md) |
| `INCIDENT_2026-08-20_snapshot.md` | [`docs/operations/INCIDENT_2026-08-20_snapshot.md`](operations/INCIDENT_2026-08-20_snapshot.md) |
| `MARCELLA_BRAIN_PRIMITIVE_REPLY_2026_05_27.md` | [`docs/correspondence/MARCELLA_BRAIN_PRIMITIVE_REPLY_2026_05_27.md`](correspondence/MARCELLA_BRAIN_PRIMITIVE_REPLY_2026_05_27.md) |
| `MARCELLA_INTENT_GATE_MIGRATION.md` | [`docs/operations/MARCELLA_INTENT_GATE_MIGRATION.md`](operations/MARCELLA_INTENT_GATE_MIGRATION.md) |
| `MORNING_BRIEFING_S1_STEPS_1_2.md` | [`docs/correspondence/MORNING_BRIEFING_S1_STEPS_1_2.md`](correspondence/MORNING_BRIEFING_S1_STEPS_1_2.md) |
| `SECURITY_REVIEW_SELF.md` | [`docs/operations/SECURITY_REVIEW_SELF.md`](operations/SECURITY_REVIEW_SELF.md) |
| `TYPE_SEAM_AUDIT.md` | [`docs/operations/TYPE_SEAM_AUDIT.md`](operations/TYPE_SEAM_AUDIT.md) |
| `VALIDATION.md` | [`docs/reference/VALIDATION.md`](reference/VALIDATION.md) |
| `answer-for-art-vector-brain.md` | [`docs/correspondence/answer-for-art-vector-brain.md`](correspondence/answer-for-art-vector-brain.md) |
| `bracket-data-pipeline.js` | [`playground/bracket/bracket-data-pipeline.js`](../playground/bracket/bracket-data-pipeline.js) |
| `bracket-engine.js` | [`playground/bracket/bracket-engine.js`](../playground/bracket/bracket-engine.js) |
| `bracket-engine.test.js` | [`playground/bracket/bracket-engine.test.js`](../playground/bracket/bracket-engine.test.js) |
| `bracket-run.js` | [`playground/bracket/bracket-run.js`](../playground/bracket/bracket-run.js) |
| `bracket.jsx` | [`playground/bracket/bracket.jsx`](../playground/bracket/bracket.jsx) |
| `fixlist-for-bee.md` | [`docs/correspondence/fixlist-for-bee.md`](correspondence/fixlist-for-bee.md) |
| `gigi-site.jsx` | [`playground/components/gigi-site.jsx`](../playground/components/gigi-site.jsx) |
| `gigi-tui-mockup.html` | [`mockups/gigi-tui-mockup.html`](../mockups/gigi-tui-mockup.html) |
| `gigi_report.html` | [`mockups/gigi_report.html`](../mockups/gigi_report.html) |
| `gigi_sheaf_completion_spec.md` | [`docs/specs/gigi_sheaf_completion_spec.md`](specs/gigi_sheaf_completion_spec.md) |
| `gql-playground.jsx` | [`playground/components/gql-playground.jsx`](../playground/components/gql-playground.jsx) |
| `test_sheaf_completion.py` | [`tests/python/test_sheaf_completion.py`](../tests/python/test_sheaf_completion.py) |
