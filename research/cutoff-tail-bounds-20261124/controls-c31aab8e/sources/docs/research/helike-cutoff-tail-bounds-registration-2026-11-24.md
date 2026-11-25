# Rigorous cutoff and infinite-tail bounds: frozen construction

24 November 2026. Registered before the new four-control bound calculation.
This is an explicit analytical-method amendment to section 4 of the quasimode
construction registration. It replaces box integration **only for cutoff leakage,
removed mass and normalization**, not the residual or charge calculations.

## Scope and frozen inputs

Keep s=1.15, t=3, g^2=0.004,0.002, Mq=6,8 and the four saved states per control.
Use the hash-verified controls-e21087a1 archive; do not run an eigensolver, refit
coefficients, change the Hamiltonian, select different states, or release the grid.
All prior scientific thresholds remain unchanged, including leakage norm <=1e-10.
The absolute enclosure-width budgets remain 1e-12 for squared tails/leakage and
1e-10 for normalization. No claim about spectral ordering follows from this gate.

The cutoff remains eta=h(D/Ds)[1-h(N/R)], Ds=0.01 det G0, R=4||G0||F,
D=det(qq^T), N=||qq^T||F. Both derivative terms remain:

    [H,eta]psi = -grad eta . grad psi - (Delta eta)psi/2.

The positive-sheet function is extended by zero through the other sheet. The
cutoff vanishes near rank loss; no logarithmic chart is evaluated there.

## Exact trial definition and arithmetic

Use python-flint 0.9.0 Arb at 128-bit midpoint precision, with outward ball
arithmetic throughout. No float64 or sampling step may be used to certify an
inequality. Binary64 values defining a trial function are exact dyadic constants,
not asserted exact eigenvectors or exact symbolic model parameters.

Freeze the stored S, w, basis and the binary64 product coeff@mixing used by the
previous diagnostic. Reconstruct u0 by the existing ConeChart once, record its
bits, and verify the reconstructed S agrees bitwise with saved S. Save the full
derived trial constants and their byte hash. In differentiation, invert the exact
saved S with Arb; do not treat its floating approximate inverse as exact.
The physical density is the exact function 8|det S|exp(3u0+2u1+u2), not a rounded
log-density approximation. This defines the mathematical trial represented by
the saved constants; earlier floating evaluators approximate it.

Enclose both exact mathematical Ds,R from G0 and the binary64 values used by the
old cutoff evaluator. Prove bounds uniformly over these enclosures. This covers
the declared cutoff and its previous constant-rounding implementation, without
adjusting its location to obtain a pass. No Hamiltonian potential is changed.

Serialize authoritative endpoints as integer-mantissa/base-two-exponent pairs.
Decimal displays are descriptive only. Underflow to a displayed zero cannot
replace a positive bound. All comparisons use balls or exact dyadic endpoints.

## 1. A geometry-only plateau ball

Put x_j=sqrt(w_j)z_j, A=S diag(1/sqrt(w)), u=u0+Ax. The norm measure is
pi^-3 exp(-|x|^2) dx. For |x|<=rho,

    D >= exp(2 sum(u0[0:3]) - rho ||2 sum(A[0:3,:])||).

Bound Cholesky diagonal entries by exp(u0_i+rho||A_i||), and each off-diagonal
entry by |u0_i|+rho||A_i||. Form entrywise absolute upper bounds for LL^T:

    G00 <= r0^2; G11 <= r1^2+a^2; G22 <= r2^2+b^2+c^2;
    |G01| <= r0 a; |G02| <= r0 b; |G12| <= a b+r1 c.

Their Frobenius norm (off-diagonals counted twice) bounds N. Select the largest
rho in the fixed list {1/4,2/4,...,64/4} whose entire ball is proved to satisfy
D>=2Ds and N<=R. This rule uses geometry alone, never coefficients, measured
leakage, or acceptance thresholds. Refuse if no candidate is proved. No radius
search beyond this list and no tighter post-result replacement in this run.

All removed mass and all nonzero cutoff derivatives then lie in |x|>rho.

## 2. Global cutoff derivative majorants

For s=x-1 in (0,1/2], y=1/s>=2, the logistic representation gives
h(1-h)<=exp(2-y). Symmetry covers the other half. Consequently

    |h'| <= exp(2-y)(y^2+4) <= 8,
    |h''| <= exp(2-y)(y^4+2y^3+8y^2+32) < 128.

For the second inequality maximize each term separately: 256/e^2+54/e+64<128,
using e>5/2. The flat endpoints have zero derivatives. Fix H1=8,H2=128.

Let U=2R. On derivative support D>=Ds, N<=U; on inner-derivative support D<=2Ds.
Use ||cof(q)||F^2<=N^2, tr(G^3)<=N^3, tr(G)<=sqrt(3)N and the Cartesian
identities already independently checked in the construction suite. With

    ga=sqrt(8 U^2/Ds), gb=2sqrt(U)/R,
    la=2U^2/Ds, lb=14sqrt(3)/R,
    Geta=8(ga+gb),
    Leta=128ga^2+8la+128gb^2+8lb+2*8^2*ga*gb,

we have |grad eta|<=Geta and |Delta eta|<=Leta. The final mixed term must not be
removed. Use lower denominators and upper numerators across the Ds,R enclosure.

## 3. Polynomial and infinite Gaussian tails

For normalized Hermites h_n=H_n/sqrt(2^n n!), r>=rho and |x_j|<=r,

    |h_n(x_j)| <= k_n(rho) r^n,
    k_n = n!/sqrt(2^n n!) * sum_k 2^(n-2k)/(k!(n-2k)!rho^(2k)).

For a multi-index take products of k_n. Let m be maximum total degree, C_n the
fixed complex coefficients, and

    aP=sum_n |C_n| k_n / rho^(m-|n|),
    b_j=sum_(n_j>0) |C_n|sqrt(2n_j) k_(n-e_j)/rho^(m-|n|).

Then |P|<=aP r^m. Including both Gaussian and density derivatives,

    D_i P = sum_j (S^-1)_(j,i)(partial_zj-w_j z_j)P - p_i P/2,
    p=(3,2,1,0,0,0),
    aDi=sum_j |(S^-1)_(j,i)|sqrt(w_j)(aP+b_j/rho^2)+p_i*aP/(2rho),
    |D_i P|<=aDi r^(m+1).

The exact radial integral in six dimensions is

    T_k(rho,lambda) = pi^-3 integral_(|x|>rho) r^(2k) exp(-lambda r^2) dx
      = (k+2)! exp(-lambda rho^2)/(2 lambda^(k+3))
        * sum_(j=0..k+2) (lambda rho^2)^j/j!.

This covers the infinite exterior, not just a larger finite box. Removed squared
norm is bounded by aP^2 T_m(rho,1), which also bounds *all* mass outside the ball.

## 4. Physical-gradient bound without a global chart condition estimate

Use the unchanged exact finite Laurent cometric B_u from the positive-cone
module. Verify its integer coefficients and independently verify its symbolic
Cartesian metric identity. Bound every term, including off-diagonal entries.

For a metric monomial c exp(sum_i<3 k_i u_i) prod_i>=3 u_i^k_i, put

    beta_vector=sum_i<3 k_i A_i,
    q=sum_i>=3 k_i,
    F=|c|exp(sum_i<3 k_i u0_i) prod_i>=3 (|u0_i|/rho+||A_i||)^k_i.

For r>=rho the absolute monomial is <=F r^q exp(|beta_vector|r).
Freeze epsilon=1/16. Young's inequality bounds the exponential by
exp(||beta_vector||^2/(4epsilon)) exp(epsilon r^2).
For each ordered metric entry (i,j), multiply F by aDi*aDj and integrate the
result against the radial Gaussian with lambda=15/16. For odd q use
r^(2m+2+q)<=r^(2m+2+q+1)/rho. Sum **every** term with positive upper bounds.
Call this bound Q for integral_(exterior) |grad_q psi|^2. This retains the exact
nonconstant cometric, Gaussian derivatives, density derivative and inverse chart.

If M bounds exterior squared norm, the cutoff commutator norm is at most

    Geta sqrt(Q) + (Leta/2)sqrt(M).

Hermite orthogonality gives the uncut squared norm exactly as sum |C_n|^2;
enclose it with Arb and divide all squared bounds by its positive lower endpoint.
The normalized cut squared norm lies in [max(0,1-M/N_lower),1]. This follows
because eta=1 on the ball and 0<=eta<=1 everywhere. It is not inferred from
quadrature returning one. Fail closed on a nonpositive norm lower endpoint.

## 5. Gates, budgets and reporting

1. Behavioral RED before implementation. Known radial integrals (including tails
   below binary64 range), complex Hermite coefficients, non-unit normalization,
   plateau refusals, exact outward endpoint serialization, and malformed data.
2. Independent oracles: high-precision incomplete gamma; explicit Hermite
   polynomials; exact symbolic Cartesian metric; manufactured shells with
   nonzero derivatives. Numerical oracle agreement tests an implementation;
   the inequalities plus outward arithmetic supply the enclosure argument.
3. Assertion-caught mutants: missing radial normalization, finite-tail truncation,
   discarded complex part, wrong derivative terms, dropped metric terms,
   dropped cutoff commutator term, wrong plateau acceptance, omitted
   normalization, inward-rounded serialization, and false full certification.
4. Commit and push spec/code/focused tests before the same four-control run.
   Fixed 128-bit balls, no adaptive precision, <=900 seconds/control, <=12 GiB,
   <=100,000 analytic metric-term contributions/control. No box budget increase.
5. Save checkpoints, exact derived constants, source snapshots, versions and
   hashes. A failed computation must not leave an overall PASS. No reused output
   directory. Acceptance requires all sixteen states, not a favorable subset.

The only possible newly closed gate is CUTOFF_TAIL_BOUNDS. Full status remains
UNRESOLVED_INTEGRATION, certified=false, full_grid_released=false: residual and
charge norms still need rigorous enclosures, followed by projection and spectral
separation arguments. Loose but valid bounds are reported as an open gate, not
as evidence that the physical state fails.

References: [Arb ball arithmetic and directed bounds](https://python-flint.readthedocs.io/en/latest/arb.html),
[upper incomplete gamma integer identity, DLMF 8.4.8](https://dlmf.nist.gov/8.4.E8).
