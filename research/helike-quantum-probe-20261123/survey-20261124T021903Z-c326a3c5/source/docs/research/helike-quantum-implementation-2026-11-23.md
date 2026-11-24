# Helike quantum probe implementation record

**Bee Rosa Davis | Davis Geometric**  
23 November 2026. Engineering details fixed before the interacting calculation.

This implements the [experiment](helike-quantum-probe-preregistration-2026-11-23.md)
and [GPU amendment](helike-quantum-gpu-amendment-2026-11-23.md), without changing
their Hamiltonian, probes, couplings, cutoffs or tolerances.

## The whole physical polynomial space

For three colour vectors in R^3, the SO(3) invariant polynomials are generated
by their six scalar products and their determinant. This is the special
orthogonal first fundamental theorem; see Kraft and Procesi,
[*Classical Invariant Theory*, Theorem 10.2](https://dmi.unibas.ch/fileadmin/user_upload/dmi/Personen/Kraft_Hanspeter/Classical_Invariant_Theory.pdf).
The determinant squared equals the Gram determinant. Therefore powers zero and
one of the determinant suffice. Independence follows on nonsingular matrices:
both determinant signs are realized at a positive-definite Gram matrix, so a
relation a(S) + det(M)b(S) = 0 forces both polynomials to vanish on an open set.
This establishes the basis count, not merely a numerical rank observation.

## Exact Gaussian preconditioning

Replace each homogeneous invariant polynomial of degree d by its Wick-ordered
polynomial for Gaussian weight exp(-q.q). In one dimension use the monic Hermite
polynomials h_n = 2^(-n) H_n, with exact inner product

    <h_n, h_m> = delta_nm n! / 2^n.

Wick ordering sends q^n to h_n and lowers only lower-degree terms. It commutes
with colour and spatial rotations. Since all lower invariant degrees are
included, this is a triangular invertible change of the registered basis, not
a truncation, new ansatz or dropped Gram direction. Degrees become exactly
orthogonal. Integer polynomial coefficients and factorial weights factor the
Gaussian moment matrix analytically; only the final square roots and numerical
matrix operations use double precision. No sampled quadrature is introduced.

The primary builder generates invariant monomials using integer polynomial
multiplication, converts their Wick-ordered coefficients into normalized
oscillator coordinates, and contracts the Hamiltonian and observables there.
The Gram matrix is retained and the generalized problem solved explicitly.

The independent N = 4 builder does not use those invariant polynomials. It
enumerates all nine-coordinate oscillator states, builds the three infinitesimal
colour rotations, and finds their common kernel. It applies coordinate matrices
on the complete N <= 8 oscillator space before projecting, so quartic products
are not incorrectly truncated between factors. It constructs the potential from
the nine squared two-by-two minors and the signed determinant. Compare physical
subspaces, Hamiltonians, probes and complete spectra between the two routes.

## Numerical checks

Generalized CUDA solves retain S-normalization and original-problem residuals.
The CPU SciPy generalized solver is a separate numerical backend. An explicit
CPU request is allowed for reference work; a CUDA request may not fall back.

The full physical basis at each cutoff is used before probe selection. Exterior
residuals and probe tails are computed by applying untruncated polynomial
operators into oscillator degrees up to N + 4. The formula uses action degree
N + 4; squared norms are evaluated in that orthonormal space, not by incorrectly
limiting Gaussian moment degree to N + 4.

Report finite-cutoff observations even when convergence fails. Do not calculate
the registered usefulness score until all of its prerequisites pass. A resource
stop or numerical disagreement cannot become a negative physics conclusion.

## Gate sequence

First add failing tests for the physical basis, oscillator answer, independent
construction, generalized metric, precision/device refusals, projector weights,
zero-overlap handling and receipt validity. Then implement and run the focused
tests. Deliberate mechanism-removal checks follow before any survey result is
accepted. No GIGI server or full engine regression suite is involved in these
standalone research programs.

Status at creation: implementation and interacting survey **NOT RUN**.
