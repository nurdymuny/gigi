# The O(g²) splitting at the origin: frozen contract

27 November 2026. Pure deduction plus exact polynomial algebra; no spectral matrix computation.

## Statement and dependencies

Keep the unsourced s = 1 reduced Hamiltonian, Euclidean metric, Lebesgue measure and full-space Friedrichs
realization. Fix a compact simple real Lie algebra with positive Ad-invariant inner product, n = its dimension,
t > 1 and g > 0. Let u = t^(-1/2), omega = (sqrt(t), u, u), S = sum omega, C = <X1,[X2,X3]>,
W = sum omega_k ||Xk||^2 / 2 - g C, and H = -Delta/2 + ||grad W||^2/2. Let kappa be the adjoint Casimir in the
chosen metric: sum_c ad(e_c)^2 = -kappa on an orthonormal basis.

**Target (the H7 Step 6 comparison, reduced model, at q = 0).** The two-quantum colour singlets at harmonic level
2u split at order g² as follows.

1. Charge ±2: P = ||X2||² - ||X3||² ± 2i<X2,X3> satisfies L P = 2u P exactly, where L is H - nS/2 written on the
   factor e^(-W). So the formal Rayleigh-Schrödinger series is 2u with every correction zero.
2. Charge 0: the series is 2u + e2 g² + O(g³), with e2 = -2 kappa (1 + 1/t) < 0.
3. Rigorous consequence (quasimodes): in each of the two sectors, H has spectrum within O(e^(-c/g)) of
   nS/2 + 2u (charge ±2) and within O(g³) of nS/2 + 2u + e2 g² (charge 0).

Dependencies: the factorization H = (1/2) sum a*a + nS/2 and the constant Laplacian (vacuum-floor note); the
cubic-quartic bound |C| <= c Q^(3/4) is not needed. Schur's lemma is used for the invariant quadratic forms on a
compact simple algebra.

## Not claimed

- That these are the only spectrum of H near those energies, or that they belong to the q = 0 well rather than the
  q_MC well. That counting step is the harmonic-approximation localization (Simon 1983; Helffer-Sjöstrand), and it
  stays a cited, unformalized boundary.
- Any statement for the physical room. The fast-mode O(g²) dressing (H2b) is not included.
- The q_MC (competitor) reading of H8 Step 7, any finite-g remainder constant, t <= 1, or uniformity in the group.

H7 and H8 remain open; the top-level count remains seven.

## Gates, in order

1. Freeze this statement and the exact tests. Record the missing-proof and missing-Lean-declaration failures before
   writing either.
2. Exact symbolic checks:
   - SU(2) on all nine coordinates: L1 P = 0, L0 P = 2uP, L1 r = -4C, L0 C = (sqrt(t) + 2u) C, e2 = -4(1 + 1/t);
   - SU(3) with Gell-Mann structure constants on all 24 coordinates: L1 P = 0 and e2 = -6(1 + 1/t) (kappa = 3);
   - a rescaled metric: kappa and e2 rescale together;
   - the second-order quasimode: polynomials phi1 and phi2 solve the order-g and order-g² equations exactly;
   - non-degeneracy: at t > 1, r is the only charge-0 singlet eigenpolynomial of L0 at 2u in degree <= 4.
3. Mechanism removal must change the outcome:
   - a cubic that is not ad-invariant gives L1 P != 0;
   - omega2 != omega3 breaks L0 P = 2uP;
   - removing the cubic gives e2 = 0;
   - reversing the resolvent sign flips e2.
4. Lean checks the scalar algebra of e2, the sign, and the conditional ordering deduction from the two quasimode
   estimates. It does NOT formalize polynomial differentiation, Gaussian integration, Schur's lemma, or spectral
   localization. Report this boundary with the result.
5. Fresh output directories only. Retain red and failed receipts, compiler logs, pinned dependencies and source
   hashes. No full engine suite, no GPU run.

## Publication

Add the proof and bounded verification to the research notes. Update only the H7 Step 6 and H8 entries it touches
and the closed-set change log. Do not change the paper, model, normalizations, thresholds or any top-level status.
