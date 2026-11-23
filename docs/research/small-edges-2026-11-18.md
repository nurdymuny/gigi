# The small edges: three ideas from the Crouzeix, harmonic and graph-fold work

2026-11-18. Research note, written **before** any of it is measured. Nothing here
is a result. Each idea says where it came from, how strong the link is, what we
would run, and what would prove it wrong.

## Where this note came from

**In plain English.** While S2b was finishing, I pointed at three pieces of my
own work: my Crouzeix-conjecture work inside GIGI, the harmonic and holomorphic
tools GIGI gained from it, and my "Good Will Hunting graph fold" article on the
Davis Geometric site. I suggested they might help us understand "the smaller
edges". This note writes out what each one offers, in plain English first and
then in technical detail.

"The smaller edges" can mean two things here, and both matter:

1. **Small-scale roughness.** These are the fine, jagged flux patterns that the one-ring
   recovery mistakes for real curve and amplifies. That amplification is what made one ring
   unstable in the Abelian paper (489 of 961 physical modes with negative real part at
   n = 2). It shows again in SU(2) (S2b: 207 and then 1,451 rough modes).
2. **The smallest piece of rim.** This is the least neighbourhood that still reads the curve
   reliably. One ring is too little. Two rings work. Nobody has asked what lies in
   between.

Idea 1 addresses roughness over time, idea 2 the smallest rim, and idea 3 both.

### Provenance, source by source

| Source | What it is | Where it lives |
| --- | --- | --- |
| My message, 2026-11-18 | The prompt: "my Crouzeix conjecture work … the holomorphic harmonics and the Good Will Hunting station fold … might help us understand the smaller edges" | quoted in this note |
| Crouzeix status ledger | My C3 ledger (2026-10-29): the factors F1 = ‖B‖/w(B) and F2, and the non-normality index η(B) = ‖BB*−B*B‖/‖B‖² with Lemmas 1–3 | `davis-wilson-lattice/validation/crouzeix/C3_STATUS.md` (untracked folder in that repo) |
| GIGI Crouzeix protocol | How Crouzeix data is loaded into GIGI; GIGI's job is "a certified upper bound on max \|p\| over a disk, with a disclosed error budget" | `davis-wilson-lattice/validation/crouzeix/GIGI_CROUZEIX_PROTOCOL.md` |
| HHE/Crouzeix diagnostic on the Ricci control | Mass whitening and the harmonic-conjugate polynomial bound on the 142-dimensional Abelian operator. Modes above eigenvalue 8 carry 0.00237851% of the field's squared mass norm and 99.97958017% of the squared strong-flow residual | `docs/RICCI_OPERATOR_CONSISTENCY_SPEC.md` (HHE/Crouzeix extension) and `docs/RICCI_OPERATOR_CONSISTENCY_RESULTS.md` ("What the HHE/Crouzeix work contributed"), both on this branch |
| Harmonic-conjugate endpoint | `POST /v1/bundles/{name}/brain/harmonic_conjugate`: from a complete real harmonic polynomial it recovers the conjugate, the holomorphic function, derivatives and a full-complex bound | `src/geometry/harmonic_conjugate.rs`, `docs/HARMONIC_CONJUGATE_SPEC.md`, `docs/HARMONIC_CONJUGATE_RESULTS.md` (main working tree, not yet committed) |
| Bounded-holomorphic endpoint | `POST /v1/bundles/{name}/brain/holomorphic`: bounded (Schur) interpolation on the unit disk with a Pick feasibility test and marginal admissible-value disks | `src/geometry/holomorphic.rs`, `docs/BOUNDED_HOLOMORPHIC_SPEC.md`, `docs/BOUNDED_HOLOMORPHIC_RESULTS.md` (main working tree, not yet committed) |
| Good Will Hunting graph fold | My article: fold stations 1 and 4 into one label S; the matching-menu (equitable partition) test; AP = PB, hence AⁿP = PBⁿ; 14 walks of length 4; GIGI replay to length 128 | site source `notebook/content/good-will-hunting-graph-fold.html` in `math-website`, origin/main commit `8eb87c00` (2026-11-11); replay `examples/good_will_hunting_walks.rs` (main working tree) |
| My counting papers, as the article cites them | *Zero Does Not Exist* (path addition P_a ⊕ P_b = P_{a+b−1}); my connected-partition counting paper (contraction) | cited inside the article |
| Outside literature | Crouzeix and Palencia, *The numerical range is a (1+√2)-spectral set*, SIAM J. Matrix Anal. Appl. 38 (2017) 649–655; Berger–Pearcy ‖B‖ ≤ 2w(B); Chan and Godsil, *Symmetry and Eigenvectors* §§5–6 (equitable partitions, partition refinement). The full conjecture (constant 2) is recorded in my protocol as proved by Lorist and Schwenninger (arXiv:2608.03841). That citation was not re-checked here, and nothing below depends on it. | as cited |
| S2b (this branch) | The SU(2) two-ring operator has no rough modes, but it is strongly non-normal (\|Im λ\| up to 150) | `docs/RICCI_SU2_PG_STABILITY_RESULTS.md`, `research/su2-pg-stability-20261117/` |

**Who contributed what.** The sources above are my work, apart from the
outside literature and S2b's measurements. The *translations* into the curvature-recovery
problem below were worked out here, not in those sources. Each one carries an
altitude tag:

- **STRUCTURAL:** a theorem applies directly, once its stated hypotheses are checked.
- **RESONANCE:** the same question, asked of a different object. It suggests an
  experiment, and proves nothing.

---

## Idea 1. Crouzeix bounds the time integration of a non-normal force

*Altitude: STRUCTURAL. The theorem applies to the linearised operator. Nothing here
speaks for the nonlinear flow.*

### In plain English

S2b showed that the two-ring SU(2) force has no "rough" eigenvalues. But its
eigenvalues come with large imaginary parts: the operator is lopsided (non-normal).
For a lopsided operator, eigenvalues tell you where the motion ends up, not how
much it can swell on the way there. A brief swell, before things settle, would hit
the small rough pieces first, and those are exactly the pieces the Ricci control
showed to be the most violent (0.0024% of the field carrying 99.98% of the residual).

My Crouzeix work is about exactly this gap. Crouzeix's question is how much a
lopsided matrix can amplify, judged only by its *numerical range*. The numerical
range is a convex region in the complex plane that contains the eigenvalues but
also "sees" the lopsidedness. Its left edge alone bounds the exact flow's growth: if
the region stays to the right of zero, nothing can swell. For the time-stepping rule
we will actually run, the proved theorem (Crouzeix–Palencia) says the rule's
amplification is at most 1 + √2 ≈ 2.414 times its peak over the region. So one
picture, the numerical range, tells us both whether a swell can happen and how large
the time steps can safely be.

The symmetric Ricci control did not need this: after mass whitening it was
symmetric, and the spectral theorem was sharper (Ricci consistency spec). **The SU(2)
two-ring operator does need it.** It is non-normal in the physical mass norm itself.

### Technical

The semi-discrete flow is B ξ′ = −K ξ, linearised at the Berger field (S2b convention:
K = ∂a/∂ξ, B the native SU(2) tangent mass). With the Cholesky factorisation B = RᵀR
and y = Rξ (so ‖y‖₂ = ‖ξ‖_B),

  y′ = −Ĝ y, Ĝ = R⁻ᵀ K R⁻¹.

The generalized spectrum of (K, B) measured in S2b is the spectrum of Ĝ. The numerical
range is W(Ĝ) = {⟨Ĝy, y⟩ : ‖y‖ = 1}.

- **Leftmost point, and the exact flow.** min Re W(Ĝ) = λ_min(½(Ĝ + Ĝᵀ)) =: −m. The
  numerical-abscissa bound gives ‖e^{−tĜ}‖ ≤ e^{mt} with constant 1, so Crouzeix is not
  needed for the exact flow. If m < 0, this is the paper's dissipativity Proposition:
  no swell at all. Because S2b's native operator has an eigenvalue at −4.65, which lies
  in its W, a field that is itself moving forces m ≥ 4.65 there. The question is
  whether the two-ring operator's m is close to the native one's, or much larger. A much
  larger m would allow transient growth that the eigenvalues did not show.
- **Time stepping, Crouzeix–Palencia.** For f holomorphic inside W and continuous up to
  its boundary, ‖f(Ĝ)‖ ≤ (1+√2) sup_{z∈W} |f(z)|. This covers any polynomial or rational
  stepping rule, and powers of it, with no symmetry assumption.
  - Implicit Euler, r(z) = 1/(1 + hz), with its pole at z = −1/h outside W (that is, h < 1/m):
    ‖(I + hĜ)^{−k}‖ ≤ (1+√2) sup_W |1 + hz|^{−k}.
  - Explicit Euler, r(z) = 1 − hz: ‖(I − hĜ)^k‖ ≤ (1+√2) sup_W |1 − hz|^k.
    Because W stretches to |Im| ≈ 150 and beyond, this gives a computable step limit.
    It is a design input for the time-integration rung, not a guess.
- **My own diagnostics, applied unchanged.** My ledger's two non-normality indices,
  computed for Ĝ:
  - η(Ĝ) = ‖ĜĜᵀ − ĜᵀĜ‖ / ‖Ĝ‖² ∈ [0, 1] (Lemma 1), a single number for how lopsided
    the operator is;
  - F1 = ‖Ĝ‖ / w(Ĝ) ∈ [1, 2] (Berger–Pearcy).

  These come from the native, two-ring and one-ring operators, at n = 1 and 2, from the
  already-saved spectra setup.

Computing W: rotate, Ĝ_θ = ½(e^{iθ}Ĝ + e^{−iθ}Ĝᵀ), and take λ_max over a θ grid
(Johnson's method). Each θ is one symmetric eigenproblem of size 426 or 3,372. The
leftmost point alone is a single symmetric eigenproblem.

### What we would run (gates fixed now)

- **W-left.** Report min Re W for native, two-ring and one-ring at n = 1, 2.
  - *Gate:* the two-ring value lies within a factor of 2 of the native value. A
    two-ring value below −10 means a swell exists that S2b's eigenvalues hid. That
    would be reported as a finding against the force.
- **Removal (the gate must fail when the mechanism is removed).** The one-ring operator
  must fail W-left at least as badly as it failed G-rough. If it passes, the gate is
  not measuring roughness.
- **Reported, not gated:** η, F1, the full W boundary, and the explicit-Euler step limit
  that Crouzeix–Palencia implies.
- **Control:** the eigenvalues of Ĝ reproduce S2b's saved `eigs.npy` to 1e-8 relative.
  Ĝ is built from the same K and the same B.

### Honest limits

- This is linear, at one field. A bound on e^{−tĜ} is not a bound on the nonlinear flow.
- K is a central-difference Jacobian, with O(ε²) error at ε = 1e-6.
- Gauge directions sit in W near zero. They do not threaten the left-edge gate, but
  they will appear in the picture.

---

## Idea 2. The fold: the smallest piece of rim that still reads the curve

*Altitude: RESONANCE for the stencil question. It becomes STRUCTURAL only if an
AP = PB identity is proved for a specific reduction, which this note does not do.*

### In plain English

In my article, two subway stations can share one label when they offer the same
"menu": the same number of ways onward into every group. When the menus match, the
smaller map gives exactly the same journey counts, for every journey length. That is
the identity AP = PB, carried to every power. When the menus do not match, merging by
*averaging* gives the wrong count. My lesson is to keep the differences that change
the answer, and let go of the ones that don't.

The curvature recovery has the same question hiding in it. One ring is the smallest
rim. It has exactly as many independent pieces of information as unknowns (11 and
11), so it cannot tell real curve from roughness: it interpolates, and a rough pattern
becomes a steep fake slope. Two rings fix this. But they add 24–28 faces (closure space
26–29 against 11), and very likely many of those faces repeat information the fit
already has, so they add no steadiness. Folding asks which of them actually matter.
If a stencil between one and two rings, say "one and a half", reads the curve just as
steadily, that is the smallest stable piece of rim. It would also tell us which edges
carry the real information.

A second, separate echo: the "no register" removal behaves like the bad fold. It
pools neighbour holonomies taken in different frames as if they were the same, which is
merging stations whose menus differ. It stalls the force at 29%, much as averaged
menus give wrong walk counts. This is a resemblance only, recorded so we don't lose it.

### Technical

For cell K, the recovery holds K's own four face fluxes exactly and fits the
neighbour faces by least squares, using the null-space method. Let X_K be the matrix that
maps the free coefficients to the neighbourhood's closure-space fluxes.
The paper's measured amplification is

  Λ_K ≥ 1/σ_min(X_K).

Here Λ_K is the error amplification and σ_min(X_K) is X_K's smallest singular value.
The measured ratios to that bound are 1.03–1.13 for one ring and 1.07–2.17 for two
rings. Medians are 125–252 for one ring and 30.5–35.2 for two rings.

- **Greedy fold (the "1.5 rings" question).** Start from the one-ring cell set. Among the
  second-ring cells, add the one that most raises σ_min(X_K), and repeat. Stop when
  1/σ_min ≤ 1.25 × its full-two-ring value. Record, per cell, the number of
  second-ring cells needed, and which ones were chosen. Adding whole cells rather than
  single faces keeps the flux-closure structure intact. A face alone is not an
  independent datum, because each cell's four fluxes sum to zero.
- **Exact fold of the stability operator (partition refinement).** Colour refinement on
  the weighted cell-adjacency structure finds the coarsest equitable partition with no
  symmetry spotted by hand (Chan–Godsil §5, and my article, "find the grouping
  without spotting the symmetry"). If the partition is nontrivial, the quotient matrix's
  eigenvalues are a subset of K's (AP = PB ⇒ every eigenvector of the quotient lifts).
  That would give exact access to part of the n = 3 spectrum (11,358 directions), which
  S2b deferred.
  - *Honest expectation:* the meshes are shape-regular samples, and the Berger field
    breaks the round symmetry, so the partition is probably trivial. It is a cheap
    check, and a trivial answer is reported as such.

### What we would run (gates fixed now)

- **Fold-Λ:** the distribution, over all cells, of the second-ring cells needed to
  reach 1.25× the two-ring amplification, at n = 1, 2, 3, 5.
- **Fold-force:** if the median is below the full second ring, rebuild the force with
  the folded stencil, then run BW05 (Abelian, then SU(2) Berger) and S2b's G-rough.
  - *Gate:* the folded force passes both.
  - *Removal:* a stencil of the same size made of randomly chosen second-ring cells must
    do measurably worse on Λ. Otherwise the greedy choice is not what matters.
- **Refinement:** report the coarsest equitable partition size at n = 1, 2.

### Honest limits

- The greedy choice is per cell, so neighbouring cells may choose different rims. That
  breaks the "same rule everywhere" simplicity the paper's radius-two stencil enjoys.
  If the result is to go into the paper, that is a disclosed design change.
- The amplification is a lower bound (Λ_K ≥ 1/σ_min). It is the quantity the paper
  measured, but it is not the whole error.

---

## Idea 3. Bounded harmonics as a guard rail for one ring

*Altitude: RESONANCE. It would be a different method, with its own gates. The negative
result about harmonic restriction is STRUCTURAL: it is a direct computation, shown below.*

### In plain English

My harmonic obsession started with one line in *Good Will Hunting*: "bounded
harmonic functions" (my article tells the story). GIGI now carries that idea as a
working tool. The bounded-holomorphic endpoint fills in a function from incomplete data
*without letting it exceed a stated bound*. When the data leave room, it returns a
range of admissible values instead of one forced answer. When the data contradict
the bound, it says so (the Pick test).

One ring fails because it is forced to pass exactly through every rough value, and
it pays for that with steep, fake slopes. A *bounded* one-ring fit would be allowed to
say "the data allow a range here, and I will not invent a slope steeper than the
evidence supports." That might make one ring stable without the second ring.

Two honest catches:

- **It adds a guard rail.** That is a bound, a kind of regularisation. The paper's
  current claim is that stability comes "without any damping parameter, filter or
  regularization", so a bounded one-ring fit would be a second method beside it, not a
  fix inside it.
- **It must not be *harmonic*.** It is tempting to make one ring oversampled by allowing
  only harmonic fields, which have fewer unknowns. But that would erase precisely the
  signal the force measures. Harmonic belongs as a bound, not as a constraint.

### Technical

**Why harmonic restriction kills the force.** In a chart, a linear 2-form has a vector
proxy P(x) = c + Mx: 12 parameters.

| Condition | What it imposes | Dimension |
| --- | --- | --- |
| Closed, dF = 0 | div P = tr M = 0 | **11**, the paper's unknown count |
| Closed and co-closed (harmonic) | M also symmetric (curl P = 0) | 3 + 5 = **8** |

With 8 unknowns, one ring (11 independent fluxes) would be oversampled. But the force
is −δF, and for linear P, δF corresponds to curl P, the constant set by the
antisymmetric part of M (3 parameters). Restricting to harmonic fields sets exactly those
three to zero. The Abelian control has −δdA = 4A ≠ 0, so the restriction removes the
quantity being measured.

**The bounded one-ring fit.** Keep the own-face equality. Replace the exact
neighbour interpolation by

  min ‖X_K c − y‖ subject to ‖∇P_K(c)‖ ≤ β_K.

That is: fit the neighbour data as closely as possible while keeping the field's
slope below a bound β_K. Then return the fitted c, plus a flag saying whether the bound
was active.

For consistency, the bound must never be active on smooth fields as the mesh refines.
A natural choice is β_K = κ × (the largest slope the two-ring fit would infer at that
cell's scale), with κ fixed before measurement. Another is a bound set from the data's
own spread, in the spirit of the endpoint's admissible-value disks. The Pick/Schur
machinery itself lives on the unit disk, for scalar holomorphic data. Only its
principle carries over: certify feasibility, return ranges, refuse contradictions.
The code does not carry over.

### What we would run (gates fixed now)

- **Active fraction:** the share of cells where the bound is active, at each level. On a
  consistent method it must fall under refinement.
- **Force:** BW05 (Abelian) and G-rough (Abelian S3). The bounded one-ring force must
  pass both to count.
- **Removal:** with the bound off (κ → ∞) it must reproduce the unstable one-ring
  spectrum (489 of 961 physical modes negative at n = 2). Otherwise the bound is not
  what is acting.
- **Report:** whether it beats two rings on anything (stencil size, error, operator
  normality). If it only ties two rings while adding a parameter, two rings stay the
  method.

### Honest limits

- It introduces κ, a parameter the two-ring method does not have.
- A fit with an inequality constraint is a nonlinear map of the data. The force would
  no longer be linear in the fluxes, and S3-style linear stability analysis would apply
  only around a fixed active set.

---

## Order, and what unblocks what

1. **Idea 1 first.** The flow and time integration are the next rung anyway, and idea 1
   uses S2b's operators as they are. It needs no new recovery code, only Ĝ, its
   symmetric parts and a θ sweep. Its step-size bound feeds directly into the
   time-integration spec.
2. **Idea 2 alongside it.** The greedy fold needs only the per-cell fit matrices, which
   the two-ring code already builds. Fold-Λ comes first; Fold-force runs only if
   Fold-Λ shows a real saving.
3. **Idea 3 last, and only if wanted.** It is a second method with a parameter, and it
   changes what the paper can claim. It is worth doing if the question becomes "can the
   smallest rim be made to work at all", which is a different question from "is the
   two-ring force sound".

Nothing in this note has been run. Results, if any, go to their own spec and results
documents with gates fixed before measurement, as with S1, S2a and S2b.
