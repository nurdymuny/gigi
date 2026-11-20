# Pre-registration: what Tests 2 and 3 of the spectral-skeleton hypothesis must show (20 November 2026)

Written before any Test 2 or Test 3 data. It tightens §4 of
[`spectral-skeleton-2026-11-20.md`](spectral-skeleton-2026-11-20.md). The only computations behind it are the
design checks in §1, which are made at ε = 0 or are about the perturbation itself. None of them measures a splitting
or a floor away from the Berger family.

## 0. Why the tests as first written cannot fail

The note's Test 3 says: perturb, follow the floor mode, and "if it persists continuously and stays lowest, the
skeleton is structure". Its falsifier is "the floor mode disappearing under arbitrarily small symmetry breaking".

**That cannot happen.**

- In each sector the floor is an isolated eigenvalue of a finite Hermitian matrix.
- Under a small perturbation it moves continuously, and its spectral projection does too (Kato, *Perturbation Theory
  for Linear Operators*, II §1–§5; for a finite matrix, Weyl's inequality and the Davis–Kahan sin θ theorem).
- It stays lowest for every perturbation of 2-norm less than half its gap.
- When sectors mix (perturbations that are not left-invariant), the same holds for perturbations that are relatively
  bounded with bound below one (Kato, V §4, VII §2).

So "persists and stays lowest" passes for every operator with an isolated floor, skeleton or not. A test that cannot
fail tests nothing. Below, each test is restated so that the outcome depends on the skeleton being true.

## 1. What is automatic and therefore not evidence (design checks, recorded)

**Symmetry.** On the Berger sphere (t, 1, 1), rotation about the long axis is an isometry, and it leaves the Berger
field invariant. Its generator is the fibre charge Q = J₀ + R_frame + R_Lie.

**Checks.** Made at t = 3 and 4, for j = 2, 3, 4, in double precision (`scripts/ricci_su2_skeleton_design_checks.py`,
receipt `research/su2-floor-family-20261119/skeleton_design_checks.txt`):

- ‖[Q, H + GG*]‖ ≤ 4.1 × 10⁻¹⁴ and ‖[Q, H]‖ ≤ 3.7 × 10⁻¹⁴;
- G intertwines charges: ‖QG − GQ_ξ‖ ≤ 2.3 × 10⁻¹⁵, with Q_ξ = J₀ + R_Lie on the gauge parameters;
- in the field space of sector j, the maximal charge is j/2 + 2, carried by exactly one state;
- in the gauge-parameter space the maximal charge is j/2 + 1.

**Consequence (a counting argument).**

- The unique state of charge j/2 + 2 spans a one-dimensional block. It is therefore an eigenvector of every
  operator that commutes with Q.
- G*b has charge j/2 + 2. The gauge-parameter space has no such charge, so G*b = 0.
- So b is gauge-orthogonal, and Hb = (H + GG*)b.
- Its eigenvalue is its diagonal entry.
- **This is Proposition 6.1 of the Floor Theorem paper, from symmetry plus counting.** It holds for any
  circle-invariant background on (t, 1, 1), not only the Berger field.

**What follows for the hypothesis.**

- "There is an exact, gauge-orthogonal, closed-form extremal mode in every sector" is automatic wherever the circle
  symmetry is. It is not evidence for a skeleton.
- The content of the skeleton claim is that the extremal mode is the **floor**, and how that floor behaves when the
  circle is broken.

**The perturbation used in Test 3A.** Let g_ε = (t, 1 + ε, 1). Write the sector operator as
A(ε) = A₀ + εA₁ + ε²A₂ + …, estimated by central differences at ε = 10⁻³.

- A₁ couples charges q and q′ only when |q − q′| ∈ {0, 2}. Measured: |Δq| = 2 entries are 4.0 to 7.0, and
  |Δq| ∈ {1, 3, 4} entries are ≤ 2 × 10⁻¹².
- A₂ also couples only |Δq| ∈ {0, 2}. Measured: other entries ≤ 6 × 10⁻⁹, which is finite-difference noise at
  ε² = 10⁻⁶.
- In general, order n can change the charge by at most 2n. That is all the prediction below uses.

**Kramers.** At odd j every eigenvalue of A₀ is doubly degenerate. Measured: pair spacing ≤ 7 × 10⁻¹⁴. This is
expected: the representation is quaternionic, the coefficients are real, and the antiunitary symmetry squares to −1.

## 2. Test 3A: breaking the circle, keeping left-invariance (runnable now)

### Setup, fixed now

- **Operator.** A(ε) = H + GG* at the Levi-Civita field of g_ε = (t, 1 + ε, 1), read as an SU(2) connection, in the
  sector of spin j/2. The field is left-invariant, so sectors still separate. Only the circle about the long axis is
  broken.
- **Points.** t ∈ {3, 4}, the two certified stretches. j ∈ {0, 1, 2, 3, 4, 5, 6, 8, 12}.
- **Precision.**
  - The exponent fits run in mpmath at ≥ 60 significant digits. The splittings reach ε^8 ≈ 10⁻³² at j = 12.
  - The multiprecision builder must first reproduce `hessian_and_gauge` (double) to 10⁻¹² relative, at ε = 0 and at
    ε = 0.01, for every j.
- **ε grids.**
  - For the fits: |ε| ∈ {1, 2, 5, 10} × 10⁻⁴, both signs, so 8 points.
  - For the description: ε ∈ {0.01, 0.02, 0.05, 0.1, 0.2, 0.5}.
- **Pairs.**
  - At ε = 0, a *pair* is a two-dimensional eigenspace of A₀ spanned by states of charge +q′ and −q′, with q′ > 0.
    Charges are read by diagonalising Q in the eigenspace.
  - A pair is *isolated* if every other eigenvalue lies at least 10⁻³ away. Only isolated pairs enter a gate.
  - The twist pair has q′ = j/2 + 2.
- **Continuation.** At ε ≠ 0 the continued pair is the two eigenvalues nearest the ε = 0 value. This is valid while
  2‖A(ε) − A₀‖₂ is less than the isolation gap, which is checked at every ε used.
- **Splitting and slope.** s(ε) = |μ₂(ε) − μ₁(ε)|. The slope is the least-squares slope of log s against log|ε| over
  the 8 fit points.

### The prediction, and why it can fail

**Even j.** At even j the antiunitary symmetry squares to +1. It maps charge q to −q and survives the perturbation,
because the coefficients stay real. So the two members of a ±q′ pair shift together at every order. They split only
through the coupling between +q′ and −q′. With charge steps of at most 2 per order of ε, that coupling first appears
at order ε^{q′}. Generically,

  **s(ε) ∝ ε^{q′},  so the twist pair splits as ε^{(j+4)/2}, later than every other pair in its sector.**

- **What "the skeleton is the highest-weight structure of the circle" asserts here.** The floor degeneracy is
  protected by extremal charge, to an order set by that charge, and more than anything else in the sector.
- **The competing reading.** If the doubling of the floor were an accident of the exact symmetry, or protected by
  something other than the charge, the twist pair would not stand out from the other pairs.

**Odd j.** Δq = j + 4 is odd and the perturbation changes the charge by even steps. Kramers forces exact degeneracy
anyway. So s(ε) = 0 at every ε. This is a code check, not evidence.

### Gates

| gate | what must hold | if it fails |
| --- | --- | --- |
| **A-code** | the mp builder reproduces the double builder (above) | stop: code error |
| **A-sym** | at ε = 0, ‖[Q, A₀]‖ ≤ 10⁻¹²‖A₀‖; the twist pair is isolated, with charges ±(j/2 + 2) | stop: the premise is wrong |
| **A-odd** | j odd: s(ε) ≤ 10⁻⁴⁰ max\|λ\| at every ε | stop: code or symmetry error |
| **A-exp** | j even ∈ {0, 2, 4, 6, 8, 12}: the twist-pair slope is (j + 4)/2 ± 0.15 (2, 3, 4, 5, 6, 8) | below: **counts against**. Above: an extra protection to be explained; it counts neither way |
| **A-rank (i)** | in each even sector, the twist pair's slope is the largest of all isolated pairs | **counts against**: the extremal pair is not the most protected |
| **A-rank (ii)** | in each even sector j ≥ 2, at least one isolated pair has q′ ≤ 2 and slope ≤ 2.15 | **inconclusive**: no low-charge pair splits early, so the test cannot tell charge protection from anything else |

A-rank is the removal arm. It shows the mechanism by its absence: pairs without extremal charge must split early.

**Pass.** All gates hold. The claim it supports is this: "When the circle about the long axis is broken, the floor
pair stays degenerate to order (j + 4)/2, more than any other pair in its sector. The protection is the extremal
charge." Status: **Numerical** (multiprecision, not interval-certified).

**Descriptive, with no gate.**

- **ε_W.** The largest ε on a fine grid with 2‖A(ε) − A₀‖₂ < gap₀(j), where gap₀ is the distance from λ_t(j) to the
  third eigenvalue. Within ε_W the continued pair is the floor by Weyl's inequality, so no outcome there is evidence.
- **ε_c.** The first ε on the descriptive grid at which the continued pair is not the lowest two.
- Both are reported with their ratio. The hypothesis predicts no value for either, so neither counts for or against.

## 3. Test 3B: breaking left-invariance, so the sectors mix (not designed yet)

- No code exists that couples sectors. Before any is written, its gates will be fixed the same way.
- Every prediction must be one that a non-skeleton operator would fail. §0 rules out persistence.
- The natural candidate is a Peter–Weyl selection rule for which sectors the floor pair couples to, and at what
  order. That design is open.

## 4. Test 2: other families (exact criteria)

### What does not count

- **Quotients** (lens spaces, the Poincaré sphere). The local operator is unchanged; a quotient only selects sectors.
  These are not independent families.
- **Exactness, gauge-orthogonality, or a closed form for the extremal mode** in a circle-invariant family. By §1
  these are automatic.

### What is measured

For an independent family with a circle symmetry, fix in advance the parameter range and the sectors. Then classify:

- **E:** the unique maximal-charge state is the floor of every listed sector over the whole range;
- **P:** it is the floor only on a subrange, bounded by a threshold, as t* bounds it for the Berger family;
- **N:** it is the floor nowhere in the range.

Classification is Numerical (double precision). Any result used in a paper is certified with the interval machinery.

### Already known

The Berger family, t ∈ (0, 10], is **P**:

- the twist is the floor for t ≥ t* ≈ 2.698 (numerical);
- it is certified on [2.75, 10] in 15 sectors;
- it is the floor of no sector at t ≤ 2, including the round sphere, where the meron's floor is lower.

So the universal reading, "the floor is the extremal mode", is already false within the first family. What remains
testable is the scoped reading below.

### The scoped hypothesis, and its verdict rule

> In every independent circle-invariant family, the unique maximal-charge state is the floor of every listed
> sector on a parameter range of positive length.

- **Supported** if every tested family is E or P.
- **Counted against** if any tested family is N.
- P is a weak positive. It says the skeleton governs part of the family, not all of it.

### Families, fixed now

1. **The squashed meron** (the resting field of Result 6). Ansatz m₀ = x e₀, m₁ = y e₁, m₂ = y e₂, so it is
   circle-invariant. Range t ∈ [0.05, 10]; sectors j ≤ 12. Existing code: `ricci_su2_meron_family_soft.py`.
   Not yet classified.
2. **SU(3).** The design is open, and the choice of background is Bee's to make before any build. The Levi-Civita
   connection of a squashed SU(3) takes values in so(8), not in su(3), so the Berger-field construction does not
   carry over. A background (for example, a multiple of the Maurer–Cartan form on a squashed metric) and its circle
   or torus symmetry must be chosen first.

## 5. What would count against the hypothesis, in one list

- **Test 3A:** A-exp below its window, or A-rank (i), at any even sector tested.
- **Test 2:** any independent tested family classified N.
- **Not against** (inconclusive or descriptive):
  - A-exp above its window;
  - A-rank (ii);
  - any value of ε_c or ε_W;
  - persistence at small breaking, which is automatic.
