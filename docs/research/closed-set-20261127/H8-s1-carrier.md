# H8: the carrier at the unsourced end ($s=1$)

*Closed set, 27 November 2026. Line 18 (part), with line 19. My status labels: [P] proved here, complete and checked; [P-sketch] a standard argument I give in outline but have not written in full; [guess] conjecture with the evidence stated.*

Units throughout are those of the reduced Hamiltonian $H_{\rm red}$ on the unit Berger sphere (reduced energies are half Yang–Mills energies, line 13). I write $u=t^{-1/2}$, so $\omega=(\sqrt t,u,u)$ and $S=\sum_k\omega_k=\sqrt t+2/\sqrt t$.

---

## 1. Statement (high fidelity)

**Objects.** Fix $t>0$ and $g>0$. Let $\mathcal H=L^2(\mathbb R^{3\times3})^{SO(3)}$ be the colour-invariant functions ($\psi(qO^T)=\psi(q)$ for all $O\in SO(3)$), with Lebesgue measure. At $s=1$ the sources vanish ($a=b=0$, line 19) and

$$
H_g=-\tfrac12\Delta_q+V_0,\qquad V_0(q)=\tfrac12\sum_{k,c}\bigl(\omega_kq_{kc}-g\,\mathrm{cof}(q)_{kc}\bigr)^2,
$$

the natural realization (closure from $C_c^\infty$). By Theorem A at $s=1$ ($\kappa=\min(t,1/t)$, line 19) it is self-adjoint with compact resolvent. For $t\neq1$ the Berger circle acts by $(U_\varphi\psi)(q)=\psi(R_\varphi^Tq\,R_\varphi)$, with $R_\varphi$ the rotation of rows 2,3 (on colour singlets this is the same as $\psi(R_\varphi^Tq)$). $\mathcal H=\bigoplus_{m\in\mathbb Z}\mathcal H_m$ by charge $m$, and $H_g$ preserves each $\mathcal H_m$.

Let $E_0(g)=\inf\operatorname{spec}H_g$. Following Part V (notation of the switch), define, at $s=1$,

- $\Delta_C(g)$ = (second eigenvalue of $H_g|_{\mathcal H_0}$, counted with multiplicity) $-\,E_0$;
- $\Delta_m(g)$ = (lowest eigenvalue of $H_g|_{\mathcal H_m}$) $-\,E_0$ for $m\neq0$; Helike's channel is $\Delta_H=\Delta_{\pm2}$.

**H8 (precise form).** For the given $t\neq1$ and coupling $g$:

$$
\text{(H8)}\qquad E_0 \text{ is a simple eigenvalue with eigenvector in }\mathcal H_0,\quad\text{and}\quad \Delta_C(g)<\min_{m\neq0}\Delta_m(g).
$$

The page's deduction (line 18, IVT) needs only the weaker $\Delta_C(g)<\Delta_H(g)$; I state the stronger "lowest excitation has charge 0" because that is the wording of line 18. At $t=1$ the symmetry is $SO(3)$ and the statement is the same with "charge 0" replaced by "$J=0$" and "charge $\pm2$" by "$J=2$" (line 18's "small-room $0^{++}<2^{++}$").

**Two versions.** (H8$_\infty$) asymptotic: there is $g_0(t)>0$ with (H8) for all $0<g<g_0(t)$. (H8$_{\rm reg}$) at the couplings where line 18 asserts a switch: $t=3$, $g^2\in\{0.002,0.004\}$ (controls, line 12) and $g^2\in\{0.03,0.05,0.07\}$ (registered $s^*$). The IVT at a given $g$ needs both ends at that same $g$, so line 18 needs H8$_{\rm reg}$, not only H8$_\infty$.

**Ambiguity, and the reading I choose.** The informal line says "the lowest excitation *about the background* $q_{\rm MC}$". In the natural realization there are *two* wells, $q=0$ and the orbit $q_{\rm MC}SO(3)$ (Theorem 8(1)); they are one point of $\mathcal A/\mathcal G$ but distinct points here. The deduction in line 18 uses spectral quantities of the whole operator (Part V's $\Delta_H,\Delta_C$), so I state H8 for the whole $H_g$. I also prove the one-well version (excitations of the $q_{\rm MC}$ well alone) and show the two give the same answer, with the same leading number, for a reason that turns out to be exact (§4, Step 4).

---

## 2. Interface

**Given to the page.**
- Line 18: the sign $\Delta_H(1,g)-\Delta_C(1,g)>0$. With line 12 ($\Delta_H<\Delta_C$ at $s=1.15$, conditional on H1) and line 19 (continuity of ordered eigenvalues in each sector on $[1,s_{\rm controls}]$), the IVT gives a switch. Line 17 (different sectors) makes it an exact crossing.
- Line 20 (H3): "starting from the unsourced room at $s=1$ with the scalar carrier": the small-$R$ end of M2 takes the identity (charge 0) and the leading size $\Delta_C\to\omega_C$ of the carrier.

**Taken from the page.** Line 8 ($q_{\rm MC}$, the line potential), line 9/19 (Theorem A at $s=1$: self-adjointness, compact resolvent), line 7 (colour action), line 17 (charge sectors invariant).

**Where the page needs more than H8 as worded.**
1. Line 18 needs H8$_{\rm reg}$ at each $g$ of a claimed switch. H8$_\infty$ alone gives the switch only for $g<g_0(t)$ with $g_0$ not yet quantified.
2. Line 20's sketch says "carrier persistence with only exact crossings". At $s=1$ the second charge-0 state is *not* a field quantum about $q_{\rm MC}$: it is the ground state of the $q_{\rm MC}$ well, the $q=0$ well being lower (§4, Step 4). Between $s=1$ and $s\approx1+O(g^2)$ the charge-0 sector has an *avoided* crossing between the two wells' ground states (§4, Step 6). The IVT is unaffected, because ordered eigenvalues are continuous whatever the branch identity. But "the competitor's branch at $s=1.15$ continues to the second charge-0 level at $s=1$" is false at small $g$, and near the avoided crossing the charge-0 gap $E_1-E_0$ is exponentially small. Line 20 must start *at* $s=1$ (as worded) and must not assume a gap uniform along the bridge.

---

## 3. What is known

**Proved (literature).**
- Harmonic approximation for low eigenvalues of $-\hbar^2\Delta/2+V$ with non-degenerate wells: B. Simon (1983), *Ann. Inst. H. Poincaré* A 38; B. Helffer and J. Sjöstrand (1984), *Comm. PDE* 9 ("Multiple wells in the semi-classical limit I"). Tunnelling between wells is controlled by the Agmon distance (same authors).
- Witten (1982), *J. Differential Geom.* 17, "Supersymmetry and Morse theory": $-\Delta+|\nabla W|^2\mp\Delta W$ and harmonic levels at critical points by Morse index; Helffer–Sjöstrand (1985), *Comm. PDE* 10, made this rigorous.
- Perron–Frobenius for Schrödinger operators: the ground state is simple and positive (Reed–Simon, *Methods of Modern Mathematical Physics* IV, 1978, §XIII.12).
- SU(2) on $S^3$ in small volume with a reduction containing the degree-1 vacuum and the sphaleron: P. van Baal and collaborators, e.g. van Baal and Hari Dass (1992). I cite this by author and year only and rely on none of its details. Their reduction (left- and right-invariant lowest modes) is invariant under the degree-1 gauge map; mine (left-invariant only) is not, which is why my two wells are not equivalent.

**Proved here (below).** The harmonic spectrum at $q_{\rm MC}$ with charges, exactly, for every $t$ [P]; the identity $H_g=\frac12\sum a_i^*a_i+\frac32S$ [P]; the ground energy to exponential accuracy [P]; the zero-point offset between wells equals $\omega_C$ exactly [P]; the harmonic switch point $s^*_0(t)$ [P at harmonic order]; the tunnelling-action bound [P].

**Expected, not proved.** H8$_\infty$ for the full anharmonic operator (standard semiclassics, Morse–Bott version) [P-sketch]; H8$_{\rm reg}$ [guess].

---

## 4. Best-guess proof

**Step 1. The superpotential [P].** Put $W(q)=\tfrac12\sum_k\omega_k\|q_k\|^2-g\det q$. Since $\partial\det q/\partial q_{kc}=\mathrm{cof}(q)_{kc}$, $\nabla W=\omega\circ q-g\,\mathrm{cof}\,q$, so $V_0=\tfrac12|\nabla W|^2$. On homogeneous connections $W=CS/(2\sqrt t\,g^2)$: the Chern–Simons quadratic term is $\sum_k c_k\|q_k\|^2$ with $c_k\propto\omega_k$ (because $d\theta^1=\sqrt t\,\theta^2\wedge\theta^3$ and $d\theta^{2,3}=t^{-1/2}\theta^{3,1}\wedge\theta^{1,2}$ up to a common sign), and the cubic term is $\propto\det q$. Along the line, $W(\lambda q_{\rm MC})=(3\lambda^2-2\lambda^3)/(2\sqrt t\,g^2)=CS(\lambda)/(2\sqrt t\,g^2)$ (line 8), which fixes both constants. Since $\det$ is linear in each entry, $\Delta W=3S$ is **constant**. With $a_i=\partial_i+\partial_iW$, $a_i^*=-\partial_i+\partial_iW$:

$$
H_g=\tfrac12\sum_i a_i^*a_i+\tfrac32S\qquad\text{(exact, on }C_c^\infty).
$$

Hence $E_0\ge\frac32S$ for every $g$ and $t$.

**Step 2. Hessians [P].** Scale $q=p/g$: $H_g=g^{-2}\bigl[-\tfrac{\hbar^2}2\Delta_p+V_1(p)\bigr]$ with $\hbar=g^2$, $V_1=V_0|_{g=1}$. So the harmonic frequencies do not depend on $g$. At a zero of $V_0$, $\operatorname{Hess}V_0=J^2$ with $J=\operatorname{Hess}W$ (symmetric), so the frequencies are $|{\rm eig}\,J|$.

*At $q=0$:* $J_0=\operatorname{diag}(\omega_k)\otimes 1_{\rm colour}$, giving frequencies $\sqrt t$ (row 1, ×3 colours) and $u$ (rows 2,3, ×6).

*At $q_{\rm MC}$* (diagonal $m=(u,1,1)/g$; I set $g=1$). Linearizing cof at a diagonal matrix: $\delta\,\mathrm{cof}_{kk}=\sum_{j\ne k}m_l\,\delta_{jj}$, and $\delta\,\mathrm{cof}_{kc}=-m_l\,\delta_{ck}$ for $k\ne c$, where $l$ is the third index. So $J$ splits into blocks:

- Diagonal block on $(\delta_{11},\delta_{22},\delta_{33})$: $J_d=\begin{pmatrix}1/u&-1&-1\\-1&u&-u\\-1&-u&u\end{pmatrix}$.
  - $(0,1,-1)$ is an eigenvector with eigenvalue $2u$: this is $E_{22}-E_{33}$.
  - On $\{e_1,(e_2+e_3)/\sqrt2\}$ the block is $\begin{pmatrix}\sqrt t&-\sqrt2\\-\sqrt2&0\end{pmatrix}$, with eigenvalues $\mu_\pm=\tfrac12(\sqrt t\pm\sqrt{t+8})$, $\mu_+\mu_-=-2$.
- Off-diagonal pair $(\delta_{kc},\delta_{ck})$: $\begin{pmatrix}\omega_k&m_l\\m_l&\omega_c\end{pmatrix}$. Since $\omega_km_k=m_cm_l$, the vector $(m_k,-m_c)$, a colour rotation, has eigenvalue $0$. The other eigenvalue is $\omega_k+\omega_c$.

Charges under $q\mapsto R qR^T$: $E_{11}$ and $E_{22}+E_{33}$ have charge 0; $E_{23}-E_{32}$ has charge 0 and is gauge; $\{E_{22}-E_{33},E_{23}+E_{32}\}$ has charge $\pm2$; $\{E_{12},E_{13}\}$ and $\{E_{21},E_{31}\}$ have charge $\pm1$. That gives 9 = 3 gauge + 6 physical modes:

| mode | charge | frequency |
|---|---|---|
| competitor $\nu_C$ | 0 | $\omega_C:=|\mu_-|=\tfrac12(\sqrt{t+8}-\sqrt t)=2/\mu_+$ |
| breathing-partner | 0 | $\mu_+=\tfrac12(\sqrt{t+8}+\sqrt t)$ |
| Helike $(E_{22}-E_{33},\,E_{23}+E_{32})$ | $\pm2$ | $\omega_H=2/\sqrt t$ (×2) |
| pair (1,2),(1,3) | $\pm1$ | $\sqrt t+1/\sqrt t$ (×2) |

*Checks.* (i) $\omega_H^2=4/t$ agrees with Proposition "Helike in the reduced model", $2s(1+2/t-s)$, at $s=1$. (ii) At $t=1$: $\omega_C=1$ on $(1,1,1)$, the breathing mode along $q_{\rm MC}$ ($J=0$; it matches $V(\lambda q_{\rm MC})=\frac32(\lambda-\lambda^2)^2$ with $\|p_{\rm MC}\|^2=3$). The other five frequencies all equal $2$, a $J=2$ quintet. (iii) The same method with sources, at $t=3$, $s=1.15$, gives the charge-0 block $\begin{pmatrix}5.75&-2.5352\\-2.5352&2.645\end{pmatrix}$, whose frequencies $1.1067$ and $2.6777$ reproduce the page's computed competitor and second charge-0 mode. For general $s$, my charge-0 quadratic form is
$$Q_s=s(t+2)x_1^2-2\sqrt2\,s\bigl(\sqrt t+2(1-s)/\sqrt t\bigr)x_1y+2s^2y^2.$$

At $t=3$: $\omega_C=0.79229$, $\omega_H=1.15470$, $\mu_+=2.52434$, and the charge-$\pm1$ pair is at $2.30940$.

**Step 3. The ordering at $q_{\rm MC}$, for all $t$ [P].** $\omega_C<\omega_H\iff 2/\mu_+<2/\sqrt t\iff\sqrt{t+8}>\sqrt t$, which holds for every $t$. Also $\omega_C<\sqrt2\le2\le\sqrt t+1/\sqrt t$, and $\omega_C<\mu_+$. So the lowest normal mode at the background is the charge-0 competitor for **every** $t>0$, and $\omega_C/\omega_H=2\sqrt t/(\sqrt t+\sqrt{t+8})<1$, which tends to $1$ only as $t\to\infty$. *Corollary (harmonic switch, [P] at harmonic order).* Using $Q_s$ and $\omega_H(s)^2=2s(1+2/t-s)$, the condition that $\omega_H^2$ is an eigenvalue of the charge-0 block reduces (after dividing by $t-1$) to $t\sigma^2+t(t+2)\sigma-2(t+2)^2=0$ with $\sigma=2s$. So
$$
s^*_0(t)=\frac{(t+2)(\sqrt{t+8}-\sqrt t)}{4\sqrt t}=\frac{t+2}{2\sqrt t}\,\omega_C(1),\qquad s^*_0(3)=\frac{5\sqrt{33}-15}{12}=1.1436,
$$
and $1<s^*_0(t)<(t+2)/(t+1)$ exactly when $t>1$. This agrees with Helike being lowest at $s=1.15$ harmonically, and with the registered $s^*$ (1.1674, 1.1809, 1.1931) moving up from about this value as $g^2$ grows. The page's $c_C-c_H=-2.208<0$ at $s=1.15$ is the reason.

**Step 4. Two wells; the offset is exactly $\omega_C$ [P].** The harmonic ground energies are $e_0(0)=\tfrac12\cdot3S$ and $e_0(q_{\rm MC})=\tfrac12\sum|{\rm eig}\,J_{\rm MC}|$ over the 6 physical modes. Both $J$'s have trace $3S$ (the cofactor has zero diagonal derivatives). $J_0$ is positive, while $J_{\rm MC}$ has exactly one negative eigenvalue, $\mu_-$, and three zeros. Hence
$$
e_0(q_{\rm MC})-e_0(0)=\tfrac12\bigl(\textstyle\sum|{\rm eig}|-\sum{\rm eig}\bigr)=|\mu_-|=\omega_C .
$$
In Witten's language: $0$ is the minimum of $W$, and $q_{\rm MC}$ is an index-1 saddle (mod gauge) whose unstable direction is the competitor mode. $e_0(0)=\frac32S$ is exactly the bound of Step 1. So:

*(a) Ground state [P].* $\tfrac32S\le E_0\le\tfrac32S+C_\eta\,g^{-p}\,e^{-(1-\eta)/(\sqrt t\,g^2)}$ for any $\eta>0$ (some fixed power $p$). Proof of the upper bound: take the trial function $\chi e^{-W}$, which is colour invariant. Then $\langle\chi e^{-W},(H-\frac32S)\chi e^{-W}\rangle=\frac12\int|\nabla\chi|^2e^{-2W}$. Take $\chi=f(W)$ on the connected component $B$ of $\{W<w_0\}$ containing $0$, where $w_0=W(q_{\rm MC})=1/(2\sqrt t g^2)$, and $\chi=0$ off $B$, with $f=1$ below $(1-\eta)w_0$. $B$ is bounded: along a ray $W=Ar^2-Br^3$ first reaches $w_0$ at $r\le\sqrt{2w_0/\min\omega_k}$. Every ray maximum is $\ge w_0$, because the minimum over rays of the ray maximum is a critical point of $W$ (Nehari), and the only non-zero critical points are the orbit, where $W=w_0$. The denominator is close to $\prod_k(\pi/\omega_k)^{3/2}$. So **the vacuum energy at $s=1$ has no perturbative corrections at all**: the vacuum is the $CS=0$ well, not the background.

*(b) Level list at harmonic order.* The low levels of $H_g$ are $\frac32S$ plus one of:
- $q=0$ well: colour singlets need $\ge2$ quanta ($\delta_{cc'}$) or 3 ($\epsilon$). The lowest are $2u$ for two row-(2,3) quanta (charges $0,\pm2$, degenerate at this order), $\sqrt t+u$ (charge $\pm1$), and $2\sqrt t$ (row 1, charge 0).
- $q_{\rm MC}$ well (singlets constant along the orbit, so the 6-dim slice): $\omega_C+\sum n_i\nu_i$. The lowest are $\omega_C$ (well ground, charge 0), $2\omega_C$ (charge 0), $\omega_C+2u$ (charge $\pm2$), and so on.

Therefore, at harmonic order:
$$
\Delta_C\to\min(\omega_C,\,2\sqrt t),\qquad \Delta_{\pm2}\to 2/\sqrt t,\qquad \Delta_{\pm1}\to\sqrt t+1/\sqrt t,\qquad \Delta_{|m|\ge3}\ \text{larger}.
$$
The margin is $\min(2/\sqrt t,\sqrt t+1/\sqrt t)-\min(\omega_C,2\sqrt t)\ge\min(2/\sqrt t,\sqrt t+1/\sqrt t)-\omega_C>0$ for all $t\neq1$. At $t=3$ it is $2/\sqrt3-\omega_C=0.3624$. ($2\sqrt t<\omega_C$ only for $t<1/3$, and that level is charge 0 too.) **Both readings agree:** in the whole model the first excitation is the $q_{\rm MC}$ well's ground state, at offset $\omega_C$, charge 0. In the one-well reading it is the competitor quantum, at frequency $\omega_C$, charge 0. The equality of the two numbers is Step 4, not a coincidence.

**Step 5. Transfer to the anharmonic operator, $g\to0$ [P-sketch].** In $p$-variables, $V_1\ge0$ vanishes exactly on $\{0\}\cup p_{\rm MC}SO(3)$ (Theorem 8). It is non-degenerate transversally: $J_0>0$, and $J_{\rm MC}$ has no zero eigenvalue on the slice since $\mu_-\neq0$. And $V_1\to\infty$ at infinity, from $V_1=\frac12\sum\omega_k^2\|p_k\|^2-S\det p+\frac12\|\mathrm{cof}\,p\|^2$ and $\|\mathrm{cof}\|^2\ge3|\det|^{4/3}$. IMS localization (Simon 1983) to $O(\hbar^{2/5})$-balls around $0$, around the orbit (where colour-singlet functions are functions on the 6-dim slice, with smooth positive orbit-volume weight of relative variation $O(g)$ on the localization scale), and the remainder gives, for each fixed $n$ and each charge sector, $E_n=$ ($n$-th harmonic level of that sector) $+\,o(1)$. Analyticity of $V$ and parity of the cubic terms give $O(g^2)$. Positivity improvement of $e^{-\tau H}$ on $L^2(\mathbb R^9)$ makes $E_0$ simple, and its positive eigenvector is invariant under colour and $U(1)$, so it lies in $\mathcal H_0$. With Step 4(b) this gives **H8$_\infty$** with $\Delta_C=\omega_C+O(g^2)$ and $\Delta_H=2/\sqrt t+O(g^2)$ for $t>1$. What is not written out: the Morse–Bott slice details at the orbit.

**Step 6. The exact inter-well action [P; leading-order harmonic for $\varepsilon^*$ below].** The wells' harmonic ground levels differ by $\omega_C=O(1)$. Here I determine the geometric action, not the quantum splitting. For the existing potential at s = 1 and every t > 0, the Agmon distance in p-variables is
$$
d=\frac{1}{2\sqrt t},\qquad
L_{\rm straight}=\frac{\sqrt{(t+2)(2t+1)}}{6t},\qquad
L_{\rm straight}^2-d^2=\frac{(t^{-1}-1)^2}{18}.
$$
The former gradient-line guess is closed. The inward unstable branch in $\operatorname{diag}(x,y,y)$ stays in $0<x<u$, $0<y<1$, with $u=t^{-1/2}$, and converges to 0. The inherited metric is $dx^2+2dy^2$. Its action equals the change of W; endpoint connectors justify the path infimum. The straight line is strictly longer except at the round sphere. At t = 3 the action is $0.2886751346\ldots$, replacing the old interval ending at $0.3286710991\ldots$.

Full proof and formal boundary: [The curved path reaches the other well](../h8-interwell-proof-2026-11-27.md). The written global ODE argument uses the standard unstable-manifold theorem; Lean checks the local geometry and calculus, not the entire orbit-existence theorem. Returning to q gives action $d/g^2$. This does not establish a tunnelling prefactor, an exact level splitting, or a finite-g carrier ordering. H8 remains open.

For $s=1+\varepsilon$ the background well deepens. From line 8 and the sources, $V(q_{\rm bg})=-(1+2/t)\varepsilon/(2g^2)+O(\varepsilon^2/g^2)$, while the $q=0$ well stays at $V=0$ with Hessian $\operatorname{diag}(\omega_k^2-{\rm src}_k)>0$. The leading harmonic estimate places a charge-0 avoided crossing at the value below. The expected exponential splitting scale $e^{-d/g^2}$ is a semiclassical expectation, not a consequence proved by the action calculation:
$$
\varepsilon^*\simeq\frac{2t}{t+2}\,\omega_C\,g^2\qquad(\approx0.95\,g^2\text{ at }t=3).
$$
At the controls the $q=0$ well lies $\approx0.19/g^2$ above (computed from $V(q_{\rm bg})$ at $t=3$, $s=1.15$), which is invisible there. This is the content of Interface item 2.

**Step 7. To the registered couplings [guess].** H8$_{\rm reg}$ at $t=3$ needs the $O(g^2)$ corrections to keep $\Delta_H-\Delta_C>0$ from a harmonic margin of $0.3624$ up to $g^2=0.07$, i.e. $|c_H(1)-c_C(1)|<5.2$. The evidence is induction from $s=1.15$, where the page's exact coefficients are $c_H=-0.530$ and $c_C=-2.738$. Their difference, $2.21$, is below that threshold, and with the sign seen there (competitor pushed *down*) it *widens* the $s=1$ margin. One more fact helps: the ground level has zero perturbative shift (Step 4a), so all of $c_X$ sits in the excited level.

---

## 5. Where it can fail

1. **Finite-$g$ reordering** (kills H8$_{\rm reg}$, not H8$_\infty$). If $c_H(1)-c_C(1)<-0.3624/g^2$, i.e. Helike's channel is pushed down by more than 5 per unit $g^2$ relative to the competitor at $s=1$, $t=3$, $g^2=0.07$. One would observe a charge-$\pm2$ level below the second charge-0 level in the converged $s=1$ spectrum.
2. **Wrong kind of carrier** (kills the interpretation, not the sign). At $s=1$ the second charge-0 state is the other well's vacuum. In full Yang–Mills, $0$ and $q_{\rm MC}$ are one gauge orbit, and the fast modes must make up the offset. Gauge invariance of the total zero-point energy suggests $E_0^{\rm fast}(q_{\rm MC})-E_0^{\rm fast}(0)=-\omega_C$ in reduced units [guess; gauge-zero-mode bookkeeping and the ghost determinant not checked]. In $H_{\rm eff}$ (line 16) the two wells would then be degenerate to $O(g^2)$, and the lowest charge-0 excitation could be an exponentially small two-well splitting. Over the true chain of winding vacua there is one Bloch vacuum per $\theta$, so in the $\theta=0$ sector this state is an artefact of the two-well truncation, not a glueball. One would observe the $s=1$ charge-0 gap of the dressed reduced model shrinking like $e^{-d/g^2}$. The *charge* in H8 survives this (both wells' vacua are $U(1)$-fixed), but line 20's small-$R$ input $\Delta\to\omega_C$ would then have to be read from the competitor quantum on the identified vacuum, which is a different statement.
3. **Inter-well path: resolved for this potential.** The inward branch reaches 0 for every t > 0, by Step 6 and its proof note. This removes the former no-heteroclinic concern. It does not supply the anharmonic spectral estimates in H8.
4. **$t$ near 1.** As $t\to1$ the charge label must be replaced by $J$. Nothing fails, but H8 as a $U(1)$ statement is empty at exactly $t=1$.

---

## 6. Smallest next step

**Effective harmonic transfer at the unsourced end.** For $t=3$ and all $0<g^2\le0.07$:
$$
\inf\operatorname{spec}\Bigl(\tfrac12\textstyle\sum_ia_i^*a_i\Big|_{\mathcal H_{\pm2}}\Bigr)\ \ge\ \tfrac{2}{\sqrt3}-C_2g^2
\quad\text{and}\quad
\lambda_2\Bigl(\tfrac12\textstyle\sum_ia_i^*a_i\Big|_{\mathcal H_0}\Bigr)\ \le\ \omega_C+C_0g^2,
$$
with explicit constants satisfying $C_0+C_2<0.3624/0.07\approx5.18$. Here $\lambda_2$ is the second eigenvalue, $a_i=\partial_i+\partial_iW$, and $W=CS/(2\sqrt3\,g^2)$. By Steps 1 and 4(a) the ground energy is already pinned to $\frac32S$ up to $e^{-(1-\eta)/(\sqrt3g^2)}$, so this pair of one-sided bounds is all of H8$_{\rm reg}$ for $\Delta_H>\Delta_C$ at $t=3$. The first inequality is a weighted Poincaré inequality for charge-2 functions against $e^{-2W}$ on the basin $B$. The second is a two-dimensional min–max with the two well-localized Gaussians.

---

### Flag

None. H8 is internal to the small-room programme; no clause of the Jaffe–Witten statement depends on it directly.
