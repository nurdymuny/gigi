# Checked sphere transport: GIGI-side DTP follow-up

2026-11-13. Status: implemented and verified for the bounded library scope below.
See [SPHERE_TRANSPORT_RESULTS.md](SPHERE_TRANSPORT_RESULTS.md) for receipts,
environmental failures, exclusions and the explicit non-deployment boundary.

## Scope

Add `gigi::dials::sphere_parallel_transport(from, to, tangent)` and a record
adapter `sphere_parallel_transport_records(from, to, fields, tangent)`.
This is an additive Rust library interface for DTP/embedded-engine consumers,
not a new GQL verb or REST endpoint. Preserve the existing plane-rotation and
WINDOWED_COHERENCE conventions. Do not edit DTP storage, interpretation or tests.

The model is the unit sphere in a declared orthonormal Cartesian frame. Inputs
are three-dimensional; coordinates are dimensionless. It is not a general
manifold solver, a unit converter, or a flight-qualification function.

## Mathematics

For unit endpoints p,q and tangent v at p, choose the unique shortest
great-circle arc. For non-antipodal endpoints its parallel transport is

    T(v) = v - (v dot q)/(1 + p dot q) * (p + q).

The implementation uses the existing plane-rotation constructor with angle
atan2(||p cross q||, p dot q). The formula above is an independent test oracle,
not a second production implementation. R maps p to q and maps tangent vectors
isometrically. Return R as a row-major 3-by-3 matrix acting on column vectors.
Loops compose with the last edge on the left. Arbitrary changes of coordinate
units are outside this Euclidean metric contract.

Numerical policy version 1:
- All arrays have exactly three finite components.
- Endpoint norms differ from one by no more than 1e-12. Normalize accepted
  roundoff deviations, return normalized endpoints and original endpoint norms.
  Never normalize arbitrary non-unit input into acceptance.
- Tangent norm is at most 1e100; zero is valid. Require
  abs(p dot v)/||v|| <= 1e-12 for nonzero v. Do not project away a normal component.
- Refuse when 1 + p dot q <= 1e-8: antipodal/near-antipodal path is unresolved
  under this policy. Ask for an intermediate waypoint away from that regime.
- Same-point transport is identity. The inherited small-angle identity branch
  is accepted only within the same postcondition checks; no exact-symbolic claim.
- Require endpoint residual ||Rp-q||, orthogonality ||R^T R-I||_F, relative
  transported tangency and relative norm error <= 1e-10. Return these diagnostics
  and angle. They are measured residuals, not certified global error bounds.
- The record adapter requires three distinct numeric fields at both endpoints.
  Missing/null/text values must name the field and endpoint in a refusal; no
  zero fill, guessed unit, inferred order, silent projection or numeric-string parse.

## Gates

1. First add a compiling refusal-only scaffold and behavior tests; capture red
   successful-path assertions before implementation (not missing-symbol failures).
2. Quarter sphere: verify all positions, both tangent basis vectors, whole map,
   forward +90, reverse -90, retrace zero, and 21 circuits. A single tangent can
   miss a wrong middle edge; whole-map and position controls are mandatory.
3. Non-axis cases: compare against the independent short-arc formula, norm,
   tangency, inverse and orthonormal-frame covariance. Use predeclared tolerances.
4. Refusal boundaries: dimensions, NaN/infinity, unit length, tangency, magnitude,
   antipodal conditioning, missing/invalid/duplicate fields. Check exact reason
   classes and no caller-input modification. Legacy behavior fence remains green.
5. Mutation checks must fail assertions when rotation is replaced by identity,
   sign is reversed, required guards are bypassed, record data are zero-filled,
   or numerical postconditions are skipped. Compile failure does not count.
6. Embedded GIGI showcase: persist sphere positions, read actual records, derive
   edge matrices with the public checked adapter, transport a tangent basis,
   store typed results, checkpoint, reopen, and verify all answers. Also use a
   non-axis closed path with independently computed expectations. This proves
   embedded use, not deployment of a new HTTP route or raw sensor calibration.
7. Run default all-target Rust tests and focused feature-enabled regression;
   preserve commands, logs, source hashes, exclusions and results. No gate closes
   from HTTP status, a plausible-looking vector, or compilation alone.

## Documentation correction

The lens/lattice holonomy module must describe its restricted SU(2) readout,
not assert trivial holonomy iff trivial fundamental group for arbitrary curved
connections. The sphere is a counterexample to that universal reading. A flat
connection's holonomy factors through the fundamental group; even there trivial
holonomy does not imply trivial fundamental group without further assumptions.

## Out of scope

Changing the legacy GQL rotation's conventions, metric-aware arbitrary frame
changes, exact signed DTP uncertainty, byte-count optimization, uncertain-I/O
state and disk fault injection remain separately owned/gated work.
