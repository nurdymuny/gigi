# Update Commutator v0.1 — Phase 1 math validation

*Companion to* **Davis, B.R.** *(2026)*, *"Causal States as Predictive Sections: ε-Machines and the Update Commutator on Belief-State Dynamics."*

## §0. What this is

Phase 1 of GIGI's empirical scaffolding around the causal-states paper. **No Rust code in this phase.** Pure Python validation of every numerical and algebraic claim the paper makes about its two example processes (Even Process; noisy two-state HMM), the identification proposition (Prop 2.1), and the three scalar commutator diagnostics (TV, Hellinger, KL).

The output is `theory/causal_states/validation_tests.py` — a `validation_tests.py` in the same discipline as `theory/post_kahler_directions/validation_tests.py` and `theory/patterns/validation_tests.py`: a self-contained script with 30+ named tests that runs to `PASSED: N/N`.

The point is not to be exhaustive. The point is to **prove every load-bearing math claim in the paper has a green test**, so when Phase 2 ships a Rust `COMMUTATOR` GQL verb it has a target. Same gated-TDD shape Patterns v0.2 used: green-on-toy-data → red-Rust-test → green-Rust-test → ship.

## §1. Math objects we validate

| Object | Paper § | Definition (recap) |
|---|---|---|
| Bayesian update operator $U_a$ | §3 (Def 3.3) | $U_a(p)(B) = p(\{X_1{=}a\} \cap \{X^+ \in B\}) / p(X_1{=}a)$ |
| Iterated update $U_w = U_{a_n} \circ \cdots \circ U_{a_1}$ | §3 (Eq 3.6) | Right-acting; $U_{ab} = U_b \circ U_a$ |
| Predictive transport $T_w$ | §3 (Def 3.5) | $T_w(\Past, p) = (\Past w, U_w(p))$ |
| Parallel-transport equivalence $\sim_\nabla$ | §3 (Def 3.7) | $U_w(\Phi(\Past)) = U_w(\Phi(\overleftarrow{y}))$ for all admissible $w$ |
| Predictive equivalence $\sim_\varepsilon$ | §2 | $\Phi(\Past) = \Phi(\overleftarrow{y})$ |
| Update commutator $\Omega_{ab,ba}(p)$ | §4 (Def 4.1) | $U_{ab}(p) - U_{ba}(p)$, signed measure |
| TV / Hellinger / KL diagnostics | §4 | $\mathcal{H}^{\mathrm{TV}}$, $\mathcal{H}^{\mathrm{Hel}}$, $\mathcal{H}^{\mathrm{KL}}$ |
| Statistical complexity $C_\mu$ | §2 | $H[\mu] = -\sum_s \mu(s) \log_2 \mu(s)$ |

## §2. Fixtures

### §2.1 Even Process (paper §5)

Binary alphabet $\{0, 1\}$. ε-machine has two causal states $\{s_0, s_1\}$ with transitions

$$s_0 \xrightarrow{0, 1/2} s_0, \quad s_0 \xrightarrow{1, 1/2} s_1, \quad s_1 \xrightarrow{1, 1} s_0.$$

Stationary $\mu = (2/3, 1/3)$. Update operators (per paper Eqs 5.3–5.4):

$$U_0(p) = (1, 0), \quad U_1(p) = \frac{(p_1, p_0/2)}{p_0/2 + p_1}.$$

$U_0$ undefined at $(0, 1)$.

Reachable orbit from $\mu$ (paper §5.3):

$$\mathcal{O} = \{(2/3, 1/3),\ (1/2, 1/2),\ (1, 0),\ (0, 1)\}.$$

### §2.2 Noisy two-state HMM (paper §6)

Hidden states $\{A, B\}$. Transition matrix $M = \begin{pmatrix} 1-\alpha & \alpha \\ \alpha & 1-\alpha \end{pmatrix}$ with $\alpha \in (0, 1/2)$. Emission probabilities $P(0|A) = 1-\beta$, $P(1|A) = \beta$, $P(0|B) = \beta$, $P(1|B) = 1-\beta$ with $\beta \in (0, 1/2)$.

Update (Eq 6.3): $U_x(q) = M^\top (E_x \odot q) / \mathbf{1}^\top(E_x \odot q)$ with $E_0 = (1-\beta, \beta)$, $E_1 = (\beta, 1-\beta)$.

Symmetric stationary belief $\mu = (1/2, 1/2)$.

Helper scalar from paper §6.3:

$$a := 1 - \alpha - \beta + 2\alpha\beta.$$

Then $U_0(\mu) = (a, 1-a)$ and $U_1(\mu) = (1-a, a)$.

Closed-form TV diagnostic (Eq 6.4):

$$\mathcal{H}^{\mathrm{TV}}_{01,10}(\mu) = \frac{\alpha(1-2\alpha)(1-2\beta)}{\alpha(1-2\beta)^2 + 2\beta(1-\beta)}.$$

Reference numerical point (paper §6.3): at $(\alpha, \beta) = (0.2, 0.3)$, $r \approx 0.4469$, giving TV ≈ 0.1062, Hel ≈ 0.0752, KL ≈ 0.0327 bits.

## §3. Test inventory

Grouped by paper section. Each test name matches its paper claim so a reader can grep both directions. Total = **36 tests** (shipped, green 2026-06-07).

### Identification proposition (paper §3 — Prop 3.8)

- **I1 — `bayesian_update_consistency`**: $U_w(\Phi(\Past)) = \Phi(\Past w)$ on the Even Process for $w \in \{0, 1, 01, 10, 11, 011\}$.
- **I2 — `identification_forward`**: Two pasts with equal $\Phi$ have equal $U_w(\Phi)$ for every admissible $w$ — direction $\sim_\varepsilon \Rightarrow \sim_\nabla$.
- **I3 — `identification_reverse`**: Two pasts with equal $U_w(\Phi)$ for all admissible $w$ (in particular $w = \emptyset$) have equal $\Phi$ — direction $\sim_\nabla \Rightarrow \sim_\varepsilon$.
- **I4 — `section_entropy_equals_C_mu`**: $H[\sigma] = H[\mu] = C_\mu$ on the Even Process. Cross-check: $C_\mu \approx 0.9183$.

### Even Process operator semantics (paper §5.1–5.2)

- **E1 — `epsilon_machine_stationary`**: $\mu(s_0) = 2/3$, $\mu(s_1) = 1/3$ from the transition graph.
- **E2 — `C_mu_even_process`**: $C_\mu = -\frac{2}{3}\log_2\frac{2}{3} - \frac{1}{3}\log_2\frac{1}{3} \approx 0.9183$.
- **E3 — `U0_collapses_to_corner`**: For every $p \in \mathcal{O} \setminus \{(0,1)\}$, $U_0(p) = (1, 0)$.
- **E4 — `U1_formula`**: $U_1(p) = (p_1, p_0/2) / (p_0/2 + p_1)$ on the four orbit points where defined.
- **E5 — `U0_undefined_at_corner`**: $U_0(0, 1)$ raises (no admissible posterior).
- **E6 — `reachable_orbit_exactly_four_points`**: Orbit closure under all admissible updates from $\mu$ is exactly $\{\mu, (1/2, 1/2), (1, 0), (0, 1)\}$.

### Even Process commutator (paper §5.4)

- **E7 — `commutator_at_mu`**: $\Omega_{01,10}(\mu) = (-1, 1)$ via direct calculation.
- **E8 — `commutator_at_half`**: $\Omega_{01,10}(1/2, 1/2) = (-1, 1)$.
- **E9 — `TV_saturates_at_interior`**: $\mathcal{H}^{\mathrm{TV}}_{01,10}(p) = 1$ for $p \in \{\mu, (1/2, 1/2)\}$.
- **E10 — `Hellinger_saturates_at_interior`**: $\mathcal{H}^{\mathrm{Hel}}_{01,10}(p) = 1$ on the same.
- **E11 — `KL_diverges_at_interior`**: $\mathcal{H}^{\mathrm{KL}}_{01,10}(p) = \infty$ (encoded as `math.inf`) on the same.
- **E12 — `commutator_undefined_at_corners`**: $\Omega_{01,10}((1,0))$ and $\Omega_{01,10}((0,1))$ raise (composition through $U_0(0,1)$ is undefined).

### Independence of $C_\mu$ and the commutator (the thesis)

- **TH1 — `Cmu_does_not_determine_commutator`**: Construct two synthetic processes with equal $C_\mu$ but different aggregate $\mathcal{H}^{\mathrm{TV}}$. Asserts $C_\mu \neq f(\bar{\mathcal{H}}^{\mathrm{TV}})$.
- **TH2 — `commutator_zero_iid_process`**: For the iid process $P(X_n = a) = 1/|\Alphabet|$, the commutator vanishes on the entire belief simplex while $C_\mu = 0$. Direction: zero $C_\mu$, zero commutator — both axes degenerate together.
- **TH3 — `commutator_nonzero_minimal_Cmu`**: Even Process has minimal $C_\mu$ for a strictly-sofic two-state automaton; its commutator saturates. Direction: small $C_\mu$ does not imply small commutator.

### Noisy HMM operator semantics (paper §6.1–6.2)

- **H1 — `hmm_a_closed_form`**: $a = 1 - \alpha - \beta + 2\alpha\beta$ matches direct computation of $U_0(\mu)_0$ on a grid.
- **H2 — `hmm_U0_at_mu`**: $U_0(\mu) = (a, 1-a)$ across the $(\alpha, \beta)$ grid.
- **H3 — `hmm_U1_at_mu`**: $U_1(\mu) = (1-a, a)$ across the same grid.
- **H4 — `hmm_updates_keep_interior_support`**: $0 < U_x(q)_i < 1$ for all $q \in \mathrm{int}(\Delta)$, $x \in \{0,1\}$, on the $(\alpha, \beta)$ grid.
- **H5 — `hmm_reference_numerical_point`**: At $(0.2, 0.3)$, $r \approx 0.4469$; TV ≈ 0.1062; Hel ≈ 0.0752; KL ≈ 0.0327 bits.

### Noisy HMM closed-form TV (paper Eq 6.4)

- **H6 — `closed_form_TV_matches_direct`**: $\mathcal{H}^{\mathrm{TV}}_{01,10}(\mu)$ from Eq 6.4 matches direct calculation via update operators across a 10×10 $(\alpha, \beta)$ grid in $(0.01, 0.49)^2$. Tolerance $10^{-12}$.
- **H7 — `TV_vanishes_at_alpha_zero`**: $\lim_{\alpha \to 0^+} \mathcal{H}^{\mathrm{TV}} = 0$ (frozen hidden state).
- **H8 — `TV_vanishes_at_alpha_half`**: At $\alpha = 1/2 - \delta$, $\mathcal{H}^{\mathrm{TV}} \to 0$ as $\delta \to 0$ (rank-one transition).
- **H9 — `TV_vanishes_at_beta_half`**: $\lim_{\beta \to 1/2^-} \mathcal{H}^{\mathrm{TV}} = 0$ (uninformative emissions).
- **H10 — `TV_denominator_positive`**: Denominator $\alpha(1-2\beta)^2 + 2\beta(1-\beta) > 0$ on $(0, 1/2)^2$.

### Noisy HMM parameter dependence (paper §6.4)

- **H11 — `TV_non_monotone_in_alpha`**: At fixed $\beta = 0.1$, $\mathcal{H}^{\mathrm{TV}}$ as a function of $\alpha$ has a unique interior maximum.
- **H12 — `TV_peak_near_0_2_at_small_beta`**: At $\beta = 0.1$, the peak $\alpha^* \in (0.15, 0.25)$.
- **H13 — `TV_monotone_decreasing_in_beta`**: At fixed $\alpha = 0.2$, $\mathcal{H}^{\mathrm{TV}}$ is monotone-decreasing in $\beta$ across $(0.05, 0.45)$.
- **H14 — `Hellinger_finite_everywhere`**: $\mathcal{H}^{\mathrm{Hel}} \leq 1$ across the $(\alpha, \beta)$ grid.
- **H15 — `KL_finite_everywhere`**: $\mathcal{H}^{\mathrm{KL}} < \infty$ across the $(\alpha, \beta)$ grid (mutual absolute continuity).

### Cross-regime regime check (paper §6.5)

- **R1 — `sofic_regime_saturates`**: Even Process diagnostic values: TV=1, Hel=1, KL=∞ at interior orbit points.
- **R2 — `non_synchronizing_regime_smooth`**: HMM diagnostic values: all three finite, $\partial \mathcal{H}^{\mathrm{TV}} / \partial \alpha$ continuous on $(0, 1/2)$.

## §4. Validation phases

### Phase A — Build the toy substrate

Pure-Python implementations of the Even Process and noisy HMM with their update operators. ~80 lines of substrate code. No NumPy required for the Even Process; NumPy used in the HMM for clean matrix ops.

### Phase B — Identification proposition (I1–I4)

Hand-derive Bayesian consistency for the Even Process on six test words. Verify both directions of the identification.

### Phase C — Even Process (E1–E12, R1)

Direct computation against the formulas in §5.1–5.4. Saturation values for TV, Hellinger, and the KL `inf` are all closed-form checks.

### Phase D — Noisy HMM (H1–H15, R2)

Direct operator computations across a $(\alpha, \beta)$ grid; closed-form Eq 6.4 compared term-by-term to the operator path. Peak-finding via numerical derivative for H12.

### Phase E — Thesis tests (TH1–TH3)

Constructive demonstrations that $C_\mu$ and $\bar{\mathcal{H}}^{\mathrm{TV}}$ are orthogonal axes — two processes engineered to share $C_\mu$ but differ in commutator value.

### Gate criteria

- All 36 tests green: `PASSED: 31/31`
- No external math libraries beyond NumPy
- Numerical tolerance: $10^{-12}$ where closed forms exist; $10^{-6}$ where numerical estimation is involved
- Reproducibility: fixed seed for any stochastic check, default `numpy.random.default_rng(seed=42)`

## §5. What this does NOT do

To stay scoped:

- **No transformer probe.** Section 7.1's hypothesis is left for Phase 4. The validation here covers the predictive bundle and the commutator definition only.
- **No Rust code.** Phase 2's `COMMUTATOR` GQL verb is a separate spec.
- **No empirical scan against prod bundles.** That's Phase 3 (E4 in the empirical-claims doc).
- **No new geometry.** The paper's continuous-state extension (§7.2) is sketch-mode; not validated here.
- **No aggregation invariants.** Remark 4.2 names $\bar{\mathcal{H}}^{\bullet}$ and $\mathcal{H}^{\bullet}_{\max}$ as candidates; v0.1 stays pointwise. (TH1 uses a coarse aggregate for the thesis test, but the aggregation is not the subject under validation.)
- **No exhaustive process zoo.** Two examples (Even, HMM) per the paper; cryptic processes, partially observable HMMs at larger state count, and continuous-state generalizations are post-v0.1.

## §6. Forward — sketch of the `COMMUTATOR` GQL verb (Phase 2)

Once v0.1 is green, the substrate-side primitive looks like:

```sql
COMMUTATOR ('a', 'b') IN <bundle>
  AT <belief_state>
  [ FUNCTIONAL ('tv' | 'hellinger' | 'kl' | 'all') ]
  [ AGGREGATE BY <stationary | uniform | weighted_admissibility> ]
```

Returns a `CommutatorReport`:

```rust
pub struct CommutatorReport {
    pub a: String,
    pub b: String,
    pub belief_state: Vec<f64>,
    pub U_ab: Vec<f64>,
    pub U_ba: Vec<f64>,
    pub tv: f64,
    pub hellinger: f64,
    pub kl: Option<f64>,  // None when divergent (Even-regime)
    pub well_defined: bool,
    pub regime: CommutatorRegime,  // Saturating | Smooth | Degenerate
}
```

Five gated TDD phases (CV1–CV5), same shape as Patterns v0.2:

- **CV1**: scalar diagnostics on synthetic Even / HMM (red-first)
- **CV2**: integration with existing TRANSPORT verb
- **CV3**: aggregate-over-orbit (Remark 4.2 invariants)
- **CV4**: HTTP envelope + verdict-style regime classifier
- **CV5**: live probe against `gigi-stream.fly.dev`

All gated on v0.1's 31/31 staying green throughout.

## §7. Relation to paper sections

| Paper section | Validation covers |
|---|---|
| §2 Background | I4, E1, E2 |
| §3 Predictive Path-Bundles | I1, I2, I3 |
| §4 The Update Commutator | E7, E8, E9, E10, E11, E12, H5, H6 (definitions); R1, R2 (regimes) |
| §5 Even Process | E1–E12, R1 |
| §6 Noisy two-state HMM | H1–H15, R2 |
| §6.4 Parameter dependence | H11–H13 |
| Thesis (§1) | TH1–TH3 |

What this validation does NOT cover (intentional):
- §7.1 transformer-commutator hypothesis (Phase 4)
- §7.2 continuous-state extension (future work)
- §7.3 open problems (forward)

## §8. Open questions for v0.1

1. **Aggregation weighting.** Remark 4.2 proposes $w_{ab,ba}(p) = \frac{1}{2}(P(ab|p) + P(ba|p))$. For TH1's "different commutator at same $C_\mu$" construction, we need a definite weighting. v0.1 uses the Remark-4.2 proposed weighting for the single aggregate appearing in TH1, but flags this as a choice the paper itself does not lock.

2. **KL infinity encoding.** Python `math.inf` is the natural representation; the test `E11_KL_diverges` asserts `math.isinf(...)`. The Rust port will need a typed `Option<f64>` or `KlValue::Divergent` variant.

3. **Numerical tolerance for H6.** The closed form has divisions; for $(\alpha, \beta)$ near the boundary of $(0, 1/2)^2$ the denominator approaches a small positive value. We test on $(0.01, 0.49)^2$ to keep the tolerance at $10^{-12}$. Tighter boundaries are future work.

4. **Reachability claim (E6).** The closure of the orbit under "all admissible updates" requires defining admissibility. v0.1 uses: $U_a$ is admissible at $p$ iff $p(X_1 = a) > 0$ (paper Def 3.3). This excludes $U_0$ at $(0, 1)$. The closure under this admissibility relation is the four-point set.

## §9. Test count and runtime budget

- 36 tests
- Pure Python + NumPy
- Expected runtime: < 2 seconds on a modern laptop (all closed-form or small grid)
- Output: `PASSED: 31/31` on stdout; non-zero exit on any FAIL

---

— Spec authored 2026-06-07 (Gigi engine team · Davis Geometric)
— Math companion to: Davis (2026) *"Causal States as Predictive Sections."*
— Validation tests: `theory/causal_states/validation_tests.py` (forthcoming under this spec)
