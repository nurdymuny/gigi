# Zenodo record: version 3

Paste-ready fields for the third version of the existing record.

1. Open the version 2 record (doi:10.5281/zenodo.24375254) on Zenodo and click **New version**. Zenodo then keeps
   all versions under the concept DOI 10.5281/zenodo.24328085, which the paper cites.
2. Paste `description.html` into Zenodo's HTML source view (the editor's **Source code** button).

**Files.** A new version starts with the previous version's files. Delete those, then upload these four from the
upload folder built by `build_package_v3.py`:

1. `paper.pdf`: the paper, 46 pages, identical to `docs/papers/two-ring-pg/paper.pdf` at tag
   `reading-the-curve-v3`.
2. `reading-the-curve-v3-latex.zip`: the LaTeX source, figures, tables and style file. Compiles with `pdflatex`.
3. `reading-the-curve-v3-code-and-data.zip`: the scripts, receipts, specifications and engine files behind the
   paper's numbers, plus the frozen inputs (verified against their recorded hashes). Not the GIGI engine as a
   whole; that stays in the repository.
4. `MANIFEST.txt`: SHA-256 of each file, the commits, and the full archive listing.

**Title**
Reading the Curve, Not Counting the Points: A Two-Ring Petrov–Galerkin Curvature Force for Discrete Yang–Mills
Dynamics on the Poincaré Homology Sphere

Version 3 changes the subtitle. Version 2 read "A Consistent, Stable Petrov–Galerkin Curvature Force. An Abelian
Control for Discrete Yang–Mills Dynamics on the Poincaré Homology Sphere".

**Creator** Davis, Bee Rosa. ORCID 0009-0009-8034-4308. Affiliation: Davis Geometric. Version 2 lists Brown
University; Zenodo's metadata is per version, so set Davis Geometric here.

**Version** 3

**Publication date** the day you publish

**Resource type** Publication / Preprint

**Language** English

**Licence** Creative Commons Attribution 4.0 International (as versions 1 and 2)

**Keywords**
lattice gauge theory; Yang–Mills; Whitney forms; finite element exterior calculus; Petrov–Galerkin; curvature
recovery; constrained least squares; Poincaré homology sphere; binary icosahedral group; SU(2); Berger sphere;
Ricci flow; spectral gap; Weitzenböck identity; computer-assisted proof; numerical range; Cayley map;
Lie-group integrators; GIGI

**Notes** MSC 2020: 65N30, 65N12, 81T13, 81T25, 58E15, 58J50, 53C30, 65G30. This work received no funding. The
author declares no competing interests.

**Related identifiers**
- *is new version of*: 10.5281/zenodo.24375254 (version 2). Zenodo sets this automatically with New version.
- *references*: 10.5281/zenodo.24386614, *The Floor Theorem for the Yang–Mills Hessian at the Berger Field*
  (cited in the paper as `davisberger`).
- *references*: 10.5281/zenodo.24375338, *Three Ways Across a Turn* (cited as `threeways`).
- *is supplemented by*: https://github.com/nurdymuny/gigi/tree/reading-the-curve-v3 (software).
- *is supplemented by*: https://github.com/nurdymuny/gigi/tree/reading-the-curve-v3-engine (software: the SU(2)
  engine verbs).
- *references*: 10.5281/zenodo.18895462, 10.5281/zenodo.17642422, 10.5281/zenodo.19477365 (listed under "Related
  work by the author" in the description).

**Software fields** Repository URL https://github.com/nurdymuny/gigi/tree/reading-the-curve-v3; languages Python,
Rust.
