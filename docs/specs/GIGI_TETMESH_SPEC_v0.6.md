# GIGI_TETMESH_SPEC_v0.6.md

**Version:** 0.6
**Author:** Bee Davis — Davis Geometric
**Date:** July 2026
**Status:** chapter draft — published as a standalone spec until the book ships
**Book placement:** *GIGI Builds* (Vol 1 of the *GIGI Solves* series) — the tetrahedral-mesh build: **"The Mesh That Audits Itself."** Like every build in the book, it runs on a stock GIGI instance and ships its receipts: the validation harness in this repository (`examples/tetmesh_fiber_harness.py`) regenerates every number in the appendix, and the companion visualization (`examples/tetmesh_visual.html`) renders the build in the browser. Readers who want the substrate story underneath (the Davis Field Equation, bundles, curvature-as-first-class-state) can go one shelf over to *The Geometry of Flight* (Davis 2026, ISBN 979-8-1983-7541-3); this chapter assumes that background and gets straight to the mesh.

**Changes from v0.5:** generalized from a single-recipient handoff draft to a standalone companion note; all GQL examples rewritten in actual GQL (`COVER` / `SECTION AT` / `INTEGRATE`) and executed against a live `gigi-stream` build; per-tet fiber record flattened to GIGI's flat fiber schema, with matrices carried as `vector(n)` fields; the refinement audit trail modeled as a typed events bundle plus materialized `(root, level)` ancestry instead of graph-pattern traversal; `unit_cube_512` corrected to `unit_cube_384` (a Kuhn triangulation has $6n^3$ tets — 512 is not attainable); validation harness added (`examples/tetmesh_fiber_harness.py`) with measured results in the appendix.

## 0. One-line summary

A tetrahedral mesh is a discrete fibered object over its simplicial complex: each tetrahedron carries a fiber of metric, dihedral, classifier, and refinement-state data. A semi-algebraic shape classifier supplies the fiber labels; subdivision reduction sequences supply typed transitions between parent and child fibers. GIGI's engineering payoff is a queryable, streaming AMR audit trail — every refinement event preserves enough certified shape data to detect sliver approach, class proliferation, or failure of a tagged termination invariant.

## 1. The engineering problem

Many serious 3D physics solvers — CFD, structural FEA, MHD plasma (CHIHIRO), electromagnetic — run on tetrahedral or mixed unstructured meshes, and every one of them fights the same three enemies:

1. **The sliver problem.** A tet with a dihedral angle near $0$ or $\pi$ has a stiffness matrix condition number that degrades sharply as the angle degenerates. Shewchuk's analyses make this concrete: small angles hurt conditioning, and badly shaped elements can dominate stiffness-matrix eigenvalues. Sliver detection today is usually scalar or heuristic (aspect ratio, radius ratio, skewness); it does not expose a queryable semi-algebraic classifier of tet shape, and different heuristics capture different failure modes.

2. **Refinement control.** Adaptive schemes (longest-edge bisection, red-green, Kuhn, Maubach, Traxler) subdivide tets to resolve local error. Maubach, Traxler, and the newest-vertex-bisection literature proved key results: finite compatibility chains, finite similarity classes, bounded shape regularity. Naive schemes do not have these guarantees, and the failure modes in production show up as unbounded reachable state, degrading shape-quality lower bounds, or closure blowup.

3. **No queryable structure.** Classical mesh libraries (MOAB, VTK, deal.II, CGAL) store the mesh as flat tables of vertex/edge/face/cell indices. Asking "give me every tet whose classifier cell lies in region $R$" requires a full scan and reclassification. The bundle structure is there in principle and lost in practice.

The mathematics for (1) and (2) largely exists at the level of pure math. GIGI turns the answers into runtime.

## 2. The mathematics as the engine

**Convention.** Throughout, $\alpha_{ij} \in (0, \pi)$ is the internal dihedral angle along the edge shared by faces $i$ and $j$. The normals $n_1, \ldots, n_4$ used in the angular Gram matrix are outward unit face normals, so $n_i \cdot n_j = \cos(\pi - \alpha_{ij}) = -\cos(\alpha_{ij})$.

**Fiedler realizability.** A nondegenerate Euclidean tetrahedron's six internal dihedral angles are encoded by the $4 \times 4$ outward-face-normal angular Gram matrix

$$
A_{ij} = \begin{cases} 1 & i=j \\ -\cos(\alpha_{ij}) & i \neq j \end{cases}
$$

Euclidean realizability requires $A$ to be positive semidefinite of rank exactly $3$, together with the positive-kernel condition corresponding to positive facet areas and facet-area closure ($\sum_i a_i n_i = 0$ with $a_i > 0$). A CAD (cylindrical algebraic decomposition) classifier partitions the feasible semi-algebraic region in angular-Gram coordinates $c_{ij} = -\cos\alpha_{ij}$ (with $\vec\alpha$ retained for reporting and quality metrics) into sign-invariant cells, then quotients by the tetrahedral vertex-symmetry action of $S_4$.

*Implementation convention for the $S_4$ quotient.* Two equivalent implementations exist and are not interchangeable at the code level: either canonicalize each dihedral tuple under the $S_4$ action *before* cell lookup, or store classifier cells as $S_4$-orbits of CAD cells and look up orbit membership. The reference dataset uses the first convention (canonicalize first, then lookup); any external classifier implementation must match, or ship a documented adapter.

Two tets are **classifier-equivalent** iff their angular-Gram coordinate tuples $\vec c \in \mathbb{R}^6$ lie in the same selected CAD cell modulo $S_4$:

$$
T \sim_{\text{CAD}} T' \iff \vec c_T, \vec c_{T'} \text{ lie in the same CAD cell modulo } S_4.
$$

This is a classifier equivalence relation, **not** geometric similarity. Because a CAD cell is generally positive-dimensional in $\vec c$-space, it contains a continuum of non-similar tetrahedra; classifier equivalence is intentionally coarser than similarity. The open mathematical question is whether this coarser relation is minimal, sufficient, or over-refined for the two invariants that matter downstream: sliver detection and refinement termination.

**Subdivision reduction.** Given a subdivision operator $\sigma$ and a classifier cell $C_k$, does the tagged refinement transition system have (i) a finite reachable state set, (ii) a proven closure/compatibility bound, and (iii) a shape-quality lower bound? These three together certify termination in the Maubach/Traxler sense. Existence of such a triple for $\sigma$ restricted to $C_k$ is a **certified termination result** for AMR on that class family.

Note the two-layer structure: an *untagged* class-transition graph is telemetry — cycles in it are diagnostic, not proofs. The *tagged* refinement system, which includes marked-edge state, generation, and compatibility chain data, is where certified termination lives.

## 3. GIGI encoding

**Base space $B$.** The abstract simplicial complex: vertex set $V$, edge set $E$, face set $F$, tet set $\mathcal{T}$.

**Vertex coordinate map.** $x: V \to \mathbb{R}^3$ is the geometric primitive; every fiber record below is derived from it.

**Shape space $\mathcal{X}$.** Scale-invariant coordinates on tet shape:
- Dihedral vector $\vec\alpha \in (0, \pi)^6$
- Normalized edge lengths $\tilde\ell = \ell / L$, where $L = \sqrt{\tfrac{1}{6}\sum_i \ell_i^2}$ is RMS edge length
- Normalized volume quality $q_{\text{vol}}(T) = 6 V(T) / L(T)^3 = \sqrt{\det G_{\text{edge}}} / L(T)^3$, where $V(T)$ is unsigned tetrahedron volume and $\det G_{\text{edge}} = (6V)^2$ so the square root is well-defined and orientation-independent

These are all scale-free; two tets differing only in overall size have identical $\mathcal{X}$-coordinates.

**Fiber $\mathcal{F}_T$ over each tet.** GIGI fibers are flat — fields are scalars, text, timestamps, or dense `vector(n)` values; there are no nested records. The spec's conceptual fiber therefore lands in the engine as a flat schema, with matrices flattened row-major into vector fields:

| Field | GIGI type | Contents |
|---|---|---|
| `id` | `text` (key) | tet id on the base space |
| `level`, `root` | `float`, `text` (indexed) | materialized refinement ancestry (see audit trail below) |
| `edge_lengths` | `vector(6)` | $\ell \in \mathbb{R}_{>0}^6$ |
| `anchor_edges` | `vector(9)` | three edge vectors from the anchor vertex, row-major |
| `edge_metric_gram` | `vector(9)` | $G_{\text{edge}}$, $(G_{\text{edge}})_{ij} = \langle e_i, e_j \rangle$ |
| `face_normal_gram` | `vector(16)` | angular Gram $A$ per §2 convention |
| `dihedrals` | `vector(6)` | $\vec\alpha$, canonicalized ($S_4$-invariant sorted order) |
| `classifier_cell` | `text` (indexed) | CAD cell label modulo $S_4$ |
| `symmetry_orbit` | `text` | $S_4$ orbit id |
| `psd_mode` | `text` | `"exact"` or `"numeric"` — never mixed in one record |
| `psd_numeric_rank`, `psd_min_eig`, `psd_det_abs`, `psd_det_tolerance` | `float` | numeric-mode certificate payload |
| `psd_kernel_positive` | `float` | 1.0/0.0 — kernel vector strictly positive (numeric: with tolerance) |
| `volume`, `radius_ratio`, `min_dihedral`, `max_dihedral`, `q_vol`, `d_bad` | `float` | shape-quality record |

**PSD certificate semantics.** Rank $3$ for an exact Euclidean tet gives determinant exactly $0$; a nonzero determinant on an exact-mode certificate is a bug. Numeric mode carries `psd_det_abs` against `psd_det_tolerance` and evaluates positivity claims with tolerance. The mode lives in `psd_mode` and the two payloads are never mixed in one record.

**Sliver set on shape space.** Explicitly geometric, not a CAD boundary:

$$
\mathcal{B}_{\text{sliver}} = \left\{ T \in \mathcal{X} : \min_i \alpha_i(T) < \epsilon_{\min} \;\text{ or }\; \max_i \alpha_i(T) > \pi - \epsilon_{\min} \;\text{ or }\; q_{\text{vol}}(T) < \epsilon_{\text{vol}} \right\}
$$

with thresholds set by solver-conditioning requirements. Distance-to-badness is measured on shape space, not on angle space alone:

$$
d_{\text{bad}}(T) = \text{dist}_{\mathcal{X}}(T, \mathcal{B}_{\text{sliver}})
$$

using the product Euclidean metric on $(\vec\alpha, \tilde\ell, q_{\text{vol}})$.

*Metric status.* The product Euclidean metric here is an engineering metric on a chosen redundant embedding of shape space — $\vec\alpha$, $\tilde\ell$, and $q_{\text{vol}}$ are not algebraically independent, so this is not a canonical intrinsic metric. Its role is to provide a stable, solver-tunable distance-to-badness score, and different weightings on the three factors are legitimate for different solver contexts. This is not the same as distance to the nearest CAD cell boundary — most CAD boundaries separate perfectly healthy shape classes.

**Refinement morphism and audit trail.** A subdivision scheme $\sigma$ is a functor $\sigma_*: \mathcal{F}_T \to \prod_i \mathcal{F}_{\sigma(T)_i}$ that lifts a base-level combinatorial operation to fiber-level operations on all the derived data. In the engine this lands as two things:

1. a **typed events bundle** — one record per subdivision, `(event_id, parent, child, scheme, tag, level, root)`, each child fiber computed and reclassified in-line at insert time; DHOOM is the wire format the events ride on, and `SUBSCRIBE` delivers them as a live stream to any listening client;
2. **materialized ancestry on the tet fiber** — every tet carries its `root` (level-0 ancestor) and `level`. GQL has no graph-pattern traversal (it is not a Cypher-style graph language), so transitive queries like "descent curve from each root" are expressed as indexed aggregations over `root` and `level` rather than path expressions. This is a deliberate encoding choice, and it is O(index) instead of O(path enumeration).

**GQL queries.** Real GQL, executed against a live `gigi-stream` build (see appendix):

```gql
-- All slivers (distance to geometrically defined sliver set below threshold)
COVER tets WHERE d_bad < 0.05;

-- All tets in classifier cell C12 (indexed -> bitmap lookup, not a scan)
COVER tets ON classifier_cell = 'C12';

-- One tet's full fiber, O(1) point query
SECTION tets AT id = 't00042';

-- Reachable classifier state under refinement, with per-cell quality floor
INTEGRATE tets OVER classifier_cell MEASURE count(q_vol), min(q_vol);

-- Shape-quality descent curve across refinement levels
INTEGRATE tets OVER level MEASURE min(q_vol);
INTEGRATE tets OVER level MEASURE min(d_bad);

-- Audit trail: subdivision events per level for one scheme
INTEGRATE tet_refine_events OVER level MEASURE count(level);
```

The last three are the empirical instrument for the subdivision-reduction program: they produce the reachable classifier state and the shape-quality descent curve directly from mesh telemetry. (Validating this chapter surfaced three `INTEGRATE` defects in the engine — `count(*)` and `count` over text/key fields silently returned an empty result set, multiple same-function measures over different fields all returned the first field's value, and the global no-`OVER` form always returned empty. All three were fixed in the engine alongside this spec — 2026-07-01, `aggregation::group_by_measures` — so on current builds the queries above run as written, including `count(*)` and mixed multi-field measures in one statement. On older builds, count over a numeric fiber field and keep one `min()` per statement.)

## 4. Reference dataset

Reference package: **`gigi_tetmesh_ref_v0.6`** — three meshes of increasing complexity, all in GIGI's flat fiber-bundle schema plus standard `.msh` (Gmsh).

| Mesh | Domain | Tets | Purpose |
|---|---|---|---|
| `unit_cube_384` | $[0,1]^3$, structured Kuhn triangulation, $n=4$ ($6n^3$ tets) | 384 | Ground truth: all tets congruent up to reflection, expected exactly one classifier cell modulo $S_4$ |
| `sphere_in_cube_2100` | Unit cube with spherical inclusion, unstructured | ~2,100 | Realistic geometry, curved boundary produces near-sliver tets, useful for testing classifier robustness and $d_{\text{bad}}$ discrimination |
| `nozzle_section_8400` | Synthetic aerospace nozzle benchmark generated in Gmsh | ~8,400 | Real engineering geometry with anisotropic refinement, useful for stress-testing tagged termination measures |

The `unit_cube_384` mesh, its full fiber computation, and its load path are generated end-to-end by `examples/tetmesh_fiber_harness.py` in the GIGI repository; the two unstructured meshes ship as data. Each record per tet follows the flat schema of §3 (numeric mode shown; exact mode analogous):

```json
{
  "id": "t04271",
  "level": 0.0,
  "root": "t04271",
  "edge_lengths": [1.02, 0.98, 1.15, 1.07, 0.94, 1.11],
  "anchor_edges": [ 9 floats ],
  "edge_metric_gram": [ 9 floats ],
  "face_normal_gram": [ 16 floats ],
  "dihedrals": [0.998, 1.089, 1.107, 1.231, 1.315, 1.402],
  "classifier_cell": "C7",
  "symmetry_orbit": "S4_orbit_id_03",
  "psd_mode": "numeric",
  "psd_numeric_rank": 3.0,
  "psd_det_abs": 2.1e-4,
  "psd_det_tolerance": 1.0e-3,
  "psd_min_eig": 1.4e-16,
  "psd_kernel_positive": 1.0,
  "volume": 0.0842,
  "radius_ratio": 0.71,
  "min_dihedral": 0.998,
  "max_dihedral": 1.402,
  "q_vol": 0.62,
  "d_bad": 0.29
}
```

Distribution: single `.tar.gz` (~40 MB compressed). Placeholder location: `https://davis.geometric/handoffs/gigi_tetmesh_ref_v0.6.tar.gz`.

## 5. Experimental program

Four experiments, each a `.gql` script plus a Jupyter notebook. Experiment A is already reproduced end-to-end by the harness in this repository; B–D are specified against the two unstructured meshes.

**Experiment A — Classifier ground truth.**
Run a Fiedler+CAD classifier on `unit_cube_384`. Expect **exactly one classifier cell modulo $S_4$** if orientation/reflection conventions are quotient-correct. More than one cell indicates either a classifier bug, a symmetry-quotient bug, or an intentional convention split that must be documented. This is a crisp unit test. *(Validated: the harness computes all 384 fibers and finds exactly one canonicalized dihedral class — the Kuhn path simplex, $\vec\alpha = (\pi/4, \pi/4, \pi/3, \pi/2, \pi/2, \pi/2)$ — with every angular Gram passing the rank-3 PSD + positive-kernel certificate. See appendix.)*

**Experiment B — Classifier stress.**
Run the classifier on `sphere_in_cube_2100`. Expect a wide distribution of classifier cells and a long tail of low-$d_{\text{bad}}$ tets near the curved surface. Plot the histogram of $d_{\text{bad}}$ on shape space. Cross-check against classical skewness metrics and test whether the Fiedler classifier separates low-quality tail behavior that scalar heuristics merge into a single "bad" bin. Whether the classifier is strictly finer, coarser, or incomparable to classical measures on real geometry is the empirical question.

**Experiment C — Refinement diagnostic on a naive scheme.**
Apply naive longest-edge bisection to `nozzle_section_8400` for 5 refinement levels. Track:

- growth in the number of distinct classifier cells reached
- descent curve of $\min_T d_{\text{bad}}(T)$ across levels
- reachable state in the tagged transition system (tag = marked-edge state)

Predict one or more of: unbounded classifier-cell growth, monotone degradation of the shape-quality lower bound, no finite bound on tagged reachable state. Any of these is the diagnostic. The untagged class-transition graph alone is not a termination proof; untagged cycles are noted for context but not treated as witnesses. **Run this on the unstructured meshes, not the cube:** on `unit_cube_384` even naive longest-edge bisection is benign, because the Kuhn path simplex is self-similar under bisection — the harness measures a clean period-3 cycle through exactly three shape classes with the quality floor returning to its level-0 value. That is Maubach's bounded-class behavior appearing as telemetry, and it makes the structured mesh a *negative control* for this experiment rather than a subject.

**Experiment D — Certified termination on Maubach/Traxler.**
Apply Maubach or Traxler bisection to the same mesh, same 5 levels. Over the 5-level run, expect empirical evidence of bounded reachable tagged state and preserved shape-quality lower bound. The certification itself comes from the Maubach/Traxler/NVB finite-closure theorem, not from the finite experiment — the 5-level telemetry is corroboration, not proof. The mesh-generation literature (Maubach, Traxler, Bänsch, Bey, Kossaczký, Mitchell) proves the finite-closure and bounded-similarity-class behavior for newest-vertex-bisection families with correctly tagged initial meshes. The structural contribution left open is to recast that marked-edge / reflection-tag invariant as a categorical statement of well-foundedness on the tagged transition system. GIGI supplies the typed transition language in which to express it.

## 6. What GIGI gives back to the mathematician

- **A decidable classifier.** Fiedler + PSD certificate + CAD is fixed-cost per tet once the CAD decomposition is precomputed. Given a compiled point-location / predicate-evaluation path and indexed cell lookup, per-tet classification is $O(1)$ modulo the cached decision structure. With vectorization, million-tet classification is practical, though the constant depends on implementation choices, not on the math.
- **Streaming refinement audit trail.** Every subdivision is a typed event with tag, source cell, target cells, delivered over DHOOM via `SUBSCRIBE`. Certified termination becomes a bounded-reachable-tagged-state invariant verifiable in real time.
- **Category-theoretic language for the termination question.** The bundle framing lets certified termination be stated as well-foundedness of a tagged functor between refinement categories — structural vocabulary the classical AMR literature lacks.

## 7. Open questions worth flagging

1. Are the CAD cells minimal, or do they refine further under $S_4$? The classifier must quotient before it can be called an equivalence relation on tet shape.
2. Is classifier equivalence sufficient for the invariants that matter (sliver detection, refinement termination), or is a finer relation needed? Similarity is strictly finer; the interesting middle ground is what a careful treatment should characterize.
3. Does subdivision reduction extend to non-Euclidean tets (spherical, hyperbolic)? The angular Gramian's positivity condition changes — this connects to Regge calculus and is a natural next chapter.
4. GIGI can aggregate per-tet dihedral data into edge-star Regge curvature signatures:
   $$ \kappa_e = 2\pi - \sum_{T \supset e} \alpha_{T,e} $$
   Is there a classifier-preserving map from tets into edge-star curvature signatures? If yes, the classification problem becomes a combinatorial classification of PL discrete curvature — a much bigger result than tet quality. (The harness computes $\kappa_e$ on the flat cube mesh and confirms it vanishes to machine precision on all 316 interior edges — the encoding is ready for curved inputs.)

## 8. Companion artifacts

- This spec (`GIGI_TETMESH_SPEC_v0.6.md`)
- The validation harness (`examples/tetmesh_fiber_harness.py`) — mesh generation, full fiber computation, math validation, live load-and-query against a local GIGI instance
- The companion visualization (`examples/tetmesh_visual.html`) — three.js rendering of the Kuhn mesh, the bisection cycle, and the descent-curve telemetry; self-contained, open in any browser
- The `gigi_tetmesh_ref_v0.6` dataset (the structured mesh regenerable from the harness; unstructured meshes ship as data)
- Four experiment scripts (A–D) as GQL + Jupyter
- One-page quickstart on loading a mesh into GIGI (`docs/GETTING_STARTED.md`, Track 3)
- Reading list: Fiedler 2011 (Euclidean simplex geometry), Maubach 1995, Traxler 1997, Bey 2000, Kossaczký, Mitchell (bisection review), Shewchuk (tet quality and conditioning), Regge 1961, Adiprasito–Pak 2016 on triangulations

**Where this thread goes next.** Whatever Experiment C surfaces on real unstructured geometry is a publishable observation, either as a standalone note or as part of a longer GIGI + discrete-geometry paper. If Experiment D produces a clean categorical termination statement, it becomes a lemma in the Yang–Mills GIGI work (*GIGI Solves* Vol 4) — subdivision morphisms on the fiber bundle are exactly the refinement operations Yang–Mills needs on the connection.

## Appendix: validation receipt (v0.6, 2026-07-01)

`examples/tetmesh_fiber_harness.py`, run against a release build of `gigi-stream` on this repository's `main`-era source. Offline math validation, all 384 tets of `unit_cube_384`:

```
PSD rank-3 + positive kernel:     all 384 pass (min eig 0.0, |kernel eig| <= 7.5e-17)
facet-area closure |sum a_i n_i|: max 0.0
det G_edge = (6V)^2:              max err 5.4e-20
distinct classifier cells mod S4: 1  (expected 1)
Kuhn tet dihedrals (rad):         [0.785398 0.785398 1.047198 1.570796 1.570796 1.570796]
Regge deficit, 316 interior edges: max |kappa_e| = 1.8e-15  (expected 0: flat)
```

Naive uniform longest-edge bisection, 3 levels (the Experiment C instrument, here acting as negative control):

```
level 0:   384 tets,  1 class,  min q_vol 0.4648
level 1:   768 tets,  1 class,  min q_vol 0.4703
level 2:  1536 tets,  1 class,  min q_vol 0.5060
level 3:  3072 tets,  1 class,  min q_vol 0.4648   <- period-3 cycle, returns to level-0 shape
```

Live run: 5,760 tet fibers + 5,376 refine events inserted; `COVER` (indexed and scan), `SECTION AT`, `INTEGRATE OVER classifier_cell/level`, and `HEALTH` all return correct values with curvature/confidence ride-alongs. Reachable classifier state measured by the engine: `{C0: 3456 (levels 0+3), C1: 768 (level 1), C2: 1536 (level 2)}` — the three-class Maubach cycle, recovered by a GQL aggregation.

## 9. Version notes

v0.1 — initial spec.
v0.2 — math cleanup: correct Gramian, PSD certificate, classifier-cell language, tagged well-foundedness, geometric sliver set, framework anchors removed.
v0.3 — residual correctness cleanup:
- Classifier equivalence explicitly stated as intentionally coarser than geometric similarity, not "may or may not coincide."
- PSD certificate split into `mode: "exact"` and `mode: "numeric"` with consistent rank/determinant semantics; the two are never mixed.
- Sliver set defined on scale-invariant shape space $\mathcal{X}$ with normalized volume quality $q_{\text{vol}} = 6V/L^3$; $d_{\text{bad}}$ is distance on $\mathcal{X}$, not on $\vec\alpha$-space alone.
- Angle/normal sign convention stated explicitly: internal dihedrals, outward unit face normals, $n_i \cdot n_j = -\cos(\alpha_{ij})$.
- Experiment A expected outcome tightened to exactly one classifier cell modulo $S_4$.
- Maubach/Traxler contribution recast: literature proves finite closure and bounded similarity classes; the open contribution is the categorical reformulation.
- Runtime claim rewritten as an implementation dependency, not a guarantee of the math.
- Nozzle mesh honestly labeled as a synthetic Gmsh benchmark.
v0.4 — precision edits:
- Shape-space metric labeled as an engineering metric on a redundant embedding, not an intrinsic geometric distance.
- Experiment B expected outcome recast from "strictly more discriminating" to a test hypothesis about tail separation.
- Volume quality formula annotated for unsigned $V$ and $\det G_{\text{edge}} = (6V)^2$, removing orientation-sign ambiguity.
- $S_4$-quotient implementation convention stated explicitly (canonicalize before lookup); classifier must match dataset convention or provide an adapter.
- §1 sliver-detection language softened from "not exact classifiers" to "no queryable semi-algebraic classifier of shape," matching what the spec actually delivers.
v0.5 — polish and shipping cleanup:
- Facet-area symbol renamed to $a_i$ to avoid clashing with the angular Gram matrix $A$.
- CAD phrased in angular-Gram coordinates $c_{ij} = -\cos\alpha_{ij}$, with $\vec\alpha$ retained for reporting. Classifier-equivalent definition and coarser-than-similarity note updated to reference $\vec c$-space.
- JSON caption corrected to match the numeric-mode record actually shown.
- Experiment D distinguishes 5-level telemetry from the theorem-level certification. Certification comes from Maubach/Traxler/NVB finite-closure theorem; the experiment corroborates.
- §1 opening softened from "every serious 3D physics solver" to "many serious 3D physics solvers," and mixed unstructured meshes acknowledged.
v0.6 — generalization and engine validation (this version):
- Reframed as a standalone companion note to *The Geometry of Flight*; recipient-specific framing removed throughout.
- GQL examples rewritten in actual GQL and executed against a live build; Cypher-style graph-pattern syntax removed (GQL has none). Transitive refinement queries expressed via materialized `(root, level)` ancestry + `INTEGRATE`, and the audit trail as a typed events bundle over DHOOM/`SUBSCRIBE`.
- Fiber record flattened to GIGI's flat fiber schema; matrices carried as `vector(n)` fields; PSD certificate flattened to `psd_*` fields with `psd_mode` discriminator.
- `unit_cube_512` corrected to `unit_cube_384`: a Kuhn triangulation of an $n^3$ grid has $6n^3$ tets, and $6n^3 = 512$ has no integer solution.
- Experiment A validated end-to-end; Experiment C annotated with the structured-mesh negative-control result (period-3 Kuhn cycle, three classes).
- Validation-receipt appendix added. Validation surfaced three `INTEGRATE` engine defects (`count(*)`/text-field count returned empty; multiple same-function measures aliased the first field; global no-`OVER` form returned empty) — all fixed in the engine alongside this version via `aggregation::group_by_measures`.
- Reframed as a chapter draft for *GIGI Builds* (*GIGI Solves* Vol 1); companion three.js visualization added (`examples/tetmesh_visual.html`).
