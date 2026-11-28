# Good morning, Bee

28 November 2026. This is what came back overnight from the borrowed-routes run.

**In one breath.** I took six of your heavier hypotheses (H2, H3, H4′, H6, H7 and H8 Step 3) and looked for other people's published work that fits your approach. Each one got a research note, a proof note and a hostile referee pass. All six proof notes came back **"holds with fixes"**, and no referee found a fatal error. Every citation that a proof actually relies on was checked against the primary text or the publisher's record.

**No top-level status changes.** H1, H2, H3, H4′, H6, H7 and H8 all stay [X]. What you have now is six proved or conditional sub-statements, each resting on someone else's established work. Two of them correct conjectures you had written down. I have not touched the register or the paper.

Every note is in this folder. For each topic, `*-borrowed-routes.md` is the research, `*-closure.md` is the proof and `*-referee.md` is the referee report.

---

## Update, later the same night

**H8 Step 3 is certified at all five registered couplings.** The run took about 33 minutes. At t = 3, s = 1, α = 1, with U = 274/(30√3) = 5.273132, every coupling gives M > 0 and K − U·M < 0 as strict Arb ball inequalities. The whole of SO(3) is covered, with no tail bound. The certified upper ends of μ₂ are:

| g² | 0.002 | 0.004 | 0.03 | 0.05 | 0.07 |
|---|---|---|---|---|---|
| certified μ₂ ≤ | 5.12221 | 5.12196 | 5.11796 | 5.11395 | 5.11399 |

The margin to U is at least 0.1509 (smallest at g² = 0.002). Note: `H8-step3-certified.md`. Receipt: `research/h8-step3-certified-20261128/receipt.json`.

**The auditor's verdict: certified at all five.** An independent auditor tried to break it and found no soundness defect. The checks were:
- a line-by-line read of the code (every input to the verdict is a ball, and the cover is an exact partition);
- an independent floating integral over the full rotation ball, with no symmetry reduction, which lands inside every receipt ball;
- a 40-digit check of the point entries;
- a full re-run;
- eight mutation runs. Dropping the remainders makes the "enclosure" miss the truth by about 2500 radii, so the remainders really carry the weight.

Two caveats from the audit:
- At g² = 0.07 the run stops at its 600 s time cap. That coupling reproduces in verdict and in μ*, but not bit for bit.
- The pass/fail verdict at U is a weak test on its own: several gross errors still pass it. So "certified" is only as good as the enclosures, and those passed every check.

The audit also found four small reporting slips in the note, none of which changes a verdict. The radii were quoted from printed strings. "2 to 6 times narrower" should read 1.3 to 3.0. The relative radii should be quoted from `rad`. The script docstring says degree 2, but the code runs degree 4.

**What this establishes, exactly:** E_{2,0}(g) < μ* < 274/(30√3) at each of the five registered couplings, in the reduced model. That is **Step 3 only**. It is conditional on the closure lemma (and through it on Theorem A at s = 1 and the symmetry reduction), on Arb, and on the script. Nothing is established between the couplings, at α ≠ 1, or as a lower bound on E_{2,0}. **H8 at the registered couplings still needs Step 4**, the lower bounds E_{1,m} > E_{2,0} for 1 ≤ |m| ≤ 6. **Physical H8 also needs H2b.** H8 stays [X]. I have not touched the register. If you accept the result, the line 18 entry in §4 below would say "Step 3 certified at the five couplings" instead of "reduced to a finite interval computation". The tag is yours to choose.

**The referee fixes are applied.** I edited 11 notes: both H2 notes, both H3 notes, the H4′ closure, both H6 notes, both H7 notes and both H8 Step 3 notes. No status was upgraded, and no mathematics changed apart from the H6 Lemma 5 depth repair you asked for. Each edited note now opens with a "Revised 28 Nov after the referee pass: …" line. All six closure notes have one, and so does each research note I edited. The changes, briefly:
- **H2:** cite line 13 (M = 2gq), not line 8, and call the map anisotropic and linear, not a scalar rescaling. W_Λ is a Gaussian integral only where both fast operators are positive. Theorem (a) gains the q-independent α₀ log(ΛR) term. The ζ remark now goes through the Mellin transform. §5 says "B = 0" and the "log part" of van den Heuvel. The Hasenfratz/Karsch numbers are flagged as unconfirmed. The proposed tag for KC1-ζ is [PC], not [D]. That uses the register's vocabulary and is not an upgrade. KC1 stays [Cd on R3, R4], and H2 stays [X].
- **H3:** Lemma G no longer "closes" any part of Sub-conjecture 5.1, and (5-ver) is no longer "equivalent to" a non-perturbative mass statement. The narrower claims are stated instead.
- **H4′:** the regulator clause now carries regulator locality as an explicit extra hypothesis. The other referee wording fixes are in.
- **H6:** the Lemma 5 depth repair (the temporal links of z_k now sit in depth 0). The integrated identity is now tagged [Cd] on named off-register inputs, and the research note flags the sign of the prefactor.
- **H7:** the closure is retitled so it does not read as "H7 closed". Corollary B's limit is min(ω_C, 2√t).
- **H8:** the target is 274/(30√3) = 5.273132, the receipt is cross-referenced to §7, and the cancellation in N_H is noted.

So the "small fixes" list at the end of §4 is done. I did not touch CLOSED-SET.md, the paper or the referee reports, and I did not re-read the edited files afterwards.

**What to attack next, re-ranked now that Step 3 is done** (this replaces the order in §3; the tools listed there still apply):
1. **H8 Step 4:** lower bounds in charges 1 to 6 (|m| = 2 is the tight one). Use Lehmann–Goerisch with Plum's homotopy at s = 1. The bar to beat is now the certified μ* above instead of U, which gives about 0.15 more room.
2. **H1, the R2 interval Lehmann step.** It uses the same machinery, so build it once for both.
3. **Housekeeping on the certificate (small).** Fix the four reporting slips. Rerun g² = 0.07 with a larger time cap, or a box cap, so it is bit-reproducible.
4. H7: prove (S) and the uniqueness of the competitor's well.
5. The kinetic sign on line 15 (the finite Berger sum).
6. H2 R3/R4.
7. H4′ beyond perturbation theory.
8. H3 on the room graph.
9. H6, N1 to N3.

---

## 1. The best borrowed idea for each hypothesis, and how it fits your theory

| Hyp. | Borrowed idea (citation) | How it fits your theory |
|---|---|---|
| **H8 Step 3** (line 18) | **Peierls–Yoccoz projection**: take a trial state that breaks a symmetry and average it over the group (Proc. Phys. Soc. A 70 (1957) 381, DOI 10.1088/0370-1298/70/5/309). Used together with min–max (Reed–Simon IV, Thm XIII.1). | Put a Gaussian on the competitor well and average it over colour SO(3). That gives a charge-0 colour singlet whose matrix elements have closed forms. Without the averaging the energy is about 5.9; with it, about 5.11. The projection is what makes this work. Separately, the **matrix bootstrap** (Han–Hartnoll–Kruthoff, PRL 125 (2020) 041601) has the same shape as your sum-of-squares certificates (H = ½Σa*a + 3S/2). It can only give *lower* bounds, though, so it suits Step 4 and not Step 3 or H1. |
| **H2** (line 16) | **Heat-kernel coefficient a₄ for Yang–Mills in background-field gauge** (Vassilevich, Phys. Rept. 388 (2003) 279, arXiv:hep-th/0306138). Cross-checked against **van den Heuvel**, Nucl. Phys. B488 (1997) 282 (the round S³, t = 1). | KC1 says the electric and magnetic couplings carry *one* log: the speed of light does not run. Locality of a₄ forces the kinetic (q̇²) log and the potential log to share the single coefficient b₀/8π², for every Berger t. |
| **H3** (line 20) | **The Dobrushin–Shlosman one-box criterion C_V** (1985), in Shlosman's restatement (arXiv:2505.24750), plus the averaged Weitz/BBLR form. | Your central-block condition MS has no iteration theorem behind it. Every criterion that actually works averages the influence over the whole box. Switching to the averaged form **MS′** absorbs the (ρ/a)³ boundary count you were worried about. Chatterjee (CMP 385, 2021) shows the worst-case boundary clause is at least as strong as confinement. That fits the physics, and it also tells you how hard this step is. |
| **H4′** (line 23) | **Hollands, renormalized Yang–Mills on curved spacetime**: perturbative, all orders (Rev. Math. Phys. 20 (2008) 1033, arXiv:0705.3340). | ℝ×S³_{R,t} meets all his hypotheses. Every locally covariant counterterm is a polynomial in curvature of dimension 4, so **no counterterm can separate tr F₀ᵢ² from tr Fᵢⱼ²**. c₀ is data about the regulator, not a parameter of the theory. A bonus: his renormalization-group scaling g → μ²g is exactly motion along R. |
| **H6** (line 25) | **The lattice interaction-measure identity** (Boyd et al., Nucl. Phys. B469 (1996) 419, eq. 2.10), with the trace anomaly of **Collins–Duncan–Joglekar** (PRD 16 (1977) 438) as its perturbative counterpart. | The trace anomaly is the derivative along your bundle's scale coordinate. The integrated version follows from H4(a) using only the chain rule and dominated convergence. |
| **H7** (line 24) | **Whitehead's first lemma** (H¹(su(2), V) = 0). The precedent at t = 1 is Dasgupta–Sheikh-Jabbari–Van Raamsdonk (arXiv:hep-th/0205185). | The vacua of the unsourced reduced model, for *any* compact simple group, are orbits labelled by homomorphisms su(2) → 𝔤. Whitehead shows they are non-degenerate (Morse–Bott). The long-root su(2) is your competitor for every group. |

---

## 2. What was proved tonight, and what survived the referee

### Proved and survived (proof notes only, no register change)

- **H8: projected-Gaussian min–max lemma [D].** For t = 3, s = 1, every g > 0 and every α > 0: E_{2,0}(g) ≤ μ₂, the larger root of a 2×2 pencil built on the origin Gaussian and the colour-averaged Gaussian. It rests only on registered lines 9 (Theorem A) and 17 (symmetry). Step 3 is now one finite interval computation.
  - Floating μ₂ is 5.114 to 5.122 against the target 5.2731, a margin of about 0.15, at all five couplings with α = 1. Two independent implementations agree. These are not enclosures yet.
- **H7: Theorem A [D]**, in the reduced model at s = 1, for every compact simple 𝔤, every t > 0 and every g > 0:
  - the vacua are classified by Hom(su(2), 𝔤), and there are finitely many orbits;
  - each orbit is Morse–Bott;
  - the index and zero-point offset are given per spin;
  - the long-root well has index 1 and offset ω_C, and every other well has offset ≥ ω_C.
- **H3: Lemma G and Proposition G0 [P].**
  - **Lemma G.** For the Gaussian free field, the Dobrushin–Shlosman boundary sum equals the sum over the box of the killed-walk exit probability. It is exactly |V| when m = 0 (the criterion fails), and its limit as the lattice spacing goes to zero is below 1 when m > 0. So the criterion is cutoff-uniform at fixed physical size.
  - **Proposition G0.** Any exactly shift-symmetric specification has α ≥ 1.
- **H2: KC1-ζ [PC].** In background-Feynman gauge with a proper-time or ζ regulator, for every t > 0, the one-loop log of homogeneous paths is −(b₀/8π²) log(ΛR)·S₀[A_q]. That is one coefficient for both the kinetic and the potential terms. The referee notes that this is close to a standard fact. What is new (the j = 0 subtraction and the reading for homogeneous paths) is correct but small.
- **H4′: C₀ [PC + D], perturbative, Lorentzian.** In every locally covariant scheme on ℝ×S³_{R,t}, c₀ ≡ 1 for all R and t. The only marginal freedom is g. One caveat: the Yang–Mills case of Hollands' uniqueness theorem rests on his "completely analogous" remark, which he uses himself in §4.4.
- **H6: Theorem 1 [P].** At each fixed (n, L), the infinite-time Wilson measure exists and β ↦ ⟨F⟩ is C¹, with the exact plaquette-insertion formula. This needs the referee's repair to Lemma 5, which moves the temporal links of z_k into depth 0.

### Conditional

- **H7: Corollary B [Cd on (S)].** At small g, for every group, charge 0 is the lowest excitation. The limit of E_{1,0} − E_{0,0} is min(ω_C, 2√t). (S) is the orbit-well semiclassical input.
- **H6: integrated trace identity [Cd].** It is conditional on (L1), a C¹ coupling path, and (L2), torus H4(a) for an interval of base spacings. **Neither is a register line.** The referee says to tag it [Cd], not [D]. It is also mostly a consistency check: it says nothing yet about the local field Θ.
- **H2: KC1 as worded in H2 §6 [Cd on R3, R4].** R3 is the move from the Coulomb chart to background gauge. R4 is the move to your spatial cutoff. Both are open, and R3 is a real risk, because a_n^tot is gauge-independent only on shell.
- **H4′: the regulator clause [Cd].** It needs a Reisz-type theorem for your regulator, plus an extra regulator-locality hypothesis before you can claim R-independence.

### Failed or corrected (all caught before they reached you)

- **H6:** the research note had the **sign** of the prefactor wrong. The correct definition is c_n = −s β_n′ > 0. The referee confirmed this twice.
- **H4′:** route step 4 ("no local timelike vector") was **wrong**. On this geometry the Ricci tensor does pick out ∂_τ. What actually excludes the split is polynomiality in curvature plus exact dimension 4. Separately, **your paper's §8 conjecture that c₀ is controlled by the trace anomaly does not hold up**: the anomaly governs the running of g(R), not c₀.
- **H2:** route step (e) failed. R2 links the Born–Oppenheimer mass to the transverse fibre, not to the background-gauge determinant, so it cannot bridge the two gauges.
- **H7:** the research note's limit ω_C holds only for **t ≥ 1/3**. Below that, the origin's breathing mode at 2√t is lower. Your controls at t = 3 are not affected.
- **H8:** three corrections.
  - μ₂ < 5.2731 is *sufficient* for Step 3, not equivalent to it.
  - The suggestion to use α = 4 is **not supported**: an independent run disagrees by about 0.1. Certify at **α = 1**.
  - N_H is not positive, so expect some cancellation to widen the interval enclosures.
- **H3:** the overclaims were cut back. Lemma G does **not** close Sub-conjecture 5.1 as stated. It also does not prove that (5-ver) is "equivalent to" non-perturbative mass generation. It supports the move to MS′, and only in the Gaussian template.
- **Bootstrap for H1:** this cannot work. Positivity can exclude values but cannot count eigenvalues, so H1 stays on the R2 Lehmann–Goerisch route.

---

## 3. What to attack next (ranked)

1. **Certify H8 Step 3.** Enclose the two SO(3) integrals in intervals at α = 1 and all five couplings, then run the square-root-free test that K − U·M is negative definite. This is minutes of CPU.
   - *Tool:* the min–max lemma from tonight plus a box cover of the rotation ball. At g² ≥ 0.03 the whole ball must be covered, because the cheap tail bound only works at g² = 0.002 and 0.004.
   - Record the certified upper end of μ₂ itself. That gives Step 4 about 0.36 of room instead of 0.21.
2. **H8 Step 4: lower bounds in charges 1 to 6** (|m| = 2 is the tight one).
   - *Tool:* Lehmann–Goerisch with Plum's homotopy (Behnke–Mertins–Plum–Wieners, Proc. R. Soc. A 2000; the Nakao–Plum–Watanabe book), rerun at s = 1.
   - Backup: a sum-of-squares bootstrap with exact rational certificates (Peyrl–Parrilo, TCS 409 (2008)).
3. **H1, the R2 interval Lehmann step.** It uses the same Plum and Lehmann–Goerisch machinery as item 2, so build it once and use it for both.
4. **H7: prove (S) and the uniqueness of the competitor's well.**
   - *Tools:* Helffer–Sjöstrand, *Puits multiples VI* (Ann. IHP 46 (1987) 353). It treats wells that are submanifolds in the uniformly degenerate case, which is exactly your case. It still needs a symmetry reduction plus IMS localisation in the sectors.
   - For uniqueness: Kostant's conjugacy of sl₂-triples, with Collingwood–McGovern Ch. 3 for the step from complex to compact conjugacy. That theorem number is not yet verified.
5. **The kinetic sign on line 15.** In van den Heuvel's scheme this becomes a finite one-loop sum on the Berger spectrum that depends on F. It is a computation, not a proof. It did not run tonight because the CPU was busy.
6. **H2 R3/R4.**
   - *Tools:* Abbott's background-field theorem (Nucl. Phys. B185 (1981) 189) for R3. For R4, Hasenfratz–Hasenfratz (NPB 193, 1981) and Karsch (NPB 205, 1982); I could not read their texts, so none of their numbers are quoted.
   - Hard. R3 is where KC1 could genuinely fail.
7. **H4′ beyond perturbation theory.**
   - *Tools:* Reisz (NPB 318 (1989) 417), for a Reisz-type theorem for your regulator. The Wilson-flow criterion E_ss = E_st (Klassen; Borsányi et al., arXiv:1205.0781), to tune c₀ once in flat space.
   - Lead: Costello (arXiv:0706.1533) might give a Euclidean C₀. I have not checked its hypotheses.
8. **H3: rerun the Dobrushin/Weitz iteration on the room graph** (looks routine; not done). Verifying MS′ at one scale is still the wall. The referee's calibration puts the box needed at about 11 to 12 correlation lengths across, which is deep-infrared territory.
9. **H6, N1 to N3** (tightness of the local field, a correlator version of H2, and infrared decoupling). Long-term.

---

## 4. What would change if you accept (nothing is edited yet)

The register allows only status changes or proof changes inside a line. Each item below is a proof or sub-statement added inside a line. **No top-level tag moves.**

| Register line | What I would add | Line status |
|---|---|---|
| **18** (H8) | Proof note: "[D] projected-Gaussian min–max lemma, E_{2,0} ≤ μ₂ (t = 3, s = 1, all g, α); Step 3 reduced to a finite interval computation at α = 1." | stays [X] |
| **16** (H2) | "KC1-ζ [PC] (Vassilevich, hypotheses matched); KC1 as in H2 §6 [Cd on R3, R4]." | stays [X = H2] |
| **15** | Optional pointer: "the open O(g²) kinetic sign reduces, in van den Heuvel's scheme, to a finite one-loop Berger sum (not computed)." | stays [C, potential part only] |
| **20** (H3) | Change the proof route from MS to **MS′** (averaged DS/BBLR form). Add "Lemma G, Prop. G0 [P] (Gaussian calibration)". | stays [X = H3] |
| **23** (H4′) | Under "two bare parameters": "Perturbatively, on **Lorentzian** ℝ×S³_{R,t}, every locally covariant scheme has c₀ ≡ 1 for all R, t (Hollands 2008 + note, [PC + D]); c₀ is regulator data, needed only for non-covariant regulators such as H2's cutoff [Cd on a Reisz-type theorem + regulator locality]." The two-parameter tuning stays as written, because it is about your non-covariant regulator. | stays [X = H4′] |
| **24** (H7) | "Theorem A (vacua of the unsourced reduced model, every 𝔤, t, g) [D]; Corollary B (charge 0 lowest at small g, s = 1) [Cd on (S)]." Add the note that the limit is min(ω_C, 2√t), so ω_C applies only for t ≥ 1/3. | stays [X = H7] |
| **25** (H6) | "Infinite-time β-derivative at fixed (n, L) [P]; integrated trace identity [Cd on torus-H4(a) for an interval of schemes + (L1)], a consistency check, not evidence about the local field." | stays [X = H6] |
| **11** (H1) | No change. | stays [X = H1] |

**Outside the register:**

- **Paper §8:** replace the labelled conjecture "c₀ is controlled by the trace anomaly" with "c₀ is regulator data fixed by local covariance (perturbative)".
- **Small fixes to tonight's notes before anything is copied:**
  - In the H2 notes, cite line 13 (M = 2gq), not line 8, and change "scalar rescaling" to "anisotropic linear map".
  - In the H8 closure, the target is 274/(30√3) = 5.273132, and the receipt is in §7, not §8.
  - Retitle the H7 closure so it does not read as "H7 closed".
  - Apply the H6 Lemma 5 depth repair.

That's everything. Rest well before 5 if you can. The first job when you're up is item 1, and it should take minutes. (Later: item 1 is done. See "Update, later the same night" at the top.)
