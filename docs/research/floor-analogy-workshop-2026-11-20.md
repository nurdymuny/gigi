# What the floor law is like: an analogy workshop (20 November 2026)

I asked for a physical picture of the floor law, for a sentence or two in the paper and for a long blog post about
how I found it. Three readers worked on it separately: a physicist (physical systems with the same structure), a
writer (everyday images), and a historian (the timeline, from my notes and the commit history). This note keeps what
survived, what to say, and what not to say.

## 1. The lead analogy: a charged, spinning gauge particle in a magnetic field (Nielsen–Olesen)

This one is not only an analogy. The floor law has the same three-term anatomy as the energy of a spin-1 gauge boson
in a constant (chromo)magnetic field, E² = k∥² + (2n + 1)gB − 2gB·s_z (Nielsen–Olesen; for the W boson,
Ambjørn–Olesen):

| floor law, λ_t(j) = κ² + 2(j + t) + (12 − 8t) | spin-1 particle in a magnetic field |
| --- | --- |
| κ² = (2t − 4 − j)²/t, momentum along the long circle | k∥², momentum along the field |
| 2(j + t), the twist in its lowest Landau orbit (X₊b = 0, proved) | (2n + 1)gB at n = 0, the lowest orbit's zero-point cost |
| 12 − 8t = (4 − 2t) Ricci + (8 − 6t) field strength | the spin term −2gB·s_z (plus, here, the curvature of space) |
| the factor 2 in R₀ = Ric − 2[F, ·] | the gyromagnetic ratio 2 of a gauge boson |
| the twist (highest charge: top weight, frame +1, gauge +1) | n = 0, s_z = +1: lowest orbit, spin along the field |
| t_j, where a twist goes free | the critical field where that state goes unstable |
| squashed meron: lowest orbit, but its field strength opposes the twist | a state in the lowest orbit with its spin against the field: never unstable |

The control is what makes it right. On the squashed meron the twist sits in the same lowest Landau orbit, but it is
not the floor, because there the field strength pushes the other way (`gyromagnetic_split.txt`). Symmetry gives the
orbit; the field's alignment with the twist's spin decides the floor. That is exactly the Nielsen–Olesen logic.

Where it stops:
- The Nielsen–Olesen background is at rest (a solution); the Berger field is not. My statement is about the shape of
  the landscape around the Berger field, not about an equilibrium becoming unstable.
- Flat space against a compact sphere: k∥ is free there, while here κ is fixed by j and t, and the spectrum is
  discrete. The coefficients differ (spin −8t against orbit +2t here; −2 against +1 there). The structure matches, not
  the numbers.
- Do not write that the Berger field "condenses W bosons" or says anything about the quantum theory or the mass gap.

**Paper sentence (proposal):** "The three terms of λ_t(j) mirror the Nielsen–Olesen spectrum of a spin-1 gauge
excitation in a magnetic field: a momentum along the field, the zero-point cost of the lowest Landau orbit, and a
spin term, here the Ricci curvature plus the field strength with gyromagnetic factor 2, that the twist's spin aligns
with. On the squashed meron the twist occupies the same orbit, but the field strength opposes it, and the twist is
not the floor."

## 2. Supporting images (for the blog)

- **The buckling ruler (threshold, landscape).** Press a plastic ruler end-on. At one definite force its gentlest bow
  costs nothing, and past it bowing releases energy. Each twist has its own such stretch, t_j. It is honest about
  "landscape, not movie". It is weak on what the cheapest mode looks like.
- **The bell (sectors).** The disturbances sort into families like a bell's tones, finer and higher as j grows. It
  is scaffolding only, but it is the right scaffolding.
- **The compass needle or precessing magnet (alignment).** A circular wobble lined up with the field gets cheaper as
  the field grows, until it costs nothing. This is the best everyday picture of the spin term. It has no orbit.
- **The coiled phone cord (my phrase "a twist becomes a squeeze").** It is the most vivid, and the least honest. A
  real cord moves and snaps into loops, and twist-into-coils (writhe) is a different mechanism with a topological law
  behind it that nothing here proves. Use it only with a guardrail set up early: hold the cord still, and ask only
  what each disturbance would cost.
- Rejected: the skater, water down a drain, the guitar string (tightening raises tones, the wrong direction) and
  wringing a towel.

## 3. The phase-transition framing, made precise

- **Defensible.** At t_j the floor of sector j crosses zero linearly. The number of downhill directions at the
  Berger field (its Morse index) jumps there. The twist is the least-cost disturbance: the minimiser of the second
  variation. This has the same logical form as an ignition criterion (the lowest eigenvalue of the linearisation
  crossing zero), so "the twist is the ignition mode" is fair as a statement about the linear criterion.
- **Not defensible today:**
  - "a phase transition happens";
  - "the field ignites".
  
  The Berger field is not at rest. Under the actual Yang–Mills flow it relaxes to a flat field faster than any twist
  grows, and there is no order parameter and no saturated new state.
- **What would upgrade it:** a background that is at rest where the twist is the floor, and a nonlinear computation
  showing the twist saturating into a new branch. Ambjørn–Olesen is the template.
- **My sentence, sharpened by the control:** not "every field will be disturbed when its twist becomes a squeeze"
  (the squashed meron has the twist and is never disturbed along it), but **"a field yields along its twist when its
  own field strength lines up with the twist's spin."**

## 4. The story, in order (from the historian; times are 19–20 November)

1. **Why I needed an exact answer.** The lattice forces of *Reading the Curve* on the Poincaré sphere had negative
   eigenvalues and nothing exact to check them against. "Have an exact answer to measure against before trusting a
   number." The floor work began as a side check while the variational-force gate stayed open.
2. **19 Nov, 16:29.** The 9 × 9 left-invariant sector, exactly: −14/3 and the rest, all matched by the lattice.
   Then a correction: three growth directions, not five (one value was pure gauge).
3. **17:44. The twist appears.** The circular field is an exact eigenvector in every sector, (j(j+2) − 14)/3. "The
   field's strongest instability and the bottom of every higher sector are one formula." And 168/3, one third of
   the Poincaré sphere's first tone.
4. **18:17–21:31.** The Weitzenböck identity, certified inertia counts, the Floor Theorem, its DOI. Along the way a
   referee pass corrected what 168/3 is: the distance from the growth mode to spin 6, not the spectral gap.
5. **22:39.** Stretch the sphere: the same twist is exact for every t. The floor law, the crossings t_j, and
   t* ≈ 2.698. And: "one third of the first scalar eigenvalue is specific to t = 3, not a rule."
6. **Night of 19–20 Nov.** I tested the phase-transition reading. With the background frozen, each twist goes free
   exactly at t_j. With it moving, the field collapses to flat first. "A landscape law, not yet a dynamical
   transition." The resting relative, the squashed meron, never goes soft.
7. **20 Nov, morning.** The spectral-skeleton note. Then the Landau level (proved within twenty minutes), the charge
   blocks, and the pre-registration, which demoted my own tests before any data. Then Test 2: the squashed meron is
   class N, and the scoped skeleton hypothesis is counted against. The weave was tested and stays a visual
   resemblance.
8. **The answer to the question I was left with.** I had asked what about the Berger field puts the symmetric state
   at the bottom. It is the field strength, aligned with the twist's spin, with gyromagnetic factor 2. Then came the
   proof in every sector for t ∈ [2.75, 10], and the SU(3) test, pre-registered next.

Failures worth telling:
- The Henrici departure test (a prediction that failed, and a reading I retracted).
- The variational force, still not closed.
- The corrections to 168/3.
- The squashed meron.

## 5. Traps for the blog

- **"One third of the Poincaré sphere's eigenvalue, a deep link."** It holds at t = 3 only, the one stretch where
  the twist's lift is the Casimir over t. It is the distance from the growth mode to spin 6, not the spectral gap.
- **"A law across all shapes."** Proved: exact for every t (automatic from symmetry), and the floor in every sector
  for t ∈ [2.75, 10]. It is not the floor below t* ≈ 2.698, nor on the squashed meron. "Law of nature" is my
  conjecture, and I label it as mine.
- **"A phase transition," or anything about the mass gap.** It is a statement about the classical second variation,
  at a field that is not at rest.
