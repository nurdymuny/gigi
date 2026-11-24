# Flux-background review: results and remaining domain requirement

24 November 2026. Review of the thirteen commits from `24356819` through
`0b330274` on `feat/helike-quantum-convergence-20261123`.

## Findings, in order of consequence

### 1. The local Fock continuation is not a physical-domain spectral solver

The reduced coordinates are the entries of a positive-definite Gram matrix,
G=qq^T, on the chosen det(q)>0 sheet. The exact kinetic cometric A(G) is
linear in G. Write G=G0+Mz with MM^T=A(G0). The oscillator basis used in the
new solver lives on all of R^6, not just the region G>0.

At z=-2 M^-1 G0, the continued matrix is

    B(z) = M^-1 A(-G0) M^-T = -I.

This is an open negative-kinetic region, not a roundoff effect. A smooth
packet supported there can oscillate increasingly fast: the kinetic
expectation becomes negative proportional to the squared frequency, while
the expectation of any fixed Taylor polynomial potential remains bounded
on that compact support. Thus the unrestricted continued operator is not
bounded below. Finite oscillator matrices can still produce useful local
approximations, but stability of a few cutoffs cannot certify convergence
of their lowest eigenvalues to the physical problem.

The independent witness uses G0=diag(2,3,4). All six eigenvalues of B are -1
to floating-point precision; max|B+I| is **6.661338147750939e-16**. The full
matrix and coordinates are in
[`domain-witness.json`](../../research/flux-review-20261124/domain-witness.json).
The same statement follows algebraically for any positive G0 and valid M.

**Correction:** the full registered run now stops before constructing a
solver, with `HOLD_DOMAIN` / `NOT_EVALUATED`, exit code 2. The registered
couplings, physical formulas and numerical thresholds are unchanged.
Diagnostic smoke runs remain available, explicitly uncertified.

This finding does **not** refute the exact positive-cone reduction or the
local perturbative coefficients. It identifies a missing domain condition
in the attempted nonperturbative continuation.

### 2. The convergence rule missed one of its two approximation directions

The ladder is (P,Mq)=(6,6),(6,8),(8,8). The original code checked only the
last pair, so it ignored basis drift at fixed Taylor order. A planted
fixture with gaps 1.000, 1.500, 1.501 passed that old check. The repaired
assessment independently checks basis and Taylor changes against the
unchanged 0.003 threshold. A one-rung smoke result is not converged.

The interpretation change was specified before any registered-grid run in
the [execution amendment](helike-flux-review-amendment-2026-11-24.md).

### 3. Evidence could be lost or misclassified

Completed rungs were not yet attached to the receipt when it was written.
An interruption during the second rung therefore erased the first rung
from the recorded evidence. Points are now attached before their first
solve, with validated completed rungs saved atomically as they arrive.

The runner also accepted empty case lists as unanimous support and selected
the first crossing even when several reversals existed. It now refuses
empty coverage, unordered/mismatched/nonfinite crossing data, ambiguous
zero plateaus and multiple crossings. No smoke run can emit scientific
`SUPPORTED` status.

Receipts retain source snapshots, the revision, source-dirty disclosure,
the imported calculation modules, original registration and amendment.
Replay checks source integrity, case coverage and recomputed summaries;
failure replaces an old PASS. Its scope is explicitly **receipt consistency,
not independent matrix replay, authentication, or physical certification**.
Text hashing explicitly normalizes platform line endings. A failing-then-
passing cross-check verifies that a Git LF/CRLF conversion does not destroy
replay portability while changed content remains rejected.

### 4. Level selection and matrix checks needed safeguards

The selector indexed twelve eigenpairs even when fewer existed. It now
uses the available pairs. Accidental harmonic degeneracy is refused when
frequency matching cannot isolate exactly the intended doublet. Nonfinite
and materially asymmetric Hamiltonians are rejected before symmetrization
or diagonalization; only roundoff-scale asymmetry is averaged away.

The selection remains the strongest single-quantum match in the returned
window. It is not an independent certificate of the lowest eigenstate in
an exactly projected charge sector. That limitation is now stated.

### 5. Integer-looking Molien output did not detect aliased quadrature

With M=1, the original routine returned wrong integer counts without
raising its integrality alarm. The quadrature now requires M to exceed
the Fourier support in every integration variable: max(2D+1,
D+max(j_L,j_R)+1) for the requested square k,l<=D. The default grid was
already sufficient and is unchanged. The smallest sufficient grid is
cross-checked against the default in a separate fixture.

The quadrature formula is exact in exact arithmetic under that support
condition. Floating-point evaluation followed by integer rounding is not
an interval-certified dimension proof; the module now makes that distinction.

## What reproduces

All **74 supplied focused tests** pass before and after the implementation
corrections: 49 Helike/curl/Berger/condensate tests and 25 two-grain tests.
The **24 added adversarial and positive controls** also pass, for **98 total**.
The original RED failures, intermediate GREEN run and final focused logs
are retained in the [evidence directory](../../research/flux-review-20261124/README.md).

The passing tests cover the curl/flux decomposition, vector identities,
tested floor conditions, round-model limits, reduced measure identity,
perturbative scaling, local harmonic frequencies, small-coupling Fock/PT
agreement, two-grain potential and sphaleron controls, and low-degree
invariant counting/basis checks. They are computational checks of these
claims, not a substitute for reviewing every theorem in the papers.

All **43 mechanism-removal controls** are caught by assertion failures,
not incidental exceptions: 13 new review controls and 30 supplied
mathematical mutants (curl 6, Berger 4, condensate 3, perturbation 3,
Fock 3, Molien 4, two-grain 7). Their JSON receipts are retained. No full
Rust suite, GPU survey, new continuum run, or registered-order flux grid
is claimed.

## Two-point runner check after publication of the fixes

Source `384e1226f48102eab9880c20387e65435b7dac9a` was committed and pushed
before this check. Its recorded source-dirty flag is false. Only the two
previously exposed smoke points were used: t=3, g^2=0.05, P=Mq=4,
dimension 210. No registered-order computation was performed.

| s | selected Helike gap | selected competitor gap | Helike weight | competitor weight |
| --- | --- | --- | --- | --- |
| 1.15 | 1.1385485969924334 | 1.066829324390432 | 0.9039660097189148 | 0.7982782839680251 |
| 1.20 | 1.1011458375646415 | 1.1610586962138427 | 0.9156837025268787 | 0.8436539260059432 |

The selected levels reverse order. Straight-line interpolation of these
two low-order differences gives s=1.17724231231398. That is a **diagnostic
readout**, not confirmation of the registered prediction 1.1809: the
competitor weights are below the registered 0.9 identification threshold,
there is only one approximation rung, and the physical-domain requirement
remains open. Both points explicitly report `converged: false`.

The two solves took 41.2 and 37.5 seconds. The raw matrix asymmetry was
4.440892098500626e-16 at each point. The run ended
`COMPLETE / SOFTWARE_SMOKE_ONLY`; the
[receipt-consistency replay passed](../../research/flux-review-20261124/smoke-384e1226/verification.json).
The [complete receipt](../../research/flux-review-20261124/smoke-384e1226/receipt.json)
retains unrounded values and source snapshots.

A separate actual invocation of the default command returned exit 2,
`HOLD_DOMAIN / NOT_EVALUATED`, with no cases or solver results. Its
[receipt](../../research/flux-review-20261124/hold-384e1226/receipt.json) and
the independent command-line assertion are retained.

## Exact next mathematical question

Can we construct a semibounded approximation on the positive Gram cone,
with the correct measure, sheet/boundary treatment and charge resolution,
and establish an error estimate for the targeted excitation gaps?

An alternative is a deliberately local quasimode calculation, but it needs
quantified localization/leakage and truncation errors. Well depth alone
does not supply those estimates. Increasing the unrestricted Fock cutoff
is not the next justified experiment.

The original registration is preserved. This review does not move the
predicted crossing, relax tolerances, or convert a diagnostic into a
successful physical gate.
