# Referee report on `H3-closure.md` (Lemma G and Proposition G0)

*28 November 2026 (night of the 27th). Written as a skeptical referee of my own note. I tried to refute it. I edited nothing: `H3-closure.md`, `H3-borrowed-routes.md` and `CLOSED-SET.md` are as I found them.*

**Verdict: holds with fixes.** I re-derived every step of Lemma G (o)–(v) and Proposition G0 and found no mathematical error. I re-ran the check script and every table entry reproduced. The one load-bearing citation is real, and it says what the note claims. The fixes concern **how much the note claims it closes**. Three status sentences go further than the lemma does, and they are listed in §3.

---

## 1. Citations (checked tonight)

| Citation | Checked how | Result |
|---|---|---|
| S. Shlosman, "The critical temperature $T_{cr}$(Ising) is DS-computable", arXiv:2505.24750 | arXiv abstract page; full text of v2 extracted from the PDF | **Real.** Sole author Senya Shlosman. v1 dated 30 May 2025, v2 dated 3 Dec 2025. §1.1 says, word for word, "single spins $\sigma_x$ taking values in a finite set $S$", with $U_A$ translation-invariant and $U_A\equiv0$ for $\operatorname{diam}A>R$, and $\partial V=\{y\notin V:\operatorname{dist}(y,V)\le R\}$. The definition of $k_{V,y}$ (a max over $\sigma_y\ne\sigma'_y$ and $\sigma_{\partial V\setminus y}$ of $d_K/\rho$, with the sum metric $\rho_V$), Definition 1 ($\sum_y k_{V,y}<|V|$), Theorem 2 (unique Gibbs state and exponential decay), and the coupling definition of $d_K$ in §2.1 all match the note's quotations. |
| Dobrushin–Shlosman, "Constructive criterion for the uniqueness of Gibbs field", *Statistical Physics and Dynamical Systems: Rigorous Results*, Birkhäuser Boston 1985, pp. 347–370 | Shlosman's reference [DS]; web search for the volume | Shlosman's reference list gives exactly this title, publisher, city, year and page range. Search results confirm the editors as Fritz, Jaffe and Szász. One search hit cites the same authors' *next* chapter in that volume, "Completely analytical Gibbs fields", at p. 371. That fits a chapter ending at p. 370. I could not open the Springer chapter page because it redirected to a login. The note says honestly that it took the editors from an index entry, which is fine. |
| Bhattacharjee, Bhattacharya, Lockwood, Romanov, "One Cube Estimate Implies the Yang–Mills Mass Gap", hexagon:2610.00341 v7 | hexagonmath.org landing page (the link is in `H3-M2-infrared.md`, ref. 1) | **Real.** The title and the four authors match, v7 is dated 6 Nov 2026 in the version history, and the page says the paper "does not solve the Clay Millennium problem". The landing page does not show Definition 42 or Theorems 43 and 45, so I could not check their content. That matters only for the research note. The closure note uses this preprint only in §6.3 and §6.5, as a non-load-bearing mention. |

**Use of the citation.** The note borrows only the *definition* of $k_{V,y}$ and $C_V$ and applies it to real-valued spins, outside the finite-$S$ setting. It says this openly (§1, §6.1) and does not invoke Theorem 2. That is the correct and honest use. No borrowed theorem is applied outside its hypotheses.

---

## 2. Step-by-step check of the mathematics

- **Setting (§2).** Expanding $H_V(\phi_V\mid\tau)$ gives the quadratic form $\tfrac12\phi^{\mathsf T}(L_V+\operatorname{diag}\#\partial(x)+(ma)^2)\phi$, and $\deg_V(x)+\#\partial(x)=2d$, so $A=(2d+(ma)^2)I-\mathrm{Adj}_V$. The linear term is $-\phi^{\mathsf T}B\tau$. Correct. Each $y\in\partial V$ has exactly one coordinate in $\{0,\ell\}$ and exactly one neighbour in $V$. Correct for the $\ell^1$ boundary. The extra $\ell^\infty$ diagonal sites have $k=0$. Correct.
- **Step 1.** $A\succ0$: correct. $\mathrm{Adj}_V$ is nonnegative and symmetric, so its spectral radius is its top eigenvalue, which is $<D$. So the Neumann series converges and $A^{-1}\ge0$. The killed-walk reading of $H_V(x,y)$ is correct.
- **Steps 2–6, part (o).** The conditional law is $\mathcal N(H_V\tau,A^{-1})$ with covariance independent of $\tau$. The translation coupling gives the upper bound $|\epsilon|\sum_xH_V(x,y)$. Jensen gives the matching lower bound, coordinatewise in $\ell^1$. So $k_{V,y}=\sum_xH_V(x,y)$, independent of $\tau$. **Valid.**
- **(i).** $(A\mathbf 1)_x=\#\partial(x)=(B\mathbf 1)_x$ when $m=0$, so $u_V\equiv1$. **Valid.**
- **(ii).** Maximum principle, giving $u\le 2d/D<1$. **Valid.**
- **(iii), the exact formula.** I used $w=(ma)^2A^{-1}\mathbf 1$, the Kronecker-sum diagonalisation, and $\sum_j\sin(j\varphi)$. For odd $k$, $\sin(n\varphi/2)\sin((n+1)\varphi/2)=\sin^2(\pi k/2)\cos(\varphi/2)$, so the sum is $+\cot(\varphi/2)$. For even $k$ it is $0$. The substitution $ma=m\ell/(n+1)$ is right. **Valid.** The script's "Fourier" routine is an independent coding of this formula, and it agrees with the Gauss–Seidel solve.
- **(iii), the limit.** The bound $\cot\vartheta\le1/\vartheta$ on $(0,\pi/2]$ gives the dominating sequence $16/(\pi^2k_i^2)$, which sums to $2^d$. Tannery applies. $\sum_{k\text{ odd}}8/(\pi^2k^2)=1$, so $0<\alpha<1$. **Valid.** The identification with the weak Dirichlet solution uses completeness of the Dirichlet sine basis on the cube. That is standard and not reproved here, and nothing in (v) depends on it. The remark in §0 that $u$ is not $C^2$ up to the corners is correct: at a corner $u_{xx}=u_{yy}=0$ along the edges, but $\Delta u=m^2\ne0$.
- **(iv).** I checked $v_i(x\pm ae_i)$: their sum is $2\cosh(\mu a)\,v_i$, and $2\cosh(\mu a)+2(d-1)=D$. So $v=\sum_iv_i$ is exactly killed-harmonic on all of $a\mathbb Z^d$, and $v\ge1$ on $\partial V$. Comparison through $A^{-1}B\ge0$ follows. $\sum_{j=1}^n\cosh(\theta(j-c))=\sinh(n\theta/2)/\sinh(\theta/2)$ and $\sinh(\theta/2)=ma/2$, both correct. The chain down to $2d/(m(\ell-a))$ is correct. Equality in $d=1$ holds by uniqueness. **Valid.**
- **(v).** (iii) handles the tail, (ii) handles the finitely many remaining $n$, and the explicit constant $4d/(m\ell)$ from $a\le\ell/2$ is correct. **Valid.**
- **Proposition G0.** I telescope over boundary sites using the sup in the definition of $k$, and get a lower bound $|V||c|$ from the mean shift. **Valid**, under the stated hypotheses: probability laws for every $\tau$, finite first moments, exact translation covariance. A direction-by-direction version, with a separate shift $c_\mu$ per link direction, goes through the same way. That supports the Maxwell remark *if* the gauge-fixed box law is normalizable, which the note correctly leaves as [guess].
- **§6.1, non-uniqueness for $m>0$.** $h=e^{\mu x_1}$ is killed-harmonic, and $H_Vh|_{\partial V}=h_V$, so the shifted field has the same specification. **Correct**, and it is the right warning: $C_V$ holds while uniqueness fails among all Gibbs measures. That shows DS Theorem 2 genuinely does not extend verbatim to unbounded spins.
- **Numerics.** I re-ran `H3-closure-check.py` at low priority. All seven rows and all three continuum values reproduce to the printed digits. The $d=4$, $m\ell=4$ truncations $k\le39$ and $k\le59$ differ by $9.2\times10^{-6}$, as claimed.

I found **no fatal issue**.

---

## 3. Where the note claims more than it proves (fixable)

**F1. "Closes the normalization part of my old Sub-conjecture 5.1" is too strong.** Sub-conjecture 5.1 (`H3-M2-infrared.md`, line 156) has three ingredients:
- it is about **$\rho$-smoothed observables of the central block**;
- it concerns SU(2) **at weak bare coupling**;
- it predicts suppression **by a power of $a/\rho$**.

Lemma G is about a different quantity in a different model: the sum-metric influence on **all** sites, averaged over the box, for a **scalar Gaussian**. What it actually shows is that **after** the reformulation MS → MS′, the boundary-count worry that *motivated* 5.1 does not arise in the Gaussian template. It neither proves nor refutes 5.1 as stated. Two facts show the gap concretely:
- In the template, the per-site influence $k_{V,y}$ is of order $\ell/a$, so it **grows** as $a\to0$. It is not suppressed.
- Only the averaged quantity $k_{V,y}/|V|\sim(a/\ell)^{d-1}$ has 5.1's "power of $a/\rho$" shape. Even then it exactly cancels the boundary count and gives $O(1)$, not something small.

Suggested wording: "removes, for the Gaussian template, the boundary-count obstruction that motivated Sub-conjecture 5.1, and so supports replacing MS by MS′".

**F2. The worst-case clause is vacuous in the template, and the note should say so.** In Lemma G, $k_{V,y}$ does not depend on $\tau$. So the sup over boundary data, which is exactly where H3's failure mode 3 sits (large or rough fields at scale $\rho$; `H3-M2-infrared.md`, line 167), is never tested. The Gaussian calibrates the *normalization* only under linear, data-independent influence. This belongs in §6 next to item 2.

**F3. "(5-ver) is irreducibly a non-perturbative mass statement" goes beyond G0.** G0 excludes only specifications with *exact* translation covariance. Here is what is proved:
- (a) In the Gaussian template, $C_V$ holds iff $m>0$.
- (b) Every exactly translation-covariant real-spin specification has $\alpha_V\ge1$ in this metric.

Here is what is not proved:
- That corrections beyond the linearization, from compactness or the cubic and quartic terms, could not push $\alpha$ below 1 in some controlled expansion.
- The research note's stronger word "equivalent in content to non-perturbative mass generation" (`H3-borrowed-routes.md` §4 Step 3). Only one direction is available, and even that only through a DS-type implication theorem that is not established for compact gauge fields. The converse, mass ⇒ $C_V$ at some scale, is proved only for the Gaussian, and for Ising by Shlosman's Theorem 4.

The honest sentence is: "the leading massless linearization sits *exactly* on $\alpha=1$, so any verification must come from effects that break the constant-shift symmetry, such as a mass, compactness or curvature".

**F4. "The property of any massless linearized approximation" is too broad.** That holds for a flat-space linearization around a constant background in a shift-preserving gauge. A linearization on a curved room is different. On $S^3_R$ the linearized Yang–Mills operator has a curvature-induced lower bound of order $1/R$, which acts like a mass of order $1/R$. That gives $\alpha<1$ at every finite $R$ but $\alpha\to1$ as $R\to\infty$ at fixed $\ell$, so the bound is not uniform. This supports the note's point but should be stated, because H3 lives on the room. [My claim about the $1/R$ curvature term is standard Hodge–Weitzenböck reasoning. I did not re-derive a constant here.]

**F5. Hypotheses of the G0 example.** "Any nearest-neighbour gradient model $\sum V(\phi_x-\phi_z)$, Gaussian or not" needs normalizability and finite first moments of the box law for every $\tau$, for instance $V$ with superlinear growth. Say this explicitly.

**F6. Minor.** In the remark after G0, "these differ whenever $\mu$ has a first moment" can drop the qualifier. A probability measure on $\mathbb R$ cannot be invariant under a nonzero translation, so the translates always differ. The remark is also true vacuously when no Gibbs measure exists, for example the massless field in $d\le2$.

---

## 4. A calibration the note could add (small computation, [C])

From the continuum series in $d=4$, truncated at $k\le39$ and only approximate at large $m\ell$: $\alpha(8)\approx0.629$, $\alpha(16)\approx0.395$, $\alpha(32)\approx0.223$. The bound (iv) limit $\frac{8}{m\ell}\tanh\frac{m\ell}2$ gives about 1.00, 0.50 and 0.25. So $\alpha\le\tfrac12$ needs a box roughly 11 to 12 Gaussian correlation lengths across. The BBLR rate $\log(1/\alpha)/(2\ell)$ is then a few percent of $m$. In the template, then, the one box that verifies MS′ is many correlation lengths across. That is a deep-infrared, strongly coupled scale for Yang–Mills, which matches the note's conclusion that (5-ver) is the hard part.

---

## 5. Honest status

- **Lemma G (o)–(v) and Proposition G0: [P], earned.** The proofs are elementary, complete, and checked numerically two ways. Only the DS *definition* is borrowed, and it is used openly outside its finite-$S$ setting.
- **Effect on H3:** a calibration result for a Gaussian scalar template, plus a sharp negative for exactly shift-symmetric specifications. It supports switching from MS to MS′. It does **not** close any part of Sub-conjecture 5.1 as stated, and it does not establish that (5-ver) is "equivalent to" or "irreducibly" a non-perturbative mass statement.
- **H3 (line 20): [X], unchanged.** The note says the same.

### Sources consulted

- arXiv abstract page and PDF of 2505.24750: https://arxiv.org/abs/2505.24750
- Springer chapter record for the 1985 volume (search-indexed only): https://rd.springer.com/chapter/10.1007/978-1-4899-6653-7_21
- BBLR landing page: https://hexagonmath.org/2610.00341v7
