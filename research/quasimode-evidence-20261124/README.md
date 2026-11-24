# Focused quasimode evidence repair

Software fixtures and mathematical counterexamples only. No Mq=8 solve or registered grid.
See the dated spec amendment under docs/research. RED logs precede the implementation repair.

## Logs

- `red.log`: original broad RED attempt; stops at the intentionally planted, then-unhandled KeyboardInterrupt.
- `red-rejections.log`: completed RED subset, seven tests with 17 assertion failures against the old checker.
- `red-overflow.log`: later RED control for the release helper's overflowing measurement path.
- `green-initial.log`, `green.log`, `green-final.log`: incremental evidence tests, ending at 27 passing tests.
- `math-controls.log`: eight independent small mathematical controls, not physical certification tests.
- `flux-regression*.log`, `cone-regression.log`, `boundary-regression.log`: focused regression receipts.
- `focused-final.log`: all 96 selected tests together on the final implementation.
- `mutations-final.json`: 15 caught in-memory mutants, including full assertion traces; `mutations-final.log`
  is the compact summary. The earlier `mutations.json` and log are retained as well.
- `source-manifest.json`: byte hashes of the final reviewed source/test/spec files, base revision and dirty
  state before committing. This is traceability, not a signature or a physical result.

## Reproduce from the feature-branch checkout

```powershell
python -m unittest discover -s scripts -p test_helike_cone_convergence.py -v
python -m unittest discover -s scripts -p test_helike_quasimode_math_controls.py -v
python -m unittest discover -s scripts -p test_flux_review.py -v
python -m unittest discover -s scripts -p test_helike_positive_cone.py -v
python -m unittest discover -s scripts -p test_helike_qspace_boundary.py -v
python scripts/check_helike_convergence_mutations.py "$env:TEMP/helike-convergence-mutations.json"
```

To reproduce the combined selection:

```powershell
python -c "import sys, unittest; sys.path.insert(0, 'scripts'); names=['test_helike_cone_convergence','test_helike_quasimode_math_controls','test_flux_review','test_helike_positive_cone','test_helike_qspace_boundary']; suite=unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromName(n) for n in names); result=unittest.TextTestRunner(verbosity=2).run(suite); sys.exit(not result.wasSuccessful())"
```

The receipt tests use temporary planted rows and fake solver objects. They do not masquerade as Mq=8 physical
measurements. Existing saved positive-cone and boundary matrices/measurements are independently replayed by
their own suites. No full engine test suite was run because no engine code changed.

The original Hamiltonian/geometry modules and the original flux/preregistration documents have an empty
`git diff f5c93266 -- <those paths>`; no physical repair or acceptance-threshold change is bundled here.
