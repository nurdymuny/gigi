# Handoff: "The Lowest Note" (27 November 2026)

My narrative theory of the Yang–Mills mass gap. It is not a proof. It supersedes my earlier Zenodo posting, which
claimed a proof; that claim is withdrawn, and the PDF says so under the title.

## Files and build
- `statement.tex`: the preamble, the box environment (`claim`, with a status label), and the title.
- `sections/s0-abstract.tex` … `s10-what-it-is.tex`. The order is: problem, geometry, the note of one turn, the small
  room, carriers (s5), fast modes (s6), the room grows, the limit, contents, what the gap is.
- `refs.tex`: every entry verified on 27 Nov (pages, venues, my own DOIs). Every key is cited.
- Build: `pdflatex statement.tex` twice (TeX Live 2025). Current result: 10 pages, 45 boxes, no warnings.

## The one rule
Every box status comes from the register, `docs/research/closed-set-20261127/CLOSED-SET.md`, and is never stronger
than it. Statuses change only when something is proved or fails, never to make a sentence read better. A claim that
something required is missing must quote the Jaffe–Witten clause it comes from.

## Already checked
- Citations: each entry and each use, meaning the cited work supports the sentence it is attached to.
- Completeness: every register line 1–28 (line 29 is merged into 23) and every Clay clause has a box. The two tables
  in §10 cite boxes by `\ref`, so they cannot go stale.
- Consistency: the kinetic-term caveat reads the same in §§5, 6 and 10.
- Fixed by hand after the referee: the abstract called Helike's charge "topological". It is a charge under the room's
  rotation symmetry, not a winding number.

## Soft spots worth pushing on
1. **The kinetic O(g²) term** (non-adiabatic mass plus the Coulomb term). It is uncomputed, and it decides both the sign
   of the M3 dressing and H8, the carrier at the unsourced end.
2. **H8 in the physical theory.** It is proved for the undressed reduced operator. Physically q_MC is a gauge copy of
   q = 0, and the charge-0 and charge-2 states tie at harmonic order there.
3. **The large-room end of H3** (one-scale mixing, MS). This is the wall.
4. **The switch at the registered couplings.** The intermediate value theorem brackets it only for g² ≲ 0.0075.
   Elsewhere it needs a control at s₁ ∈ (s*, 1.25).
5. **The Euclidean time–space residue.** On S³ × ℝ this means two bare parameters, the coupling and the speed of light.
6. **Box 43 mixes standings.** It is labelled PROVED but includes cited parts and my unrefereed papers. It may want
   splitting.
7. **Density.** The layout is tight (10pt, narrow margins) to fit 10 pages. At 0.75in margins it runs about 11 pages.

## Cut for length, for the blog post
GIGI query counts; the fourth control's details; R2 margins; the shell-estimate formula; the anharmonic slope check.
