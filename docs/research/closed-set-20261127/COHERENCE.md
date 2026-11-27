# Coherence report: closed working set H2–H8 (27 November 2026)

*Referee pass over H2-M3-uniform-dressing.md, H3-M2-infrared.md, H4-continuum-limit.md, H5-one-geometry.md, H6-local-fields.md, H7-every-group.md, H8-s1-carrier.md against the fixed 29-line page. No section was edited. I re-checked every number marked "check" below by hand and with a short pure-Python script (Jacobi diagonalisation of the full 9×9 Hessian at $q_{\rm MC}$).*

Severity labels: **[A]** breaks a deduction on the page or makes two sections contradict each other; **[B]** a gap or drift that has to be resolved in the text; **[C]** an editorial or minor mathematical fix.

---

## 0. Summary

1. **[A] The two sections disagree about what the $s=1$ carrier is.** H8 works with the undressed left-invariant operator on all of $\mathbb R^9$. There the first excitation is the ground state of the second well ($q_{\rm MC}$), at offset $\omega_C$, and the margin is decided at harmonic order. H2, H7 (Step 6) and H3 (Step 3) work around $q=0$, with $q_{\rm MC}$ treated as a gauge copy. There the charge-0 and charge-$\pm2$ two-quantum states sit together at $2/\sqrt t$, and the order is decided only at $O(g^2)$. Both computations are correct for their own operator. But H8's proved margin rests on a level that H2 (citing Corollary p4cor:minus2) and H8's own failure mode 2 call an artefact of the truncation. **In the physical theory H8 is open at $O(g^2)$ and needs H2b. It is not $[P\text{-sketch}]$.** Section 5 has the details.
2. **[A] The IVT for line 18 at the registered couplings does not bracket.** The registered $s^*=1.1674,1.1809,1.1931$ all lie *above* $s_{\rm controls}=1.15$. So at $g^2=0.03$–$0.07$ the competitor is already lowest at $s=1.15$, both ends of $[1,1.15]$ have charge 0 lowest, and the IVT gives nothing. Line 18 needs, at each such $g$, a control at some $s_1\in(s^*,(t+2)/(t+1))=(s^*,1.25)$ where Helike is lowest. Line 19 then has to be stated on $[1,s_1]$. Neither line 12 nor H8 supplies this.
3. **[A] Circularity between H4(b) and H5.** H4 Step 5 gets the $L\to\infty$ limit from a torus gap "$\Delta(L)\ge c\Lambda$, line 20 via line 29". H5 identifies continuum $\mathbb R^4$ limits only; it does not transport the sphere gap to the torus. H5 Step 7 / (c3) in turn takes uniqueness of the $\mathbb R^4$ limit from H4. So H4(b) ⇐ H5 ⇐ H4. On top of the cycle, no hypothesis provides a torus gap at all.
4. **[A] H5 Step 2 omits a marginal counterterm.** On $S^3\times\mathbb R$ the symmetry is (isometries of $S^3_t$) × (time translations, $\Theta$). Nothing mixes $\tau$ with space, so $\operatorname{tr}F_{0i}^2$ and $\operatorname{tr}F_{ij}^2$ are *separately* allowed marginal operators. The claim "no relevant or marginal counterterms except $g_0^{-2}$ and c-numbers" is false for both of H5's regulators. H2 already identifies the needed second parameter $c_0(\Lambda)$ (speed of light). H5(a)'s "a bare coupling $g_0(\varepsilon)$" has to become $(g_0(\varepsilon),c_0(\varepsilon))$.
5. **[B] Line 26 is not closed from H6 as stated.** H6 shows that a generalized free field can reproduce the $1/\ln^2$ two-point function. The corrected criterion also needs $\langle\Theta\Theta\Theta\rangle_T\neq0$ at leading order, i.e. the free Wick triangle $T_O\not\equiv0$, which is uncomputed. Alternatively, line 26 can restrict "free theory" to Wick-polynomial theories.
6. **[B] Unique vacuum of the *continuum* finite-volume theory has no route.** Line 28 cites the lattice positivity-improving transfer matrix [PC]. That property does not survive limits (H5 says so too). H4(a) asserts "$\Omega_L$ unique" and H5(a) asserts "$\ker H_R=\mathbb C\Omega_R$". H5 Step 3 takes this from H3, while H3 Step 1 assumes it (Sub-conjecture 1). The clause is double-booked and proved by neither.
7. **[C] Concrete fixes:** H7's Young weight $(4\alpha-1)/2$ should be $(2\alpha-1)/2$. H6's $\Theta_a$ sums over $p\ni x$, which counts each plaquette four times. H5 includes bare Wilson loops, which are not finite in 4-D. Line 14's sign, as H2 reads it, has to be fixed on the page.
8. **The H8 harmonic spectrum is confirmed exactly** (Section 5). Every number checks.

---

## 1. Interfaces: does each deduced line get what it needs?

### Line 5 (Hamiltonian lattice gap, strong coupling)
- It needs no H. H7 Step 1 carries line 4 to every $G$ ($E_1-E_0=2C_{\min}(G)g^2/a$). The proof is correct, but it assumes the periodic lattice side is $\ge4$. Otherwise a Polyakov loop of length $<4$ is cheaper than a plaquette. **[C]** Add "$L\ge4$".
- **Mismatch with line 20's sketch [B].** H3 (§2, Step 5e) and H5 (§2) both say lines 3–5 are *not* the large-$R$ end of the continuum family. At every fixed $R$ the continuum limit sends $g_0\to0$. So the page's sketch "lattice end lines 3–5" is not a valid endpoint, and H3 has replaced it with the finite-size mixing criterion MS. The page sketch should be updated so it does not cite 3–5 as an endpoint.

### Line 12 (Helike below competitor at 3 of 4 controls, conditional on H1)
- It needs H1 (not in this set) and nothing from H2–H8 *inside the reduced model*.
- **[B]** H2 §1.4: the sources have no gauge-invariant lift to the full theory on the page. So line 12 is a statement about the reduced model only, and its full-theory meaning is undefined until a lift is chosen.
- **[B]** "3 of 4 controls": H8 lists the control couplings as $g^2\in\{0.002,0.004\}$, which is two values. The page should say which four controls these are and which one fails, because the IVT at a given $g$ needs that $g$'s control to pass.

### Line 18, existence of a switch (IVT + 17 + 19, given H8)
- **Within the reduced model at weak coupling ($g^2\lesssim0.0075$ at $t=3$) the deduction works.** H8 at $s=1$ gives $\Delta_C<\Delta_H$. Line 12 gives $\Delta_H<\Delta_C$ at $s=1.15$. Line 19 gives continuity and line 17 gives an exact crossing.
- **[A] At the registered couplings it fails as written.**
  - From the page's own numbers, $\omega_C-\omega_H=0.01655$ at $s=1.15$, and the anharmonic $c_C-c_H=-2.208$ quoted in H8.
  - So the competitor drops below Helike at $s=1.15$ once $g^2\gtrsim0.0075$. That is consistent with $s^*(0.03)=1.1674>1.15$.
  - For $g^2\in\{0.03,0.05,0.07\}$ both endpoints of $[1,1.15]$ have charge 0 lowest.
  - **Needed:** a computed control at $s_1\in(s^*(g),1.25)$ with $\Delta_H<\Delta_C$, at the same $g$, and line 19 on $[1,s_1]$. Line 19's argument covers any $s_1$ inside Theorem A's window, so only the computation is missing.
  - H8 §1 correctly notes that "both ends must be at the same $g$" but misses this $s$-range issue.
- **[B] The registered $s^*$ are incomplete at their own order.** By H2 (Interface, line 15), the $O(g^2)$ level shift has a kinetic part $-m^{\rm ren}_{uu}\omega_k$ that has not been computed. Line 15's widening from $0.0165$ to $0.0191$ at $g^2=0.002$ is about $+1.3$ per unit $g^2$ in the separation, and it is the potential part only.
  - My check: the registered shifts $(s^*-s^*_0)/g^2=0.79,0.75,0.71$ match the pure anharmonic prediction $(c_H-c_C)/\partial_s(\omega_C-\omega_H)=2.208/2.569=0.86$ at leading order.
  - So the registered $s^*$ almost certainly **exclude** the line-15 fast dressing. If the dressing were included, the slope would be about $0.35$.
  - The page should say which Hamiltonian the predictions refer to.

### Line 19 (source bridge $s\to1^+$)
- **[P] confirmed:**
  - The sources vanish at $s=1$.
  - $\kappa=\min(\omega_k^2-{\rm src}_k)$ gives $\omega_1^2-a=s(t+2-2s)$ and $\omega_2^2-b=s[(t+2)-s(t+1)]/t$. These are $t$ and $1/t$ at $s=1$, and the window is $s<(t+2)/(t+1)$.
  - $H(s)-H(1)=-\frac12\sum_k{\rm src}_k(s)\|q_k\|^2$ is quadratic. It is $H(1)$-bounded because $V_1\ge\frac\kappa2\|q\|^2-C$, so this is a type-(A) analytic family.
- Two caveats, both correctly made by H8 and H2, that the page should carry:
  1. Only the *ordered* eigenvalues are continuous through the charge-0 avoided crossing near $s=1+\varepsilon^*$, $\varepsilon^*\approx\frac{2t}{t+2}\omega_Cg^2$ (I confirm $0.951\,g^2$ at $t=3$). The branch identity "competitor at 1.15 ↦ second charge-0 level at 1" is false at small $g$.
  2. H2: a full-theory version of the sourced background cannot be continued to $s=1$, because Faddeev–Popov near-kernel in sector 2 gives $\gamma(s)\lesssim|s-1|$.
- Line 19 is fine as a reduced-model statement. It is not a bridge in the full theory.

### Line 26 (non-triviality, from 25)
- **[B]** As above. H6(b3)+(c) gives $|x|^8G_\Theta\to C_O(N^2-1)/(16\ln^2(1/|x|\Lambda))$. I check the constant: $(b_0/32\pi^2)(8\pi^2/b_0)=1/4$, which squares to $1/16$.
- That excludes Wick-polynomial free theories (H6's lemma is correct) but not generalized free fields.
- Either add the finite computation $T_O\not\equiv0$ to line 26, or define "free theory" as Wick-polynomial on the page.
- The page's "closed from 25" is currently an overstatement.

### Line 27 (unique vacuum, from 28)
- Correct as a deduction, with one wording point **[C]**.
- It follows from line 28's *hypothesis*, not its conclusion. A truncated decay $\le C_Ae^{-m\tau}$ on a dense set forces $\nu_A(\{0\})=0$ for $(A-\langle A\rangle)\Omega$, hence $\ker H\cap\Omega^\perp=0$.
- The conclusion $\operatorname{spec}H\cap(0,m)=\emptyset$ alone does not exclude a degenerate eigenvalue at 0.
- H4 offers clustering (e) as an alternative route. That is consistent.

### Line 28 (gap survives the limit)
Needs: (a) a unique vacuum at each $R$; (b) an $R$-uniform rate; (c) uniform constants $\sup\|(A-\langle A\rangle)\Omega\|^2$; (d) convergence of truncated two-point functions on a dense set; (e) all of these **in the same family**.
- (b) H3 supplies $\Delta(R)\ge c\Lambda$ for every $R$ on the Berger sphere. H3 correctly notes that only $\liminf\Delta(R_n)$ along the convergent sequence is needed.
- (c) **Both H4 and H5 supply it**, by different means. H4: bounded flowed observables, $\mathrm{Var}\le\|\Phi\|_\infty^2$, free of charge. H5: (b)'s uniform bound $\sup_{R\ge R_0}|S_R|<\infty$ for each fixed observable. Either suffices. The question raised in the task ("does H4 or H5 provide a uniform two-point bound?") is answered: **yes, both**, provided the observables are bounded (H4) or (b) holds (H5).
- (d), (e) **[A] Family mismatch.**
  - The page's line 28 is written on the *sphere* ($H_R$, rate from line 20).
  - H4's Interface reads it on the *torus*, with convergence $\langle\Theta F;\tau_\tau F\rangle_L$ as $L\to\infty$, using a torus gap that nobody supplies.
  - The consistent route is the sphere: gap from H3, convergence from H5(b). H4's role is then only to identify the $\mathbb R^4$ theory (H5 (c3)).
  - H4 §2 and Step 5 should be rewritten to say that line 28 runs along the sphere family.
- (a) **[B]** Simplicity of $\ker H_R$ in the continuum: no route (Summary item 6). Put it inside H3, since $\Delta(R)>0$ over $\{\Omega_R\}^\perp$ already forces it. Then delete it from H5(a) and H4(a), or give it its own sub-conjecture.
- **[B] Observable class.** H5's dense set uses bare Wilson loops. In 4-D these need perimeter renormalisation and have $\langle W_C\rangle\to0$ as $a\to0$ (H4 §1 says exactly this). Replace them with flowed Wilson loops, H4's class. H6(a)'s "jointly with the Wilson-loop Schwinger functions" needs the same fix.

### Lines 20 ← 16 (H3 ← H2)
- H3 §2 asks for a "strengthened H2" with (i) units $1/R$, (ii) a high-block lower bound, (iii) off-diagonal control. **H2 as now written already supplies all three**: it is stated spectrally (ii)/(iii), with a Feshbach resolvent bound in Step 3 and units of $1/R$. H3's remark that "H2 bounds only the compressed block" is out of date. **[C]** Update H3 §2.
- **[B]** H3 Step 3 needs $\delta_0$ to be the lowest level in H2's window. H2 §5.1 says it is unverified whether $E_{\max}(3)=\min(2\omega_f,\omega_f+\omega_s)$ lies above the two-quantum slow level $2/\sqrt3$. Until that finite check is done, H3's small-$R$ end is conditional on it.
- **[C]** H3 allows $t=1$ ("with $t=1$ the round sphere"). H2 excludes $t=1$, where the window is empty. H3 should fix $t\in T\not\ni1$, i.e. $t=3$.

### Line 23 ← H4, H6
- H4(g) and H6(a) are one statement split across two sections. Both say so, which is consistent. H6 adds a needed rider: H4 must include the renormalised plaquette observables $Z(a)a^{-d}P_a$, or H6(a) is an extension of H4 rather than a consequence of it.
- H4 also needs SC-loc to apply line 22 to flowed (non-local) observables. That is correctly placed inside H4.

---

## 2. Circularity

### Dependency graph (arrows mean "the best-guess proof uses")

```
                 page lines 7-9,13,14,17,19  (established)
                     |            |
                     v            v
        H8 <---(physical meaning)--- H2 ----(global def. of H_{R,Lambda})----> H5(a)
        |                             ^                                       |  ^
        | (route only, H3 Step 3)     | (strengthened, small-R end)           |  | simple vacuum, general R
        v                             |                                       v  |
        H3 ---------------------------+------- existence of H_R, Lambda <---- H5(a)
        |  ^                                                                    
        |  +---- H5(b) Step 6 uses H3's tau-decay
        |
        +----> line 28 <---- H5(b) convergence + uniform bound
                 ^
                 +---- H4 (identifies the R^4 theory)

   H4(b) Step 5 ----"torus gap via line 29"----> H5 ----(c3), Step 7 uniqueness----> H4      <- CYCLE 1
   H5(b) Step 6ii --stress tensor (Suzuki)-----> H6 ----Step 5 uses sphere at R~|x|--> H5(b)  <- CYCLE 2
   H4(g) <----------------------------------- joint ----------------------------------> H6(a) (declared joint)
   H3 Step 1 assumes simple vacuum (Sub-conj 1);  H5 Step 3 takes simple vacuum from H3  <- DOUBLE-BOOKED
   H2(i) (UV convergence of low eigenvalues at small R)  overlaps  H5(a) at small R      <- REDUNDANT
   H7 -> every other H, read through D_G (transport; no back-edges)
   H6 -> H3, H5 (Lambda, line of constant physics);  H4 Step 7 -> H6 (non-Gaussianity)
```

### Findings
- **Cycle 1 [A]: H4(b) ⇄ H5.** It is vicious as written. Two ways to break it:
  - (i) Drop H4 Step 5, take the infinite-volume theory as the sphere limit H5(b), and use the box only for (c3). Then (c3) needs uniqueness of the box limit, which is H4(b) proved some other way.
  - (ii) Add a torus version of H3 as a new hypothesis.
  - Option (i) matches the page's line 28, which is already written on the sphere.
- **Cycle 2 [B]: H5(b) ⇄ H6.**
  - H5's metric interpolation (Step 6ii) inserts the stress tensor on the *curved* background. H6(b) constructs $T_{\mu\nu}$ only on flat space or the torus.
  - H6 Step 5 uses H5 for the sphere at radius $|x|$.
  - Fix: H5(b) should use a flowed-observable comparison that does not need $T_{\mu\nu}$, or H6(b) must be stated on $S^3_R\times\mathbb R$ as well.
- **H3 ⇄ H5(a) on the simple vacuum [B].** This is not a logical cycle, since H3 *assumes* the clause. But the clause has no proof route anywhere. Move it into H3's statement (it is implied by $\Delta(R)>0$ as H3 defines it) and remove it from H5(a).
- **H2 ⇄ H5 [C].** H2 defers the global definition of $H_{R,\Lambda}$ to H5. H5 uses H2 for the small-$R$ simple vacuum. H2(i) duplicates H5(a)'s UV existence at small $R$. Not vicious. Say once which hypothesis owns small-$R$ UV existence.
- **H3 is not circular with line 28.** H3 Step 5c uses only the *lemma* half of line 28 [P]. That is clean.
- **H8 has no H-dependencies in its proof.** It uses only lines 7–9, 17 and 19. It is the only fully self-contained section.
- **H7's route form overloads the chain [B].** It lists H1(G) and H8(G) as part of the route. H3 Step 2 shows carrier persistence is *not* needed for continuity of $\delta$: the minimum of continuous functions is continuous, and avoided crossings are harmless. So Clay(G) rests on H2(G)–H6(G) only. H7's "stronger need" (following carriers with $C$ labels for $\mathrm{Out}(\mathfrak g)\ne1$) applies only to H3's optional finite-sector reduction in 5.2. As written it contradicts H3 Step 2.

---

## 3. Statement drift (precise statement vs page line)

| Section | Page line | Drift | Verdict |
|---|---|---|---|
| H2 | 16: $\lVert P_{\rm low}(H_\Lambda-H_{\rm eff})P_{\rm low}\rVert\le Cg^2$ uniformly in $\Lambda_{\rm cut}$ | (a) Replaced by spectral (i)–(iii), with the literal form shown to be nearly a definition. (b) Restricted to $s=1$, a chart $U$ around $q=0$ excluding $q_{\rm MC}$, $t\in T\not\ni1$, $g(R)\le g_*$. (c) Adds H2b with $o(g^2)$. (d) Adds a second bare parameter $c_0$. (e) Reinterprets line 14's sign. | **Different and justified.** The spectral form is what the deductions use. But the domain is *weaker* than line 16: there is no sourced case, and lines 15 and 18 at $s\ne1$ lose their M3 interface (§1.4 needs a lift). The page should adopt H2's form and record the restriction. |
| H3 | 20: $\Delta(R)/\Lambda\ge c$ for every $R$, "starting from the unsourced room with the scalar carrier" | Gap over the whole physical Hilbert space, with the carrier moved to the route. $\Lambda$ fixed by a two-loop definition, and $g^2(R)$ re-read as a parametrisation. $t$ fixed. | **Faithful.** It matches line 28's need and the Jaffe–Witten wording. The route on the page (lines 12–19, then 3–5) is **not** H3's route. Lines 12 and 18 are not inputs (H2, H8 Interface 2), and 3–5 are not an endpoint. Update the sketch, not the requirement. |
| H4 | 23: limit exists, Euclidean symmetry; with 22, OS; relativistic reconstruction | Split into (a)–(g). The observables are flowed bounded ones. Two-loop scale setting. (g) is shared with H6. | **Faithful, with a rider.** Line 22's [P] does not cover flowed observables, so "with 22" needs SC-loc (placed inside H4, as it should be). (a) adds "Ω_L unique" with no route (§1, line 28). |
| H5 | 29: $S^3_R\times\mathbb R$ exists for every $R$; →$\mathbb R^4$ on compacta; sphere cutoff and box give the same theory up to $\Lambda$ | Adds a simple vacuum, regulator independence among admissible families, isometry invariance, and uniform bounds. Restricted to Berger $h_t$. Wrong counterterm count (Summary item 4). Bare Wilson loops. | **Stronger** in (a) (simple vacuum: move it to H3) and **wrong** in the one-parameter renormalisation. It also omits $O(4)$, correctly assigning that to H4. |
| H6 | 25: local fields for gauge-invariant polynomials in $F$, short-distance agreement with PT and AF | Adds stress tensor and trace anomaly (b), OS0′, and asymptotic (not convergent) agreement (c). | **Stronger but required.** (b) is needed for line 26 and is in Jaffe–Witten. Fine. Line 26 additionally needs $T_O\ne0$. |
| H7 | 24: every compact simple $G$ | $G$ simply connected, local sector shared with quotients, $\theta=0$. Route form adds H1(G) and H8(G). | **Faithful** in (i)/(ii). The route form overreaches (§2). |
| H8 | 18 (part): at $s=1$ the carrier is the charge-0 competitor, small-room $0^{++}<2^{++}$ | Stated for the *whole undressed reduced operator on $\mathbb R^9$*, where the charge-0 excitation is the second well's vacuum, not a competitor quantum. Stronger ($\min_{m\ne0}$, simple $E_0$). Split into H8$_\infty$ and H8$_{\rm reg}$. | **Different operator.** True as stated for that operator. In the physical theory (q_MC identified with 0) the line-18 claim is H2's $O(g^2)$ question and is not decided. See Section 5. |

---

## 4. Mathematical errors and fixes

1. **H7, Step 3, Young weights [C].** For $gS|\mathcal C|\le gS\,c\|q\|^{3-4\alpha}Q^\alpha$ split as $(g^2Q)^{w_1}(\|q\|^2)^{w_2}(g^{-2})^{w_3}$, matching powers gives $w_1=\alpha$, $w_2=(3-4\alpha)/2$, $w_3=(2\alpha-1)/2$. These sum to 1, and $w_3>0$ iff $\alpha>\frac12$. H7 writes $(4\alpha-1)/2$, which gives a sum of $1+\alpha$. The conclusion stands with the corrected weight. (At $\alpha=\frac34$, $w_2=0$, and the $\frac\kappa4\|q\|^2$ term is not even needed.)
2. **H6, $\Theta_a$ normalisation [C].** The check $\sum_{6}s_p=\frac{a^4g_0^2}{16}O$ is correct for the six plaquettes *based* at $x$. I verified it: $s_p=\frac{a^4g^2}8\sum_a(F^a_{\mu\nu})^2$ per plane. But $\Theta_a$ is defined with $\sum_{p\ni x}$, and 24 plaquettes contain a site in 4-D, so the sum is $4\times$ too large. Step 4's integrated identity $\int a^{-4}\sum_{p\in\text{based at }y}=S_W$ also needs the based-at convention. Fix: "the six plaquettes based at $x$", or $\frac14\sum_{p\ni x}$.
3. **H5, observables [B].** Bare Wilson loops are not finite in the 4-D continuum. They have a perimeter divergence, cusp anomalous dimensions, and $\langle W\rangle\to0$. The statement "need no renormalization beyond the coupling" is false for them. Use flowed loops, as H4 does.
4. **H5, Step 2, counterterms [A].** See Summary item 4. Missing: the electric/magnetic anisotropy $c_0$, which is marginal on $S^3\times\mathbb R$ and on any (cell complex)×$\varepsilon\mathbb Z$ regulator. H2 §1.1 and its Flag already contain the fix.
5. **Page line 14, sign [B] (raised by H2).**
   - Let the classical potential be $\mathcal S(F)/g_0^2$. Finiteness requires $1/g_0^2=1/g^2-(\text{coefficient of }\log\Lambda_{\rm cut}\text{ in }E_0)$.
   - Asymptotic freedom ($1/g_0^2\to+\infty$) therefore needs that coefficient to be **negative**. This agrees with the Nielsen–Hughes paramagnetic dominance: the vacuum energy in a background field has $-\tfrac{11N}{48\pi^2}g^2B^2\ln\Lambda^2$.
   - Line 14's "$+\frac{b_0}{8\pi^2}\log(\Lambda_{\rm cut}R)S_{\rm cl}$" is consistent only if $S_{\rm cl}$ or $E_0$ carries a compensating sign convention.
   - H2 silently reads it as "$-$". The page should state the sign or the convention explicitly.
6. **H8, Step 4(b), level list [C].** $\Delta_C\to\min(\omega_C,2\sqrt t)$ omits the $q=0$ well's charge-0 two-quantum level at $2/\sqrt t$. The result is unchanged because $\omega_C<2/\sqrt t$ for all $t$. Write $\min(\omega_C,2/\sqrt t,2\sqrt t)$.
7. **H3, Step 3 cites H8 wrongly [B].** H3 says H8 shows "the scalar ($0^{++}$) combination [of the two-quantum level] is the lowest". H8 says something else: the first excitation is the $q_{\rm MC}$-well vacuum at $\omega_C$, and the two-quantum states at $q=0$ are degenerate at harmonic order. H3's $\delta_0$ follows H2's chart. Fix H3's attribution. H3 needs only $\delta_0>0$, which is unaffected.
8. **H3, Step 2, "finitely many sectors" [C].** The isometry labels are unbounded. Continuity of $\delta$ as a minimum needs an argument that only finitely many sectors can come near the minimum on a compact $x$-range, for example a sector-uniform lower bound that grows with the label. Add it as part of Sub-conjecture 2.
9. **H3, Step 5c needs cube measures with boundary conditions [B].** MS is stated for continuum cubes in $\mathbb R^4$ and $S^3_{R,t}\times\mathbb R$ with arbitrary boundary data. No hypothesis supplies such conditional measures uniformly in the cutoff. H4 is periodic and H5 has no boundary. H3's Smallest next step (fixed-$\beta$ lattice) plus Sub-conjecture 5.1 is where this has to live. Say so in H3's statement of what it takes.
10. **H7 vs H3 on exact crossings [B].** See §2. H7 §2 "Line 20 assumes crossings along scale are exact" contradicts H3 Step 2.
11. **H8 Step 7, unsourced coefficients [C].** $c_H=-0.530$ and $c_C=-2.738$ at $s=1.15$ are called "the page's exact coefficients", but they are not on the 29-line page. Cite the computation they come from. They are consistent with the registered $s^*$ (see §1, line 18).
12. **Notation [C].** H5 uses $\beta_0,\beta_1$ for the beta-function coefficients. H6 uses $\beta_0=4/g_0^2$ for the lattice inverse coupling, and the page uses $\beta=4/g^2$. Use $b_0,b_1$ everywhere for the coefficients.
13. **Constants I checked and found consistent across sections:**
    - $\Lambda$ two-loop definitions in H3, H4 and H5 agree. I re-derived $1/g^2=(b_0/8\pi^2)\ell+(b_1/16\pi^2b_0)\ln(2\ell)$ from H5's $\Lambda_S$.
    - $b_1=136/3$ (SU(2)) and $34(h^\vee)^2/3$ (H7).
    - $d_S\leftrightarrow r_S$ in H5 Step 5.
    - $-d\beta_0/d\ln a=b_0/2\pi^2$ and $d\ln F/d\beta_0=-2\pi^2/b_0$ in H6.
    - The sign of the Callan–Symanzik identity in H6 Step 4.
    - $\Delta/\Lambda=\delta(x)/x$ and $c$ in H3 Step 5d.
    - $\sum_mKg_m^22^{-m}\le Kg^2(R)$ in H2 Step 5.
    - The scalar Laplacian bound $\gamma(0)^2\ge\min(2+1/t,3)$ in H2.
    - Long-root decomposition dimensions in H7 ($(N-2)^2$, $G_2$: 3, $E_8$: 133).
    - $C_{\min}$: SU(2) $3/4$, $G_2$ 2, $E_8$ 30.
    - $\sigma_{\rm adj}/\sigma_{\rm min}$: $8/3$, $9/4$, $2$.
    - The centraliser Hessian $\omega_2^2-b=0.15333$, frequency $0.39158$.
    - Line 6 versus H7: $2C_2/\beta=g^2C_2/2$.

---

## 5. H8: independent re-derivation of the harmonic spectrum at $q_{\rm MC}$

**Setup.** Set $g=1$, $u=t^{-1/2}$, $\omega=(\sqrt t,u,u)$, $W=\frac12\sum_k\omega_k\|q_k\|^2-\det q$, $m=\operatorname{diag}(u,1,1)$.
- $m$ is a zero of $\nabla W$: $\omega\circ m=(1,u,u)=\operatorname{cof}m$.
- $V_0=\frac12|\nabla W|^2$. Along the line, $\nabla W(\lambda m)=(\lambda-\lambda^2)\operatorname{diag}(1,u,u)$, so $V_0=(1+\frac2t)(\lambda-\lambda^2)^2/2$, which is line 8. Also $W(\lambda m)=u(3\lambda^2-2\lambda^3)/2=\mathrm{CS}/(2\sqrt t)$.
- $\Delta W=3S$ (det is harmonic), so $H=\frac12\sum a_i^*a_i+\frac32S$ exactly. I verified this by expanding $a^*a=-\partial^2+W'^2-W''$.

**Hessian.** $J=\operatorname{Hess}W=\operatorname{diag}(\omega)\otimes1-\operatorname{Hess}\det|_m$. At a diagonal $m$:
- $\partial^2\det/\partial q_{aa}\partial q_{bb}=m_l$.
- $\partial^2\det/\partial q_{ab}\partial q_{ba}=-m_l$, with $l$ the third index.

So:
- **Diagonal block** $\begin{pmatrix}\sqrt t&-1&-1\\-1&u&-u\\-1&-u&u\end{pmatrix}$. Eigenvalue $2u$ on $(0,1,-1)$. On $\{e_1,(e_2+e_3)/\sqrt2\}$ the block is $\begin{pmatrix}\sqrt t&-\sqrt2\\-\sqrt2&0\end{pmatrix}$, so $\mu_\pm=\frac12(\sqrt t\pm\sqrt{t+8})$.
- **Pairs** $(q_{kc},q_{ck})$: $\begin{pmatrix}\omega_k&m_l\\m_l&\omega_c\end{pmatrix}$, with determinant $\omega_k\omega_c-m_l^2=0$ for all three pairs ($\sqrt t\cdot u-1=0$, $u^2-u^2=0$). The eigenvalues are $0$ (gauge) and $\omega_k+\omega_c$, i.e. $\sqrt t+u$ (twice) and $2u$ (on $E_{23}+E_{32}$).
- **Charges** under $q\mapsto R_\varphi^TqR_\varphi$: $\{E_{22}-E_{33},E_{23}+E_{32}\}$ is the spin-2 doublet ($\pm2$). $E_{11}$, $E_{22}+E_{33}$ and $E_{23}-E_{32}$ (gauge) have charge 0. $\{E_{12},E_{13}\}$ and $\{E_{21},E_{31}\}$ have charge $\pm1$.

**Numbers ($t=3$, full 9×9 Jacobi diagonalisation):** eigenvalues of $J$ are
$$\{-0.79229,\ 0,\ 0,\ 0,\ 1.15470,\ 1.15470,\ 2.30940,\ 2.30940,\ 2.52434\},\qquad \operatorname{tr}J=8.66025=3S .$$
Frequencies are $|\mathrm{eig}\,J|$ because $\operatorname{Hess}V_0=J^2$ at a zero:
- competitor (charge 0): $\omega_C=0.79229=\frac12(\sqrt{11}-\sqrt3)$;
- Helike (charge $\pm2$): $\omega_H=1.15470=2/\sqrt3$;
- charge $\pm1$ pair: $2.30940$;
- breathing partner: $\mu_+=2.52434$.

**Checks against the page and other sections:**
- $\omega_H^2=4/t$ matches $2s(1+2/t-s)$ at $s=1$.
- At $s=1.15$, my charge-0 block $\begin{pmatrix}5.75&-2.5352\\-2.5352&2.645\end{pmatrix}$ gives $1.106657$ and $2.677744$, and Helike is $1.090107$. Separation $0.016550$, which matches lines 15 and 12.
- H8's $Q_s$ at $s=1$ reproduces $J_{\rm block}^2=\begin{pmatrix}t+2&-\sqrt{2t}\\-\sqrt{2t}&2\end{pmatrix}$ exactly.

**Confirmed conclusions:**
- (i) $\omega_C<\omega_H$ for every $t>0$, since this is equivalent to $\sqrt{t+8}>\sqrt t$, and $\omega_C$ is the lowest normal mode at $q_{\rm MC}$.
- (ii) The zero-point offset between the wells is exactly $\omega_C$: $\frac12(\sum|\mathrm{eig}|-\sum\mathrm{eig})=0.792287$.
- (iii) Harmonic margin $2/\sqrt3-\omega_C=0.362414$. Threshold $0.3624/0.07=5.18$.
- (iv) Harmonic switch $s^*_0(3)=1.143568$, where $\omega_C(s)-\omega_H(s)=0$ to $10^{-15}$, with slope $\partial_s(\omega_C-\omega_H)=2.569$.
- (v) Agmon bounds $0.28868\le d\le0.32867$.
- (vi) $V(q_{\rm bg})=-0.19011/g^2$ at the controls.
- (vii) $\varepsilon^*=0.9507\,g^2$.

**Verdict.** **The harmonic spectrum and every derived number in H8 are correct.** H8's conclusion "the lowest excitation at $s=1$ has charge 0, with margin $0.3624$ at harmonic order" is **correct for the undressed left-invariant operator on $\mathbb R^9$**, and Steps 1–4 are genuinely proved there.

**But the conclusion does not carry over to the physical small room, for three reasons:**
- **(a)** The charge-0 excitation at offset $\omega_C$ is the vacuum of the second well, and $q_{\rm MC}$ is a gauge copy of $0$.
  - Gauge invariance of the total zero-point energy, with the gauge-covariant regulator that H2 and line 14 require, forces the fast offset to be $-\omega_C$. H2 cites this as proved at $t=1$ (Corollary p4cor:minus2: fast shift $-1$ reduced, and $\omega_C(1)=1$). H8's failure mode 2 guesses it for general $t$.
  - In the dressed theory with $\theta=0$ the two wells are one, and this level is not a glueball.
- **(b)** What remains around $q=0$ is H2's and H7's comparison: charge 0 ($\|q_2\|^2+\|q_3\|^2$) against charge $\pm2$ ($\|q_2\|^2-\|q_3\|^2$, $2q_2\cdot q_3$), all at $2/\sqrt t$ at harmonic order. The order is decided at $O(g^2)$ and needs H2b including the kinetic term $m$. **That is open.**
- **(c)** Consequently:
  - The line-18 IVT is valid as a *reduced-model* statement.
  - H8's Interface item for line 20 ("small-$R$ end takes $\Delta_C\to\omega_C$") is **refuted**. H3's small-$R$ input is $\delta_0$, the lowest two-quantum colour singlet in H2's window, not $\omega_C$.
  - H8 should label H8$_\infty$ "[P-sketch] for the undressed reduced operator; open for the physical room (needs H2b)".
  - None of this touches the Clay chain, because H3 needs only $\delta_0>0$ (H3 Step 3, H8 Flag).

A small extra observation from the check: H8 Step 4(a) shows the reduced ground energy has *no* perturbative correction. In H2's chart reading, the $O(g^2)$ order of $0^{++}$ against $2^{++}$ is therefore entirely the excited-level shift of the two-quantum states at $q=0$. That is a finite second-order computation in the cubic vertex $-gS\det q$ plus first order in the quartic, *before* fast dressing. It is the natural first rung of physical H8, ahead of H8's proposed Poincaré/min–max pair, which targets the artefact level.

---

## 6. Recommended page edits (for my decision; nothing changed here)

1. Line 18: replace the IVT endpoint $s=1.15$ with a computed $s_1>s^*(g)$ at each registered $g$. State that the registered $s^*$ are reduced-model, anharmonic-only predictions.
2. Line 18 / H8: state the operator. "Undressed reduced, two-well" is proved at harmonic order. "Physical, around $q=0$" is open at $O(g^2)$.
3. Line 20 sketch: small-$R$ end = H2 at $s=1$ around $q=0$ (not lines 12–19); large-$R$ end = finite-size mixing MS (not lines 3–5); carrier persistence is not needed for continuity.
4. Line 28: run it along the sphere family (H3 + H5(b)). H4 identifies the limit only. Move "unique vacuum at each $R$" into H3.
5. Line 26: add $T_O\not\equiv0$, or define "free theory" as Wick-polynomial.
6. Line 29 / H5: two bare parameters $(g_0,c_0)$; flowed Wilson loops; break the H4(b)–H5 and H5(b)–H6 cycles as in §2.
7. Line 14: state the sign convention (the asymptotically free sign is negative in units where the classical term is $\mathcal S/g_0^2$).
8. Small fixes: H7 Young weight; H6 plaquette count; H3's citation of H8; H3 fix $t=3$; H7 route form drops H1(G) and H8(G); notation $b_0,b_1$.
