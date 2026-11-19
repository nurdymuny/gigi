# Yang–Mills inside GIGI: the strategy

19 November 2026. A strategy note, not a result. Nothing here is measured yet. Markers
**[decide]** are choices I have not made.

## How I want to work on the Millennium problems

I don't start from the problem statement and try to prove it on paper. I put the object
itself inside GIGI, as data, together with the verbs that act on it. I fix in advance what
would count as success and what would count as failure. Then I try constructions until one
converges, and I look at why it did.

That is how the last two days went:
- the two rings came out of trying to read curvature from the data;
- the 1/12 came out of the time stepper;
- the explanation of the stiff directions came out of a prediction that failed.

None of that was planned. It fell out of the object.

Part of mathematics has always worked this way. The Birch and Swinnerton-Dyer conjecture,
itself one of the Millennium problems, came out of computations on elliptic curves on the
EDSAC machine at Cambridge in the early 1960s. The data came first, then the statement, then
the hunt for a proof.

## What the data can and cannot give me

The data gives me the right statement, and a reason to believe it. A proof still has to come
from one of two places.

- **An argument that explains why the construction works.** For the two-ring force I have
  one: the consistency argument in *Reading the Curve*. That is the piece that becomes a
  theorem.
- **A computer-assisted proof with rigorous error bounds,** in the way the four-colour
  theorem and Hales's proof of the Kepler conjecture were done. Those used interval
  arithmetic, not floating point. I have just watched a 10⁻¹⁰ noise floor stop a Newton
  solve, so I know the difference.

## Where the Yang–Mills work actually stands

The Clay problem asks for a *quantum* Yang–Mills theory in four dimensions, constructed
rigorously, with a mass gap. What I have is classical SU(2) flow on a lattice over a
3-manifold: the Poincaré homology sphere, with the Berger metric moving under Ricci flow.

That is a real prerequisite. Lattice Yang–Mills is impossible without a force that
converges, and now I have one: 0.87% off the exact trajectory at n = 3. But it is several
steps from the prize statement.

So I don't write "solves the mass gap" or "proves the mass gap" anywhere. The eigenvalue 4
seen earlier comes from the curved space, not from the field, and must not be read as a gap.

## The plan

1. **Put the SU(2) force and the Cayley flow into the engine.** Today they are research
   scripts. In the engine they have to pass the official flow gate, BW08, and that closes
   the release gate on `RECOVER_CURVATURE` that has been open since the verb shipped. The
   batched force (`docs/RICCI_SU2_BATCHED_FORCE_SPEC.md`) is the step before this: the same
   arithmetic for all cells at once, which is the form an engine verb takes anyway.
2. **Choose one mass-gap observable GIGI can measure, and fix in advance what result would
   falsify it.** The candidate is the lowest excitation of the linearised lattice operator
   above the background, followed as the mesh refines. The first gate has to separate what
   the curved space contributes, such as that eigenvalue 4, from what the field contributes.
   If the observable cannot make that separation, it is the wrong observable.
   **[decide]** which operator: the S2b stiffness K, the mass-whitened Ĝ, or the
   Yang–Mills Hessian itself.
3. **Join Halcyon to this work.** Halcyon is my SU(2) and SU(3) lattice on the buckyball,
   already in GIGI, with the December harvest pipeline running in production. The two-ring
   force gives it a curvature reading that converges. **[decide]** whether the first joint
   run is SU(2) on the Poincaré sphere (where the force was proved out) or on the buckyball
   (where Halcyon lives).
4. **Four dimensions.** Everything so far is three-dimensional. The Clay problem is on ℝ⁴.
   A 4D lattice, or a 3+1 split with the flow time as the fourth direction, is a separate
   spec. It comes after the engine gate, not before.

## Every Millennium problem, the same first question

For each problem, before any building, I ask: which quantity would GIGI measure, and what
result would falsify the approach? Some problems will have a natural curvature observable
and some won't. Finding that out early is part of the method, not a failure of it.

| Problem | The first question to ask inside GIGI |
| --- | --- |
| Yang–Mills mass gap | the plan above |
| Navier–Stokes | which quantity of the flow blows up, and does a verb measure it as the mesh refines |
| Riemann hypothesis | the SPECTRAL_GAUGE work already exists; what does it measure that a classical computation does not |
| Birch and Swinnerton-Dyer | the original data-first problem; which bundle the rank lives in |
| Hodge conjecture | which classes GIGI can represent at all |
| P vs NP | whether a geometric invariant separates the classes, or nothing does |
| Poincaré (solved, Perelman) | the control: Ricci flow is already in GIGI, so the proof's own route can be replayed |

Each row is a question, not a claim. **[decide]** which comes second after Yang–Mills.

## How this one started

From the record:
- The lattice side goes back to Halcyon in June 2026, the SU(2) Yang–Mills lattice on the
  buckyball and the Davis–Wilson programme.
- The Ricci correspondence (Ricci velocity equals the negative Yang–Mills gradient) entered
  this repository on 17 November 2026 as a checkpoint with a handoff note,
  `docs/CLAUDE_RICCI_HANDOFF_2026_11_17.md`.
- *Geometry of Fuel*, 2nd edition, was proofread and replayed the same day.

**[decide]** I remember it starting from *Geometry of Fuel*. Nothing in the record links
the two yet; the handoff note is the first place to check.

## What comes next, in order

1. Finish the batched force (parity gates, then GPU).
2. Write the engine spec for the SU(2) force and the Cayley flow against BW08.
3. Write the mass-gap observable spec, with its falsifier, before any measurement.
