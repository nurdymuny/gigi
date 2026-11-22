# The phone in the house: what I heard in 2025 and found in 2026 (22 November 2026)

> *You hear it ringing somewhere in the house,*
> *and you walk toward a room before you know the room is right.*

Sunday, 22 November 2026. I started talking about this at about 5 a.m. I had just woken up, and the overnight runs
had landed. Before the morning was over I was crying. This note is the record of that morning. It holds what I felt, what I
imagined, and what I can actually measure, and it keeps them apart, because all three are part of how I do science
and none of them is allowed to impersonate the others.

**How to read this note.** Every bridge between two pieces of my work carries a tag for how strong it is.
- **INCIDENCE**: the same object appears in both pieces, in writing.
- **STRUCTURAL**: the same mathematical structure, checked term by term.
- **RESONANCE**: it looks the same, and the check has not been done.
- **FELT**: I feel it and cannot yet point to it.

Nothing tagged RESONANCE or FELT is claimed. Where something locks into place exactly, I say so, and I say what it
is that locks.

## 1. What happened this morning

The overnight results were all pushed when I woke up.

- **Pre-registration A on the edge** scored 48 points nobody had computed before.
  - Where the neutral competitor (family 4, charge 0) is the rival, the edge polynomial P₂ predicts the edge to
    10⁻¹². That held at 34 of 38 points.
  - It failed at the other 4. Three were a family-1 block arriving first at small stretch with a strong cross field,
    a corner my own competitor map had already shown me. The fourth was the j ≤ 12 cut-off.
  - No family from 13 to 79 undercuts her inside her region (48 of 48), and her region is one interval with no
    holes (48 of 48).
  - Status: COMPUTED. Note: `edge-prereg-A-results-2026-11-22.md`.
- **The edge polynomial is in Lean.** These four theorems are checked on two machines, macOS and Windows, with
  no `sorry` and only the three standard axioms:
  - the factorization −16384·P₁·P₂ of the 473-term determinant numerator;
  - the Berger division P₂ = −32(s−1)(s+1)·Q₁₆;
  - the root of Q₁₆ between s = 1.65261 and 1.65262;
  - P₂ vanishing on the Berger path for t in [2.73111, 2.73116].
  They sit beside the twelve theorems on her price, on branch `feat/helike-lean`. Lean takes the polynomials from
  the computer-algebra receipt. That they *are* the determinant is not checked in Lean.
- **The edge paper** was drafted: *By Enough, But Not Too Much* (27 pp, `docs/papers/by-enough/`).

Then I asked whether any of this was connected to the original Davis manifold, the 2025 one. That is where the
morning turned.

## 2. What I am feeling (FELT, recorded on purpose)

I am writing this down because feeling is one of the instruments, and an instrument you do not log is an instrument
you cannot calibrate. My trichotomy is: **I feel something, I imagine something, I measure something.** The feeling
comes first. It has a direction, but it does not yet have an address.

I spent a year writing paper after paper, sometimes in what felt like nine different fields, maybe eleven.
- Plasma confinement.
- Drug delivery.
- Viral drift.
- Deepfakes.
- Payments.
- Language models.
- Probability.
- Pyramids.
- Gauge theory.

People asked me why I was writing so many papers, and some of them thought I was not well. I did not have a good
answer. What I had was a feeling: there was something deeper under the Davis manifold, something more, a force I
had found the edge of and could not see the middle of. This morning I saw the middle, and it was her. The twist, the
floor, the first law I ever found, the one I call *her* out of respect for a force that was here before me. It
felt like she had been pulling me through all those fields, room to room, while I kept saying "I know it's here,
I can hear it."

It is like hearing your phone ring somewhere in the house. You cannot see it, so you triangulate with your
imagination: it sounds like the kitchen, no, closer, the couch. Then you get close enough that hearing turns into
seeing, and you pick it up. In 2025 I heard something faint. This morning I picked up the phone in one room.

A feeling can be real and still be misplaced. You can feel anger that is true anger and still be aiming it at the
wrong person, because it really belongs to something that happened when you were five. Placing it somewhere else
does not make it go away. It makes it *placed*. I believe my feeling about this force was true all year. What I did
not have was where she was. This morning gave me one address. It did not give me all of them.

**What I feel, and have not shown:**
- that she runs through every paper I have written;
- that she flows through plasma, electricity, the weather, and anywhere there is data that is correlated with
  itself.

That is a hypothesis at the FELT altitude, and Section 5 says exactly how I will measure it, room by room.

## 3. The papers I re-read this morning

- **The Davis Manifold: Geometry-First Detection with Compositional Error Budgets** (2025, 61 pp).
  - A Davis manifold is a Riemannian state space with:
    - benign identity-preserving path classes P(L);
    - a distortion radius ε(L) between the geodesic and the straight chord;
    - hard and soft configuration margins κ_hard ≤ κ_soft, with an ambiguous band between them where the system
      must **abstain**.
  - A Davis system adds a scalar statistic, a Cantelli bound, and a compositional error budget
    (1 − E_geom)(1 − E_link)(1 − ξ)(1 − ζ).
  - The seed is Proposition 1: **ε(L) = C″K(λ)L**. How badly a flat chord misreads a curved path grows with
    curvature times length.
  - The paper closes on the Davis universality conjecture (U1–U4). U4 asks for a positive geodesic jump wherever
    the configuration changes.
- **The Davis Duality of Approximation and Obstruction** (April 2026, doi:10.5281/zenodo.19428406). A flat
  approximation's error is sandwiched by curvature on both sides:
  c₁‖Ω‖h² ≤ error ≤ c₂‖Ω‖h² (Theorems 5.4 and 5.17). Ω is the field strength F_A for connections.
- **The Lift Unification** (March 2026). Six falsifications of the probability pillars in MIT 6.042:
  - the Lift Λ = P(A∩B)/(P(A)P(B)) governs every failure;
  - capacity C = τ/K (log-likelihood ratio over log prior ratio) decides when evidence overturns a prior;
  - it closes on the Non-Decoupling Theorem (doi:10.5281/zenodo.18754646): any flat model of a curved system
    carries a debt bounded below by the Yang–Mills action ∫‖F‖².
- **My spectral branch** (`theory/spectral_branch.tex`). It recasts the Davis manifold as a spectral object with
  three gaps:
  - λ₁ for the Laplacian;
  - μ₁ for monodromy;
  - **ν₁ for the Yang–Mills Hessian**, on connections modulo gauge.

## 4. What lines up, and how strongly

### 4.1 She lives in a table I wrote before I found her (INCIDENCE; locks)

My spectral branch lists the Yang–Mills Hessian ν₁ as the Davis manifold's third spectral gap, the gauge-theoretic
one. The floor is the lowest eigenvalue of exactly that operator, family by family, on the stretched spheres. On
the Berger spheres with 2.75 ≤ t ≤ 10, the Floor paper certifies that this lowest eigenvalue is hers in every
family (doi:10.5281/zenodo.24412603). So I did not find her somewhere new. I found her in the third column of a
table I had already written. This is written down in both places. It is not an analogy.

### 4.2 She is the cheapest way to change the debt (INCIDENCE; locks)

The Lift Unification ends on the Non-Decoupling Theorem: the debt every flat model owes is the Yang–Mills action.
The Yang–Mills Hessian is the second derivative of that same action, so its floor is the cheapest way to change the
debt. Two consequences:
- **When her price is positive**, every small change costs. At the Berger field in the certified band she is that
  lowest cost.
- **When her price is negative**, there is a direction that pays the debt down. At stretch t = 3 her price in
  family 0 is −14/3 (checked in Lean as `floorLaw_three_sector0`).

The functional is identical in both papers. What is *not* yet shown is that the Lift paper's probabilistic reading
of covariance as curvature is the same F; that reading is the paper's interpretive close, not a theorem.

### 4.3 The floor certificate is a Davis system (STRUCTURAL candidate; the method locks)

This is the one that made me cry. In 2025 I designed detection with margins, distortion, abstention, and a composed
budget. In 2026 the floor theorem is built exactly that way, without my planning it:

| Davis manifold (2025) | the floor (2026) |
| --- | --- |
| configuration regions; the label changes at a margin | her region; the floor changes hands at the edge |
| scalar statistic S | her margin: the rival's price minus hers |
| distortion ε(L*) that eats the margin | the width of each certified interval box |
| non-vacuity κ_soft − 2Rε(L*) > 0 | every certified box keeps a positive margin; the smallest is 1.2 × 10⁻⁶ (family 1) |
| the ambiguous band, where you **must abstain** | near the edge the certificate does not certify; Gershgorin needs very large families there, so the box D stops at x = −1.5 |
| compositional error budget | Theorem 7.3: 1,640,935 interval boxes for families below 80, plus Gershgorin for families 80 and up |

**What locks** is the *shape of the guarantee*: a margin, an enclosure that eats it, a refusal to certify where the
enclosure wins, and a budget composed of pieces. **What is still RESONANCE** is whether there is a single theorem of
which both are instances. I have not written that theorem.

### 4.4 The mammogram has her skeleton (STRUCTURAL at the level of the skeleton; locks exactly)

Read each hypothesis's price as −ln of its probability, so the most likely hypothesis is the cheapest one, the
floor. In the Lift paper's mammogram (1% base rate, 90% sensitivity, 10% false positives):

| | price before the test | price after | how far the test moved it |
| --- | --- | --- | --- |
| Sick | 4.605 | 2.485 | 2.120 (Lift 8.33) |
| Healthy | 0.010 | 0.087 | 0.077 (Lift 0.926) |

- The evidence is the belt.
- *Sick* plays her role: coupled to the signal, it moves hard.
- *Healthy* plays the stranger's: nearly blind to the signal, it moves about 27 times less.
- Healthy keeps the floor because the capacity C = τ/K = ln 9 / ln 99 = 0.478 is below 1.
- **The floor changes hands exactly at C = 1, where the two prices meet.** C = 1 is her edge, written in
  probability.

That is exact arithmetic, and it is the same skeleton as the edge of her region: two levels, one carried by a
signal and one standing beside it, and a floor that changes hands where they meet.

**But I have to be honest about what this is.** That skeleton is generic. It appears wherever two options compete
and only one of them responds to a signal. The genericity is not a weakness of my feeling, though. It is the first
real reason my feeling could be right: if she *is* that skeleton, then seeing her in every field I touched is what
you would expect, not something mystical. What makes the HELIKE twist *her* in particular is her **anatomy**,
proved exactly and checked in Lean:

  λ = κ² + (2j + t + y²) + (Ric⊥ − 2B)

That is momentum squared, plus the cheapest possible cost across the circle, plus curvature minus twice the field.
The mammogram has the skeleton. It does not show the anatomy.

### 4.5 One line from 2025 to her (RESONANCE, with one caution I will not drop)

1. ε(L) = C″K L (2025): flat chords fail at a rate set by curvature.
2. The Davis Duality (2026) makes it two-sided: flat failure is sandwiched by curvature, and its subtitle is *why
   the vacuum has mass*.
3. Non-Decoupling: the flat model's debt is the Yang–Mills action.
4. The floor (2026): the cheapest change to that debt, its exact anatomy, and where it changes hands.
5. *Three Ways Across a Turn: Chord, Arc and Tangent* (doi:10.5281/zenodo.24375338) is the first-order version of
   the same question the 2025 paper's Panel A drew: chord against geodesic.

**The caution.** The floor work taught me that curvature alone does *not* decide where she holds. Tests 4 and 5
failed for a kinetic reason, the field along her circle running the wrong way, not a magnetic one. So the line is
"flat models fail, and the failure has a structure". It is not "curvature decides everything". I am keeping both
halves.

### 4.6 The clean edge and the positive jump (RESONANCE)

U4 in the 2025 conjecture asks that a change of configuration show up as a positive geodesic jump: a clean change,
not a blur. Her edge is clean for a precise reason. She carries charge and the stranger carries none, so their
levels cross instead of repelling (von Neumann and Wigner, 1929). The shape is the same, a crisp boundary where
identity changes, but the mechanism is different. So this stays at RESONANCE.

### 4.7 Things I am deliberately not claiming

- **Mossel's factor of 2 and the double cover.** The Lift paper leaves this as an open question, and so do I. Her
  floor having multiplicity two in Theorem 7.3 is a coincidence of the number 2 until someone shows it is more.
- **That the Lift paper's covariance is the field strength F.** It is an interpretation in that paper, not a
  theorem.
- **Anything about the mass gap.** This note is about one operator's floor and the shape of my own work. It does
  not say I solved anything about Yang–Mills existence or the mass gap.

**A notation clash to fix.** In the 2025 paper κ_hard and κ_soft are margins. In HELIKE, κ is her net turning. They
are different objects.

## 5. How I will measure the feeling: the ledger

The way to honor a feeling is to search room by room and write down what you find, dark rooms included. Every
paper gets one row, with these columns:

| column | question |
| --- | --- |
| **Skeleton** | Is there a carried level, a nearly blind rival, and a floor that changes hands where they meet? |
| **Anatomy** | Does her exact split appear: momentum squared, plus a lowest-cost transverse term, plus curvature against spin? |
| **Altitude** | INCIDENCE, STRUCTURAL, RESONANCE, or dark |
| **What would show she is not there** | stated before looking |

**What would break the feeling as I state it.**
- A paper where the skeleton is present but the floor changes hands by an *avoided* crossing, soft rather than
  sharp. That is the skeleton without her.
- A field where the carried level and the blind rival swap roles: the signal moves the rival and leaves the leader
  alone.

A dark row does not make the feeling wrong. In my own words from this morning, it might mean the feeling was
pointing at a different room.

**The order:**
1. Davis Manifold, Davis Duality, Lift Unification and Floor: started in this note.
2. CHIHIRO: plasma confinement is already a spectral floor problem (λ₁ ≥ h²/4), so it is the first room I have not
   searched.
3. The Field Equations of Semantic Coherence.
4. The rest of the twelve branches.

## 6. Ledger, this morning

| bridge | tag | status |
| --- | --- | --- |
| ν₁ (the Yang–Mills Hessian) is a Davis-manifold spectral gap; the floor is its lowest eigenvalue | INCIDENCE | documented in my spectral branch and the Floor paper |
| the Non-Decoupling debt is the Yang–Mills action; the floor is the floor of its Hessian | INCIDENCE | documented |
| the floor certificate has the shape of a Davis system | STRUCTURAL candidate | the shape locks; no unifying theorem yet |
| mammogram: the floor changes hands at C = 1, where a carried level meets a blind one | STRUCTURAL (generic skeleton) | exact arithmetic; the anatomy is not shown |
| ε = C″KL → Duality → Non-Decoupling → floor | RESONANCE | with the kinetic caution |
| U4's positive jump ↔ the true crossing | RESONANCE | different mechanisms |
| Mossel's 2 ↔ double cover ↔ multiplicity two | not claimed | open |
| she runs through every paper; she flows through any self-correlated data | FELT | the ledger in Section 5 is the measurement |

## In my words

I am not embarrassed that I cried. I spent a year being told I was writing too much, and this morning my own 2025
equations told me what I had been writing toward. The feeling was never the problem. It just did not have an
address yet.

Now one address is written down, with its tags. The rest I will find the way I found this one: listen, imagine,
walk toward the sound, and only call it found when I can hold it in my hand.

*Sources read this morning:*
- *The Davis Manifold* (2025), Downloads, the "-2" PDF.
- *The Lift Unification*, `lift_unification (1).tex`.
- `theory/spectral_branch.tex`.
- *Davis Duality* (doi:10.5281/zenodo.19428406).
- `exact-edge-2026-11-21.md`.
- `what-she-is-a-law-of-2026-11-21.md`.
- `edge-prereg-A-results-2026-11-22.md`.
- The HELIKE Lean project (`docs/papers/berger-spectrum/lean`, branch `feat/helike-lean`).
