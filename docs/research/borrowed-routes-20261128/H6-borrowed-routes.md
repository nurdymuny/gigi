# H6 borrowed routes: local fields, operator mixing and the trace anomaly

28 November 2026 (written overnight). This is a research note. It changes no status in `CLOSED-SET.md`. H6 stays [X].

Each citation below was checked on INSPIRE or arXiv tonight. Where I could not see a detail (a journal volume, the exact
wording of a theorem), I leave it out or mark it "paraphrase from the abstract".

---

## 1. The precise target

H6 is line 25, made precise in `closed-set-20261127/H6-local-fields.md` §1. It has three clauses.

- **(a) Existence.** Renormalization matrices $Z^{(d)}(a)$ exist, mixing only with equal or lower dimension. The
  renormalized lattice transcriptions $\sum_j Z_{ij}(a)a^{-d_j}P_{j,a}$ have Schwinger functions on $\mathcal S_{\neq}$
  in the H4 order of limits. Those functions extend to tempered distributions and satisfy OS.
- **(b) Stress tensor.** $T_{\mu\nu}$ is conserved (b1) and generates translations (b2). Its trace is (b3)
  $\Theta=\frac{\beta(g)}{2g}[O]_\mu$, RG invariant. On the lattice it is the limit of
  $\Theta_a(x)=-\frac{d\beta_0}{d\ln a}a^{-4}\sum_{p\ni x}(s_p-\langle s_p\rangle)$ **with no further multiplicative
  renormalization**.
- **(c) Agreement to all orders, asymptotically.** At short distance, $\lambda^{\sum d_i}\mathcal Z^{-1}S_n(\lambda x)$
  has an asymptotic expansion in $\bar g(\lambda)^2$. Its coefficients come from renormalized perturbation theory.

I asked two questions:
1. Which parts of H6 are already theorems of renormalized perturbation theory (all orders, as formal power series)?
2. Does "agreement to all orders" reduce to those theorems plus H4′?

The short answers:
1. All of (b), and the existence and computability of the coefficients in (c), are perturbative theorems.
2. **No.** H4′ plus the perturbative theorems is not enough for (c). One more non-perturbative estimate is needed. It
   lines up naturally with my sphere family: by exact scaling, short distance is the small room. Separately, one
   **non-perturbative** piece of (b3) does follow from H4 alone, with no estimate: the *integrated* trace identity with
   exactly the lattice normalization. §5 proves it.

---

## 2. Established approaches (verified citations, borrowed statements, hypotheses)

### 2.1 Gauge-invariant operator mixing

**[KSZ75a]** H. Kluberg-Stern and J.-B. Zuber, "Ward identities and some clues to the renormalization of gauge invariant
operators", Phys. Rev. D 12 (1975) 467–481.
**[KSZ75b]** H. Kluberg-Stern and J.-B. Zuber, "Renormalization of non-Abelian gauge theories in a background field gauge.
2. Gauge invariant operators", Phys. Rev. D 12 (1975) 3159–3180.
**[JL76]** S. D. Joglekar and B. W. Lee, "General theory of renormalization of gauge invariant operators",
Ann. Phys. 97 (1976) 160.

*Borrowed statement (the standard content of these papers, in my paraphrase).* In perturbation theory with a BRST-invariant
gauge fixing, a gauge-invariant local operator of dimension $d$ mixes under renormalization with three kinds of operator:
- gauge-invariant operators of dimension $\le d$;
- BRST-exact operators;
- operators that vanish by the equations of motion.

The last two kinds have vanishing matrix elements between physical states, and vanish in correlators of gauge-invariant
operators at non-coincident points. So, restricted to gauge-invariant correlators at separated points, the mixing matrix
is triangular in dimension, as H6(a) demands.

*Hypotheses.* Formal perturbation theory to all orders, in a renormalizable BRST-invariant gauge (Lorentz-type or
background-field gauge), with a regularization that respects BRST (dimensional regularization in practice). Flat
$\mathbb R^4$. Nothing is said about a lattice regulator or about the non-perturbative theory.

### 2.2 The trace anomaly

**[CDJ77]** J. C. Collins, A. Duncan and S. D. Joglekar, "Trace and dilatation anomalies in gauge theories",
Phys. Rev. D 16 (1977) 438–449.
**[ACD77]** S. L. Adler, J. C. Collins and A. Duncan, "Energy-momentum-tensor trace anomaly in spin-1/2 quantum
electrodynamics", Phys. Rev. D 15 (1977) 1712. *Note: this paper is QED. The non-abelian statement is [CDJ77].*
**[N77]** N. K. Nielsen, "The energy momentum tensor in a nonabelian quark gluon theory", Nucl. Phys. B 120 (1977)
212–220. (I confirmed the citation from secondary bibliographies; I did not see the text.)

*Borrowed statement ([CDJ77], paraphrase from the abstract).* For non-abelian gauge bosons with or without fermions, the
trace of the renormalized energy-momentum tensor is, to all orders,
$$\theta^\mu{}_\mu=\frac{\beta(g)}{2g}[F^2]\;+\;(\text{fermion mass terms})\;+\;(\text{BRST-exact and equation-of-motion operators}).$$
The abstract stresses that gauge-variant operators which mix with the naive trace enter this identity. They drop out of
gauge-invariant correlators at separated points (2.1). For pure YM the identity is H6(b3).

*Hypotheses.* Perturbation theory to all orders, BRST-covariant regularization, flat space.

### 2.3 The lattice energy-momentum tensor and the lattice trace

**[CCMP90]** S. Caracciolo, G. Curci, P. Menotti and A. Pelissetto, "The energy-momentum tensor for lattice gauge theories",
Ann. Phys. 197 (1990) 119.

*Borrowed statement (paraphrase from the abstract).* The lattice $T_{\mu\nu}$ is fixed by imposing the translation Ward
identities, perturbatively and non-perturbatively. The correct scale anomaly is reproduced on the lattice. The hypercubic
group splits the traceless symmetric part into two irreps, $\mathbf 3$ and $\mathbf 6$, which need separate finite
normalizations. The trace (singlet) is the action density.

*Hypotheses.* Wilson-type lattice action. The non-perturbative part *assumes* a continuum limit with restored
translation symmetry. That is exactly H4.

**[BEKLLLP96]** G. Boyd, J. Engels, F. Karsch, E. Laermann, C. Legeland, M. Lütgemeier and B. Petersson, "Thermodynamics
of SU(3) lattice gauge theory", Nucl. Phys. B 469 (1996) 419–444, arXiv:hep-lat/9602007.

*Borrowed statement (eq. (2.10)).* $(\varepsilon-3p)/T^4=N_\tau^4\,T\frac{d\beta}{dT}\,[S_0-S_T]$, where
$T\,d\beta/dT=-a\,d\beta/da$ (their eq. (2.11)). This is the integrated form of my Step 4 identity applied to the free
energy: the trace density *is* the plaquette times the lattice beta function, with only the vacuum subtraction
$S_0$. This is the working lattice community's version of "no further multiplicative renormalization".

*Hypotheses.* Isotropic lattice. The derivation is exact at finite lattice spacing. The continuum reading assumes a line
of constant physics $a(\beta)$.

**[K82]** F. Karsch, "SU(N) gauge theory couplings on asymmetric lattices", Nucl. Phys. B 205 (1982) 285–300.
I need this for the anisotropic case (§3, the sphere with two bare parameters).

### 2.4 Lattice perturbation theory converges to continuum perturbation theory

**[R88]** T. Reisz, "A convergence theorem for lattice Feynman integrals with massless propagators", Commun. Math. Phys.
116 (1988) 573; "Renormalization of Feynman integrals on the lattice", Commun. Math. Phys. 117 (1988) 79;
"Renormalization of lattice Feynman integrals with massless propagators", Commun. Math. Phys. 117 (1988) 639.
**[R89]** T. Reisz, "Lattice gauge theory: renormalization to all orders in the loop expansion", Nucl. Phys. B 318 (1989)
417–463.

*Borrowed statement ([R89], paraphrase from the INSPIRE summary).* With a BRS-symmetric gauge fixing, lattice gauge theory
has a continuum limit that is finite to all orders in the loop expansion. Only coupling and field-strength renormalization
are needed, and the limit is universal. *The register cites Reisz 1988 (the power-counting theorem). The gauge-theory
theorem H6 actually uses is [R89].*

*Hypotheses.* Order-by-order perturbation theory, on infinite lattice volume (or with IR regularization), with gauge
fixing.

### 2.5 Rigorous perturbative YM, composite-operator bounds, and the OPE

**[EGK17]** A. N. Efremov, R. Guida and C. Kopper, "Renormalization of SU(2) Yang-Mills theory with flow equations",
J. Math. Phys. 58 (2017), DOI 10.1063/1.5000041, arXiv:1704.06799. Rigorous perturbative renormalizability of 4D SU(2)
YM from Wilson–Polchinski flow equations, with physical renormalization conditions and bounds on massless correlators.

**[FHH16]** M. B. Fröb, J. Holland and S. Hollands, "All-order bounds for correlation functions of gauge-invariant
operators in Yang-Mills theory", J. Math. Phys. 57 (2016) 122301, arXiv:1511.09425.
*Borrowed statement (paraphrase from the abstract).* A complete, rigorous proof that Euclidean YM is perturbatively
renormalizable, with correlation functions of composite local operators satisfying the appropriate Ward identities.
There are analytic bounds on these correlation functions in terms of weighted spanning trees. The method is
Wilson–Wegner–Polchinski–Wetterich flow equations, with full control of BRST invariance.

**[FH16]** M. B. Fröb and J. Holland, "All-order existence of and recursion relations for the operator product expansion
in Yang-Mills theory", arXiv:1603.08012. (The arXiv page lists no journal; I cite the preprint.)
*Borrowed statement (paraphrase from the abstract).* In Euclidean YM, the OPE exists as a short-distance expansion to all
orders in perturbation theory. Ward identities carry over to the OPE, so the OPE of gauge-invariant composite operators
contains only gauge-invariant composite operators. The OPE coefficients obey recursion relations, and these are finite
without further renormalization.

**[HK12]** S. Hollands and C. Kopper, "The operator product expansion converges in perturbative field theory", Commun.
Math. Phys. 313 (2012) 257–290, arXiv:1105.3375. **[HHK15]** J. Holland, S. Hollands and C. Kopper, "The operator product
expansion converges in massless $\varphi^4_4$-theory", DOI 10.1007/s00220-015-2486-6, arXiv:1411.1785.
*Borrowed statement.* At each order of perturbation theory, the OPE converges at finite distances. This is proved for
massive [HK12] and massless [HHK15] Euclidean $\varphi^4_4$, **not YM**.

**[H08]** S. Hollands, "Renormalized quantum Yang-Mills fields in curved spacetime", Rev. Math. Phys. 20 (2008),
DOI 10.1142/S0129055X08003420, arXiv:0705.3340.
*Borrowed statement (paraphrase from the abstract).* Perturbative quantum YM can be defined consistently as a renormalized
perturbative QFT on any globally hyperbolic Lorentzian spacetime. The renormalization group flow does not leave the
space of gauge-invariant operators.

*Common hypotheses for 2.5.* All of these are formal power series in $g$ (order by order). [FHH16], [FH16] and [EGK17]
work in Euclidean $\mathbb R^4$ with IR regularization or bounds. [H08] is Lorentzian and works on curved backgrounds,
locally covariantly. None of them is a non-perturbative statement.

### 2.6 Gradient flow, flowed fields, and the small-flow-time stress tensor

**[L10]** M. Lüscher, "Properties and uses of the Wilson flow in lattice QCD", JHEP 08 (2010) 071, arXiv:1006.4518.
**[LW11]** M. Lüscher and P. Weisz, "Perturbative analysis of the gradient flow in non-abelian gauge theories",
JHEP 02 (2011) 051, arXiv:1101.0963.
*Borrowed statement ([LW11], paraphrase from the abstract).* Correlation functions of the flowed gauge field at positive
flow time need no renormalization beyond the usual four-dimensional one. This holds to all loop orders.
**[HMS17]** K. Hieda, H. Makino and H. Suzuki, "Proof of the renormalizability of the gradient flow", Nucl. Phys. B (2017),
DOI 10.1016/j.nuclphysb.2017.02.017, arXiv:1604.06200. An alternative all-orders perturbative proof.
**[L13]** M. Lüscher, "Chiral symmetry and the Yang-Mills gradient flow", JHEP 04 (2013) 123, arXiv:1302.5246. Uses the
(4+1)-dimensional local field theory representation for renormalizability at positive flow time.

**[S13]** H. Suzuki, "Energy-momentum tensor from the Yang-Mills gradient flow", PTEP 2013, 083B03, arXiv:1304.0533.
*Borrowed statement (eq. (4.29) of the arXiv version, as an asymptotic relation):*
$$\{T_{\mu\nu}\}_R(x)\;\stackrel{t\to0^+}{\sim}\;\alpha_U(t)\,U_{\mu\nu}(t,x)+\alpha_E(t)\,\delta_{\mu\nu}\big[E(t,x)-\langle E\rangle\big].$$
Here $U_{\mu\nu}$ is the traceless flowed $G_{\mu\rho}G_{\nu\rho}$ and $E$ is the flowed action density. At one loop,
$\alpha_E(t)\simeq\frac{b_0}{2}[1-\dots]$, with $b_0=\frac{11N}{48\pi^2}$ in his normalization. The trace part is fixed by
the trace anomaly $\delta_{\mu\nu}\{T_{\mu\nu}\}_R=-\frac{\beta}{2g^3}\{F^2\}_R$ in his field normalization. That is why
the trace coefficient is the beta function.
*Hypotheses, as stated.*
- Dimensional regularization, with $T_{\mu\nu}$ normalized by the translation Ward identity.
- Perturbation theory and asymptotic freedom for small $t$.
- The small-flow-time expansion (flowed products expand in renormalized local operators), with $O(t)$ terms dropped.
- The order of limits $a\to0$ before $t\to0$.
- Finite-size effects ignored.
The non-perturbative validity of the small-$t$ asymptotics is **assumed**, not proved.

**[DPR13]** L. Del Debbio, A. Patella and A. Rago, "Space-time symmetries and the Yang-Mills gradient flow", JHEP 11
(2013) 212, arXiv:1306.1173. Translation Ward identities along the flow. On the lattice they give a non-perturbative
renormalization of $T_{\mu\nu}$. Dilatations are also discussed.

**[HKL18]** R. V. Harlander, Y. Kluth and F. Lange, "The two-loop energy-momentum tensor within the gradient-flow
formalism", Eur. Phys. J. C 78 (2018) 944, arXiv:1808.09837. Suzuki's matching coefficients at NNLO.

---

## 3. Alignment with my framing

**Strong alignment.**
- *The flow is my variational smoothing.* The Wilson flow is the energy gradient of the Wilson action, the same
  "force must be an energy gradient" rule my programme insists on. H4 already uses flowed bounded observables and fixes
  the scale non-perturbatively with $t_0^2\langle E_{t_0}\rangle=\text{const}$ ([L10]). [LW11], [HMS17] and [S13]
  turn this into the field-theoretic route of my H6 §4, Steps 1–3. I am not inventing that route: it is the standard
  one. What is mine is the non-perturbative claim.
- *My Step 4 identity is standard lattice thermodynamics.* [BEKLLLP96] eq. (2.10) is the same exact identity
  ($\partial_\beta$ of the measure, times $a\,d\beta/da$), used to define the interaction measure. That makes
  H6(b3)'s lattice formula, "plaquette times the running coupling with only the vacuum subtracted", believable to any
  lattice reader.
- *The room size R is the RG direction.* §5 shows that, at fixed lattice data, changing $\Lambda$ is changing $\beta_0$,
  so the trace anomaly is literally the derivative along the bundle's scale coordinate. This is my "R as a base
  coordinate" picture, written as an exact identity.
- *Curved backgrounds.* [H08] builds perturbative YM on curved globally hyperbolic spacetimes, with local covariance, and
  shows the RG preserves gauge-invariant operators. $\mathbb R\times S^3_R$ is static and globally hyperbolic. So the
  perturbative coefficients of H6(c) on my rooms are covered *in Lorentzian signature*. The OPE coefficients are local,
  and curvature enters only through higher-dimension terms suppressed by $(|x|/R)^2$. This is the right perturbative
  partner for H4′'s "every room".

**Partial alignment, with an extra step needed.**
- *The sphere has two bare parameters.* H4′ tunes $g_0$ and a speed of light $c_0$ on $S^3\times\mathbb R$. Then
  $\Lambda\partial_\Lambda$ at fixed lattice data hits both, and the lattice trace density gains a second term: the
  anisotropy (electric-minus-magnetic plaquette) density, weighted by $dc_0/d\ln a$. Karsch's anisotropy coefficients
  [K82] are the template. So H6(b3)'s formula, written for the hypercubic torus, holds on the sphere family only after
  adding that term. In the limit the extra term must drop out, by restored relativistic invariance (part of H4′). That
  is a check, not a gift.
- *[FH16] and [FHH16] are Euclidean $\mathbb R^4$ with BRST gauge fixing.* My lattice measure has no gauge fixing
  (line 1). Gauge-invariant correlators at separated points agree between the two schemes order by order ([R89] plus
  gauge-parameter independence). That is a theorem in perturbation theory, not something non-perturbative.

**Does not align (honest negatives).**
- *Convergence of the OPE ([HK12], [HHK15]) is scalar $\varphi^4$ only.* It is not available for YM even in
  perturbation theory. In any case my H6(c) asks for asymptotic expansions, not convergence, so I do not need it. I
  record it only as the model a future YM OPE convergence theorem would follow.
- *Every theorem in 2.1, 2.2, 2.4 and 2.5 is perturbative.* They give the coefficients $F^{(k)}$ in H6(c) and the
  form of (b), as formal power series. None of them says the non-perturbative correlator *has* that asymptotic
  expansion. That gap is the whole content of (c).
- *[S13]'s small-$t$ formula assumes the small-flow-time asymptotics.* Using it non-perturbatively is my Step 2 guess,
  re-labelled. It does not close it.
- *[CCMP90]'s non-perturbative statement assumes the continuum limit.* It is a consequence of H4-type input, not a
  replacement for it.

---

## 4. A reduced route for H6

The pieces sort into four layers: what is perturbative and done, what is exact on the lattice, what is conditional on H4
alone, and what needs a new estimate.

**Layer P (perturbative theorems, all orders, cited).**
- P1. Gauge-invariant operators mix triangularly in dimension at separated points ([KSZ75a], [KSZ75b], [JL76]).
- P2. $\theta^\mu{}_\mu=\frac{\beta}{2g}[F^2]$ modulo BRST-exact and equation-of-motion operators ([CDJ77]; [N77]).
- P3. Lattice perturbation theory goes to continuum perturbation theory order by order ([R88], [R89]).
- P4. The OPE exists to all orders, closes on gauge-invariant operators, and has rigorous bounds ([FH16], [FHH16];
  [EGK17] for SU(2) renormalizability).
- P5. Flowed fields are finite ([LW11], [HMS17]). $T_{\mu\nu}$ is a small-flow-time combination of flowed fields, with
  matching coefficients at one and two loops ([S13], [HKL18]).
- P6. These carry to curved static backgrounds ([H08]).

So H6(b) and the *coefficients* of H6(c) are theorems in perturbation theory. In particular, the coefficients of my line
26 diagnostics ($C_O$, $T_O=0$, $T_O^{(4)}$) are leading coefficients of P4-type expansions.

**Layer E (exact lattice identities).**
- E1. $\partial_{\beta_0}\langle X\rangle=-\langle X;\sum_p s_p\rangle$ on any finite lattice. This is line 1 itself.
- E2. The lattice scaling tautology. The lattice expectation depends only on lattice-unit data and $\beta_0$, so
  changing $\Lambda$ at fixed $a$ is the same as rescaling all physical lengths (§5).

**Layer H4 (conditional on H4′ only).**
- The integrated trace identity, distributionally in the scale variable, with the lattice normalization
  $-d\beta_0/d\ln a$ and no other renormalization. This is §5, proved below.

**Layer N (new non-perturbative input still needed).**
- N1 (H6(a) for $\operatorname{tr}F^2$): tightness of the local $\Theta_a$ correlators at separated points, my H6 §6
  smallest step. It upgrades the integrated identity to a local field identity.
- N2 (H6(c), via the bundle): **weak-coupling asymptotics in the small room, for correlators**. By exact scaling (E2),
  the short-distance limit $\lambda\to0$ at fixed shape in a room of radius $R=\lambda R_0$ *is* the small-room limit
  at fixed $x/R$. H6(c) at fixed ratio $|x|/R$ then follows from: "the small-room correlators of flowed observables have
  the weak-coupling asymptotic expansion in $g(R)$, uniformly in the cutoff". This is the correlator analogue of H2
  (H2 is stated for levels). Its coefficients are P6.
- N3 (IR decoupling): $S_{R}(x)\to S_\infty(x)$ as $R/|x|\to\infty$, **uniformly in $\lambda$**. This lets me remove
  the room after taking the short-distance asymptotics. In perturbation theory it is P6 locality: curvature
  corrections are $O((|x|/R)^2)$ times higher-dimension operators. Non-perturbatively it is the same uniformity
  question as H4′'s $R\to\infty$ end, restricted to small separations.
- N4 (removing the flow): the non-perturbative small-flow-time asymptotics, i.e. my H6 Step 2 together with [S13]'s
  assumption.

**Verdict on "agreement to all orders reduces to known perturbative theorems plus H4′".** False as stated. It reduces to
P1–P6, plus H4′, plus **N2 and N3** (plus N4 for unflowed local fields). The useful thing the borrowing buys is that N2
lives inside my small room, where my machinery already is (H2, the reduced model, the M3 dressing). That is a genuinely
geometric route to (c): short distance in $\mathbb R^4$ = small room + IR decoupling. The perturbative literature already
supplies every coefficient on the way.

---

## 5. The single most closable sub-statement: the integrated trace identity from H4 alone

### Statement

**Proposition (integrated trace anomaly, conditional on H4(a)).** Work in H4's setting: $G=SU(2)$, Wilson action, lattices
$\Lambda_{n,L}$, bounded flowed observables $F\in\mathcal O$ (H4 file §1), and $|F|\le\|\Phi\|_\infty$. Assume:
- (L1) *Continuous line of constant physics.* For $s$ in an open interval $J\ni1$, let $\beta_n(s)$ be the bare
  coupling fixed by H4's scale condition with $\Lambda$ replaced by $s\Lambda$, i.e. with $t_0$ replaced by $t_0/s^2$.
  I assume $s\mapsto\beta_n(s)$ is $C^1$ on $J$ for each $n$.
- (L2) *H4(a) for each scheme $s\in J$.* For all $F\in\mathcal O$ the limit
  $S_s(F):=\lim_{n\to\infty}\langle F\rangle_{\mu_{n,L},\beta_n(s)}$ exists.

Define the lattice anomaly density with H6(b3)'s normalization:
$$\Theta_n^{(s)}(y):=-\frac{d\beta_0}{d\ln a}\Big|_{n,s}\,a_n^{-4}\sum_{p\ni y}\big(s_p-\langle s_p\rangle\big),\qquad
-\frac{d\beta_0}{d\ln a}\Big|_{n,s}:=s\,\frac{d\beta_n}{ds}(s).$$
Here $\sum_{p\ni y}$ runs over the six plaquettes based at $y$, so each plaquette is counted once. Along an exact line of
constant physics $\beta_0=\beta_0(a\Lambda)$, the derivative in $\ln\Lambda$ at fixed $a$ equals the derivative in $\ln a$
at fixed $\Lambda$, so this is the register's $-d\beta_0/d\ln a$. Then, for every $\varphi\in C_c^\infty(\ln J)$,
$$\lim_{n\to\infty}\int du\;\varphi(u)\sum_{y}a_n^4\,\big\langle F;\Theta_n^{(e^u)}(y)\big\rangle_{\beta_n(e^u)}
\;=\;-\int du\;\varphi'(u)\,S_{e^u}(F).\tag{$\ast$}$$
That is, the integrated anomaly insertion converges, as a distribution in the scale variable $u=\ln s$, to $\frac{d}{du}S$.

**Corollary (dilatation form).** Write $F=F_{\gamma,t}$ for loops $\gamma$ at flow time $t$ in the box $L$. Assume the H4
limit does not depend on the base spacing $a_0$ of the dyadic sequence. ("*The* limit" in line 23 already means this.)
Then $S_s(F_{\gamma,t};L)=S_1(F_{s\gamma,s^2t};sL)$, and the right side of $(\ast)$ is the distributional derivative of
$u\mapsto S_1(F_{e^u\gamma,\,e^{2u}t};e^uL)$. That is the geometric dilatation $(x\cdot\partial_x+2t\partial_t+L\partial_L)$
acting on the limit correlator. So
$$\int d^4y\,\langle\Theta(y);F\rangle\;=\;\mathcal D\,S(F)\qquad\text{(distributionally in scale)},$$
with the lattice normalization $-d\beta_0/d\ln a$ and **no further multiplicative renormalization**. This is the
integrated form of H6(b3). It fixes the constant in my H6 §4 Step 4 ("the integrated identity fixes it") without any
local tightness.

### Deduction

1. *Exact identity at fixed $n$ (E1).* First take time extent $T<\infty$, so everything is finite-dimensional. The measure
   is $Z^{-1}e^{-\beta\sum_p s_p}\prod dU$ on a compact group product, and $F$ is a bounded smooth function of the links
   (the lattice flow is a smooth map). Differentiating under the integral gives
   $\partial_\beta\langle F\rangle=-\langle F;\sum_p s_p\rangle$, analytic in $\beta$.
   - For H4's infinite-time lattice: at fixed $(n,L)$ the transfer matrix is positive and compact on
     $L^2(SU(2)^{\text{spatial links}})$, with a simple top eigenvalue (Perron–Frobenius, as in line 27), and its gap is
     continuous and positive on compact $\beta$-sets.
   - So connected correlations decay exponentially in time, uniformly on compact $\beta$-sets. The $T\to\infty$ limits
     of $\langle F\rangle$ and of $\langle F;\sum_p s_p\rangle$ exist, the second uniformly in $\beta$, so the derivative
     commutes with $T\to\infty$.
   - This bullet is a standard argument. I have sketched it here rather than written it out.
2. *Chain rule.* With $u=\ln s$:
   $\frac{d}{du}\langle F\rangle_{\beta_n(e^u)}=\frac{d\beta_n}{du}\,\partial_\beta\langle F\rangle
   =\sum_y a_n^4\langle F;\Theta_n^{(e^u)}(y)\rangle$, by step 1 and the definition of $\Theta_n$ (the subtraction of
   $\langle s_p\rangle$ is what makes it a connected correlator). Under (L1) this is continuous in $u$.
3. *Integration by parts at fixed $n$.* $\int\varphi(u)\frac{d}{du}\langle F\rangle_{\beta_n(e^u)}du
   =-\int\varphi'(u)\langle F\rangle_{\beta_n(e^u)}du$.
4. *Limit.* $|\langle F\rangle|\le\|\Phi\|_\infty$ for every $n$ and $u$, and by (L2) the integrand converges pointwise in
   $u$. Dominated convergence on $\operatorname{supp}\varphi'$ gives $(\ast)$.
5. *Corollary (E2).* The lattice expectation depends only on the integer data $\gamma/a_n$, $t/a_n^2$, $L/a_n$ and on
   $\beta$. The coupling $\beta_n(s)$ solves the scale condition at lattice flow time $t_0/(s^2a_n^2)$, which is the
   $s=1$ condition at spacing $a'=s\,a_n$. So
   $\langle F_{\gamma,t}\rangle_{a_n,\beta_n(s),L}=\langle F_{s\gamma,s^2t}\rangle_{a',\beta_1,sL}$.
   - The right side runs along the dyadic sequence based at $s a_0$. By H4(a) at that base, and independence of the
     base, the limits agree.
   - The two sides use the same lattice and the same lattice-unit data, so this is an identity, not an approximation.

### What it does and does not give

- **It gives** the first fully non-perturbative piece of H6(b3), conditional only on H4(a) (pointwise convergence of bounded
  flowed observables along continuously varying schemes) plus the regularity (L1). It needs no estimate at all: every
  step is exact or dominated convergence. It pins the anomaly's normalization to $-d\beta_0/d\ln a$, so a hidden
  multiplicative factor in the trace is excluded at the integrated level.
- **It does not give** the *local* field $\Theta$. That needs N1 (H6 §6). The integrated insertion includes the region
  where $y$ approaches the loops of $F$. Contact contributions there are part of the left side of $(\ast)$, and only N1
  plus the extension clause of H6(a) separates them.
- **It does not give** pointwise convergence in $u$. Upgrading from $\mathcal D'$ to pointwise needs equicontinuity of
  $u\mapsto\langle F\rangle_n$, which is again a uniform bound on the integrated $\Theta_n$ insertion.
- **Hypothesis bookkeeping.**
  - (L1) is a regularity statement about H4's scale-setting map at fixed $n$: solvability, and $C^1$ dependence of the
    solution of $t^2\langle E_t\rangle=c$. It is plausible, because the expectation is analytic in $\beta$. It needs
    monotonicity in $\beta$, which I have not proved.
  - (L2) is H4(a) for base spacings in an interval, not only $a_0$. This is a mild uniformization of H4 as worded.
  - On my sphere family (H4′), steps 1–4 go through verbatim with two bare parameters, but $\Theta_n$ gains the
    anisotropy term $\frac{dc_0}{du}\,\partial_{c_0}S_{\rm lat}$ ([K82]).
- **Status if adopted:** [D, conditional on H4(a) for base spacings in an interval, and (L1)]. H6 itself stays [X].

### Why this is the right first bite

Bee's H6 §6 asks for a uniform two-point bound on local $\Theta_a$, which is a real estimate. The proposition above is
its zero-momentum, integrated shadow. It is exactly what H4 already pays for, it fixes the normalization H6(b3)
asserts, and it is the non-perturbative counterpart of [BEKLLLP96] eq. (2.10) and of [CDJ77]. It also writes the trace
anomaly as the derivative along the bundle's scale coordinate.
