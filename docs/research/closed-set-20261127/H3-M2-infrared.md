# H3 (line 20, M2): the gap in the field's own scale, uniformly in the room

*Status: hypothesis [X]. This is the wall. Nothing in this section is proved unless it is marked as proved, and every guess is labelled [guess].*

The page fixes $G = SU(2)$ (the value $b_0 = 22/3$ is the SU(2) coefficient; other compact simple groups are H7). In this section "the room" is the Berger sphere $S^3_{R,t}$ of radius $R$ at a **fixed** squashing $t$: the $t$ at which H8 and the carrier lines 12 and 18 are stated, with $t = 1$ the round sphere. Every constant below may depend on $t$. As $R \to \infty$ at fixed $t$, every ball of fixed physical radius in $S^3_{R,t}$ becomes flat, so fixing $t$ costs nothing in the $\mathbb R^4$ limit (H5).

---

## 1. Statement (high fidelity)

**Objects.**

1. *Regularized theory.* For a UV regulator $\Lambda_{\rm cut}$ (the gauge-invariant heat-trace/spectral cutoff of line 14, or a lattice of spacing $a = \Lambda_{\rm cut}^{-1}$ on $S^3_{R,t}$ as in line 29), the bare coupling is $g_0^2(\Lambda_{\rm cut})$. Let $\mathcal H_{R,\Lambda_{\rm cut}}$ be the physical Hilbert space: states annihilated by the Gauss constraint, so they are invariant under small gauge transformations, including the global colour SU(2). They are also invariant under large gauge transformations ($\pi_3(SU(2)) = \mathbb Z$), i.e. $\theta = 0$, which is the sector line 1's Wilson measure produces. $H_{R,\Lambda_{\rm cut}} \ge 0$ is the Hamiltonian, normalized so that $\inf\operatorname{spec} = 0$. It is reconstructed from the reflection-positive Euclidean measure on $S^3_{R,t}\times\mathbb R$ (lines 2 and 22).

2. *The scale $\Lambda$ (a definition, fixed once).* The renormalization trajectory is the curve $\Lambda_{\rm cut}\mapsto g_0^2(\Lambda_{\rm cut})$ along which
$$
\Lambda \;:=\; \lim_{\Lambda_{\rm cut}\to\infty}\Lambda_{\rm cut}\,
\Big(\tfrac{b_0\, g_0^2}{16\pi^2}\Big)^{-b_1/(2b_0^2)}
\exp\!\Big(-\frac{8\pi^2}{b_0\, g_0^2}\Big),
\qquad b_0 = \tfrac{22}{3},\; b_1 = \tfrac{136}{3},
$$
exists and is held fixed. Existence of this limit along a trajectory on which the theory converges is part of the UV construction (line 21, H5). Changing the scheme or regulator multiplies $\Lambda$ by a finite positive constant, which H5 asserts ("up to the value of $\Lambda$"). Statement H3 is unchanged by such a rescaling, though $c$ changes.

   *Reading of the page's formula.* Line 20 writes $R\Lambda = \exp(-8\pi^2/(b_0 g^2(R)))$. I read this as the one-loop **definition of the running coupling $g^2(R)$ in terms of $R$ and the fixed $\Lambda$**:
$$
g^2(R) := \frac{8\pi^2}{b_0\,\log\!\big(1/(R\Lambda)\big)}\quad (R\Lambda<1).
$$
   It is not a second definition of $\Lambda$: read that way, $\Lambda$ would drift with $R$ beyond one loop. In this reading $g^2(R)$ is a label for the room and no input. The theory at radius $R$ is fixed by $(R, \Lambda)$ alone. Any finite-volume scheme (for example van den Heuvel's $f = g^2(R)/2\pi^2$) agrees with this to leading order as $R\Lambda\to 0$.

3. *Continuum room Hamiltonian.* $H_R$ on $\mathcal H_R$ is the UV limit $\Lambda_{\rm cut}\to\infty$ along the trajectory, at fixed $R$ and $\Lambda$. Its existence is H5 (line 29). By dimensional analysis, which is exact once only $\Lambda$ and $R$ remain, there is a one-parameter family $\mathcal K(x)$ on the unit-radius room with
$$
H_R \;\cong\; R^{-1}\,\mathcal K(x),\qquad x := R\Lambda \in (0,\infty).
$$

4. *The gap.* Let $\Omega_R$ be a ground state ($H_R\Omega_R = 0$). Define
$$
\Delta(R) := \inf\operatorname{spec}\big(H_R\restriction \{\Omega_R\}^{\perp}\big),\qquad \delta(x) := R\,\Delta(R) = \inf\operatorname{spec}\big(\mathcal K(x)\restriction\{\Omega\}^\perp\big).
$$
The gap is taken over the **whole** physical Hilbert space: every isometry quantum number, both Helike charges, both parities. The ground state is excluded only through $\{\Omega_R\}^\perp$, so a degenerate vacuum gives $\Delta(R) = 0$.

**Statement H3.** *There is a constant $c > 0$, depending only on $t$ (and on $G$), such that*
$$
\forall R\in(0,\infty):\qquad \frac{\Delta(R)}{\Lambda} \;=\; \frac{\delta(x)}{x}\;\ge\; c .
$$
Equivalently, $\inf_{x>0}\delta(x)/x > 0$. "For every $R$" means one constant $c$ for all $R$. The order of limits is: first $\Lambda_{\rm cut}\to\infty$ at fixed $R$ (giving $H_R$), then the bound holds for each $R$ with the same $c$. No limit $R\to\infty$ is taken inside H3. That limit is H4/H5 and line 28.

**Regularized form H3′ (sufficient, stronger).** There are $c>0$ and $\Lambda_0(R)<\infty$ such that $\Delta_{\Lambda_{\rm cut}}(R) \ge c\Lambda$ for all $R$ and all $\Lambda_{\rm cut}\ge\Lambda_0(R)$. Given H5's convergence of the two-point functions, H3′ implies H3 by the lemma of line 28: a decay rate that holds uniformly is preserved under limits of the correlation functions.

**Why this reading.** Line 20 says "starting from the unsourced room at $s=1$ with the scalar carrier". The carrier and the starting point belong to the proof route (§4), not to the statement. Line 28 applies the bound to every local gauge-invariant $A$, so it needs the gap over all sectors, not the gap of the scalar sector alone. I therefore state H3 for the full gap. The scalar carrier enters only as the identity of the sector that realizes $\delta(x)$ near $x = 0$ (H8).

---

## 2. Interface

**What other lines take from H3.**

- **Line 28** (gap survives the limit). It needs:
  (a) a **unique** vacuum $\Omega_R$ for every $R$ in the sequence used for the limit. H3 as defined contains this ($\Delta(R) > 0$ forces $\dim\ker H_R = 1$); line 28 also gets it independently from positivity improvement;
  (b) the bound $\langle A;e^{-\tau H_R}A\rangle \le \|(A-\langle A\rangle)\Omega_R\|^2 e^{-\tau\Delta(R)}$ with rate $\Delta(R)\ge m := c\Lambda$ **independent of $R$**, for every local gauge-invariant $A$, hence the full gap;
  (c) the **same** $\Lambda$ that labels the $\mathbb R^4$ theory, which H5 supplies.

  Strictly, line 28 needs less than H3: only $\liminf_{n}\Delta(R_n)\ge m>0$ along the sequence $R_n\to\infty$ on which H5 gives convergence. The small-$R$ half of H3 is used only inside the proof route, not by line 28. The conclusion line 28 draws is $\operatorname{spec}H\cap(0,c\Lambda) = \emptyset$ on $\mathbb R^4$.

- **Line 27** (unique vacuum) uses H3 only through line 28.

**What H3 takes from other lines (along my route).**

- Line 19 (the bridge from the controls to $s=1$) and H8 (line 18). At $s=1$ the sources vanish: $a = (1-s)(t-2s) = 0$ and $b = (1-s)\big((1-s)/t - s\big) = 0$. The background $q_{MC}$ is a classical vacuum ($V(\lambda q_{MC})\propto(\lambda-\lambda^2)^2 = 0$ at $\lambda = 1$, $\mathrm{CS} = 1$), gauge-equivalent to $A = 0$ by a large gauge transformation. So the "unsourced room" is ordinary Yang–Mills on $S^3_{R,t}$ and needs no further matching. Theorem A holds there with $\kappa = \min(t,1/t)$.
- Lines 7–9 and 13: discrete spectrum of the slow sector, and the exact slow/fast Peter–Weyl split.
- **H2 (line 16)**, in a **strengthened form**. H2 bounds only the compressed block $P_{\rm low}(H-H_{\rm eff})P_{\rm low}$. Comparing the low spectrum of $H_R$ with that of $H_{\rm eff}$ needs three more things: (i) the bound read in units of $1/R$, i.e. $R\,\|P_{\rm low}(H_{\Lambda_{\rm cut}}-H_{\rm eff})P_{\rm low}\|\le Cg^2(R)$; (ii) a lower bound $P_{\rm high}H P_{\rm high}\ge E_{\rm high}/R$, with $E_{\rm high}$ above the low window; (iii) a bound on the off-diagonal block, so the Schur complement moves the low levels by $O(g^2(R))/R$. Only with all three does $|\delta(x)-\delta_{\rm eff}(g^2(R))| \le C'g^2(R)$ follow. I record this as what line 20 needs from line 16.
- H5 (line 29): existence of $H_R$ and the definition of $\Lambda$.
- Lines 3–5 (strong coupling) enter only heuristically, through §4 Step 5. At every fixed $R$ the continuum limit sends the bare coupling to zero, so the strong-coupling lattice regime is **not** the large-$R$ end of the continuum family. Lines 3–5 can only describe an effective theory at a coarse scale (see §4, Step 5e).

---

## 3. What is known

**Proved.**
- Lattice gauge theory on a finite lattice: the transfer matrix is reflection positive (Osterwalder–Seiler 1978, Ann. Phys.) and positivity improving, so the vacuum is unique and the gap is positive **at each fixed lattice and volume**. At small $\beta$ the gap is uniform in volume (Osterwalder–Seiler 1978; line 3). Shen, Zhu and Zhu (Commun. Math. Phys. 400 (2023) 805–851) prove a unique infinite-volume measure and exponential decay at strong coupling in any dimension, by a Bakry–Émery mechanism.
- UV stability for 4-D lattice Yang–Mills in a finite periodic box, uniform in the spacing (Bałaban, Commun. Math. Phys. 1987–89). Schwinger functions of SU(2) YM$_4$ with the UV cutoff removed at a **fixed IR cutoff** (Magnen, Rivasseau and Sénéor, Commun. Math. Phys. 155 (1993) 325). Neither result removes the IR cutoff or proves a gap.
- Finite-size criteria in statistical mechanics, where mixing at one finite scale implies mixing in infinite volume: Dobrushin (1968), Dobrushin–Shlosman (1985, 1987), Stroock–Zegarlinski (1992), Martinelli–Olivieri (Commun. Math. Phys. 161, 1994). These are proved for lattice systems with compact spins and finite-range interactions. **I know of no version for lattice gauge theory with constants uniform in the lattice spacing.** Knabe (1988) proves a finite-size gap criterion for frustration-free spin Hamiltonians. Yang–Mills is not frustration-free.
- This programme: lines 7–9 (Theorem A, with $s=1$ inside the window), 13, 17, 19 [P]; line 14 [C].

**Expected, not proved.**
- Small volume (Lüscher, Nucl. Phys. B219 (1983) 233; Koller–van Baal, Nucl. Phys. B302 (1988) 1; van Baal–Hari Dass 1992; van den Heuvel, Phys. Lett. B386 (1996) 233). In a small room the low spectrum is the slow-mode Hamiltonian dressed by the fast modes, with gap $\propto 1/R$. These are physics computations. Van den Heuvel reads the scalar/tensor crossing beyond $f\approx 0.3$ as a breakdown of the approximation, not as physics.
- Finite-size behaviour. Given a gapped infinite-volume theory, the torus shift is exponentially small (Lüscher, Commun. Math. Phys. 104 (1986) 177; this *assumes* the gap). On $S^3_R$ I expect polynomial curvature corrections instead. Either way the expectation is $\delta(x) = x\,\Delta_\infty/\Lambda + o(x)$: linear growth.
- Step scaling (Lüscher–Weisz–Wolff, Nucl. Phys. B359 (1991) 221) and lattice spectroscopy (for SU(3), Morningstar–Peardon, Phys. Rev. D60 (1999) 034509, with $0^{++}$ lightest) support a continuum gap numerically.

---

## 3.5 The bundle skeleton (my framing: the room is a base coordinate)

The size of the room is a coordinate of the bundle's base. Over each room $R$ sits a fibre: the theory in that room, with its levels, carriers and gap. The renormalization group is motion along this coordinate, which is why the running coupling (line 14) belongs to the base. H3 is then a statement about one **section** over the half-line,
$$R\ \longmapsto\ \Delta(R)/\Lambda,\qquad R\in(0,\infty),$$
equivalently $x\mapsto\delta(x)/x$ with $x=R\Lambda$ (Step 0). A section of this kind is bounded away from zero when three things hold:

1. **Continuity along the base.** $x\mapsto\delta(x)$ is continuous on $(0,\infty)$ (Step 2). Crossings between sectors are exact (line 17) and avoided crossings within a sector do not break continuity, so a switch of carrier never breaks the section.
2. **No zero in any room.** $\delta(x)>0$ for every finite $x$ (Step 1): compact resolvent and a unique ground state in each room.
3. **Control at both ends.**
   - *Small rooms* (Step 3): $g^2(R)\to0$, and H2 (M3, with the computed dressing) gives $\delta(x)/x\to\infty$.
   - *Large rooms* (Step 5): $\liminf_{x\to\infty}\delta(x)/x>0$.

Parts 1 and 2 give a positive minimum on every compact stretch $[x_0,X]$ (Step 4). Part 3 closes the two open ends. Together: $\inf_{x>0}\delta(x)/x>0$, which is H3.

**Where the weight sits.** Parts 1 and 2 and the small end are the shape the bundle gives for free, and Steps 1–4 carry them. Everything hard is the large end, Step 5.

**Correction to my earlier sketch.** I had placed the strong-coupling lattice regime (lines 3–5) at the large-$R$ end. That is wrong for the continuum family. At every fixed room the continuum limit sends the bare coupling to zero, so the strong-coupling regime is never the large-$R$ end of this section. The large end must come from a finite-size criterion at a fixed physical scale (Step 5). Lines 3–5 can at most describe an effective theory at a coarse scale (Step 5e). H3's statement is unchanged; only my sketch of its proof moved.

**Seeing the room grow.** In GIGI every computed room is a base point with its fibre stored. Adding rooms extends the section, so $\Delta/\Lambda$ can be watched along $R$. Carriers are labelled by charge, so a switch appears as an exact crossing, a structure rather than noise. A loop in the parameters $(R,s,g^2)$ whose carrier returns as itself is a holonomy check: trivial holonomy means no hidden crossing. Watching the section extend is evidence (induction). The proof is parts 1–3.

## 4. Best-guess proof

**Step 0 (reduction to one function).** By the scaling in §1.3, $\Delta(R)/\Lambda = \delta(x)/x$. So H3 is equivalent to $\inf_{x>0}\delta(x)/x>0$. [Exact, given H5.]

**Step 1 (positivity in each room).** For every $x\in(0,\infty)$: $\mathcal K(x)$ has compact resolvent and a unique ground state, so $\delta(x)>0$.
*Justification:* proved for every lattice regularization (positivity-improving transfer matrix [PC]). For the slow sector it is proved by line 9. [guess] *Sub-conjecture 1:* compact resolvent and uniqueness survive the UV limit at fixed $R$. Evidence: the room is compact, the heat-trace regularization of line 14 is gauge invariant, and the divergence is exactly the coupling renormalization, matched to $10^{-4}$.

**Step 2 (continuity in scale).** $x\mapsto\delta(x)$ is continuous on $(0,\infty)$.
*Justification:* if $x\mapsto\mathcal K(x)$ is continuous in the norm-resolvent sense, then each isolated eigenvalue and each spectral projection is continuous (Kato), and $\delta(x)$ is the minimum, over finitely many sectors, of each sector's lowest level above the vacuum. [guess] *Sub-conjecture 2:* norm-resolvent continuity of the renormalized family in $x$.
*What carriers add and do not add.* Crossings between sectors are exact (line 17), and the minimum of continuous functions is continuous, so crossings never break continuity of $\delta$. Avoided crossings within a sector do not either. **So carrier persistence is not needed for Steps 1–4.** Its role is to say *which* finitely many sectors can realize $\delta(x)$. That reduces each room-level lower bound to Lehmann/Temple bounds sector by sector, using the S_C template $\lambda_k(H)\ge\min(\lambda_k(H_{\rm in}),\inf H_{\rm out})$ (line 11).

**Step 3 (small rooms).** As $x\to0$, $g^2(R) = 8\pi^2/(b_0\log(1/x))\to0$, and
$$
\delta(x) = \delta_0 + O\big(g^2(R)\big),
$$
where $\delta_0>0$ is the lowest colour-singlet excitation of the free harmonic Hamiltonian on $S^3_{R,t}$. Physical states are global-colour singlets, so this is a two-quantum level built from the lowest frequencies $\omega_k$ of line 9. H8 says that at $s=1$ the scalar ($0^{++}$) combination is the lowest; H3 needs only $\delta_0>0$.
*Justification:* lines 9 and 13, lines 14–15 (the fast dressing is finite after coupling renormalization), and H2 in the strengthened form of §2. Hence $\delta(x)/x\to\infty$, and there is $x_0>0$ with $\delta(x)/x\ge1$ on $(0,x_0]$.

**Step 4 (any bounded range).** On $[x_0,X]$, Steps 1–2 give $\min\delta>0$, so $\delta(x)/x\ge\min_{[x_0,X]}\delta/X>0$.
**This is all that line 20's sketch, as worded ("continuity along scale by carrier persistence"), can give.** Continuity and positivity give a bound on every compact range of $R$, never a uniform one as $R\to\infty$. The uniform bound needs an asymptotic estimate. That is Step 5, and it is the wall.

**Step 5 (large rooms: a finite-size criterion).** The goal is $\liminf_{x\to\infty}\delta(x)/x>0$.

*5a. Observables.* Let $\mathcal A_\rho(B)$ be the bounded gauge-invariant functions of holonomies smoothed at physical scale $\rho$, supported in a region $B$ of $S^3_{R,t}\times\mathbb R$. Wilson loops are bounded: $|\operatorname{tr}U|\le 2$. This algebra is dense enough for line 28's lemma.

*5b. One-scale mixing condition $\mathrm{MS}(\rho,L,\alpha)$.* Let $Q$ be a Euclidean 4-cube of side $L\rho$, tiled into $L^4$ blocks of side $\rho$. Every boundary condition $\omega$ on $\partial Q$ (a gauge class of tangential data) gives a conditional measure $\mu_Q^\omega$. Let $\gamma_b$ be the total-variation influence on $\mathcal A_\rho$(central block) of changing $\omega$ only on boundary block $b$, taken as the supremum over all such pairs of boundary conditions. The condition is
$$
\sum_{b\subset\partial Q}\gamma_b\;\le\;\alpha\;<\;1,
$$
uniformly in the UV cutoff. **The sum runs over blocks of physical size $\rho$, not over lattice links.**

*5c. Key estimate [guess, the Dobrushin mechanism at block scale].* If $\mathrm{MS}(\rho,L,\alpha)$ holds for cubes in $\mathbb R^4$, and for cubes in $S^3_{R,t}\times\mathbb R$ when $R\ge R_1(\rho L)$, then for $F\in\mathcal A_\rho(B_1)$ and $G\in\mathcal A_\rho(B_2)$,
$$
|\mathrm{Cov}(F,G)|\;\le\;C\,\|F\|_\infty\|G\|_\infty\;\alpha^{\,\lfloor \operatorname{dist}(B_1,B_2)/(L\rho)\rfloor},
\qquad
m_\infty \;=\; \frac{\log(1/\alpha)}{L\rho}.
$$
Here $C$ depends on $(L,\alpha)$ only, uniformly in $R\ge R_1$ and in the UV cutoff. Applying this in the Euclidean time direction and using the lemma of line 28 on the dense set $\mathcal A_\rho$ gives $\Delta(R)\ge m_\infty$ for $R\ge R_1$.

*5d. Conversion to $\Lambda$.* Suppose MS holds at a scale $\rho_*$ with $\rho_*\Lambda = y_*$, a fixed number. Then $m_\infty = \Lambda\log(1/\alpha)/(L y_*)$. Steps 3, 4 and 5 together give H3 with
$$
c=\min\Big(1,\ \min_{[x_0,R_1\Lambda]}\delta/(R_1\Lambda),\ \log(1/\alpha)/(Ly_*)\Big)>0 .
$$

*5e. Verifying MS at one scale: the remaining wall, now a single-room statement.* MS is a **finite-volume** statement about a cube of side $L\rho_*$ with all boundary conditions. Every ingredient of it is compact-volume, discrete-spectrum and continuous in parameters. This is the sense in which H3 reduces to a "finite" problem, as in Martinelli–Olivieri. Two sub-conjectures remain:
- [guess] *Sub-conjecture 5.1 (UV smoothness of influence).* At weak bare coupling, a change of boundary data concentrated at the lattice scale has an influence on $\rho$-smoothed observables suppressed by a power of $a/\rho$. So the influence sum is a boundary integral in physical units, and its value does not grow like $(\rho/a)^3$. This is the uniform-in-cutoff "fast modes only dress" content of H2, carried to cubes with boundary. Evidence: the computed M3 dressing (lines 14–15) has exactly this structure on the sphere.
- [guess] *Sub-conjecture 5.2 (one room is large enough).* There is a $y_*$ at which, for some $L$, the influence sum is below $1$. Evidence by induction: lattice spectroscopy shows the correlation length is a fixed multiple of $1/\Lambda$, and influences fall off on that length. At an effective coarse scale where the blocked coupling is strong, lines 3–5 and Shen–Zhu–Zhu give boundary-uniform decay for Wilson-like actions. The programme's carriers give the room-level tool: the lowest levels in a room lie in finitely many identified sectors (lines 17–18), each bounded below by interval Lehmann bounds of the S_C type.

The bridge in 5.2 runs from the small room, where the carrier is known (the $0^{++}$ competitor at $s=1$, H8), to the cube at $y_*$. Following the carrier through that intermediate coupling ($f\gtrsim0.3$ in van den Heuvel's variable) is exactly where no expansion is known to work.

---

## 5. Where it can fail

1. **Sublinear growth of $\delta(x)$.** If $\delta(x)\sim x^{p}$ with $p<1$, then every room has a gap (Steps 1–4 hold) but $\Delta(R)/\Lambda\to0$. This would be observed as $R\Delta(R)$ growing without bound while $\Delta(R)/\Lambda$ falls. The test $R\Delta(R)\to\infty$ cannot detect this failure (one-note §4.3).
2. **Worst-case boundary conditions.** MS asks for *all* boundary conditions. Atypical boundary flux (large holonomies, vortex-like data) might mix slowly even when typical data mix fast. This would be observed as one boundary condition with a near-zero conditional gap. The fix would be a weak-mixing ("almost all boundary conditions") criterion, and none is known for gauge fields.
3. **No local effective theory at scale $\rho_*$.** Bałaban's control is small-field. At $\rho\Lambda = O(1)$, large fields are typical, and Sub-conjecture 5.1 can fail. The influence sum would then scale with the lattice-link count, the required room size would diverge as $a\to0$ (logarithmically if influences are only exponentially small per link), and $c$ would be lost in the continuum.
4. **Vacuum degeneracy from topology.** Fixing the winding sector instead of $\theta$ gives near-degenerate vacua split by tunnelling and a spurious $\Delta\approx0$. The statement avoids this with $\theta = 0$. A regulator that breaks large gauge invariance would reintroduce it.
5. **Failure of Sub-conjecture 1 or 2.** The continuum room Hamiltonian could fail to have compact resolvent, or the family could fail to be continuous in $x$, which would mean a non-analyticity at finite $R$. This would be observed as an accumulation of levels at a finite $x$.
6. **A carrier crossing inside a sector at intermediate $R$**, for example a real version of van den Heuvel's $0^+/2^+$ crossing. This does **not** break H3. It breaks the finite-sector reduction used in 5.2.

---

## 6. Discussion: fixed-scale mixing and the large-room bound

H3's large-room task is to keep the *physical* gap from collapsing as $R\to\infty$. The relevant question is not whether every finite room has a positive gap; Steps 1--4 already separate that finite-room issue from the endpoint. The question is whether a gap estimate can be made uniform in the ultraviolet cutoff and in sufficiently large rooms, with a rate measured in physical units.

The closest proof architecture is the fixed-physical-cube argument of Bhattacharjee, Bhattacharya, Lockwood and Romanov. Their Theorem 45 shows, under their averaged cube condition along a lattice tuning and assuming a continuum limit exists, that the limit has a physical spectral gap. The mechanism is useful for H3: block-scale control yields lattice transfer-matrix decay whose rate may shrink in lattice units but stays positive after conversion to physical units. This is precisely the scale bookkeeping H3's Step 5 needs. [1, Theorems 43 and 45]

The hypotheses are not interchangeable. Their condition is formulated for gauge-fixed link variables on hypercubic lattice blocks, with a Wasserstein/Kantorovich influence and an averaged cube estimate. H3's proposed $\mathrm{MS}(\rho,L,\alpha)$ instead uses worst-case total-variation influence between gauge-class boundary data and a smoothed central-block observable, on the Berger-room family. A theorem transferring the former criterion to the latter object is not supplied by that paper. H3 must either prove its own block criterion or establish a precise comparison of metrics, conditioning, averaging, and geometry.

There is also a useful warning in the same source: the cube-gap condition alone does not guarantee that a continuum limit is nontrivial; the paper constructs strong-coupling sequences satisfying its gap condition whose limits are trivial. [1, Proposition 192] For H3, this does not invalidate the proposed gap route. It clarifies its scope: the criterion can address the spectral lower bound conditional on the continuum room theory already existing, while the construction and nontriviality inputs belong to other parts of the program.

The large-room implication in H3 §5c should therefore be stated as a separate lemma with explicit hypotheses. A suitable version must provide: (i) a block specification with a uniform conditional mixing bound for every relevant translated block and exterior condition; (ii) an iteratable influence estimate, such as a weighted Dobrushin--Shlosman matrix whose row sums are bounded strictly below one; (iii) a resulting covariance bound for the stated observable class, with constants uniform in cutoff and $R\ge R_1$; and (iv) reflection positivity/transfer-matrix assumptions plus density of those observables in the vacuum-orthogonal Hilbert space. Only after these steps does Euclidean-time decay imply $\Delta(R)\ge m_\infty$ for the full physical spectrum. This identifies exactly what the current one-cube sketch compresses into “the Dobrushin mechanism at block scale.”

Related results support parts of the landscape but not this missing H3 estimate. Shen, Zhu and Zhu obtain uniqueness, functional inequalities and exponential correlation decay for lattice Yang--Mills in an explicit strong-coupling regime; that is a different coupling regime from H3's UV trajectory at fixed room. [2] Adhikari and Cao prove decay for finite (possibly non-Abelian) gauge groups at weak coupling, but their structure group is finite, not continuous $SU(2)$, and their result does not give H3's cutoff-uniform Berger-family bound. [3] These results support correlation decay as a productive language for a gap argument while leaving H3's specific uniformity problem untouched.

The synthesis is therefore narrow: existing work gives a rigorous model for turning fixed-physical-scale mixing into a physical spectral gap. H3's distinctive mathematical obligation is to build that bridge for its own geometry and its full physical Hilbert space, uniformly along the cutoff and large-room limits. The next tractable target is not a new numerical scan over $R$, but a precise H3 block-influence lemma, followed by a proof that its hypotheses hold at one scale $\rho_*=y_*/\Lambda$ uniformly in the required limits.

### References for this discussion

1. D. Bhattacharjee, U. Bhattacharya, J. M. Lockwood and T. Romanov, “One Cube Estimate Implies the Yang--Mills Mass Gap,” version 7 (2026), especially Theorems 43 and 45 and Proposition 192. [Paper and version record](https://hexagonmath.org/2610.00341v7); [PDF](https://hexagonmath.org/pdf/2610.00341v7); [formalization and source release](https://zenodo.org/records/23895262).
2. H. Shen, R. Zhu and X. Zhu, “A stochastic analysis approach to lattice Yang--Mills at strong coupling,” *Communications in Mathematical Physics* 400 (2023), 805--851. [arXiv:2204.12737](https://arxiv.org/abs/2204.12737); [journal DOI](https://doi.org/10.1007/s00220-022-04609-1).
3. A. Adhikari and S. Cao, “Correlation decay for finite lattice gauge theories at weak coupling,” *Annals of Probability* 53 (2025), 140--174. [arXiv:2202.10375](https://arxiv.org/abs/2202.10375); [journal DOI](https://doi.org/10.1214/24-AOP1702).

## 7. Smallest next step

**Target lemma (finite-size criterion for lattice gauge theory, at fixed $\beta$).** Let $\mu^\omega_{Q}$ be the SU(2) Wilson measure at any inverse coupling $\beta$ on a lattice cube $Q\subset\mathbb Z^4$ with boundary links $\omega$. Suppose that for some side $\ell$, the influences $\gamma_b$ of boundary blocks $b$ (of side $\ell_0\mid\ell$) on bounded gauge-invariant functions of the central block satisfy $\sum_{b\subset\partial Q_\ell}\gamma_b\le\alpha<1$, uniformly in $\omega$. Then the infinite-volume gauge-invariant Gibbs state is unique, and for bounded gauge-invariant $F$ and $G$,
$$|\mathrm{Cov}(F,G)|\le C(\ell,\alpha)\,\|F\|_\infty\|G\|_\infty\,e^{-\operatorname{dist}(F,G)\log(1/\alpha)/\ell},$$
with $C$ and the rate depending **only on $(\ell,\ell_0,\alpha)$, not on $\beta$**.

This is a fixed-spacing, compact-spin, finite-range statement, the kind Dobrushin–Shlosman methods prove. The new work is handling gauge invariance, for example axial gauge inside $Q$ or a gauge-invariant definition of $\gamma_b$. With Steps 0–4, the lemma moves all of H3 into one finite-volume hypothesis plus Sub-conjecture 5.1, which is the uniform-in-$a$ smallness of block influences.

---

### Flag

No new requirement. One reading point: the Jaffe–Witten statement requires "some constant $\Delta>0$ such that every excitation of the vacuum has energy at least $\Delta$". That is why §1 defines $\Delta(R)$ over the whole physical Hilbert space, not over the scalar sector named in line 20's wording.
