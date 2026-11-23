# Results: one reconstruction, both force routes, Abelian control

2026-11-17. Spec: `RICCI_PG_CURVATURE_SPEC.md` (with two dated
amendments). Evidence: `output/ricci-pg-curvature-20261117/`. Python research
scripts, same methods as RF2D-RF2G; nothing ran through the GIGI engine. No
native change, full suite, Berger run, commit or deployment.

**BW05 is not closed.** The route Bee chose, Petrov-Galerkin, passes every
static BW05 number but fails stability. The variational alternative is stable
but inconsistent. With this reconstruction, the two requirements exclude each
other.

## The table that matters

Static instantaneous force, BW05 measurement exactly as `scalar_control`, q6.

| | n=1 | n=2 | n=3 | n=5 | Consistent? | Stable? |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| Original Whitney (RF2D, reproduced exactly) | 29.81% | 25.99% | 25.86% | 26.13% | no, floor | yes |
| Petrov-Galerkin, this reconstruction | 4.84% | 3.55% | 2.30% | **1.17%** | **yes** | **no** |
| Variational, same reconstruction | 58.2% | 663% | 778% | not run | **no, growing** | yes |

This is the consistency-versus-stability tradeoff, mapped completely for one
reconstruction.

## What is established

**The floor is not in the data.** From the original signed edge observations
alone, a closed linear curvature on each cell's face-star converges at order
1.84, 1.93, 1.97 (0.193% at n=5), and the Petrov-Galerkin force built from it
reaches 1.17% at n=5 with rates 1.07 and 1.32, against BW05's 5% and 0.5. The
q4/q6 differences are 6.5e-12 to 1.9e-8, more than six orders below the error.
No fitted parameter, time scale, projection, or target force was used.

**Petrov-Galerkin is unstable on rough modes.** Against the native mass, 70 of
121 physical modes at n=1 and 489 of 961 at n=2 have negative real part, most
negative −13,479 and −23,940. Gauge counts match Whitney exactly (21, 163).
The unstable modes are the rough ones: Whitney roughness at least 317 and 1283
respectively, while the smooth target mode, roughness 4.0, is stable. Damping
them would need a fitted stabilization parameter, which the contract excludes.
Level 2 was not decomposed, as S3 had already failed at levels 0 and 1.

**The variational force answers the handoff's central question, negatively, for
this reconstruction.** Its energy is correct: density 1.899, 1.973, 1.987
towards 2; Rayleigh quotient 3.882, 3.968, 3.985 towards 4. Its gradient is
wrong by 58%, 663%, 778%. Accurate curvature did not give an accurate force,
because differentiating the reconstruction changes the test variations. n=5
was not run: the error is already 150 times the threshold and growing.

## What was verified, and how

- **M1**: with the Whitney trial, this assembly reproduces RF2D's published
  errors at all four levels to every printed digit, so the measurement is BW05's.
- **M2**: Whitney curvature converges at order 0.97, 0.99, 1.00, distinguishing
  it from the reconstruction's order two.
- **13 focused tests**, RED before each implementation phase, then GREEN.
  Two RED runs first errored at class setup and are kept
  (`RED0_setup_only.log`, `RED_R3_class_level.log`); each was redone so every
  test reached its own assertion.
- **16 mechanism removals, all caught** (`MUTATIONS_R3.json`). The first run
  missed one: replacing the equality-constrained fit with plain least squares
  changed nothing, because every face-star is exactly determined (16 faces, 5
  closure relations, 11 unknowns). That mutation was inert, not a weak test.
  The spec was amended and the actual mechanism, exact interpolation with
  refusal, became T9 (`MUTATIONS.json` keeps the miss).
- **Not exercised**: every cell weight in this data is 1.0, so weight handling
  is carried for fidelity with `assemble` but untested.

## Two findings about the mesh

**Deck transforms are left quaternion multiplications.** At n=2, 1728 face pairs
store shared vertices identically and 192 differ by a left multiplication,
residual 3.3e-16; right multiplication does not fit. `base_log` is invariant
under left multiplication, so the Abelian data are consistent across sheets.
Ignoring the transport drove conditioning to 2.4e14; with it, the worst cell at
every level is about 134. The coincidence check (T8) refuses untransported
neighbours. Conditioning alone would not have: it stayed under the 1e12 cap at
n=1 and n=2. No sign or branch error was found.

**The 2585 in the HHE section is this operator.** The largest Whitney eigenvalue
here is 2585.5146598 at n=1, equal to the HHE value, and 9247.6 at n=2, a
ratio of 3.58 against the factor 4 that one-over-h-squared scaling predicts.

## Not claimed

No closure of BW05. No Berger result, no SU(2) nonlinearity, no flow. No claim
that every reconstruction faces this tradeoff; only this one was tested. No
claim about the cause of the variational failure beyond what is measured: the
energy is accurate and its gradient is not.

## Open directions, for Bee to choose among

1. **A reconstruction designed for adjoint consistency.** The variational route
   needs the reconstruction's transpose to behave like Whitney's on smooth
   fields. The assessment's sufficient condition, a correction to Whitney with
   zero mean on every cell, is a design target that this reconstruction does
   not meet. It was built to replace curvature wholesale, not as Whitney plus
   a correction. **Update, same day: that condition was wrong, and option 1
   was then run and failed BW05 at 40.5%; see `RICCI_ZERO_MEAN_RESULTS.md`.**
2. **Stabilized Petrov-Galerkin.** Keep the consistent force and damp only rough
   modes. Every version found so far needs a parameter, which conflicts with
   the contract unless Bee chooses to change it.
3. **Stop here** with the positive result in hand: the RF2D floor is a property
   of how curvature is represented, not of the original data.
