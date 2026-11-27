# Handoff: "The Lowest Note" (27 November 2026)

My narrative theory of the Yang–Mills mass gap. It is not a proof. It supersedes my earlier Zenodo posting, which
claimed a proof; that claim is withdrawn, and the PDF says so under the title.

## Files and build
- `statement.tex`: the preamble, the box environment (`claim`, with a status label), and the title.
- `sections/s0-abstract.tex` … `s10-what-it-is.tex`. The order is: problem, geometry, the note of one turn, the small
  room, carriers (s5), fast modes (s6), the room grows, the limit, contents, what the gap is.
- `refs.tex`: every entry verified on 27 Nov (pages, venues, my own DOIs). Every key is cited.
- Build: `pdflatex -interaction=nonstopmode -halt-on-error statement.tex` until references stabilize.
  The accuracy-review build uses MiKTeX pdfTeX 1.40.25, the existing NewPX fonts, and 46 boxes. The former
  mixed-status all-group box is now two boxes; the theory's five-part structure is unchanged.
  Current result after the descriptive additions: 13 pages, no LaTeX warnings or overfull/underfull boxes;
  all pages rendered and visually checked. The prior accuracy-review version was 12 pages.

## The one rule
Every box status comes from the register, `docs/research/closed-set-20261127/CLOSED-SET.md`, and is never stronger
than it. Statuses change only when something is proved or fails, never to make a sentence read better. A claim that
something required is missing must quote the Jaffe–Witten clause it comes from.

## Checks and scope
- Original citation pass: each entry and each use was reported checked. The accuracy review revisited the primary
  sources relevant to its corrections; it did not independently re-prove every cited result.
- Completeness: every register line 1–28 (line 29 is merged into 23) and every Clay clause has a box. The two tables
  in §10 cite boxes by `\ref`, so they cannot go stale.
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
7. **Layout.** The existing 10pt type and fonts are retained. References begin after the statement-map float so the
   tables no longer interrupt the bibliography.

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

## Format assessment
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

The follow-up preserves the existing layout; it does not silently recast the narrative into a journal submission.

## Cut for length, for the blog post
GIGI query counts; the fourth control's details; R2 margins; the shell-estimate formula; the anharmonic slope check.
