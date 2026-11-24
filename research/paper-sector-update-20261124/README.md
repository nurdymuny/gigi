# Working-paper sector update: publication evidence

24 November 2026. No physical experiment was rerun. No engine code or deployment
changed. The update publishes the existing controls-abd81f97 evidence in the
[working paper](../../docs/papers/helike-quantum-probe/README.md).

## Behavioral gates

- `red.txt`: the new publication adapter initially returned empty case/final
  lists and pass dictionaries without validation. The 13-test suite reported
  seven assertion failures, including subtests, and zero unexpected errors.
  These exercised missing results, fabricated acceptance, nonfinite values,
  changed gaps, lost scope and missing case detection.
- `green.txt`: all 15 report tests pass after implementation. Two additional
  symbolic tests differentiate the invariants in nine Cartesian coordinates,
  verify the displayed co-metric and Laplacians, check the tensor contraction,
  and verify the potential square, Laplacian of W, connecting path and action.
- The publication verifier rejects six deliberate removals by assertion:
  stale survey identity, lost response normalization, reversed quadrature,
  removed sector gate validation, promoted certification and unchecked sector
  arithmetic. Their full outputs are in the linked `report-verification.json`.
- The numerical experiment's earlier 69 tests and 19 mutants are distinct;
  those receipts are retained in the source run, not claimed as newly run here.

Commands:

```powershell
python -m unittest discover -s scripts -p test_ym_helike_report.py
python scripts/build_ym_helike_report.py --pdf
python scripts/verify_ym_helike_report.py
```

## Publication validation

The builder validates both experiments separately, verifies all source and
array hashes, compares sector receipt values with saved arrays, reconstructs
gate arithmetic, and generates tables/plots from those checked values.
It does not rerun the Hamiltonian solve. The verifier binds both receipts,
both verification files, builder, manuscript and outputs to the artifact
manifest. A failed verification overwrites the result with FAILED, not PASS.

The final PDF has 15 pages and eight figures. All pages were rendered and
visually inspected; equations, tables, captions and figure labels fit. LaTeX
reports no overfull/underfull boxes or warnings. Author metadata is empty.
The original five figure assets remain byte-identical; three new assets are
provided as PDF, SVG and PNG. The publication verifier checks page containment
and required result/derivation text in addition to file hashes.

## Contribution review

Primary sources checked online on 24 November 2026:

- H.-P. Pavel, [SU(2) Yang-Mills quantum mechanics of spatially constant fields](https://arxiv.org/html/hep-th/0701283v1):
  invariant variables, oscillator trial functions and spin-resolved variational
  methods are prior art. The homogeneous quartic problem is not this exact
  sphere potential with its quadratic and signed cubic terms.
- Acharyya et al., [Glueball Spectra from a Matrix Model of Pure Yang-Mills Theory](https://arxiv.org/html/1606.08711v3):
  a variational spectrum calculation for the related SU(3) sphere model.
- Acharyya and Balachandran, [Matrix Model of QCD: Edge Localized Glue Balls and Phase Transition](https://arxiv.org/html/1702.06430v2):
  model Hamiltonian and the significance of the boundary-domain choice.
- Acharyya, Pandey and Vaidya, [Axial Anomaly in SU(N) Yang-Mills Matrix Models](https://doi.org/10.1103/PhysRevLett.127.092002):
  matrix-model instantons have a published precedent.
- Zhu, Argentati and Knyazev, [Bounds for the Rayleigh quotient and the spectrum of self-adjoint operators](https://arxiv.org/abs/1207.3240):
  residual-based spectral estimates are established mathematics.

These sources justify attribution of the ingredients, not an exhaustive novelty
verdict. The paper identifies the explicit Helike-derived probe, sector calculus
and verified refinement workflow as the contribution documented by this study.
It does not assert that nobody else has used geometry or symmetry reduction.

## Numerical scope preserved

The N=12 response weights and all-spin survey remain historical measurements.
The N=24 run reports improved target-state residuals, not new probe weights,
a measured coupling crossing, full-spectrum convergence, certified separation,
or a continuum mass-gap result. Exact basis counts are not timing benchmarks.
The paper distinguishes scalar functions, tensor multiplets and all-spin states.
No claim of 100-digit eigensolve accuracy is made.
