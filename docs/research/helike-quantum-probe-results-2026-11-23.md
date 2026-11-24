# Helike in a published quantum Yang-Mills model: first GPU survey

23 November 2026, Pacific time. **COMPUTED: finite-cutoff observations.**

**Illustrated report:** [PDF, reusable figures and editable LaTeX](../papers/helike-quantum-probe/README.md).
**Next integration:** [GIGI cross-model geometric evidence plan](ym-evidence-continuum-gigi-plan-2026-11-23.md).

## What I tested

I took an existing quantum Yang-Mills matrix model, left its energy rules alone,
and used the registered translation of Helike's geometry to ask which quantum
excitations it reaches. The calculation ran on the RTX 5070 Laptop GPU. It did
not replace a quantum energy with a classical Hessian eigenvalue.

The bounded survey is finished. Its scientific verdict is **UNRESOLVED**:
the finite matrices and their numerical solutions passed the checks below,
but the cutoff-convergence requirements did not. There is a measured response
worth examining, not yet an accepted answer for the untruncated model.

The frozen contract is the [preregistration](helike-quantum-probe-preregistration-2026-11-23.md),
with its [GPU amendment](helike-quantum-gpu-amendment-2026-11-23.md) and
[implementation record](helike-quantum-implementation-2026-11-23.md).
Their historical NOT RUN labels record their status when written; this separate
note reports the subsequent experiment.

The external Hamiltonian is the pure SU(2) specialization of Balachandran,
Vaidya and de Queiroz, [*A Matrix Model for QCD*, arXiv:1412.7900v1](https://arxiv.org/abs/1412.7900v1),
with the radius convention checked against
[Acharyya and Balachandran, arXiv:1702.06430v2](https://arxiv.org/abs/1702.06430v2).
This is a homogeneous quantum reduction on the round three-sphere, in the
registered regular Cartesian colour-singlet realization. It is not the full
four-dimensional field theory. No claim about its missing spatial modes is made.

## The two probes are different

- **H2:** the five-component spatial spin-2 observable whose local gradients
  retain Helike's two real quadratures and their rotational orbit.
- **H0:** the separately registered scalar, quartic composite of that orbit.
  This is a Helike-derived observable, not the original directional eigenfield.

The Hamiltonian is diagonalized in the whole physical finite basis, before any
probe is selected. The probe does not decide which state is called lowest.
Vacuum response is subtracted, and complete degenerate levels are measured.
Weights are fractions of the channel's excited response **inside that finite
basis**. They are not probabilities that a theory is correct.

## What the largest calculation says

All rows below use cutoff N = 12, dimension 1,134, R = 1 and hbar = 1.
Energies and gaps have units 1/R. The accessible gap uses the preregistered
weight threshold 0.0001; it is not a claim of exact zero below that threshold.

| g squared | Ground energy | Absolute gap | Lowest level | H2 accessible gap | H2 weight on lowest | H0 accessible gap | H0 weight on lowest |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| 1, primary | 5.433212201446 | 2.289814624087 | spin 0, one state | 2.642429574254 | 2.7715e-29 | 2.289814624087 | 0.3657882568 |
| 0.5 | 4.891790029661 | 1.566333798579 | spin 0, one state | 2.339593686775 | 1.9337e-29 | 1.566333798579 | 0.3032147032 |
| 2 | 6.259094500838 | 3.094157392270 | spin 2, five states | 3.094157392270 | 0.9915399533 | 3.153908889022 | 2.6369e-26 |

At g squared = 2, H2 places **99.15399533%** of its finite-basis excited response
in the independently computed lowest level. At the other two couplings, H0
reaches the lowest level while H2 reaches a higher spin-2 level. The small
opposite-channel weights are reported rather than silently rounded to zero;
their size is consistent with the registered spin selection rules.

This is evidence that the mapped observables can interrogate different parts
of the quantum spectrum. It does not establish a unique advantage over other
quadrupole or scalar observables; the matched-control usefulness test has not run.

## The cutoff history must stay attached

| N | Physical basis dimension | Absolute gap, g squared = 1 | Absolute gap, g squared = 0.5 | Absolute gap, g squared = 2 | Lowest spin at g squared = 2 |
| --- | ---: | ---: | ---: | ---: | --- |
| 4 | 29 | 2.867846745506 | 2.037058771365 | 4.351929591164 | 2 |
| 6 | 91 | 2.465167224631 | 1.774780654081 | 3.603696384214 | 2 |
| 8 | 238 | 2.379861209466 | 1.667228247135 | 3.264328103066 | 0 |
| 10 | 546 | 2.319649057791 | 1.593988389619 | 3.179460361039 | 2 |
| 12 | 1,134 | 2.289814624087 | 1.566333798579 | 3.094157392270 | 2 |

At the strongest coupling, the scalar is lowest at N = 8, while the tensor is
lowest at N = 10 and 12. Its directional lowest-level weight changes from
1.5961e-27 to 0.9868117108 to 0.9915399533 across those three cutoffs.
That is unresolved truncation-dependent ordering, **not** an established
physical phase transition. A difference of variational energy estimates is
not automatically a rigorous bound on the true gap.

## Why the scientific gate remains open

Two successive cutoff changes must meet the frozen energy/gap tolerance 0.001
times max(1, magnitude), with weight changes at most 0.01. They do not.
The calculation also measures what happens when the full polynomial Hamiltonian
acts on each reported finite-basis state, including its output beyond the cutoff.
This is a different test from solving the finite matrix accurately.

| g squared | Largest full residual among reported levels | Allowed residual | Discarded H2 response fraction | Discarded H0 response fraction |
| --- | ---: | ---: | ---: | ---: |
| 1 | 0.899488334825 | 0.022898146241 | 0.000808032091 | 0.005604987952 |
| 0.5 | 0.513805038072 | 0.015663337986 | 0.000657017616 | 0.003439156549 |
| 2 | 2.075830350460 | 0.030941573923 | 0.002881309937 | 0.023846338098 |

The response fractions are squared norms after vacuum subtraction. The maximum
permitted fraction is 0.001. H2 passes this tail test at the first two couplings;
H0 does not. Neither passes at the strongest coupling. Every coupling fails the
full-residual requirement. The full per-state and complete-level residuals are
in the receipt, not just the worst values shown here.

Therefore no converged model gap, efficiency advantage, or mass-gap result is
claimed. The matched random-seed and trace-control utility comparisons are
**NOT RUN**, because their convergence prerequisite failed. No cutoff beyond 12
was attempted and no threshold, coupling, probe or Hamiltonian was changed.

## What did pass

- Twenty focused tests, including a planted oscillator spectrum, colour-singlet
  basis counts, a nonidentity generalized metric, complex CUDA arithmetic,
  vacuum removal, whole-level projectors and explicit exterior action.
- Fifteen exact symbolic coordinate/probe identities. The earlier four injected
  algebra defects remain recorded beside the algebra receipt.
- Eight in-memory mechanism-removal tests all turned their selected tests red:
  missing cubic term, ignored metric, retained vacuum, incomplete multiplet,
  discarded exterior, accepted energy drift, stale receipt and missing gates.
- An independent N = 4 construction on all oscillator states, imposing the Gauss
  constraint without the primary invariant-polynomial basis. Its comparison
  error was 7.90697e-15; its Gauss-kernel residual was 3.44790e-14.
- Across all 15 interacting cases, the maximum scaled CPU/GPU spectral difference
  was 1.16089e-15, complete low-level projector difference 1.64558e-12, GPU
  finite-matrix residual 3.34568e-16, and metric orthogonality error 1.17202e-13.
  These passed the fixed 1e-10 checks.
- CUDA matrix-exponential correlations agreed with the spectral kernels at all
  six registered times, with maximum normalized discrepancy 2.08410e-14.
- The evidence tests reject altered gaps, altered weights, nonfinite claims,
  stale run identities and missing acceptance gates. A nonfinite-claim test was
  first observed failing, then repaired; its red output is retained.

These are implementation and finite-calculation checks, not a declaration that
all HP00--HP10 research gates have closed. In particular HP08 convergence and
HP09 usefulness remain open. Numerical precision is not a substitute for them.

One numerical issue was caught during construction: the default CPU eigensolver
returned inaccurate vectors in the highly degenerate Gauss nullspace. The
independent builder now uses the divide-and-conquer solver and explicitly checks
the nullspace residual and orthogonality. No tolerance was loosened.

## Evidence and reproduction

Authoritative run: `20261124T021903Z-c326a3c5`. The UTC date is 24 November;
the local research date is 23 November.

- [Terminal receipt, complete spectra metadata and kernels](../../research/helike-quantum-probe-20261123/survey-20261124T021903Z-c326a3c5/receipt.json)
- [Saved matrices, vectors, immutable source snapshot and test transcript](../../research/helike-quantum-probe-20261123/survey-20261124T021903Z-c326a3c5/)
- [Independent evidence replay result](../../research/helike-quantum-probe-20261123/survey-20261124T021903Z-c326a3c5/verification.json)
- [Initial implementation red test output](../../research/helike-quantum-probe-20261123/implementation-red.txt)
- [Nonfinite-evidence red test output](../../research/helike-quantum-probe-20261123/evidence-nonfinite-red.txt)

The base Git revision is `ed5d3cf5209bb8cecd6d97c4f0df3065af90363e` on
`feat/helike-quantum-probe-spec-20261123`. The implementation is uncommitted:
the receipt identifies the actual executed files by SHA-256, and saves their
contents under `source/`. The base revision alone does not identify this code.
The earlier `20261124T021234Z-fa426cb9` survey is retained as preliminary evidence,
superseded by this source-snapshotted replay. Between them, exterior diagnostic
selection was generalized to cover any first-accessible probe level, and evidence
validation tests were added. The levels selected in these measured cases did
not change. The registered physical choices stayed fixed.

From this worktree, using its existing Python environment:

```powershell
python scripts/run_ym_helike_quantum.py
python scripts/verify_ym_helike_evidence.py research/helike-quantum-probe-20261123/survey-20261124T021903Z-c326a3c5 --run-id 20261124T021903Z-c326a3c5
```

The first command starts a **new** bounded survey and creates a new unique
directory. UNRESOLVED intentionally returns a nonzero process status. The second
command rebuilds the saved matrices, re-solves them on CPU, checks hashes,
recomputes probe overlaps and final exterior diagnostics, and preserves the
UNRESOLVED verdict. If working sources change, run the verifier from the saved
`source/scripts/` path instead; the full source snapshot is self-contained for
this replay. Hashes detect accidental changes, not a malicious rewrite of every
source and receipt together.

The completed replay returned `integrity: VERIFIED`, `replayed_cases: 15`,
`survey_status: UNRESOLVED`, and `scientific_acceptance: false`.

Runtime was 72.817 seconds for the worker survey, including its construction
checks, not including the separate evidence replay. Peak sampled host memory
was 2.4165 GiB. PyTorch peak device allocation was 157.992 MiB and reservation
226 MiB; these exclude driver/context allocations and other applications.
The runtime was PyTorch 2.11.0.dev20260108+cu128, CUDA 12.8, NumPy 2.4.0,
on the NVIDIA GeForce RTX 5070 Laptop GPU. No CPU fallback was allowed for GPU
requests. Construction and the independent CPU comparison still use the CPU.
These timings are resource receipts, not a claimed speedup.

No GIGI engine source, live endpoint or deployment changed. The full Cargo
regression suite was not run for these standalone research scripts.

## The next question, without another run yet

Can the scalar and spin-2 levels be resolved in the **same unchanged model**
with a converged physical basis and small exterior residuals? Only then can I
decide whether the directional probe truly reaches its lowest excitation and
whether it reduces the work needed to find it. Any larger cutoff or changed
basis scale needs a separate dated amendment; this survey does not authorize it.
