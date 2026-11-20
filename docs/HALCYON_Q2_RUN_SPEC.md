# Q2 run spec: an exact quantum answer for EFFECTIVE_MASS (2D SU(2))

2026-11-19. Written before the sampler, the oracle script or any ensemble exists. Branch
`feat/ym-engine`. Parent: `docs/HALCYON_QUANTUM_GAP_SPEC.md` §Q2, which this run spec amends.

## Two corrections to the parent spec, made before any data

**1. The convention of "3β/8".** The parent spec, citing `clay7_gigi_math.md` §1, takes the
heat-kernel gap as m = 3β/8 with β = 1/(g²a²). That column cannot be right in either convention:

- the physical heat kernel of area a² weighs representation j by exp(−g²a² C₂(j)/2), so with
  β = 1/(g²a²) the fundamental string tension is σa² = C₂(½)/(2β) = 3/(8β), which falls as β grows;
- the same document sets it beside the Wilson string tension σ_W = −ln(I₂(β)/I₁(β)), which uses the
  standard SU(2) β = 4/(g²a²), and says the two agree as β → ∞. In that convention the matched heat
  kernel is exp(−2C₂(j)/β), so σ_HK a² = 3/(2β).

| β | Wilson σ_W | heat kernel 3/(2β) | stored column 3β/8 |
|---|---|---|---|
| 1 | 1.4263 | 1.5000 | 0.3750 |
| 2 | 0.8367 | 0.7500 | 0.7500 |
| 3 | 0.5658 | 0.5000 | 1.1250 |
| 8 | 0.1994 | 0.1875 | 3.0000 |

3/(2β) tracks Wilson and converges to it; 3β/8 grows with β and agrees with 3/(2β) only at β = 2.
The stored column has the convention inverted. **This run uses β = 4/(g²a²) for both actions**, so
the two actions at the same β are the same continuum theory, and the heat kernel's character
coefficients are λ_j = exp(−2 j(j+1)/β). The Wilson ones are λ_j = I_{2j+1}(β)/I₁(β).

**2. The channel.** In two dimensions plaquettes decouple (up to the torus constraint), so the
plaquette correlator has no gap to measure, and "3/(2β) per unit area" is a string tension, not a
mass. The exact gap is in the Polyakov loop. On an N_s × N_t torus, with P(t) the ordered product of
the N_s spatial links at time slice t and p(t) = ½ tr P(t), the character expansion gives, exactly,
for either action,

    ⟨p(0) p(t)⟩ = ¼ · Σ_{j, k : |j−k| = ½} λ_j^{N_s t} λ_k^{N_s (N_t − t)}  /  Σ_j λ_j^{N_s N_t},

and ⟨p⟩ = 0 (centre symmetry), so it is also the connected correlator. The leading terms (j, k) =
(0, ½) and (½, 0) are a pure cosh with mass

    m = −N_s ln λ_½        (heat kernel: m = 3 N_s / (2β)).

The plaquette channel becomes the second null control (below).

## The run

- Lattice N_s = 2, N_t = 32, periodic. β ∈ {3, 4, 6, 8}. These replace the parent's {0.5, 1, 2, 3}.
  At β ≤ 2 with N_s ≥ 2 the mass is ≥ 1.5 and C(t) is lost in noise by t = 3, so no plateau of three
  points could be resolved at feasible statistics. The chosen window puts m between 0.375 and 1.13.
- Exact values (independent script, Bessel functions from scipy, sum to 2j = 59; exact m_eff(t) is
  flat to 1e-6 over t = 1..5 at every β):

  | β | m heat kernel | m Wilson | difference |
  |---|---|---|---|
  | 3 | 1.0000 | 1.1315 | 0.132 |
  | 4 | 0.7500 | 0.8370 | 0.087 |
  | 6 | 0.5000 | 0.5417 | 0.042 |
  | 8 | 0.3750 | 0.3987 | 0.024 |

- Sampler: engine code (`src/gauge/lattice2d.rs`), Metropolis per link with the exact class-function
  weight of the action (the heat kernel's as a character sum to 2j = 59). The same sampler serves
  both actions, so the removal changes only the action. 2,000 thermalisation sweeps, 20,000
  measurements, one every 5 sweeps, fixed seeds recorded in the receipt.
- Estimator: the Q1 verb unchanged (plateau rule as fixed there), with a blocked jackknife
  (blocks of 100 measurements) because Monte-Carlo measurements are autocorrelated. The block size
  is fixed here, before data.

## Gates (fixed now)

- **Q2-a, the oracle.** For every β, on the heat-kernel ensemble, EFFECTIVE_MASS returns a plateau
  (no refusal) and |m_plateau − m_exact| ≤ 2σ, with m_exact the exact m_eff averaged over the same
  window with the same weights.
- **Q2-a′, the whole correlator.** For every β and t ∈ [0, N_t/2], |C_meas(t) − C_exact(t)| ≤ 4σ(t).
  (The C(t) are correlated across t, so this is a per-point bound, not a χ².)
- **Q2-w, the second oracle.** The same two gates on the Wilson ensemble against the Wilson exact
  values. The character expansion is exact for any class-function action, so Wilson has an exact
  answer too.
- **Q2-r, mechanism removal.** At every β, the Wilson plateau differs from the heat-kernel exact
  m by more than 3σ. If it does not at some β, the record says the run cannot tell the two actions
  apart there; that β does not count for Q2-a.
- **Q2-n, null controls.** On every ensemble EFFECTIVE_MASS refuses (a) on the plaquette channel
  (p(t) = mean over x of ½ tr U_p(x, t)) and (b) on the Polyakov series shuffled across
  configurations and time slices (Halcyon's null construction).
- **Sampler sanity (a test, not a gate).** ⟨½ tr U_p⟩ is λ_½ up to corrections of order λ_½^{A}
  (A = N_s N_t); the engine test checks it to 4σ at one β per action.

## Not in this run

- The convention fix in Halcyon's stored bundle and in `clay7_gigi_math.md`. That is Halcyon's
  record. This run reports it, and Bee decides how to correct it.
- Any claim about four dimensions. Two-dimensional Yang–Mills has no propagating gluons. Q2 tests
  the estimator, the sampler and the plateau rule against an exact answer, nothing more.
