# H6 closure note: the integrated trace identity, conditional on H4(a)

28 November 2026. Proof note. It changes no status in `CLOSED-SET.md`, and H6 (line 25) stays [X].
If adopted, it adds one conditional sub-statement under H6(b3):
**[D, conditional on (L1) and (L2)]** (Corollary 3 also needs (A) and (B)).

Source of the route: `H6-borrowed-routes.md` §5. Here I write out every step, including the step that note only
sketched (the β-derivative at infinite time extent).

---

## 0. Verdict in one paragraph

The route works, with one correction and three changes of scope.

- **Correction (sign).** The research note defines the prefactor by "$-d\beta_0/d\ln a := s\,d\beta_n/ds$". That has the
  wrong sign. Along a line of constant physics, $s\,d\beta_n/ds = +\,d\beta_0/d\ln a$ (§5). With the note's definition,
  the identity $(\ast)$ holds with the opposite sign on the right-hand side. With the corrected definition
  $-d\beta_0/d\ln a := -s\,d\beta_n/ds$, which is positive under asymptotic freedom and agrees with the register's
  $b_0/(2\pi^2)+\dots$, the identity holds exactly as written.
- **Scope 1.** I prove it for the observables $\mathcal O_{\rm Lip}$, where the function $\Phi$ is Lipschitz on the
  cube. This class is dense in H4's class $\mathcal O$ in sup norm, uniformly in $n$. For a merely continuous $\Phi$ I
  cannot show that the plaquette sum on the left converges absolutely at fixed $n$.
- **Scope 2.** The identity $(\ast)$ does not use the scale condition at all. It holds for **any** $C^1$ path of bare
  couplings. The scale condition only names the prefactor ($-d\beta_0/d\ln a$) and gives the dilatation corollary.
- **Scope 3.** I replace the sketched transfer-matrix-gap argument with a Doeblin (minorization) argument. It is
  elementary and explicitly uniform in β, and it needs no Perron–Frobenius theory.

Every step below is proved here, or is a definition, or is a textbook theorem of analysis (dominated convergence,
Kolmogorov extension, existence of ODE flows on compact manifolds). No physics theorem enters the proof.

---

## 1. Setting

Everything is H4's setting (`closed-set-20261127/H4-continuum-limit.md` §1), at a fixed box $L$.

- **Lattice.** $G=SU(2)$. $a_n=2^{-n}a_0$, $L\in a_0\mathbb N$, $N:=L/a_n$, and
  $\Lambda_{n,L}=(a_n\mathbb Z/L\mathbb Z)^3\times a_n\mathbb Z$: periodic space, infinite time.
- **Action.** $s_p=1-\tfrac12\operatorname{tr}U_p\in[0,2]$, summed over unoriented plaquettes (register line 1), at
  coupling $\beta=4/g_0^2\ge0$.
- **Slices.** For each time slice $k\in\mathbb Z$ let $z_k\in\Sigma:=SU(2)^{3N^3}\times SU(2)^{N^3}$ collect the
  spatial links at time $k$ and the temporal links from $k$ to $k+1$. $\nu$ is normalized Haar measure on $\Sigma$.
  The configuration space is $\Omega=\Sigma^{\mathbb Z}$, compact metrizable in the product topology.
- **Plaquette assignment.** Assign each plaquette to its lowest time slice $j$. A plaquette assigned to $j$ depends
  only on $(z_j,z_{j+1})$. Write
  $$w(z,z')=\sum_{p\text{ assigned to the slice of }z}s_p,\qquad 0\le w\le 2P,\qquad P:=6N^3,$$
  where $P$ counts the $3N^3$ spatial plaquettes at a slice plus the $3N^3$ temporal plaquettes to the next slice. Each
  plaquette appears in exactly one $w(z_k,z_{k+1})$. Put $K_\beta(z,z')=e^{-\beta w(z,z')}$, so that
  $$e^{-2\beta P}\le K_\beta\le1 .$$
- **Finite slabs.** $\mu_M$ is the probability measure on $\Sigma^{[-M,M]}$ with density
  $\propto\prod_{k=-M}^{M-1}K_\beta(z_k,z_{k+1})$ with respect to $\nu^{\otimes(2M+1)}$. This is the Wilson measure of
  the time slab with plaquette set $\mathcal P_M$ (the plaquettes assigned to slices $-M,\dots,M-1$) and open time
  boundary. The temporal links in $z_M$ appear in no plaquette, so they are independent and Haar distributed.
- **Flow.** I use Lüscher's lattice Wilson flow [L10, eq. (1.4)], with flow time in lattice units $\tau=t/a_n^2$.
  Its generator is $g_0^2S_w=\sum_{\text{oriented }p}\operatorname{Re}\operatorname{tr}(1-U(p))=4\sum_{p}s_p$, so the
  flow does not depend on β.
- **Observables, $\mathcal O_{\rm Lip}$.** The generators are
  $F_{\gamma,t}=\Phi\big(\tfrac12\operatorname{tr}\operatorname{hol}_{V_t}(\gamma_1),\dots,\tfrac12\operatorname{tr}\operatorname{hol}_{V_t}(\gamma_m)\big)$,
  with $\Phi$ Lipschitz on $[-1,1]^m$ and with H4's lattice approximations $\gamma_i^{(n)}$ of the loops. The class
  also contains their finite lattice-translate sums (H4's smeared versions, as Riemann sums), and is closed under sums
  and products. Every $F\in\mathcal O_{\rm Lip}$ obeys $\sup_n\sup_U|F_n(U)|=:C_F<\infty$. The infinite-time meaning
  of $F_n$ is fixed in Lemma 5.

**Density.** Lipschitz functions are dense in $C([-1,1]^m)$, and $|F_{\Phi}-F_{\Phi'}|\le\|\Phi-\Phi'\|_\infty$ for
every $n$ and every configuration. So $\mathcal O_{\rm Lip}$ is sup-norm dense in $\mathcal O$, uniformly in $n$.

---

## 2. Statements

**Theorem 1 (β-derivative at fixed lattice, infinite time).** Fix $n$, $L$ and $F\in\mathcal O_{\rm Lip}$. The
infinite-time Wilson measure $\mu_{n,L,\beta}$ exists for every $\beta\ge0$ (Lemma 4). The map
$\beta\mapsto\langle F\rangle_{n,L,\beta}$ is $C^1$ on $[0,\infty)$, and
$$\partial_\beta\langle F\rangle_{n,L,\beta}=-\sum_{p\in\Lambda_{n,L}}\langle F;s_p\rangle_{n,L,\beta},$$
where the sum over all plaquettes of the infinite-time lattice converges absolutely, uniformly on compact β-sets.

**Theorem 2 (integrated trace identity).** Let $J\ni1$ be an open interval. Assume:

- **(L1)** For each $n$, $\beta_n:J\to[0,\infty)$ is $C^1$.
- **(L2)** For each $s\in J$ and each $F\in\mathcal O_{\rm Lip}$, the limit $S_s(F):=\lim_{n\to\infty}\langle F\rangle_{n,L,\beta_n(s)}$ exists.

Define, for $y\in\Lambda_{n,L}$ and $s\in J$,
$$\Theta_n^{(s)}(y):=c_n(s)\,a_n^{-4}\sum_{p\in\mathcal P_y}\big(s_p-\langle s_p\rangle_{n,L,\beta_n(s)}\big),\qquad c_n(s):=-\,s\,\beta_n'(s),$$
where $\mathcal P_y$ is the set of the six plaquettes with base point (lower corner) $y$, as in Lüscher's $P_x$
[L10, eq. (3.1)]. If "$p\ni y$" meant all 24 plaquettes through $y$, every plaquette would be counted four times.
Then for every $F\in\mathcal O_{\rm Lip}$ and every $\varphi\in C_c^\infty(\ln J)$:

- for each $n$ and $u\in\ln J$, the sum $\sum_y a_n^4\langle F;\Theta_n^{(e^u)}(y)\rangle$ converges absolutely and is
  continuous in $u$;
- $$\lim_{n\to\infty}\int du\;\varphi(u)\sum_{y\in\Lambda_{n,L}}a_n^4\,\big\langle F;\Theta_n^{(e^u)}(y)\big\rangle_{n,L,\beta_n(e^u)}=-\int du\;\varphi'(u)\,S_{e^u}(F).\tag{$\ast$}$$

That is, the integrated, vacuum-subtracted plaquette insertion converges in $\mathcal D'(\ln J)$ to $\frac{d}{du}S_{e^u}(F)$.

**Naming the prefactor.** Let $\beta_n(s)$ be H4's coupling with $\Lambda\to s\Lambda$, that is, $t_0\to t_0/s^2$.
Then $c_n(s)=-\,d\beta_0/d\ln a$, evaluated at spacing $s a_n$ (§5). This is the normalization of H6(b3), and no
other multiplicative factor appears. The research note's sign is corrected here.

**Corollary 3 (dilatation form).** Take $\beta_n(s)$ from H4's scale condition as just described. Write the condition
as $\beta=B(\tau_0)$, where $B(\tau_0)$ is the (assumed unique) solution of
$\tau_0^2\langle E_{\tau_0}\rangle_{\beta}=c$ in lattice units on H4's infinite-volume lattice. So
$\beta_n(s)=B\big(t_0/(s a_n)^2\big)$. Assume also:

- **(A) Covariant loop approximation.** The lattice path approximating $s\gamma$ on $s a\mathbb Z^4$ is $s$ times the
  path approximating $\gamma$ on $a\mathbb Z^4$. A nearest-point rule stated in lattice units has this property.
- **(B) Base independence.** For every base $b\in a_0J$ and every box $L'\in b\mathbb N$, the limit
  $\lim_n\langle F\rangle_{2^{-n}b,\,B(t_0 4^n/b^2),\,L'}$ exists, and it depends only on $(F,L')$, not on $b$. Call it
  $S_1(F;L')$.

For $F\in\mathcal O_{\rm Lip}$ let $F^{[s]}$ be its dilate: loops $\gamma_i\to s\gamma_i$, flow times
$t_i\to s^2t_i$, smearing functions $f(x)\to s^{-4}f(x/s)$. Then (B) implies (L2), and
$S_s(F;L)=S_1(F^{[s]};sL)$. So $(\ast)$ reads
$$\lim_{n}\int du\,\varphi(u)\sum_y a_n^4\langle F;\Theta_n^{(e^u)}(y)\rangle=-\int du\,\varphi'(u)\,S_1\big(F^{[e^u]};e^uL\big),$$
which is the distributional derivative along the dilation orbit, formally $(x\cdot\partial_x+2t\partial_t+L\partial_L)S$.

**Remark 4 (two bare parameters; the sphere family).** The proofs of Theorems 1 and 2 use only three things:

- a finite spatial cell complex times $a\mathbb Z$;
- an action whose terms each couple two neighbouring slices and are bounded;
- bounded quasi-local observables with summable approximation errors (Lemma 5 holds on any complex of bounded degree).

So the same proofs apply verbatim to an anisotropic Wilson action
$\beta_\sigma\sum_{\rm spatial}s_p+\beta_\tau\sum_{\rm temporal}s_p$, on the torus or on any finite lattice model of
$S^3_R$, along any $C^1$ path $s\mapsto(\beta_\sigma,\beta_\tau)(s)$. The trace density then has two terms,
$-s\big(\beta_\sigma'\,\Sigma_{\rm spatial}+\beta_\tau'\,\Sigma_{\rm temporal}\big)$. This is the lattice form of H4′'s
two bare parameters ($g_0,c_0$). Corollary 3 does **not** transfer: a lattice on $S^3_R$ has no exact dilation
covariance.

---

## 3. What I borrow, with exact hypotheses

None of the items below is used as a premise in the proofs. Item 1 supplies definitions. Items 2 and 3 are context
that shows the identity is the standard one.

1. **[L10]** M. Lüscher, "Properties and uses of the Wilson flow in lattice QCD", JHEP 08 (2010) 071,
   arXiv:1006.4518. I read v3 directly.
   - Eq. (1.3): $S_w(U)=\frac1{g_0^2}\sum_p\operatorname{Re}\operatorname{tr}\{1-U(p)\}$, with $p$ over oriented
     plaquettes.
   - Eq. (1.4): $\dot V_t(x,\mu)=-g_0^2\{\partial_{x,\mu}S_w(V_t)\}V_t(x,\mu)$, $V_t|_{t=0}=U$.
   - His hypothesis: "The existence, uniqueness and smoothness of the Wilson flow at all positive and negative times
     $t$ is rigorously guaranteed on a finite lattice." I use (1.4) only on finite slab lattices (Lemma 5). The
     infinite-time flowed observable is defined by my uniform limit.
   - Eq. (3.1): $E=2\sum_{p\in P_x}\operatorname{Re}\operatorname{tr}\{1-V_t(p)\}$, with $P_x$ the unoriented
     plaquettes with lower-left corner $x$.
   - Eq. (3.3): $t^2\langle E\rangle|_{t=t_0}=0.3$, the scale condition H4 imitates. His paper is SU(3). The SU(2)
     transcription is H4's.
2. **[BEKLLLP96]** G. Boyd, J. Engels, F. Karsch, E. Laermann, C. Legeland, M. Lütgemeier, B. Petersson,
   "Thermodynamics of SU(3) lattice gauge theory", Nucl. Phys. B 469 (1996) 419, arXiv:hep-lat/9602007. I read the
   arXiv text directly.
   - Eq. (2.8): $\frac{f}{T^4}\big|_{\beta_0}^{\beta}=-N_\tau^4\int_{\beta_0}^{\beta}d\beta'\,[S_0-S_T]$.
   - Eq. (2.10): $\frac{\epsilon-3p}{T^4}=N_\tau^4\,T\frac{d\beta}{dT}[S_0-S_T]$.
   - Eq. (2.11): $T\frac{d\beta}{dT}=-a\frac{d\beta}{da}$.
   - (2.8) is the integrated form of $\partial_\beta\ln Z=-\langle S\rangle$, which is exact at finite spacing.
   - Their hypotheses for (2.10): an isotropic lattice at fixed $N_\tau$ (so $T$ varies through $a$), plus the relation
     between free energy and pressure, for which "one has to insure that the spatial volume is large enough".
   - Their positive prefactor is $-a\,d\beta/da$, the same sign as my corrected $c_n=-s\beta_n'$.
   - Theorem 2 is the analogue for a general bounded flowed $F$ in place of $\ln Z$, with the continuum limit taken.
3. **[CDJ77]** J. C. Collins, A. Duncan, S. D. Joglekar, "Trace and dilatation anomalies in gauge theories",
   Phys. Rev. D 16 (1977) 438–449. This is a paraphrase from the INSPIRE abstract.
   - Content: the form of the trace anomaly in a general theory of fermions and non-abelian gauge bosons, involving
     gauge-variant operators that mix with the naive trace.
   - Hypotheses: perturbation theory to all orders, BRST-covariant regularization, flat space.
   - It is the perturbative counterpart of H6(b3). **Nothing in it is used here**, and Theorem 2 does not identify
     $\Theta$ with $\frac{\beta(g)}{2g}[F^2]$.

Standard analysis used without citation: dominated convergence, including its version for series; uniform limits of
derivatives; the Kolmogorov extension theorem on products of compact metric spaces; global existence of ODE flows for
smooth vector fields on compact manifolds. I prove the Doeblin contraction inline (Lemma 1).

**No longer needed:** the Perron–Frobenius and transfer-matrix gap of register lines 2 and 27. My argument uses only
$e^{-2\beta P}\le K_\beta\le1$.

---

## 4. Proofs

Throughout, $n$ and $L$ are fixed, $I=[0,\beta_+]$, and $\delta:=e^{-4\beta_+P}\in(0,1)$. For probability measures I
write $d(\lambda,\lambda')=\sup_{|f|\le1}|\lambda f-\lambda'f|\le2$.

### Lemma 1 (Doeblin contraction)
Let $p(z,z')$ be a Markov transition density with respect to $\nu$, with $p\ge\delta$. Then
$d(\lambda p,\lambda'p)\le(1-\delta)\,d(\lambda,\lambda')$.

*Proof.* Write $p=\delta\cdot1+(1-\delta)q$, where $q$ is a Markov density. Then
$\lambda p-\lambda'p=(1-\delta)(\lambda-\lambda')q$. For $|f|\le1$ we have $|qf|\le1$, so
$|(\lambda-\lambda')qf|\le d(\lambda,\lambda')$. ∎

### Lemma 2 (Markov structure and minorization of the slab)
Put $\varphi^+_j:=K^j1$ and $\varphi^-_j:=(K^*)^j1$, where $(Kf)(z)=\int K(z,z')f(z')\,d\nu(z')$ and
$K^*(z,z')=K(z',z)$. Under $\mu_M$:

- $(z_k)$ is a forward Markov chain with transition density
  $p_k(z,z')=K(z,z')\varphi^+_{M-k-1}(z')/\varphi^+_{M-k}(z)$;
- $(z_k)$ is a backward Markov chain with density
  $q_k(z',z)=\varphi^-_{k+M}(z)K(z,z')/\varphi^-_{k+M+1}(z')$ for $z_k=z$ given $z_{k+1}=z'$.

For $\beta\in I$ all of these densities are $\ge\delta$.

*Proof.* The forward claim: integrating out $z_{k+1},\dots,z_M$ from the product density leaves
$\prod_{i<k}K\cdot\varphi^+_{M-k}(z_k)$. Dividing consecutive marginals gives $p_k$. The backward claim is the mirror
argument.

Lower bound, forward case. If $j:=M-k-1\ge1$, then $\varphi^+_j(z')=\int K(z',y)\varphi^+_{j-1}(y)\,d\nu\ge e^{-2\beta P}I_{j-1}$,
where $I_i=\int\varphi^+_i\,d\nu$. Also $\varphi^+_{j+1}(z)\le I_j\le I_{j-1}$, because $K\le1$. So
$p_k\ge e^{-2\beta P}\cdot e^{-2\beta P}\ge\delta$. If $j=0$, then $p_k=K/\varphi^+_1\ge e^{-2\beta P}\ge\delta$. The
backward case is identical. ∎

### Lemma 3 (uniform decay of correlations)
Let $X$ be bounded and measurable with respect to slices $\le b$, and $Y$ with respect to slices $\ge c>b$. Then, for
all $M$ and $\beta\in I$,
$$|\langle X;Y\rangle_{\mu_M}|\le2\|X\|_\infty\|Y\|_\infty(1-\delta)^{c-b}.$$
The same holds with the roles of past and future exchanged, and for $\mu_\infty$ (Lemma 4).

*Proof.* By the Markov property, $\mathbb E[Y\mid z_{\le b}]=g(z_b)$ with $g(z)=(\delta_zp_b\cdots p_{c-1})G$, where
$G(y)=\mathbb E[Y\mid z_c=y]$ and $|G|\le\|Y\|$. Also $\mathbb E Y=(\lambda_bp_b\cdots p_{c-1})G$, with $\lambda_b$ the
law of $z_b$. Lemma 1, applied $c-b$ times, gives $|g(z)-\mathbb EY|\le2(1-\delta)^{c-b}\|Y\|$. Then
$|\langle X;Y\rangle|=|\mathbb E[X(g(z_b)-\mathbb EY)]|$ is bounded as claimed. For $\mu_\infty$, pass to the limit
using Lemma 4. ∎

### Lemma 4 (infinite time extent)
For every cylinder function $A$ (depending on slices in $[a,b]$),
$\langle A\rangle_\infty:=\lim_{M\to\infty}\langle A\rangle_{\mu_M}$ exists, uniformly in $\beta\in I$ and in
$\|A\|_\infty\le1$. The limits come from one Borel probability measure $\mu_{\infty}=\mu_{n,L,\beta}$ on $\Omega$. The
same limit arises from periodic time, or from any positive continuous boundary weights $g_-(z_{-M})$, $g_+(z_M)$.

*Proof.*

- **(i) $L^1$ convergence.** Let $\hat\varphi^+_j:=\varphi^+_j/\int\varphi^+_j$, a probability density. Under the slab
  chain on $[0,j']$ (free ends), $z_0$ has density $\hat\varphi^+_{j'}$. By Lemma 2 its backward kernels $q_i$ depend
  only on the left end, not on $j'$. For $j'\ge j$, the laws of $z_0$ under $[0,j]$ and under $[0,j']$ are therefore
  $\lambda Q_{j-1}\cdots Q_0$ and $\lambda'Q_{j-1}\cdots Q_0$, with $\lambda,\lambda'$ the respective laws of $z_j$.
  Lemma 1 gives $\|\hat\varphi^+_j-\hat\varphi^+_{j'}\|_{L^1(\nu)}\le2(1-\delta)^j$. The same holds if the right end
  carries any weight $g_+$, since the $q_i$ are unchanged.
- **(ii) Sup-log convergence.** $\psi_j:=K\hat\varphi^+_j$ takes values in $[e^{-2\beta_+P},1]$, and
  $|\psi_j-\psi_{j'}|\le\|\hat\varphi^+_j-\hat\varphi^+_{j'}\|_1$. Since $\hat\varphi^+_{j+1}=\psi_j/\int\psi_j$, we
  get $\sup|\log\hat\varphi^+_{j+1}-\log\hat\varphi^+_{j'+1}|\le\eta_j:=4\delta^{-1/2}(1-\delta)^j$. So
  $\hat\varphi^+_j\to h^+>0$ uniformly in sup-log norm, with error at most $\eta_{j-1}$. The same holds for
  $\varphi^-$, with limit $h^-$.
- **(iii) Blocks.** Integrating out the outside slices, the law of $(z_a,\dots,z_b)$ under $\mu_M$ has density
  proportional to $\hat\varphi^-_{M+a}(z_a)\prod_{k=a}^{b-1}K(z_k,z_{k+1})\,\hat\varphi^+_{M-b}(z_b)$. Let $\rho_\infty$
  be the same expression with $h^\mp$. The two normalized densities have ratio in $[e^{-2\eta},e^{2\eta}]$, with
  $\eta=\eta_{M+a-1}+\eta_{M-b-1}$. Hence $|\langle A\rangle_M-\rho_\infty(A)|\le\|A\|(e^{2\eta}-1)\to0$, uniformly on
  $I$.
- **(iv) The measure.** The $\rho_\infty$ are consistent across blocks. Kolmogorov extension gives $\mu_\infty$.
- **(v) Boundary weights.** With product weights $g_\pm$ the same computation applies, with $K^jg_\pm$ in place of
  $K^j1$. By (i) and (ii) the limits $h^\pm$ are unchanged.
- **(vi) Periodic time.** With $M'$ slices, the block density is
  $\propto\prod K\cdot Q(z_b,z_a)$, where $Q$ is the kernel of $K^m$ and $m=M'-(b-a)$. For fixed $v$,
  $Q(\cdot,v)=K^{\lfloor m/2\rfloor}f_v$ with $f_v>0$, so by (v) it is within sup-log $\eta_{\lfloor m/2\rfloor-1}$ of a
  multiple of $h^+$, uniformly in $v$. Symmetrically, $Q(u,\cdot)$ is close to a multiple of $h^-$. So
  $R:=Q/(h^+\otimes h^-)$ satisfies $R(u,v)/R(u',v')\in e^{\pm2(\eta+\eta')}$. The block law is therefore within
  $e^{\pm4(\eta+\eta')}$ of $\rho_\infty$. ∎

### Lemma 5 (flowed observables on the infinite-time lattice are quasi-local)
Let $F=F_{\gamma,t}$ have loops in slices $[-k_\gamma,k_\gamma]$, and let $\tau=t/a_n^2$. For $k>k_\gamma$, let
$F^{(k)}$ be computed from the flow (1.4) on the finite slab lattice of slices $[-k,k]$: its links, and the plaquettes
lying entirely inside it. Then $F^{(k)}$ is a continuous cylinder function, and for $k'>k$
$$\sup_U|F^{(k)}-F^{(k')}|\le\varepsilon_k:=\operatorname{Lip}(\Phi)\,\textstyle\sum_i|\gamma_i^{(n)}|\;D_0\,\frac{(c\tau)^{k-k_\gamma}}{(k-k_\gamma)!},$$
with constants $D_0$ and $c$ depending only on the lattice geometry. So $F:=\lim_kF^{(k)}$ exists uniformly, is
continuous on $\Omega$, and has $\sum_k\varepsilon_k<\infty$. I take this limit as the meaning of the flowed observable
at infinite time. The class $\mathcal O_{\rm Lip}$ inherits the property, with summable errors.

*Proof.*

- **The ODE.** Write (1.4) as $\dot V_\ell=X_\ell(V)V_\ell$, with $X_\ell\in su(2)$ a fixed smooth function of the
  link $\ell$ and of the links $N(\ell)$ sharing a plaquette with it, $|N(\ell)|\le19$. On a finite slab this is a
  smooth vector field on a compact manifold, so the flow exists for all $\tau$.
- **Lipschitz bound.** Let $d$ be the bi-invariant Riemannian distance on $SU(2)$ (Frobenius metric). By compactness,
  $|X_\ell(V)-X_\ell(W)|\le c_1\sum_{\ell'\in N(\ell)}d(V_{\ell'},W_{\ell'})$.
- **Dini bound.** Left invariance, then right invariance, give
  $d(e^{hX}V,e^{hY}W)\le d(V,W)+d(e^{hX},e^{hY})$. Hence
  $D^+d(V_\ell,W_\ell)\le|X_\ell(V)-X_\ell(W)|$.
- **Depth.** Compare slabs $k$ and $k'$, starting from the same $U$. The vector fields differ only on the spatial
  links at slices $\pm k$. Give each slab-$k$ link a depth, its graph distance to that set. A link at depth $r\ge1$ has
  all its neighbours at depth $\ge r-1$, and they lie in slab $k$.
- **Iteration.** Let $f_r(\tau)$ be the maximum of $d(V_\ell,W_\ell)$ over depth $\ge r$. Then
  $f_r\le19c_1\int_0^\tau f_{r-1}$ and $f_0\le D_0:=\operatorname{diam}SU(2)$, so $f_r\le D_0(c\tau)^r/r!$ with
  $c=19c_1$.
- **Loop depth.** One adjacency step changes the time slice by at most one, so loop links have depth
  $\ge k-k_\gamma$.
- **Holonomy.** $|\tfrac12\operatorname{tr}A-\tfrac12\operatorname{tr}B|\le\|A-B\|\le d(A,B)$, and
  $\|\prod A_i-\prod B_i\|\le\sum\|A_i-B_i\|$ for unitaries. Combined with $\operatorname{Lip}(\Phi)$ (ℓ¹ norm on the
  cube), this gives $\varepsilon_k$.
- **The class.** Products and finite sums of such $F$ keep summable errors. ∎

### Proof of Theorem 1
Fix a compact $I=[0,\beta_+]$.

**(a) Cylinder $A$ on $[a,b]$.**

- Put $g_M(\beta)=\langle A\rangle_{\mu_M}$. This is a finite-dimensional compact integral whose integrand is smooth in
  β, so $g_M'=-\sum_{p\in\mathcal P_M}\langle A;s_p\rangle_{\mu_M}$ exactly.
- Let $p$ be assigned to slice $j$. Lemma 3 gives $|\langle A;s_p\rangle|\le4\|A\|(1-\delta)^{j-b}$ for $j>b$,
  $\le4\|A\|(1-\delta)^{a-j-1}$ for $j+1<a$, and $\le4\|A\|$ otherwise.
- Hence $\sum_p|\langle A;s_p\rangle_{\mu_M}|\le4P\|A\|(b-a+3+2/\delta)$, uniformly in $M$ and $\beta\in I$.
- By Lemma 4, $g_M\to\langle A\rangle_\infty$ uniformly on $I$, and each $\langle A;s_p\rangle_{\mu_M}$ (built from
  the cylinder functions $As_p$, $A$, $s_p$) converges uniformly.
- With the uniform geometric tail, $g_M'\to h:=-\sum_p\langle A;s_p\rangle_\infty$ uniformly on $I$. So $h$ is
  continuous, and $\langle A\rangle_\infty(\beta)-\langle A\rangle_\infty(0)=\lim\int_0^\beta g_M'=\int_0^\beta h$.

**(b) $F\in\mathcal O_{\rm Lip}$.**

- Take cylinder approximants $A_k$ on $[-k,k]$ with $\|F-A_k\|\le\varepsilon_k$, where $\varepsilon_k$ is
  non-increasing and summable (Lemma 5). Then $\langle A_k\rangle_\infty\to\langle F\rangle_\infty$ uniformly on $I$.
- Termwise, $|\langle A_k;s_p\rangle-\langle F;s_p\rangle|\le4\varepsilon_k$.
- Domination. Let $p$ be at slice $j$ with $|j|\ge2k_0+2$, and put $m=\lfloor|j|/2\rfloor$. The slices of $s_p$ are at
  distance $\ge|j|/2-1$ from $[-m,m]$.
  - For $k<m$, Lemma 3 gives $|\langle A_k;s_p\rangle|\le4C(1-\delta)^{|j|/2-1}$, with $C=C_F+\varepsilon_{k_0}$.
  - For $k\ge m$, $|\langle A_k;s_p\rangle|\le|\langle A_m;s_p\rangle|+|\langle A_k-A_m;s_p\rangle|\le4C(1-\delta)^{|j|/2-1}+8\varepsilon_m$.
  - The dominating sequence $P\big(4C(1-\delta)^{|j|/2-1}+8\varepsilon_{\lfloor|j|/2\rfloor}\big)$ is summable over
    $j$, because $\sum\varepsilon_m<\infty$. It does not depend on $k$ or β.
- Dominated convergence for series gives $(\langle A_k\rangle_\infty)'\to-\sum_p\langle F;s_p\rangle_\infty$ uniformly
  on $I$, with the sum converging absolutely. So $\langle F\rangle_\infty\in C^1(I)$ with the stated derivative. ∎

### Proof of Theorem 2

1. **Derivative identity.** Theorem 1, with $\langle F;\Theta\rangle=\langle F\Theta\rangle$ because
   $\langle\Theta\rangle=0$, and with each plaquette based at exactly one $y$, gives
   $$\sum_ya_n^4\langle F;\Theta_n^{(s)}(y)\rangle=c_n(s)\sum_p\langle F;s_p\rangle=-s\beta_n'(s)\sum_p\langle F;s_p\rangle,$$
   with absolute convergence.
2. **Chain rule.** By (L1), $u\mapsto\beta_n(e^u)$ is $C^1$ into $[0,\infty)$. Theorem 1 then makes
   $f_n(u):=\langle F\rangle_{n,L,\beta_n(e^u)}$ a $C^1$ function, with
   $f_n'(u)=e^u\beta_n'(e^u)\,\partial_\beta\langle F\rangle=-e^u\beta_n'(e^u)\sum_p\langle F;s_p\rangle$. By step 1
   this equals $\sum_ya_n^4\langle F;\Theta_n^{(e^u)}(y)\rangle$, which is therefore continuous in $u$.
3. **Integration by parts.** At fixed $n$, $\int\varphi f_n'\,du=-\int\varphi'f_n\,du$, since
   $\varphi\in C_c^\infty$ and $f_n\in C^1$.
4. **Limit.** $|f_n(u)|\le C_F$ for all $n$ and $u$. By (L2), $f_n(u)\to S_{e^u}(F)$ pointwise; the limit is Borel as
   a pointwise limit of continuous functions. Dominated convergence on $\operatorname{supp}\varphi'$ gives $(\ast)$. ∎

### Proof of Corollary 3

- **Lattice-unit data.** A lattice expectation depends only on the lattice-unit data and on β. Here those data are
  $N=L/a$, the lattice paths $\gamma^{(n)}/a$, the flow times $t/a^2$, and the smearing weights $a^4f$ on lattice
  points.
- **Exact identity.** $\beta_n(s)=B\big(t_0/(sa_n)^2\big)$ is the $s=1$ coupling at spacing $sa_n=2^{-n}(sa_0)$. Under
  (A), $(a_n,\beta_n(s),L,F)$ and $(sa_n,B(t_0/(sa_n)^2),sL,F^{[s]})$ have identical lattice-unit data. So
  $\langle F\rangle_{a_n,\beta_n(s),L}=\langle F^{[s]}\rangle_{sa_n,\beta^{(sa_0)}_n,sL}$ exactly, with
  $sL\in sa_0\mathbb N$.
- **Limit.** Take $n\to\infty$ and use (B) with base $b=sa_0$. This gives existence (so (L2) holds) and
  $S_s(F;L)=S_1(F^{[s]};sL)$. Substitute into $(\ast)$. ∎

---

## 5. The sign, checked

Along a line of constant physics, $\beta_0$ is a function of $a\Lambda$ alone. So the $\ln\Lambda$-derivative at fixed
$a$ equals the $\ln a$-derivative at fixed $\Lambda$, **with the same sign**.

- Exactly: $\beta_0(a)=B(t_0/a^2)$, so $d\beta_0/d\ln a=-2\tau_0B'(\tau_0)$, and
  $s\,d\beta_n/ds=s\,\frac{d}{ds}B\big(t_0/(sa_n)^2\big)=-2\tau_0B'(\tau_0)$ at $\tau_0=t_0/(sa_n)^2$.
- Hence $s\beta_n'=+\,d\beta_0/d\ln a$ and $c_n=-s\beta_n'=-\,d\beta_0/d\ln a$.
- One-loop check: $\beta_0=\frac{b_0}{2\pi^2}\ln\frac1{a\Lambda}$ gives $-d\beta_0/d\ln a=b_0/(2\pi^2)>0$, the register's
  value (H6 file §1(b3)), while $s\beta_n'=-b_0/(2\pi^2)$.
- Boyd et al.'s positive prefactor $-a\,d\beta/da$ (eq. (2.11)) agrees.
- The research note set $-d\beta_0/d\ln a:=s\beta_n'$. That definition flips the sign of $\Theta$, and with it the sign
  of the right side of $(\ast)$.
- The task statement repeats the same definition. With it, $(\ast)$ as written is false. With the corrected
  definition it is true.

---

## 6. What this does not give

- **No local field.** $(\ast)$ is the zero-momentum, integrated statement. The local $\Theta$ needs tightness of
  smeared $\Theta_a$ correlators at separated points (N1 in `H6-borrowed-routes.md`; H6 §6). The left side of $(\ast)$
  includes the region where $y$ meets the loops of $F$. Contact terms are inside it and are not separated.
- **No pointwise derivative in the scale.** $(\ast)$ holds in $\mathcal D'(\ln J)$. A pointwise statement needs
  equicontinuity of $u\mapsto f_n(u)$, which is a uniform-in-$n$ bound on the integrated insertion. I do not have that
  bound. My Lemma 3 constants ($\delta=e^{-24\beta N^3}$) degrade without limit as $n\to\infty$. They serve at fixed
  $n$ only.
- **No identification with $\frac{\beta(g)}{2g}[F^2]$.** The prefactor is the lattice one, $-d\beta_0/d\ln a$. That it
  equals $\frac{b_0}{2\pi^2}+O(\beta_0^{-1})$, and that $\Theta$ is the renormalized $\frac{\beta}{2g}[O]$, are
  perturbative statements ([CDJ77]; two-loop asymptotics of $B$). Neither is proved here.
- **No non-triviality.** Nothing shows that $\frac{d}{du}S_{e^u}(F)\neq0$. That is H4(f) and H6, through line 26.
- **No infinite volume.** Everything is at fixed box $L$, which is H4(a). Summing $y$ over $\mathbb R^4$ after
  $L\to\infty$ needs clustering uniform in $L$ (H3, via H4(b)). Not given.
- **Only Lipschitz $\Phi$.** For $\Phi\in C_b$ only, I cannot show absolute convergence of $\sum_p\langle F;s_p\rangle$
  at fixed $n$. The class $\mathcal O_{\rm Lip}$ is dense, but $(\ast)$ is not shown to extend by density, because
  its left side is not controlled in sup norm of $\Phi$.
- **Not on the sphere family as such.** Remark 4 gives the two-coupling identity on any finite spatial lattice model
  of $S^3_R$. H4′ does not fix a sphere regulator, and Corollary 3's exact dilation fails on the sphere. So nothing is
  claimed for H4′'s continuum rooms.
- **The hypotheses are real.**
  - (L1) presumes that H4's scale condition is meaningful and $C^1$. That needs the infinite-volume lattice state
    $\mu_{n,\infty}$ (four infinite directions) to exist and make $\tau_0^2\langle E_{\tau_0}\rangle_\beta$ well
    defined, with a unique, $C^1$ solution $B$. Monotonicity in β is not proved, and my one-dimensional Lemma 4 does
    not cover that state.
  - (L2)/(B) is H4(a) for bases in an interval. That is open.
- **H6 stays [X].** This is one conditional sub-statement of (b3).

---

## 7. Status if adopted

Under H6(b3): **[D, conditional on (L1), (L2)]**. The integrated trace identity holds at fixed box, with the lattice
normalization $-d\beta_0/d\ln a=-s\beta_n'$ (sign corrected) and no further multiplicative renormalization, for
$\mathcal O_{\rm Lip}$. The dilatation form is [D, conditional also on (A), (B)]. The infinite-time β-derivative
(Theorem 1) is [P] at each fixed $(n,L)$. H6 and the seven top-level hypotheses are unchanged.
