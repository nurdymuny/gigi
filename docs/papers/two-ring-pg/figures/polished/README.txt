POLISHED PAPER FIGURES

These plates are used by Figures 1--3 in paper.tex and the rebuilt paper.pdf.
The original figure files remain available. No research calculation or
acceptance gate was rerun to make these illustrations.

Open three_plates_preview.pdf to see all three plates. Individual plates are
also supplied as PDF, SVG, and PNG. PDF/SVG labels and measurement lines are
vector artwork; the translucent 3-D scenes are high-resolution, depth-composited
raster artwork embedded in those files. They are not entirely vector images.

Rebuild from the repository root:
  python docs/papers/two-ring-pg/make_polished_plates.py

Focused checks:
  python -m unittest discover -s docs/papers/two-ring-pg -p test_polished_plates.py -v

Inputs:
  data/rings_3d.json: recorded chart geometry and amplification readings.
  data/cell_views.json: recorded lengths and dihedral angles.
  verification.json: their SHA-256 hashes and numerical/layout checks.

The saved vertex coordinates have six decimal places. Labels are taken from
the measurement receipt, not substituted with measurements recomputed from
rounded drawing vertices. This is the original illustration geometry, not a
fresh full-precision export of the computational mesh.

DEPTH AND TRANSPARENCY

Every image pixel sorts all contributing faces and edges by actual camera
depth, then applies alpha composition from back to front. There is no rule
that draws the central cell on top. Shared geometric faces/edges are counted
once. Their colour belongs to the more central incident cell, solely as an
illustration convention. These colours and opacities are not measured optical
properties or additional scientific data.

The separated views translate each neighbouring tetrahedron along its centroid
ray. They do not stretch, rotate, or resize it. Their gaps are illustrative.
All three separated panels use a common camera and display scale.

Focused tests include independently calculated overlapping-layer colours,
depth reversal, crossing depths within an image, independence from submission
order, duplicate-face rejection by deduplication, rigid camera transformation,
and rejection of corrupted measurements/selection counts.

INTEGRATION INTO THE PAPER

The paper trims the standalone mastheads and outer margins, preserves the
drawings' proportions, and supplies captions matching the current views.
Figure 2 now puts the six edge lengths in a keyed readout and
omits the neighbouring-cell silhouettes. Its oblique view uses translucent
faces, not the original dashed-hidden-edge convention. Figure 3 uses actual
depth composition instead of drawing the central cell over its neighbours.
Its omitted second-ring cells remain as faint context and are not included
in the folded-stencil cell count. No numerical values have been changed.
