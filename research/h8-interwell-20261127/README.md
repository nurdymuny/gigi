# Inter-well path: proof and verification receipts

27 November 2026. Baseline `f67812e5`.

## Result

The written proof establishes a negative-gradient trajectory connecting
diag(u,1,1) to 0 for every u > 0, and the exact ambient Agmon distance
d = u/2 = 1/(2 sqrt(t)) for the unchanged unsourced reduced potential.
The straight-line excess squared is (u^2-1)^2/18. In original q coordinates
the action is d/g^2. This is not a quantum splitting or a closure of H8.

The full orbit argument uses the standard unstable-manifold theorem,
explicit trapping and decay estimates, and endpoint connectors. Lean
checks the local algebra and calculus, not the complete global existence
or path-infimum proof. See the [proof](../../docs/research/h8-interwell-proof-2026-11-27.md)
and [formal coverage](../../formal/h7-cubic-quartic/H8-README.md).

## Gates

| Gate | Retained result |
| --- | --- |
| Initial red | 12 exact geometry tests pass; absent written proof fails; all 14 missing Lean declarations rejected |
| Integration red | The added H8 status/link test fails before the research register is updated |
| Exact release checks | 14 tests pass, including full nine-variable differentiation and mechanism-removal controls |
| Lean release | 14 named local lemmas and four exact examples compile |
| Axiom audit | All targets use only propext, Classical.choice, Quot.sound |
| Broken sources | Wrong metric coefficient, reversed flow, halved action and missing domain each rejected |
| Poisoned audit | sorryAx and UnverifiedOrbit both rejected |
| H7 regression | Eight Lean declarations retain their permitted dependencies; 12 Python checks pass |

`red/` retains the original stub and the absent-proof diagnostics.
`integration-red.log` records `python -m unittest discover -s scripts -p
check_h8_interwell.py -v` before the H8 register edit. `release-01/` is a
failed development build: the chain-rule lemma needed explicit function
forms of addition and multiplication; no mathematical statement changed.
`release-02/` is the successful full gate. `h7-algebra-regression/` is the
additional H7 exact-algebra replay. Failed receipts are not overwritten.

## Adversarial proof review

- The plane is invariant under the full nine-dimensional gradient, not an
  arbitrary constraint imposed on a trajectory.
- Its metric is dx^2 + 2 dy^2; forgetting the second occurrence of y fails.
- Hyperbolicity is used in the plane, not at the ambient gauge-degenerate
  saddle. The inward tangent has both components strictly negative.
- Boundary signs are supplemented by uniqueness and integrating factors;
  boundedness is supplemented by an explicit decay estimate.
- The action lower bound applies to every ambient path and every endpoint
  in the SO(3) orbit, not just paths in the chosen plane.
- Infinite-time endpoints are handled by finite connectors of vanishing
  action; the gradient trajectory is not mistaken for finite-time arrival.
- The kinetic factor 1/2 and the conversion p = gq are retained. No
  uniform lower bound as t tends to infinity is asserted.
- H8, the quantum prefactor and splitting, dressing and continuum claims
  are not promoted. Paper source and PDF remain untouched.

## Reproduce

Use the pinned project at `formal/h7-cubic-quartic` (Lean 4.36.0, Mathlib
`fc4f59e577feb1a4956671bc04ac18aaaf8cd842`). After its cache setup:

```text
python scripts/check_h8_interwell.py <fresh-output-directory>
python scripts/check_h7_cubic_quartic.py <another-fresh-output-directory>
```

Receipts contain the commands, exit codes, theorem dependencies, baseline
revision and LF-normalized source hashes. They were produced before this
change's commit; identify the checked files by their hashes, not solely by
the baseline revision. Compiler diagnostics are retained verbatim.
Deliberately broken `.lean` files are evidence, not library targets.

No numerical trajectory, eigensolve, GPU job or full engine suite was run.
