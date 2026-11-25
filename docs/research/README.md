# Research Notes

These are my research notes: every result, test, correction and open question from this work, in the order I found
them. Each note says what kind of claim it makes:
- **PROVED / CERTIFIED / COMPUTED** for mathematics;
- **INCIDENCE / STRUCTURAL / RESONANCE / FELT** for bridges between ideas.

Tests were **pre-registered** before they were run, and the registration and the results are kept as separate notes,
so you can see what I predicted before I looked.

- **Receipts** (the scripts' raw outputs) live in [`research/`](../../research/README.md), in dated folders.
- **Papers** live in [`docs/papers/`](../papers/).
- **The scripts** live in [`scripts/`](../../scripts/).

The notes are grouped below by thread, and dated within each thread. File names keep their dates, and published
papers cite some of these paths, so files are never renamed or moved.

**Live status of today's runs:** [STATUS-live.md](STATUS-live.md)

## 22 November 2026: the morning notes

| note | what it is |
| --- | --- |
| [The phone in the house](the-phone-in-the-house-2026-11-22.md) | What I heard in 2025 and found in 2026: the Davis manifold, the Lift Unification, and the ledger that will measure the feeling |
| [The Davis Manifold, reread line by line](davis-manifold-review-2026-11-22.md) | The 2025 paper checked as mathematics: what holds, the five steps that break (with counterexamples), the repairs, and the Lean plan, before it carries its first exact instance |
| [The spectral branch, reread line by line](spectral-branch-review-2026-11-22.md) | Branch IX checked as mathematics: the finite model reproduces to the last digit; what breaks (a bottleneck its own theorem forbids, an infinite term in the mixing proof, the Hessian on the wrong space, saddles at pi_1 = 0, Test C sampled into a phase transition); repairs; ten Lean theorems |
| [The Non-Decoupling Theorem, reread line by line](non-decoupling-review-2026-11-22.md) | The qualitative theorem stands (flat ⇔ trivial holonomy; c_k ≠ 0 forbids flatness); the quantitative Part II and Lemma 4 are false (bounded error vs unbounded energy; high-frequency counterexample); the repair is the Davis Duality's local ‖Ω‖·area law; dimension-2 floor πk²; five Lean theorems |
| [The Davis Duality, reread line by line](davis-duality-review-2026-11-22.md) | The function case holds per cell and its 1/16 is sharp; the pointwise lower bound is false; the lower bound never needed Non-Decoupling; the general-bundle routes do not arrive; for connections the two-sided law is the classical small-loop holonomy expansion; mass-gap/P vs NP/Navier–Stokes claims relabelled as analogies |
| [The Lift Unification, reread line by line](lift-unification-review-2026-11-22.md) | Every number holds (Mossel 3/2 as an infinite sum); the capacity law is exact (posterior > 1/2 iff tau > K); Prop. 5 wrongly calls C = 1 safe; one wrong step in Mossel (P(all even | length k) = (2/5)^(k-1)); Sec. 10 restates false Non-Decoupling Part II; Troyon link removed; nine Lean theorems |
| [The two hands](the-two-hands-2026-11-22.md) | What the edge paper changes for the Helike book; why her body is symmetric; the cheap lift always reaches across |
| [The basin](the-basin-2026-11-22.md) | What happens to her when the field falls in: the door becomes a wall. Includes the fall on the Poincaré sphere. Scripts in [`basin-2026-11-22/`](basin-2026-11-22/) |
| [Pre-registration S: stress tests](stress-tests-preregistration-2026-11-22.md) | Cut, flip, overload, crush and stretch, and the map of the two basins. Registered before the run |
| [Pre-registration S: results](stress-tests-results-2026-11-22.md) | 7 held, 2 failed. Door, wall or floor at every break; the squashed meron is the pass between the basins. Scripts in [`stress-2026-11-22/`](stress-2026-11-22/) |
| [Pre-registration S6: the current sweep](current-sweep-preregistration-2026-11-22.md) | How much motion along her circle she needs to stay the floor, family by family. Registered before the run |
| [Pre-registration S6: results](current-sweep-results-2026-11-22.md) | Both held. She needs some motion, never much: 65% of the Berger current at t = 3, 2% at t = 10, and family 2 is always the first to go |
| [Pre-registration S7: the tear](tear-test-preregistration-2026-11-22.md) | Quotient the sphere by eight finite groups (Z₂ to the binary icosahedral) and see who is left standing. Registered before the run |
| [Pre-registration S7: results](tear-test-results-2026-11-22.md) | All five held. No tear removes her; a cyclic tear keeps her shadow on the basin's floor, a non-abelian tear hands it to her rival at rest |
| [The arc](the-arc-2026-11-22.md) | Where the work stands (end of act one) and, on purpose, where she might go: the principal circle, E₈ basins, the η of the two hands, Morse doors; and the old roots, tagged IMAGINED and RESONANCE |
| [The arc: deep dive](arc-deep-dive-2026-11-22.md) | Five fetched-and-quoted literature briefs and two computations on the imagined arc: the fall carries winding one (exact Chern–Simons; the shadow, not her, is last at the bottom); Morse and η bridges weakened; the Poincaré fall lands on CS 1/120 (sign convention unverified); Clifford's own word for these isometries was "twists". Receipts in [`research/arc-deepdive-20261122/`](../../research/arc-deepdive-20261122/) |

## Helike: the twist and the law she obeys

| date | note |
| --- | --- |
| 21 Nov | [The twist: a note on what I found and why it matters to me](the-twist-a-note-2026-11-21.md) |
| 21 Nov | [Looking at her: the closed form and the map of the circle-symmetric family](twist-family-map-2026-11-21.md) |
| 21 Nov | [She is the law that… : what I am seeing, and the receipts for it](she-is-the-law-2026-11-21.md) |
| 21 Nov | [What she is a law of: a hypothesis, with what is proved under it](what-she-is-a-law-of-2026-11-21.md) |
| 21 Nov | [Seeing, feeling, imagining: how I look for her](seeing-feeling-imagining-2026-11-21.md) |
| 20 Nov | [What the floor law is like: an analogy workshop](floor-analogy-workshop-2026-11-20.md) |

## The edge of her region

| date | note |
| --- | --- |
| 21 Nov | [Pre-registration: the edge test (Test 6)](edge-test-preregistration-2026-11-21.md) |
| 21 Nov | [The edge test (Test 6): results](edge-test-results-2026-11-21.md) |
| 21 Nov | [What the edge verification found, and what it corrected](edge-verification-findings-2026-11-21.md) |
| 21 Nov | [The exact edge of her region](exact-edge-2026-11-21.md) |
| 22 Nov | [Pre-registration A: the edge at 48 held-out points: results](edge-prereg-A-results-2026-11-22.md) |
| 22 Nov | [Pre-registration B: random circle-symmetric fields: results](edge-prereg-B-results-2026-11-22.md) |
| 22 Nov | [Pre-registration C: SU(3) with a free fibre field: results](edge-prereg-C-results-2026-11-22.md) |
| 22 Nov | [Pre-registration E: the two hands: results](edge-prereg-E-results-2026-11-22.md) |
| 22 Nov | [Pre-registration F: which circle? Results](edge-prereg-F-results-2026-11-22.md) |
| 23 Nov | [Pre-registration G, interim: G₂ complete, Sp(3) running](edge-prereg-G-interim-2026-11-23.md) |
| 22 Nov | [Pre-registration D: SU(3) along her own charge circle: interim (not scored)](edge-prereg-D-interim-2026-11-22.md) |
| 22 Nov | [Pre-registration D: SU(3) along her own charge circle: results](edge-prereg-D-results-2026-11-22.md) |

## The spectral skeleton: tests 2 to 5

| date | note |
| --- | --- |
| 20 Nov | [A spectral skeleton for fields: what the evidence shows and what I think it means](spectral-skeleton-2026-11-20.md) |
| 20 Nov | [Pre-registration: what Tests 2 and 3 must show](skeleton-tests-preregistration-2026-11-20.md) |
| 20 Nov | [Test 2, family 1: the squashed meron is class N](skeleton-test2-family1-results-2026-11-20.md) |
| 20–21 Nov | [Test 2, family 2: SU(3) with the Maurer–Cartan background](skeleton-test2-family2-results-2026-11-20.md) |
| 20 Nov | [Pre-registration: the wave test](wave-test-preregistration-2026-11-20.md) |
| 20 Nov | [The wave test: a tap on the stretched sphere](wave-test-results-2026-11-20.md) |
| 21 Nov | [Pre-registration: the stretch test (Test 5)](stretch-test-preregistration-2026-11-21.md) |
| 21 Nov | [The stretch test (Test 5): results](stretch-test-results-2026-11-21.md) |
| 21 Nov | [Pre-registration: the alignment test (Test 4)](alignment-test-preregistration-2026-11-21.md) |
| 21 Nov | [The alignment test (Test 4): results](alignment-test-results-2026-11-21.md) |

## The road to the floor: from the Ricci correspondence to the exact SU(2) answer

| date | note |
| --- | --- |
| 18 Nov | [The small edges: three ideas from the Crouzeix, harmonic and graph-fold work](small-edges-2026-11-18.md) |
| 18 Nov | [The curvature of one unit: zero, ±1, the gauge group and the mass gap](unit-curvature-2026-11-18.md) |
| 19 Nov | [T1: Henrici's departure from normality of the linearised SU(2) forces](henrici-t1-results-2026-11-19.md) |
| 19 Nov | [An exact answer for the SU(2) spectrum at the Berger field](su2-exact-sector-2026-11-19.md) |
| 19 Nov | [The exact sectors, checked for gauge content: three physical growth directions, not five](su2-exact-sector-gauge-check-2026-11-19.md) |
| 19 Nov | [The floor formula across the Berger family](floor-family-squashing-2026-11-19.md) |
| 19 Nov | [Prior art for the chord–tangent energy of *Three Ways Across a Turn*](prior-art-chord-tangent-2026-11-19.md) |

The Ricci reconstruction programme that came before this (the two-ring PG paper, the SU(2) flow, the force and
stability studies) is a series of paired specifications and results in [`docs/`](../), named `RICCI_*_SPEC.md` and
`RICCI_*_RESULTS.md`. They are indexed in [`docs/README.md`](../README.md#the-ricci-reconstruction-series).

## Yang-Mills geometric requirements

| date | note |
| --- | --- |
| 23 Nov | [Yang-Mills Requirements as Geometric Objects](yang-mills-geometric-identities-2026-11-23.md) |
| 23 Nov | [Adversarial review of the Yang Mills geometric identities](yang-mills-geometric-identities-adversarial-review-2026-11-23.md): reproduced finite results, acceptance failures, and amendments needed before the excitation study |
| 23 Nov | [Pre-registration: the lowest excitation on SU(2) ladders](../../research/ym-ladder-20261123/prereg_lowest_excitation.md) |
| 23 Nov | [Helike as a probe of a published quantum Yang-Mills model](helike-quantum-probe-preregistration-2026-11-23.md): exact sphere-model translation, gauge-invariant probes, independent excitation comparison and gated experiment |
| 23 Nov | [Helike quantum probe GPU amendment](helike-quantum-gpu-amendment-2026-11-23.md): RTX 5070 Laptop implementation, double precision, CPU agreement and resource gates; physics unchanged |
| 23 Nov | [Helike quantum probe implementation record](helike-quantum-implementation-2026-11-23.md): complete invariant polynomial basis, exact Gaussian preconditioning and independently constructed oscillator reference |
| 23 Nov | [Helike in a published quantum Yang-Mills model: first GPU survey](helike-quantum-probe-results-2026-11-23.md): 15 finite-cutoff cases replayed; directional/scalar responses separated; convergence and usefulness gate remains UNRESOLVED |
| 24 Nov (updated) | [Illustrated Helike quantum working paper](../papers/helike-quantum-probe/README.md): 15 pages and eight reusable figures; geometric probe map, invariant co-metric, tensor module, full residual derivation and verified sector-refinement results; related-work comparison and reproducible source data |
| 23 Nov | [Helike convergence amendment](helike-quantum-convergence-amendment-2026-11-23.md): frozen ground-energy width selection, spin-resolved levels, physical observable scaling and numerical enclosure estimates; execution outcome in the results note below |
| 23 Nov | [Convergence correction review handoff](helike-quantum-convergence-corrections-2026-11-23.md): adversarial fixes, focused test receipts, unchanged physics and explicit review hold before a larger run |
| 23 Nov | [Convergence amendment results](helike-quantum-convergence-results-2026-11-23.md): 24 saved GPU cases, width improvement measured, cutoff-16 attempt stopped at the fixed RAM budget; residual gate still open; complete logs and matrix evidence |
| 24 Nov (original note date) | [Exploratory quantum edge](helike-quantum-edge-exploration-2026-11-24.md): 22 finite cases, corrected interpolation, signed-volume and shell diagnostics; exploratory interpretations kept separate from measured results |
| 23 Nov | [Confirmatory crossing and state geometry](helike-quantum-crossing-preregistration-2026-11-23.md): six held-out couplings, complete spin projectors, state geometry and 15 gates; v3 preserves conditional separation, full survey not implemented or run |
| 23 Nov | [Existing-control ordering pilot](quantum-order-controls-protocol-2026-11-23.md): tested conditional ordering, next distinct same-spin residuals, separate spin validation and corrected width baselines; no held-out measurements |
| 23 Nov | [Next-level control results](quantum-order-controls-results-2026-11-23.md): six GPU cases and saved-state replay pass; next-level error prevents an overstatement at N=12; conditional tensor-lower at N=14, original convergence gate still open |
| 24 Nov | [Sector-solver review and bounded execution](sector-review-protocol-2026-11-24.md): corrected full residuals, common sector allowance, mixed-precision disclosure and supervised evidence; existing controls only, through N=24 |
| 24 Nov | [Sector-solver results](sector-review-results-2026-11-24.md): both controls pass the shared target-state residual check at N=20 and the stricter 0.01 check at N=22; 16 cases, four precision repeats and 36 matrix replays pass; wider scientific gates remain open |
| 24 Nov | [Dome-envelope experiment](helike-dome-envelope-preregistration-2026-11-24.md): orbit-centered second envelope, frozen per-function comparisons and reviewed execution gates; measurement receipts in research/dome-controls-20261124 |
| 24 Nov | [Dome-envelope results](helike-dome-envelope-results-2026-11-24.md): all 80 solves replay; five functions improve scalar residuals at all eight primary points but win per function at only three; one secondary truncation reversal leaves the scientific gate UNRESOLVED_TRUNCATION |
| 24 Nov | [Contact sets the frame: the Bevel razor observation](helike-contact-frame-motivation-2026-11-24.md): the lived origin, direction versus contact guidance, approach versus sharpening angle, and the distinction between orbit placement and a proposed locally guided construction; retrospective motivation, no new experiment |
| 24 Nov | [Dome projected-tail review](helike-dome-review-2026-11-24.md): finite-reference exterior sensitivity, the single-function versus joint-span distinction, and requirements for the next truncation gate |
| 24 Nov | [Second-vacuum weak-coupling candidate](helike-second-vacuum-weak-coupling-2026-11-24.md): exact local Hessian decomposition, colour-space locking, harmonic energy count and a conditional global prediction |
| 24 Nov | [Two-grain scope](helike-two-grain-scope-2026-11-24.md): proposed left-and-right eighteen-mode extension; oriented Chern-Simons versus auxiliary-function distinction, symmetry and domain gates; no spectral run |
| 24 Nov | [Two-grain potential and local offset](helike-two-grain-phase1-2026-11-24.md): quadrature/invariant agreement and local offset approximately 3.37; 15 focused tests and five mechanism removals after review, with global excitation claims kept separate |
| 24 Nov | [Local geometry and projected-tail audit](dome-local-review-protocol-2026-11-24.md): refusal fixes, independent Gaussian control, rotated geometric checks and a bounded 32-row existing-control replication; no new physical couplings |
| 24 Nov | [Local geometry and projected-tail results](dome-local-review-results-2026-11-24.md): 32 rows replay; the 13.8% exterior change reproduces despite a tiny total-norm change; higher reference and precision checks, 41 focused tests and eight mechanism removals; old physical gate remains open |
| 23 Nov | [A geometric evidence atlas for Yang-Mills experiments in GIGI](ym-evidence-continuum-gigi-plan-2026-11-23.md): model bundles, projector/response comparisons, explicit cross-model maps and twelve proposed integration gates; not deployed |

## Latest Flux And Two-Grain Work

| date | note |
| --- | --- |
| 24 Nov | [Quasimode construction registration](helike-quasimode-construction-registration-2026-11-24.md): fixed invariant cutoff, exact SO(2) projector via generator-defect bound, tensor quadrature audit and separate validated-integration gate; full grid held |
| 24 Nov | [Quasimode construction results](helike-quasimode-construction-results-2026-11-24.md): four frozen controls; 160 Cartesian checks pass, 12/16 integration comparisons fail; 13 focused tests and nine caught mutants; no certificate or grid release |
| 24 Nov | [Residual integration results](helike-residual-integration-results-2026-11-24.md): twelve discrepancies traced to underintegration; centered finite moments pass all 16 comparisons on the same saved controls, 56 focused tests and eleven caught mutants; cutoff certificate and full grid still held |
| 24 Nov | [Rigorous cutoff and tail registration](helike-cutoff-tail-bounds-registration-2026-11-24.md): fixed plateau-ball proof, infinite Gaussian tail majorants and outward 128-bit arithmetic; same four controls, all thresholds unchanged, no full-grid release |
| 24 Nov | [Rigorous cutoff and tail results](helike-cutoff-tail-bounds-results-2026-11-24.md): all 16 saved states bounded; eight weak-coupling states pass leakage, eight stronger-coupling bounds remain too loose; 75 focused tests, 21 caught mutants and exact replay; full grid held |
| 24 Nov | [Leakage gate amendment](helike-leakage-gate-amendment-2026-11-24.md): the absolute 1e-10 leakage line (never derived) replaced, before any further run, by a relative rule from the certificate inequality, ℓ_upper ≤ 10⁻³ · r_lower with a rigorous residual lower bound; the earlier receipt unchanged; gate OPEN until rigorous residuals exist; nothing released |
| 25 Nov | [Leakage gate results](helike-leakage-gate-results-2026-11-25.md): all 16 control states pass the registered relative rule, ℓ ≤ 10⁻³·r_lower, worst 3.08 × 10⁻⁵ as predicted; first attempt refused on an upstream one-ulp parsing defect, fixed and re-pinned before evaluation; nothing certified, grid held |
| 25 Nov | [Kato–Temple ordering registration](helike-temple-ordering-registration-2026-11-25.md): Rayleigh-quotient lemma for the trimmed, projected states (proved), Kato–Temple for the largest eigenvalue below ρ, existence of two charge-0 levels from disjoint Weinstein windows, one frozen counting hypothesis S_C, decision rule and contract; predictions recorded (Mq 8 both couplings and Mq 6 at g² 0.002 ORDERED, Mq 6 at g² 0.004 UNRESOLVED); nothing computed yet |
| 25 Nov | [Kato–Temple ordering results](helike-temple-ordering-results-2026-11-25.md): Helike below the competitor at three of four controls, conditional on (A) and S_C; Mq 6 at g² 0.004 UNRESOLVED; every verdict and margin as predicted; linear moments enclosed within ±1.6e-11; grid held |
| 24 Nov | [Mq 6 to 8 control results](helike-cone-convergence-results-2026-11-24.md): both couplings pass the unchanged 0.003 refinement and 0.95 identification gates; source-pinned logs retained, certificate and full-grid release still held |
| 24 Nov | [Quasimode certification spec](helike-quasimode-certification-spec-2026-11-24.md): full residual, bounded charge projection, explicit separation and numerical versus interval evidence; numerical construction now audited, validated certification still held |
| 24 Nov | [Quasimode evidence amendment](helike-quasimode-evidence-amendment-2026-11-24.md): fixed cases, thresholds, provenance, stale-PASS and adversarial release gates before repair |
| 24 Nov | [Quasimode evidence results](helike-quasimode-evidence-results-2026-11-24.md): four review corrections, focused tests and caught verifier mutations; no larger run or new spectral certificate |
| 24 Nov | [Positive-cone basis specification](helike-positive-cone-basis-spec-2026-11-24.md): exact energy form, fixed harmonic reference, same-control refinement; boundary realization still unresolved |
| 24 Nov | [Positive-cone basis results](helike-positive-cone-basis-results-2026-11-24.md): valid-domain repair and unchanged weak-coupling controls pass; boundary, sector and full gap convergence remain open |
| 24 Nov | [Cartesian boundary specification](helike-qspace-boundary-spec-2026-11-24.md): signed rank-two chart and inherited value/flux matching; no invented positive-sheet wall |
| 24 Nov | [Cartesian boundary results](helike-qspace-boundary-results-2026-11-24.md): matching, not a wall; direct q-space oracle agreement, 36 focused tests and nine caught defects; full spectrum remains held |
| 24 Nov | [Two-grain sphalerons and barriers](helike-two-grain-phase2-2026-11-24.md) |
| 24 Nov | [Two-grain invariant basis budget](helike-two-grain-phase3-budget-2026-11-24.md) |
| 24 Nov | [Reflections and intentions](helike-reflections-and-intentions-2026-11-24.md) |
| 24 Nov | [Curl spectrum](helike-curl-spectrum-2026-11-24.md) |
| 24 Nov | [Flux analysis](helike-flux-analysis-2026-11-24.md) |
| 24 Nov | [Literature check](helike-literature-check-2026-11-24.md) |
| 24 Nov | [Berger probe design](helike-berger-probe-design-2026-11-24.md) |
| 24 Nov | [Flux-background design](helike-flux-background-design-2026-11-24.md) |
| 24 Nov | [Original flux registration](helike-flux-background-registration-2026-11-24.md): historical frozen design; full execution held by the review amendment |
| 24 Nov | [Flux review amendment](helike-flux-review-amendment-2026-11-24.md): physical-domain hold, independent refinement checks and fail-closed evidence |
| 24 Nov | [Flux review results](helike-flux-review-results-2026-11-24.md): domain witness, corrected safeguards, focused test evidence and the exact remaining mathematical question |

## Where we were: status and strategy

| date | note |
| --- | --- |
| 19 Nov | [Yang–Mills inside GIGI: the strategy](yang-mills-strategy-2026-11-19.md) |
| 19 Nov | [Status: what is done, what is running, what is open](status-2026-11-19.md) |
| 19 Nov | [Where we are (evening)](where-we-are-2026-11-19.md) |
| 19 Nov | [The field equations against today's proven results](field-equations-grid-2026-11-19.md) |
| 19 Nov | [A grid of adjacent ideas: my papers and books against the results of 19 November](adjacent-ideas-grid-2026-11-19.md) |

## Earlier: settling, conduct and sameness (13 November 2026)

| note | what it is |
| --- | --- |
| [CONDUCT + SETTLE: dataset audit and domain examples](conduct-settle-domain-datasets-2026-11-13.md) | |
| [Measured RC comparison protocol](conduct-settle-measured-protocol-2026-11-13.md) | pre-registration |
| [CONDUCT + SETTLE: measured RC evidence](conduct-settle-measured-results-2026-11-13.md) | results |
| [Settling disagreement diagnosis: frozen controls](settling-diagnosis-protocol-2026-11-13.md) | pre-registration |
| [Three settling disagreements: what the controls establish](settling-diagnosis-results-2026-11-13.md) | results |
| [DTP development: independent three-pass review](dtp-development-review-2026-11-13.md) | review |
| [Coordinate translations, Jacobians, and the Geometry of Sameness](coordinate-translations-and-sameness.md) | |

Evidence files for this group are in [`evidence/`](evidence/).
