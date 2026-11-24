# Flux background review and execution amendment

24 November 2026. Review target `0b330274`. This amendment is written before
any registered-order physical computation in this review. It preserves the
original registration as history rather than silently rewriting its rule.

## Domain finding

The exact colour reduction uses positive-definite G=qq^T, on the chosen
positive-determinant sheet. An oscillator basis on all z in R^6 does not
enforce that domain. Since A(G) is linear, A(-G_bg)=-A(G_bg). With
G=G_bg+Mz and M M^T=A(G_bg), the continued principal matrix at
z=-2 M^-1 G_bg is B=-I. A wave packet supported in a neighbourhood of
that point has negative high-frequency kinetic energy; a finite Taylor
potential cannot bound that contribution below as frequency increases.

This is a defect of the unrestricted continuation, not of the exact
positive-cone reduction or the local perturbation expansion. Small finite
matrices may still give useful local diagnostics. They do not establish
convergence to the physical ground state without a justified domain and
boundary treatment or a quantified local-state approximation. Well depth
alone is neither a tunnelling-error bound nor a positive-cone leakage bound.

Therefore the full registered run is on HOLD_DOMAIN. The default runner
must refuse before constructing a solver and retain NOT_EVALUATED status.
Software smoke runs remain possible but cannot emit a scientific SUPPORTED
verdict. No command-line override may relabel the domain gap as resolved.

## Engineering corrections

1. Test basis refinement (6,6) to (6,8) and Taylor refinement (6,8) to (8,8)
   separately, both against the existing 0.003 threshold. The original rule
   looked only at the latter and could miss a large basis change.
2. Validate finite gaps, weights, sorted coordinates and coverage. An empty
   case list is not unanimous support. Multiple ordering changes are not
   silently reduced to a convenient first crossing.
3. Save the point before its first ladder solve, so a later interruption
   preserves every completed rung. Include the complete imported source
   closure and registration hash, immutable source snapshots and revision.
4. Replay decisions from saved rows; reject changed or incomplete evidence,
   overwrite stale PASS on failure, and keep smoke evidence explicitly
   diagnostic. This is receipt replay, not independent matrix reconstruction.
5. Bound state selection to available eigenpairs, reject ambiguous harmonic
   channel labels and materially asymmetric/nonfinite matrices. Do not
   silently repair a defective matrix by symmetrizing it.
6. Molien quadrature must reject a sampling grid that aliases its Fourier
   support, even if the resulting wrong coefficients happen to be integers.

## Gates

Behavioral RED precedes each implementation fix. Use planted evidence for
unstable basis/stable Taylor, invalid numbers, incomplete runs and failure
after one rung. Test the domain witness directly against A(G). Check the
tiny-spectrum and accidental-degeneracy cases without expensive solves.
Preserve GREEN and mechanism-removal receipts. Run the existing focused
research suites, not the full Rust suite or the held-out physical grid.

The missing mathematical work is explicit: define a semibounded operator
and admissible approximation on the positive cone, or prove a controlled
local-quasimode statement with leakage and truncation error. That work has
not been replaced by looser numerical thresholds.

## Reporting corrections

The original exposure paragraph calls both s=1.15 and s=1.20 registered
points at t=3. Only 1.15 is in the frozen grid; 1.20 is an exposed nearby
diagnostic point. That historical paragraph is retained, with this explicit
correction. The quoted depth range concerns the primary crossing region,
not a uniform tunnelling bound for every primary and secondary point.

Level identification currently selects the strongest single-quantum match
within the returned eigenpairs. It is not an independent proof that this
is the lowest state of an exactly projected charge sector. A later physical
solver must address that identification requirement as well as the domain.

Replay verifies receipt integrity, source snapshots, coverage and derived
summaries. Its hash is an accidental-corruption check, not a signature; it
does not establish that fabricated eigenvalues came from a matrix solve.
The verifier reports this limited scope on every PASS. An interrupted
verification replaces any old PASS with RUNNING before inspecting evidence.
Text hashes use the declared `sha256-universal-newline-v1` policy: CRLF and
CR are normalized to LF before hashing. Source and receipt content changes
still fail; Git's platform line-ending conversion does not invalidate replay.
