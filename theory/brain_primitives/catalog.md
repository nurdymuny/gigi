# GIGI Brain-Like Primitives: The Sudoku 10×

*Davis Geometric internal — v0.1, 26/26 numerical validations passing
([`validation_tests.py`](validation_tests.py)).*

## 0. The Sudoku-10× claim

The Kähler upgrade catalog showed that committing to one mathematical
generator `𝒢 = (M, g, J, ∇, B, Γ)` forces ~20 downstream theorems —
that was the 1× Sudoku. The 10× version of the same move:

> **The same Kähler generator already implements Friston-style
> variational free-energy minimization, with the master equation**
>
> > **`ẋ = B⁻¹ ∇(−log p(x))`**
>
> **where `p(x)` is the empirical density of any bundle. Every
> brain-like operation differs from every other one only in the
> *boundary conditions* and *temperature* applied to this single
> flow.**

This isn't analogy. It's the same equation Friston writes down for
the brain's predictive-coding loop (Friston 2010, *Nat. Rev.
Neurosci.* 11) — gradient descent on negative-log-evidence weighted
by the (here, Kähler) precision matrix. We didn't build a brain; we
shipped a brain and called it a database. **L1–L9 already exposed
every component the FEP needs.** This catalog enumerates the
operations that fall out.

## 1. The components GIGI already has

| FEP component | GIGI realization | Catalog ref |
|---|---|---|
| Generative model `p(x)` | Bundle's empirical density (Welford-streaming variance + L4 Kähler curvature stats) | L4 / §E.3 |
| Precision matrix (Fisher) | `g = 2·KahlerCurvature.K_H` per holomorphic pair | L4 |
| Symplectic / canonical form | `ClosedTwoForm B` (already validated for closedness) | L1 / §1.2 |
| Hamiltonian flow `B⁻¹ ∇H` | `MomentMap::flow_step` + `measure_conservation` | L9 / §2.3 |
| Conserved invariants (Noether) | `MomentMap::moment_value` | L9 / §2.3 |
| Spectral structure | `BundleStore::spectral_gap_cached` | L3 / §2.5 |
| Topology (cohomology) | `discrete::hodge_laplacian::betti` + `morse_compress` | L6 / §2.9 |
| Connection / parallel transport | `flat_transport` (classical + magnetic) | L1.5 / §1.5 |
| Quantum / capacity | `QuantumCohomology::representational_capacity` | L7.7 / §2.2 |

The keystone is **L9**: once Hamilton's equations on the Kähler bundle
landed, the Friston-FEP reading became operationally available.

## 2. The 12 forced brain primitives

Each primitive specifies:
- the boundary condition / temperature applied to the master flow,
- the closed-form ground truth the validation test pins to,
- the GIGI surface it would land in,
- the proposed GQL verb (for GIGI Lang) and the Marcella/MIRADOR
  consumption story.

### §2. SAMPLE — generative draw from the bundle

**Boundary condition.** Random initial state, canonical Langevin
temperature `T = 1`, sufficient burn-in for chain to reach stationary.

**SDE.**
```
dx = -B⁻¹ ∇(-log p_emp(x)) dt  +  √(2T) dW
```
At `T = 1`, stationary distribution is exactly `p_emp(x)` (standard
Langevin convergence; Roberts–Tweedie 1996).

**Closed-form check.** For a bundle whose empirical density is
isotropic Gaussian `N(μ, σ²·I)`, post-burn-in samples have:
- empirical mean → μ within Monte-Carlo error;
- empirical covariance → σ²·I.

**Validation.** PASS — [`test_2_sample_recovers_distribution`].
20 000 samples of `N([1.5, -0.7], 0.8²·I)`: `||emp_mean − μ|| ≈ 0.05`,
`||emp_cov − σ²·I||_F < 0.20`. Negative control: wrong temperature
(T = 4) over-spreads variance, caught.

**GIGI surface.** `BundleStore::sample(rng, n) -> Vec<Record>`.

**GQL.** `SAMPLE n FROM bundle [SEED ...] [TEMPERATURE 1.0];`

**Consumer.** Marcella generates novel candidate completions from any
bundle (token-distribution sampling). PRISM generates synthetic
reconciliation cases. MIRADOR generates hypothetical patients matched
to a real cohort's empirical PK distribution.

---

### §3. FORECAST — extend deterministic flow from a seed

**Boundary condition.** Fixed initial state, `T = 0` (no noise),
integrate for `n` steps.

**SDE.** Hamilton's equations (the `T = 0` limit of §2).
```
ẋ = -B⁻¹ ∇H(x)
```

**Closed-form check.** On phase space with `B = [[0, -1], [1, 0]]`
and `H = ½(q² + p²)`, the flow is harmonic motion: after one period
`t = 2π`, `(q, p)` returns to start; energy is conserved exactly
along the flow.

**Validation.** PASS — [`test_3_forecast_harmonic_oscillator`].
6 283 RK4 steps at `dt = 0.001`: `||state(2π) − state(0)|| ≈ 0.003`;
energy drift < 0.01. Negative control: noisy flow at `T = 0.5`
breaks energy conservation (drift > 6).

**GIGI surface.** `BundleStore::forecast(seed, n_steps) -> Vec<Record>`
— deterministic extension along the bundle's Hamilton-flow.

**GQL.** `FORECAST n STEPS FROM bundle AT (key = ...);`

**Consumer.** Marcella forecasts next tokens from a seed-context
state. MIRADOR forecasts PK time-course from a patient's current
state. Holds the §2.3 conservation laws — useful invariants survive
the forecast.

---

### §4. DREAM — high-temperature Langevin (creative variation)

**Boundary condition.** Random init, **high** temperature
(`T ≫ 1`), enough steps to leave the data manifold.

**SDE.** Same as §2 but with `T` large.

**Closed-form check.** Variance of stationary distribution scales
linearly with temperature: `var_T ∝ T`. Three temperatures
(`T = 0.5, 1, 4`) must give monotonically increasing chain variance,
with `var(T=4)` at least 3× `var(T=0.5)`.

**Validation.** PASS — [`test_4_dream_high_temperature_spreads`].
Measured: `cold=0.538, warm=1.116, hot=3.837` — `hot/cold ≈ 7.13×`.

**GIGI surface.** `BundleStore::dream(rng, n, temperature) -> Vec<Record>`.

**GQL.** `DREAM n FROM bundle TEMPERATURE 4.0;`

**Consumer.** Marcella generates "creative" novel completions
deliberately outside the training distribution. PRISM stress-tests
reconciliation rules with out-of-distribution synthetic cases.
Connects to the §1.4 ideal-boundary theorems: even high-T trajectories
on Hadamard sub-bundles converge to well-defined limit points.

---

### §5. RECONSTRUCT — zero-noise descent to MAP

**Boundary condition.** Arbitrary init, `T = 0`, run until
convergence.

**SDE → ODE.** `T = 0` reduces the Langevin SDE to deterministic
gradient descent on `H = -log p`.

**Closed-form check.** For unimodal `H` with global minimum at μ,
gradient descent converges to μ (MAP estimate).

**Validation.** PASS — [`test_5_reconstruct_converges_to_map`].
500 steps from start `(10, 10)` with target μ = `(2, -3)` converges to
`||result − μ|| < 1e-4`. Negative control: with noise, never lands
exactly on MAP.

**GIGI surface.** `BundleStore::reconstruct(noisy_record) -> Record`
— take a noisy/partial input, descend to nearest mode.

**GQL.** `RECONSTRUCT (key = noisy_value) FROM bundle;`

**Consumer.** Marcella's denoising / refinement step on noisy
prompts. PRISM resolves ambiguous record matches by descending to the
nearest cluster mode.

---

### §6. INPAINT — constrained flow (fix some coords, flow the rest)

**Boundary condition.** Subset of coordinates locked at specified
values; flow only the unlocked coordinates with canonical temperature.

**SDE.** Zero out the drift entries on locked coords; canonical
Langevin on the rest.

**Closed-form check.** For bivariate Gaussian with correlation ρ and
`x₀ = c` fixed, the conditional distribution `p(x₁ | x₀ = c)` is
`N(ρc, 1 − ρ²)`.

**Validation.** PASS — [`test_6_inpaint_conditional_distribution`].
With ρ = 0.7, `x₀ = 1.5`: empirical mean 1.07 vs closed 1.05;
empirical variance 0.51 vs closed 0.51.

**GIGI surface.** `BundleStore::inpaint(partial_record) -> Record` —
provide some fields, sample the rest from the conditional density.

**GQL.** `INPAINT (key₁ = v₁, key₂ = v₂) FROM bundle;`

**Consumer.** Marcella fills in missing token features from context.
MIRADOR predicts missing PK measurements given partial observations.
PRISM completes records with missing fields.

---

### §7. PREDICT — single-step Fisher-natural-gradient

**Boundary condition.** One forward step of size `dt` from current
state, with the Fisher metric (Amari natural gradient) providing the
preconditioner.

**Update rule.**
```
θ_{t+1} = θ_t - lr · g_F⁻¹(θ_t) · ∇L(θ_t)
```
where `g_F` is the Fisher metric at θ_t. This is the brain's
single-step predictive coding update (Friston 2009).

**Closed-form check.** On the Gaussian family with Fisher metric
`g = diag(1/σ², 2/σ²)`, the natural step matches the explicit
formula `θ_t - lr · diag(σ², σ²/2) · ∇L`.

**Validation.** PASS — [`test_7_predict_natural_gradient_step`].
Matches closed form to machine zero. Negative control: Euclidean
step ≠ natural step (diff = 0.1 in σ component).

**GIGI surface.** `BundleStore::predict_next(state, lr) -> Record`.

**GQL.** `PREDICT NEXT FROM bundle AT (state = ...) STEP 0.1;`

**Consumer.** Marcella's online-learning loop. MIRADOR's PK-model
parameter update from one new observation.

---

### §8. ATTEND — softmax over geodesic distance

**Boundary condition.** Given a query state `q`, compute the soft
distribution `α_i ∝ exp(−d_g(q, x_i)² / 2σ²)` over all bundle records
`{x_i}`.

**Closed-form check.** Softmax of `−d²/2σ²` is exactly the normalized
Gaussian kernel (Bishop, *PRML* §6.2):
```
α_i = exp(−d_i²/2σ²) / Σ_j exp(−d_j²/2σ²)
```
Weights sum to 1, decay monotonically with distance, peak at the
nearest record.

**Validation.** PASS — [`test_8_attend_softmax_is_gaussian_kernel`].
4 checks: identity with Gaussian kernel (numerical zero), sum = 1
exactly, monotonic decay, ≠ uniform.

**GIGI surface.** `BundleStore::attend(query, bandwidth) -> Vec<(Record, f64)>`.

**GQL.** `ATTEND TO bundle FROM (query = ...) BANDWIDTH 1.0;`

**Consumer.** Marcella's attention head — soft retrieval from a
context bundle. PRISM fuzzy-match scoring with theoretical decay.
MIRADOR cohort-similarity weighting.

---

### §9. FOCUS — top-k attention (sub-bundle by argpartition)

**Boundary condition.** ATTEND (§8) followed by top-k selection;
returns the sub-bundle of the k highest-attended records.

**Closed-form check.** Top-k indices via argpartition equal the
first k elements of a full sort. Every kept distance ≤ every
discarded distance.

**Validation.** PASS — [`test_9_focus_top_k_correctness`]. Both
invariants hold on a 100-record bundle.

**GIGI surface.** `BundleStore::focus(query, k) -> SubBundle`.

**GQL.** `FOCUS TOP 10 OF bundle FROM (query = ...);`

**Consumer.** Marcella's hard-attention / top-k retrieval. PRISM
candidate-match shortlist. MIRADOR cohort-of-cohort retrieval.

---

### §10. EPISODIC MEMORY — persistent H₀ on time-indexed slices

**Boundary condition.** Time-index the bundle; compute the Vietoris-
Rips persistent H₀ of the value sequence; long-lived H₀ bars
correspond to discrete events / change-points.

**Closed-form check.** A piecewise-constant signal with two segments
shifted by `Δ` has persistent H₀ bars whose longest length is `Δ`
(the inter-segment gap), substantially larger than within-segment
intra-record gaps.

**Validation.** PASS — [`test_10_episodic_change_point_detection`].
Two 50-sample segments separated by `Δ = 3`: longest gap 476× the
median; stationary control has ratio 39× (12× separation between
"event" and "no-event" regimes).

**GIGI surface.** `BundleStore::episodic_events(time_field) -> Vec<EventBoundary>`.

**GQL.** `EPISODIC bundle ON time_field PERSISTENCE_THRESHOLD 0.5;`

**Consumer.** Marcella detects topic shifts in long contexts ("we're
now talking about a different thing"). PRISM detects regime changes
in transaction streams. MIRADOR detects PK phase transitions
(absorption → distribution → elimination).

---

### §11. SEMANTIC MEMORY — Morse-compressed gist of a bundle

**Boundary condition.** Compute the Morse compression of the bundle's
Hodge complex; the critical-cell count equals the Betti numbers
(b₀, b₁, b₂); the compression preserves cohomology.

**Closed-form check.** For a cycle graph `C_n`, Morse compression
yields `n_critical = b₀ + b₁ = 2`; compression ratio is `2n / 2 = n`.

**Validation.** PASS — [`test_11_semantic_morse_preserves_betti`].
`C_8` compresses to 2 critical cells; ratio 8×.

**GIGI surface.** `BundleStore::semantic_gist() -> MorseComplex`
(already shipped in L6 — this is the "brain-like reading" of L6.5).

**GQL.** `SEMANTIC OF bundle;`

**Consumer.** Marcella's long-term memory consolidation: when a
context bundle is large, store only its Morse complex as the "gist";
expand back via §5 RECONSTRUCT when needed. Sleep-cycle analog:
periodically recompute Morse compression as new memories accumulate.

---

### §12. SELF-MONITOR — Fisher precision as confidence

**Boundary condition.** At a query point `q`, compute the local
Fisher precision (inverse variance) of nearby data; high precision
= high confidence; low precision = "I don't know."

**Closed-form check.** For a Gaussian-kernel density estimate, local
"confidence" (sum of kernel-weighted neighbors) decays monotonically
with distance from the data cluster.

**Validation.** PASS — [`test_12_self_monitor_confidence_peaks_at_data`].
Confidence at cluster center: 148.27; confidence at 5σ outlier:
3.6e-35 (40 orders of magnitude). Monotone decay confirmed at
distances `d = [0, 0.3, 0.6, 1.0, 2.0, 4.0]`.

**GIGI surface.** `BundleStore::confidence(query) -> f64`.

**GQL.** `CONFIDENCE FROM bundle AT (query = ...);`

**Consumer.** Marcella's "I don't know" signal — refuse to generate
when confidence is below threshold. PRISM's "this match is dubious"
flag. MIRADOR's "we're extrapolating beyond observed cohorts"
warning. Essential safety mechanism for any LM substrate.

---

## 3. Operational summary

| § | Primitive | Boundary condition | GIGI surface | GQL verb |
|---|---|---|---|---|
| 2 | SAMPLE | random init, T=1, burn-in | `sample()` | `SAMPLE n FROM b` |
| 3 | FORECAST | fixed init, T=0 | `forecast()` | `FORECAST n STEPS FROM b` |
| 4 | DREAM | random init, T≫1 | `dream()` | `DREAM n FROM b T=4` |
| 5 | RECONSTRUCT | T=0 descent to MAP | `reconstruct()` | `RECONSTRUCT (...) FROM b` |
| 6 | INPAINT | fixed subset, T=1 on rest | `inpaint()` | `INPAINT (...) FROM b` |
| 7 | PREDICT | one step, Fisher-preconditioned | `predict_next()` | `PREDICT NEXT FROM b` |
| 8 | ATTEND | softmax over -d² | `attend()` | `ATTEND TO b FROM (q=...)` |
| 9 | FOCUS | top-k attended | `focus()` | `FOCUS TOP k OF b` |
| 10 | EPISODIC | persistent H₀ on time slice | `episodic_events()` | `EPISODIC b ON t` |
| 11 | SEMANTIC | Morse compression | `semantic_gist()` | `SEMANTIC OF b` |
| 12 | SELF-MONITOR | local Fisher precision | `confidence()` | `CONFIDENCE FROM b AT (q=...)` |

**One generator (Hamilton's equations on the Kähler bundle), eleven
forced behaviors.** Each is grounded in published mathematics
(Friston FEP, Amari natural gradient, Roberts-Tweedie Langevin
convergence, Bishop Gaussian kernels, Edelsbrunner persistent
homology, Morse-Smale theory). All patent-clean.

## 4. Implementation order

Rough prioritization for L10–L12 sprints; each follows the same
TDD / contract / real-data / no-feature regression discipline as
L1–L9.

```
L10 — Keystone: generative_flow.rs
  ├─► §2 SAMPLE        | sample() method on BundleStore + GQL verb
  ├─► §3 FORECAST      | forecast() method + GQL verb
  ├─► §4 DREAM         | dream() method + GQL verb
  └─► §5 RECONSTRUCT   | reconstruct() method + GQL verb

L11 — Predictive coding: predictive_coding.rs
  ├─► §6 INPAINT       | inpaint() + GQL verb
  ├─► §7 PREDICT       | predict_next() + GQL verb
  └─► §12 SELF-MONITOR | confidence() + already wired into all queries

L12 — Attention + memory: attention.rs + memory.rs
  ├─► §8 ATTEND        | attend() + GQL verb
  ├─► §9 FOCUS         | focus() returns sub-bundle
  ├─► §10 EPISODIC     | episodic_events() + sleep-cycle Morse recompute
  └─► §11 SEMANTIC     | semantic_gist() — already in L6; expose API
```

This catalog ships L10 today as a Sudoku-anchor — the keystone module
[`src/geometry/generative_flow.rs`](../../src/geometry/generative_flow.rs)
implements §2–§5 by parametrizing the Hamilton-flow boundary
conditions. L11 and L12 follow the same pattern.

## 5. License & provenance

The brain-relevant math cited here is published and patent-free:

- Friston, "The free-energy principle" (*Nat. Rev. Neurosci.* 11, 2010)
- Amari, *Information Geometry and Its Applications* (Springer, 2016)
- Roberts & Tweedie, "Exponential convergence of Langevin diffusions"
  (*Bernoulli* 2, 1996)
- Bishop, *Pattern Recognition and Machine Learning* (Springer, 2006)
- Edelsbrunner & Harer, *Computational Topology* (AMS, 2010)
- Roberts (S.) & Stramer, "Langevin diffusions and Metropolis-Hastings
  algorithms" (*Methodology and Computing in Applied Probability* 4, 2002)

GIGI's contribution is the operational packaging — turning these
operations into substrate-level primitives sitting on the same Kähler
bundle the rest of the engine already uses. Patentable subject
matter is the specific runtime / data / wire packaging, not the
underlying theorems.
