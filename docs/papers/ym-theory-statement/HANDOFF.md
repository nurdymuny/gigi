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
  Final result: 12 pages, no LaTeX warnings or overfull/underfull boxes; all pages rendered and visually checked.

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
- `scripts/check_lowest_note_review.py` runs 14 small algebra, saved-receipt and document checks. No solver or
  full GIGI suite was run for this editorial task. Receipts are in `research/lowest-note-review-20261127/`.
- `CLOSED-SET.md` logs the narrowed readings. Earlier referee notes and old proof drafts remain historical records;
  their broader wording is not silently treated as the current statement.

## Cut for length, for the blog post
GIGI query counts; the fourth control's details; R2 margins; the shell-estimate formula; the anharmonic slope check.
