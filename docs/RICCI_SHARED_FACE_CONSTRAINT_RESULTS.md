# Shared-face constraint: checker verified, original patch incompatible

2026-11-17. The requested bounded spec/TDD gate is complete for the CHECKER.
No constraint-enforcement solve or operator repair was attempted. The mapping,
mass, input links, clock, existing tolerances and BW05 thresholds are unchanged.

## What neighbors must agree on

The along-face part (tangential trace) of their metric-dual curvature must
agree after both sides are expressed in the SAME physical face coordinates.
Outward-normal components need not agree for this cancellation. Signs and
coordinate transformations matter; equal magnitudes are insufficient.

The checker also integrates the mismatch against all THREE original Whitney
face-edge test functions. These span the original discrete trace test space;
they are not three spatial tangent directions. Their moments replay any
original discrete face test, including the two saved observation adjoints.
An integrated-moment match is not promoted to a full trace match.

The strong diagnostic checks36 q6 nodes per face. It does NOT certify the
continuum between those nodes. The global flow/convergence gates remain open.
See `RICCI_SHARED_FACE_CONSTRAINT_SPEC.md` for formulas, pronunciations,
unchanged acceptance conditions, primary references and the prior Davis
control-topology analogy.

## Same six faces, actual retained data

Original n=2 Abelian control; cells0,2,3,456,458,459. The twelve external
faces of the six-cell patch are not covered by this compatibility claim.
No new flow/adjoint solves, eigensolves, refinement sweep, native build,
production edits, full suite, commit or deployment.

| Face | Original sampled physical trace gap, maximum | Largest absolute original face moment | Original verdict |
|---|---:|---:|---|
| 0 | 0.089003908633 | 0.002368510294 | MISMATCH |
| 3 | 0.079793646093 | 0.001727268252 | MISMATCH |
| 8 | 0.042427879371 | 0.001026006445 | MISMATCH |
| 9 | 0.091663398529 | 0.002508797431 | MISMATCH |
| 1009 | 0.083191605073 | 0.001930032081 | MISMATCH |
| 1012 | 0.034896538413 | 0.001268225401 | MISMATCH |

These are curvature-trace and moment quantities, NOT speed percentages.
Every smooth-reference face matches both checks. Its largest sampled physical
trace gap is5.194e-15 and largest absolute moment2.239e-16.
Absolute diagnostic tolerance:1e-9, fixed before the run; no retuning.

The three moments reproduce both previously measured contributions on every
face: worst absolute discrepancy6.242e-15. Reversing the common face chart
preserves signed global-edge moments: worst discrepancy5.725e-17.
Both owners' original Whitney test traces are explicitly checked before use.

## Strict focused TDD

- Kernel RED receipt:11 unimplemented test failures/errors before coding;
  then11 tests green. Includes physical metric, normal-only mismatch,
  nonfinite/invalid input, sign, coordinate and orientation controls.
- Patch/evidence RED receipt: setup blocked by the unimplemented replay;
  then9 real-patch/evidence tests green. No claim that nine behavioral tests
  had individually run in that initial setup failure.
- Combined focused baseline:20 tests passed.
- Eight deliberate mechanism removals each produced the expected assertion
  failure, not an import/setup error: normal leakage, sign folding, removal
  of physical metric, weak-to-strong promotion, ignored orientation, dropped
  third moment, skipped independent replay, and stale-marker removal disabled.
- Six evidence attacks were refused and removed a pre-existing VERIFIED
  marker: missing face, forged averaged/compatible trace, altered moment,
  NaN, forged source hash, forged orientation.

The planted quadratic-jump fixture has zero affine moments but a nonzero
tangential trace. It correctly reports weak compatibility without claiming
strong compatibility. This prevents an average-only constraint from hiding
the very disagreement being investigated.

## What this does not authorize

H (aitch), the metric-dual curvature, is computed from the frozen original
edge field and metric. The checker takes no substitute shared face field.
Simply averaging the two sides would disconnect that face value from its
declared cell field. Forged evidence claiming such a repair fails replay.

The existing lowest-order fields contain no independent face-value freedom.
Enforcing a different compatible representation therefore needs a separately
specified construction and its full energy derivative. This checker neither
chooses such a construction nor establishes its feasibility under the frozen
contract. It does not justify deleting interface terms or changing BW05.

## Retained evidence

Directory: `output/ricci-shared-face-20261117/`

`red.json`, `green.json`, `contract-red.json`, `contract-green.json`,
`mechanism-removals.json`, `diagnostic.json`, `VERIFIED.json`.
Sources and retained input hashes are recorded. Verification independently
recomputes both fields on all six faces and clears stale verification BEFORE
reading submitted evidence. Strict JSON rejects nonfinite and duplicate keys.

```text
python scripts/ricci_shared_face_patch.py run --output <fresh-directory>
python scripts/ricci_shared_face_patch.py verify --output <same-directory>
python -m unittest discover -s scripts -p test_ricci_shared_face_math.py -v
python -m unittest discover -s scripts -p test_ricci_shared_face_patch.py -v
```

Recorded verdict: **constraint_checker_verified=true;
original_patch_compatible=false; release_gate=UNCHANGED_OPEN; new_solves=0.**
Stop here. No repair or broad test run has been started.
