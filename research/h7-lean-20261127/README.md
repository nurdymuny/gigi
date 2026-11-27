# H7 Lean verification receipts

27 November 2026. Baseline: `1d3b91e0`, after the two incoming
publication-package commits. No paper source or PDF is changed by this work.
The later packaging commit `7dd0ef8d` was also fast-forwarded before
publication; it changes no Lean or mathematical source.

## Result and boundary

The real-algebraic implication is kernel-checked for all configurations
satisfying the explicit assumptions. The external global Lie-algebra
bound is a theorem argument, not a custom axiom. The concrete bracket/trace
instantiation and compact-resolvent proof remain outside this Lean project.
No end-to-end formal verification of Statement 47 or H7 is asserted.

Lean: `leanprover/lean4:v4.36.0`.
Observed compiler: Lean 4.36.0, `x86_64-w64-windows-gnu`, release commit
`611c117bfaf7b7ae1cfcf246ce6fba31e8f509e9`.
Mathlib: `fc4f59e577feb1a4956671bc04ac18aaaf8cd842`.
The project manifest pins all transitive dependency revisions.

## Retained runs

- `red/`: the initial library stub built, but all eight required named
  declarations were absent. The expected-red gate observed these errors;
  its receipt deliberately retains `passed: false`.
- `release-01/`: the completed library builds, all eight target signatures
  and axiom reports are checked, and four exact examples compile.
  All four broken-source compilations fail for mathematical proof/type
  errors rather than missing imports. The overall audit passes.
- `python-crosscheck/`: the existing independent SymPy algebra and
  document-scope checks, rerun alongside the Lean source: 12 passed.
- `python-crosscheck-after-pull/`: the same 12 checks passed again after
  incorporating `7dd0ef8d`; this receipt records the updated PDF hash.

All eight axiom reports list exactly `propext`, `Classical.choice`, and
`Quot.sound`. The auditor also rejects reports poisoned with `sorryAx` or
`UnverifiedGlobalBound`. These are standard-foundation proofs, not
"axiom-free" proofs.

| Broken variant | Observed rejection |
| --- | --- |
| 3/4 changed to 2/3 | The proved inequality has the wrong type for the altered conclusion |
| Global bound replaced by True | The proof can no longer apply the bound to the rescaled configuration |
| Cubic homogeneity replaced by True | The rescaling rewrite has lost its hypothesis |
| 27/32 changed to 27/64 | The polynomial factorization has an unsolved false identity |

The exact examples separately establish that 27/64 fails at z = 3/2,
27/32 gives equality there, kappa is zero at t = 3, s = 5/4, and pointwise
domination without the scaling assumptions is insufficient.

## Reproduction and receipts

From the repository root, after the dependency setup in the
[project README](../../formal/h7-cubic-quartic/README.md):

```text
python scripts/check_h7_lean.py <fresh-output-directory>
python scripts/check_h7_cubic_quartic.py <another-fresh-output-directory>
```

The wrapper never reuses an output directory. Each receipt records its
baseline Git revision plus LF-normalized source hashes, commands, exit
codes, diagnostics and dependency revisions. The files were verified
before their containing commit was created; use those hashes to identify
the checked source, not the baseline revision alone. The deliberately
broken `.lean` files are evidence, not project build targets.
Compiler logs are retained verbatim, including whitespace emitted by Lean.

The first Mathlib setup cloned the dependencies and built its cache tool.
Its full-cache update hook was stopped to use the six targeted imports;
the targeted cache command then completed successfully, reusing 2,069
cached files. This setup interruption is not a proof failure. An initial
development build also required an explicit potential-bound rearrangement
and a `noncomputable` marker for the real-valued kappa definition; no
mathematical hypotheses or conclusions were relaxed.

No simulations, eigensolves, GPU jobs, engine changes or full engine test
suite were involved.
