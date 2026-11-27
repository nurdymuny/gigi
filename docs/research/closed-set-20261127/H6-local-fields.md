# H6 (line 25): local fields, the trace anomaly, and agreement with perturbation theory

*Status: hypothesis [X]. Nothing in this section is proved by me unless it is marked so. Line 26 is deduced from H6 here. That deduction is the only theorem in this section, and it is conditional.*

Conventions. $G=SU(N)$ (for line 24, H7, replace $N^2-1$ by $\dim G$ and $N$ by the dual Coxeter number $h^\vee$). I use canonical normalization $A\to g A$, $F^a_{\mu\nu}$ real, Euclidean action $\tfrac14\int F^a_{\mu\nu}F^a_{\mu\nu}$, and
$$O(x):=F^a_{\mu\nu}F^a_{\mu\nu}(x)\qquad(\text{"}\operatorname{tr}F^2\text{" on the page, up to a fixed positive constant}).$$
The beta function is $\beta(g)=\mu\,\tfrac{dg}{d\mu}=-\tfrac{b_0}{16\pi^2}g^3-\tfrac{b_1}{(16\pi^2)^2}g^5-\dots$, with $b_0=\tfrac{11N}{3}$ ($=22/3$ for $SU(2)$, the page's value) and $b_1=\tfrac{34N^2}{3}$. At one loop $1/g^2(\mu)=\tfrac{b_0}{8\pi^2}\ln(\mu/\Lambda)$, which is the page's $R\Lambda=\exp(-8\pi^2/(b_0g^2(R)))$. On the lattice, $\beta_0=4/g_0^2$ for $SU(2)$, $s_p=1-\tfrac12\operatorname{tr}U_p$, and $S_W=\sum_p s_p$.

---

## 1. Statement (high fidelity)

**Inputs from other hypotheses.** Let $\{S_n\}$ be the limit theory of H4 (line 23), reached along the line of constant physics $a\mapsto\beta_0(a)$ with $\Lambda$ held fixed (H3, H5). It satisfies the OS axioms and reconstructs to $(\mathcal H,\Omega,H\ge0,U(g))$ with $g$ in the Euclidean or Poincaré group. "Limit" means the order of limits fixed in H4: $a\to0$ at fixed box $L$ (or at fixed sphere radius $R$ under H5), then $L\to\infty$.

**The field space.** $\mathcal P$ is the real vector space of gauge-invariant local polynomials in $F_{\mu\nu}$ and its covariant derivatives $D_{\alpha_1}\cdots D_{\alpha_k}F_{\mu\nu}$. It is graded by canonical dimension $d$ (each $F$ counts 2, each $D$ counts 1). Write $\mathcal P_{\le d}$ for the finite-dimensional subspace of dimension $\le d$, split by $O(4)$ representation, parity and charge conjugation. For each basis element $P$ fix a *lattice transcription* $P_a(x)$. This is a gauge-invariant function of the links in a bounded lattice neighbourhood of $x$ (built from clover plaquettes and covariant differences) with $P_a(x)=a^{d}P(x)+O(a^{d+1})$ on smooth configurations $U_{x,\mu}=e^{agA_\mu}$. The basic case is $\sum_{\text{6 plaquettes at }x}s_p=\tfrac{a^4g_0^2}{16}O(x)+O(a^5)$ for $SU(2)$, which I checked from $U_p=e^{a^2 g F}$ and $\operatorname{tr}T_aT_b=-\tfrac12\delta_{ab}$.

**H6.** *There are renormalization matrices $Z^{(d)}(a)$ (one block per symmetry type), depending only on $a$ and $\beta_0(a)$ and not on $L$, $n$, the points or the test functions, with $Z^{(d)}$ mapping $\mathcal P_{\le d}$ to itself and mixing $P$ only with operators of equal or lower dimension (the identity included), such that the following hold.*

**(a) Existence.** For every $n$, all $P_1,\dots,P_n\in\mathcal P$ and every $f\in\mathcal S_{\neq}(\mathbb R^{4n})$ (Schwartz functions vanishing with all derivatives on every diagonal $x_i=x_j$),
$$S_n^{P_1\cdots P_n}(f)=\lim\Big\langle \prod_{i}\textstyle\sum_j Z_{ij}(a)\,a^{-d_j}P_{j,a}\Big\rangle_{\beta_0(a),L}(f)$$
exists in the H4 order of limits. Jointly with the Wilson-loop Schwinger functions it satisfies OS0–OS4, including the linear growth condition OS0′. Each $S_n^{P_1\cdots P_n}$ extends from $\mathcal S_{\neq}$ to a tempered distribution on $\mathcal S(\mathbb R^{4n})$, and this extension is unique up to contact terms (distributions supported on diagonals) of bounded order. By OS reconstruction each $P$ then gives an operator-valued tempered distribution $\phi_P(f)$, $f\in\mathcal S(\mathbb R^4)$, on a common dense invariant domain $D\ni\Omega$, transforming covariantly and local in the Wightman sense. Fields of dimension $d$ are defined up to a finite renormalization $\phi\mapsto\sum_{d'\le d}c\,\Lambda^{d-d'}\phi'$ (scheme freedom). The vacuum $\phi_{\mathbf 1}=1$ is the only field of dimension 0.

**(b) Stress tensor and trace anomaly.** In $\mathcal P_{\le4}$ the symmetric tensor $F^a_{\mu\rho}F^a_{\nu\rho}-\tfrac14\delta_{\mu\nu}O$ can be renormalized, including its mixing with $\delta_{\mu\nu}O$ and with $\mathbf 1$, to a field $T_{\mu\nu}$ with three properties.
- (b1) It is conserved: $\partial_\mu T_{\mu\nu}=0$ away from other insertions.
- (b2) It generates translations: Ward identities hold with contact terms $\delta(x-x_i)\partial_\nu$.
- (b3) Its trace is
$$\Theta:=T_{\mu\mu}=\frac{\beta(g)}{2g}\,[O]_\mu,$$
where $[O]_\mu$ is $O$ renormalized at scale $\mu$ with $\langle[O]_\mu\rangle$ subtracted. $\Theta$ does not depend on $\mu$: it is RG invariant and has no anomalous dimension. The overall sign depends on the Euclidean convention for $T$. On the lattice, $\Theta$ is the limit of
$$\Theta_a(x)=-\frac{d\beta_0}{d\ln a}\;a^{-4}\sum_{p\ni x}\big(s_p-\langle s_p\rangle\big),\qquad -\frac{d\beta_0}{d\ln a}=\frac{b_0}{2\pi^2}+O(\beta_0^{-1}),$$
**with no further multiplicative renormalization.** This is the precise meaning of line 25's "plaquette, renormalized by the running coupling". As a check of factors: $\tfrac{b_0}{2\pi^2}\cdot\tfrac{g_0^2}{16}O=\tfrac{b_0g_0^2}{32\pi^2}O$, which is the one-loop value of $-\tfrac{\beta(g)}{2g}O$.

**(c) Agreement to all orders (asymptotic).** For each $\mu$-independent choice of renormalized fields, let $\bar g(\lambda)$ be the running coupling at scale $1/\lambda$. Any $K$-loop solution normalized by the limit theory's $\Lambda$ will do. The statement is invariant under these choices, because they differ at higher order and re-expansion absorbs the difference. For every $n$, all $P_i$ and every configuration $x=(x_1,\dots,x_n)$ of distinct points, there are coefficients $F^{(k)}_{P_1\cdots P_n}(x)$, computed in renormalized perturbation theory (any BRST-invariant scheme, matched to the lattice scheme), such that for every $K$
$$\Big|\,\lambda^{\sum d_i}\,\mathcal Z(\lambda)^{-1}S_n^{P_1\cdots P_n}(\lambda x)-\sum_{k=k_0}^{K}\bar g(\lambda)^{2k}F^{(k)}(x)\Big|\le C_K(x)\,\bar g(\lambda)^{2K+2}\qquad(0<\lambda\le\lambda_K(x)).$$
Here $\mathcal Z(\lambda)=\prod_i\exp\!\big(-\int_{\bar g(1)}^{\bar g(\lambda)}\gamma_{P_i}(g')/\beta(g')\,dg'\big)$ carries the anomalous dimensions (diagonalized within each mixing block). For RG-invariant fields such as $\Theta$, $\mathcal Z\equiv1$. The constants $C_K$ are not claimed summable: the expansion is asymptotic, not convergent.

*Reading I chose.* "Agree to all orders" means (c): an asymptotic expansion in the running coupling at short distance with fixed geometry $x$, uniform on compact sets of distinct configurations. I do not take it to mean convergence, or agreement at fixed $g$. Line 26 needs only $n=2,3$ and $K=k_0$ for $\Theta$.

---

## 2. Interface

**What other lines take from H6.**
- **Line 26** (closed from 25) uses (b3) and (c) for $\langle\Theta\Theta\rangle$ at leading order, and also needs $\langle\Theta\Theta\Theta\rangle_T$ at leading order. That second need is stronger than line 26's wording; see §4, step 6.
- **Line 28** needs a dense set of vectors $A\Omega$. The polynomial algebra of smeared $\phi_P(f)$ and Wilson loops applied to $\Omega$ is dense in $\mathcal H$ by construction of the OS Hilbert space. H6(a) supplies the local part of that set.
- **Line 24** (H7): the same statement with $b_0=\tfrac{11}{3}h^\vee$.

**What H6 takes from other lines.**
- **Line 1** gives the lattice transcriptions.
- **Line 23** (H4): H4 must hold for the renormalized plaquette fields, not only for Wilson loops. *Flagged requirement: H4 as worded must include the observables $Z(a)a^{-d}P_a$. Otherwise H6(a) is an extension of H4, not a consequence of it.*
- **Lines 20 and 29** (H3, H5) fix $\Lambda$ and the line of constant physics $\beta_0(a)$. Line 29 makes the sphere-regulated fields agree with the lattice ones up to $\Lambda$ and a finite renormalization.
- **Line 21** (Balaban) gives finite-volume UV control.
- **Line 14** [C] is my own one-loop check of the $O$ renormalization in the sphere regulator. The coefficient $\tfrac{b_0}{8\pi^2}\log(\Lambda_{\rm cut}R)$ of $S_{\rm cl}$ is exactly the logarithmic running whose $\Lambda_{\rm cut}$-derivative is the one-loop trace anomaly. It was matched to $10^{-4}$ with gauge-invariant heat-trace regularization.

---

## 3. What is known

**Proved (perturbative, to all orders).**
- YM is perturbatively renormalizable: 't Hooft 1971; 't Hooft and Veltman 1972; Lee and Zinn-Justin 1972. BRST symmetry: Becchi, Rouet and Stora 1976; Tyutin 1975. A rigorous flow-equation proof: Efremov, Guida and Kopper 2017.
- Asymptotic freedom: Gross and Wilczek 1973 (Phys. Rev. Lett.); Politzer 1973 (Phys. Rev. Lett.). The two-loop coefficient: Caswell 1974; Jones 1974.
- Renormalization of gauge-invariant composite operators: Kluberg-Stern and Zuber 1975; Joglekar and Lee 1976. Gauge-invariant operators mix with BRST-exact and equation-of-motion operators, but these drop out of gauge-invariant correlators at non-coincident points.
- Trace anomaly $\theta^\mu{}_\mu=\tfrac{\beta}{2g}[F^2]$: Adler, Collins and Duncan 1977; Collins, Duncan and Joglekar 1977; Nielsen 1977.
- Callan–Symanzik equations: Callan 1970; Symanzik 1970.
- Lattice perturbation theory converges to continuum renormalized perturbation theory order by order: Reisz 1988 (Commun. Math. Phys.), via a lattice power-counting theorem.
- Lattice energy–momentum tensor and its hypercubic mixing: Caracciolo, Curci, Menotti and Pelissetto 1990.
- Gradient-flowed fields are finite to all orders without renormalization: Lüscher 2010; Lüscher and Weisz 2011. Stress tensor from the small-flow-time expansion: Suzuki 2013.

So the coefficients $F^{(k)}$ in (c), and the matching between lattice and continuum, are theorems in perturbation theory.

**Proved (non-perturbative, related).**
- 4D: Balaban's UV stability for lattice YM in finite volume (Commun. Math. Phys., 1980s series), and Magnen, Rivasseau and Sénéor 1993 (line 21). Neither constructs the correlations of local $\operatorname{tr}F^2$ fields in a continuum limit.
- The closest complete analogue is an asymptotically free theory with local fields and short-distance agreement with perturbation theory, all proved: the 2D Gross–Neveu model (Gawedzki and Kupiainen 1985; Feldman, Magnen, Rivasseau and Sénéor 1986).
- 2D YM is solvable (line 6).

**Not proved (expected).** Every part of H6 for 4D non-abelian YM in a continuum limit.

---

## 4. Best-guess proof

The route defines the fields through the YM gradient flow, then removes the flow. The flow is the energy gradient of the Wilson action, so it is the variational smoothing my programme already insists on.

**Step 1 (flowed fields are finite) [guess; perturbative version proved].** Let $B_t$ be the lattice Wilson flow of $U$ at flow time $t>0$, and let $E_t(x)=\tfrac14 G^a_{\mu\nu}G^a_{\mu\nu}(t,x)$. For fixed $t>0$ and $f\in\mathcal S_{\neq}$ (with separations $\gg\sqrt t$), correlators of $E_t$ converge as $a\to0$ with no $Z$ at all.

*Evidence.* Lüscher and Weisz prove this to all orders. Non-perturbatively, the flow is a smoothing map at scale $\sqrt t$, so the claim reduces to H4 for observables that depend only on the configuration coarse-grained at scale $\sqrt t$. Balaban's block-averaged effective actions are exactly such objects.

**Step 2 (small-flow-time OPE; this is the key estimate) [guess].** For $f$ whose support lies at distance $\ge\delta$ from the other insertions $X$,
$$\Big|\Big\langle \tfrac{E_t(f)-\langle E_t\rangle}{c_O(t)}\,X\Big\rangle_T-\Big\langle \tfrac{E_s(f)-\langle E_s\rangle}{c_O(s)}\,X\Big\rangle_T\Big|\le C(f,X)\,\frac{\max(t,s)}{\delta^2}\,\big(1+|\log(\delta^2\Lambda^2)|\big)^{m},$$
where $c_O(t)=c_0\,\bar g(\sqrt t)^{2}\,[1+O(\bar g^2)]\cdot\mathcal Z_O(t)$ is the perturbative matching coefficient. Then the definition
$$[O](f):=\lim_{t\to0}\frac{E_t(f)-\langle E_t\rangle}{c_O(t)}$$
is Cauchy in $t$. The bound expresses that dimension-6 operators enter with $t^{1}$. The justification would be a multiscale argument: between the scales $\sqrt t$ and $\delta$ the effective coupling is $\le\bar g(\sqrt t)$, which is small, and the Balaban-type remainders are bounded by powers of it.

**Step 3 (stress tensor) [guess].** Define $T_{\mu\nu}$ as the $t\to0$ limit of Suzuki's combination of $E_t$ and $G_{\mu\rho}G_{\nu\rho}$. Conservation (b1) and the Ward identity (b2) are inherited from the translation invariance of the lattice in the limit, given H4's recovery of Euclidean symmetry. Hypercubic symmetry is enough for $\partial_\mu T_{\mu\nu}$ to vanish once the two lattice irreps of $T_{\mu\nu}$ have been renormalized to a common normalization.

**Step 4 (trace anomaly from an exact lattice identity) [identity proved; limit guessed].** On the finite lattice
$$\partial_{\beta_0}\langle X\rangle=-\langle X;S_W\rangle\qquad\text{(exact)}.$$
Along the line of constant physics, $a\Lambda=F(\beta_0)$, so at fixed $a$ we have $\Lambda\partial_\Lambda=(d\ln F/d\beta_0)^{-1}\partial_{\beta_0}$. At one loop $d\ln F/d\beta_0=-2\pi^2/b_0$. In the limit $\Lambda$ is the only scale, so for RG-invariant fields of dimensions $d_i$,
$$\Big(\sum_i x_i\!\cdot\!\partial_i+\sum_i d_i\Big)S_n=\Lambda\partial_\Lambda S_n=-\Big(\frac{d\beta_0}{d\ln a}\Big)\big\langle X;S_W\big\rangle=\int d^4y\,\langle\Theta_a(y)\,X\rangle_T .$$
This identifies $\Theta_a$ as the lattice density of the dilatation anomaly, with the running-coupling factor and nothing else. The guess is that the local field $\Theta_a(x)$ converges and equals $\tfrac{\beta}{2g}[O]$ from Step 2. Two facts support it. Both are RG-invariant dimension-4 scalars. And by (a) the space of such scalars modulo $\mathbf 1$ is one-dimensional, so they agree up to a constant, which the integrated identity fixes.

**Step 5 (asymptotics; the order of limits that matters).** RG invariance gives $S_n(\lambda x;\Lambda)=\lambda^{-\sum d}\mathcal Z\,S_n(x;\lambda\Lambda)$. Short distance is therefore weak coupling at the fixed scale $1/|x|$. A multiscale expansion splits $S_n(x;\Lambda)$ into two parts:
- scales finer than $|x|$, which reproduce renormalized perturbation theory in $\bar g(|x|)$ with remainder $C_K\bar g^{2K+2}$ [guess, the core estimate; proved for Gross–Neveu in 2D];
- scales coarser than $|x|$, which contribute at most $C\,(|x|\Lambda)^{p}$ for some $p>0$ (condensates such as $\langle O\rangle\Lambda^{-4}$ enter as $(|x|\Lambda)^4$) [guess].

Because $(|x|\Lambda)^p=\exp(-8\pi^2p/(b_0\bar g^2))$ at one loop, the second part is $o(\bar g^{2K+2})$ for every $K$. That gives (c), and it explains why (c) can only be asymptotic. The order of limits is: $a\to0$, then $L\to\infty$, then $\lambda\to0$. Line 29 (H5) is what allows the sphere at radius $R\sim|x|$ to be used for the first part. That is the regime where the small room and the M3 dressing (lines 13–15) are perturbative computations.

**Step 6 (deduction of line 26 from H6; conditional theorem).** Let $G_\Theta(x)=\langle\Theta(x)\Theta(0)\rangle$ for $x\ne0$. In the free theory, $|x|^8\langle O(x)O(0)\rangle=C_O(N^2-1)$, where $C_O>0$ is a pure number: a sum of squared free propagator terms, and positive by the Källén–Lehmann positivity of the free spectral density. By (b3) and (c) with $k_0=2$ and $\mathcal Z\equiv1$,
$$|x|^8G_\Theta(x)=C_O(N^2-1)\Big(\frac{b_0\,\bar g^2(|x|)}{32\pi^2}\Big)^2\big[1+O(\bar g^2)\big]=\frac{C_O(N^2-1)}{16\,\ln^2\!\big(1/(|x|\Lambda)\big)}\big(1+o(1)\big),\qquad |x|\to0,$$
using $\bar g^2=8\pi^2/(b_0\ln(1/|x|\Lambda))$ at leading order. Note that $b_0$ cancels at leading order.

*Lemma (free theories have no such term).* Consider any theory of finitely many free fields, massive or massless, whose local fields are Wick polynomials. Every free propagator has the form $|x|^{-2s}\big[A(m^2x^2)+m^2x^2\ln(m^2x^2)B(m^2x^2)\big]$ with $A$ and $B$ entire. So the two-point function of any local field is a finite sum $\sum c_{k,j}|x|^{-2k}(m|x|)^{2l}\ln^j(m|x|)$, in which every logarithm carries a positive power of $m|x|$. Hence $|x|^{2k_{\max}}G\to c\ne0$. It can never tend to $0$ like $1/\ln^2$.

*Gap in line 26 as worded.* A **generalized free field** can have any positive two-point function, $1/\ln^2$ included. Excluding it needs a nonzero truncated higher function. H6(c) gives
$$\langle\Theta\Theta\Theta\rangle_T=\Big(\frac{b_0\bar g^2}{32\pi^2}\Big)^3\,T_O(x)\,[1+O(\bar g^2)],$$
where $T_O$ is the free-field Wick "triangle" of three $O$'s. I expect $T_O\ne0$, but I have not computed it here: [guess, a finite free-field computation]. Together the two facts give the corrected criterion in a form that excludes both kinds of free theory. **Theorem (conditional on H6 and $T_O\not\equiv0$):** the limit theory is neither a Wick-polynomial free theory nor a generalized free field in its $\Theta$ sector. So it is non-trivial.

---

## 5. Where it can fail

1. **Power-divergent mixing.** For $d\ge6$ the coefficients mixing with lower dimensions grow like $a^{-(d-d')}$. Perturbation theory fixes them only up to renormalon-sized ambiguities of order $\Lambda^{d-d'}$. If no non-perturbative normalization condition (flow, or conditions at a reference scale) removes them, the limits in (a) depend on the prescription. *Observed:* smeared dimension-6 correlators fail to converge, or shift by $O(\Lambda^2)$ between prescriptions.
2. **The OPE remainder in Step 2 is not uniformly small.** Large-field regions at small scales could spoil it, for example a small-instanton density not suppressed like $\rho^{b_0-5}$, or Gribov-type obstructions to the multiscale gauge fixing. *Observed:* corrections to $c_O(t)^{-1}E_t$ that are not $O(t)$.
3. **Wrong line of constant physics.** If H3/H4 converge to a trivial or different fixed point, (b3) fails at the root. *Observed:* $|x|^8G_\Theta$ tends to a constant or behaves like a power, instead of $1/\ln^2$.
4. **Temperedness.** If some coincident-point singularities grew faster than polynomially, the extension from $\mathcal S_{\neq}$ would fail, and so would OS0′. This is not expected anywhere in perturbation theory.
5. **Line 26's GFF loophole** stays open if $T_O\equiv0$. In that case the first nonvanishing truncated function needed would be the 4-point one.

---

## 6. Smallest next step

**Uniform two-point bound for the running-coupling plaquette in finite volume.** On $T^4_L$, assuming only Balaban's finite-volume UV control (line 21), prove:

For all $f,h\in C_c^\infty(T^4_L)$ with $\operatorname{dist}(\operatorname{supp}f,\operatorname{supp}h)\ge\delta>0$,
$$\sup_{0<a\le a_0}\big|\langle\Theta_a(f)\,\Theta_a(h)\rangle_{\beta_0(a),L}\big|\le C(f,h,\delta,L)<\infty,\qquad \Theta_a(x)=-\frac{d\beta_0}{d\ln a}\,a^{-4}\sum_{p\ni x}(s_p-\langle s_p\rangle).$$

This gives tightness of the one field that matters most. Through the exact identity of Step 4 it is the first non-perturbative content of the trace anomaly, and it is the quantity whose short-distance form decides line 26.

---

## Flag

Jaffe and Witten's official description says, as I recall it and paraphrase here, that the predictions of asymptotic freedom and perturbative renormalization "include among other things the existence of a stress tensor and an operator product expansion, having prescribed local singularities". The stress tensor is a gauge-invariant polynomial in $F$, so it is already inside H6, and I made it explicit in (b). The **operator product expansion** is not named on line 25. Clause (c) with nested coincidences would carry it. I record it here and do not add it to the statement. I also flag (§2) that H4 must cover the renormalized plaquette observables for H6(a) to follow from it.
