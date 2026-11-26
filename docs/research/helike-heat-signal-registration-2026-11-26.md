# The Helike heat signal: registration (before any code or value)

26 November 2026. **REGISTRATION of an experiment.** No code exists for it yet, and no signal has been computed. It
tests three ideas from my note *One note in every room* (`helike-one-note-in-every-room-2026-11-26.md`) inside the
homogeneous reduction, and only there:
- **The corridor.** Every disturbance in a sector ends on the same lightest note.
- **The double cover.** Across rooms the octave, the overall scale, moves while the tone, the pattern of ratios,
  stays.
- **Gluing must not become part of the reading.** Changing the seams must not move the note.

It also runs one exploratory geometric gap bound (Bakry–Émery). Floating point; it is not a bound and not a proof.

## 1. The signal

The setting is the sourced condensate at (s, t, g²), solved by the positive-cone Ritz solver with exact Kronecker
moments (helike-lab `helike cone`). That gives k = 12 Ritz pairs (Eₙ, ψₙ) in the colour singlet, all charges
together, with ψ₀ = G the ground state.

**Sources (pulses).** Each source is a normalised vector b in the solver's span, built on the same Gauss–Hermite grid
and at the same resolved orders as the reference weights (flux-method amendment §5.2):
- **H (Helike, registered source):** the dressed single quanta of her doublet, b_m = y_m G − ⟨G|y_m G⟩G for m in the
  doublet, used as the 2-dimensional source space. It is the same construction as the §5.3 cross-check.
- **H′:** the bare reference single quanta of the doublet, √(2w)·y_m|0⟩, projected onto the span.
- **H″:** a quadratic source in the same charge: the charge-±2 combinations of y_a y_b G with a, b in the doublet,
  minus their G component.
- **C (competitor, registered source):** the dressed single quantum on the low Q = 0 mode, b_c.
- **C′ and C″:** the bare reference on that mode, and a quadratic Q = 0 source (|y_doublet|² G minus its G
  component).
- **R:** a random source with a fixed seed (20261126), projected onto each sector by the exact charge projector of the
  cone solver, one for each sector.

**The heat signal** of a source space S, with P_S its orthogonal projector, is

    C_S(τ) = Σₙ ‖P_S ψₙ‖² e^{−(Eₙ − E₀)τ} / Σₙ ‖P_S ψₙ‖²,   τ ≥ 0.

The sum is over the k Ritz levels. The **missing weight**, μ_S = 1 − Σₙ ‖P_S ψₙ‖² (on normalised sources), is
reported. Its contribution is bounded by μ_S·e^{−(E_k − E₀)τ} and shown as a band. The **effective note** is
m_S(τ) = −d log C_S/dτ, and the **plateau** m_S(∞) = E_{n*} − E₀, where n* is the lowest level with
‖P_S ψ_{n*}‖² ≥ 10⁻⁶.

## 2. Tests and decision rules (fixed now)

**T1. The corridor (many sources, one note).**
- In each sector, every source whose plateau level carries weight ≥ 10⁻⁶ must reach the **same** plateau level n*.
- A source with weight < 10⁻⁶ on the sector's lowest level is reported as a **selection rule**, with the symmetry
  that zeroes it, not as a failure.
- Verdict per point and sector: CORRIDOR if all weighted sources share n*. Otherwise SPLIT, naming the sources and
  levels.

**T1b. The corridor against the registered identification.** At each point, the Helike-sector corridor level n*_H
and the Q = 0 corridor level n*_C are compared with the levels selected by the registered identification (w̃ argmax,
§5.2).
- Verdict: AGREES or DISAGREES. DISAGREES means the registered identification selects a level that is not the
  sector's lowest weighted note.
- Like §5.5, this is a report-only qualifier. It cannot change a registered grid verdict.

**T2. The tone (the double cover).**
- For each sector, the **octave** is Δ = m(∞).
- The **tone** is the ratio vector rⱼ = (E_{nⱼ} − E₀)/Δ for the first three weighted levels n₁ = n* < n₂ < n₃ of the
  registered source, together with their weight fractions.
- The **normalised curve** is c(x) = C(x/Δ) on x ∈ [0, 6].
- Comparisons run between consecutive rooms along the coupling ladder at fixed (s, t): g² = 0.002 → 0.004 at the
  controls, then 0.03 → 0.05 → 0.07 (t = 3) and 0.03 → 0.05 (t = 4) along each registered s.
- Measured: max |Δrⱼ|, and sup over x of |Δc(x)|.
- Verdict per step: TONE-STABLE if max |Δrⱼ| ≤ 0.02 and sup |Δc| ≤ 0.02. Otherwise TONE-SHIFT, with its size.

**T2 expectation.**
- **Controls:** TONE-STABLE, and close to harmonic ratios. At weak coupling the spectrum is nearly harmonic, so the
  controls are a check of the machinery, not of the idea.
- **Grid couplings:** no prediction is registered. The grid couplings are where the idea is actually tested.

**T3. The octave trend (does the note survive the growing room?).** In this model energies are in units of 1/R, so
the model gap Δ_model is already Δ·R. The trend of Δ_model along increasing g² is reported for both sectors, as
increasing, flat, or decreasing, with differences and their sizes. The ordering of rooms is by g²: at one loop, a
larger g² means a larger radius. For context only, an explicit radius is computed with the one-loop running
RΛ = exp(−1/(2b₀g²)), with b₀ = 11N/(48π²) for SU(N). It carries a stated scheme caveat and enters no verdict.

**Scope statement, fixed now.** T3 is a statement about the homogeneous reduction only. A decreasing Δ_model would
mean the reduction's note is the room's. It would say nothing about four-dimensional Yang–Mills.

**T4. Gluing invariance of the signal** (the §5.5 principle applied to the signal). Seams varied, with what must not
move:
- **Source dressing** (H against H′, C against C′): the plateau level n* (tested in T1).
- **Number of Ritz levels** (k = 8 against k = 12, the same solve truncated): the plateau, exactly, and c(x) within
  the reported missing-weight bands.
- **Truncation** (Mq = 8 against Mq = 10): the plateau value within 0.003, the registered convergence tolerance.
- **Quadrature** (resolved pair against the next pair): the overlaps within 10⁻⁵.

Verdict per point: SIGNAL-INVARIANT if all four hold. Otherwise SIGNAL-SENSITIVE, naming the seam.

**T5. A curvature gap bound (exploratory; no verdict).**
- At each control, sample the ground state on the quadrature grid. Compute the smallest eigenvalue of
  Hess(−2 log ψ₀), in the solver's flat chart coordinates, over the region carrying ground-state mass ≥ 1 − 10⁻⁶. Call
  it κ_BE.
- Report κ_BE beside the true gap, and whether ψ₀ is log-concave on that region. The Bakry–Émery hypotheses ask for
  log-concavity everywhere, which a multi-well landscape may fail. A failure is reported as such.
- Report-only; it neither supports nor refutes anything here. It records whether a dimension-free geometric gap bound
  comes anywhere near the true gap in this model, which is the property that could matter beyond it.

## 3. Order of work and what is not done

- **Order.** First the code (§4) and its tests. Then the controls (g² = 0.002 and 0.004, s = 1.15, t = 3, Mq = 10),
  where T1 must give CORRIDOR and T4 must give SIGNAL-INVARIANT before anything else runs. Then the registered grid
  points, **only after** the registered grid (§5.4) has completed. The unregistered exposure point (g² = 0.04,
  s = 1.12) is run only if the controls pass, and is labelled as unregistered.
- **Not done.** No four-dimensional claim. No change to any registered grid verdict. No new threshold chosen after
  seeing a signal.
- **If a control fails** T1 or T4, the experiment stops there. That outcome is reported as a result about the method.

## 4. Code and tests (before any value)

**Code.**
- The solver gains an option, `--source-overlaps`, that writes, for every Ritz level, the overlaps with each
  registered source space, plus the source norms. It reuses `dressed_overlap_terms`.
- A Python module computes C, m, the plateau, the tone, the bands and the verdicts.

**Tests.**
- A harmonic oscillator with known spectrum and weights: the exact C(τ), plateau and tone are reproduced.
- A planted two-level crossing: T1 reports the correct n*.
- A planted zero-weight source: reported as a selection rule, not a SPLIT.
- T2 and T4 tolerances are enforced as registered.
- The qualifiers cannot change any registered grid function.

**Mutants that must be caught:**
- the plateau taken from the first level instead of the first *weighted* level;
- the missing weight dropped from the bands;
- Δ normalised by room size instead of by the plateau;
- the T2 tolerance loosened;
- T1b allowed to change a grid verdict.

## Amendment 1 (before code): source construction made explicit; T5 deferred

**Why.** The cone solver has no general charge projector to call. Each source is therefore built from monomials of
definite charge times the ground state. The charges are known exactly from the frame:
- the soft doublet (Helike's modes, h₁ and h₂) has charge 2;
- the stiff doublet (s₁, s₂) has charge 1;
- the two singlets have charge 0. The competitor's mode is c, and the stiff singlet is x.

The modes are identified by their frequencies, exactly as in the registered identification. This replaces "projected
by the exact charge projector" for R. It is the same object, built exactly.

**Monomials** (22 in all; each is multiplied by G and has its G component removed):
- h₁, h₂, c, x
- h₁c, h₂c, h₁x, h₂x, s₁², s₂², s₁s₂, h₁², h₂², c², x², cx
- h₁³, h₁h₂², h₁²h₂, h₂³, ch₁², ch₂²

**Charge ±2 sector** (two-dimensional source spaces, the real and imaginary parts of one complex charge-2 vector):
- **H:** (h₁, h₂).
- **H′:** the bare reference doublet, using the solver's reference amplitudes, which are orthonormal.
- **H″:** (s₁² − s₂², 2s₁s₂). This is two stiff quanta: charge 1 + 1 = 2.
- **H‴:** (h₁c, h₂c). A Helike quantum times the competitor mode: charge 2 + 0 = 2.
- **R₂:** the same random real coefficients (seed 20261126) applied to the pairs (h₁, h₂), (h₁c, h₂c), (h₁x, h₂x),
  (s₁² − s₂², 2s₁s₂) and (h₁(h₁² + h₂²), h₂(h₁² + h₂²)), giving one charge-2 doublet.

(The H″ of the registration, "y_a y_b G with a, b in the doublet", had charge 4 or 0, not 2. It is corrected here
before any computation.)

**Charge 0 sector** (one-dimensional sources):
- **C:** c.
- **C′:** the bare reference on c.
- **C″:** h₁² + h₂².
- **R₀:** random coefficients (seed 20261126) on c, x, h₁² + h₂², s₁² + s₂², c², x², cx and c(h₁² + h₂²).

**Missing weight.** For a source space of dimension d, the missing weight is μ = 1 − Σₙ‖P_S ψₙ‖²/d.

**T5.** Deferred to a later build. The heat-signal code does not implement it, and nothing about T5 is claimed until
it is registered in detail and run.

**Solver output.** A new `helike cone-heat` command solves, identifies at the resolved orders, and writes:
- the bare reference amplitudes;
- the monomial overlaps with all 12 Ritz levels, their Gram matrix and their G components;
- everything at both the resolved fine order and the coarse order (for the T4 quadrature seam).

## Amendment 2 (before any value; found by the synthetic tests): T1 made decidable, tone over distinct energies

**Why.** As registered, the T1 corridor was the lowest level reached by *any* source, and a source with zero weight
there was a "selection rule". Under that rule SPLIT can never occur. The registration also asked for "the symmetry that
zeroes it", which the code cannot check by itself. The synthetic tests exposed this before any control was run.

**T1, amended.**
- The corridor of a sector is the plateau of its **registered** source (H or C).
- "Reaching" the corridor means weight ≥ 10⁻⁶ on its degenerate cluster: all levels within 10⁻⁶ in energy of the
  corridor level. That way, Helike's two degenerate partners count together.
- Verdict: CORRIDOR if every other source in the sector reaches the corridor. Otherwise SPLIT. The sources that miss it
  are listed by name, and a symmetry that zeroes them may be named by hand in the report. Naming a symmetry never
  turns a SPLIT into a CORRIDOR.

**T2, amended.** The tone runs over the first three **distinct** weighted energies, so a degenerate pair counts once.
Each ratio's weight fraction sums the weights in its cluster.
