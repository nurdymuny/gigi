# Adversarial Review of `paper_kahler_substrate_v0.2.tex`

**Reviewer persona:** Nitpicky differential geometer + suspicious database-systems reviewer + careful applied mathematician. Reads provisional patents for fun. Has never met the author. Has a Friday afternoon to read this and a paper to reject quota to fill.

**Date:** 2026-05-26 (Sacramento author time)
**Paper version reviewed:** v0.2 (21 pp, compiled `paper_kahler_substrate_v0.2.pdf`)

**Bottom line up front:** The empirical evidence is strong; the math has real bones; the engineering claim (optionality contract) is genuinely novel. But the formal apparatus is loose in places that a hostile reviewer will exploit. There are **two definitional inconsistencies, two overclaimed theorems, one quantitatively imprecise comparison, and several missing derivations.** Fix the severity-1 items before any external submission. The severity-2 items can wait but should be acknowledged in a revision.

Severity scale: **🔴 1 = fix before submission** (actual error or overclaim); **🟡 2 = needs work** (weak justification, missing derivation); **🟢 3 = nitpick** (style, precision).

---

## 🔴 Severity 1 issues — fix before submission

### S1.1 Definition 1.1 has internal contradiction on which connection is meant

**Location:** §1, Definition 1.1 (the Davis Kähler generator)

**Current text:**
> "$J$ satisfying $J^2 = -I$ and $\nabla J = 0$; $\nabla$ is the Chern connection induced by $g$ and $J$"

Then in the §4.1 expansion:
> "$\nabla J = 0$, where $\nabla$ is the Levi-Civita connection of $g$"

**Problem:** The Chern connection and the Levi-Civita connection are *in general different connections* on different bundles. The Chern connection is the unique connection on the holomorphic tangent bundle $T^{1,0}M$ that is (a) compatible with the Hermitian metric $h(\cdot, \cdot) = g(\cdot, \cdot) + i\omega(\cdot, \cdot)$ and (b) such that the $(0,1)$-component of $\nabla$ equals $\bar\partial$. The Levi-Civita connection is on the real tangent bundle $TM$.

On a *Kähler* manifold, the Chern connection on $T^{1,0}M$ extends to a real connection on $TM$ that *coincides* with the Levi-Civita connection — this is in fact one of the equivalent characterizations of Kähler. But this is a non-trivial theorem (Kobayashi–Nomizu Vol II, Ch. IX, Thm. 4.6), not a definition.

**A geometer will flag this as the author confusing two connections.**

**Fix:** Pick one of the two consistent framings.

*Option A (cleanest):* drop both phrases. Just say "$M$ is a Kähler manifold" and let the unique-Kähler-connection theorem be implicit. Add a footnote: *"By the Kähler property, the Levi-Civita connection of $g$ on $TM$ coincides with the (real extension of the) Chern connection on $T^{1,0}M$. We refer to either as $\nabla$ without ambiguity."*

*Option B (more careful):* explicitly say "$\nabla$ denotes the Levi-Civita connection of $g$. On a Kähler manifold, $\nabla J = 0$ and $\nabla$ coincides with the Chern connection on $T^{1,0}M$ under the canonical identification (Kobayashi–Nomizu Vol II §IX)."

### S1.2 Theorem 5.1 (Optionality contract) is an empirical claim, not a mathematical theorem

**Location:** §5.1, Theorem 5.1

**Current text:**
> "A layered geometric upgrade to a production database engine can ship eight layers, approximately 5,000 lines of new code, and approximately 180 new tests without changing the no-feature engine build's observable behavior."

With a "proof sketch" that says the no-feature build dead-code-eliminates Kähler symbols and a CI byte-equality check verifies binary identity.

**Problem:** This is not a theorem. It's an empirical claim about the behavior of a specific Rust codebase under a specific compiler version, with a specific CI setup. Calling it a "Theorem" in the LaTeX `theorem` environment is overclaiming.

Additionally, "bit-identical binary" is not always achievable in modern Rust — `cargo` and `rustc` don't guarantee deterministic output without explicit reproducible-builds configuration (timestamps in `.rlib` metadata, hash seeds, etc., can vary). The actual claim is closer to "behaviorally bit-identical at the test-output level" or "byte-equal under a reproducible-build configuration."

**A skeptical database-systems reviewer will note that "theorem" is being used in the colloquial "claim" sense rather than the mathematical "proven implication" sense.**

**Fix:** Demote to one of:

*Option A:* `\begin{proposition}` (mathematical-adjacent claims about engineered systems).

*Option B:* a named italicized `\textit{Empirical observation 5.1}` with no `theorem`-environment formatting. State explicitly that the claim is empirical, the verification is via CI, and the binary identity is established under a specified reproducible-build configuration.

*Option C* (best): split into two claims. **Proposition 5.1 (formal):** "Under reproducible-build configuration, the no-feature engine compiles to the same binary independent of which Kähler layer commits are checked out." **Engineering observation 5.2:** "In our CI history across 8 layer commits, the 720-test no-feature run produced identical PASS counts and within-noise timings."

### S1.3 Theorem 6.1 (Self-consistency of the meter) is a definitional identity, not a theorem

**Location:** §6.4, Theorem 6.1

**Current text:**
> "The drift applied by quasi-batch recomposition equals the meter reading exactly. That is... $\text{drift\_applied} = \|\mathbf{r}_{\text{batch}} - \mathbf{r}_{\text{seq}}\|_2 = \text{meter}(\mathbf{r}_{\text{seq}}, \mathbf{r}_{\text{batch}})$."

With proof:
> "The meter is defined as the $L^2$ distance between the two residues. The drift is defined as the magnitude of the substitution... Norm symmetry gives equality."

**Problem:** If drift is defined as $\|\mathbf{r}_{\text{batch}} - \mathbf{r}_{\text{seq}}\|_2$ and meter is defined as $\|\mathbf{r}_{\text{seq}} - \mathbf{r}_{\text{batch}}\|_2$, then their equality is the *definition of the $L^2$ norm being symmetric*. That's not a theorem; that's $\|x - y\| = \|y - x\|$.

**A pure mathematician will note that what's being claimed is trivially true by definition.** The operationally significant claim — that the codebase computes these two quantities via different code paths and they still agree — is an engineering claim about the implementation, not a math theorem about the residues.

**Fix:** Reframe completely. Two cleaner options:

*Option A (math version):* Drop the theorem. Add a `\begin{remark}` saying "Under the definitions of drift and meter (both as $L^2$ distance between residues), the two quantities are equal by symmetry of the norm."

*Option B (engineering version):* Reframe as **Proposition 6.1 (Operational equivalence of drift and meter).** "Although the implementation computes drift and meter via different code paths — drift from the runtime substitution event, meter from a separate measurement query — both reduce to the same $L^2$ distance. Therefore numerical agreement between them is guaranteed to machine precision, not approximated." This is the actually interesting claim and makes clear what the paper is asserting.

### S1.4 The "4 decimal places" claim is quantitatively imprecise

**Location:** §1 (Thesis paragraph), §6.6 (multiple instances), Abstract placeholder

**Current text (multiple places):**
> "three independent computations... agree to four decimal places"
> "Maximum relative error between analytic and Wilson-loop: $\leq 3 \times 10^{-4}$"

**Problem:** These two statements are not quite the same. Looking at Table~6.2:

| $\Delta/\varepsilon$ | rel.\ err |
|---:|---:|
| 0.00 | 0.0000 |
| 0.25 | 4×10⁻⁵ |
| 0.50 | 4×10⁻⁵ |
| 1.00 | 9×10⁻⁵ |
| 2.00 | 18×10⁻⁵ |
| 5.00 | 26×10⁻⁵ |
| 100.00 | 29×10⁻⁵ |

The maximum relative error is $29 \times 10^{-5} = 2.9 \times 10^{-4}$, which is *three* significant figures of relative agreement, not four decimal places.

On the absolute value of the integrality deviation, the agreement *is* to four decimal places (e.g., 0.4472 vs 0.4473 = absolute difference $10^{-4}$). So the claim "integrality deviations agree to four decimal places" is true; the claim "the Berry-phase values agree to four decimal places" is not quite, since the values themselves are around 6.28 and the absolute error around $10^{-3}$.

**Fix:** Replace all "four decimal places" claims with one of:

- "The integrality deviations agree to four decimal places" (precise, true)
- "Relative error in the Berry phase is below $3 \times 10^{-4}$ across the sweep" (precise, true)
- "Implementations agree to three significant figures in the Berry phase and to four decimal places in the integrality deviation" (most precise)

Drop the standalone "agree to four decimal places" — pick the metric and state it.

### S1.5 The 0.4472 BLG deviation is *outside* the cited Wu–Yang range 0.33–0.40

**Location:** §6.6 (Headline paragraph)

**Current text:**
> "deviation $\mathbf{0.4472}$ in the Dirac-string region --- sitting in the same regime as Wu--Yang's reported $0.33$--$0.40$ deviation for non-integer $2q$"

**Problem:** 0.4472 > 0.40. It is *not* in the range 0.33–0.40; it is slightly above it.

A careful reviewer will not let this slide as "the same regime." They will ask why the BLG value is higher than the maximum cited Wu–Yang value, and the author needs an answer.

**The honest answer:** BLG has winding number 2 (Chern $-1$ at $\Delta = 0$ corresponds to a half of a full integer Chern accounting), while Wu–Yang at $q = 0.3, 1/\pi, 1/3, 0.7$ never has $|q| > 1$. The Dirac-string regime *depth* (i.e., how far from the nearest integer the holonomy goes) depends on the winding, and BLG's mid-bias is at a deeper non-integer position than any of the four Wu–Yang test cases.

**Fix:** Rewrite the headline paragraph:

> "The BLG mid-bias deviation $0.4472$ is comparable in magnitude to but slightly above Wu–Yang's reported deviations $0.33$–$0.40$ for non-integer charges $q \in \{0.3, 1/\pi, 1/3, 0.7\}$. Both lie in the non-integer-Chern regime where the predicate is designed to fire; the slight elevation reflects BLG's winding number 2 vs.\ Wu–Yang's tested range of $|q| < 1$. We do not claim quantitative equality between the two consumers' deviations; we claim qualitative co-firing of the predicate, with quantitative magnitudes in the same order ($\sim 0.4$) consistent with the expected substrate-specific Dirac-string depth."

This is honest and actually *more* impressive as a claim — same predicate, different consumers, magnitudes consistent with substrate-specific winding.

---

## 🟡 Severity 2 issues — needs work but not blocking

### S2.1 $\varepsilon$ is never defined in the paper before it appears in the BLG formula

**Location:** §4 (L1.5), §6.6 (the McCann–Koshino formula)

**Current text:**
> "$\gamma_{BLG}(\Delta) = -2\pi (1 - \Delta / \sqrt{\Delta^2 + 4\varepsilon^2})$"

**Problem:** $\varepsilon$ appears in the formula without definition. A reader who hasn't read McCann–Koshino can't tell what it is. The DPU spec defines it ($\varepsilon = \hbar^2 R^2 / (2 m^*)$ where $R$ is the Wilson-loop radius in $k$-space deviation from the K point and $m^* = \gamma_1 / (2 v_F^2)$ is the BLG effective mass) but the paper inherits the formula without inheriting the definitions.

**Fix:** Either:
- Insert a one-line definition: "where $\varepsilon = \hbar^2 R^2 / (2 m^*)$ is the kinetic energy at the Wilson-loop radius $R$ in $k$-space, with $m^*$ the BLG effective mass."
- Or cite McCann–Koshino §III.B precisely so the reader knows where to look.

### S2.2 The 7.6 pp non-associativity bound has no derivation in this paper

**Location:** §1 (Thesis), §6.3 (Headline number)

**Current text:**
> "closed-form non-associativity bound from L7.5 = $\mathbf{7.6\;\text{pp}}$"

**Problem:** Where does 7.6 pp come from? The number appears multiple times in the paper but no derivation is shown. A reader can't reproduce it from what's in the paper.

**Fix:** Either:
- Add a Lemma 6.X in §6.3 deriving the 7.6 pp from the L7.5 quantum cohomology parameters of the embedding-bundle ambient. ~10 lines of math.
- Or explicitly point at the catalog file `catalog.md §2.10` where the derivation lives, and note that the derivation is appendix material in this paper.

### S2.3 "Classical and deterministic" framing of the DPU is misleading

**Location:** §1 (third paragraph), and inherited from the DPU patent

**Current text:**
> "computation natively, not simulation of the physics... classical and deterministic, not quantum"

**Problem:** Berry phase is a *quantum-mechanical* phenomenon. It arises from the U(1) bundle of Bloch wavefunctions of a single quantum particle. Calling the DPU "classical" misleads readers about what physics is being exploited.

The right framing: the DPU uses *single-particle quantum-coherent ballistic transport* (the Berry phase requires phase coherence of the Bloch wavefunction) but does *not* use multi-particle superposition, entanglement, or coherent register operations (which would make it a quantum computer in the Heron / Forte sense). At the macroscopic output level (valley index of the detected electron), the device is deterministic.

A pedant will catch "classical" and either reject the framing or interpret it as the author not understanding quantum mechanics.

**Fix:** Replace "classical and deterministic" everywhere with something like "single-particle quantum-coherent at the substrate level, classically deterministic at the macroscopic output level" or "uses quantum-mechanical Berry phase but operates in a deterministic single-particle regime, distinct from coherent-superposition multi-particle quantum computing."

The patent has the same issue, but the patent is filed; the paper is not.

### S2.4 The three-way validation is three implementations of one ground truth, not three independent ground truths

**Location:** §6.6 (Three-way validation paragraph)

**Current text:**
> "Three independent computations of $|\gamma_{BLG}(\Delta)|$ are performed and compared: (a) Closed-form analytic formula... (b) Discretized Wilson loop in a host language (Python)... (c) Discretized Wilson loop in the processor's production simulation engine (Rust)"

**Problem:** All three computations are derived from the same 2-band BLG Hamiltonian. (a) is the closed-form integration of that Hamiltonian's Berry curvature; (b) is the discrete Wilson loop on the Hamiltonian's eigenstates; (c) is the same Wilson loop implemented in production code. They are *three implementations of the same underlying physics*, not three independent ground truths.

The paper's framing risks a reviewer thinking the three serve as independent confirmations, when really they serve as implementation self-consistency checks.

**The actually impressive cross-validation:** the *same integrality predicate* (from the catalog's `test_7`) fires on both a *toy substrate* (Wu–Yang monopole on $S^2$) and a *real physical substrate* (BLG Berry phase). That's substrate-independence of the predicate, which is the real result.

**Fix:** Rewrite the three-way validation paragraph to be honest about what's being shown:

> "Three independent implementations of the McCann–Koshino BLG Berry phase computation are compared as an implementation-correctness check: (a) the closed-form analytic formula; (b) a discretized Wilson loop in Python, independently coded; (c) the production Rust Wilson loop in `dgp-core`. Agreement among the three at $\leq 3 \times 10^{-4}$ relative error establishes that the BLG Berry phase is implemented consistently across our codebase. The substrate-independence claim is *separately*: the L7.1 integrality predicate, originally validated on a toy Wu–Yang $S^2$ monopole, fires correctly on this BLG Berry phase, with deviation magnitudes consistent with the catalog's predicted Dirac-string regime."

Two separable claims: implementation consistency (the three-way) and substrate-independent predicate (the catalog/BLG comparison).

### S2.5 The Bistritzer–MacDonald magic angle is given as 0.975° but the experimental value is ~1.1°

**Location:** §4 (L1 / L4 mentions of BM theory)

**Problem:** The paper says (and the patent says) that the magic angle is approximately 0.975°. This is the *chiral-model* BM value. The experimental value is ~1.1° due to lattice relaxation. A condensed-matter physicist reading the paper will notice the difference.

**Fix:** Add one sentence wherever the BM magic angle appears: "in the chiral BM continuum model; the experimental value with lattice relaxation included is ~$1.1°$ (Cao et al., Nature 2018)."

### S2.6 "Independently citable" claim list overlaps with the patent's claim list

**Location:** §5.4, §8 ("Independently citable" subsection)

**Current text (§5.4):**
> "The optionality contract pattern is reusable for any math-heavy production-DB upgrade."

**Problem:** The patent (No. 64/073,981) contains claim 12 covering the shared `davis-geometry` crate and the typed-refusal vocabulary. The paper's "independently citable" framing risks confusion about which claim is patent-protected and which is freely citable academic methodology.

**Fix:** Add a clarification footnote in §5.4: "*The optionality contract pattern as an engineering technique is freely citable. The specific software implementation including the typed-refusal vocabulary and the `davis-geometry` shared crate is the subject of U.S. Provisional Application No. 64/073,981 (claim 12). Other implementations of the same pattern are not covered by the provisional.*"

### S2.7 The 4 stationary sessions are a tiny sample for the §7 stationarity claim

**Location:** §7 (The surprise)

**Current text:**
> "$n = 4$ stationary sessions"

**Problem:** Four sessions is a *very* small sample. A reviewer will note that with $n = 4$, any monotonic trend has substantial probability of being coincidental.

**The honest statistical claim:** Under the null hypothesis that meter readings are i.i.d. across turns within a session, the probability of strict monotonic decrease in 3 consecutive turns is $1/3! = 1/6$. For 4/4 sessions showing this independently, the joint probability is $(1/6)^4 \approx 0.00077$. So the result is statistically significant under that null, but with the strong assumption of i.i.d.

Mention this. The §7 reads as if the n=4 is the limitation; really the limitation is the joint i.i.d. assumption (turns within a session are not i.i.d. by construction — they're a sustained conversation).

**Fix:** Add a sentence at end of §7.2: "Under the null hypothesis of i.i.d. meter readings, the joint probability of monotonic decay in all 4 sessions independently is $\approx 0.00077$, suggesting the effect is real; however, turns within a session are not i.i.d. by construction, so the appropriate null is a more careful conversation-trajectory model that we defer to a follow-up paper."

---

## 🟢 Severity 3 issues — nitpicks

### S3.1 The musical isomorphism $\sharp$ in Lemma 4.2 proof is not defined

**Location:** §4 (L1.5, Lemma 4.2 proof)

**Current text:**
> "$\nabla_{\dot\gamma} \dot\gamma = B(\dot\gamma, \cdot)^\sharp$"

**Problem:** The musical isomorphism $\sharp: T^*M \to TM$ is standard in differential geometry but not universally known. A reader from CS or physics might not know what $^\sharp$ means.

**Fix:** Either drop the notation and use the equivalent expression in coordinates, or add a one-line definition: "where $\sharp$ denotes the musical isomorphism $T^*M \to TM$ induced by $g$."

### S3.2 The phrase "two independent consumers in two unrelated domains" overclaims

**Location:** §6.6 (What this section adds)

**Current text:**
> "Two independent consumers in two unrelated domains"

**Problem:** Both consumers are $U(1)$ line bundles over compact manifolds. They are categorically the same kind of mathematical object, so "unrelated" is too strong. The substrate (data vs solid-state physics) is unrelated; the underlying math object is the same.

**Fix:** Rewrite: "Two independent consumer instantiations from unrelated physical/data substrates — Marcella's learned BGE embedding bundle and the DPU's BLG Bloch bundle — both realizing the same categorical object (a $U(1)$ line bundle over a compact base) and both validating the same predicate." This is more precise and *also* a stronger claim because it explicitly says the predicate is robust across instantiations of the same categorical object.

### S3.3 The Berger formula for K\"ahler curvature decomposition is incomplete

**Location:** §4 (L4, the Berger formula)

**Current text:**
> "Berger's formula relates the scalar curvature to the four K\"ahler invariants: $K = \frac{1}{2n+1}(K_H + K_B^{\min} + K_B^{\max})$"

**Problem:** Berger's classical formula on a Kähler manifold of complex dimension $n$ relates the scalar curvature to an integral over the holomorphic 2-planes, not just the min/max bisectional. The formula as stated is a simplification that's not exactly right.

**Fix:** Either cite Berger's original result precisely with the integral, or note that the formula given is a heuristic approximation suitable for the data-streaming context.

### S3.4 The non-trivial-validation paragraph in §6.6 doesn't explain *why* same predicate firing on both substrates is significant

**Location:** §6.6 (closing paragraphs)

**Problem:** The paper says "the catalog is not specific to one substrate" but doesn't explain why this is a meaningful claim. A philosopher of mathematics would point out that any mathematical predicate by definition is substrate-independent (if you can't apply it to two different cases, it's not a predicate, it's a description).

The interesting claim is: a predicate *that we extracted from one substrate* (the Wu–Yang toy case) reproduces the same *quantitative regime* (mid-bias deviation $\sim 0.4$) on *another* substrate (BLG). That is consistent with — but does not prove — that the predicate captures a universal feature of $U(1)$ line bundles, not a coincidence of the specific toy case.

**Fix:** Add a sentence: "The claim is not that the predicate is substrate-independent in principle (any well-defined predicate is), but that the *quantitative regime* it reports on the BLG substrate falls in the same range we calibrated on the toy substrate. This is evidence that the predicate captures a generic feature of $U(1)$ line bundles in the non-integer-Chern regime, rather than an accident of the Wu–Yang toy parametrization."

### S3.5 The "byte-identical" engineering claim should specify the determinism assumption

**Location:** §5.1 (and Theorem 5.1 proof sketch)

**Current text:**
> "the engine binary is bit-identical to the pre-upgrade engine binary"

**Problem:** Rust's reproducible builds are not enabled by default. Without explicit configuration, `.rlib` files contain timestamps, source-path hashes, and other non-deterministic metadata. The "bit-identical" claim is true only under reproducible-build configuration.

**Fix:** Add: "Under a reproducible-build configuration (deterministic compiler flags, no source-path-dependent hashing, no embedded timestamps), the no-feature builds are byte-identical. Under the default CI configuration, behavioral identity is established via the 720-test PASS-count invariant; binary byte-identity is established by a separate reproducible-build CI run."

### S3.6 Catalog references use heterogeneous notation (§E.5 vs §2.10 vs §1.1)

**Location:** Throughout

**Problem:** Sometimes the paper says "catalog §1.1," sometimes "catalog §E.5," sometimes just "catalog." A reader wanting to look these up will need to know that the catalog is `theory/kahler_upgrade/catalog.md` and that sections inside it use a custom numbering. Minor friction.

**Fix:** Add a line in §2.3 (What this paper adds): "Catalog section references throughout the paper refer to `theory/kahler_upgrade/catalog.md`. Sections numbered §1.x through §2.x are catalog Part I (Adachi program borrows); §E.x are engineering extensions discovered during implementation."

### S3.7 The "morse_compress is O(V³)" claim cites a specific dense graph but no general scaling argument

**Location:** §3.2 (`morse_compress` is O(V³)... )

**Problem:** "175 seconds on a 20-record sensor bundle" is a specific data point, not a scaling analysis. A reviewer might ask: is $O(V^3)$ the asymptotic complexity, or empirical at the cited sample?

**Fix:** Add: "The $O(V^3)$ scaling is from the algebraic Morse pairing's eigendecomposition step; on sparse graphs the complexity drops to $O(V \log V)$. The cited 175s is for the dense $V = 20, F = 1140$ case where the pairing step dominates."

### S3.8 The §1 thesis paragraph contains two distinct headline numbers (0.0747 and 7.6 pp) without making clear they're the same physical quantity

**Location:** §1 (Thesis paragraph)

**Current text:**
> "non-associativity at **0.0747** against the closed-form bound of **7.6 percentage points**... agree to within 0.0013"

**Problem:** $0.0747$ is dimensionless; "7.6 percentage points" is a fraction times 100. The two are: $0.0747$ ≈ $0.0760$ (i.e., 7.6 pp) and $|0.0747 - 0.0760| = 0.0013$. They're the same quantity in different units (one as raw fraction, one as pp). A reader has to do the conversion mentally.

**Fix:** State explicitly: "Per-turn non-associativity was measured at $0.0747$ (7.47 pp). The closed-form bound is $7.6$ pp ($0.076$). The agreement $|0.0747 - 0.076| = 0.0013$ is below sampling noise."

---

## What this review does NOT push back on

I checked carefully and have no objection to:

- The overall framing of the paper as substrate-paper-underneath-companion-runtimes
- The eight-layer catalog organization
- The optionality contract as a real engineering claim (just rename it from Theorem)
- The §6.6 DPU cross-validation table itself (the numbers are right; the framing needs S1.4, S1.5, S2.4)
- The honest-negatives section §3 (this is actually a paper strength)
- The §6.1–§6.5 Marcella validation outline (when expanded)
- The reference list (mostly complete; could add a few more current 2D-materials review citations)

## Summary of required fixes before submission

**Severity 1 (5 items):** definitional contradiction (S1.1), two overclaimed theorems (S1.2, S1.3), quantitative imprecision (S1.4), and overstated range comparison (S1.5).

**Severity 2 (7 items):** missing definition of $\varepsilon$, missing 7.6 pp derivation, "classical/deterministic" framing, three-way-validation framing, magic angle versions, IP-claim clarification, statistical-significance addendum.

**Severity 3 (8 items):** various polish.

**Estimated work:** 1 focused day to address all 12 of S1+S2. The S3 items can be folded into a copy-editing pass.

**My recommendation if this came to me for review:** *Reject with major revisions.* The empirical work is strong and worth publishing; the formal apparatus needs cleanup. The reviewer would want to see v0.3 with at least the Severity 1 items addressed.

---

*Reviewer signature: A. Curmudgeon, Department of Mathematics, [redacted]. Reviewer specialty: complex differential geometry, K\"ahler manifolds, applied algebraic topology. No conflicts of interest declared, save a general suspicion of provisional patents.*
