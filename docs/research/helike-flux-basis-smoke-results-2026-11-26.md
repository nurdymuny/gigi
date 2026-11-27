# Flux-grid repair smoke: a more accurate calculation, not a repaired gate

**COMPUTED. Overall UNRESOLVED. Full grid held. No mathematical acceptance threshold changed.**

Registration: [one bounded basis test](helike-flux-basis-smoke-registration-2026-11-26.md).
Raw evidence: [receipt](../../research/flux-basis-smoke-20261126/receipt.json),
[Mq 10](../../research/flux-basis-smoke-20261126/mq10.json),
[Mq 12](../../research/flux-basis-smoke-20261126/mq12.json).

## What I tried

I separated two obstacles: the competitor's unresolved basis dependence, and its failure of the stricter
reference-weight selection used by the domain exporter. I tested only one already registered point,
s = 1.18, t = 3, g^2 = 0.03. I first reproduced Mq 10, then increased the total-occupation basis to Mq 12.
The Hamiltonian, chart, measure, exact-moment calculation and numerical tolerances stayed fixed.

The fresh Mq 10 gaps reproduce the saved gate57 result to within 4.3e-14. Both new solves completed within
their individual 900-second limits. No Mq 14 or full-grid run was started.

## Exact measured outcome

| Quantity | Fresh Mq 10 | Mq 12 |
| --- | ---: | ---: |
| Trial dimension | 8,008 | 18,564 |
| Ground energy, with the existing energy shift | 5.420377676484968 | 5.420012553018869 |
| Helike gap | 1.0571307308726174 | 1.0560672396516049 |
| Competitor gap | 1.0966943469227540 | 1.0903923233976611 |
| Helike reference weight w/Z | 0.9650134913870881 | 0.9653883710547538 |
| Competitor reference weight w/Z | 0.9136404500327204 | 0.9128515410191000 |
| Competitor dressed-overlap cross-check | 0.9757546558403825 | 0.9740076181555388 |
| Solver time, seconds | 166.2687 | 393.7830 |

| Frozen check | Measurement | Outcome |
| --- | ---: | --- |
| Helike basis-step change <= 0.003 | 0.001063491221012569 | Pass for this step |
| Competitor basis-step change <= 0.003 | 0.006302023525092793 | **Fail** |
| Grid identification w/Z >= 0.9, both channels | Minimum 0.9128515410191 | Pass at this point |
| Competitor export requirement w/Z >= 0.95 | 0.9128515410191 | **Fails a necessary selection condition** |
| Physical-domain certificate at Mq 12 | Not computed | **Not run; remains open** |

The competitor step decreased from the historical 0.009140197642756753 to 0.006302023525092793: a reduction of
about 31%, but still about 2.10 times the permitted change. The larger basis did not repair either blocker.
These are differences between finite-space estimates, not certified error bars relative to an exact gap.

The new exporter was not invoked: the measured competitor weight already fails its unchanged necessary
condition. Passing that condition alone would not prove the Helike partner selection passes. Independently,
the current rigorous cutoff implementation caps degree at 10; this experiment does not lift that cap.

## Where the discrepancy is, and what I have not established

At Mq 12, the true finite-space residuals for the ground state, selected Helike state and competitor are
4.77e-14, 5.83e-14 and 5.80e-14 respectively. All 12 returned states satisfy the solver's original scaled
1e-10 residual criterion. The largest residual, 6.49e-10, belongs to an upper returned level, not either
selected channel.

The exact-moment versus independent quadrature Rayleigh-quotient cross-check is 9.50e-14 at Mq 12 (7.28e-14 at
Mq 10). Reference-weight quadrature resolves at orders 18 and 20, with discrepancy 5.52e-7, below its frozen
1e-6 rule. Thus the observed 0.00630 movement is not explained by a failed finite-space eigensolve or the
measured operator/quadrature mismatch. The estimates remain sensitive to the size of the trial space.

This does not prove convergence, rule out every shared modeling error, establish the physical boundary
realization, or settle state identity in the continuum. The relatively high dressed-overlap cross-check
does not authorize replacing the registered w/Z measure. I cannot yet say whether the remaining reference
weight deficit is mostly genuine mixing or unresolved representation error.

## What was actually repaired

The grid wrapper now preserves a refused subprocess's command, exit code, stdout and stderr. It still
returns OPEN. Previously a real message such as `ambiguous state selection` was lost behind the generic
`CalledProcessError`. The new behavioral test failed before that three-line repair and passes afterward.

The isolated smoke harness also checks the pinned baseline, replay agreement, finite-space residuals,
weight normalization, unchanged settings, and separate convergence/identification decisions. It refuses
directory reuse and records failures explicitly, so it cannot inherit a stale PASS.

Focused evidence: [test record](../../research/flux-basis-smoke-20261126-tests/README.md).
46 focused tests pass; all eight deliberate mechanism removals are rejected. One separately attempted
archived-domain integration test errored before the binary existed and was not rerun; it is not counted
among those 46. No full-repository green claim is made.

## Smallest next experiment, not executed

I would inspect the competitor's actual occupation coefficients and residual distribution before spending
on a uniform Mq 14 expansion. The question is whether its missing support is concentrated in identifiable
directions or broadly distributed. That measurement can justify a targeted basis enrichment; the present
numbers alone cannot. Any new basis must retain the original operator and thresholds and demonstrate
agreement on the existing controls. State-identification and degree-12 domain certification remain separate
work, not side effects of a smaller basis step.

## Reproduction and provenance

```powershell
cd helike-lab
cargo build --release --bin helike
cd ..
python -u scripts/run_helike_flux_basis_smoke.py research/flux-basis-smoke-NEW-RUN
```

Use a new output directory; the harness refuses an existing one. The raw execution receipts retain commands,
timings and exit codes. Both runs used eight Rayon threads on Windows x86_64, no GPU.

- Solver source revision: `d3e03ad13d99755f9559900e2ec6d6e48ff795c1`; the `helike-lab` source tree was clean.
- Rust solver source digest, universal-newline policy:
  `e4600c0bed5abf04e5e77e5480415695efa436ec712aec1c762c56995d1ec242`.
- Newly built executable SHA256:
  `d981b537d6bd1f1dff305d80c22e833ebddb53b3193d8f768aad7cf0e3cc4c89`.
- Smoke harness was an uncommitted addition during execution; its recorded LF digest is
  `3e2ca5a5965ec627701fd25b9d6a5d9b3e634ba41dca1484e6eab070c274b4ee`.
- Aggregate receipt LF digest, independently checked after completion:
  `a2dbf056709b8b8b02a11d3aa0a8ef09bafad9abb80c5e8f3f3aaa8b8e2e4479`.

This is a bounded negative result for the proposed repair, with a useful localization of the remaining
problem. The prior grid receipt and its verdicts are untouched.
