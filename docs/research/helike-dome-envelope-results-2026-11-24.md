# Dome-envelope controls: measured gains and a truncation-sensitive verdict

24 November 2026. All 80 registered solves are saved and replay successfully.
**Scientific decision: UNRESOLVED_TRUNCATION. The scientific gate is not closed.**
The terminal receipt's status is INVALID because D04 failed, not because the
process crashed. The runner exits nonzero for this registered refusal.

The five-function dome improves the scalar residual in all eight primary cases,
by 1.2% to 33.1%. It beats ordinary expansion per added function in three of eight
cases, not uniformly. One secondary comparison reverses its efficiency verdict
when the series truncation changes. These are separate findings: finite-space
improvement, a mixed primary comparison, and an unresolved stability gate.

## What was tested

The [registered experiment](helike-dome-envelope-preregistration-2026-11-24.md)
asks whether functions centered on the second classical vacuum orbit improve
the lowest excited scalar state more efficiently than adding ordinary
origin-centered basis functions. The Hamiltonian, sector, couplings, width
selection, cutoffs and acceptance thresholds were not changed during execution.

The model is the same finite homogeneous SU(2) sphere model:

\[
H=-\frac12\Delta+\frac12T-3gd+\frac{g^2}{4}(T^2-P),\qquad
T=\operatorname{tr}(qq^{\mathsf T}),\quad
P=\operatorname{tr}((qq^{\mathsf T})^2),\quad d=\det q.
\]

Only the J=0 colour-singlet sector is measured here. The original functions are
invariant monomials times an origin-centered Gaussian. The added functions are

\[
\Phi_m(q)=m(T,P,d)e^{-\beta T/2}F_K((\beta/g)q),\qquad
F(X)=\int_{SO(3)}e^{\operatorname{tr}(R^{\mathsf T}X)}\,dR.
\]

The orbit is \(\{R/g:R\in SO(3)\}\). A common nonzero prefactor of the
orbit-centered Gaussian is omitted because it does not change the trial space.
The finite Haar series \(F_K\), not the infinite series, defines the actual
functions used in each calculation. Gaussian moment formulas evaluate their
matrix elements at the declared working precision. The eigensolve is float64.

The primary addition is five functions: \(m\in\{1,T,d,T^2,P\}\). The one- and
three-function additions are secondary comparisons, not alternative primary
tests selected after seeing the results. A dome function is a structured
combination of higher-degree monomials; counting functions does not count the
arithmetic needed to construct them.

Width selection at N=10 picked beta=2.0 for g^2=1 and beta=3.0 for g^2=2.
Both choices are interior to the frozen grid. Selection minimized the second
scalar Ritz energy, not the later measured residual. The origin widths remain
alpha=1.7 and 2.3 respectively. The earlier timing exposure is disclosed in the
registration; these are the existing controls, not new held-out couplings.

## Primary measurements

The residual is the full-operator quantity
\(\epsilon=\|(H-E)\psi\|\) for a normalized approximate state, including
components outside the trial space. It is not merely the small eigensolver
residual inside the projected matrix. Smaller is better.

All eight five-function additions reduce the scalar residual at K=30. The
percentage below is \(100(1-\epsilon_{\rm dome}/\epsilon_{\rm origin})\).
Values are rounded for display; the receipt retains the unrounded results.

| g^2 | N | Origin residual | +5 dome residual | N+2 origin residual | Reduction with +5 domes |
| --- | --- | --- | --- | --- | --- |
| 1 | 12 | 0.158879704627 | 0.121020520497 | 0.111475664736 | 23.828836% |
| 1 | 14 | 0.111475664736 | 0.0745471051132 | 0.0496736669847 | 33.127015% |
| 1 | 16 | 0.0496736669847 | 0.0382710678173 | 0.0196980095857 | 22.955018% |
| 1 | 18 | 0.0196980095857 | 0.0188398875641 | 0.0116257077322 | 4.356390% |
| 2 | 12 | 0.223010314720 | 0.181813616037 | 0.126114635141 | 18.473001% |
| 2 | 14 | 0.126114635141 | 0.122178306933 | 0.0606694966281 | 3.121230% |
| 2 | 16 | 0.0606694966281 | 0.0574138830798 | 0.0336727966536 | 5.366146% |
| 2 | 18 | 0.0336727966536 | 0.0332814491212 | 0.0172705634475 | 1.162207% |

The N+2 comparator adds 13, 17, 20 and 24 ordinary functions respectively.
It achieves a smaller absolute scalar residual in every primary case, but uses
more added functions. The registered comparison accounts for this:

\[
r_{\rm dome}(\ell)=\frac{\log(\epsilon_\ell(N)/\epsilon_\ell(N+\text{dome}))}{5},
\qquad
r_{\rm orig}(\ell)=\frac{\log(\epsilon_\ell(N)/\epsilon_\ell(N+2))}{\Delta n}.
\]

H1a asks whether the dome has greater scalar gain per added function. H1b asks
whether the dome's scalar gain exceeds its vacuum gain. These are deterministic
comparisons, not statistical significance tests.

| g^2 | N | Scalar r_dome | Scalar r_orig | H1a | H1b |
| --- | --- | --- | --- | --- | --- |
| 1 | 12 | 0.0544374438191 | 0.0272570021240 | true | true |
| 1 | 14 | 0.0804750209218 | 0.0475489035435 | true | true |
| 1 | 16 | 0.0521561504191 | 0.0462478679482 | true | true |
| 1 | 18 | 0.00890825863732 | 0.0219707818572 | false | false |
| 2 | 12 | 0.0408471900222 | 0.0438482099002 | false | true |
| 2 | 14 | 0.00634195721329 | 0.0430441324119 | false | true |
| 2 | 16 | 0.0110309810910 | 0.0294375377309 | false | true |
| 2 | 18 | 0.00233802642181 | 0.0278202822082 | false | false |

H1a holds at 3/8 primary points and H1b at 6/8. Both hold at 3/8. At fixed K=30
the primary pattern is mixed, not an all-eight success. That description does
not override the separate numerical validity gates.

## Why the truncation gate matters

At g^2=1, N=18, the secondary one-function comparison changes its H1a truth
value when the Haar series truncation order changes from K=30 to K=36:

| Quantity | K=30 | K=36 |
| --- | --- | --- |
| Scalar residual | 0.019296837655856977 | 0.019197836562243033 |
| Scalar r_dome | 0.020576363925256483 | 0.025720000747617610 |
| Scalar r_orig | 0.021970781857173615 | 0.021970781857173615 |
| H1a | false | true |
| H1b | true | true |

The gate was explicitly registered for every top-cutoff dome size, not just
the five-function primary. This reversal therefore counts. Neither truncation
is promoted to the correct infinite-series answer. Changing K changes the
trial functions; it is not a change of arithmetic precision or a field-theory
continuum limit. Both finite-K calculations remain measurements of their
specified trial spaces.

The full N=18 sensitivity checks are:

| g^2 | Added functions | H1a at K=30 / K=36 | H1b at K=30 / K=36 | D04 |
| --- | --- | --- | --- | --- |
| 1 | 1 | false / true | true / true | fail |
| 1 | 3 | false / false | false / false | pass |
| 1 | 5 | false / false | false / false | pass |
| 2 | 1 | false / false | false / false | pass |
| 2 | 3 | false / false | true / true | pass |
| 2 | 5 | false / false | false / false | pass |

Neither five-function primary verdict reverses in this check. Nevertheless the
pre-registered all-size condition fails. The result is not relabelled MIXED by
discarding the secondary sensitivity failure.

## Secondary measurements, without selection

All 24 original-grid dome results are shown. The function count is the actual
number added: the code's n_d values 0, 3 and 4 mean one, three and five functions,
not zero, three and four.

| g^2 | N | Functions | Vacuum residual | Scalar residual | Next scalar residual | H1a | H1b |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 12 | 1 | 0.0384785460321 | 0.159147534898 | 0.271114350498 | false | false |
| 1 | 12 | 3 | 0.0353239893283 | 0.132938596518 | 0.266987053066 | true | true |
| 1 | 12 | 5 | 0.0339455857340 | 0.121020520497 | 0.235354542006 | true | true |
| 1 | 14 | 1 | 0.0245812742811 | 0.0949813373699 | 0.155171836731 | true | true |
| 1 | 14 | 3 | 0.0202724341979 | 0.0835997375991 | 0.138543225088 | true | true |
| 1 | 14 | 5 | 0.0187581555118 | 0.0745471051132 | 0.134921673028 | true | true |
| 1 | 16 | 1 | 0.0115748822696 | 0.0475795901478 | 0.0869497824246 | false | false |
| 1 | 16 | 3 | 0.0111260871019 | 0.0411593201265 | 0.0747115860347 | true | true |
| 1 | 16 | 5 | 0.0105064831317 | 0.0382710678173 | 0.0728911498532 | true | true |
| 1 | 18 | 1 | 0.00491692413217 | 0.0192968376559 | 0.0527457844180 | false | true |
| 1 | 18 | 3 | 0.00460948645196 | 0.0188708318117 | 0.0461000289470 | false | false |
| 1 | 18 | 5 | 0.00455988794491 | 0.0188398875641 | 0.0414347152207 | false | false |
| 2 | 12 | 1 | 0.0481338139117 | 0.204631685084 | 0.467898041849 | true | true |
| 2 | 12 | 3 | 0.0434939580092 | 0.182034877823 | 0.434946708976 | true | true |
| 2 | 12 | 5 | 0.0434590955193 | 0.181813616037 | 0.431527373448 | false | true |
| 2 | 14 | 1 | 0.0239818011256 | 0.126436220122 | 0.262209951847 | false | false |
| 2 | 14 | 3 | 0.0239343206412 | 0.124304646929 | 0.261516740246 | false | true |
| 2 | 14 | 5 | 0.0234475726296 | 0.122178306933 | 0.242427401715 | false | true |
| 2 | 16 | 1 | 0.0109776726888 | 0.0595454750410 | 0.162223375017 | false | true |
| 2 | 16 | 3 | 0.0106795163280 | 0.0588961236090 | 0.161478222023 | false | false |
| 2 | 16 | 5 | 0.0106147387336 | 0.0574138830798 | 0.157722666316 | false | true |
| 2 | 18 | 1 | 0.00551209712864 | 0.0337011300543 | 0.0874919383517 | false | false |
| 2 | 18 | 3 | 0.00551103939555 | 0.0333102113741 | 0.0874766957822 | false | true |
| 2 | 18 | 5 | 0.00544715674988 | 0.0332814491212 | 0.0877872466317 | false | false |

Residual reduction is not guaranteed by the variational energy principle.
For example, one function worsens the scalar residual at (g^2,N)=(1,12),
(2,14) and (2,18). The primary five-function addition at (2,18) improves the
scalar but worsens the next scalar residual. These outcomes are retained.

## Geometry of the added component

The measured fraction is \(1-\|P_{\rm origin}\psi\|^2\), computed with the
overlap matrix and the orthogonal projection onto the origin trial space.
It is not a position-space probability around the second vacuum.

| g^2 | N | Vacuum outside-origin fraction | Scalar fraction | Next scalar fraction |
| --- | --- | --- | --- | --- |
| 1 | 12 | 1.32838599277e-6 | 1.91112028413e-5 | 2.13476444435e-4 |
| 1 | 14 | 3.69187734232e-7 | 9.71072975293e-6 | 1.45498264716e-5 |
| 1 | 16 | 2.90459331420e-8 | 7.11647512370e-7 | 4.84931078285e-6 |
| 1 | 18 | 3.16094767360e-9 | 4.80535058033e-8 | 7.59798070824e-7 |
| 2 | 12 | 1.13086152447e-6 | 3.06484513639e-5 | 8.68881750644e-5 |
| 2 | 14 | 4.25669390326e-8 | 2.34699631372e-6 | 2.00681684859e-5 |
| 2 | 16 | 9.92143728255e-9 | 4.85789331096e-7 | 3.49832797381e-6 |
| 2 | 18 | 3.38726965188e-10 | 2.43834207159e-8 | 4.05318614328e-8 |

For the primary additions the scalar's fraction exceeds the vacuum's at all
eight points, but that fact alone does not imply a larger residual gain.
The N=18 comparisons demonstrate the distinction. These are trial-space
projections, not a direct measurement of tunnelling or its mechanism.

## Evidence and scope

| Gate | Observed outcome |
| --- | --- |
| D00, no-dome identity | pass: all eight origin cases agree with the existing J=0 solver within 1e-12 for energies and residuals |
| D01, variational energies | pass: the three measured Ritz levels do not rise beyond 1e-12 in any original-grid augmentation |
| D02, Gram conditioning | pass: maximum reported digits lost is 14.545706; all solves remain below their dps minus 25 limit |
| D03, precision | pass: all ten 120-digit repeats return the same six energies and residuals as their lower-precision comparators at the stored float64 level |
| D04, Haar truncation | fail: one of six top-cutoff dome-size checks reverses H1a |
| D05, projection fractions | pass: original-grid augmented fractions range from 9.315122e-12 to 0.001774213, inside the declared bounds |

The precision repeats include both origin comparators and all three dome sizes
at each coupling. Equality of the returned float64 values is not a claim of
120-digit eigenvalue accuracy. The arithmetic check does not establish
convergence of the untruncated Haar envelope.

The registered source was committed and pushed before execution:
`74ec06775770aaea5e08b8c9eaf71a3c16f27fa3`.
The evidence directory is
[`research/dome-controls-20261124/registered-74ec0677/`](../../research/dome-controls-20261124/registered-74ec0677/).
It retains per-solve matrices, eigenvectors, residuals, source snapshots,
width-selection measurements, immediate progress events and resource samples.

The 80 archives comprise 16 width-selection solves, 48 original-grid solves and
16 precision/truncation repeats. The worker completed the measurements between
10:28:23 and 10:42:52 UTC. The saved-matrix verifier then replayed all 80 archives,
including eigenvalues, residual contractions and origin projections, and checked
source hashes, width selection, case coverage, gain arithmetic and decisions.
Its PASS certifies replay consistency, not satisfaction of the scientific
hypothesis or an independent fresh assembly of the physical operator.

Including replay, the supervised run took 952.484 seconds (15.87 minutes), with
165.793 MiB peak worker RSS. It used one BLAS thread on CPU and remained below
the fixed 45-minute, 8 GiB limits. These are observed resources for this run,
not a runtime-efficiency comparison against another method.

Evidence entry points:

- [Terminal receipt](../../research/dome-controls-20261124/registered-74ec0677/receipt.json)
- [Matrix-replay result](../../research/dome-controls-20261124/registered-74ec0677/verification.json)
- [Immediate progress log](../../research/dome-controls-20261124/registered-74ec0677/events.jsonl)
- [Worker output](../../research/dome-controls-20261124/registered-74ec0677/worker.log)
- [Resource samples](../../research/dome-controls-20261124/registered-74ec0677/resources.csv)
- [Displayed-table check](../../research/dome-controls-20261124/registered-74ec0677/report-check.json), with four altered-number rejection checks

Replay without running another experiment, from a checkout with the pinned
numerical source files:

```powershell
python scripts/run_ym_helike_dome.py research/dome-controls-20261124/registered-74ec0677 --verify
python research/dome-controls-20261124/registered-74ec0677/check_report.py
```

The displayed-table checker covers the 48 rows in the primary-residual,
primary-gain, secondary-residual and outside-origin tables. It does not certify
the surrounding prose or replace the numerical replay.

The [preflight evidence](../../research/dome-controls-20261124/README.md)
records 11 mathematical tests, seven execution tests and nine assertion-based
mutation rejections. The tiny smoke run is a software check, not a scientific
result. No full Rust suite or GIGI endpoint run is claimed for this Python
research experiment.

This experiment measures basis enrichment in a finite model, not a continuum
Yang-Mills gap. Gains per function do not establish gains per second. The
calculations are not interval-certified. The existing geometric-sector results
are not replaced by this additional test, and no first-use or priority claim
is made.

## The next question, not another run

Before treating an efficiency comparison as stable, determine how much Haar
series resolution is needed at the chosen widths and top cutoff. K=36 is not
declared sufficient merely because it gives a more favorable one-function
answer. That requires a new, bounded convergence protocol. An anisotropic dome
is a separate shape hypothesis, not a repair demonstrated by this result.
No further widths, cutoffs, couplings or dome shapes were tried after the
registered outcome.
