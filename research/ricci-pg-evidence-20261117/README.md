# Ricci research: Petrov-Galerkin evidence, 2026-11-17

Branch `feat/ricci-pg-curvature`, based on commit 99882091
(`feat/rf2h-adjoint-consistent-recovery`, itself on the Codex checkpoint).
**Progress preservation, not a release or convergence claim. BW05 is OPEN.**

## Start here, in order

1. [Independent assessment](../../docs/RICCI_INDEPENDENT_ASSESSMENT_2026_11_17.md):
   the fixed-exterior refusal is structural. Whitney curvature is tangentially
   discontinuous at 1124/1124 edges; its potential agrees at 0/1124.
2. [PG curvature spec](../../docs/RICCI_PG_CURVATURE_SPEC.md) and
   [results](../../docs/RICCI_PG_CURVATURE_RESULTS.md): the RF2D floor is not in
   the data (Petrov-Galerkin reaches 1.17% at n=5), but that force is unstable,
   and the variational energy of the same reconstruction is inconsistent.
3. [Zero-mean correction](../../docs/RICCI_ZERO_MEAN_RESULTS.md): my option 1,
   chosen on a sufficient condition that proved wrong; fails at 40.5%.
4. [Magnet geometry and projection layer](../../docs/RICCI_PROJECTION_LAYER_RESULTS.md):
   the literature behind my idea, a symmetric non-negative projection layer
   that cuts the floor to about 5.8% but still floors, and checks of RF2H at n=1.

## Evidence

`MANIFEST.json` lists every archived file with its SHA-256; all 37 entries were
verified byte-for-byte against their sources when packed. Unpack each zip at
the repository root to restore `output/ricci-pg-curvature-20261117/` and
`output/ricci-exterior-conformity-20261117/`. Receipts refuse to overwrite.

Earlier receipts in those directories pin the source hashes of the code that
produced them. Later edits (adding the variational branch) left the
Petrov-Galerkin results bit-identical; this was checked by rerunning.

## Not claimed

No BW05 closure, Berger result, SU(2) flow, native change, or engine run. Weight
handling is untested here because every cell weight in this data is 1.0.
