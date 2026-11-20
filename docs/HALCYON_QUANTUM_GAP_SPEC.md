# Halcyon in the engine: the quantum gap as gigi verbs, and the hunt for a uniform bound (spec)

2026-11-19. Written before any code for it exists. Branch `feat/ym-engine`.

Builds on:

- `docs/YANG_MILLS_ENGINE_SPEC.md` (the classical verbs, YM1–YM7, all passed);
- *Reading the Curve*, Theorem 1 (the exact gap of the classical linearised operator on the Poincaré
  sphere);
- Halcyon's quantum work as consolidated in `davis-wilson-lattice/docs/theory/clay7_gigi_math.md`, §1.

**Scope, stated once.** This is lattice quantum SU(2) (and SU(3)) at finite spacing and volume.
Its purpose is twofold: (1) move the gap *measurement* into the engine, where Halcyon's own audit
says it is not yet; (2) collect the data that would show whether a lower bound on the gap is
uniform in the lattice spacing. Such a uniform bound is the missing argument that would reduce the
continuum problem to finitely many checks. Nothing here proves a mass gap, and no verb, document or
response built from this spec says it does. The lattice gap itself has been measured since the
1980s; what is new would be (1) and, if it appears, the structure in (2).

## What already exists (not repeated here)

From `clay7_gigi_math.md` §1. I have not rerun these numbers; they are cited as the receipts state
them.

- **2D exact layer.** Heat-kernel SU(2): m_gap = 3β/8 per unit area (Migdal–Witten), as a stored
  column. Measured plaquette against the exact one: Δ_P ~ 10⁻³–10⁻², over β ∈ [0.5, 3].
- **4D Monte Carlo channels (client-side fits).**
  - **M1:** plaquette correlator, effective-mass plateau. SU(2) β = 2.3 at L = 6/8/12/16 gives
    2.24/2.05/1.98/1.95, extrapolating to m_∞ = 1.900 ± 0.018. SU(3) β = 6.0 gives
    2.17/1.97/1.78.
  - **M3:** Creutz-ratio string tension.
  - **Wilson-flow scale:** t₀/a² = 2.466 ± 0.054.
  - **Null control:** 4D U(1), no gap.
  - **M2:** variational glueball, five sentinel failures.
- **The audit's caveat:** gigi does not compute the 4D gap. The 3β/8 is a stored closed form, and the
  plateau fits are client-side. gigi's load-bearing role is CHERN_CLASS → SPECTRAL_GAUGE plus the
  bundle.

## The plan, in phases

### Q1: the correlator and the effective mass become verbs

- **`CORRELATOR field ON observable SEPARATION t …`** returns C(t) = ⟨O(0)O(t)⟩ − ⟨O⟩² with
  binned-jackknife errors. The first observable is the plaquette (Halcyon's M1), from the stored
  heat-bath configurations.
- **`EFFECTIVE_MASS correlator [WINDOW t1..t2]`** returns the cosh effective mass and the plateau
  value with its error. The plateau criterion is fixed in the verb, not chosen per call:
  - successive m_eff agree within 1σ over at least three consecutive t;
  - χ²/dof ≤ 2 for the constant fit.
- **Refusals:** no plateau by that criterion is a refusal that names the failing t. It never falls
  back to a best-guess fit. M2's sentinel rows are the model for this.

**Gates.**

- **Q1-a, parity.** On Halcyon's stored SU(2) β = 2.3 configurations, the engine reproduces M1's
  per-L plateau values within their stated errors: 2.24, 2.05, 1.98, 1.95.
- **Q1-b, null control.** On the 4D U(1) configurations in the same path, EFFECTIVE_MASS refuses or
  returns a value consistent with zero. It must not report a gap.

### Q2: an exact quantum answer to measure against

The 2D heat-kernel theory has an exact gap: the Casimir of the lowest non-trivial representation,
m = 3β/8 per unit area for SU(2). It plays the part BW05's exact velocity plays for the force.

- **Q2, gate.** The engine's EFFECTIVE_MASS, run on 2D heat-kernel Monte-Carlo configurations at
  β ∈ {0.5, 1, 2, 3}, recovers 3β/8 within its error bar at every β.
- **Mechanism removal.** On Wilson-action configurations at the same β, the measured gap must
  differ from 3β/8 at finite β (Halcyon's own contrast: β = 1 gives m = 0.375 against σ ≈ 1.43).
  If it does not, the gate cannot tell the actions apart.

This is the first *quantum* oracle in the engine. The classical oracle (Theorem 1's exact spectrum)
stays the control for the classical operator only. It says nothing about the quantum gap, and this
spec never treats it as if it did.

### Q3: scaling, measured in the engine

The question a uniform bound must answer: as β grows and the lattice spacing a shrinks, does the
gap in lattice units, m·a, shrink exactly like a physical scale? Equivalently, does m/√σ approach a
constant?

- **Measured:** m·a (Q1) and √σ·a (M3 as a verb, by the same refusal discipline). SU(2) at β in a
  window of at least four values, each at two volumes with mL ≥ 4 or so (the window is chosen in the
  run spec, from Halcyon's existing β range).
- **Prediction, fixed now:** m/√σ is constant within errors across the window. It is compared with
  the published continuum ratio for SU(2). The exact citation and table value go into the run spec
  before any data is read, not after.
- **Falsifier:** a monotone drift of m/√σ larger than its errors across the window means the window is
  not in scaling, and no structural conclusion is drawn from it.

### Q4: the hunt (exploratory, labelled as such)

This is where the method that gave Theorem 1 is pointed at the quantum problem.

- **The analogue of the Weitzenböck step.** In Theorem 1 an argument handled every sector beyond
  j = 30 and the computer handled four. The quantum analogue would be an inequality: the
  transfer-matrix gap, in physical units, is at least some explicit c > 0 for every spacing below
  some a₀. Then only finitely many spacings above a₀ are left for the computer.
- **What the engine can contribute:**
  1. The measured gap and its error at every spacing it can reach.
  2. Candidate forms of a lower bound, tested against all of it: for example, a bound in terms of
     σ, or of a plaquette or Wilson-loop quantity that is itself provably bounded.
  3. Where a candidate *fails*, the spacing and volume where it fails.

  A candidate that survives every measured point is a conjecture worth trying to prove. Nothing
  more.
- **Gate (procedural, not a result):** every candidate inequality is written into the run's
  specification before the data it is tested on is read. Candidates suggested by the data are tested
  only on new data.

## Order of work

1. Q1 (the verbs and their parity with M1).
2. Q2 (the exact 2D oracle).
3. Q3 (scaling in the engine).
4. Q4, only after Q1–Q3 pass.

Each phase gets its run spec, its tests before its code, and a results document.

## Guardrails

- Language as in the YM engine spec. The words "mass gap" appear only in the problem's name and in
  citations. Every result is "the lattice gap at spacing a and volume L".
- Bee's engine on :3147 is never touched. Monte-Carlo runs use their own processes and data
  directories.
- Every number cited from Halcyon's receipts is re-derived by the engine (Q1-a) before it is used
  as a reference.
- Responsible disclosure is not relevant here, since nothing in this programme touches
  cryptography. It is recorded so the question is closed.

## Not in this spec

- Four-dimensional continuum constructions.
- Any Osterwalder–Schrader or Wightman axioms.
- SU(3) beyond the Q1 parity check.
- The variational glueball channel (M2) beyond keeping its sentinel discipline.
