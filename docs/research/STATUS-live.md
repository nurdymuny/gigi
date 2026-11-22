# Live status (updated by the session at every milestone)

Open this file in your breaks: it says what is running, what landed and where to read it.
Nothing here is a result in itself. Each result lives in its own note, scored as registered.

**Last updated: 22 November 2026, afternoon: F resumed under Amendment 2.**

## Running

| what | started | expected | where it lands |
| --- | --- | --- | --- |
| **Test F**: which circle? A sweep across the chamber of Sp(2) and SU(4) | 22 Nov, late morning | 6–10 h | results note `edge-prereg-F-results-2026-11-22.md`; receipt `research/edge-theorem-20261122-f/` |

The deep-dive covers:
- five fetched-and-quoted literature briefs: the principal circle, McKay/E₈, the η-invariant, Morse theory, and the
  ancient sources;
- the Chern–Simons winding of the fall;
- a principal-circle survey of SU(5), Sp(3) and G₂.

## Active research: needs adversarial review

Each item below is written and pushed, but **nobody has tried to break it yet**. Each needs an adversarial review
before it is cited, posted or put in the book. The reviewer's job is to refute, not to confirm. Default to
"refuted" whenever a step cannot be checked.

| item | where | what to attack |
| --- | --- | --- |
| **The Davis Manifold, second edition (draft)** | paper: `docs/papers/davis-manifold/the_davis_manifold_v2.tex` (PDF beside it; the 2025 text is unchanged next to it). Argument: [review note](davis-manifold-review-2026-11-22.md). Lean: `DavisManifold/Review.lean` on `feat/helike-lean` (13 theorems) | see the list below |
| **The spectral branch (Branch IX), review note** | [review note](spectral-branch-review-2026-11-22.md); recomputation `spectral-branch-2026-11-22/checks.py` + `checks_output.json`; Lean `SpectralBranch/Review.lean` on `feat/helike-lean` (10 theorems). The paper itself (`davis_plasma/theory/spectral_branch.tex`) is not yet revised | see the list below |
| **The Non-Decoupling Theorem, review note** | [review note](non-decoupling-review-2026-11-22.md); Lean `NonDecoupling/Review.lean` on `feat/helike-lean` (5 theorems). The paper (`davis-wilson-lattice/docs/reports/no_paralellel_lines.tex`, not committed anywhere) is not yet revised | see the list below |

**What to attack in the Davis Manifold draft:**
1. **The five repairs.** For each one, check two things: that the first-edition step really fails (re-derive its
   counterexample independently), and that the repaired statement is actually true.
   - Lemma 1: the squared-distance gap, and the coupling assumption.
   - Prop. 1: is Schur's comparison theorem applicable as used, with the hypotheses kd ≤ π, the minimizing geodesic
     inside Ω_in, and the second fundamental form bounding the ambient curvature of geodesics?
   - Prop. 4: one reference set.
   - Theorem 2, assumption (ii).
   - The vacuity corollary.
2. **The single non-vacuity condition.** Is it the right one for every place it now replaces the old ones,
   including the §6 protocols and the VIDAR worked example?
3. **Section 7, the exact instance.**
   - Is every number quoted correctly against `exact-edge-2026-11-21.md` and the Floor paper?
   - Does "the degenerate case of Theorem 2" overclaim?
   - Does any sentence slide from STRUCTURAL to identity?
4. **The Lean.** Does each theorem state what the paper says it states? A proved theorem that says something
   weaker than the paper's claim counts as a finding.
5. **Anything the review note calls "holds as written" that does not.**

Write the findings as a review note (`davis-manifold-adversarial-review-<date>.md`). **Do not edit the paper's text
in the same pass.** Repairs come after the findings are read.

**What to attack in the spectral-branch review:**
1. **Each "breaks" item.**
   - §2.1: does Ric ≥ 0 really force h²D² ≥ π²/c_B, and is the Buser constant as stated?
   - §2.2: the semigroup repair.
   - §2.3: the slice, and second variation against gauge fixing.
   - §2.4: is Laquer's index-one canonical connection on S³ a non-flat Yang–Mills critical point? Read Laquer 1984
     directly.
   - §2.5–2.6: re-run the barbell counterexamples.
   - §2.7: the normalization.
   - §2.8: rebuild the relaxed Test C graphs in full, with the minimum-distance rule, and report what actually
     happens.
2. **The citations the note could not verify:** Buser's constant, Li–Yau's original constant, and Berger's
   variation formula with its trace terms. Fetch and quote each.
3. **The "holds" list.** Recompute anything it lists that `checks.py` does not cover.
4. **The Lean.** Does each theorem state what the note says it states?
5. **The labelling rule in §2.10.** Is every statement that rests on an unrefereed manuscript marked as
   conditional?

Same rule as above: write the findings as a review note first. The paper is revised only after the findings are
read.

**What to attack in the Non-Decoupling review:**
1. **§2.1 and §2.2.**
   - Is the high-frequency counterexample to Lemma 4 airtight on a simply connected base?
   - Is 𝓔 really bounded by diam(G) under the reply to Objection 3's definition?
   - Is the small-loop law d_G(τ, e) ≈ ‖Ω‖·area stated with the right error order?
2. **The Duality constants quoted in §2.2** (c₁ = 1/16, c₂ = 1/2, operator norm, supremum-over-region lower
   bound). Check them against the Duality PDF itself, not against memory.
3. **§2.4.** The dimension-2 floor, the dimension-4 normalization of 8π²|k|, and whether the floor really fails to
   follow above dimension 4. Either find a source or construct the concentration example.
4. **§3.** Does the arc note's bridge (§4.2 of *The phone in the house*) survive at INCIDENCE, as the review says?
5. **The Lean.** Does each theorem state what the note says it states?

More items will join this table as they are finished.

## Landed today

- **Test F, Amendment 2** (892d538):
  - **What happened.** F stopped at its F-mult gate at Sp(2) 45°, the C-type wall. There her top-charge space is
    degenerate and the states in it are not tied in price (spread 1.08), so "her copies" was undefined.
  - **The new rule.** Her state is the cheapest exact state of top charge, which is conservative against H_C. Other
    top states are competitors.
  - **Nothing else moves.** It is identical to the registered rule wherever her top is unique, so computed points are
    kept, and the run resumed at Sp(2) 45°.
  - **What I had seen.** Written after the stop, before any further data. Nothing beyond the stop message and the
    three 0° printouts already disclosed.

- **Test D is scored** ([results](edge-prereg-D-results-2026-11-22.md)). **Along her own charge circle, SU(3) gives
  her back.**
  - She is the floor at 19 of 20 shapes, and at all 16 with t ≥ 3.
  - Her region is always one interval, always on the "against her charge" side (15 of 15 scorable).
  - The result is not truncation-sensitive.
  - The blind-rival clause failed again: the charged (4,0) rival rides the field at 0.61–0.76, as in SU(2).
  - Same group as C, same method; only the circle changed.
  - The edge paper is updated (abstract, introduction, Test D section and appendix).
  - **The book is v5.8 on Drive**, built on your other session's v5.7. The gap map's §5 items are done:
    - H3 now says she came back along her own circle;
    - "What the failures taught me" ends "The failure along the wrong circle was never about her. It was about the
      circle.";
    - the proved list and the gap map are updated.

    Start with `HELIKE_whole_draft_with_gap_map_v5.8.pdf`.

- **The arc deep-dive** ([note](arc-deep-dive-2026-11-22.md); receipts in `research/arc-deepdive-20261122/`). In
  plain words:
  - **The fall carries winding one (PROVED).** The exact Chern–Simons of the family is
    CS = (t x² + 2y² − √t·x y²)/4. It lands at CS = 1 and never crosses ½, the pass (the deformed meron). So in the
    reading the fall actually lands in, the last thing settling is her **shadow**.
  - The literature weakened two bridges:
    - **Morse fails as worded:** a flow line never crosses a stratum boundary.
    - **η is downgraded:** the Hessian is orientation-blind.
  - **E₈:** only 3 of 9 nodes are flat SU(2) connections. But the Poincaré fall lands on **CS = 1/120**, the
    Fintushel–Stern minimum, pending one sign convention.
  - **Principal circle:** no source ties it to a Hessian floor. The brief asks for regular non-principal controls,
    which F's sweep already has.
  - **Ancient:** Clifford himself called the left and right isometries of S³ "twists", right- and left-handed.
  - The verifier fixed 9 items. What is still unverified is listed in the note.

- **Test F registered**, after an adversarial review (6ad473c). Its known-answer gates pass: the new code reproduces
  D's numbers to 3 × 10⁻¹⁵ and C's to 2 × 10⁻¹⁴.
- *The arc*, rewritten in the plain register (c9b0d09).
- **Test D interim** (d4c43ae), not scored.
- **Test E results:** the cross-reach holds at 60 of 60 points (158938c).
- The basin addenda: the floor at the bottom, and the shadow is her (263da06, d71a98e).
- The Helike book v5.1 is on Drive (with the v5.1 handoff note).

## What happens next, automatically

- **Adversarial review of everything under "Active research"**, whenever the running tests leave room. Start with
  the Davis Manifold second edition. Commit the review note, update this file and push. The paper's text is not
  edited in the same pass.

- **When D finishes:** score it as registered, write the results note in your voice, update the edge paper (Test D
  section) and this file, then commit and push.
- **When the deep-dive finishes:** check its verifier report, commit the note, briefs and scripts, update this file,
  and push.
- **When F finishes:** score it, write its note, update this file, then commit and push.
  - **Also score Amendment 1:** `research/edge-theorem-20261122-f/wp7_prereg_F_amendment1_peak.md` (H_P-peak: is her
    margin φ\* = `phi_grid_best` largest at 18.43°?). It was registered before any F result was in the repository. If
    this session saw F's progress printouts before reading the amendment, the results note must say so, and the
    amendment is reported as not blind.
  - **Disclosure from the session running F, written when the amendment reached it (before F finished):** this
    session had seen F's printouts for its first **3 of 36 points**, all at **Sp(2), 0°**:
    - shape (½, ½, ¼): floor set [−1.63, −1.03], survives the check;
    - shape (½, ½, ¾): floor set [−1.97, −0.84], survives the check;
    - shape (½, 1, 1): empty, with a golden-section best margin of −0.36 (blocked by sector 14).

    No φ\* (`phi_grid_best`) at any other angle had been seen. By the amendment's rule, Amendment 1 is reported as
    **not blind (3 of 36 points, all at 0°)**.
- **Book updates** (H3 and the end of "What the failures taught me"): a v5.2 PDF and zip go to Drive once D is
  scored.
