# H3 closure note: Lemma G, the Gaussian calibration of the Dobrushin–Shlosman one-box criterion

*28 November 2026 (night of the 27th). Proof note. Nothing in the register changes because of this note. H3 (line 20) stays [X]. This note closes one sub-statement inside H3 Step 5. It concerns the normalization of the finite-size criterion and is stated for a Gaussian template, not for Yang–Mills. Tags: [P] proved here, elementary and self-contained; [cited] a statement taken from a source with that source's hypotheses; [C] a small computation here; [guess] mine, labelled.*

---

## 0. Where this sits

In my research note `H3-borrowed-routes.md` (§3a–b, §4) I replaced my central-block mixing condition MS with its averaged, Dobrushin–Shlosman form MS′. In MS′ the sum of boundary influences on a box is compared with the box volume $|V|$, not with $1$. The worry behind my old Sub-conjecture 5.1 was this. A box of physical side $\ell$ at spacing $a$ has about $(\ell/a)^{d-1}$ boundary sites. A sum of boundary influences could therefore blow up as $a\to0$, and then no cutoff-uniform one-box criterion could exist even in principle.

Lemma G settles this for the Gaussian template, exactly. The DS sum equals the integral of a killed-walk exit probability. The boundary count is cancelled exactly by the volume normalization. The normalized coefficient has a limit as $a\to0$. That limit lies strictly below 1 when there is a mass and equals 1 exactly when there is none. Proposition G0 shows that the massless failure is not a Gaussian accident. It holds for every specification with a global translation symmetry of the spins.

While writing, I made two changes to the route as it was handed to me:
1. Part (iii) is now **proved here** by an explicit discrete-sine computation, so the citation to "standard convergence of the discrete Dirichlet problem" is no longer needed. That citation would also have needed care. With constant boundary data on a cube, the continuum solution is not $C^2$ up to the edges and corners, because the data are incompatible there with $(-\Delta+m^2)u=0$. So the textbook $O(a^2)$ consistency argument does not apply as stated.
2. I added an explicit bound, (iv), valid at **every** spacing. It also yields $\sup_a\alpha_V<1$ directly.

---

## 1. What I borrow (verified)

**Source of the definition.** R. L. Dobrushin and S. B. Shlosman, "Constructive criterion for the uniqueness of Gibbs field", in *Statistical Physics and Dynamical Systems: Rigorous Results* (J. Fritz, A. Jaffe, D. Szász, eds.), Birkhäuser, Boston, 1985, pp. 347–370. I could not obtain the chapter. I take the definitions from Shlosman's own restatement: S. Shlosman, "The critical temperature $T_{cr}$(Ising) is DS-computable", arXiv:2505.24750 (v1 30 May 2025; v2 3 Dec 2025), §1.1 and §2.1. I read the full text of v2 tonight. Its reference list gives the DS chapter with exactly the bibliographic data above (pp. 347–370, Birkhäuser Boston, 1985).

**The setting there, quoted in content.** "Consider a finite range lattice spin system $\sigma=\{\sigma_x,x\in\mathbb Z^d\}$ defined by the (formal) Hamiltonian $H(\sigma)=-\sum_{A\subset\mathbb Z^d}U_A(\sigma)$, with single spins $\sigma_x$ taking values in a **finite set** $S$". The $U_A$ are **translation-invariant**, depend only on $\sigma_A$, and vanish when $\operatorname{diam}A>R$. The boundary is $\partial V=\{y\notin V:\operatorname{dist}(y,V)\le R\}$. The conditional Gibbs distribution in $V$ is $q_V(\sigma_V\mid\sigma_{\partial V})=e^{-H_V(\sigma_V\mid\sigma_{\partial V})}/Z(V,\sigma_{\partial V})$.

**The coefficients (§1.1).** Give the single-spin space $S$ a metric $\rho$. Give $\Omega_V$ the sum metric $\rho_V(\sigma_V,\bar\sigma_V)=\sum_{x\in V}\rho(\sigma_x,\bar\sigma_x)$. Let $d_K$ be the Kantorovich distance for $\rho_V$. Then
$$k_{V,y}=\max_{\sigma_y\neq\sigma'_y;\ \sigma_{\partial V\setminus y}}\frac{d_K\big(q_V(\cdot\mid\sigma_{\partial V\setminus y}\cup\sigma_y),\,q_V(\cdot\mid\sigma_{\partial V\setminus y}\cup\sigma'_y)\big)}{\rho(\sigma_y,\sigma'_y)} .$$

**The Kantorovich distance (§2.1).** On a metric space $(X,\rho)$, $d_K(\mu,\nu)=\inf_{P\in\Pi(\mu,\nu)}\int\rho(x,y)\,P(dx,dy)$, where $\Pi(\mu,\nu)$ is the set of couplings: probability measures on $X\times X$ with marginals $\mu$ and $\nu$.

**Definition 1 ([DS], as restated).** For a finite box $V\subset\mathbb Z^d$, the interaction satisfies $C_V$ if $\sum_{y\in\partial V}k_{V,y}<|V|$.

**Theorem 2 ([DS], as restated).** "If for some finite $V\subset\mathbb Z^d$ the interaction $U$ satisfies the $C_V$ condition, then it has unique Gibbs state. Moreover, this state has exponentially decaying correlations."

**How I use it.** I use **only the definition** of $k_{V,y}$ and of $C_V$, and I do not use Theorem 2. My spins are real-valued, so $S=\mathbb R$ is not finite. I therefore read the definition with $\max$ replaced by $\sup$. In Lemma G the ratio turns out to be constant, so the supremum is attained. I set $\rho(s,s')=|s-s'|$, the restriction of the Kantorovich setting to measures with finite first moment, which Gaussians have. The interaction has range 1. If I take $\operatorname{dist}$ to be graph ($\ell^1$) distance, $\partial V$ is the outer vertex boundary. With $\ell^\infty$ distance, $\partial V$ also contains edge- and corner-diagonal sites. Those sites do not interact with $V$, so their $k_{V,y}=0$ and every sum below is unchanged. §6 explains why Theorem 2 **cannot** be applied to this template verbatim.

---

## 2. Setting and notation

Fix $d\ge1$, a mass $m\ge0$, a physical side $\ell>0$ and an integer $n\ge1$. Put $a=\ell/(n+1)$ and
$$V=V_n=\{a,2a,\dots,na\}^d\subset a\mathbb Z^d,\qquad \partial V=\{y\in a\mathbb Z^d\setminus V:\ y\sim x\text{ for some }x\in V\},$$
where $y\sim x$ means nearest neighbours. Every $y\in\partial V$ has exactly one coordinate in $\{0,\ell\}$ and the others in $\{a,\dots,na\}$, and it has exactly one neighbour in $V$. Fields are dimensionless lattice fields $\phi:a\mathbb Z^d\to\mathbb R$. Given boundary data $\tau\in\mathbb R^{\partial V}$, the conditional Hamiltonian is
$$H_V(\phi_V\mid\tau)=\tfrac12\!\!\sum_{\{x,z\}\subset V,\ x\sim z}\!\!(\phi_x-\phi_z)^2+\tfrac12\!\!\sum_{x\in V,\ y\in\partial V,\ x\sim y}\!\!(\phi_x-\tau_y)^2+\tfrac12(ma)^2\sum_{x\in V}\phi_x^2 ,$$
and $q_V(d\phi_V\mid\tau)\propto e^{-H_V(\phi_V\mid\tau)}\,d\phi_V$.

Matrices on $\mathbb R^V$ and $\mathbb R^{V\times\partial V}$:
- $\mathrm{Adj}_V$ is the adjacency matrix of $V$;
- $A=(2d+(ma)^2)\,I-\mathrm{Adj}_V$;
- $B_{x,y}=\mathbf 1[x\sim y]$ for $x\in V$, $y\in\partial V$;
- $H_V:=A^{-1}B$ is the *killed harmonic measure*;
- $u_V:=H_V\mathbf 1_{\partial V}$, so $u_V(x)=\sum_{y}H_V(x,y)$;
- $\alpha_V:=|V|^{-1}\sum_{y\in\partial V}k_{V,y}$, so $C_V\iff\alpha_V<1$.

I write $\alpha_n(m\ell)$ for $\alpha_{V_n}$, and §3(iii) shows that it depends on $(m,\ell)$ only through $m\ell$.

---

## 3. Statements

**Lemma G [P].** For every $d\ge1$, $m\ge0$, $\ell>0$, $n\ge1$, and with the notation of §2:

**(o)** For every $y\in\partial V$, $k_{V,y}=\sum_{x\in V}H_V(x,y)$, and hence
$$\sum_{y\in\partial V}k_{V,y}=\sum_{x\in V}u_V(x).$$
Here $H_V(x,y)\ge0$, and $u_V(x)\in(0,1]$ is the probability that the walk started at $x$ leaves $V$ before it dies. At each step this walk moves to each of the $2d$ neighbours with probability $1/(2d+(ma)^2)$ and dies with probability $(ma)^2/(2d+(ma)^2)$.

**(i)** If $m=0$, then $u_V\equiv1$ and $\sum_yk_{V,y}=|V|$ exactly, for every $d,n,\ell$. So $C_V$ fails, with $\alpha_V=1$.

**(ii)** If $m>0$, then $0<u_V(x)\le 2d/(2d+(ma)^2)<1$ for every $x\in V$. So $\alpha_V<1$, and $C_V$ holds, for every $V_n$.

**(iii)** For $m\ge0$, $\alpha_n$ is the explicit sum
$$\alpha_n(m\ell)=1-\sum_{k\in\{1,3,5,\dots\}^d,\ k_i\le n}\ \prod_{i=1}^d\frac{2\cot^2\!\big(\tfrac{\pi k_i}{2(n+1)}\big)}{n(n+1)}\cdot\frac{(m\ell)^2}{(m\ell)^2+(n+1)^2\sum_i4\sin^2\!\big(\tfrac{\pi k_i}{2(n+1)}\big)} .$$
As $n\to\infty$ at fixed $m\ell$ (equivalently $a\to0$ at fixed $m$ and $\ell$),
$$\alpha_n(m\ell)\ \longrightarrow\ \alpha(m\ell):=1-\sum_{k\in\{1,3,5,\dots\}^d}\ \prod_{i=1}^d\frac{8}{\pi^2k_i^2}\cdot\frac{(m\ell)^2}{(m\ell)^2+\pi^2|k|^2},$$
and $\alpha(m\ell)=\ell^{-d}\int_{[0,\ell]^d}u$, where $u$ solves $(-\Delta+m^2)u=0$ in $(0,\ell)^d$ with $u=1$ on the boundary, the sine series being that solution's expansion. For $m\ell>0$, $0<\alpha(m\ell)<1$. In $d=1$, $\alpha(m\ell)=\frac{2}{m\ell}\tanh\frac{m\ell}{2}$.

**(iv) (bound at every spacing).** For $m>0$, with $\theta=2\operatorname{arcsinh}(ma/2)$,
$$\alpha_n(m\ell)\ \le\ d\cdot\frac{\sinh(n\theta/2)}{n\,(ma/2)\cosh((n+1)\theta/2)}\ \le\ \frac{2d}{m(\ell-a)}=\frac{2d}{m\ell}\cdot\frac{n+1}{n},$$
with equality in the first inequality when $d=1$. In the limit, $\alpha(m\ell)\le\frac{2d}{m\ell}\tanh\frac{m\ell}{2}$.

**(v) (uniformity in the cutoff).** For every $m\ell>0$, $\displaystyle\bar\alpha(m\ell):=\sup_{n\ge1}\alpha_n(m\ell)<1$.

**Proposition G0 [P] (translation symmetry forces $\alpha_V\ge1$).** Let $V$ be finite and $\partial V$ finite. Let $q_V(\cdot\mid\tau)$, $\tau\in\mathbb R^{\partial V}$, be any family of probability laws on $\mathbb R^V$ with finite first moments. Assume it is **translation covariant**: $q_V(\cdot\mid\tau+c\mathbf 1)$ is the image of $q_V(\cdot\mid\tau)$ under $\phi_V\mapsto\phi_V+c\mathbf 1$, for all $c\in\mathbb R$. This holds whenever the conditional Hamiltonian satisfies $H_V(\phi_V+c\mid\tau+c)=H_V(\phi_V\mid\tau)$, for example any nearest-neighbour gradient model $\sum V(\phi_x-\phi_z)$, Gaussian or not. Then, with $\rho(s,s')=|s-s'|$ and the sum metric, $\sum_{y\in\partial V}k_{V,y}\ge|V|$, so $C_V$ fails.

---

## 4. Proofs

**Step 1 ($A$ is positive definite, and $A^{-1}\ge0$ entrywise).** For $\phi\in\mathbb R^V$,
$$\phi^{\mathsf T}A\phi=\sum_{\{x,z\}\subset V,\,x\sim z}(\phi_x-\phi_z)^2+\sum_{x\in V}\#\{y\in\partial V:y\sim x\}\,\phi_x^2+(ma)^2\sum_x\phi_x^2 .$$
This uses $2d=\#\{z\in V:z\sim x\}+\#\{y\in\partial V:y\sim x\}$. If the form vanishes, $\phi$ is constant on the connected set $V$ and vanishes at any $x$ adjacent to $\partial V$, which exist. So $\phi=0$, and $A\succ0$. Put $D=2d+(ma)^2$ and $P=\mathrm{Adj}_V/D$. $\mathrm{Adj}_V$ is symmetric and nonnegative, and all its eigenvalues are $<D$ because $A\succ0$. By Perron–Frobenius its spectral radius equals its largest eigenvalue, so $\mathrm{spr}(P)<1$. Hence $A^{-1}=D^{-1}\sum_{j\ge0}P^j\ge0$ entrywise, and $H_V=A^{-1}B\ge0$. Expanding the Neumann series gives
$$H_V(x,y)=\sum_{j\ge0}\sum_{x'\in V}P^j(x,x')\,D^{-1}\mathbf 1[x'\sim y],$$
the probability that the walk of (o) first leaves $V$ at $y$ before dying. Summing over $y$ shows that $u_V(x)$ is the probability of leaving before dying.

**Step 2 (the conditional law).** Expanding $H_V$, I get $H_V(\phi_V\mid\tau)=\tfrac12\phi^{\mathsf T}A\phi-\phi^{\mathsf T}B\tau+\mathrm{const}(\tau)$. Since $A\succ0$,
$$q_V(\cdot\mid\tau)=\mathcal N\big(h_\tau,\ A^{-1}\big),\qquad h_\tau=A^{-1}B\tau=H_V\tau .$$
The covariance does not depend on $\tau$. The mean solves $D\,h(x)=\sum_{z\sim x}h(z)$ for $x\in V$, with $h=\tau$ on $\partial V$.

**Step 3 (one boundary site moved).** Let $\tau'=\tau+\epsilon e_y$ with $\epsilon\ne0$. Then $q_V(\cdot\mid\tau')=\mathcal N(h_\tau+\epsilon H_V(\cdot,y),A^{-1})$, which is the law of $\phi+\epsilon H_V(\cdot,y)$ when $\phi\sim q_V(\cdot\mid\tau)$.

**Step 4 (upper bound).** The coupling $(\phi,\phi+\epsilon H_V(\cdot,y))$ has $\mathbb E\,\rho_V=\sum_x|\epsilon|H_V(x,y)$, using $H_V\ge0$. So $d_K\le|\epsilon|\sum_xH_V(x,y)$.

**Step 5 (lower bound).** For any coupling $P$ of the two laws, both have finite first moments, so
$$\int\sum_x|\sigma_x-\sigma'_x|\,dP\ \ge\ \sum_x\Big|\int(\sigma_x-\sigma'_x)\,dP\Big|=\sum_x|h_\tau(x)-h_{\tau'}(x)|=|\epsilon|\sum_xH_V(x,y).$$
Taking the infimum gives $d_K\ge|\epsilon|\sum_xH_V(x,y)$.

**Step 6 (proof of (o)).** By Steps 4–5 the ratio $d_K/\rho(\tau_y,\tau'_y)$ equals $\sum_xH_V(x,y)$ for every $\tau_{\partial V\setminus y}$ and every $\tau_y\ne\tau'_y$. So the supremum is attained and $k_{V,y}=\sum_xH_V(x,y)$. Summing over $y$ and exchanging the finite sums gives $\sum_yk_{V,y}=\sum_xu_V(x)$. ∎(o)

**Step 7 (proof of (i)).** Let $m=0$. Then $(A\mathbf 1_V)_x=2d-\#\{z\in V:z\sim x\}=\#\{y\in\partial V:y\sim x\}=(B\mathbf 1_{\partial V})_x$. So $u_V=A^{-1}B\mathbf 1_{\partial V}=\mathbf 1_V$ exactly, and $\sum_yk_{V,y}=|V|$. ∎(i)

**Step 8 (proof of (ii)).** Let $m>0$. Then $u_V=A^{-1}B\mathbf 1\ge0$, and $u_V>0$ because $\sum_jP^j$ connects every $x$ to a boundary-adjacent site. Let $M=\max_Vu_V$, attained at $x^*$. The mean equation gives $D\,M=\sum_{z\sim x^*}u(z)\le2d\max(M,1)$, with $u=1$ on $\partial V$. If $M\ge1$, this says $DM\le2dM$, which is impossible since $D>2d$. So $M<1$, and then $DM\le2d$, that is $M\le2d/(2d+(ma)^2)$. ∎(ii)

**Step 9 (proof of (iii): the exact formula).** Put $w=\mathbf 1_V-u_V$. By Step 7's identity, $A\mathbf 1_V=B\mathbf 1_{\partial V}+(ma)^2\mathbf 1_V$, so $w=(ma)^2A^{-1}\mathbf 1_V$. The one-dimensional Dirichlet matrix $T_n=2I-\mathrm{Adj}_{\{1..n\}}$ has the orthonormal eigenbasis
$$e_k(j)=\sqrt{\tfrac{2}{n+1}}\sin\tfrac{\pi kj}{n+1},\qquad k,j=1,\dots,n,$$
with eigenvalues $4\sin^2\frac{\pi k}{2(n+1)}$. Also $A=\sum_iT_n^{(i)}+(ma)^2I$, a Kronecker sum. So $A$ is diagonal in the product basis $e_k=\otimes_ie_{k_i}$, with eigenvalue $(ma)^2+\sum_i4\sin^2\frac{\pi k_i}{2(n+1)}$. Writing $\varphi=\pi k/(n+1)$,
$$\sum_{j=1}^n\sin(j\varphi)=\frac{\sin(n\varphi/2)\sin((n+1)\varphi/2)}{\sin(\varphi/2)}.$$
For odd $k$ this equals $\cot(\varphi/2)$, since then $\sin((n+1)\varphi/2)=\pm1$ and $\sin(n\varphi/2)=\sin(\pi k/2)\cos(\varphi/2)$. For even $k$ it is $0$. Hence $\langle\mathbf 1,e_k\rangle^2=\prod_i\frac{2}{n+1}\cot^2\frac{\pi k_i}{2(n+1)}$ for $k\in\{1,3,\dots\}^d$ and $0$ otherwise. Then
$$\alpha_n=1-\frac{1}{|V|}\langle\mathbf 1,w\rangle=1-\frac1{n^d}\sum_k\langle\mathbf 1,e_k\rangle^2\frac{(ma)^2}{(ma)^2+\sum_i4\sin^2\frac{\pi k_i}{2(n+1)}},$$
and with $ma=m\ell/(n+1)$ this is the displayed formula. It depends only on $m\ell$.

**Step 10 (proof of (iii): the limit).** Extend each summand by $0$ for $k_i>n$. Fix $k$. As $n\to\infty$, $\frac{2}{n(n+1)}\cot^2\frac{\pi k_i}{2(n+1)}\to\frac{8}{\pi^2k_i^2}$ and $(n+1)^2\,4\sin^2\frac{\pi k_i}{2(n+1)}\to\pi^2k_i^2$. So each summand converges to the corresponding continuum summand. For domination, use $\cot\vartheta\le1/\vartheta$ on $(0,\pi/2]$:
$$\frac{2}{n(n+1)}\cot^2\frac{\pi k_i}{2(n+1)}\le\frac{8(n+1)}{n\pi^2k_i^2}\le\frac{16}{\pi^2k_i^2},$$
and the second factor is $\le1$. Since $\sum_{k\in\{1,3,\dots\}^d}\prod_i16/(\pi^2k_i^2)=2^d<\infty$, dominated convergence (Tannery) gives $\alpha_n(m\ell)\to\alpha(m\ell)$.

Bounds on the limit: $\sum_{k\text{ odd}}8/(\pi^2k^2)=1$, so the weights $\prod_i8/(\pi^2k_i^2)$ sum to $1$. For $m\ell>0$ every factor $(m\ell)^2/((m\ell)^2+\pi^2|k|^2)$ lies strictly in $(0,1)$. Hence $0<\alpha(m\ell)<1$.

Identification with the continuum problem: $\{\prod_i\sqrt{2/\ell}\sin(\pi k_ix_i/\ell)\}$ is the Dirichlet eigenbasis of $(0,\ell)^d$, and $\ell^{-1/2}\langle 1,\sqrt{2/\ell}\sin(\pi k\cdot/\ell)\rangle=\sqrt{8}/(\pi k)$ for odd $k$. So the series equals $1-\ell^{-d}\int w$, where $w=m^2(-\Delta_D+m^2)^{-1}1$ and $u=1-w$. This $u$ is the solution of $(-\Delta+m^2)u=0$, $u|_\partial=1$, in the usual weak sense ($u-1\in H^1_0$).

$d=1$: $u(x)=\cosh(m(x-\ell/2))/\cosh(m\ell/2)$, with mean $\frac{2}{m\ell}\tanh\frac{m\ell}{2}$. ∎(iii)

**Step 11 (proof of (iv): an explicit supersolution).** Let $\mu=\theta/a$ with $\theta=2\operatorname{arcsinh}(ma/2)$, so that $2\cosh(\mu a)=2+(ma)^2$. For each direction $i$ put
$$v_i(x)=\frac{\cosh(\mu(x_i-\ell/2))}{\cosh(\mu\ell/2)}>0 .$$
Two neighbours of $x$ lie in direction $i$, contributing $2\cosh(\mu a)\,v_i(x)$, and $2(d-1)$ lie in the other directions, contributing $v_i(x)$ each. So $D\,v_i(x)=\sum_{z\sim x}v_i(z)$ at every $x$. Put $v=\sum_iv_i$. It satisfies the same equation, and on $\partial V$ the face coordinate $x_i\in\{0,\ell\}$ gives $v\ge v_i=1$. In matrix form, $A\,v|_V=B\,v|_{\partial V}$. Therefore $v|_V-u_V=A^{-1}B(v|_{\partial V}-\mathbf 1)\ge0$ by Step 1, so $u_V\le v$ pointwise.

Average over $V$. With $c=(n+1)/2$,
$$\sum_{j=1}^n\cosh(\theta(j-c))=\frac{\sinh(n\theta/2)}{\sinh(\theta/2)}\quad\text{and}\quad\sinh(\theta/2)=ma/2 .$$
So
$$\alpha_n\le d\cdot\frac{\sinh(n\theta/2)}{n(ma/2)\cosh((n+1)\theta/2)}\le\frac{2d\tanh((n+1)\theta/2)}{nma}\le\frac{2d}{m(\ell-a)},$$
using $na=\ell-a$. In $d=1$, $v=u$ by uniqueness of the Dirichlet solution, which is invertibility of $A$, so the first bound is an equality. As $n\to\infty$, $(n+1)\theta/2\to m\ell/2$ and $n\,ma/2\to m\ell/2$, which gives the continuum bound. ∎(iv)

**Step 12 (proof of (v)).** Fix $m\ell>0$. By (iii) there is $N$ with $\alpha_n\le\frac{1+\alpha(m\ell)}{2}<1$ for $n>N$. By (ii), each of $\alpha_1,\dots,\alpha_N$ is $<1$. So $\bar\alpha(m\ell)\le\max\{\alpha_1,\dots,\alpha_N,\frac{1+\alpha(m\ell)}2\}<1$. When $m\ell>4d$, (iv) gives this directly with the explicit value $\bar\alpha\le4d/(m\ell)$, because $a\le\ell/2$ for every $n\ge1$. ∎(v)

**Proof of Proposition G0.** Fix $\tau$ and $c\ne0$. List $\partial V=\{y_1,\dots,y_r\}$ and set $\tau^{(0)}=\tau$, $\tau^{(s)}=\tau^{(s-1)}+c\,e_{y_s}$. Consecutive data differ at the single site $y_s$, by $|c|$. By the definition of $k$ and the triangle inequality for $d_K$,
$$d_K\big(q_V(\cdot\mid\tau),q_V(\cdot\mid\tau+c\mathbf 1)\big)\le|c|\sum_sk_{V,y_s}.$$
By covariance, $q_V(\cdot\mid\tau+c\mathbf 1)$ is $q_V(\cdot\mid\tau)$ translated by $c\mathbf 1_V$. The mean argument of Step 5 then gives $d_K\ge\sum_{x\in V}|c|=|V||c|$. Dividing by $|c|$ gives $\sum_yk_{V,y}\ge|V|$. ∎

This is consistent with Theorem 2's conclusion. A translation-covariant specification can never have a unique Gibbs measure: if $\mu$ is Gibbs, so is every translate $\mu(\cdot-c)$, and these differ whenever $\mu$ has a first moment. Lemma G(i) is the Gaussian case, with equality.

---

## 5. Numerical check [C]

The check script is `/Users/bdavis/.claude/jobs/c8f0f39d/tmp/hqwork/docs/research/borrowed-routes-20261128/H3-closure-check.py`. It is pure Python, uses $m=1$ and $\ell=m\ell$, and ran in seconds. For each case it computes $\alpha_n$ two independent ways: a Gauss–Seidel solve of $Au=B\mathbf 1$ (to $10^{-13}$), and the closed formula of Step 9. It also evaluates the bound (iv) and the continuum series.

| $d$ | $n$ | $m\ell$ | direct | formula (iii) | $\max u_V$ | bound (ii) | bound (iv), first / second |
|---|---|---|---|---|---|---|---|
| 1 | 50 | 3 | 0.5957 | 0.5957 | 0.9485 | 0.9983 | 0.5957 (equal) / 0.6800 |
| 2 | 8 | 4 | 0.6145 | 0.6145 | 0.8389 | 0.9529 | 0.8597 / 1.125 |
| 2 | 16 | 4 | 0.6436 | 0.6436 | 0.9338 | 0.9863 | 0.9060 / 1.0625 |
| 2 | 8 | 16 | 0.1195 | 0.1195 | 0.3427 | 0.5586 | 0.1263 / 0.2812 |
| 4 | 4 | 4 | 0.7348 | 0.7348 | 0.8303 | 0.9259 | 1.586 / 2.5 |
| 4 | 6 | 4 | 0.7604 | 0.7604 | 0.8997 | 0.9608 | 1.668 / 2.333 |
| 4 | 4 | 16 | 0.1585 | 0.1585 | 0.2670 | 0.4386 | 0.1792 / 0.625 |

Continuum values from the series: $\alpha(3)=0.603432$ in $d=1$, matching $\frac23\tanh\frac32$ to $10^{-13}$; $\alpha(4)=0.6797$ and $\alpha(16)=0.2301$ in $d=2$; $\alpha(4)=0.8422$ in $d=4$. The last is stable to $10^{-5}$ between truncations $k\le39$ and $k\le59$. These agree with the research note's table (0.7348, 0.7604, 0.7766 at $d=4$, $m\ell=4$) and continue its upward trend toward 0.842. **Observed, not proved:** $\alpha_n$ increases with $n$ in every case I computed. Lemma G does not need this.

At $m\ell=4$ in $d=4$, bound (iv) is not informative, since it exceeds 1. (v) still holds there through (ii)+(iii). Bound (iv) is the useful one at large $m\ell$.

---

## 6. What this does not give

1. **It does not apply DS Theorem 2 to the free field, and it could not.** The restated Theorem 2 is for a finite single-spin space $S$ with translation-invariant finite-range interactions. My spins are real. The failure is real, not formal. For $m>0$, $C_V$ holds by (ii), yet the massive lattice free field does **not** have a unique Gibbs measure among all Gibbs measures. Let $h(x)=e^{\mu x_1}$ with $2\cosh(\mu a)=2+(ma)^2$. It solves $Dh(x)=\sum_{z\sim x}h(z)$ on all of $a\mathbb Z^d$, so the free field shifted by $h$ has the same specification (the conditional mean $H_V(\tau-h)+h_V=H_V\tau$). Uniqueness can only hold within a growth class, for example tempered measures. I do not know whether the 1985 original treats non-compact spins with such a class, because I have not seen it. Lemma G calibrates the **number** $\alpha_V$, not the theorem. For compact SU(2) link variables this obstruction is absent, but there the Gaussian is only a small-field picture.
2. **Nothing about Yang–Mills.** The template is a scalar Gaussian with $\rho=|s-s'|$. The actual MS′ uses SU(2) links with the geodesic metric, gauge invariance (or gauge fixing inside $V$), and plaquette interactions. Lemma G is evidence that the averaged normalization is the *right* one: it is cutoff-uniform whenever there is a mass. It is not a verification of MS′ for any gauge theory, at any coupling.
3. **It is DS's $C_V$, not BBLR's $D(n,\alpha)$.** The averaged condition I wrote as MS′ follows Bhattacharjee–Bhattacharya–Lockwood–Romanov (unrefereed) and counts $2\mathbb Z^4$-translates of a cube. Lemma G concerns the single-box sum $\sum_yk_{V,y}$ versus $|V|$. I expect a counting translation between the two normalizations, but I have not written it.
4. **The massless negative is exactly as wide as Proposition G0.** I proved: every translation-covariant specification on real spins, Gaussian or not, has $\alpha_V\ge1$ (with $\rho=|s-s'|$ and the sum metric). The massless scalar Gaussian sits exactly at $\alpha_V=1$. The non-compact free Maxwell field in a gauge preserved by constant shifts $A_\mu\mapsto A_\mu+c_\mu$ has that symmetry (constant connections are flat). G0 should then apply direction by direction, *provided* the gauge-fixed conditional law in the box is normalizable. I have not checked normalizability for any specific gauge, so for Maxwell this is a **[guess]**. Compact SU(2) has no exact translation symmetry, so G0 says nothing about the true lattice theory. Its content is about approximations: no fixed-gauge, massless, linearized approximation can verify a DS-type criterion with this metric.
5. **No rate, no RP, no room.** Nothing here converts $\alpha<1$ into a decay rate or a spectral gap. The explicit rate $\log(1/\alpha)/(2n)$ belongs to BBLR's Theorem 43 (unrefereed), and the room-graph rerun (my Step 2 of §4 in the research note) is untouched. The geometry is a flat cube with boundary, not my Berger room. (5-imp) and (5-ver) of H3 Step 5 keep their status.
6. **Bound (iv) is not sharp in $d\ge2$, and monotonicity in $n$ is unproved.** For $d\ge2$, (iv) overestimates by up to about $d$ at small $m\ell$, because the faces' supersolutions add at edges and corners. The rise of $\alpha_n$ with $n$ is only observed.
7. **The choice of metric matters.** With a bounded metric such as $\min(|s-s'|,1)$, the coefficients change, and the clean identity (o) is lost. I chose $\rho=|s-s'|$ because it makes $k_{V,y}$ invariant under rescaling the field. So the cutoff-uniformity above is not an artefact of the field normalization $\phi\sim a^{(d-2)/2}\phi_{\rm cont}$.

---

## 7. Status, for the register owner to judge

- **Lemma G (o)–(v) and Proposition G0: [P].** Every step is proved above with elementary linear algebra and dominated convergence. The only borrowed item is the **definition** of $k_{V,y}$ and $C_V$ (Dobrushin–Shlosman 1985, via Shlosman arXiv:2505.24750 §1.1, §2.1), used outside its finite-$S$ setting as a definition only.
- **What it closes inside H3:** the normalization part of my old Sub-conjecture 5.1, in the averaged (DS) form and for the Gaussian template. A boundary count of order $(\ell/a)^{d-1}$ is not an obstruction, because the averaged one-box coefficient has a cutoff-independent limit $\alpha(m\ell)<1$ and $\sup_a\alpha_V<1$. The other closed item is a clean negative. Exact translation symmetry, the property of any massless linearized approximation, forces $\alpha_V\ge1$. So (5-ver) is irreducibly a non-perturbative mass statement at one physical scale.
- **H3 (line 20): unchanged, [X].**

---

### References

1. R. L. Dobrushin, S. B. Shlosman, "Constructive criterion for the uniqueness of Gibbs field", in *Statistical Physics and Dynamical Systems: Rigorous Results* (J. Fritz, A. Jaffe, D. Szász, eds.), Birkhäuser, Boston, 1985, pp. 347–370. Statement taken via ref. 2. Bibliographic data from ref. 2's reference list. I took the editors from a search-indexed Springer record of the volume, but I did not open that page.
2. S. Shlosman, "The critical temperature $T_{cr}$(Ising) is DS-computable", arXiv:2505.24750 [math-ph], v2 (3 Dec 2025). Read in full: §1.1 (setting, $k_{V,y}$, Definition 1, Theorem 2) and §2.1 (Kantorovich distance).
3. D. Bhattacharjee, U. Bhattacharya, J. M. Lockwood, T. Romanov, "One Cube Estimate Implies the Yang–Mills Mass Gap", Hexagon preprint hexagon:2610.00341 v7 (2026). Unrefereed. Mentioned only in §6.3 and §6.5, and not used.
