# H7 (line 24): every compact simple gauge group

*Status of this section: hypothesis [X]. Lines 1–3 hold for every compact $G$ [PC]. The statements marked [P-here] below are short proofs I give in this section. Sub-conjecture A_g is now CLOSED BY DEDUCTION from the cited Austing--Wheater bound (27 November 2026); the full H7 hypothesis remains open. Everything else retains its stated standing.*

---

## 1. Statement (high fidelity)

**Groups.** $\mathfrak g$ runs over the compact simple Lie algebras: $\mathfrak{su}(N)$ for $N\ge2$, $\mathfrak{sp}(n)$ for $n\ge2$, $\mathfrak{so}(N)$ for $N\ge7$, and $\mathfrak g_2,\mathfrak f_4,\mathfrak e_6,\mathfrak e_7,\mathfrak e_8$. I write $G$ for the simply connected compact group with Lie algebra $\mathfrak g$. I fix the ad-invariant inner product $\langle\cdot,\cdot\rangle$ on $\mathfrak g$ for which long roots have $(\theta,\theta)=2$. With that normalisation:

- $C_2(\lambda)=\tfrac12(\lambda,\lambda+2\rho)$;
- $C_2(\mathrm{adj})=h^\vee$, the dual Coxeter number;
- for $\mathfrak{su}(N)$, $\operatorname{tr}_{\rm f}(T_aT_b)=\tfrac12\delta_{ab}$ and $C_2(\mathrm f)=(N^2-1)/(2N)$, which is $3/4$ at $N=2$.

**Reading of "every compact simple $G$".** The Jaffe–Witten statement requires local fields (line 25) and a gap in the spectrum of $H$ on the vacuum sector. Neither depends on the global form $G/\Gamma$, $\Gamma\subset Z(G)$: the gauge-invariant polynomials in $F$ are the same for $G$ and for $G/\Gamma$. The global form changes only the line operators and the topological sectors (Aharony, Seiberg and Tachikawa, 2013). So I take $G$ simply connected and define the $G/\Gamma$ theory's local sector to be the $G$ theory's. I take $\theta=0$, which is what the Wilson action of line 1 defines. This is a choice of precise reading. It adds no requirement.

**Dictionary $D_G$.** In every line, $SU(2)$ as structure group is replaced by $G$. The base $S^3_t\cong SU(2)$ is space, so it does **not** change. The other replacements:

- $b_0=22/3$ becomes $b_0(G)=11h^\vee/3$.
- $R\Lambda_G=\exp(-8\pi^2/(b_0(G)g^2(R)))$.
- The note $3/4$ becomes $C_{\min}(G)=\min\{C_2(r): r\ \text{nontrivial irreducible}\}$.
- The reduced space $\mathbb R^9/SO(3)$ becomes $\mathfrak g^3/\mathrm{Ad}(G)$.
- The Maurer–Cartan line uses a Dynkin-index-1 (long-root) embedding $\iota:\mathfrak{su}(2)\to\mathfrak g$.

**H7.** For every compact simple $\mathfrak g$ there is a constant $c_{\mathfrak g}>0$, with no uniformity in $\mathfrak g$ required, such that the conclusions of lines 20, 23, 25–29 hold with $SU(2)$ replaced by $G$ through $D_G$. Concretely:

(i) there is a Euclidean theory on $\mathbb R^4$ satisfying the OS axioms. It has local fields for gauge-invariant polynomials in $F$ (starting with $\langle F,F\rangle$), short-distance behaviour given by perturbation theory with $\beta(g)=-b_0(G)g^3/16\pi^2+O(g^5)$, and a unique vacuum;

(ii) $\operatorname{spec}H\cap(0,\Delta_G)=\emptyset$ with $\Delta_G\ge c_{\mathfrak g}\Lambda_G$.

The order of quantifiers is $\forall\mathfrak g\ \exists c_{\mathfrak g}$, and then the order of limits of H3–H5 for that fixed $\mathfrak g$: $\Lambda_{\rm cut}\to\infty$ at fixed $R$, then $R\to\infty$. The **route form** of H7 is the conjunction $\mathrm{H1}(G),\mathrm{H2}(G),\mathrm{H3}(G),\mathrm{H4}(G),\mathrm{H5}(G),\mathrm{H6}(G),\mathrm{H8}(G)$, each read through $D_G$. As §4 shows, $\mathrm{H1}(G)$ and lines 12 and 18 need more than a dictionary.

---

## 2. Interface

**What the page takes from H7.** Nothing inside the chain. Line 24 is the Clay clause "for any compact simple gauge group $G$", and only §F uses it. Lines 26 and 27 transfer once 25 and 28 do. Line 26 needs $b_0(G)\neq0$, which holds because $h^\vee>0$.

**What H7 takes.** All the other lines, read through $D_G$. Line by line:

- **Verbatim for every compact $G$ [PC/P].** Lines 1, 2, 3 (Osterwalder–Seiler 1978 is for compact $G$), 13 (Peter–Weyl on the base does not depend on $G$; colour is a tensor factor), 22 and the lemma half of 28 (abstract), and 27 (Perron–Frobenius for $-\tfrac12\Delta+V$ on $\mathfrak g^3/G$ does not depend on $G$).
- **Need replacement, done here [P-here].** Lines 4, 6, 7, 8, and the eigenvector half of 10.
- **Closed by deduction from a cited bound.** Line 9's reduced-model extension: the cubic-quartic inequality, section 4 step 3.
- **New per group (finite computations, carrier form).** Lines 11 and 18, i.e. $\mathrm{H1}(G)$ and $\mathrm{H8}(G)$. The sourced controls of line 12 do **not** transfer as a two-carrier story (§4 step 5).
- **Expected to transfer with $b_0\to b_0(G)$.** Lines 14–16 and 20–23, 25, 29. The numbers in line 15 are specific to $SU(2)$.

**A stronger need.** Line 20 assumes that crossings along scale are exact, i.e. symmetry-protected. For $G$ with $\mathrm{Out}(\mathfrak g)\ne1$ ($SU(N\ge3)$, $\mathrm{Spin}(2n)$, $E_6$) the sectors also carry charge conjugation $C$ (or the triality/outer label). $\mathrm{H3}(G)$ must follow the carrier with labels (isometry charge, $P$, $C$), not isometry charge alone. For $SU(2)$, $C$ is inner and every state is $C=+$, so line 20 never needed it.

---

## 3. What is known

**Proved.**

- **Osterwalder and Seiler (1978, Ann. Phys. 110).** For every compact $G$: reflection positivity of the Wilson action, the transfer matrix, and a strong-coupling gap with exponential clustering. This is lines 2–3 for every $G$.
- **Asymptotic freedom (Gross and Wilczek, 1973; Politzer, 1973).** $b_0=11C_2(\mathrm{adj})/3$ for every simple $\mathfrak g$. The two-loop $b_1=34(h^\vee)^2/3$ is standard.
- **Topology.**
  - Bott (1956): $\pi_3(G)=\mathbb Z$ for compact simple simply connected $G$.
  - The minimal instanton sits in a long-root $SU(2)$, the embedding of Dynkin index 1 (Dynkin, 1952; Bernard, Christ, Guth and Weinberg, 1977).
  - $\pi_2(G)=0$, so every $G$-bundle over $S^3$ or $S^3\times\mathbb R$ is trivial, and the small room has no bundle choice.
- **Centres.** $Z(SU(N))=\mathbb Z_N$, $Z(Sp(n))=\mathbb Z_2$, $Z(\mathrm{Spin}(2n+1))=\mathbb Z_2$, $Z(\mathrm{Spin}(4n))=\mathbb Z_2^2$, $Z(\mathrm{Spin}(4n+2))=\mathbb Z_4$, $Z(E_6)=\mathbb Z_3$, $Z(E_7)=\mathbb Z_2$. $G_2$, $F_4$ and $E_8$ have trivial centre.
- **Long-root decomposition (standard; Borel–de Siebenthal).** For the long-root $\mathfrak{su}(2)_\iota$,
  $$\mathfrak g_{\mathbb C}=\mathbf 3\ \oplus\ (2h^\vee-4)\cdot\mathbf 2\ \oplus\ \mathfrak k_{\mathbb C}\otimes\mathbf 1,\qquad \dim\mathfrak k=\dim\mathfrak g-4h^\vee+5,$$
  with $\mathfrak k$ the centraliser. Checks:
  - $\mathfrak{su}(3)$: $\mathfrak k=\mathfrak u(1)$.
  - $\mathfrak{su}(N)$: $\mathfrak k=\mathfrak u(1)\oplus\mathfrak{su}(N-2)$, of dimension $(N-2)^2$.
  - $\mathfrak g_2$: $\mathfrak k=\mathfrak{su}(2)$, and $14=(\mathbf3,\mathbf1)+(\mathbf1,\mathbf3)+(\mathbf2,\mathbf4)$.
  - $\mathfrak e_8$: $\mathfrak k=\mathfrak e_7$, and $248=(\mathbf3,\mathbf1)+(\mathbf1,\mathbf{133})+(\mathbf2,\mathbf{56})$.
  
  The **j = 1 isotypic component is exactly $\iota(\mathfrak{su}(2))$**, because $\mathfrak g_{2}$ in the long-root grading is one-dimensional. $\mathfrak k$ has a centre only for $\mathfrak{su}(N)$, $N\ge3$; for the other algebras it is semisimple ($\mathfrak{sp}(n-1)$, $\mathfrak{su}(2)\oplus\mathfrak{so}(N-4)$, $\mathfrak{su}(6)$, $\mathfrak{so}(12)$, $\mathfrak{sp}(3)$, $\mathfrak{su}(2)$, $\mathfrak e_7$).
- **Ultraviolet.** I cite Bałaban's programme (1980s) and Magnen, Rivasseau and Sénéor (1993, Commun. Math. Phys. 155) for line 21 only as far as I have checked them, which is $SU(2)$-type settings. Their extension to general compact $G$ is expected, since the estimates use compactness and the Lie bracket, but I have not verified it and I do not claim it.

**Expected (lattice evidence, not proof).**

- SU(3) glueballs (Morningstar and Peardon, 1999, Phys. Rev. D60 034509): the lightest is $0^{++}$.
- $G_2$ lattice gauge theory (Holland, Minkowski, Pepe and Wiese, 2003; Pepe and Wiese, 2007; Greensite, Langfeld, Olejník, Reinhardt and Tok, 2007): confinement-like behaviour with Casimir scaling at intermediate distances, followed by string breaking, and a deconfinement transition with no centre symmetry.
- I know of no rigorous continuum result for any exceptional group beyond what holds for all compact $G$ (lines 1–3, 22). I claim none.

---

## 4. Best-guess proof: the $SU(2)$ chain transported, plus what is new per group

**Step 1 (line 4 for every G) [P-here].** On the Hamiltonian lattice, $H_E=\frac{g^2}{2a}\sum_\ell C_2(\ell)$ acts on $L^2(G^{\rm links})^{G^{\rm sites}}$. Its first excited level is
$$E_1-E_0=\frac{g^2}{2a}\cdot4\,C_{\min}(G)=\frac{2C_{\min}(G)\,g^2}{a}.$$

*Proof.*
1. Peter–Weyl diagonalises $H_E$: a state is a spin network, with a representation $r_\ell$ on each link, and it has energy $\frac{g^2}{2a}\sum C_2(r_\ell)$.
2. Gauge invariance at a site requires $\bigotimes_{\ell\ni x}r_\ell\ni\mathbf 1$. So no site has exactly one nontrivial incident link.
3. Hence the excited links form a graph with every degree $\ge2$. Such a graph contains a cycle, and the shortest cycle on the cubic lattice is a plaquette. So there are at least 4 excited links, each costing at least $C_{\min}$.
4. The bound is attained by $\chi_{r_{\min}}(U_p)\Omega$. $\square$

At $SU(2)$ this gives $2\cdot\tfrac34 g^2/a=3g^2/(2a)$, which is line 4.

- $SU(N)$: $C_{\min}=(N^2-1)/2N$.
- $G_2$: $C_{\min}=C_2(\mathbf 7)=2$, from $C_2(r)\dim r=T(r)\dim\mathfrak g$ with $T(\mathbf7)=1$ and $T(\mathrm{adj})=4$.
- $E_8$: the smallest irrep is the adjoint, so $C_{\min}=h^\vee=30$.

Line 5 transfers verbatim. Kato perturbation in the bounded magnetic term is $G$-independent, and the Osterwalder–Seiler cluster expansion gives volume uniformity for compact $G$.

**Step 2 (line 6) [P-here, standard].** In 2-D, $\langle\chi_r\rangle/d_r=(c_r(\beta)/(d_rc_0(\beta)))^{|A|}$, where $c_r$ are the character coefficients of the plaquette weight. At weak coupling the weight is the heat kernel with time $g^2$ per unit area. This gives
$$\sigma_r=\tfrac12 g^2C_2(r)+O(g^4),\qquad \sigma_{\rm adj}/\sigma_{r_{\min}}\to h^\vee/C_{\min}.$$
For $SU(2)$ the ratio is $8/3$, for $SU(3)$ it is $9/4$, and for $G_2$ it is $2$.

In 2-D every nontrivial $r$ has an area law, even when the centre is trivial. In 4-D, **for trivial centre "the note" keeps its spectral meaning but loses its string meaning**. Lines 4–5 still make $2C_{\min}g^2/a$ the leading strong-coupling gap. The Wilson loop in $r_{\min}$ of $G_2$, $F_4$ or $E_8$ is expected to break at long distance: the flux is screened by gluons, because $r_{\min}$ sits in a tensor power of the adjoint when $Z(G)=1$. The note becomes the scale of the lowest glueball, not of an asymptotic string tension. That does not matter for Clay, which asks for a gap and not for confinement.

**Step 3 (small room, lines 7–9).** Homogeneous fields are $A=\sum_k\theta^k\otimes X_k$ with $q=(X_1,X_2,X_3)\in\mathfrak g^3\cong\mathbb R^{3\dim\mathfrak g}$. The residual gauge group is constant $G$ acting by $\mathrm{Ad}$. A generic pair already has finite stabiliser, so $\dim(\mathfrak g^3/G)=2\dim\mathfrak g$, which is $6$ at $SU(2)$.

*(7 replaced) [P-here].* Lebesgue measure pushed to the quotient has, on a slice normal to the orbits, the density
$$\rho(q)\propto\sqrt{\det M(q)},\qquad M(q)_{ab}=\sum_k\langle[T_a,X_k],[T_b,X_k]\rangle .$$
This is the orbit volume. The reduced operator is $-\tfrac12\rho^{-1}\nabla\!\cdot\rho\nabla+V$. Line 7's $\prod_{i<j}|\lambda_i+\lambda_j|$ is the same pushed-forward measure written on the polar slice $\mathrm{Sym}(3)$. That slice exists only because $\mathrm{Ad}(SU(2))=SO(3)$ acts on all of $\mathbb R^3$. For $\dim\mathfrak g>3$, $\mathrm{Ad}(G)\subsetneq SO(\dim\mathfrak g)$, there is no polar slice, and $\sqrt{\det M}$ is the formula to use.

*(8 replaced) [P-here].* Let $q^\iota_{\rm MC}=\frac1g\,\mathrm{diag}(1/\sqrt t,1,1)$ written in the basis $\iota(T_a)$. Then $\iota$ preserves brackets, and with index 1 it preserves norms. So
$$V(\lambda q^\iota_{\rm MC})=(1+2/t)\frac{(\lambda-\lambda^2)^2}{2g^2},\qquad \mathrm{CS}=3\lambda^2-2\lambda^3 ,$$
exactly as for $SU(2)$, and $\lambda=1$ is the winding-1 vacuum, a generator of $\pi_3(G)$. An embedding of index $j$ multiplies $V$, $|q|^2$ and CS by $j$. That is $SU(2)$ at coupling $g^2/j$, with winding $j$.

*(8 extended: the curved path) [D, 27 November].* The inter-well trajectory
transports through any injective su(2) Lie-algebra map. With the source basis
orthonormal and cyclic brackets normalized to one, measure I > 0 from the
pullback inner product. The ambient Agmon distance from 0 to the specified
embedded Maurer--Cartan orbit is I/(2 sqrt(t)) in p coordinates, or
I/(2 sqrt(t) g^2) in q coordinates. The full gradient intertwines without
an I factor; the action includes I. The endpoint bound rules out a cheaper
path through the additional colour directions. This is not a statement
about the lowest quantum carrier or a minimum over different embeddings.
See [the full deduction and verification boundary](../h8-group-transport-proof-2026-11-27.md).
The scalar I is not identified with an integer Dynkin index without the
matching metric convention. H7 remains open.

*(9) Potential for G.* Write $Y_k=\tfrac12\varepsilon_{klm}[X_l,X_m]$, the replacement for $\operatorname{cof}q$, and $\mathcal C(q)=\langle X_1,[X_2,X_3]\rangle$, the replacement for $\det q$. Ad-invariance gives $\langle X_k,Y_k\rangle=\mathcal C$ for each $k$. Then
$$V(q)=\sum_k\tfrac12\|\omega_kX_k-gY_k\|^2-\tfrac12\sum_k\mathrm{src}_k\|X_k\|^2=\tfrac12\sum_k(\omega_k^2-\mathrm{src}_k)\|X_k\|^2-gS\,\mathcal C(q)+\tfrac{g^2}2\sum_k\|Y_k\|^2 .$$
The quadratic part, and so $\kappa=\min_k(\omega_k^2-\mathrm{src}_k)$ and the window $0<s<(t+2)/(t+1)$, do not depend on $G$.

$SU(2)$'s proof uses $|\det q|\le(\|\operatorname{cof}q\|^2/3)^{3/4}$ (AM-GM in singular values). For general $G$, put $Q=\sum_k\|Y_k\|^2$.

> **Former Sub-conjecture A$_\mathfrak g$: CLOSED BY DEDUCTION.** For every fixed compact simple real Lie algebra with a fixed positive Ad-invariant inner product, there is $c_{\mathfrak g}<\infty$ with
> $$|\mathcal C(q)|\le c_{\mathfrak g}Q(q)^{3/4}\quad\forall q\in\mathfrak g^3.$$

This is the old statement at $\alpha=3/4$. Austing--Wheater, *Adding a Myers Term to the IIB Matrix Model*, JHEP 11 (2003) 009, [hep-th/0310170v2](https://arxiv.org/pdf/hep-th/0310170v2), eq. (2.2), gives the global inhomogeneous cubic-quartic bound. After the trace-convention translation it reads $|\mathcal C|\le A Q+B$, with constants independent of q. For $Q>0$, apply it to $rq$ with $r=Q^{-1/4}$: $r^3|\mathcal C|\le A+B$, hence the result. For $Q=0$ all brackets vanish and so does the cubic. The bound covers non-regular strata, without assuming that Q = 1 is compact. The constants can depend on the group and norm.

The existing potential consequently satisfies
$$V\ge\frac\kappa2\|q\|^2-\frac{27S^4c_{\mathfrak g}^4}{32g^2},$$
because $z^4/2-z^3+27/32=(2z-3)^2(4z^2+4z+3)/32\ge0$ for $z\ge0$, with $z=gQ^{1/4}/(Sc_{\mathfrak g})$. This retains the same positive-kappa window. The natural full-space Friedrichs realization has compact resolvent, as do its colour-singlet and charge-sector restrictions. The old general-alpha Young weight $(4\alpha-1)/2$ should have been $(2\alpha-1)/2$; this exact factorization supersedes that estimate.

Full conventions, proof, operator domain and adversarial audit: [the cubic-quartic closure](../h7-cubic-quartic-proof-2026-11-27.md). This is a reduced-model result, not a group-uniform mass gap or a new carrier ordering.

**Step 4 (Helike for G, line 10).** At $q_{\rm bg}=s\,q^\iota_{\rm MC}$:

- (a) **$q_{\rm bg}$ is critical for $G$ [P-here].** $\nabla V(q_{\rm bg})$ is a polynomial in brackets of elements of $\iota(\mathfrak{su}(2))$, so it lies in $\mathbb R^3\otimes\iota(\mathfrak{su}(2))$. There it vanishes by the $SU(2)$ result.
- (b) **The Hessian preserves $\mathbb R^3\otimes V_j$ for each isotypic component $V_j$ of $\mathrm{ad}\circ\iota$ [P-here].** Its terms are $\delta\mapsto[X^{\rm bg}_l,\delta_m]$, the residual pairing $\langle R_k,[\delta_l,\delta'_m]\rangle=\langle[R_k,\delta_l],\delta'_m\rangle$ with $R_k\in\iota(\mathfrak{su}(2))$, and the diagonal quadratic part. Each preserves $V_j$.
- Since the $j=1$ component is exactly $\iota(\mathfrak{su}(2))$, the Hessian on $\mathbb R^3\otimes\iota(\mathfrak{su}(2))\cong\mathbb R^9$ is $SU(2)$'s. **Helike's charge-$\pm2$ doublet is an exact Hessian eigenvector for every $G$, with $\omega_H^2=2s(1+2/t-s)$ unchanged.**
- (c) **New modes, the centraliser [P-here, harmonic level].** For $\delta\in\mathbb R^3\otimes\mathfrak k$: $[X^{\rm bg},\delta]=0$, and $\langle R,[\delta,\delta']\rangle=0$ because $[\mathfrak k,\mathfrak k]\subset\mathfrak k\perp\iota(\mathfrak{su}(2))$. So
  $$\nabla^2V|_{\mathbb R^3\otimes\mathfrak k}=\mathrm{diag}(\omega_1^2-a,\ \omega_2^2-b,\ \omega_2^2-b)\otimes\mathbf 1_{\mathfrak k},\qquad \omega_2^2-b=\frac{s[(t+2)-s(t+1)]}{t}.$$
  At the controls $t=3$, $s=1.15$ this is $1.15\times0.4/3=0.1533$, a frequency of about $0.392$. Helike's $\omega_H^2$ there is $2.3\times0.5167=1.1883$, i.e. $1.0901$, which matches the page.
- The $2h^\vee-4$ doublet modes ($j=\tfrac12$) need their own diagonalisation, which I have not done.

**Step 5 (consequence for lines 11, 12, 18).** By the slice theorem, colour singlets near the orbit are the functions on the normal slice that are invariant under the stabiliser $K$ of $q_{\rm bg}$.

- For $SU(N)$, $N\ge3$, $\mathfrak k\supset\mathfrak u(1)$, and for $SU(3)$ the stabiliser is the connected $U(1)$. **One centraliser quantum is a singlet.** It has Berger charge $\pm1$, from frame indices 2 and 3; colour rotation by $\iota$ acts trivially on $\mathfrak k$. Its harmonic frequency is about $0.392$, below Helike's $1.090$.
- For semisimple $\mathfrak k$ the lowest singlet uses two quanta, at harmonic frequency about $0.783$. That is still below Helike, in charges $0$ and $\pm2$.

So, **at the harmonic level, in the sourced room at the controls, for every $G\ne SU(2)$ the lowest note is not Helike and not the $SU(2)$ competitor.** Lines 12 and 18 are $SU(2)$-specific evidence, and $\mathrm{H1}(G)$ must be re-posed with the centraliser threshold included. This does not touch the route to H3, which starts at $s=1$ (line 20). It does mean that the two-carrier picture is not universal.

**Step 6 (the unsourced room, H8(G)).** At $s=1$, around $q=0$, every colour component has frequency $\omega_k$. The degree-2 invariants are $\langle X_k,X_l\rangle$, a symmetric $3\times3$ matrix whose structure does not depend on $G$. The lowest singlets are two quanta in $k=2,3$ at $2/\sqrt t$: charge $0$ from $\|X_2\|^2+\|X_3\|^2$, and charge $\pm2$ from $\|X_2\|^2-\|X_3\|^2\pm2i\langle X_2,X_3\rangle$. They are degenerate at harmonic order.

The splitting at order $g^2$ comes from the cubic and quartic terms. **Reduced model: CLOSED BY DEDUCTION (27 Nov)**, for every compact simple $\mathfrak g$ and $t>1$. In the ground-state picture the quartic enters the factorization and only $L_1=-\nabla\mathcal C\cdot\nabla$ remains. It annihilates the charge-$\pm2$ pair exactly, so their series is $2/\sqrt t$ with every correction zero. The charge-0 level moves by $e_2g^2$ with $e_2=-2\kappa(1+1/t)$, $\kappa$ the adjoint Casimir ($-4(1+1/t)$ for the register's $SU(2)$). Only $\kappa$ enters, not $\dim\mathfrak g$. Quasimodes put spectrum within $O(g^3)$ and $O(e^{-c/g^2})$ of these values; counting and well assignment remain the cited localization. See [the origin-splitting proof](../h8-origin-splitting-proof-2026-11-27.md). **$\mathrm{H8}(G)$ for the physical room: the charge-0 state lies lower.** [guess; needs H2b not to reverse the reduced splitting] The evidence is the $SU(2)$ value (line 18) and lattice $0^{++}$ lightest for $SU(3)$. For $SU(N\ge3)$ the extra cubic $d$-invariant also gives $C=-$ states, which line 20 must label.

**Step 7 (fast modes, H2(G), lines 13–16).** The Peter–Weyl sectors $j\ge1$ with multiplicity $j+1$ do not depend on $G$; colour is $\otimes\,\mathfrak g$. The one-loop heat-trace coefficient of the fluctuation operator is $\propto C_2(\mathrm{adj})\,S_{\rm cl}$. So line 14 holds with
$$E_0(q)=\frac{b_0(G)}{8\pi^2}\log(\Lambda_{\rm cut}R)\,S_{\rm cl}(q)+E_0^{\rm fin}(q),\qquad b_0(G)=\frac{11h^\vee}{3}$$
[expected, standard one-loop; the $10^{-4}$ match is for $SU(2)$ only]. H2(G) states $\|P_{\rm low}(H_{\Lambda_{\rm cut}}-H_{\rm eff})P_{\rm low}\|\le C_{\mathfrak g}g^2(R)$, with $C_{\mathfrak g}$ allowed to grow with $\dim\mathfrak g$.

**Step 8 (H3(G), the wall) [guess].** The lattice end is Step 1 plus lines 3 and 5. The small-$R$ end is Steps 3, 6 and 7. Continuity in $R$ follows the carrier with labels (isometry charge, $P$, $C$). The key estimate is the same as for $SU(2)$:
$$\inf_{R>0}\Delta_G(R)/\Lambda_G\ge c_{\mathfrak g}>0,\qquad R\Lambda_G=e^{-8\pi^2/(b_0(G)g^2(R))}.$$
Nothing in my $SU(2)$ sketch uses $\dim\mathfrak g=3$ except line 7's polar slice (replaced in Step 3) and the absence of $C$ (handled by the extra label). That is the induction behind "transported". The evidence is lattice spectroscopy for $SU(2)$, $SU(3)$ and $G_2$.

**Step 9 (H4–H6, lines 25–29).** These are expected to transfer with $G$ entering only through $b_0$, $b_1$, compactness and the bracket. The lemma half of line 28 and line 22 transfer verbatim. Line 27 transfers by Perron–Frobenius, since the reduced heat kernel on $\mathfrak g^3/G$ is positivity improving.

---

## 5. Where it can fail

1. **Former obstruction, now excluded for the stated model: A$_\mathfrak g$ at non-regular strata.** The global bound and scaling proof in step 3 exclude an escape along near-commuting triples on the positive-kappa window. This does not control a continuum limit or a change of model outside that window.
2. **Centraliser and doublet modes reorder the carriers.** Step 4(c) already shows this in the sourced room. If the $j=\tfrac12$ doublet modes also go soft at $s=1$, the $s=1$ carrier could be a non-scalar or a $C=-$ state, and $\mathrm{H8}(G)$ fails as worded. It would show as the lowest $s=1$ singlet having charge $\ne0$ or $C=-1$.
3. **Non-exact crossings in the larger symmetry structure.** With more invariants (the $d$-symbol, the Pfaffian-type invariants of $\mathfrak{so}(2n)$, triality), two carriers could share every label and avoid each other. Persistence along $R$ (line 20) would then need a real gap estimate rather than symmetry. This would show as an avoided crossing in the $R$-flow.
4. **Trivial centre removes a structure the confinement intuition uses.** Not the gap itself: Clay asks only for a gap, and the strong-coupling gap exists for $G_2$, $F_4$ and $E_8$ (line 3). But if any step of H3 or H4 secretly uses centre symmetry, such as centre-vortex area laws or Polyakov-loop order parameters, it fails for $G_2$, $F_4$ and $E_8$. That would show as a step that needs an area law for $r_{\min}$.
5. **Ultraviolet constants that grow with group size.** H7 asks for no uniformity in $\mathfrak g$. But a UV estimate whose constants blow up with $\dim\mathfrak g$ before $g^2\to0$ could leave a finite-coupling window for large groups (e.g. $E_8$, $\dim=248$) uncovered. This would show as the convergence radius in $g^2$ of the cluster/RG expansion shrinking with $h^\vee$.

---

## 6. The smallest step is closed

The proposed SU(3) inequality is now proved, and the same deduction works for each fixed compact simple algebra. Line 9's compact-resolvent conclusion transfers on the same window with the same kappa and a group-dependent additive constant. This does not close the full H7 statement, identify the lowest carrier among the extra colour modes, or settle the ultraviolet and large-room limits. No new simulation is required for this step; the exact supporting checks and their limits are in the linked proof note.

---

## Flag

None. H7 is the Jaffe–Witten clause *"for any compact simple gauge group $G$"* itself. My precise reading (simply connected $G$, local sector shared with every quotient, $\theta=0$) chooses among readings and adds no requirement.
