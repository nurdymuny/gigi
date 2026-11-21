# Q4 step 3: hunting a uniform lower bound from data stored in GIGI (draft spec)

2026-11-20. **Draft. ON HOLD (Bee, 2026-11-20)** until Amendment 4 to the Q3 run spec is adopted and scored. Written
while k9 and k10 sample, before any step-3 code or data. Branch `feat/ym-engine`.

> **Why held.** The extension results (`docs/HALCYON_Q3_EXTENSION_RESULTS.md`) left one β passing Q3-a and Q3-v, so
> 3a has one point and 3b has nothing to fit. The estimator is fixed first (`docs/HALCYON_Q3_RUN_SPEC.md`,
> Amendment 4, fresh points k11–k18, optional k19).
>
> **What changes when it resumes:**
> - 3a and 3b read the Amendment-4 receipt and its bundles, not the extension receipt.
> - 3c's fresh seeds start after the last Amendment-4 point (k ≥ 19, or k ≥ 20 if k19 is run), not at k ≥ 11 as
>   written below.
> - The rest of the draft stands as written, with its [Bee] items still open.
Parents:

- `docs/HALCYON_QUANTUM_GAP_SPEC.md` §Q4: the hunt, exploratory;
- `docs/HALCYON_Q4_RUN_SPEC.md`: candidates C1, C2 and K, fixed from published data;
- `docs/HALCYON_Q3_INGEST_SPEC.md`: the ensembles stored as bundles.

The decisions marked **[Bee]** are open and are fixed before the phase that needs them.

**Labelled exploratory.** Nothing in step 3 proves anything. Its best possible outcome is a conjecture of the right
shape, tested on data it was not drawn from. The words "mass gap" appear only in the problem's name. Every result is
"the lattice gap at spacing a and volume L".

## The shape being hunted

The Floor Theorem worked like this: an argument handled every sector beyond a cutoff, and a computer certified the
finitely many sectors below it. The quantum version is an inequality

  a·m₀₊₊ ≥ c · Q(a)  for every spacing a ≤ a₀,

where Q is a quantity that is computable, or itself provably bounded, and c > 0 is explicit. If it held, only the
finitely many spacings above a₀ would be left for the computer. The data's job is to show which Q, which c and which
a₀ the measurements support, and where each candidate fails.

Two caveats are part of the spec:

- The gap of the transfer matrix in the vacuum sector is the lightest state there. In this theory that is the 0⁺⁺
  glueball, but step 3 measures only the 0⁺⁺ channel. A lighter state in another channel would not be seen.
- The flux loop is not a vacuum-sector state, because it winds the volume. It enters only through σ.

## Three phases, each fixed before its data is read

### 3a: the registered candidates on the Q3 points (existing data)

- **What.** C1 (monotone approach from below), C2 (floor 3.158), and the control K that must fail, exactly as
  `HALCYON_Q4_RUN_SPEC.md` fixes them.
- **Which points.** The Q3 points that pass Q3-a and Q3-v after the full extension scoring, now up to four β.
- **Read from GIGI.** The masses come from EFFECTIVE_MASS on the bundles; I-parity makes them identical to the
  probe's.
- **The ratio m₀₊₊/√σ** needs one joint jackknife over both channels, and no verb does that yet. It is computed
  client-side from series read back out of the bundles (query-stream). A parity gate requires it to match the Q3
  receipt to 10⁻¹². A ratio verb is deferred.
- **Gates.** Those of the Q4 run spec, unchanged. If K holds anywhere, everything stops.

### 3b: what the data suggests, on old data only, by a procedure fixed now

This phase looks at the stored data for the quantity that tracks the gap. It looks only through a list fixed here, so
the search space is closed before the data is read.

- **Quantities Q.** Computed per point from the stored bundles:
  1. a√σ from the flux loop (already measured);
  2. the mean plaquette P, and u = P mapped to an effective coupling through the strong-coupling relation (the
     one in K);
  3. the 0⁺⁺ correlator's first ratio C(1)/C(0), a local quantity;
  4. **[Bee]** anything else. It must be listed before 3b runs.
- **Functional forms.** For each Q:
  - am₀₊₊ = c·Q;
  - am₀₊₊ = c·Q^p;
  - am₀₊₊ = c·Q + d·Q²;

  each fitted by weighted least squares over the passing Q3 points.
- **Score (fixed now).**
  - χ²/dof of the fit;
  - then, for the bound form, the smallest margin min_i (am_i − c_lower·Q_i)/σ_i. Here c_lower is the largest c for
    which the bound holds within 2σ at every point.
- **Output.** The best two (Q, form) pairs by χ²/dof, with their constants. These become candidates **D1** and
  **D2**. They are written into this spec as an addendum, with a commit, before any phase-3c data is read.

**What 3b cannot claim.** With at most four spacings and three parameters, a fit can match almost anything. 3b
proposes; it never confirms. Its candidates are tested only in 3c.

### 3c: new data, new measurements, the test

**New measurements.** Added to the sampler's measurement step and stored as new fields of the new bundles:

- **Wilson loops W(R, T)** for R, T ∈ {1, 2, 3, 4}, averaged over position and over the three spatial directions for
  R, on the smeared spatial links at one fixed level (fixed in the run spec). From them come the Creutz ratios
  χ(R, T), a second estimate of σ that does not depend on the flux loop.
- **The Polyakov loop modulus**, per configuration: a check on the confined phase.
- **[Bee] The Wilson-flow scale t₀.** Halcyon measured t₀/a² = 2.466(54). It costs a flow integration per measured
  configuration, so the decision is either every tenth configuration or not at all.

**Tests before any production data.**

- W(1, 1) on unsmeared links equals the mean plaquette to 10⁻¹².
- Every loop is gauge invariant to 10⁻¹².
- At β = 0.5, χ(1, 1) agrees with the strong-coupling value −ln u within 4σ plus the first correction (as Q3-0).
- With flow included: on a cold lattice t₀ is undefined (refused); on a hot one it is positive.

**The run.**

- **Spacings [Bee].** Fresh seeds at the four Q3 β values, plus at least one spacing finer than β = 2.5115, at a
  volume with aL√σ ≥ 3. A finer spacing is where a uniform bound either holds or breaks. Proposed: β = 2.60 at L = 28,
  the largest affordable lattice, timed first.
- **Statistics.** As Q3: 10,000 measurements, blocks of 100, seeds 20261193 + 1000·k for k ≥ 11.
- **Storage.** Ingested as bundles under the step-2 gates (I-load, I-store, I-parity, I-null, I-persist), with the
  new fields included.

**The test, fixed now.**

| gate | what must hold | if it fails |
| --- | --- | --- |
| **H-K** | the control K fails at every new point | stop: the measurement or harness is suspect |
| **H-C** | C1 and C2 at every new point that passes the Q3 calibration and volume gates | name the β and L of every failure |
| **H-D** | D1 and D2, with the constants from 3b, at every new point within 2σ | name every failure. A candidate fails at its first violation |
| **H-tight** | each surviving candidate is not trivial: at the coarsest new point its bound is at least 80% of the measured value | a looser bound is reported as "true but empty", not as a candidate |
| **H-σ** | the two estimates of σ (flux loop, Creutz ratios) agree within 2σ at each new point | report it. Any candidate using σ is reported with both |

**Outcome language (fixed now).** A candidate that passes H-D and H-tight at every new point is reported as:

> "a conjecture of the right shape: a·m₀₊₊ ≥ c·Q for the spacings tested (β = …, a√σ from … to …), tested on data it
> was not drawn from".

The statement names the number of spacings and the volumes. It never says "proves", "establishes" or "the mass gap".

## What step 3 needs from the engine

- **Ingest with new fields.** Reuse the step-2 harness with the new field list.
- **Optionally, a RATIO verb** (the joint jackknife of two EFFECTIVE_MASS channels). It is not required, because the
  client-side path has a parity gate. It is worth adding if Q4 becomes routine.
- **No geometric verb is applied without a stated hypothesis.** Running curvature or holonomy verbs on time series
  "to see what comes out" is the forking-paths problem this spec closes. If Bee wants one in 3b, it goes into the
  quantity list (**[Bee]**, item 4) with what it is expected to show.

## Order of work

1. After the full Q3 extension scoring: run phase 3a.
2. Fix the [Bee] items in 3b, then run 3b and commit the addendum with D1 and D2.
3. Fix the [Bee] items in 3c. Then, in order: the measurement code with its tests, a timing run, the statistics written
   into a run log, production, ingest, and the test.

## Not in this spec

- SU(3). That gets the same structure later, under its own spec.
- Any continuum construction or axioms.
- Any statement about the quantum theory beyond "the lattice gap at the spacings and volumes measured".
