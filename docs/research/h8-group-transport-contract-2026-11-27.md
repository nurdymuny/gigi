# Contract: the curved path in a larger colour space

27 November 2026. Frozen before implementation. Baseline `e213f9fd`.

## Claim and unchanged model

Use the unsourced reduced model, t > 0, u = t^(-1/2), p = gq, g > 0.
For a compact simple real Lie algebra with a positive Ad-invariant inner
product, put

    W_G(X) = (||X1||^2/u + u||X2||^2 + u||X3||^2)/2
             - <X1,[X2,X3]>,    V_G = ||grad W_G||^2/2.

Choose an injective Lie-algebra map i: su(2) -> g. Fix [e2,e3] = e1
cyclically and <ea,eb> = delta_ab in the source. Measure I > 0 from
<i(a),i(b)>_G = I <a,b>_2; do not infer it from a representation name.
The endpoint is the simultaneous Ad(G) orbit of (u i(e1), i(e2), i(e3)).

Target: the ambient zero-energy Agmon distance from 0 to this specified
orbit is I u/2, with original-q action I u/(2g^2). Paths may leave the
embedded subalgebra. Preserve the Frobenius/product metric, the kinetic
factor 1/2, and the existing H8 path theorem.

## Required written deduction

1. Derive the full ambient gradient from Ad-invariance, including all
   three cyclic derivatives of the cubic term.
2. Justify the scalar pullback metric and its positivity. An SU(2) group
   map may have kernel {+1,-1}; only its Lie-algebra map must be injective.
3. Prove W and V restrict with factor I while the vector field intertwines
   without I or 1/I. The induced planar metric is I(dx^2 + 2dy^2).
4. Transport the previously established global orbit, including limits.
5. Prove the action lower bound for every ambient path to every point of
   the specified orbit. Attain its infimum with the transported path and
   vanishing endpoint connectors. Extra critical points do not invalidate
   an endpoint-specific lower bound.
6. State the extension to each compact simple algebra using a cited
   rank-one-subalgebra theorem. No classification computation is implied.

## Frozen gates

- Red: absent proof note and absent named Lean declarations must fail;
  retain the receipt with passed=false, not a stale success.
- Exact fixtures: SU(3) block embedding (I=1), irreducible spin-1 embedding
  in SU(3) (I=4), two fundamental blocks in SU(4) (I=2), and a conjugated
  SU(3) embedding. Use -2 trace(AB), anti-Hermitian generators.
- Check brackets, the full Gram matrix, arbitrary nine-coordinate W/V
  restriction, inherited metric, flow, critical endpoints, action scaling,
  conjugation invariance and original-q conversion in exact arithmetic.
- Independently differentiate W in all 24 SU(3) coordinate directions;
  do not only check the projected three-colour equations.
- Negative controls: scaled/non-homomorphic generators, reversed bracket,
  inconsistent declared metric factor, and an artificial inverse-I flow
  must be rejected. Test actual failures, not alternative accepted outcomes.
- Lean: compile conditional linear-map/bracket/metric transfer identities,
  local dissipation, endpoint and action factors, and a conditional distance
  squeeze. Audit every named target; reject missing/untrusted dependencies.
  Mutate away bracket preservation, omit I, change the metric's repeated
  coordinate, and halve the claimed endpoint action; each must fail.
- Replay focused H7/H8 checks. Hash the checked sources and keep commands,
  exits, failures and toolchain pins. No simulation, GPU or engine suite.

## Formal and publication boundary

The general Lie-group structure theorem, ambient differential geometry,
global ODE existence and path infimum remain in the written proof. Lean
must expose every assumed bracket/metric preservation identity and every
assumed global distance bound. It is not an end-to-end formal theorem of
all compact simple groups. Exact matrix fixtures are examples, not a
substitute for the general deduction.

H7/H8 remain open. No lowest-carrier, prefactor, quantum-gap, continuum,
uniqueness-of-path or minimum-over-all-embeddings claim. I is a measured
metric factor, not automatically a normalized integer Dynkin index.
Do not edit the paper source/PDF or any engine implementation. Commit and
push the bounded proof, checks and evidence on the existing branch.
