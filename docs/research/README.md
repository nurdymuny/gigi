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
| 21 Nov | [What the edge workflow found, and what it corrected](edge-workflow-findings-2026-11-21.md) |
| 21 Nov | [The exact edge of her region](exact-edge-2026-11-21.md) |
| 22 Nov | [Pre-registration A: the edge at 48 held-out points: results](edge-prereg-A-results-2026-11-22.md) |
| 22 Nov | [Pre-registration B: random circle-symmetric fields: results](edge-prereg-B-results-2026-11-22.md) |
| 22 Nov | [Pre-registration C: SU(3) with a free fibre field: results](edge-prereg-C-results-2026-11-22.md) |
| 22 Nov | [Pre-registration E: the two hands: results](edge-prereg-E-results-2026-11-22.md) |
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

## Where we were: status and strategy

| date | note |
| --- | --- |
| 19 Nov | [Yang–Mills inside GIGI: the strategy](yang-mills-strategy-2026-11-19.md) |
| 19 Nov | [Status: what is done, what is running, what is open](session-status-2026-11-19.md) |
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
