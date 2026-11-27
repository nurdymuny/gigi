# H2: M3, uniform dressing at small $R$ (line 16)

*Closed set, 27 November 2026. My status labels: [P] proved here or on the page; [C] computed (not proved); [elem] an elementary argument I give in full here; [guess] conjecture, with the evidence stated. Sources: Part IV of the report (`proof/part4.tex`), bundle-framing note §§8–10, 14; Part V (`proof/part5.tex`).*

**Units.** Energies are in units of $1/R$ and in *reduced* units (Yang–Mills energies divided by $2$; Part IV checks this exactly). A homogeneous field is labelled by $q\in\mathbb R^{3\times3}$ as on line 8, with $F=2gq$. A slice eigenvalue $\mu$ (Yang–Mills Hessian) gives the reduced frequency $\sqrt\mu/2$. I write $u=t^{-1/2}$, so the slow frequencies at $q=0$ are $\omega=(\sqrt t,u,u)$.

---

## 1. Statement (high fidelity)

### 1.1 Objects

- **Parameters.** $t$ ranges over a compact set $T\subset(0,\infty)\setminus\{1\}$ (I explain in §5 why $t=1$ must be excluded). $\theta=0$. $s=1$: the unsourced theory. That is the case line 20 uses. The sourced case is §1.4.
- **Symmetry.** $I_t=SU(2)_L\times U(1)_R$, the isometries of $S^3_t$ that commute with left translations, extended by the discrete reflections. A *sector* $\sigma$ is a joint irreducible label: $SU(2)_L$ spin, $U(1)$ charge $n\in\mathbb Z$, and parity. Line 17's charge is the $U(1)$ label.
- **Slow space.** $\mathcal H_{\rm slow}=L^2(\mathbb R^9,d^9q)^{SO(3)}$, the colour-invariant functions with the flat measure. Working on $\mathbb R^9$ with invariant functions avoids the orbifold strata and the Jacobian $\prod|\lambda_i+\lambda_j|$ (line 7). It needs no boundary conditions.
- **Chart.** $U=U(g,f_0)=\{q:\ 2g|q|\le f_0\}$, with $f_0>0$ fixed and small enough that (a) the fast-gap bounds of Lemma F (§4, Step 2) hold on $|F|\le f_0$, and (b) $U$ lies inside the fundamental modular region (absolute minima of $\|A\|_{L^2}$ along gauge orbits). $U$ then excludes the orbit $q_{\rm MC}SO(3)$, where $|F|=O(1)$. $H^U$ denotes Dirichlet restriction to $U$.
- **Fast data.** At $q$, $K(F)$ is the Yang–Mills Hessian plus $G_FG_F^*$ ($G_F$ the infinitesimal gauge map) on Peter–Weyl sectors $j\ge1$ (line 13). Its slice eigenvalues are $\mu_n(F)$. $\Omega(q)$ is the Gaussian ground state of the harmonic fast Hamiltonian $H_f(q)$, with energy $e_0(q)=\tfrac12E_0(2gq)$ (reduced). $\Pi(q)=|\Omega(q)\rangle\langle\Omega(q)|$ and $\bar\Pi=1-\Pi$. At $F=0$, $\omega_f(t)=\tfrac12\min_n\sqrt{\mu_n(0)}$ and $\omega_s(t)=\min(\sqrt t,u)$.
- **Window.** $0<E_*<E_{\max}(t):=\min\bigl(2\omega_f,\ \omega_f+\omega_s\bigr)$. Below $2\omega_f$ an $SU(2)_L$-invariant state cannot hold fast quanta, by Lemma p4lem:sel (a single fast quantum has spin $\ge\tfrac12$). Below $\omega_f+\omega_s$ there is no gauge-invariant state with exactly one fast quantum.
- **Regularized theory.** $H_{R,\Lambda}$ is the Yang–Mills Hamiltonian on $S^3_t$ of radius $R$, acting on gauge-invariant states, with a gauge-invariant spectral ultraviolet cutoff $\Lambda$. Its *admissibility* means: self-adjoint and bounded below; it commutes with $I_t$ and with the degree-1 gauge map; it is real at $\theta=0$; inside the Coulomb chart over $U$ its fast modes are cut off by a smooth function of the background-covariant operator $K(F)$, $\chi(K(F)/\Lambda^2R^2)$, never by sector index (Proposition p4prop:flat shows a sector cutoff is not gauge invariant). $E_{\rm vac}(\Lambda)=\inf\operatorname{spec}H_{R,\Lambda}$. The global definition of $H_{R,\Lambda}$ is H5's business (line 29). H2 uses only its form in the chart.
- **Renormalization prescription.** The bare coupling runs as
$$\frac1{g_0^2(\Lambda)}=\frac1{g^2(R)}+\frac{b_0}{8\pi^2}\log(\Lambda R)+r(\Lambda R,g(R)),\qquad b_0=\tfrac{22}{3},$$
with $r$ fixed by one renormalization condition at scale $1/R$ (the zeta scheme at $\mu R=1$ of line 14, extended beyond one loop; $r$ contains the two-loop $\log\log$). *If the regulator breaks four-dimensional covariance* (a purely spatial cutoff does), a second bare parameter $c_0(\Lambda)=1+O(g_0^2)$ is needed: the ratio of the electric to the magnetic coefficient. It is fixed by requiring the Euclidean theory on $S^3_R\times\mathbb R$ to be the covariant one (see Flag). $R\Lambda_{\rm YM}=\exp(-8\pi^2/b_0g^2(R))$ as on line 20.
- **Sign reading of line 14.** With $\mathcal E(\epsilon)\supset C(F)\log\epsilon$ and $\epsilon\propto1/\Lambda$ (Theorem p4thm:uv), the $q$-dependent divergence of the dressing is $-\frac{b_0}{8\pi^2}\log(\Lambda R)\,\mathcal S(F)$. The growth of $1/g_0^2$ above cancels it. I read line 14's "$+$" with this sign, which is the asymptotically free one.
- **Effective Hamiltonians on $\mathcal H_{\rm slow}$.**
$$H_{\rm red}=\tfrac12|p|^2+V(q),\qquad H^{(1)}_{\rm eff}=H_{\rm red}+\tfrac12E^{\rm ren}_0(2gq)+\Phi(q)\quad(\text{Part IV, Def. p4def:bo, with }\mathcal A=0),$$
$$H^{(2)}_{\rm eff}=\tfrac12\,p_i\bigl(\delta_{ik}-2m^{\rm ren}_{ik}(q)\bigr)p_k\;+\;V(q)+\tfrac12E_0^{\rm ren}(2gq)+\Phi^{\rm ren}(q),$$
with symmetric ordering and
$$m_{ik}=m^{\rm NA}_{ik}+m^{\rm C}_{ik},\qquad m^{\rm NA}_{ik}=\operatorname{Re}\langle\partial_i\Omega|\bar\Pi(H_f-e_0)^{-1}\bar\Pi|\partial_k\Omega\rangle=4g^2\,\operatorname{Re}\langle\partial_{F_i}\Omega|\bar\Pi(H_f-e_0)^{-1}\bar\Pi|\partial_{F_k}\Omega\rangle .$$
$m^{\rm NA}\ge0$ is the second-order non-adiabatic (mass) correction. $m^{\rm C}$ is the Coulomb term. It comes from solving the fast Gauss law: the slow colour charge enters the sector-$j\ge1$ constraint through $[a_{\rm fast},E_{\rm slow}]$, and its $\Omega$-expectation is $-\tfrac12p\cdot(\ldots)\cdot p$, with $(-d^*D_{A_q})^{-1}$ inside. "ren" means the log-divergent parts are removed by the prescription above.

### 1.2 H2, precise form (unsourced, around the vacuum)

There exist $g_*>0$ and $C<\infty$, depending only on $T$, $E_*$ and $f_0$, such that the following holds for every $R$ with $0<g(R)\le g_*$, every $t\in T$ and every sector $\sigma$:

**(i) Existence.** The eigenvalues of $R\,(H_{R,\Lambda}-E_{\rm vac}(\Lambda))$ in $[0,E_*]\cap\sigma$, counted with multiplicity, converge as $\Lambda\to\infty$ (with $g_0(\Lambda)$, and if needed $c_0(\Lambda)$, run as above). Call the limits $e^\sigma_n(R)$.

**(ii) Comparison, as worded on line 16 (H2a).** Let $\varepsilon^\sigma_n$ be the eigenvalues of $H^{U}_{\rm red}-\inf H^U_{\rm red}$ in $[0,E_*]\cap\sigma$, for $SU(2)_L$-invariant $\sigma$. The two lists have the same length, and $|e^\sigma_n(R)-\varepsilon^\sigma_n|\le C\,g^2(R)$. The same bound holds with $H^{(1)}_{\rm eff}$ or $H^{(2)}_{\rm eff}$ in place of $H_{\rm red}$, since the differences are $O(g^2)$ on the window. The bound for $H_{R,\Lambda}$ at finite $\Lambda$ holds uniformly in $\Lambda\ge\Lambda_0(R)$.

**(ii$'$) Strong comparison (H2b).** The same as (ii), with $H^{(2)}_{\rm eff}$ and error $o(g^2(R))$ (my guess: $O(g^3)$).

**(iii) Nothing else in the window.** In every sector that is not $SU(2)_L$-invariant, every eigenvalue of $R(H_{R,\Lambda}-E_{\rm vac})$ is $\ge E_*$ for $\Lambda\ge\Lambda_0(R)$.

**Order of limits.** Fix $t$, then $R$ (equivalently $g(R)$). Bound uniformly in $\Lambda\ge\Lambda_0(R)$. Take $\Lambda\to\infty$. Only then consider $g(R)\to0$. $R$ enters only through $g(R)$ and the overall scale $1/R$, by classical scale invariance. "Uniformly" ranges over $\Lambda\ge\Lambda_0(R)$, $t\in T$, the sectors $\sigma$, and the finitely many levels in the window.

### 1.3 Why this reading, and not the literal one

Line 16 as written, $\|P_{\rm low}(H_{\Lambda}-H_{\rm eff})P_{\rm low}\|\le Cg^2$, compares operators on different spaces. The natural reading inserts $J_\Lambda:\psi\mapsto\psi(q)\,\Omega_\Lambda(q)$. Then $J^*_\Lambda H_{R,\Lambda}J_\Lambda=H^{(1)}_{\rm eff}+\langle\Omega|W_4|\Omega\rangle$ identically. The Berry term vanishes because $\Omega$ is real, $\tfrac12\sum\|\partial_i\Omega\|^2=\Phi$ because $\langle\Omega|\partial\Omega\rangle=0$, and cubic expectations vanish in a Gaussian. So the literal compression bound is nearly a definition. It does not control eigenvalues, because it ignores the off-diagonal block $\bar\Pi H\Pi=O(g)$, whose second-order effect is $O(g^2)$: the term $m$. The deductions on the page use eigenvalues and sector labels, so I state H2 spectrally, as (i)–(iii).

### 1.4 The sourced extension (lines 12, 15, 18 at $s\ne1$)

At $s\neq1$ the sources $-\tfrac12\sum_k{\rm src}_k\sum_c q_{kc}^2$ are defined only on homogeneous fields. A local gauge-invariant lift to the full theory is not given on the page. H2 at the background $q_{\rm bg}=s\,q_{\rm MC}$ therefore needs, first, a choice of lift. Given one, the statement is §1.2 with $U$ a neighbourhood of $q_{\rm bg}$, $\omega_f$ the fast frequency at $F_{\rm bg}$, and a hypothesis that the fast slice spectrum at $F_{\rm bg}$ is positive (Conjecture p4conj:m3 assumes this).

---

## 2. Interface

**What H2 takes from the page.**
- Line 13: the exact Peter–Weyl split, and slow sector = $H_{\rm red}$.
- Line 14: the log divergence of $E_0$ is the running coupling [C].
- Line 9 / Theorem A at $s=1$ ($\kappa=\min(t,1/t)$): $H_{\rm red}$ has discrete spectrum, so the window holds finitely many levels.
- Line 10 / the Floor method: sector-by-sector lower bounds of $K(F)$ (Lemma F).
- Line 17: $I_t$-sectors are preserved, provided the cutoff is $I_t$-invariant. That is part of admissibility.
- Part V: the barrier and Agmon length, used for localization in $U$.

**What the page takes from H2.**
- **Line 20 (H3), small-$R$ end.** H3 needs $\Delta(R)R\ge c_0>0$ for $R\le R_*$. This follows from (ii) and (iii): $\Delta(R)R\ge\min(\varepsilon_1,E_*)-Cg^2$, with $\varepsilon_1$ the first excitation of $H^U_{\rm red}$. Then $\Delta/\Lambda_{\rm YM}=\Delta R/(R\Lambda_{\rm YM})\ge c_0\,e^{8\pi^2/b_0g^2}\to\infty$. *The small-$R$ end of H3 is easy given H2a.* H2 is the entrance to H3, not the wall.
- **Line 18 / H8 (carrier at $s=1$).** This needs **H2b**, not H2a. At $q=0$ with $t>1$, the lowest colour-singlet excitations are the two-quantum states in rows $2,3$ at $2u$. Charge $0$ is $q_2\!\cdot q_2+q_3\!\cdot q_3$; charge $\pm2$ is $q_2\!\cdot q_2-q_3\!\cdot q_3$ and $2q_2\!\cdot q_3$. These are *degenerate at harmonic order* [elem; also H8 §4, Step 4]. The order $0^{++}$ vs $2^{++}$ is decided at $O(g^2)$, the same order as H2a's error. It needs H2b, with $H^{(2)}_{\rm eff}$ including $m$.
- **Line 15.** The computed widening $0.0165\to0.0191$ at $g^2=0.002$ is the potential part of the $O(g^2)$ shift. The kinetic part enters at the same order:
$$\delta\omega_k=\frac{g^2\,u_k\!\cdot\mathrm{Hess}_FE_0\cdot u_k}{\omega_k}\;-\;m^{\rm ren}_{u_ku_k}(q_{\rm bg})\,\omega_k .$$
The second term has not been computed. Until it is, line 15 does not settle the sign of the $O(g^2)$ change in the separation. Its scheme freedom has two parameters: $\kappa$ (Proposition p4prop:shift) and the finite part of $c_0$.
- **Line 19 (source bridge).** By my deduction below, H2 at the sourced background cannot be continued to $s=1$. As $s\to1^+$ the background approaches $q_{\rm MC}$, a reducible gauge copy of $0$. There the Faddeev–Popov operator has kernel in sector $2$ (the stabilizer $g^{-1}SU(2)g$), so $\gamma(s):=\inf\operatorname{spec}(G^*_FG_F)|_{j\ge1}^{1/2}\le C|s-1|$. In the $s=1.15\to1$ continuation, H2's constants grow like a negative power of $\gamma$; I expect them to come in through $m^{\rm C}\ni(-d^*D_A)^{-1}$ [guess]. So line 20 must start at $s=1$ around $q=0$, as worded, and use H2 there. H8 reaches the same conclusion independently (avoided crossing, its Interface item 2).
- **Line 27 (unique vacuum) at small $R$.** In the sphere scheme this follows from (ii) and the uniqueness of the ground state of $H^U_{\rm red}$ (Perron–Frobenius for a real Schrödinger operator on the connected $U$). The page uses the lattice transfer matrix instead. H2 gives an independent check.

---

## 3. What is known

**Proved (page or here).**
- Peter–Weyl split and unit map (line 13) [P].
- The Berry connection is flat: $\mathcal A\equiv0$ on simply connected regions where $\Omega$ is non-degenerate (Lemma p4lem:berry) [P]. Wherever all $\mu_n>0$, the Gaussian ground state is unique. On a star-shaped $U$ the line bundle is therefore trivial. It is $SO(3)$-equivariant with trivial character, because $SO(3)$ has no non-trivial one-dimensional characters. Slow wavefunctions are honest invariant functions [elem].
- Gauge equivalence of $\lambda=0,1$, and the regulator must be spectral (Proposition p4prop:flat, Corollary p4cor:minus2: fast shift $-2$ Yang–Mills, $-1$ reduced, at $t=1$) [P].
- Selection rule (Lemma p4lem:sel) [P].
- **No slow–fast linear coupling** [elem]. At any homogeneous $A_q$, $\nabla E[A_q]$ is $SU(2)_L$-invariant, so it has zero component in every sector $j\ge1$. The energy has no term linear in the fast field at any $q$. All slow–fast coupling is through $K(F)$ and higher.
- **Gauge non-degeneracy at $F=0$** [elem]. In sector $j$ the scalar Laplacian on $S^3_t$ is $4J^2+4(t^{-1}-1)J_1^2=j(j+2)+(t^{-1}-1)\,4m^2$ with $|m|\le j/2$. Hence it is $\ge j^2/t+2j$ for $t\ge1$ and $\ge j(j+2)$ for $t\le1$, so $\gamma(0)^2\ge\min(2+1/t,\,3)$.
- First-order shift formula (Proposition p4prop:shift) [P]. The UV log equals the running coupling [C, $10^{-4}$].
- At $t=1$, $A_{1/2}$ and its gauge copy $-\tfrac12\,dg\,g^{-1}$ have equal $L^2$ norm, because the round metric is bi-invariant. So the Maurer–Cartan line crosses the boundary of the fundamental modular region at the sphaleron, and $\lambda>\tfrac12$ is outside it [elem].

**Literature (proven, but not this).**
- Rigorous Born–Oppenheimer for finitely many fast degrees of freedom with a uniform gap: Hagedorn's expansions (1980s); Teufel, *Adiabatic Perturbation Theory in Quantum Dynamics*, LNM 1821 (2003); Panati, Spohn and Teufel (2003), space-adiabatic perturbation theory.
- Ultraviolet stability of lattice Yang–Mills effective actions in a finite box by multiscale renormalization: Bałaban, Commun. Math. Phys. (1983–89).
- Finite-volume continuum construction with a gauge-covariant regularization: Magnen, Rivasseau and Sénéor, Commun. Math. Phys. 155 (1993).
- Gribov copies: Gribov, Nucl. Phys. B139 (1978) 1; Singer, Commun. Math. Phys. 60 (1978) 7.
- Coulomb-gauge Hamiltonian ordering terms: Christ and Lee (1980).

**Expected, not proved (templates).**
- Lüscher, Nucl. Phys. B219 (1983) 233: the torus effective Hamiltonian, to fixed order in perturbation theory.
- Koller and van Baal (1988): torus, non-perturbative in the zero-mode sector.
- van Baal and collaborators on $S^3$ (e.g. van Baal and Hari Dass, 1992; van den Heuvel, 1996): a reduction to the 18 lowest modes (left- and right-invariant), containing both vacua and the sphaleron. I cite these by author and year and rely on no detail.

None of these is a cutoff-uniform spectral theorem for an infinite-dimensional fast system with marginal couplings. **That is the open content of H2.**

---

## 4. Best-guess proof

**Step 1. Localization in the chart.** [guess, standard in finite dimensions]
*Claim.* Eigenvectors of $H_{R,\Lambda}$ with energy $\le E_{\rm vac}+E_*/R$ have mass $O(e^{-c/g^2})$ outside the Coulomb chart over $U$, uniformly in $\Lambda$.
*Justification.* For the slow variable, Agmon estimates with $V$. Part V gives the barrier $(1+2/t)/(32g^2)$ and an Agmon length $\asymp1/g^2$. $V\ge\tfrac{\kappa}2\|q\|^2$ near $0$ by Theorem A. For the fast fields, the needed input is a lower bound on the magnetic energy of large-amplitude fast configurations, uniform in $\Lambda$. That is a large-field estimate of Bałaban type, and it is the guess here.
*Consequence.* Large-gauge ($\theta$) and Gribov effects are $O(e^{-c/g^2})$, below every power of $g$.

**Step 2. Lemma F: the fast gap, sector by sector.** [elem near $F=0$; open at $F_{\rm bg}$]
*Claim.* There are $\mu_*,c_t,C_t,\gamma_*>0$ such that for $|F|\le f_0$ and every $j\ge1$:
$$\inf\operatorname{spec}K(F)|_{j}\ \ge\ \max\!\bigl(\mu_*,\ c_tj^2-C_t\bigr),\qquad \inf\operatorname{spec}G_F^*G_F|_{\Omega^0,j}\ \ge\ \gamma_*^2 .$$
*Proof sketch.* At homogeneous $F$, $K(F)=K(0)+F\cdot L_1+F^2\cdot L_0$, with $L_1$ first order and $L_0$ zeroth order, both commuting with $SU(2)_L$. So each sector is preserved. The form bound $|\langle a,F\!\cdot\! L_1a\rangle|\le\epsilon\|\nabla a\|^2+|F|^2\epsilon^{-1}\|a\|^2$ gives $K(F)\ge(1-\epsilon)K(0)-C\epsilon^{-1}f_0^2$. $K(0)$ in sector $j$ is $\ge c_tj^2$, by ellipticity and the Casimir bound above. $K(0)>0$ because $H^1(S^3)=0$. Choose $f_0$ small. At the sourced background, $|F_{\rm bg}|=O(1)$, and this is a Floor-type certificate question: finitely many $j$ by interval arithmetic, with a closed-form tail. The Floor theorem does *not* cover $A=s\theta$ with $x>0$ (Part III).

**Step 3. One adiabatic step at fixed cutoff (Feshbach).**
Split $\mathcal H_\Lambda=\operatorname{Ran}J_\Lambda\oplus(\ldots)^\perp$. The Schur complement at energy $E$ in the window is
$$\mathcal F(E)=\Pi H\Pi-\Pi H\bar\Pi\,(\bar\Pi H\bar\Pi-E)^{-1}\bar\Pi H\Pi .$$
By Step 2 and the selection rule, $\bar\Pi H\bar\Pi-E\ge(E_{\max}-E_*)/R$ in the relevant sectors. So eigenvalues of $H$ in the window are exactly the solutions of $E\in\operatorname{spec}\mathcal F(E)$. Expanding, with $\bar\Pi H\Pi=-i\sum_i\bar\Pi(\partial_i\Pi)\,p_i+\bar\Pi W_3\Pi$ and $\partial_q=2g\,\partial_F$:
$$J^*\mathcal F(E)J=H^{(2)}_{\rm eff}+\mathcal R(E),\qquad \mathcal R=O(g^2)\cdot(\text{a function of }F)+O(g^3)\ \text{on the window}.$$
Its $q$-dependence enters only through $F=2g(q_{\rm bg}+\delta q)$, with $\delta q=O(1)$. So it moves level spacings at $O(g^3)$ or smaller. This step is honest at fixed $\Lambda$ (finite-dimensional fibres). **It is not uniform in $\Lambda$.** Power counting on $S^3$ gives roughly $j^2$ fast modes at frequency $\sim j$, and $\partial_FK$ is first order and couples each mode to $O(1)$ partners within its sector. Hence:
- $\|\bar\Pi\partial_F\Omega\|^2\sim\sum_n\omega_n^{-2}\sim\Lambda R$: linear divergence. So $\Phi=g^2(c_1\Lambda R+\ldots)$ with field-independent $c_1$ (it comes from the $F$-independent leading part of $L_1$). This is also why fast vacua at different $q$ are unitarily inequivalent as $\Lambda\to\infty$ (Shale; Part IV, concern (c)).
- $m^{\rm NA},m^{\rm C}\sim g^2\sum_n\omega_n^{-3}\sim g^2\log(\Lambda R)$: log divergences in the kinetic term.
- Individual $F$-dependent power divergences, for example in $\langle\Omega|W_4|\Omega\rangle$, must cancel in the gauge-invariant total. There is no gauge-invariant local operator of dimension $<4$ in $F$ for $\mathfrak{su}(2)$.

**Step 4. Kinetic consistency.** [guess, the key one-loop identity]
*Claim (KC).* The log-divergent parts of the kinetic and potential corrections combine so that the electric and magnetic couplings run identically. With $q$ normalized as on line 8, this is
$$2m_{ik}(q)=-\,g^2\frac{b_0}{8\pi^2}\log(\Lambda R)\,\delta_{ik}+2m^{\rm fin}_{ik}(F)+o(1)\qquad(\Lambda\to\infty),$$
uniformly for $|F|\le f_0$.
*Why this sign.* Rewriting $V=\mathcal S(F)/g_0^2$ in the renormalized variable $q_R=(g_0/g)\,q$ produces the kinetic coefficient $g_0^2/g^2$, which needs a compensating factor $1+g^2\tfrac{b_0}{8\pi^2}\log(\Lambda R)$. The free frequencies $2/R,\dots$ on $S^3$ cannot run.
*Evidence.* (a) Log divergences come from scales $1/R\ll k\ll\Lambda$, where the cutoff is invisible, so they are regulator-independent and four-dimensionally covariant. (b) The sign requires $m^{\rm C}$ (anti-screening, Coulomb) to outweigh $m^{\rm NA}\ge0$ (screening, transverse). That matches the known Coulomb-gauge decomposition of $b_0$ into a dominant anti-screening Coulomb part and a screening transverse part (Khriplovich, 1969).
*Finite parts.* The finite part of an isotropic kinetic shift is the speed-of-light renormalization $c_0$, of order $O(g^2)$. It must be fixed (§1.1, Flag), or H2b is scheme-ambiguous at its own order.

**Step 5. Multiscale, uniform in $\Lambda$.** [guess: the Bałaban step in Hamiltonian form on a curved base]
Integrate the fast modes in background-covariant spectral shells $\mathcal K_m=\{2^m\le RK(F)^{1/2}<2^{m+1}\}$, from the top shell down to $m=1$. Shell $m$ has coupling $g_m:=g(2^{-m}R)\le g(R)$, by asymptotic freedom. Write $H^{(m)}$ for the effective Hamiltonian on the slow sector plus shells below $m$. **The key estimate** is: for each $m\ge1$,
$$\Bigl\|P_{\rm low}\Bigl[H^{(m-1)}-H^{(m)}\big|_{<m}-\delta\!\left(\tfrac1{g^2}\right)_{\!m}\mathcal O_B-\delta c_m\,\mathcal O_E-\delta E_m\Bigr]P_{\rm low}\Bigr\|\ \le\ K\,g_m^2\,2^{-m}\,R^{-1},$$
with $K$ independent of $m$ and $\Lambda$. Here:
- $\mathcal O_B,\mathcal O_E$ are the marginal magnetic and electric operators, restricted to $<m$.
- $\delta(1/g^2)_m=\frac{b_0}{8\pi^2}\log2+O(g_m^2)$, the running; its accumulation is $r$.
- $\delta c_m=O(g_m^2)$, with $\sum_m\delta c_m$ convergent: logs are covariant (Step 4), and only cutoff-scale finite parts are left.
- $\delta E_m$ is a constant.
- The remainder is irrelevant (dimension $\ge5$), so it is suppressed by $(2^m/R)^{-1}$ on states of energy $O(1/R)$.

*Summation.* $\sum_{m\ge1}Kg_m^2\,2^{-m}R^{-1}\le Kg^2(R)/R$. Step 3 at $m=0$ (shell $j\sim1$, no running) then gives H2a, with $C=K+C_{\rm Step\,3}$. Step 3's $O(g^2)$ terms are exactly $H^{(2)}_{\rm eff}-H_{\rm red}$; what is left is $o(g^2)$, which gives H2b. (i) follows from the same bound applied between cutoffs $\Lambda<\Lambda'$: levels differ by $\le K\sum_{m\ge m(\Lambda)}g_m^22^{-m}\to0$, so they are Cauchy. (iii) follows from Step 3's resolvent bound and the selection rule.
*Evidence.* This is the shape of Bałaban's $T^4$ bounds and of Lüscher's and van Baal's perturbative reductions. For the one-loop part, line 14 [C] and the computed convergence of the fast sums at jmax 200 (bundle note §14) are consistent with it. Nothing beyond one loop has been checked.

---

## 5. Where it can fail

1. **The window is empty at $t=1$ and thin near it.** At $t=1$ the fast fibre contains the eigenvalue $4$ in sector $2$ (Proposition p4prop:empty), so $\omega_f=\omega_s=1$ and $2\omega_f=2$. That is exactly the lowest gauge-invariant excitation (two quanta). H2 says nothing about excitations there, and van Baal's 18-mode slow space is needed. For $t$ near $1$, constants blow up like $(\omega_f-\omega_s)^{-1}$. Whether $E_{\max}(3)$ lies above the carriers' levels is a finite spectral question at $F=0$ on the Berger sphere. I have not seen it recorded. *Observed:* states with two fast quanta, or one fast and one slow quantum, inside the window, with no $H_{\rm eff}$ counterpart.
2. **Spurious second well (Gribov / reducible copy).** On all of $\mathbb R^9$, $H^{(1)}_{\rm eff}$ has a second well at $q_{\rm MC}$. By Corollary p4cor:minus2 its dressed one-loop ground energy *equals* that of $q=0$, while the physical $\theta=0$ vacuum is unique. At $t=1$ that well also carries a single-quantum colour-singlet level (the trace mode, reduced frequency $1$). Free Yang–Mills on $S^3$ has no level at $1$ (the lowest is $2$, two gluons). That is why H2 is stated on $U$. The underlying cause: $q_{\rm MC}$ is reducible with stabilizer in sector $2$, so the fast Gauss law there couples slow and fast variables. *Observed:* near-degenerate charge-0 doublets shrinking like $e^{-d/g^2}$ (H8, Interface item 2).
3. **KC fails.** If the kinetic and potential logs do not match, level spacings carry $g^2\log(\Lambda R)$ and no uniform bound exists. If they match but $c_0$ is not tuned, H2b holds only up to an $O(g^2)$ scheme ambiguity. *Observed:* the ratio of a kinetic-dominated to a potential-dominated spacing drifts with $\Lambda$.
4. **Fast instability at the sourced background.** The Floor law $\lambda_t(j)$ is negative for $j\le2$ at $t=3$ *on the Berger field* (Part IV, sketch (a)). If the fast slice spectrum at $F_{\rm bg}$ has a negative or soft mode, §1.4 is false at the controls.
5. **Technique.** Norm estimates of state corrections diverge (Shale; Step 3), so any proof must work with forms and energies. The large-field region (Step 1) needs Bałaban-type control. No Hamiltonian version on a curved three-dimensional base exists. H2 could be true and still out of reach of the route.

---

## 6. Smallest next step

**Statement (KC$_1$).** Fix $t\in T$. For $|F|\le f_0$, with the heat-trace spectral regularization of line 14 applied to the whole fast slice spectrum of $K(F)$, the second-order kinetic correction $m_{ik}=m^{\rm NA}_{ik}+m^{\rm C}_{ik}$ of §1.1 satisfies
$$2m_{ik}(F)=-\,g^2\frac{b_0}{8\pi^2}\log(\Lambda R)\,\delta_{ik}+2m^{\rm fin}_{ik}(F)+O\bigl((\Lambda R)^{-1}\bigr),$$
with $m^{\rm fin}$ real-analytic in $F$. The $F$-dependent part of $\Phi$ is $O(g^2\log\Lambda R)\,\mathcal S(F)$ plus finite terms.

This is a one-loop identity on the explicitly known Berger spectrum, of the same type as line 14. If true, it makes $H^{(2)}_{\rm eff}$ finite with one coupling, fixes the one-loop speed-of-light renormalization, completes the $O(g^2)$ input to lines 15 and 18 (H8), and is the first rung of Step 5. If false, the Born–Oppenheimer frame has misplaced a Gauss-law term, and M3 must be rebuilt on the 18-mode space.

---

## Flag

*Jaffe–Witten clause:* "Existence includes establishing axiomatic properties at least as strong as those cited in [Streater–Wightman; Osterwalder–Schrader]". That includes Euclidean (Lorentz) invariance. A purely spatial spectral cutoff on $S^3_R$ breaks four-dimensional covariance. The renormalized limit is covariant only if the electric/magnetic ratio $c_0(\Lambda)$ is tuned, by a finite $O(g^2)$ amount if KC holds. I have not added this to H2's requirements. I record it as the precise meaning of "after renormalizing $1/g^2$" on line 16. Its covariance consequence belongs to H4 and H5 (lines 23, 29).
