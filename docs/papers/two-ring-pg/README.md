# Reading the Curve, Not Counting the Points

**Paper:** [`paper.pdf`](paper.pdf) (16 pages). **Author:** Bee Rosa Davis, Davis Geometric.

This branch holds a paper, the GIGI code behind every number in it, and the record
of how the result was found, including every attempt that failed. This README
explains what you are looking at.

## The short version

Take a smooth gauge field on a curved space, the Poincaré homology sphere, and keep
only the numbers it leaves along the edges of a mesh. Can you compute, from those
edge numbers alone, a force that converges to the true one as the mesh gets finer?

The standard lowest-order method (Whitney forms) says no. Its force stays **26%
wrong** however fine the mesh: past the coarsest mesh, refining all the way to
15,000 cells changes nothing.
This paper shows the problem is not the data. It is how the curvature is *read*
from the data. Read it from each cell's neighbours, with enough edge to spare, and
the error falls to **1.45%** and keeps falling. The dynamics are stable, no
parameter is fitted, and no data are added.

| construction | force error at the finest mesh | stable? |
| --- | ---: | --- |
| Whitney (standard) | 26.1% (stuck) | yes |
| one-ring recovery | 1.17% | **no**: hundreds of growing modes (489 of 961 at n = 2) |
| **two-ring recovery (this paper)** | **1.45%** | **yes**: no growing mode at any level checked |

## A common-sense way in

A child with a shape-sorter doesn't force the square block into the round hole. She
turns the block until its outline matches an opening, and if nothing matches, she
stops. An archaeologist does the same thing with a single broken rim sherd. She lays
its edge against a chart of circles and reads off the width of the whole pot.

Both know something the standard method doesn't. A short sherd with barely three
good points fits *some* circle no matter what, so a chip reads as curvature. Give
the reading more edge and a chip gets outvoted instead of obeyed, while a true
circle is still read exactly. That is the whole idea of the paper, worked out
rigorously on a 3-manifold:

- turn each neighbouring piece into register by the exact symmetry of the space;
- hold exactly only what must be held;
- let everything else vote;
- refuse a piece that doesn't fit.

**Please read the paper.** The opening pages make the argument in plain terms
before any equation appears.

## How it came to be

This was not one clean idea. It was found over several tries, and every try is in
this repository with its receipts. In order:

1. **The continuum side.** For one class of homogeneous SU(2) metrics, the Ricci
   velocity mapped through the Levi-Civita connection equals the negative
   Yang–Mills gradient exactly. That raised the lattice question this paper answers
   for one case. The scope is stated carefully in the paper; it makes no Ricci-flow
   claim.
2. **The wall.** On four shape-regular meshes of the Poincaré homology sphere, the
   Whitney force error stalls at 26% (`docs/RICCI_SHAPE_REGULAR_*`,
   `docs/RICCI_OPERATOR_CONSISTENCY_*`).
3. **Reconstructions that didn't get through.** Shared-face constraints, shared
   curvature patches, local pushes, boundary extensions and a final bounded
   attempt are each specified and measured in `docs/RICCI_*`. None passed the
   convergence gate.
4. **The diagnosis.** The floor is structural. Whitney curvature is
   discontinuous across faces in exactly the way that multiplies a small error by
   a large test field.
5. **A consistent force that was unstable.** The Petrov–Galerkin recovery keeps
   the original test functions and replaces only the curvature
   (`docs/RICCI_PG_CURVATURE_*`). It reached 1.17%, but its operator had
   hundreds of growing modes (489 of 961 at n = 2).
6. **Ways around it that failed.** Each is documented, with its number:
   - making the force an energy gradient (variational) diverged, at 663–778%;
   - a zero-mean correction stayed at 40.5% (`docs/RICCI_ZERO_MEAN_RESULTS.md`);
   - a symmetric projection layer was stable but floored near 6%
     (`docs/RICCI_PROJECTION_LAYER_RESULTS.md`);
   - an adjoint-consistent recovery was 44% off at the coarsest level
     (`docs/RICCI_ADJOINT_CONSISTENT_RECOVERY_SPEC.md`).
7. **The fix.** The instability was never inconsistency; it was conditioning.
   The one-ring fit is *exactly determined*, so it has to read every rough
   pattern as a large curvature. Widening the reading to two rings makes it
   overdetermined. It stays exact on true shape, and the instability disappears
   (`docs/RICCI_PG_TWORING_RESULTS.md`). That is this paper.
8. **Into the engine.** The two-ring recovery became a GIGI verb,
   `RECOVER_CURVATURE`, checked cell by cell against the research code on every
   mesh, and on a live engine.

## What is claimed, and what is not

**Claimed.** For one Abelian control on the Poincaré homology sphere:

- the two-ring force passes the BW05 convergence gate, with errors 6.22%, 3.83%,
  2.79% and 1.45% at n = 1, 2, 3, 5;
- it has no growing mode at the levels decomposed (n ≤ 3);
- it is dissipative at n = 1, 2.

**Not claimed.** The paper makes no Ricci-flow claim, and it covers none of the
Berger metrics, the full SU(2) nonlinearity or time integration. The force is not
the gradient of an energy. Whether a reciprocal version exists is an open question,
and the paper states exactly what it would take.

## Don't take my word for it: run it

Every number is computed in GIGI, an open-source geometric engine:

- the native Rust transport and mass assembly are checked against the full
  120-fold cover;
- the two-ring recovery is a native engine verb, `RECOVER_CURVATURE`, and it matches
  the research code cell by cell on every mesh;
- the inputs are frozen and hash-pinned.

The paper shows the code next to the mathematics.

```bash
# from the repository root, after extracting each archive in
# research/ricci-checkpoint-20261117/ at the root (restores output/)
python docs/papers/two-ring-pg/make_data.py              # errors.json, spectra.npz, conditioning.json  (~15 min)
python docs/papers/two-ring-pg/numerical_range.py        # numerical_range.json (n = 1, 2)
python docs/papers/two-ring-pg/make_tables.py            # tables/*.tex (orders from absolute error, as in BW05)
python docs/papers/two-ring-pg/make_figures.py           # figures/*.pdf
python scripts/ricci_pg_tworing_gate.py                  # the BW05 and stability verdicts
cd docs/papers/two-ring-pg && pdflatex paper.tex && pdflatex paper.tex
```

Native checks. Build the probe, then point the tests at it:

```bash
cargo build --release --features gauge --example ricci_operator_probe
GIGI_RF2B_OPERATOR_BINARY=target/release/examples/ricci_operator_probe \
  python -m unittest e2e/probes/test_ricci_operator_native.py
```

The engine verb. Its gates, its parity with the research code, and the live
engine run (receipts in `research/recover-curvature-parity-20261117/`):

```bash
cargo test --release --features gauge --test recover_curvature
cargo build --release --features gauge --example recover_curvature_probe
GIGI_RECOVER_CURVATURE_PROBE=target/release/examples/recover_curvature_probe \
  python -m unittest e2e/probes/test_recover_curvature_parity.py
```

```sql
RECOVER_CURVATURE cells FROM edges ON a RINGS 2;
```

Spec: [`docs/RECOVER_CURVATURE_SPEC.md`](../../RECOVER_CURVATURE_SPEC.md).

`make_tables.py` also reads the gate receipt
`research/ricci-pg-tworing-20261117/diagnostic.json`, produced by
`scripts/ricci_pg_tworing_gate.py`. The paper needs only standard LaTeX packages.

If you disagree with a number, the code that produced it is right here.

## About the author

I'm Bee Rosa Davis, founder of Davis Geometric and creator of GIGI. I work on the
geometry of how things fit: fiber bundles, gauge fields and curvature, and why
flat approximations of curved things fail. My books on the subject are available
on Amazon. I've just published second editions of four of them on my website,
[Davis Geometric](https://davisgeometric.com), where you can read them all for free.

## Licences

- **The paper:** CC BY-NC 4.0.
- **GIGI:** PolyForm Noncommercial 1.0.0 (see [`LICENSE`](../../../LICENSE)).

Both are free for education, research and non-commercial use. For commercial use,
contact Davis Geometric.
