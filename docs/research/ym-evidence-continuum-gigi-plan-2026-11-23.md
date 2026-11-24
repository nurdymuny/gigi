# A geometric evidence atlas for Yang-Mills experiments in GIGI

23 November 2026. **Implementation plan; ingestion and comparison gates NOT RUN.**

## The research aim

Run the model. Keep what it actually produces. Put those observations in GIGI.
Then follow the geometry across refinements, parameters and, eventually, different
models. The gap belongs in the record, but it is not the whole record. The
field pattern, the symmetry it carries, the way Helike couples to it and the
parts the calculation still leaves unresolved belong beside it.

The intended question is: **what structure keeps reappearing as the description
changes, and what evidence says it is the same structure?**

There are two distinct targets:

1. An empirical geometric through-line across related experiments: an atlas of
   observations and explicitly measured correspondences.
2. A continuum limit of a specified field theory: a separate claim requiring
   the appropriate cutoff, volume, scale-setting and consistency checks.

GIGI should help discover and test the first, and organize evidence needed for
the second. A smooth database visualization must not substitute for either.
The selected [matrix model](https://arxiv.org/abs/1412.7900v1) is a reduction;
increasing its polynomial basis cannot restore spatial modes absent from its
Hamiltonian. The full target is described separately in
[Jaffe and Witten's Yang-Mills problem statement](https://www.claymath.org/wp-content/uploads/2022/06/yangmills.pdf).

## First material: the run we already have

Use only the verified authoritative run `20261124T021903Z-c326a3c5` initially:

- 15 cases: three couplings, five polynomial cutoffs, one Hamiltonian/domain contract.
- 30 case/channel observations: H2 and H0 at each case.
- All computed complete levels, not only the lowest or those inside a plotted window.
- Hamiltonian, Gram matrix, normalized eigenvectors, angular Casimir, both
  observables, full response kernels and source snapshot.
- Every residual, discarded-response fraction, failed gate and NOT_RUN status.

The [results note](helike-quantum-probe-results-2026-11-23.md) and
[illustrated report](../papers/helike-quantum-probe/README.md) identify the files.
The earlier preliminary run is a separate lineage entry, never an independent
replication. Identical source/data cannot be counted twice as corroboration.

**Immediate scientific payoff:** inspect the competing scalar and tensor
branches across N = 4, 6, 8, 10, 12. Preserve the scalar-lowest result at N = 8
for g squared = 2. Track branch geometry and probe response without relabelling
whichever eigenvalue happens to be smallest as the same state at every cutoff.

## Bundle design

Use separate bundles for model-specific observations, plus shared registries and
explicit comparison bundles. The first prefix is `ym_su2_sphere_v1`. Proposed
names below are a storage contract, not already deployed endpoints.

| Bundle | Key / base | Fiber measurements and required metadata |
| --- | --- | --- |
| `ym_models` | `model_id` | Published source, exact Hamiltonian hash, domain, gauge group, spatial geometry, boundary conditions, units, symmetry conventions, scale prescription |
| `ym_runs` | `run_id` | Model ID, source hashes, evaluator version, environment, completion status, validation status, snapshot digest, artifact manifest |
| `ym_su2_sphere_v1_cases` | `case_id` | Run ID, coupling, cutoff, dimension, E0, E1, absolute gap, all diagnostics, runtime and numerical policy ID |
| `ym_su2_sphere_v1_levels` | `level_id` | Case ID, complete-level index, energy/spread, multiplicity, symmetry label, projector reference, residual, grouping policy |
| `ym_su2_sphere_v1_responses` | `response_id` | Case/level/probe IDs, kernel reference, normalized weight, absolute excited norm, discarded norm, robust-detection status |
| `ym_probes` | `probe_id` | Exact formula, normalization, symmetry transformation law, geometric origin and permitted models/maps |
| `ym_maps` | `map_id` | Source/target contracts, map formula/code hash, unit conversion, coordinate/Jacobian convention, preserved structures, domain and measured mapping errors |
| `ym_comparisons` | `comparison_id` | Map ID, source observation IDs, metric contract, distances, uncertainty, branch match or ambiguity, evidence references |
| `ym_artifacts` | `artifact_sha256` | Durable relative URI, media type, shape, dtype, byte size, source revision and verified availability |

A section is the recorded assignment of measured values to one identity.
The fiber contains these values and their declared coordinate meaning. Merely
placing observations in bundles does not establish smoothness, a shared
Hilbert space, or a physical connection between the bundles.

IDs are deterministic hashes of canonical identity fields, with an explicit
serialization version. Binary array hashes are exact byte hashes; a semantic
array digest, if added, must declare byte order, dtype and memory order. Model
parameters use canonical numeric encoding, not rounded display strings. Existing
observations are immutable by importer policy; corrections create new revisions.
Server-enforced immutability/versioning is not assumed to exist automatically.

Use numeric columns for small measured scalars and explicit key/order columns.
Large NPZ arrays remain in a durable artifact directory/object store with hash
references; GIGI is not presumed to parse NPZ or make a local temporary path
portable. Publish artifacts first, then observations, then a COMPLETE run
manifest after read-back verification. Partially imported runs are never eligible
for scientific comparison. Missing, nonfinite and unresolved values are distinct
states, not zeros. The source columns remain authoritative; derived descriptors
carry their own recipe and normalization IDs.

## What the geometric comparison measures

### 1. Compare subspaces before arbitrary eigenvectors

At fixed model and coupling, successive cutoff spaces are nested. Embed their
normalized states in a common oscillator basis using the saved basis maps and
the full metric. If U and V have orthonormal columns representing complete
levels of ranks r and s, use

\[
 d_P^2=\frac{r+s-2\|U^\dagger V\|_F^2}{r+s},\qquad r,s>0.
\]

This is the normalized squared Frobenius distance of their orthogonal
projectors. It is unchanged by phases or unitary rotations inside either level,
and it records a penalty for unequal ranks. Singular values of U-dagger V
also give principal angles. Retain the full set, not just a mean distance.
Zero-rank or ambiguous levels refuse this calculation.

For the current run, reconstruct normalized oscillator vectors using the saved
coefficient map C_N and generalized eigenvectors. Do not compare raw coefficient
arrays of different lengths as if they occupied the same Euclidean space.
At different couplings, q = M/g changes the coordinate map: comparing states in
physical M coordinates requires the corresponding unitary dilation and Jacobian.
Across different model Hilbert spaces, a common embedding or validated physical
map is required; otherwise projector distance is NOT_COMPARABLE.

### 2. Retain the response as a spectral measure

For each fixed physical probe, keep

\[
 \mu_O=\sum_{n>0} W_n\,\delta_{\widehat\Delta_n},\qquad
 \widehat\Delta_n=(E_n-E_0)\ell_{\rm ref},
\]

where the independently declared reference length makes energy dimensionless
in the adopted units. In the current model, ell_ref = R. Do not divide every
gap by itself or fit a separate energy rescaling to make models agree.
Equal bare couplings in different regularizations are not automatically matched
physical couplings. A later lattice comparison needs its own scale-setting and
renormalization contract; [Wilson-flow scale observables](https://arxiv.org/abs/1006.4518v3)
are one established option to investigate, not already supplied by these data.

Compare weighted measures with an exact one-dimensional Wasserstein metric on
the declared energy axis, preserving atomic weights. Do not replace a weighted
spectrum by an unweighted list of levels. These measures are currently normalized
finite-basis responses; their tails and nonconvergence labels travel with them.

For optional fixed-bin neighborhood search, first declare physical bin edges
and include an overflow bin. With p_i equal to the probability in each bin,
store x_i = sqrt(p_i). Then ||x|| = 1 and the Hellinger distance obeys

\[
 H^2(p,q)=1-\sum_i\sqrt{p_iq_i}.
\]

GIGI's existing half-angle sampling distance on these unit vectors would be
d_squared = H_squared / 2. This is a checkable correspondence, not an arbitrary
embedding. Binning resolution is a separate error source; the full atomic measure
is always retained. A channel with zero excited norm has no normalized measure.

### 3. Preserve kernels and a common set of observables

Retain K_AB(n), not only its trace. Compare correlation matrices at fixed
dimensionless imaginary times under a declared probe-basis transformation.
Changing probe normalization requires changing the associated kernel convention;
arbitrary componentwise flattening can create false differences. Cross-model
comparison may use a common observable algebra even when state-space embeddings
are unavailable. Agreement of that restricted observable set is not complete
physical equivalence. No full quantum eigenfield reconstruction is claimed from
the gap, or from the projector coefficients alone, without the basis functions.

### 4. Treat correspondences as measured edges

Observations are vertices; a validated comparison map is an edge. Edges store
what is preserved and what is lost. A path through bundles is a research
hypothesis about continuation of a structure, not an automatic equivalence.
Where maps compose, test composition and loop consistency with propagated errors.
Do not invoke HOLONOMY without an actual connection/transport law. Splitting,
merging, loss of rank and ambiguous matches remain explicit outcomes.

## Actual GIGI operations and boundaries

Source inspection is pinned to the present worktree's base revision
`ed5d3cf5209bb8cecd6d97c4f0df3065af90363e`. **These are source-backed capabilities,
not a claim that the import has been exercised on a live server.** The first
implementation gate must build an isolated server and prove the actual responses.

| Operation found in source | Intended role | Boundary |
| --- | --- | --- |
| `POST /v1/bundles`, `POST /v1/bundles/{name}/insert`, ordinary reads and `SELECT` | Store cases, levels, provenance and compare filtered histories | Import validation, immutable IDs, manifest publication and portable artifacts still need an adapter |
| `SPECTRAL bundle ON FIBER (...) MODES k` | Exploratory covariance modes of compatible descriptor columns | PCA/covariance spectrum, not the physical Hamiltonian spectrum; the present response gives mode/lambda/IPR, not quantum eigenvectors |
| `SPECTRAL ... ON FIBER (h) MODE MATRIX` | Possible later small symmetric-matrix cross-check, after correct matrix encoding and metric whitening | Not a native generalized H v = E S v solver or a replacement for the GPU reference |
| `POST /v1/bundles/{name}/brain/sample_transport` (`kahler`) | Reproducible sampling of nearby normalized response signatures | Angular, magnitude-blind metric; samples are not an exact nearest-neighbor ranking; validate all fields before calling |
| `TRANSPORT ... ON FIBER (...)` | Only after the requested geometric convention is proven appropriate | Current four-component path has quaternion semantics; other paths depend on schema/features. It is not the Hilbert-space embedding above |
| `WASSERSTEIN` (`post_kahler_phase1`) | Unweighted empirical one-field cohort comparisons where that contract fits | Current handler collects unweighted samples, not the atomic spectral weights; weighted spectral transport needs a separate implementation |
| `CURVATURE` | Descriptive dispersion of stored numeric fields | Not spacetime curvature, Hamiltonian curvature or evidence of a mass gap |

Source pointers: [request/insert/analysis handlers](../../src/bin/gigi_stream.rs),
[parser and executor](../../src/parser.rs), [fiber spectrum](../../src/spectral.rs),
[angular sampler](../../src/geometry/sample_transport.rs).
The parser and bundle binding contain SAMPLE_TRANSPORT, but this inspection did
not find a corresponding GQL execution arm in `gigi_stream.rs`; the REST handler
is present. Use the REST surface for its first gate and test GQL separately;
do not promote parsed syntax into a working-query claim.

Unknown/missing numeric fields can be defaulted in some existing geometric
helpers. The adapter must strictly reject these before geometry, rather than
trusting every generic verb to enforce this research contract. Discover the
isolated server's feature flags and live `/v1/ml` and `/v1/openapi.json` surfaces.
Use `127.0.0.1`, verify the actual rebuilt binary, and inspect returned rows and
values, not HTTP success alone. A refusal is not a failed scientific experiment.

## Strict acceptance gates for the next implementation

Each gate gets a failing fixture before implementation and an explicit
mechanism-removal check afterward. All statuses below are currently NOT RUN.

| Gate | Positive requirement | Required rejection / negative fixture |
| --- | --- | --- |
| YG00 Identity | Verify source/run/artifact identity and numerical contract | Corrupt one array byte; stale verification; duplicate run presented as replication |
| YG01 Import fidelity | Isolated live engine round-trips all 15 cases, 30 channel summaries and complete kernels/level counts | Drop one case, swap one channel, replace a tiny measured weight by zero |
| YG02 Publication | Idempotent retry; restart; read-back digest; COMPLETE only after every required row/artifact | Interrupt each publication step; stale COMPLETE cannot survive failure |
| YG03 Comparability | Units, probe convention, domain and matching map validated before metrics | Equal bare gaps with different units; four arbitrary fields misread as a quaternion |
| YG04 Projectors | Planted phase/sign and within-level unitary changes give distance zero; orthogonal equal-rank subspaces give distance one | Arbitrary eigenvector index match; omitted metric; incompatible embedding |
| YG05 Weighted response | Exact CDF/transport oracle; normalized weights and total norm retained | Treat one heavy atom as equal to one light atom; normalize a zero-response channel |
| YG06 Neighborhoods | For planted bin probabilities, sampler distance equals Hellinger squared / 2 | Missing coordinate, altered bin edges, omitted overflow, hidden scale change |
| YG07 Branch tracking | Recover distinct scalar/tensor histories and the actual N = 8 ordering change | Follow energy rank alone and fabricate one continuous identity |
| YG08 Refinement | Current data remains UNRESOLVED; a separate planted convergent sequence passes the frozen policy | Stable gaps with drifting energies, large exterior residual, omitted tail |
| YG09 Cross-model maps | Known equivalent coordinate/unit reformulation agrees under an explicit map | Fit the scale to the target gap; silently map incompatible groups/domains |
| YG10 Held-out evidence | Predeclare models, mapping rules and test cases; retain mismatches and negative controls | Select only matching couplings or cherry-pick successful fields |
| YG11 Live report | GIGI produces exact source-traceable tables; scalar/vector/JSON/DHOOM paths agree where supported | Bare `status: ok`, stale binary, changed ordering, stale scientific PASS |

An unavailable map returns NOT_COMPARABLE. A nonconverged case remains useful
as finite-cutoff evidence, labelled UNRESOLVED. A mathematical refusal, storage
failure and scientific counterexample are not the same status.

## Bounded work order

1. Build the evidence importer and validation manifest only. Use a new scratch
   data directory and isolated server, no production write. Close YG00--YG02
   with the present 15 cases before any new expensive simulation.
2. Implement projector/response comparison adapters with independent numerical
   oracles; close YG03--YG07. Generate the report's existing figures from GIGI
   read-back and require equality with the file-based report.
3. Preserve the current UNRESOLVED status through YG08. Separately preregister
   how to improve within-model convergence. No new basis or threshold is
   authorized by this database plan.
4. Add one second published formulation with a defensible comparison map and
   independent validation, then close YG09--YG10 on held-out cases. Prefer a
   known-equivalent reformulation as calibration before claiming universality.
5. Close YG11 with an actual live-engine showcase: compare compatible runs,
   recover an ordering change, reject an intentionally incompatible model and
   trace every plotted point to its source. Only then publish the integration.

The through-line is something to **measure**, not something the page should draw
into existence. The first useful GIGI result will be seeing exactly which
geometric information persists, which changes, and which comparison is not yet
justified, while retaining the full evidence for the next question.
