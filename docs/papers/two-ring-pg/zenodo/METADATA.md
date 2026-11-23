# Zenodo record: version 2

Paste-ready fields for the second version of the existing record. Open the first record on
Zenodo and use **New version**, so Zenodo links the two under the concept DOI. The description
is `description.html`, pasted into Zenodo's HTML source view.

**Upload folder:** `docs/papers/two-ring-pg/zenodo/upload/` (built by `build_package.py` in
this folder; not committed, because the archives are large). Upload the three files below;
`MANIFEST.txt` there (copied to `MANIFEST.txt` beside this file) gives their SHA-256.

**Title**
Reading the Curve, Not Counting the Points: A Consistent, Stable Petrov–Galerkin Curvature
Force for Discrete Yang–Mills Dynamics on the Poincaré Homology Sphere

**Author** Bee Rosa Davis (Davis Geometric)

**Version** 2 (set in Zenodo's version field only; the paper itself carries no version label)

**Publication date** the day you publish (fields were prepared 2026-11-19)

**Resource type** Publication / Preprint

**Language** English

**Files** (New version on Zenodo starts with the previous version's files: delete those, then
upload these three)
1. `paper.pdf` — the paper, 30 pages; identical to `docs/papers/two-ring-pg/paper.pdf` at git tag
   `reading-the-curve-v2`.
2. `reading-the-curve-v2-latex.zip` — the LaTeX source, figures, tables and the shared style
   file, laid out so `pdflatex` compiles it as is (checked: it reproduces `paper.pdf`'s text).
3. `reading-the-curve-v2-code-and-data.zip` — the research scripts behind the paper's numbers
   and every module they import, the figure and table scripts, receipts, frozen inputs, the
   Cayley step's iteration matrices, and the specification and results of each experiment, at
   their repository paths. Not the GIGI engine: that stays in the repository. Checked by
   unzipping into an empty folder and rerunning the n = 2 two-ring flow: it reproduces the
   committed result to 5×10⁻¹⁴ with the same Newton counts.

**Keywords**
lattice gauge theory; Yang–Mills; Whitney forms; finite element exterior calculus;
Petrov–Galerkin; curvature recovery; least squares; Poincaré homology sphere;
binary icosahedral group; SU(2); Ricci flow; Berger metric; numerical range;
Cayley map; Crank–Nicolson; Lie-group integrators; GIGI

**Related identifiers**
- *is new version of*: https://doi.org/10.5281/zenodo.24328086 (first version, 2026-11-18;
  concept DOI 10.5281/zenodo.24328085). Zenodo links them automatically when "New version" is used.
- *is supplemented by*: https://github.com/nurdymuny/gigi (software; tag `reading-the-curve-v2`)
- *references*: *Three Ways Across a Turn: Chord, Arc and Tangent*,
  https://doi.org/10.5281/zenodo.24375338 (published; cited in the paper as `threeways`)
- *references*: *The Double Cover Principle*, https://doi.org/10.5281/zenodo.18895462
- *references*: *The Geometry of Sameness*, https://doi.org/10.5281/zenodo.17642422
- *references*: *Zero Does Not Exist*, https://doi.org/10.5281/zenodo.19477365
  (these three are listed under "Related work by the author" in the description, as in the
  *Three Ways* record; the paper itself cites only *Three Ways*)

**Check before publishing**
- *Which commit the first version was.* "What this version adds" treats the first version as the
  paper as it stood at the LinkedIn post (commit `d3f167d`, 2026-11-18 11:20): Abelian two-ring
  result, the second-ring fold, the SU(2) recovery, and the SU(2) instantaneous force and its
  spectrum at n = 1, 2. If the first upload was taken at a different commit, adjust that list.
- *Licence.* Use the same licence as the first version.
