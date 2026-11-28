# H3, the large room: borrowed routes to a uniform gap from a finite-size criterion

*28 November 2026 (night of 27th). Research note. Nothing in the register changes because of this note. Every citation below was checked against a primary source or the paper's own landing page; where I could only see a secondary description, I say so. Tags: [cited] = statement taken from a source with hypotheses as stated there; [P] = proved here (elementary); [C] = small computation here; [guess] = mine, labelled.*

---

## 1. The precise target

H3 (line 20) is $\inf_{x>0}\delta(x)/x>0$, $x=R\Lambda$, with $\delta(x)=R\,\Delta(R)$ and $\Delta(R)$ the gap over the whole physical Hilbert space of the continuum room $S^3_{R,t}\times\mathbb R$ (H3 §1). Steps 0–4 of my H3 route reduce everything to **Step 5**:

$$\liminf_{x\to\infty}\delta(x)/x>0,\qquad\text{i.e.}\qquad \Delta(R)\ge m_\infty>0 \text{ for all } R\ge R_1 .$$

My Step 5 has two halves, and I keep them separate here.

- **(5-imp) The implication.** A one-scale mixing condition, holding at a fixed physical scale uniformly in the UV cutoff, implies exponential decay of covariances in physical units, uniformly in the cutoff and in $R\ge R_1$; then reflection positivity (lines 2, 22, and the lemma of line 28) turns decay in Euclidean time into $\Delta(R)\ge m_\infty$.
- **(5-ver) The verification.** The condition actually holds at one scale $\rho_*=y_*/\Lambda$.

My condition as written (H3 §4, 5b) is $\mathrm{MS}(\rho,L,\alpha)$: in a 4-cube $Q$ of side $L\rho$ tiled by $L^4$ blocks of side $\rho$, with $\gamma_b$ the worst-case total-variation influence of boundary block $b$ on the $\rho$-smoothed gauge-invariant observables of the **central** block,
$$\textstyle\sum_{b\subset\partial Q}\gamma_b\le\alpha<1\quad\text{uniformly in the UV cutoff.}$$

The question for tonight: which established finite-size criterion is closest to MS, which theorem would give (5-imp), and what would it take to check (5-ver) in one box.

---

## 2. What others have done (verified)

### 2.1 Dobrushin–Shlosman: the constructive uniqueness criterion $C_V$

**Source.** R. L. Dobrushin and S. B. Shlosman, "Constructive criterion for the uniqueness of Gibbs field", in *Statistical Physics and Dynamical Systems: Rigorous Results* (J. Fritz, A. Jaffe, D. Szász, eds.), Birkhäuser, Boston, 1985, pp. 347–370. I could not get the chapter itself. I take the statement from the restatement by Shlosman (one of the two authors), *The critical temperature $T_{cr}$(Ising) is DS-computable*, arXiv:2505.24750 (v2, 3 Dec 2025), §1.1, which I read in full.

**Statement as restated there.** For a finite-range lattice spin system on $\mathbb Z^d$, a finite box $V$, and a boundary site $y\in\partial V$: put a metric $\rho$ on the single-spin space and the sum metric $\rho_V(\sigma_V,\bar\sigma_V)=\sum_{x\in V}\rho(\sigma_x,\bar\sigma_x)$ on $\Omega_V$, and let $d_K$ be the Kantorovich distance for $\rho_V$. Define
$$k_{V,y}=\max_{\sigma_y\ne\sigma_y';\,\sigma_{\partial V\setminus y}}\frac{d_K\big(q_V(\cdot\mid\sigma_{\partial V\setminus y}\cup\sigma_y),\,q_V(\cdot\mid\sigma_{\partial V\setminus y}\cup\sigma'_y)\big)}{\rho(\sigma_y,\sigma'_y)} .$$
*Definition 1* ([DS]): the interaction satisfies $C_V$ if $\sum_{y\in\partial V}k_{V,y}<|V|$.
*Theorem 2* ([DS]): if $C_V$ holds for some finite $V$, the Gibbs state is unique and has exponentially decaying correlations.
For $|V|=1$ this is Dobrushin's single-site condition.

**Sharpness, from the same source.** Shlosman's Theorem 4: for the $d$-dimensional nearest-neighbour Ising model, $\sup_V\beta_V(d)=\beta_{cr}(d)$. So the one-box conditions $C_V$ cover the **entire** high-temperature phase. (Preprint; arXiv only, as far as I could see.)

**Hypotheses I must respect.** Finite range; on $\mathbb Z^d$; the restatement does not say whether the 1985 theorem needs a compact or finite spin space, or translation invariance, and I have not seen the original. Spins are site variables. There is no gauge symmetry and nothing uniform in a lattice spacing.

### 2.2 Dobrushin–Shlosman: complete analyticity is constructive

**Source.** R. L. Dobrushin and S. B. Shlosman, "Completely analytical interactions: constructive description", *J. Stat. Phys.* 46 (1987) 983–1014, DOI 10.1007/BF01011153.
**Statement (abstract level).** Twelve conditions (analyticity of partition functions, decay of correlations), each **constructive**: checking it in all finite volumes can be replaced by checking a finite collection of volumes of bounded size. The setting: $\mathbb Z^\nu$, finite single-spin space, finite-range, translation-invariant, finite-valued interactions. Dobrushin and Warstat extended this to interactions with infinite values (*Probab. Theory Relat. Fields*, 1990; I did not confirm the volume).
**For me:** the template for "the whole of infinite volume is decided in one room". Finite spin space, so not directly SU(2).

### 2.3 Martinelli–Olivieri: strong mixing and its finite-size (effective) form

**Sources.** F. Martinelli and E. Olivieri, "Approach to equilibrium of Glauber dynamics in the one phase region. I. The attractive case", *Commun. Math. Phys.* 161 (1994) 447–486, and "II. The general case", same volume, 487–514. Precise statements taken from F. Martinelli, "Lectures on Glauber dynamics for discrete spin models", Saint-Flour 1997, *Lecture Notes in Math.* 1717 (1999) 93–191 (mp_arc 97-578; I read the text).

**Definitions (Martinelli, §2.3–2.4).** With $\mu^\tau_{V,\Delta}$ the projection to $\Delta$ of the Gibbs measure in $V$ with boundary condition $\tau$:
- $\mathrm{SM}(V,C,m)$: for every $\Delta\subset V$ and every $y\in V^c$, $\sup_\tau\|\mu^\tau_{V,\Delta}-\mu^{\tau^y}_{V,\Delta}\|\le Ce^{-m\,d(\Delta,y)}$.
- $\mathrm{SMT}(V,l,\gamma)$: for all local $f,g$ with $d(\Lambda_f,\Lambda_g)\ge l$, $\sup_\tau|\mu^\tau_V(f,g)|\le|\Lambda_f||\Lambda_g|\,\|f\|_\infty\|g\|_\infty\,e^{-\gamma d(\Lambda_f,\Lambda_g)}$.

**Theorem 2.7.** SM with uniform constants for all multiples of $Q_{L_0}$ is equivalent to SMT with uniform constants for all multiples of $Q_{L_0}$.
**Proposition 2.9 (effectiveness, the finite-size criterion).** Given $\gamma>0$ there are $\bar l(d,r,\gamma,\|J\|)$, $\gamma_1(d,r,\|J\|)$, $\hat m(d,r,\gamma)$ such that, if $l>\bar l$ and $\mathrm{SMT}(V,l/2,\gamma)$ holds for all $V\in\mathcal F'_l$ (the unions of $l$-cubes inside the ball $B_{2l}$), then $\mathrm{SMT}(V,\gamma_1 l,\hat m)$ holds for **all** $V\in\mathcal F_l$.
**Proposition 2.11 (no intermediate decay rates).** If, for all multiples of $Q_{L_0}$, covariances decay like $\psi(n)$ with $n^{2(d-1)}\psi(n)\to0$, then exponential SMT holds.
**Warning in the notes.** Strong mixing can hold for regular volumes (multiples of a cube) and fail for others. This forced a revision of complete analyticity.

**Hypotheses.** Discrete (finite) spin space; finite range $r$; the constants depend on the interaction norm $\|J\|$. **This is the obstruction to uniformity in the lattice spacing:** along a continuum tuning $\|J\|\sim\beta\to\infty$, so $\bar l$ is not uniform. The criterion has to be applied to a block-scale system with bounded effective interaction, which is the hard part.

### 2.4 Stroock–Zegarlinski: compact continuous spins

**Source.** D. W. Stroock and B. Zegarlinski, "The equivalence of the logarithmic Sobolev inequality and the Dobrushin–Shlosman mixing condition", *Commun. Math. Phys.* 144 (1992) 303–323, DOI 10.1007/BF02096629.
**Statement (from the abstract and introduction; I could not open the full text).** For a finite-range lattice gas whose single-spin space is a compact, connected, finite-dimensional Riemannian manifold, a uniform logarithmic Sobolev inequality holds **if and only if** the Dobrushin–Shlosman mixing condition holds (their Theorem 1.2). I have not read the exact form of their (1.4) and (1.5), so I do not quote them.
**For me:** $SU(2)\cong S^3$ is such a manifold, so the single-link spin space is covered. Gauge symmetry and uniformity in $a$ are not.

### 2.5 Weitz: block criteria on any graph

**Source.** D. Weitz, "Combinatorial criteria for uniqueness of Gibbs measures", *Random Structures & Algorithms* 27(4) (2005) 445–475.
**Statement (secondary: I read it only through T. Pedersen, "Dobrushin conditions for systematic scan with block dynamics", arXiv:math/0703461, which uses Weitz's notation).** There is a collection of blocks $\Theta_k$ covering the sites. Let $\rho^k_{i,j}$ be the influence of site $i$ on site $j$ through block $k$: the worst-case total-variation change of the block-$k$ heat-bath marginal at $j$ when the configuration changes only at $i$. Weitz's parameter is
$$\alpha_W=\max_j\frac{1}{b(j)}\sum_{k\in B(j)}\sum_i\rho^k_{i,j},$$
where $B(j)$ is the set of blocks containing $j$ and $b(j)=|B(j)|$. Pedersen states that Weitz proves that $\alpha_W<1$ implies spatial mixing and uniqueness. Weitz states conditions for general metrics, on **any underlying graph**, and also gives the dual condition "influence *of* a site is small", the Dobrushin–Shlosman type.
**For me:** the right shape for a lattice on the Berger sphere, which is not $\mathbb Z^4$ and not translation invariant. The spin-space generality (finite or compact) I have not confirmed.

### 2.6 Chatterjee: worst-case boundary decay is a gauge-theory notion, and it forces confinement

**Source.** S. Chatterjee, "A probabilistic mechanism for quark confinement", *Commun. Math. Phys.* 385 (2021) 1007–1039, arXiv:2006.16229 (I read v5).
**Definition 2.3 (verbatim in content).** The theory satisfies *exponential decay of correlations under arbitrary boundary conditions* if there are $K_1,K_2>0$ depending only on $G,\beta,d$ such that for any cube $B$, any boundary condition $\delta$ (an assignment of group elements to the boundary edges of $B$), and any local $f,g$ supported on edges in $B$ with values in $[-1,1]$, $|\langle fg\rangle_{B,\delta}-\langle f\rangle_{B,\delta}\langle g\rangle_{B,\delta}|\le K_1e^{-K_2\operatorname{dist}(f,g)}$.
**Theorem 2.4.** For a lattice gauge theory on $\mathbb Z^d$ with compact connected $G$, if Definition 2.3 holds then center symmetry is unbroken. If Wilson loops use an irreducible representation acting nontrivially on the center, every Gibbs measure satisfies the area law for rectangular loops.
**His remarks.** Definition 2.3 holds at small $\beta$ "using the methods of Dobrushin and Shlosman". It is "not clear" whether the expected large-$\beta$ mass gap includes this strong version. It is strictly stronger than the transfer-matrix mass gap, which concerns one boundary condition.

### 2.7 Shen–Zhu–Zhu: Bakry–Émery for lattice Yang–Mills (strong coupling)

**Source.** H. Shen, R. Zhu and X. Zhu, "A stochastic analysis approach to lattice Yang–Mills at strong coupling", *Commun. Math. Phys.* 400 (2023) 805–851, arXiv:2204.12737.
**Statement (abstract).** For $d>1$, with 't Hooft scaling, for $|\beta|<\frac{1}{16(d-1)}$ (SU(N)) or $|\beta|<\frac{N-2}{32(d-1)N}$ (SO(N)): the Langevin dynamics has a unique invariant measure on the whole lattice, it is exponentially ergodic in a Wasserstein distance, the infinite-volume measure satisfies log-Sobolev and Poincaré inequalities, a large class of observables have exponentially decaying correlations, and there is a positive mass gap.
**Mechanism.** The Ricci curvature of the group beats the Hessian of the action (Bakry–Émery on the product manifold $G^{E}$).
**For me:** the only *continuous-group*, *gauge-invariant-without-fixing* functional-inequality result I found. It is strong coupling only, which (H3 §3.5 correction) is never the large-$R$ end of my continuum family.

### 2.8 Bauerschmidt–Bodineau: the multiscale Bakry–Émery criterion

**Sources.** R. Bauerschmidt and T. Bodineau, "Log-Sobolev inequality for the continuum sine-Gordon model", *Comm. Pure Appl. Math.* 74 (2021) 2064–2113, arXiv:1907.12308. R. Bauerschmidt, T. Bodineau and B. Dagallier, "Stochastic dynamics and the Polchinski equation: an introduction", *Probability Surveys* 21 (2024), arXiv:2307.07619 (I read v2).
**Theorem 3.6 of the survey (verbatim in content).** Let $\nu_0$ have the form $E_{\nu_0}[F]\propto E_{C_\infty}[e^{-V_0(\zeta)}F(\zeta)]$ (eq. 3.10), on a linear space $X$, with a covariance decomposition $C_\infty=\int_0^\infty\dot C_t\,dt$, $\dot C_t$ differentiable, and the continuity assumption (3.31). Let $V_t$ be the Polchinski-renormalised potential. Suppose there are real numbers $\dot\lambda_t$, which may be negative, such that for all $\varphi\in X$ and $t>0$
$$\dot C_t\,\mathrm{Hess}\,V_t(\varphi)\,\dot C_t-\tfrac12\ddot C_t\ \ge\ \dot\lambda_t\,\dot C_t ,$$
and put $\lambda_t=\int_0^t\dot\lambda_s\,ds$, $\frac1\gamma=\int_0^\infty e^{-2\lambda_t}dt$. Then $\mathrm{Ent}_{\nu_0}[F]\le\frac2\gamma E_{\nu_0}[(\nabla\sqrt F)^2_{\dot C_0}]$.
**For me:** a gap criterion that is literally an **integral along the scale coordinate**, a section over the RG axis. That is my bundle framing (H3 §3.5) in a theorem. But $X$ is a vector space and $C_\infty$ a Gaussian covariance.

### 2.9 Finite gauge groups at weak coupling

- M. P. Forsström, "Decay of correlations in finite Abelian lattice gauge theories", *Commun. Math. Phys.* 393 (2022) 1311–1346, arXiv:2104.03752. $\mathbb Z^4$, finite Abelian group, $\beta$ sufficiently large: decay of correlations of local functions.
- A. Adhikari and S. Cao, "Correlation decay for finite lattice gauge theories at weak coupling", *Ann. Probab.* 53 (2025) 140–174, arXiv:2202.10375. Finite, possibly non-Abelian, groups at weak coupling: exponential decay for a broad class of gauge-invariant functions, including functions of Wilson loops.

The mechanism is that excitations cost a fixed positive energy, so at large $\beta$ they are rare (percolation/swapping arguments). **This is exactly what fails for SU(2):** a continuous group has arbitrarily soft excitations (gluons), and the weak-coupling phase is massless in perturbation theory.

### 2.10 Area law beyond the classical strong-coupling regime

S. Cao, R. Nissim and S. Sheffield, "Expanded regimes of area law for lattice Yang–Mills theories", arXiv:2505.16585. The abstract describes U(N), all $d\ge2$, extending Osterwalder–Seiler's regime especially for large $N$, through the master loop equation. I did not confirm the explicit regime. It does not claim a mass gap or a finite-size criterion.

### 2.11 A one-cube criterion for lattice Yang–Mills (unrefereed)

**Source.** D. Bhattacharjee, U. Bhattacharya, J. M. Lockwood and T. Romanov, "One Cube Estimate Implies the Yang–Mills Mass Gap", Hexagon preprint hexagon:2610.00341, v7 (Nov 2026). This is already cited in my H3 §6. I re-read the relevant section today. **It is not peer reviewed.** The authors say themselves that the cube estimate along the continuum tuning is open and that they do not solve the Clay problem.
**Definition 42.** The condition $D(n,\alpha)$ at coupling $\beta$ is $K_c\le\alpha N_n$ for every class $c$. Here $K_c=\sum_{j\in c\setminus V}C_{V,j}$ collects the influences of links outside a cube $V$ of side $n$, and $N_n=(n/2)^4$ is the number of $2\mathbb Z^4$-translates of $V$ containing a link. In their words, "the total influence of a link on the translates that do not contain it is at most the fraction α of the number of translates that do". This is the Dobrushin–Shlosman averaged form.
**Theorem 43.** If $D(n,\alpha)$ holds at $\beta\ge0$, the transfer matrix has spectral gap at least $\log(1/\alpha)/(2n)$ in lattice units.
**Theorem 45.** If there are $\alpha<1$, $\ell>0$ and even $n_k$ with $n_ka_k\le\ell$ such that $D(n_k,\alpha)$ holds at $\beta_k$ for every $k$, then every continuum limit has $\sigma(H)\subset\{0\}\cup[m_*,\infty)$ with $m_*=\log(1/\alpha)/(2\ell)$, and $0$ is simple.
**Their own warning.** Fixed-coupling bounds with gap $\propto1/a$ give trivial limits.

---

## 3. How each aligns with my framing, and where it does not

| Source | Shape of criterion | Spins / group | Uniform in $a$? | Geometry | Alignment with MS |
|---|---|---|---|---|---|
| DS $C_V$ (2.1) | one box, sum of boundary influences $<|V|$, Kantorovich, influence on **all** of $V$ | single spin space with metric | no | $\mathbb Z^d$ | closest *shape*; averaged, not central |
| DS complete analyticity (2.2) | finite list of boxes | finite | no | $\mathbb Z^\nu$, translation inv. | the "one room decides" philosophy |
| Martinelli–Olivieri (2.3) | covariance decay in all unions of $l$-cubes in $B_{2l}$ | finite | no ($\bar l$ depends on $\|J\|$) | $\mathbb Z^d$ | closest *conclusion* (SMT = what RP needs) |
| Stroock–Zegarlinski (2.4) | DS mixing ⇔ uniform LSI | compact manifold (covers SU(2)) | no | $\mathbb Z^d$ | the continuous-spin bridge |
| Weitz (2.5) | block influences averaged over blocks containing $j$ | general metric (spin space unconfirmed) | no | **any graph** | fits a lattice on the Berger sphere |
| Chatterjee (2.6) | all cubes, all boundary conditions | compact connected $G$ | fixed $\beta$ | $\mathbb Z^d$ | MS's worst-case clause *is* his Def. 2.3; consequence: area law |
| SZZ (2.7) | Bakry–Émery, Ric + Hess | SU(N), SO(N), no gauge fixing | fixed small $\beta$ | $\mathbb Z^d$ | wrong regime |
| BB multiscale BE (2.8) | integral over scales | vector space | designed for continuum limits | torus / $\mathbb R^d$ | aligns with "room size is a base coordinate"; needs a linear field space |
| finite groups (2.9) | decay at large $\beta$ | finite $G$ | n/a | $\mathbb Z^4$ | mechanism fails for continuous $G$ |
| BBLR (2.11) | DS-averaged cube condition at fixed physical side $\ell$ | lattice YM | **yes, by design** | $\mathbb Z^4$, torus | essentially my Step 5 on flat space; unrefereed |

**What I learn, point by point.**

**(a) My central-block MS is not covered by any theorem I found.** Every working criterion (Dobrushin, DS, Weitz, BBLR) measures influence on *all* the sites of the block being resampled, either averaged over positions or in a sum metric. I see why when I try to run the argument with the central block only. DLR gives $\mu(\sigma_x\in\cdot\mid\sigma_{Q_x^c})=\mu^{\sigma_{\partial Q_x}}_{Q_x}(\sigma_x\in\cdot)$, so for two states coupled by $\pi$,
$$\|\mu_x-\nu_x\|\le\textstyle\sum_b\gamma_b\,\pi(\omega_b\ne\omega'_b).$$
To iterate I need one coupling whose disagreement at *every* boundary block is bounded by the previous step's bound at once. Such a coupling is built by resampling the whole cube and tracking disagreement at all its sites, which brings back influence on non-central sites. **So I replace MS by its averaged form MS′ (below). That is a change in my route, not in the requirement.**

**(b) The averaged form absorbs the boundary count, which was my Sub-conjecture 5.1.** In $C_V$ and $D(n,\alpha)$ the boundary sum is compared with $|V|$, not with $1$. A cube of physical side $\ell$ has $\sim(\ell/a)^3$ boundary links and $\sim(\ell/a)^4$ interior links. The criterion asks that a boundary link change, on average, fewer than about $\alpha\,\ell/a$ interior links (in the sum metric). Lemma G (§5) shows that for Gaussian fields this normalization is automatically cutoff-uniform, and that it is a statement about a **mass**. Sub-conjecture 5.1 in my old normalization asked for more than is needed.

**(c) The worst-case clause makes MS at least as strong as confinement.** On the lattice at fixed $\beta$, a criterion of complete-analyticity type (decay in all cubes, all boundary conditions, constants depending on $G,\beta,d$ only) is Chatterjee's Definition 2.3. For SU(2) (center $\mathbb Z_2$) his Theorem 2.4 then gives the area law. This **aligns** with the physics I expect (SU(2) Yang–Mills confines) and makes the programme more believable: the large-room end and confinement come out of one condition. It is also a warning about difficulty. Proving MS in one room would prove confinement at that coupling as a by-product, and Chatterjee notes it is unclear whether the expected weak-coupling gap includes this strong version. My H3 failure mode 2 (worst-case boundary data) is exactly this point.

**(d) The finite-size room is a cube with a boundary, not one of my spheres.** The one-box criteria all live on cubes with boundary data. My rooms $S^3_{R,t}$ are closed, so the small-room results (H2, the carriers, the S_C machinery) do **not** feed the criterion directly. The sphere family enters only in two places: (i) for $R\ge R_1(\ell)$ every cube of side $\ell$ in $S^3_{R,t}\times\mathbb R$ is nearly flat; (ii) once the criterion holds for all such cubes in the room, decay in the room's Euclidean time plus RP gives $\Delta(R)$. Weitz's any-graph formulation is the natural tool for (ii), because the room lattice is not translation invariant. I record honestly that the bridge from my small-room carriers to the box at $y_*$ (H3 5.2) is **not** shortened by any of these sources.

**(e) Finite-group and strong-coupling results do not transfer.** Their mechanisms (excitations with an energy gap per plaquette; Ricci curvature beating a small action Hessian) are absent at weak coupling for SU(2). They support "correlation decay" as the right language, and nothing more.

**(f) The multiscale Bakry–Émery criterion aligns in spirit and fails in hypotheses.** Its constant $\gamma^{-1}=\int_0^\infty e^{-2\lambda_t}dt$ is a functional of a section over the scale axis, as in my bundle skeleton. Using it would need gauge fixing to a linear space (with Gribov copies at large field), a Gaussian decomposition adapted to the room, and control of $\mathrm{Hess}\,V_t$ through the confinement scale. That last item is the whole problem again. I keep it as a [guess]-level alternative, not as a route.

---

## 4. The reduced route for Step 5

**MS′ (averaged form, replaces MS).** Fix the physical side $\ell$. For the lattice regularisation at spacing $a_k$, let $V$ be a lattice cube of side $n_k$ with $n_ka_k\le\ell$. Use link variables with the geodesic metric on SU(2), the sum metric on $V$, and influences $C_{V,j}$ in Kantorovich distance (gauge-fixed inside $V$ as in BBLR, or gauge-invariant; the gauge treatment is part of the open work). Require $K_c\le\alpha N_{n_k}$ for every class (BBLR's $D(n_k,\alpha)$), with $\alpha<1$ independent of $k$. For the room: the same condition for every lattice cube of physical side $\le\ell$ in $S^3_{R,t}\times\mathbb R$ with $R\ge R_1(\ell)$, in Weitz's averaged form on the room graph.

**Steps.**
1. *(flat implication)* MS′ along the tuning ⇒ every continuum limit on $\mathbb R^4$ has gap $\ge\log(1/\alpha)/(2\ell)$ and a simple vacuum. **Borrowed:** BBLR Theorems 43 and 45. **Status:** cited from an unrefereed preprint, so for my register [Cd on BBLR Thm 45]. The independent route is DS Theorem 2 + RP, but the decay rate in DS as restated is not explicit, and the explicit rate $\log(1/\alpha)/(2n)$ is BBLR's.
2. *(room implication)* MS′ on the room graph for $R\ge R_1$ ⇒ covariance decay in the room's Euclidean time at rate $\ge\log(1/\alpha)/(c\,\ell)$, uniformly in $R\ge R_1$ and the cutoff ⇒ $\Delta(R)\ge m_\infty$ via the lemma of line 28. **Borrowed:** Weitz's general-graph criterion for the iteration, if its spin-space hypotheses cover compact SU(2) (unconfirmed); otherwise redo the Dobrushin iteration on the room graph. **Status:** [guess, routine-looking]: one translation-invariance-free rerun of a known argument.
3. *(verification at one scale)* There is $\ell=y_*/\Lambda$ with MS′ uniform along the tuning. **Status:** the wall, now with its content identified by Lemma G. In any Gaussian approximation the criterion holds iff there is a mass, and in the massless (perturbative, fixed-gauge) approximation it fails *exactly* ($\alpha=1$). So (5-ver) is equivalent in content to non-perturbative mass generation inside one box of size $\ell$. Possible tools:
   - *computer-assisted check at fixed $\beta$*, as Dobrushin, Kolafa and Shlosman did for the 2-D Ising antiferromagnet (*Commun. Math. Phys.* 102 (1985) 89–103, cited from Shlosman's bibliography). This gives MS′ at one coupling and lattice; it is not uniform along the tuning, but it would give one rigorous data point for BBLR's Theorem 43;
   - *Bałaban-type RG* to carry the theory from the cutoff down to scale $\ell$, then a large-field estimate at $\ell$. That is what BBLR and H3 §5 failure mode 3 say is missing;
   - *block-scale Bakry–Émery*: if the blocked effective action at scale $\ell$ had Hessian below the Ricci bound of $SU(2)^E$, the SZZ mechanism would apply [guess; I expect the effective coupling at $\rho\Lambda=O(1)$ to be far outside SZZ's $\beta<1/(16(d-1))$ window, so I do not rate this].
4. Steps 0–4 of H3 plus 1–3 give H3, with $c$ as in H3 5d and $m_\infty=\log(1/\alpha)/(2\ell)$.

**Net effect.** (5-imp) becomes "cited + one routine rerun on the room graph", conditional on BBLR's unrefereed Theorem 45 or on an explicit-rate DS theorem. (5-ver) is unchanged in difficulty but sharpened: it is a mass statement in one box, and it implies confinement (point c).

---

## 5. The most closable sub-statement now: Lemma G (Gaussian calibration of the criterion)

**Lemma G [P].** Let $V\subset a\mathbb Z^d$ be a lattice cube with $|V|$ sites and outer boundary $\partial V$. Let $q_V(\cdot\mid\tau)$ be the conditional law in $V$ of the lattice free field with Hamiltonian $\frac12\sum_{\langle xy\rangle}(\phi_x-\phi_y)^2+\frac12(ma)^2\sum_x\phi_x^2$, given boundary values $\tau$ on $\partial V$. With $\rho(s,s')=|s-s'|$ and the sum metric, the Dobrushin–Shlosman coefficients satisfy
$$\sum_{y\in\partial V}k_{V,y}=\sum_{x\in V}u_V(x),\qquad u_V(x)=P_x\big(\text{the walk killed at rate }(ma)^2/(2d+(ma)^2)\text{ per step exits }V\text{ before it is killed}\big).$$
Hence:
(i) for $m=0$, $\sum_yk_{V,y}=|V|$ **exactly**, for every $V$ and every $a$: $C_V$ fails, with $\alpha=1$;
(ii) for $m>0$, $\alpha_V:=|V|^{-1}\sum_yk_{V,y}<1$ for every $V$;
(iii) at fixed physical side $\ell$ and $m$, as $a\to0$, $\alpha_V\to\alpha(m\ell)=\ell^{-d}\int_{[0,\ell]^d}u$, with $(-\Delta+m^2)u=0$ in the cube and $u=1$ on its boundary. This is the standard convergence of the discrete Dirichlet problem, cited not reproved, and $\alpha(m\ell)<1$. In $d=1$, $\alpha(m\ell)=\frac{2}{m\ell}\tanh\frac{m\ell}{2}$.

**Proof.** The conditional law is Gaussian with a covariance that does not depend on $\tau$ and mean $h_\tau(x)=\sum_{y\in\partial V}H_V(x,y)\tau_y$. Here $H_V(x,y)\ge0$ is the killed harmonic measure: the mean solves $(2d+(ma)^2)h(x)=\sum_{z\sim x}h(z)$ in $V$ with $h=\tau$ on $\partial V$, the Dirichlet problem of the walk that steps to each neighbour with probability $1/(2d+(ma)^2)$ and dies otherwise. Changing $\tau$ only at $y$ by $\epsilon$ shifts the mean by $\epsilon H_V(\cdot,y)$ and leaves the covariance unchanged.
*Upper bound:* the translation coupling gives $d_K\le\sum_x|\epsilon|H_V(x,y)$.
*Lower bound:* for any coupling, $E\sum_x|\sigma_x-\sigma'_x|\ge\sum_x|E\sigma_x-E\sigma'_x|=|\epsilon|\sum_xH_V(x,y)$.
So $k_{V,y}=\sum_xH_V(x,y)$, and $\sum_yk_{V,y}=\sum_x\sum_yH_V(x,y)=\sum_xu_V(x)$. For $m=0$ the walk leaves the finite set $V$ almost surely, so $u_V\equiv1$. For $m>0$, $u_V(x)\le1/(1+(ma)^2/2d)<1$ at every $x$. ∎

**Numerical check [C]** (pure-Python CG, physical side $\ell=(n+1)a$, $\alpha_V$ = mean of $u_V$; script at `/tmp/h3calc/ds_gauss2.py`, which is not part of the repo):

| $d$ | $m\ell$ | $n=4$ | $n=6$ | $n=8$ | ($d=2$: $n=8,16,32$) |
|---|---|---|---|---|---|
| 2 | 0 | | | | 1.0000, 1.0000, 1.0000 |
| 2 | 4 | | | | 0.6145, 0.6436, 0.6607 |
| 2 | 16 | | | | 0.1195, 0.1577, 0.1883 |
| 4 | 0 | 1.0000 | 1.0000 | 1.0000 | |
| 4 | 4 | 0.7348 | 0.7604 | 0.7766 | |
| 4 | 16 | 0.1585 | 0.1899 | 0.2155 | |

$\alpha_V$ rises monotonically with $n$ towards a continuum value below 1. The rise is the boundary layer being resolved, consistent with (iii). For large $m\ell$ I expect $\alpha(m\ell)\approx2d/(m\ell)$ [guess from the boundary-layer picture], so $\log(1/\alpha)/(2\ell)\sim\log(m\ell/2d)/(2\ell)$.

**What Lemma G closes, and what it does not.**
- It closes, for the Gaussian template, the uniform-in-cutoff part of my Sub-conjecture 5.1 *in the averaged normalization*: a fixed physical cube gives a cutoff-independent $\alpha$ because harmonic measure sums to at most one. The $(\rho/a)^3$ boundary count is cancelled by the $(\rho/a)^4$ volume normalization. My old central-block normalization never needed this, but it has no iteration theorem behind it (§3a).
- It proves a clean negative: **no fixed-gauge perturbative (massless Gaussian) approximation can verify MS′**. Such an approximation sits exactly on the boundary $\alpha=1$, so (5-ver) is irreducibly a non-perturbative mass statement at scale $\ell$. [guess: the same saturation holds for the free Maxwell field in a fixed gauge, link by link; I have not written that out.]
- It does not touch Yang–Mills directly, and it does not prove H3.

**Deduction this enables (conditional chain, for the register owner to judge).**
H3 Steps 0–4 + [Cd: BBLR Thm 45, or an explicit-rate DS theorem] + Step 2 rerun on the room graph + MS′ at one scale ⇒ H3, with $m_\infty=\log(1/\alpha)/(2\ell)$. The single remaining mathematical hypothesis is MS′ at one physical scale, and Lemma G says its content is a mass in one box.

---

### References (all checked tonight)

1. R. L. Dobrushin, S. B. Shlosman, "Constructive criterion for the uniqueness of Gibbs field", in *Statistical Physics and Dynamical Systems: Rigorous Results*, Birkhäuser, 1985, 347–370. (Statement via ref. 2.)
2. S. Shlosman, "The critical temperature $T_{cr}$(Ising) is DS-computable", arXiv:2505.24750 (v2, 2025).
3. R. L. Dobrushin, S. B. Shlosman, "Completely analytical interactions: constructive description", *J. Stat. Phys.* 46 (1987) 983–1014, doi:10.1007/BF01011153.
4. F. Martinelli, E. Olivieri, "Approach to equilibrium of Glauber dynamics in the one phase region. I / II", *Commun. Math. Phys.* 161 (1994) 447–486 / 487–514.
5. F. Martinelli, "Lectures on Glauber dynamics for discrete spin models", *Lecture Notes in Math.* 1717 (1999) 93–191 (Saint-Flour 1997; mp_arc 97-578).
6. D. W. Stroock, B. Zegarlinski, "The equivalence of the logarithmic Sobolev inequality and the Dobrushin–Shlosman mixing condition", *Commun. Math. Phys.* 144 (1992) 303–323, doi:10.1007/BF02096629.
7. D. Weitz, "Combinatorial criteria for uniqueness of Gibbs measures", *Random Structures Algorithms* 27(4) (2005) 445–475; read via T. Pedersen, arXiv:math/0703461.
8. S. Chatterjee, "A probabilistic mechanism for quark confinement", *Commun. Math. Phys.* 385 (2021) 1007–1039, arXiv:2006.16229.
9. H. Shen, R. Zhu, X. Zhu, "A stochastic analysis approach to lattice Yang–Mills at strong coupling", *Commun. Math. Phys.* 400 (2023) 805–851, arXiv:2204.12737.
10. R. Bauerschmidt, T. Bodineau, "Log-Sobolev inequality for the continuum sine-Gordon model", *Comm. Pure Appl. Math.* 74 (2021) 2064–2113, arXiv:1907.12308.
11. R. Bauerschmidt, T. Bodineau, B. Dagallier, "Stochastic dynamics and the Polchinski equation: an introduction", *Probability Surveys* 21 (2024), arXiv:2307.07619.
12. M. P. Forsström, "Decay of correlations in finite Abelian lattice gauge theories", *Commun. Math. Phys.* 393 (2022) 1311–1346, arXiv:2104.03752.
13. A. Adhikari, S. Cao, "Correlation decay for finite lattice gauge theories at weak coupling", *Ann. Probab.* 53 (2025) 140–174, arXiv:2202.10375.
14. S. Cao, R. Nissim, S. Sheffield, "Expanded regimes of area law for lattice Yang–Mills theories", arXiv:2505.16585.
15. D. Bhattacharjee, U. Bhattacharya, J. M. Lockwood, T. Romanov, "One Cube Estimate Implies the Yang–Mills Mass Gap", Hexagon preprint hexagon:2610.00341 v7 (2026), unrefereed.
16. R. L. Dobrushin, J. Kolafa, S. B. Shlosman, "Phase diagram of the two-dimensional Ising antiferromagnet (computer-assisted proof)", *Commun. Math. Phys.* 102 (1985) 89–103 (bibliographic data from ref. 2).
