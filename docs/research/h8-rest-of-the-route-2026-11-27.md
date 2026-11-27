# H8: the rest of the route I can see

**Status: research plan and geometric conjecture. No spectral ordering is newly established here.**

## The question I am trying to finish

I am keeping the target narrow. At $t=3$, for the *undressed, nine-coordinate reduced operator* at $s=1$, H8 asks whether the second charge-zero eigenvalue lies below the bottom of every nonzero circle-charge sector at each registered coupling
\[
 g^2\in\{0.002,0.004,0.03,0.05,0.07\}.
\]
Writing $E_{2,0}$ for that second charge-zero eigenvalue and $E_{0,m}$ for the lowest eigenvalue in charge $m$, the comparison is
\[
 E_{2,0}<\inf_{m\ne0}E_{0,m}.
\]
The common ground energy cancels. The ground state is separately expected to be simple and invariant; positivity improvement and the symmetries are the natural route to that part. A separate deduction now fixes the reduced-model $O(g^2)$ coefficient for the local charge-zero and charge-$\pm2$ quasimodes near $q=0$ for $t>1$. It does not count all nearby eigenvalues, compare the other well, or settle this whole-operator H8 question. Physical H8 still needs the complete dressed effective Hamiltonian, including H2b.

## What the geometry suggests

The charge is motion around a circle, but not all apparent circle motion is physical motion: some of it is a color-gauge rotation. The useful ruler is therefore the *horizontal* part of the circle generator, after projecting away the color orbit. For charge $m$, that gives the angular kinetic cost
\[
\frac{m^2}{2|X_{\rm hor}(q)|^2}.
\]
At full rank the orbit geometry can be expressed in the Gram variable $G=qq^T$ and its determinant sheet. This replaces nine entries of $q$ with the shape of the color orbit, without throwing away which determinant sheet the point lies on.

The first imagined closure is an all-charge tail. The current probe suggests, but does not establish, that at $t=3$
\[
 \left\|\operatorname{diag}(\sqrt3,1/\sqrt3,1/\sqrt3)q-g\,\operatorname{cof}(q)\right\|
 \ge |X_{\rm hor}(q)|/\sqrt3.
\]
If this is true everywhere, it implies $E_{0,m}\ge |m|/\sqrt3$. Then a certified $E_{2,0}<9/\sqrt3$ would remove every $|m|\ge9$ from the competition. That would turn an infinite list of charges into the finite list $1,\ldots,8$. The small margin between the exploratory harmonic estimate $5.1224$ and $9/\sqrt3\approx5.1962$ makes the upper-bound step important: a harmonic estimate is not enough.

## The route I would take

### 1. Freeze the object before measuring it

Use the exact H8 operator, its Lebesgue-space realization, color-invariant domain, circle action, and the five registered couplings above. Keep the whole reduced configuration space and both determinant sheets. Do not replace the two-well operator by a one-well oscillator, and do not quietly identify the reduced model with physical Yang--Mills.

**Gate:** a written operator/domain convention and a small independent check that the charge action, potential, and Gram formulas reproduce direct calculations in the original nine coordinates.

### 2. Put the horizontal barrier on trial

Rewrite the proposed inequality entirely in Gram variables, retaining the determinant sign wherever it occurs. Prove it on positive-definite $G$ first, then take a controlled limit to every rank-deficient stratum. The degenerate cases matter: a formula derived by dividing by $\lambda_i+\lambda_j$ cannot simply be declared valid when those denominators vanish.

For the full-rank region, the difference of the squared sides is an algebraic/rational positivity problem over $G\succeq0$ and the two determinant sheets. I would try exact factorization or a sum-of-squares certificate after clearing only denominators proved positive on the region. On the boundary, I would derive the horizontal norm directly from the orbit projection and check each stabilizer type separately.

This stage has a clean no-go. One exact counterexample on either sheet or any rank stratum refutes this proposed $|m|/\sqrt3$ tail. That would not refute H8; it would tell me this particular geometric ruler is insufficient, and the next attempt must sharpen the potential-plus-orbit minimization $b_m(g)$ instead of pretending the cutoff exists.

### 3. Certify the charge-zero level from above

For each registered coupling, construct a two-dimensional trial space in the charge-zero form domain. The min--max principle gives an upper bound on $E_{2,0}$ from the largest Rayleigh quotient on that space. Start with the two geometric carriers already present in the model, but compute their actual finite-coupling Rayleigh matrix, including their overlap and off-diagonal entry; do not substitute the harmonic number for the eigenvalue.

The proposed tail needs a strict certified inequality
\[
 E_{2,0}^{\rm upper}<9/\sqrt3.
\]
If the certified interval touches or crosses this threshold, the tail does not close. Improve the trial space or find a different bound; do not round the endpoints until they pass.

### 4. Resolve the remaining charge sectors

With charges $|m|\ge9$ excluded, compare the same charge-zero upper bound against certified lower bounds for $E_{0,m}$, $1\le |m|\le8$. The Hamiltonian is invariant under charge reversal, so $m$ and $-m$ should agree; the computation should still verify that symmetry rather than assume the implementation has respected it.

The finite-dimensional approximation needs a rigorous remainder. One route is a symmetry-adapted conforming Galerkin or finite-element calculation inside a bounded region, with interval-enclosed matrix entries and a proven exterior lower bound from the confining potential. Another is a validated oscillator-basis calculation with an explicit form bound on the omitted complement. The choice can be made after a small pilot tells us which representation preserves the circle and color symmetries cleanly. A Ritz value by itself is only an upper bound; it cannot certify the competing-sector lower bound.

**Pass condition at each coupling:** the upper endpoint for $E_{2,0}$ is strictly below the lower endpoint for every nonzero sector, with the tail inequality covering all remaining charges. Report interval widths and the gap between endpoints, not just decimal eigenvalues.

### 5. Keep the asymptotic and physical questions separate

If the registered comparison passes, the result closes only this finite list of $t=3$ reduced-model cases. A separate small-$g$ argument would be needed for H8$_\infty$: local harmonic ordering, Morse--Bott control around the color orbit, and uniform estimates showing that the remainder cannot reverse the ordering. It must treat the two wells and their charge sectors in one spectral argument.

The physical problem is another layer. At the physical vacuum, charge zero and charge $\pm2$ tie harmonically. The reduced-model origin calculation now gives a negative charge-zero second-order coefficient and zero charge-$\pm2$ correction for its quasimodes. The next physical comparison must combine that result with the complete $O(g^2)$ dressing, including the fast kinetic correction H2b, and bound the remainder below the dressed separation. The all-charge tail argument supplies neither that dressing nor a global spectral count. Even a successful reduced H8 result would not be advertised as a Yang--Mills mass-gap solution.

## What would count as the result

I would call the reduced registered H8 comparison resolved only when every registered coupling has a strict, reproducible enclosure and the charge tail covers every omitted sector. The paper should show the operator, charge convention, quotient geometry, enclosure method, and margin. If the geometric inequality fails, I would publish the counterexample and move to the exact $b_m(g)$ minimization; that is useful information, not a failed proof hidden from view.

## Why this is the path I imagine

The first move is not to diagonalize a larger and larger matrix and hope the pattern settles. It is to use the orbit geometry to ask how expensive a given charge must be, prove or break that estimate on the actual quotient, and only then spend computation on the finite set the geometry leaves behind. The geometry should tell the spectral calculation what it must certify; the spectral calculation should return intervals that can falsify the geometric prediction.

That division of labor is also consistent with established matrix-quantum-mechanics practice: gauge-invariant variables and angular sectors can reduce a model, while spectral calculations still require an explicit treatment of the operator and truncation. Those precedents motivate tools, not this proposed inequality or its conclusion. See, for example, [H.-P. Pavel, “SU(2) Yang–Mills quantum mechanics of spatially constant fields”](https://arxiv.org/abs/hep-th/0701283) and [Errasti Díez, Pandey, and Vaidya, “Born–Oppenheimer quantization of the matrix model for $N=1$ super-Yang–Mills theory”](https://arxiv.org/abs/2001.10524).

## Evidence already in hand, and its limit

The companion note [`h8-charge-tail-geometry-2026-11-27.md`](h8-charge-tail-geometry-2026-11-27.md) records the quotient-aware inequality, Gram identity, and exploratory multi-start results. The deterministic script checks the Gram-potential and horizontal-metric identities on sampled full-rank matrices and finds candidate minima near $|m|/\sqrt3$. It does not certify a global minimum, cover the rank boundary, or enclose any eigenvalue. The separate [`origin-splitting proof`](h8-origin-splitting-proof-2026-11-27.md) derives the reduced local $O(g^2)$ coefficient and its quasimode consequence, while explicitly leaving spectral counting and H2b open. These are distinct pieces: neither substitutes for the other's missing gates.
