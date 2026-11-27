# Handoff: "The Lowest Note" (27 November 2026)

My narrative theory of the Yang–Mills mass gap. It is not a proof. It supersedes my earlier Zenodo posting, which
claimed a proof; that claim is withdrawn, and the PDF says so under the title.

## Files and build
- `statement.tex`: the preamble, the unboxed statement environment (`claim`, with a status label), and the title.
- `sections/s0-abstract.tex` … `s10-what-it-is.tex`. The order is: problem, geometry, the note of one turn, the small
  room, carriers (s5), fast modes (s6), the room grows, the limit, contents, what the gap is.
- `refs.tex`: every entry verified on 27 Nov (pages, venues, my own DOIs). Every key is cited.
- Build: `pdflatex -interaction=nonstopmode -halt-on-error statement.tex` until references stabilize.
  The accuracy-review build uses MiKTeX pdfTeX 1.40.25, the existing NewPX fonts, and 46 boxes. The former
  mixed-status all-group box is now two boxes; the theory's five-part structure is unchanged.
  The descriptive additions produced 13 pages; the initial paper-format revision was 23 pages, with no LaTeX
  warnings or overfull/underfull boxes. All pages were rendered and visually checked. The prior accuracy-review
  version was 12 pages. The threshold revision below was 24 pages; the introductory doublet revision is 26 pages.
  The change in length comes from larger type, spacing and readable supporting material, followed by the threshold
  discussion and the illustrated introduction.
- `sections/appendix-a-register.tex`: the status synopsis and complete register/Clay maps. Statement numbers are
  generated from stable labels; the central conjecture is now Statement 4, beside the definitions in section 2.
- `sections/s11-availability.tex`: collected source, data and evidence links pinned to the mathematical baseline.

## The one rule
Every statement status comes from the register, `docs/research/closed-set-20261127/CLOSED-SET.md`, and is never stronger
than it. Statuses change only when something is proved or fails, never to make a sentence read better. A claim that
something required is missing must quote the Jaffe–Witten clause it comes from.

## Checks and scope
- Original citation pass: each entry and each use was reported checked. The accuracy review revisited the primary
  sources relevant to its corrections; it did not independently re-prove every cited result.
- Completeness: every register line 1–28 (line 29 is merged into 23) and every Clay clause has a statement. The two
  tables in Appendix A cite statements by `\ref`, so renumbering does not silently invalidate the map.
- Consistency: the roles of the local harmonic mode, two-well state and physical channel are distinguished in §§5,
  6 and 10; a conjecture remains a conjecture. This is a descriptive theory, not an acceptance-gate report.
- Fixed by hand after the referee: the abstract called Helike's charge "topological". It is a charge under the room's
  rotation symmetry, not a winding number.

## Open parts of the description
1. **The kinetic O(g²) term** (non-adiabatic mass plus the Coulomb term). It contributes to the full splitting at that
   order. Computing it alone is not yet known to decide either the dressed ordering or physical H8.
2. **H8 in the physical theory.** Harmonic ordering in the undressed reduction is derived; its registered finite-g
   claim remains conjectural. Physically q_MC is a gauge copy of q = 0, and the physical channels tie at harmonic order.
3. **The large-room end of H3** (one-scale mixing, MS). This is the wall.
4. **The switch at the registered couplings.** The intermediate value theorem uses established opposite orderings
   at a common coupling. The value 0.0075 is a first-order estimate, not itself such a bracket.
5. **The Euclidean time–space residue.** On S³ × ℝ this means two bare parameters, the coupling and the speed of light.
6. **All groups.** The split boxes distinguish cited compact-group lattice results from the electric note and
   embedded doublet. The latter is not automatically the floor of the additional colour modes.
7. **Layout.** NewPX is retained at 11pt with 0.85-inch margins. References are 9.5pt in two columns, after the
   appendix's full-width tables. The six central displayed equations have section-based numbers (four original,
   two in the new threshold statement).

## Accuracy-review record
- `REVIEW-2026-11-27.md` explains the changes, with finite counterexamples and source links. The purpose is to
  describe the theory accurately, not to make its conjectures satisfy a proof gate before they can be discussed.
- The original accuracy review ran 14 small algebra, saved-receipt and document checks, retained in
  `research/lowest-note-review-20261127/`. No solver or full GIGI suite was run for this editorial task.
- `CLOSED-SET.md` logs the narrowed readings. Earlier referee notes and old proof drafts remain historical records;
  their broader wording is not silently treated as the current statement.

## Descriptive follow-up: topology, geometry and quantum dynamics
- The opening, geometric framing, carrier discussion and conclusion now describe their interaction, not topology
  assigning an energy alone. The proposed topology-leaning carrier remains central; an excitation is distinguished
  from completing a winding change or paying a classical barrier height.
- Three mathematical connections are explicit in section 5: the generalized small-oscillation problem with a
  kinetic metric; the squared Hessian identity at a critical point of W; the vacuum-orthogonal variational gap.
  These explain the existing objects. They do not alter the model, registered values or box statuses.
- Section 6 adds the first-order frequency shift from both potential and kinetic-metric variations, connecting
  this interpretation directly to the existing potential-only dressing calculation without inventing a kinetic result.
- Added van Baal's 1992 configuration-space discussion, alongside the existing finite-volume, Witten and spectral
  references. No priority claim or new proof claim is attached to these standard connections.
- The checker now has 18 checks, adding kinetic-metric dependence and coordinate invariance, the Hessian identity
  at both wells (and its failure away from a critical point), vacuum-orthogonal minimization, and the frequency
  derivative checked against an exact characteristic-polynomial derivative with neither contribution dispensable.
  New receipts and the build log are in `research/lowest-note-mechanism-20261127/`; `prebuild` references the prior
  PDF, `final` records the pre-sync rebuild, and `post-sync` records the published PDF hash after incorporating
  the concurrent abstract correction. Both descriptions retain the competitor as the downhill Chern--Simons
  direction at the winding-one vacuum, not the straight Maurer--Cartan tangent. The variational discussion also
  distinguishes a unique vacuum from a degenerate vacuum space; the corresponding finite example is checked.

## Format assessment (before the authorized revision)
The appropriate description is a **conjectural framework supported by reduced-model analysis**, not a proof paper
and not just an isolated conjecture. It already contains motivation, definitions, derived identities, computations,
related work and open connections. The present layout is readable as a narrative working note. For a journal-style
version, the recommended changes are:

1. Add a self-contained abstract of roughly 150-250 words; retain the five-point narrative as the introduction or
   overview. This length is an editorial recommendation, not a universal rule.
2. Present the central conjecture once, with the model, domain and meaning of its terms immediately nearby.
   Keep supporting propositions, computational observations and conjectures visibly distinct without enclosing
   nearly every paragraph in a status box. Put the full evidence register and Clay clause map in an appendix.
3. Give the reader one compact related-work discussion explaining what is inherited and what this account adds.
   Existing citations are substantial but distributed across the story.
4. Separate derivations, numerical evidence and interpretation. For numerical statements, link the data, code,
   fixed revision and methods. A data/code availability paragraph would collect those links.
5. Retain the NewPX font. Consider 11pt body text, less compressed margins, 9pt or larger references in one or two
   columns, and numbered equations for the central relationships. These are readability choices, not universal
   submission requirements. Keep the existing correction/version notice accessible.
6. End with implications and concrete discriminating predictions, not a repetition of all status labels. Define
   personal names such as Helike beside their mathematical meaning; descriptive first-person prose can remain.
7. Before a specific journal submission, check its requirements for affiliations, funding/conflicts, data/code
   availability and assistance disclosures. APS currently requires disclosure of substantive AI assistance;
   this is separate from authorship. This assessment adds no author, coauthor or submission declaration.

There is no single mandatory conjecture-paper template. APS and AMS give different abstract conventions, and a
specific venue's instructions govern a submission. APS asks for a self-contained one-paragraph abstract without
displayed equations or numbered references; AMS allows multiple paragraphs and unnumbered displays but likewise
discourages citation/reference commands in the abstract. APS also accepts a PDF for initial review rather than
making a particular template the whole criterion. Format does not guarantee acceptance or respectful reception.

Sources checked 27 November 2026:
- https://journals.aps.org/authors/style-basics
- https://www.ams.org/arc/journals/packages/proc/proc_amslatex/Author_Handbook_Journals.pdf
- https://americanphysicalsociety.zendesk.com/hc/en-us/articles/45285501563675-Article-Templates

That assessment initially preserved the layout. The subsequent authorized revision applies the structural and
readability changes below without turning the narrative into a proof submission.

## Paper-format revision and verification

- Contract: `FORMAT-2026-11-27.md`; fixed mathematical baseline
  `df949c7a69aae59b873230aabf9ccab7e5821910`.
- A self-contained abstract opens the paper; the original five-point account is retained in the introduction.
  Related work is collected there. The central conjecture appears once beside its scope and geometric definitions.
- All 46 statement bodies and statuses survive, with reference wording updated from "Box" to "Statement".
  Every baseline math fragment, including repeated occurrences, survives. The bibliography is unchanged.
- Statements are unboxed. The status synopsis and complete maps are in Appendix A; the conclusion retains the
  discriminating predictions and first-person account. No author, coauthor, affiliation or submission declaration
  has been added. The existing correction notice remains.
- Eight format/preservation checks supplement the 18 algebra, evidence and document checks. Before editing,
  four formatting checks failed. In-memory mutation controls reject changed mathematics, promoted statuses and
  duplicated statements. They are editorial checks, not physics acceptance tests.
- Receipts and raw logs are retained in `research/lowest-note-format-20261127/`. Early folders describe intermediate
  source/PDF combinations, not the released artifact. `release-format` and `release-review` identify the final PDF
  and their respective checked sources; `final-pdflatex.log` records the clean build. The earlier `final-*`
  receipts precede the final whitespace cleanup. The `preservation` folder has only a test log because its PDF
  was absent after a failed first build; no receipt was issued for that pass.
- Final layout: 23 pages; all pages checked for text, equations, statement headings, appendix table continuations,
  reference readability and clipping. No LaTeX, overfull or underfull warnings. No solver or full engine suite run.

From the repository root, choose a distinct new directory for each command:

```text
python scripts/check_lowest_note_review.py <new-review-output-dir>
python scripts/check_lowest_note_format.py <new-format-output-dir>
```

From `docs/papers/ym-theory-statement`, run
`pdflatex -interaction=nonstopmode -halt-on-error statement.tex` until references stabilize.
Then render the PDF with `pdftoppm` for visual review. A successful algebra check alone does not verify typesetting.

## Threshold follow-up after the new edits

- Reviewed incoming revision `d627989e`; the new threshold statement is retained as COMPUTED, with the
  kinetic coefficient still OPEN. All 46 earlier statement bodies and statuses remain intact; there are now
  47 statements and six numbered equations.
- `THRESHOLD-REVIEW-2026-11-27.md` records the arithmetic, source distinctions and reasons for the sharpening.
  The rounded kinetic thresholds 7.33 and 3.19 replay correctly. They are local linear-estimate thresholds,
  not certified finite-coupling ordering or a physical switch.
- The switch derivative now evaluates coefficients at the harmonic root; the control-point substitution is
  retained as an explicit estimate. Its constant slope captures the trend, not the three registered values
  exactly. The model, registration and acceptance tolerances are unchanged.
- Local Born--Huang power counting, charge-block treatment and possible equilibrium displacement are explicit.
  No new claim of global ordering or a uniform cutoff bound is attached to the saved calculation.
- Checks: 26 focused algebra/evidence/document checks and eight format/preservation checks. The added wording
  check was red before the passage changed; numerical mutation controls reject altered separation, potential
  coefficient and kinetic sign. Full details and source/PDF hashes are in
  `research/lowest-note-threshold-review-20261127/release-review/` and `release-format/`.
- Rebuilt with stable references, no LaTeX warnings or overfull/underfull boxes. All 24 pages rendered and
  visually inspected, with the revised equations on pages 13-14 examined at full size. No solver or engine suite run.

## Introducing Helike and drawing the actual doublet

- The first page now introduces Helike before the five-point proposal: a family of field-deformation patterns,
  with the eigenvector distinguished from its eigenvalue and from a full quantum state. This keeps my voice and
  points to the existing floor domain rather than making a new floor claim.
- Page 2 explains charge 2 as a rotation law, not electric charge, beside the actual homogeneous doublet.
  Page 3 begins the five-point proposal. The diagram is a component map, not an embedding of the sphere or a
  material ribbon: spatial inputs map to internal colour outputs in a declared orthonormal frame.
- Figure matrices: D1 = diag(0,1,-1), D2 has only entries (2,3) and (3,2) equal to 1. Rows are space and columns
  are colour, so evaluation on a spatial vector v is D-transpose times v. Both send e1 to zero and have Frobenius
  norm sqrt(2). The caption distinguishes this matrix norm from global L2 normalization and higher harmonics.
- The action is R(theta) D R(theta)-transpose, with positive rotation from direction 2 toward direction 3.
  It gives D1 -> cos(2 theta) D1 + sin(2 theta) D2. The plotted sequence at 0, 45, 90, 135 and 180 degrees is
  D1, D2, -D1, -D2, D1. The sign at 90 degrees is retained, not silently identified with the original pattern.
- Reproducible generator: `scripts/build_lowest_note_doublet.py`; it uses NumPy, Matplotlib and the existing
  TeX Gyre Pagella fonts located through `kpsewhich`. The paper embeds the vector PDF. Matching SVG and PNG
  are included alongside JSON containing the exact matrices, plotted vectors, normalization and artifact hashes.
  Generated SVG and JSON use canonical LF line endings, enforced by their local `.gitattributes`, so their
  byte hashes survive Windows and Unix checkouts.
- `scripts/check_lowest_note_doublet.py` runs nine focused checks: actual entries and vector images, the symbolic
  charge law, normalization, signed return, gauge orthogonality, independent potential-Hessian calculations at
  t = 1, 3 and 7, deliberate bad-data controls, introduction placement and preservation against `1d7ce3e7`.
  All 47 incoming statements, their statuses and every incoming mathematical fragment are preserved.
- The introduction check was red before the text was inserted; geometry checks passed. Five deliberate changes
  (a diagonal sign, a missing off-diagonal entry, a wrong output arrow, reversed phase orientation and a false
  quarter-turn return) are rejected. These checks validate this illustration, not the spectral-floor theorem.
- Verification: nine doublet checks, 26 earlier review checks and eight format checks; artifact mode also verifies
  generator and figure hashes and the first three PDF pages. The final PDF has 26 pages, stable references,
  no LaTeX warnings or overfull/underfull boxes, and all pages visually inspected. No full engine or solver run.
- Logs and receipts: `research/lowest-note-doublet-20261127/`. The `red` and `edited` folders are intermediate;
  `release-*` checks precede the text-artifact portability fix. The final `release-v2-doublet`, `release-v2-review`
  and `release-v2-format` identify the released sources and PDF. The raw final build log
  is retained. No new author, coauthor or attribution is added.

Rebuild and verify from the repository root, using a new output directory for each check:

```text
python scripts/build_lowest_note_doublet.py
python scripts/check_lowest_note_doublet.py <new-doublet-output-dir> --artifacts
python scripts/check_lowest_note_review.py <new-review-output-dir>
python scripts/check_lowest_note_format.py <new-format-output-dir>
```

After regenerating the figure, rebuild `statement.tex` twice (or until references stabilize) before the artifact
check. The checker deliberately rejects an old PDF that lacks the opening spread. Use `pdftoppm` for visual review.

## Cubic-quartic closure inside H7

- Contract and full proof are `docs/research/h7-cubic-quartic-contract-2026-11-27.md` and
  `docs/research/h7-cubic-quartic-proof-2026-11-27.md`. The baseline before this change is `1ba75baa`.
- Former Sub-conjecture A_g is closed at alpha = 3/4 by a deduction from Austing--Wheater,
  hep-th/0310170v2, equation (2.2). The note spells out the Hermitian/anti-Hermitian dictionary,
  fixed invariant norm, global constants, rescaling, commuting case and operator realization.
- The reduced potential has the same positive-kappa window and the bound
  V >= (kappa/2)||q||^2 - 27 S^4 c_g^4/(32 g^2). A nonnegative polynomial factorization supplies the constant.
  Compactness follows on the natural full-space Friedrichs form and its reducing symmetry sectors.
  This c_g is not H7's gap constant. No group-uniform estimate or new carrier ordering is asserted.
- There are now 48 statements: the new Statement 47 is proved by deduction; the full H7 is Statement 48
  and remains CONJECTURE. The authoritative register records only this partial closure.
  The fold and other hypotheses remain at their previous standing. Historical referee files are not rewritten.
- Preservation checks allow one exact change to H7's evidence sentence, the named new statement, and one exact
  bibliography addition. Every baseline mathematical fragment survives; unrelated statement bodies/statuses
  still have to match their baselines. Promoting H7 is explicitly rejected.
- New checker: `python scripts/check_h7_cubic_quartic.py <new-output-directory>`.
  Twelve checks cover exact algebra, scope and five rejected bad contracts. These do not replace the cited
  global theorem or constitute a proof-assistant verification. No simulation, eigensolve or engine suite ran.
- Receipts: `research/h7-cubic-quartic-20261127/`. `red-v2` has the two expected document failures before
  the proof; `preservation-red` records rejection of the unregistered statement/bibliography changes.
  The initial `red` directory has no receipt: a Windows stdout encoding failure interrupted that first
  harness invocation; it is not a proof failure or a passed gate. Logging now precedes UTF-8 console output.
  Intermediate `*-edit` receipts do not describe the release. Use the `release-*` receipts for final hashes.
- The rebuilt PDF has 27 pages, stable references, and no LaTeX, overfull or underfull warnings.
  The new proof is on pages 21-22; H7 is explicitly still conjectural on page 22. Tables and reference 64
  identify the partial result. All pages were rendered; changed pages were visually inspected.
- The mathematical argument depends on a published global bound, not on the D=3 partition integral
  converging. This is an application to my reduced-model requirement, not a novelty claim for that inequality.

## The documented Wick-triangle conjecture

- Baseline before this change: `36eb3b4f9e240f2fa3d226dc11f2111dcf3a8c6d`.
  Contract and proof: `docs/research/h6-wick-triangle-contract-2026-11-27.md` and
  `docs/research/h6-wick-triangle-proof-2026-11-27.md`.
- H6 Step 6's explicit prediction T_O != 0 is refuted in its free Euclidean R4 setting.
  The field-strength covariance exchanges the two Hodge halves; any three-covariance product has trace zero.
  This holds for every separated triple. Contact terms and interacting corrections are not covered.
- The four-point alternative already named in H6 is nonzero. With the same canonical O = sum :F_mu,nu^a F_mu,nu^a:,
  its connected coefficient at 0, L e0, 2L e0, 3L e0 is 676 dim(g)/(27 pi^8 L^16), L > 0.
  This is an exact Wick calculation, not a numerical spectral result. Its transfer to Theta remains conditional on H6.
- Statement 44 uses that coefficient while remaining CONDITIONAL. Statement 42 (H6) remains CONJECTURE.
  The paper still has 48 statements and the register still has seven open top-level hypotheses.
  The full theory is not declared non-free just because a composite field has a nonzero connected function.
- The exact gate has ten checks plus four rejected engine mutations. Five Lean matrix declarations compile
  with only the pinned Mathlib foundational axioms; three hypothesis/sign mutants fail. The propagator and Wick
  dictionary have written proofs and exact symbolic checks, not an end-to-end formal QFT construction.
- Evidence: `research/h6-wick-triangle-20261127/release-02/receipt.json`. The initial red run and the intermediate
  document-marker failure are retained. `publication-red` confirms that the old preservation gates reject this
  substantive correction; the updated gates permit one exact frozen replacement and reject changed coefficients
  or stronger statuses. Unrelated statement bodies remain fixed.
- Reproduction: `python scripts/check_h6_wick_triangle.py <fresh-output-directory>`, plus the existing format,
  review and doublet commands above. The Lean project remains `formal/h7-cubic-quartic` with its existing pins.
  No GPU job, large eigensolve, simulation or full GIGI suite is involved.
- The PDF remains 27 pages after condensing the availability prose to avoid a nearly empty overflow page.
  The mathematical correction is on pages 20-21; its provenance is on page 23. The appendix and references
  retain their statuses and numbering. Final build and visual-review evidence travel with the receipts.

## Cut for length, for the blog post
GIGI query counts; the fourth control's details; R2 margins; the shell-estimate formula; the anharmonic slope check.
