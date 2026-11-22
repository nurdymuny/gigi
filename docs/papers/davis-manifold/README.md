# The Davis Manifold: second edition (draft)

| file | what it is |
| --- | --- |
| `the_davis_manifold_2025.tex` | the 2025 first edition, unchanged. Its `references.bib` was never found, so its bibliography builds empty. |
| `the_davis_manifold_v2.tex` | the second edition (draft, 22 November 2026) |
| `the_davis_manifold_v2.pdf` | the built second edition: 63 pages, with no LaTeX errors, overfull boxes or undefined references |

## What the second edition changes

Appendix D of the paper lists every change. The argument for each is in
`docs/research/davis-manifold-review-2026-11-22.md`.

- **Five steps repaired:**
  - Lemma 1 (InfoNCE gives a gap in squared distance);
  - Proposition 1 (the true distortion is quadratic, (kL)²/24, proved via Schur's comparison theorem);
  - Proposition 4 (one reference set);
  - one step of Theorem 2 (the high-risk event defined exactly);
  - Corollary 1 (vacuity defined by the bound itself).
- **One non-vacuity condition,** (1 − ε)κ_soft > (1 + ε)κ_hard, replaces the three in the first edition. In the
  existence theorem it becomes ε < 1/3.
- **New Section 7:** the floor certificate on the Berger spheres, read as a fully certified Davis system. It is
  the degenerate case of Theorem 2. The bridge is tagged STRUCTURAL: the shape is shared, and no common theorem
  is claimed.
- **Layout:** the first edition's `samepage` boxes ran up to 287 pt past the page bottom. They are removed.

## Machine checks

The deterministic steps are checked in Lean in `docs/papers/berger-spectrum/lean/DavisManifold/Review.lean`, on
branch `feat/helike-lean`: 13 theorems, using only `propext`, `Classical.choice` and `Quot.sound`.

## Build

```
pdflatex the_davis_manifold_v2.tex   (twice)
```
