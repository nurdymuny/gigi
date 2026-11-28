# Referee report on `H6-closure.md` (the integrated trace identity)

28 November 2026. This is a skeptical referee pass, written as I would want a hostile reader to write it. I did not edit
`H6-closure.md`, `H6-borrowed-routes.md` or `CLOSED-SET.md`.

**Verdict: holds with fixes.** I could not refute the mathematics. Theorem 1 (the infinite-time β-derivative at fixed
$(n,L)$) is a correct elementary proof once one bookkeeping gap in Lemma 5 is repaired. Theorem 2 and Corollary 3 follow
correctly from Theorem 1 and the stated hypotheses. The sign correction against the research note is right. The
citations are real and say what is claimed. My objections are about one gap in a lemma, about framing (how much the
result says about H6(b3)), and about how the status should be written in the register's vocabulary.

---

## 1. Citations: checked against primary text

I downloaded and read the arXiv PDFs myself (text extracted with `pdftotext`).

- **[L10]** M. Lüscher, "Properties and uses of the Wilson flow in lattice QCD", arXiv:1006.4518 (v3, 27 Jan 2014),
  JHEP 08 (2010) 071 (journal reference confirmed by search; an erratum, JHEP 03 (2014) 092, exists and v3 should
  include it). Confirmed verbatim:
  - eq. (1.3): $S_w(U)=\frac1{g_0^2}\sum_p\operatorname{Re}\operatorname{tr}\{1-U(p)\}$, "p runs over all oriented
    plaquettes";
  - eq. (1.4): the flow equation as quoted;
  - "The existence, uniqueness and smoothness of the Wilson flow at all positive and negative times t is rigorously
    guaranteed on a finite lattice";
  - eq. (3.1): $E=2\sum_{p\in P_x}\operatorname{Re}\operatorname{tr}\{1-V_t(p)\}$, "$P_x$ being the set of unoriented
    plaquettes with lower-left corner x";
  - eq. (3.3): $t^2\langle E\rangle|_{t=t_0}=0.3$.
  - The closure's SU(2) normalization $g_0^2S_w=4\sum_{p\ \rm unoriented}s_p$ is right. I checked it numerically on a
    random SU(2) element: $\operatorname{Re}\operatorname{tr}(1-U)+\operatorname{Re}\operatorname{tr}(1-U^\dagger)=4s_p$.
  - Hypothesis mismatch, handled correctly: Lüscher's paper is SU(3), and his existence statement is for a finite
    lattice. The closure uses (1.4) only on finite slabs and defines the infinite-time observable by its own limit
    (Lemma 5). The transcription to SU(2) is routine (replace su(3) by su(2)).
- **[BEKLLLP96]** G. Boyd, J. Engels, F. Karsch, E. Laermann, C. Legeland, M. Lütgemeier, B. Petersson, "Thermodynamics
  of SU(3) lattice gauge theory", arXiv:hep-lat/9602007, Nucl. Phys. B 469 (1996) 419–444. Confirmed:
  - eq. (2.8) $\frac{f}{T^4}\big|_{\beta_0}^{\beta}=-N_\tau^4\int_{\beta_0}^\beta d\beta'[S_0-S_T]$;
  - eq. (2.10) $\frac{\epsilon-3p}{T^4}=N_\tau^4T\frac{d\beta}{dT}[S_0-S_T]$;
  - eq. (2.11) $T\frac{d\beta}{dT}=-a\frac{d\beta}{da}$;
  - the sentence "one has to insure that the spatial volume is large enough" (for relating $f$ to $p$).
  - The closure's sign comparison ($-a\,d\beta/da>0$, same sign as the corrected $c_n$) is right. The closure uses this
    paper as context only, which is the correct use: their identity is for $\ln Z$ on SU(3), isotropic lattices, with
    a large-volume assumption, and they take no continuum limit of a general observable.
- **[CDJ77]** J. C. Collins, A. Duncan, S. D. Joglekar, "Trace and dilatation anomalies in gauge theories", Phys. Rev. D
  16 (1977) 438–449 (INSPIRE 110394). The INSPIRE abstract does say the anomaly "involves gauge-variant operators that
  mix with the naive trace". The paraphrase is fair. It is used for nothing, and that is correctly stated.

**No citation problems in the closure.** I did not re-verify the research note's other references ([KSZ75a/b], [JL76],
[N77], [R88]/[R89], [FHH16], [FH16], [S13], ...). The closure does not use them. The research note itself flags that
[N77] was confirmed only from secondary bibliographies.

---

## 2. Step-by-step check

### Setting and the minorization (Lemmas 1–2): correct
- Lemma 1 (Doeblin) is the standard one-line argument, correctly done. ν is a probability measure, so $p\ge\delta$
  makes $q=(p-\delta)/(1-\delta)$ a Markov density.
- Lemma 2. I rechecked the forward bound: $\varphi^+_j(z')\ge e^{-2\beta P}I_{j-1}$,
  $\varphi^+_{j+1}(z)\le I_j\le I_{j-1}$ (because $K\le1$), so $p_k\ge e^{-4\beta P}\ge\delta=e^{-4\beta_+P}$. The
  case $j=0$ is also fine.
- What makes this work is that each slice space Σ is **compact** and each slice-to-slice weight is **bounded**:
  $0\le w\le 2P$. This holds only at fixed $(n,L)$, and δ is astronomically small ($e^{-24\beta N^3}$). The closure
  says so (§6). It is a fixed-lattice statement only, and it is not presented as anything more.

### Lemma 3 (decay): correct
The conditional-expectation argument plus $c-b$ applications of Lemma 1 gives the bound with constant $2\|X\|\|Y\|$. It
is used only for cylinder functions, so passing to $\mu_\infty$ by Lemma 4 is legitimate.

### Lemma 4 (infinite time): correct
- (i) The key point is that the backward kernels of a free-ended slab depend only on the distance to the left end. I
  checked this, and it holds. So the laws of $z_0$ under the slabs $[0,j]$ and $[0,j']$ differ only through the law of
  $z_j$, and Doeblin contracts that difference.
- (ii) The sup-log constant: $|\log\psi-\log\psi'|\le e^{2\beta P}|\psi-\psi'|$, and likewise for the normalizations,
  gives $4e^{2\beta P}(1-\delta)^j\le4\delta^{-1/2}(1-\delta)^j$. Correct.
- (iii)–(iv) are correct. (v)–(vi) (boundary weights, periodic time) are plausible as written. They need the
  observation that (i)'s bound does not depend on the right-end weight, which the note makes. These parts matter only
  for identifying $\mu_\infty$ with H4's transfer-matrix state, which is also the state the register's lines 2 and 27
  use.

### Lemma 5 (quasi-locality of the flowed observable): a real but repairable gap
The proof says: "The vector fields differ only on the spatial links at slices $\pm k$." **That is false as written.**
By the closure's own convention, slice $z_k$ contains the temporal links from $k$ to $k+1$. Those links lie in the slab
$[-k,k]$, but every plaquette through them uses spatial links at $k+1$, which lie outside the slab.

- In slab $k$ their vector field is therefore **zero** (the links are frozen).
- In slab $k'>k$ it is not zero.

In the closure's depth assignment those temporal links sit at depth 1. The iteration step $f_1\le c\int f_0$ fails for
them, because their discrepancy is not driven only by neighbours at depth $\ge0$; their own vector field differs.

*Repair.* Put the temporal links of $z_k$ into the depth-0 set as well. Then:

- every link at depth $\ge1$ has the same vector field in both slabs, with all its neighbours in slab $k$;
- loop links keep depth $\ge k-k_\gamma$. I checked this by hand: a temporal link $j\to j+1$ is at graph distance
  $k-j$ from the spatial links at $k$, and at distance $\ge k-j+1$ from the temporal links of $z_k$, because temporal
  links at different times share no plaquette.

So $\varepsilon_k$ is unchanged. This is a fixable error, not a fatal one. The statement of Lemma 5 survives.

Smaller points in Lemma 5:
- "$|N(\ell)|\le19$" counts the link itself (6 plaquettes × 3 other links + 1). Fine.
- The "holonomy" bound $|\tfrac12\operatorname{tr}A-\tfrac12\operatorname{tr}B|\le\|A-B\|\le d(A,B)$ is right for the
  operator or Frobenius norm and the geodesic distance, up to the normalization of the metric. That only changes
  $D_0$ and $c$.
- The Dini-derivative step is correct. Bi-invariance gives $d(e^{hX}V,e^{hY}W)\le d(V,W)+d(e^{hX},e^{hY})$, and
  $d(e^{hX},e^{hY})=h|X-Y|+O(h^2)$.

### Theorem 1: correct, with one typo
- (a) The cylinder case is correct. The tail bound is uniform in $M$ and in $\beta\in I$, and uniform convergence of
  the derivatives gives the $C^1$ limit.
- (b) The domination argument is correct. "$k_0$" is never defined; it should be $k_\gamma$ (the time support of the
  loops). With that reading, the dominating sequence $P\big(4C(1-\delta)^{|j|/2-1}+8\varepsilon_{\lfloor|j|/2\rfloor}\big)$
  is summable and independent of $k$ and β.
- The flow is independent of β: Lüscher's generator is $g_0^2S_w$, so the $1/g_0^2$ cancels. Without that the
  derivative would pick up an extra term. The closure states it and it is true.
- Status [P] at each fixed $(n,L)$ is earned, after the Lemma 5 repair.

### Theorem 2: correct, and almost a tautology given Theorem 1
- Steps 1–2: $\sum_ya_n^4\langle F;\Theta_n\rangle=c_n\sum_p\langle F;s_p\rangle$, because each plaquette is based at
  exactly one $y$ and $a_n^4a_n^{-4}=1$. Also $f_n'(u)=e^u\beta_n'(e^u)\,\partial_\beta\langle F\rangle=c_n\sum_p\langle F;s_p\rangle$.
  The signs agree. I rechecked.
- Steps 3–4: integration by parts at fixed $n$, then bounded pointwise convergence on $\operatorname{supp}\varphi'$.
  Correct.
- **Referee's remark.** Once Theorem 1 is in hand, $(\ast)$ is the general fact that $f_n\to S$ boundedly and
  pointwise implies $f_n'\to S'$ in $\mathcal D'$. The closure itself concedes this as Scope 2: it holds for **any**
  $C^1$ path of couplings. All the analytic content is in Theorem 1.

### The sign correction: confirmed
Along the line of constant physics, $\beta_0(a)=B(t_0/a^2)$ and $s\,d\beta_n/ds=-2\tau_0B'(\tau_0)=d\beta_0/d\ln a$. So
the research note's definition "$-d\beta_0/d\ln a:=s\,d\beta_n/ds$" has the wrong sign, and with it $(\ast)$ is off by
an overall sign.

- I checked the one-loop case numerically with $\beta_0=\frac{b_0}{2\pi^2}\ln\frac1{a\Lambda}$: at $s=1$,
  $s\,d\beta_n/ds=-0.371511$, which equals $-b_0/(2\pi^2)$ for $b_0=22/3$.
- The corrected $c_n=-s\beta_n'$ is positive. It agrees with H6-local-fields.md §4's own formula
  $\Lambda\partial_\Lambda S=-(d\beta_0/d\ln a)\langle X;S_W\rangle=\int\langle\Theta_aX\rangle$, and with Boyd et al.
  (2.11).

### Corollary 3: correct under (A), (B) and the uniqueness of $B$
- The lattice-unit bookkeeping is exact. Under (A), loops, flow times $t/a^2$, smearing weights $a^4f$ and $N=L/a$ all
  match.
- $\beta_n(s)=B(t_0/(sa_n)^2)$ is literally the base-$sa_0$ coupling at level $n$, because $B$ is defined in lattice
  units on the infinite-volume lattice.
- (B) is genuinely stronger than H4(a) as worded. H4(a) speaks of "the full dyadic sequence" from the one base $a_0$.
  The closure correctly lists (B) as a hypothesis. The research note had claimed that "the limit" already includes it.

---

## 3. Do the hypotheses really hold here?

- **Group.** SU(2) throughout. [L10] is SU(3), but only the definition of the flow is borrowed, and that transfers
  trivially. Nothing depends on the group except compactness and the bounded plaquette action, so the proof transfers
  to every compact $G$. (That is not claimed.)
- **Geometry and volume.** The space is a periodic box $(a_n\mathbb Z/L\mathbb Z)^3$ with infinite time. Everything is
  at fixed $L$, the identity is not extended to $\mathbb R^4$, and the closure says so.
- **Perturbative vs non-perturbative.** No perturbative theorem is used. The identification of $c_n$ with
  $b_0/(2\pi^2)+O(\beta_0^{-1})$ and of $\Theta$ with $\frac{\beta(g)}{2g}[F^2]$ is explicitly **not** claimed. Correct.
- **Coupling regime.** Any $\beta\ge0$, at fixed lattice. Correct, because the Doeblin constant is only used at fixed
  $n$.
- **Register mismatch (fixable).** `CLOSED-SET.md` now states H6 on the **sphere family** (H4′, line 23; change-log
  item "H6 is stated on the sphere family"). `H4-continuum-limit.md` is kept only as "one half of H4′".
  - (L2) is torus H4(a), uniformized over an interval of schemes. That is not one of the register's seven hypotheses.
  - (L1) needs the four-direction infinite-volume lattice state and a unique $C^1$ solution $B$. That is also not a
    register line, and the closure's Lemma 4 does not cover it.
  - So if this is adopted under H6(b3), the entry must say "conditional on torus-H4(a) for an interval of schemes,
    plus (L1)". It cannot be read as "conditional on H4′". The closure says this in §6 ("Not on the sphere family as
    such"), but §7's status line does not repeat it.
- **Observable class.** $\mathcal O_{\rm Lip}$ is not H4's $\mathcal O$ ($\Phi\in C_b$). The closure correctly declines
  to extend $(\ast)$ by density, since the left side is not controlled in $\|\Phi\|_\infty$. Honest.
- **Meaning of the infinite-time flowed observable.** H4 never defines the lattice flow on the infinite-time lattice.
  The closure fixes it as the uniform limit of slab flows (Lemma 5). (L2) must be read with that meaning. That is
  acceptable, but it is a definitional choice to record when (L2) is matched to H4(a).

---

## 4. Is the claimed status honest?

Mostly yes. The closure keeps H6 at [X], changes nothing in the register, and lists what is not given (no local field,
no pointwise-in-scale derivative, no $\beta/2g$ identification, no non-triviality, no infinite volume, Lipschitz only).
Three corrections:

1. **"No further multiplicative renormalization" carries no evidential weight here.** In §7 and in the "Naming the
   prefactor" paragraph, the phrase reads as though a nontrivial H6(b3) claim were verified. At the integrated level,
   the coefficient $c_n=-s\beta_n'$ is exactly the chain-rule factor, so the identity cannot fail with this normalization
   and must fail with any other. The real H6(b3) content is that the *local* renormalized trace is this lattice operator
   with this coefficient. That is untouched, and the closure says so in §6 ("No local field", "No identification"). §7
   should then say plainly: "the integrated identity holds with the chain-rule normalization; this is a consistency
   check, not evidence about the local field." The research note's sentence "a hidden multiplicative factor in the
   trace is excluded at the integrated level" overstates in the same way.
2. **Tag vocabulary.** The register's tags are [P], [PC], [C], [Cd], [D], [X], with the form "[D on 20, 23]" used for
   deductions from register lines. "[D, conditional on (L1),(L2)]" uses hypotheses that are not register lines. The
   honest entry is [Cd] on named off-register inputs (torus H4(a) for an interval of schemes, plus (L1)), or the
   (L1)/(L2) must first be written in as sub-hypotheses of H4′/H6.
3. **"Every step below is proved here."** That was not quite true before the Lemma 5 repair. After it, it is.

**Strongest status the deduction earns:**

- Theorem 1 is [P] at each fixed $(n,L)$, after the Lemma 5 depth-set repair and with $k_0\to k_\gamma$.
- Theorem 2 is [Cd]. It is a correct, essentially kinematic integrated trace identity on the hypercubic torus
  regulator, for $\mathcal O_{\rm Lip}$, distributional in the scale variable, conditional on (L1) and (L2). Neither is
  a register hypothesis, and (L2) is the superseded torus H4(a), uniformized over schemes.
- Corollary 3 is [Cd], conditional additionally on (A), (B) and uniqueness of $B$.
- H6 and the seven top-level hypotheses stay unchanged. The result adds no evidence for the local H6(b3) claim
  beyond consistency.

---

## 5. What I tried and failed to break

- **Sign.** Rederived twice (once with the exact $B$, once at one loop numerically). The closure is right and the note
  is wrong.
- **Plaquette counting.** "Six based plaquettes" matches Lüscher's $P_x$ (unoriented, lower-left corner). Each
  plaquette is counted once. Correct.
- **Uniformity in β for the derivative interchange.** δ depends only on $\beta_+$, so all bounds are uniform on compact
  β-sets. Correct.
- **Absolute convergence at infinite time for flowed (non-cylinder) $F$.** This needs summable approximation errors,
  which Lemma 5 supplies (after the repair) with factorial decay. Correct.
- **Hidden dependence on the scale condition in Theorem 2.** None. (L1) is only $C^1$ regularity of an arbitrary path.
  That is correct and is Scope 2.

Sources:
- [arXiv:1006.4518 (Lüscher 2010)](https://arxiv.org/abs/1006.4518); [doi:10.1007/JHEP08(2010)071](https://www.doi.org/10.1007/JHEP08(2010)071)
- [arXiv:hep-lat/9602007 (Boyd et al. 1996)](https://arxiv.org/abs/hep-lat/9602007)
- [INSPIRE 110394 (Collins–Duncan–Joglekar 1977)](https://inspirehep.net/literature/110394); [doi:10.1103/PhysRevD.16.438](https://link.aps.org/doi/10.1103/PhysRevD.16.438)
