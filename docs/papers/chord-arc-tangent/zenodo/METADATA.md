# Version 2.1 (new version of the published record)

Files are in the Drive library, *Bee Davis — Mathematics Library / 02 — Geometry and Computation*, rebuilt by
`python scripts/make_paper_packages.py OUT threeways` from tag `floor-theorem-v3.3` (which holds Three Ways 2.1).

1. Open the published record (doi:10.5281/zenodo.24375338) and click **New version**.
2. Replace `paper.pdf` with the 2.1 PDF (19 pages), and add `three-ways-v2.1-latex.zip` (source, figures,
   `verify.py`, the per-face angle receipt) and `MANIFEST.txt`.
3. Paste `description.html` through **Source code**. Set the version to **2.1**.
4. What changed, for the version note: corrections after the 19 November posting (energies "to leading order", the
   two-circle and half-angle cases, the force measured directly: 0.090%, 0.0084%, 0.0021% and errors of 4.7%–36%), and
   Section 11 separated into exact statements and a labelled speculative picture (indivisibility is an added
   assumption; 1 − cos θ can be arbitrarily small; SU(3) is curved but not round).
5. Publish.

---

# Zenodo record: Three Ways Across a Turn

**Published: https://doi.org/10.5281/zenodo.24375338** (DOI 10.5281/zenodo.24375338).

Paste-ready fields for a new Zenodo record. The description is `description.html`, pasted into
Zenodo's HTML source view. Post this record first: its DOI then goes into *Reading the Curve*
(bibliography entry `threeways`; done).

**Title** Three Ways Across a Turn: Chord, Arc and Tangent

**Author** Bee Rosa Davis (Davis Geometric)

**Publication date** 2026-11-19

**Resource type** Publication / Preprint

**Files**
1. `paper.pdf` — the paper in English (19 pages; `docs/papers/chord-arc-tangent/paper.pdf`)
2. `paper_ru.pdf` — the Russian edition (21 pages; `docs/papers/chord-arc-tangent/ru/paper_ru.pdf`)
3. `perelman_ru.pdf` — *Thank You, Mr. Perelman* in Russian (24 pages; kept outside the repo, in
   `~/Documents/three-ways-ru-zenodo/`)

Adding files to the published record needs **New version** on Zenodo (files of a published version
cannot be changed). The description can be edited in place. Add language *Russian* alongside
English.

**Keywords**
chord; arc; tangent; Snellius–Huygens; Archimedes; Wilson action; Manton action;
lattice gauge theory; Yang–Mills; Cayley transform; stereographic projection;
Crank–Nicolson; Lie-group integrators; SU(2); unit quaternions; Bertrand's paradox;
Haar measure; golden spiral; GIGI

**Related identifiers**
- *is supplement to*: *Reading the Curve, Not Counting the Points*, https://doi.org/10.5281/zenodo.24328085
  (concept DOI: resolves to its latest version)
- *is supplemented by*: https://github.com/nurdymuny/gigi (software; scripts in
  `docs/papers/chord-arc-tangent/`)
- *references*: *The Double Cover Principle*, https://doi.org/10.5281/zenodo.18895462 (cited in the paper)
- *references*: *The Geometry of Sameness*, https://doi.org/10.5281/zenodo.17642422
- *references*: *Zero Does Not Exist*, https://doi.org/10.5281/zenodo.19477365

**After the DOI is issued**
1. Replace "(DOI forthcoming)" in the `threeways` entry of
   `docs/papers/two-ring-pg/paper.tex` with the DOI, rebuild that PDF, and use that PDF for
   the *Reading the Curve* upload.
2. Add the DOI under *references* in `docs/papers/two-ring-pg/zenodo/METADATA.md`.
