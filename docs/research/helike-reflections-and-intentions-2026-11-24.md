# Zooming out: one operator behind the lessons, and what I intend next

24 November 2026. **REFLECTION AND INTENTIONS. No new computation.** I am writing this to avoid missing the forest
for the trees. It collects what the last two days taught me, says which parts are established, which are
classical physics I re-derived, and which may be mine. It then sets out what I intend to do next, in order.

## Where the work stands

The working paper, *Helike as a Probe of a Quantum Yang–Mills Model* (`docs/papers/helike-quantum-probe/report.pdf`,
updated 24 November), records the survey and the verified sector advance: the vacuum, scalar and tensor residuals
jointly pass at cutoff 20 at both controls. Since then I have run or written:
- the registered dome test (`UNRESOLVED_TRUNCATION`), and its explanation by the outside-origin truncation ratio;
- the second-vacuum analysis and the double cover;
- the razor's contact frame and sweep direction;
- the two-grain model, Phases 1–3 start: the exact potential, van Baal and Hari Dass reproduced, Fig. 2, and the
  Molien budget with a proven basis.

## The thread: everything is a face of the Chern–Simons functional

Yang–Mills on S³ is the gradient structure of the Chern–Simons functional W: the magnetic energy is ½|grad CS|².
Almost every lesson turned out to be a statement about W, its Hessian (the curl operator ∗d), or its critical
points.

| Lesson | Its form in terms of W |
| --- | --- |
| The exact superpotential V = ½\|∇W\|² | Holds exactly when the truncation is the fixed set of a symmetry (symmetric criticality); that is why one grain had it |
| The floor 9/2 | H = ½A†A + ½ΔW: the floor is half the trace of Hess W over the retained modes |
| Two grains, sweep direction, parity | Left and right modes are curl eigenmodes of opposite sign, the two helicities. The oriented CS has ΔW = 0 and protects nothing; sweeping each helicity with its own W restores the floor |
| Vacua, sphalerons, Q = 0, ±½, ±1 | The critical points of W and their Chern–Simons values |
| Local geometry of the second vacuum | Hess V = (Hess W)², with Hess W = I + L_colour·L_space there: colour–space locking, the double cover |
| The razor: contact sets the frame | Measure in the frame Hess W supplies (the magnet rule: frame and widths from one object) |
| Converging tighter | Represent states by the symmetry's own invariants and covariants (Molien counts, proven bases) |
| Cutting too close | A truncation that is not a symmetry fixed set leaks curvature: V = ½\|∇W\|² + C, C = ½\|(1 − P)∗F\|² ≥ 0. The one-grain "gap → 1" was such an artefact |

**Thesis.** The floors, orientations and vacua of Yang–Mills on S³ are spectral facts about the Hessian of
Chern–Simons. The work is cleanest when measured in frames adapted to W's symmetry and critical geometry, and a
truncation is faithful exactly where restriction does not leak curvature.

## Where Helike may fit (a hypothesis, not yet checked)

> **Checked the same day ([curl-spectrum note](helike-curl-spectrum-2026-11-24.md)): refuted in this simple form.**
> The edge paper's operator is the Hessian at the non-flat Berger field, H = curl_A² + Φ. Helike is an exact curl
> eigenvector, but she is not the lowest-helicity mode. Her eigenvalue is λ = μ² − B (helicity squared minus the flux
> along her circle). She is the exact minimiser of the flux term, and the floor window is the balance between the two.
> The Chern–Simons thesis survives in that refined form.

At the trivial connection, the Hessian of the Yang–Mills energy is (curl)². If the operator in *By Enough, But Not
Too Much* is that Hessian, then Helike's classical floor on the Berger sphere is the smallest |curl eigenvalue| among
its modes. Its |m| = 2 plane is then the curl eigenspace that wins inside the stretch window, and "by enough, but
not too much" is a crossing of curl eigenvalues as the stretch t varies. Helike would then be the lowest-helicity
mode of the same operator whose trace sets the quantum floor and whose sign sets the grains. I have not checked this
against the edge paper's exact operator, gauge slice or normalization.

## What is mine and what is not

- **Classical, not mine:**
  - the Chern–Simons gradient structure of Yang–Mills on three-manifolds;
  - the two-family potential and its θ-boundary treatment (van Baal and Hari Dass 1992, whose eqs. (54), (88),
    (92), (101) and Fig. 2 my derivation reproduces);
  - symmetric criticality;
  - the Witten factorization.
- **Possibly mine:**
  - the measurement programme: exact invariant and covariant sector solvers with full-operator residuals,
    registered tests and mutation checks;
  - the floor read as half the trace of Hess W over a truncation, with protection requiring per-helicity
    factorization;
  - the colour–space-locking Hessian identity at the second vacuum;
  - the Helike–curl-spectrum link, if it holds.

  Literature checks come before any claim.

## Intentions, in order

1. **The decisive check.** Compute the curl spectrum on the left- and right-invariant modes of the Berger sphere
   (t, 1, 1), match the edge paper's operator and conventions, and test whether its ordering reproduces Helike's
   floor window t ∈ [2.75, 10]. It is cheap and settles whether the thesis has a classical anchor.
2. **The paper.** Add the second-vacuum, double-cover and two-grain material, with the corrections kept visible:
   the one-grain "gap → 1" artefact and the truncation-dependent offsets.
3. **Phase 3, registered.** Two-grain sector spectra ((0,0), then (2,0) and (1,1)) with physical-spin
   decomposition, parity pairs and a matched comparison against one grain. First decide the scope: van Baal's
   boundary identifications, or the truncated Hamiltonian stated as such. This decides whether the crossing
   survives both grains.
4. **The thought experiment below,** kept as mathematics until a concrete test is written.

## An observation: orientation partners (added later on 24 November; observation, not a claim)

Looking back over two days of corrections, one pattern keeps recurring. Almost every artefact I hit came from
keeping one member of an orientation pair and dropping its partner. Almost every repair restored the partner. I
record it here with its table so it can be checked, not so it can be quoted.

| Where | The orientation pair | Keeping one partner did this | Restoring the pair did this |
| --- | --- | --- | --- |
| Rotations | u and −u in SU(2), one element of SO(3) | Local frames cannot see the sign; the second vacuum looks like plain colour–space locking | The double cover: a Z₂ sign visible only globally |
| Vacua | Chern–Simons 0 and ±1 | Truncating around one vacuum left the dome test `UNRESOLVED_TRUNCATION` | The outside-origin truncation ratio explained the failure |
| Grains | Left and right modes: opposite curl signs, the two helicities | One grain leaked curvature (C ≥ 0); the "gap → 1" was an artefact | Two grains, with van Baal and Hari Dass reproduced |
| Sweep | With W and against W | The oriented Chern–Simons has ΔW = 0 and protects nothing | Sweeping each helicity with its own W restores the floor |
| Sheets | det q > 0 and det q < 0, meeting at rank two | Interior basis has zero trace at the fold; the linear continuation crossed into det G < 0 and the kinetic form went negative | A matched fold (values equal, conormal sum zero), not a wall |
| The cubic | The orientation-odd term g det q | — | It is where Helike lives: λ = μ² − B; her protected channel is the chiral null τ₁ + iτ₂ |

**Why this is only an observation.**
- Orientation is where the Z₂ data of any SO(3)/SU(2) problem lives. Some of these rows may say no more than
  that, and the rows are not independent: the grain and sweep rows are one fact seen twice.
- I chose the rows after the fact.
- It becomes a claim only if it predicts something I have not yet seen. Candidate: in any truncation that keeps
  one orientation partner, the leak is exactly the orientation-odd part of the energy, so the error should vanish
  for observables even under the dropped reflection. That is checkable on the two-grain model before anything is
  written up. Working name: *the orientation principle (conjecture)*.

## Where the theorem stands, and its working name

**Working name: the Helike Ladder Theorem.** Helike is the top rung of a curl ladder whose eigenvalues are
helicity squared minus flux. This name is the working one until I choose the final one.

**Proved:**
- Theorem 1, the flux spectrum.
- Proposition 2, the Cauchy–Riemann identity: the leak equals the gauge component, and the top rung is leak-free.
- The corollary λ = μ² − B, derived independently.
- Theorem 3, the sufficient floor: sector 0 for t ≥ 2√5 − 2 ≈ 2.472, and a band of sectors. The gauge factor is
  required, with a counterexample.
- The protected-channel theorem: an exact formal eigenfunction to all orders.

**Open:**
- The all-sector floor.
- The upper end of the window.
- The quantum flux edge, held on the domain question until the
  [quasimode certification](helike-quasimode-certification-spec-2026-11-24.md) and the convergence run.

The proved part is a classical spectral theorem about Helike. It refines the curl-spectrum idea, and I have not
found it stated in the literature. The decomposition into curl squared plus flux is classical (Bourguignon–Lawson);
the top-rung identity and the window it explains are what may be mine.

## A thought experiment: laminar regions as flatness, seen from outside

An earlier private project of mine treats a complex system as a geometric substrate with curvature patterns. It
looks for regions where the local forces line up into a smooth surface, and for when that smoothness breaks. The
Chern–Simons picture gives that intuition a precise mathematical form. The system there is not Yang–Mills, and
nothing below is a claim about it.

1. **Helicity is Chern–Simons.** For a velocity field u, the fluid helicity ∫u·curl u is the abelian Chern–Simons
   functional. The name of that project and the functional at the centre of this work are the same object.
2. **Smooth means flat.** Critical points of W are exactly the flat connections, F = 0. The energy ½|grad CS|² =
   ½|F|² measures how far the substrate is from smooth. "Laminar" is a basin near a critical point of W. "Turbulence"
   is the region near sphalerons and steep walls, where |F| is large. An exit alarm is |F|, or the leaked part C,
   rising.
3. **Look outside to know the inside.** A flat connection on a region is determined by its holonomies around loops,
   by π₁ of the region: global, outside data. By the non-abelian Stokes theorem, the holonomy around a small loop
   measures the curvature it encloses. So the question "will the inside be smooth?" can be asked of loops around
   it, without predicting any local trajectory. This is the inversion: do not predict the curve, read the
   organising forces that make it flat.
4. **The double cover sees what local frames cannot.** An SO(3) holonomy that returns to the identity can still
   lift to −1 in SU(2). That sign is invisible to every local frame and visible only globally. It is a Z₂ invariant,
   the kind of thing "zooming out" detects.
5. **Rarity of the smooth.** In the sphaleron plane, the basins near critical points occupy a small part of
   configuration space, bounded by steep walls (Fig. 2 of the Phase 2 note). A natural measurable analogue is the
   fraction of a trajectory, or of a Gibbs measure, with |F|² below a threshold.

To make this a test, I would need to:
- define the connection on the substrate and its gauge group;
- define the loops and holonomies;
- set a flatness threshold fixed in advance;
- compare holonomy-based "outside" predictions of local flatness with an inside measurement on held-out data.

That belongs in its own registered protocol, outside this public research log.
