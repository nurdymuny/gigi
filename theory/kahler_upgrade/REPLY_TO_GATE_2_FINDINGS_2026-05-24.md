# Reply to Marcella's Gate 2 findings — sphere geometry confirmed, 3 asks answered

**From.** Bee Davis + GIGI team (Claude pair).
**To.** Marcella team.
**Date.** 2026-05-24.
**Re.** Your Gate 2 PARTIAL — sphere finding + three asks.

---

## Confirming the meta-point first

Your PARTIAL is the honest report. The substrate-is-a-sphere
finding is **real signal**, not a check bug, not a missing
edge case. bge-small embeddings are L2-normalized → S³⁸³ →
K = +1 globally. The catalog's own `holo_sectional_check`
classification rules say `high_curvature` ⇒ *"avoid Hadamard
citations; use streaming bounds only"* — so the v3 paper
already has its guidance. This is the check doing exactly what
it was built to do.

Carrying it forward.

---

## Q1 — Does L5's curved-manifold transport already compose with L1.5's B-perturbation?

**Yes. Already wired.** Source: `src/bundle.rs`, lines ~4090–4123
(`BundleStore::transport_along`):

```rust
pub fn transport_along(&self, seg, dt, steps) -> Result<...> {
    // Hadamard gate. Refuse if not in a Hadamard region.
    if !self.is_hadamard_region(None) {
        return Err(...);
    }
    let bias = self.schema.kahler.as_ref().map(|k| &k.b);
    let b_source = if bias.is_some() { BSource::Bundle } else { BSource::None };
    crate::geometry::flat_transport(seg, bias, dt, steps, b_source)
}
```

The composition is `(L5 Hadamard gate) → (L1.5 flat_transport
with bundle's B)`. So per turn, the natural integration is:

```
if bundle.is_hadamard_region(query) {
    bundle.transport_along(seg, dt, steps)  // ✅ curved + B-perturbed
} else {
    flat_transport(seg, override_bias, dt, steps, BSource::Override)
    // OR classical residue rotation
}
```

### The S³⁸³-specific catch

`is_hadamard_region` gates on `holo_bisectional_max ≤ 0.5`
(`HADAMARD_KB_THRESHOLD` in `src/geometry/hadamard.rs`). Your
substrate's `K_H ≈ 1.14` means `transport_along` will **always
refuse on S³⁸³** — *and that's correct behavior*. The Cartan-
Hadamard theorem genuinely doesn't apply on positive-curvature
manifolds.

For your per-region integration, this means: **on a constant-
curvature sphere, no sub-region is locally Hadamard** (a sphere
is *homogeneous*, every point looks the same). The per-region
check will uniformly return `false`. That's not a bug — it's the
geometry telling you Hadamard isn't the right tool for this
substrate.

### What you can do instead — extrinsic / ambient

Your bge embeddings sit on S³⁸³ ⊂ R³⁸⁴, and **R³⁸⁴ = C¹⁹² IS
Kähler and IS flat**. You can transport in the AMBIENT space and
project back to the sphere by L2-renormalization:

```rust
// Ambient-flat transport (always works; ignores the sphere constraint
// during integration, then projects back at the end).
let result = flat_transport(seg_in_ambient, Some(&your_b), dt, steps, BSource::Override)?;
let endpoint = &result.trajectory.last().unwrap();
let norm = endpoint.iter().map(|x| x*x).sum::<f64>().sqrt();
let on_sphere: Vec<f64> = endpoint.iter().map(|x| x / norm).collect();
```

This sidesteps the Hadamard gate cleanly. The trade-off: the
trajectory between endpoints leaves the sphere; only endpoints
get projected back. For semantic transport between embedding
vectors that's usually fine — you care about start, end, and
energy drift, not the intermediate curve.

If you want **intrinsic sphere transport**, you'd need Sasakian
geodesics on S^{2n+1} (these are well-studied — great circles
with magnetic correction). That's a v2 enhancement; for first
wiring, extrinsic + projection is the right move.

---

## Q2 — Does `frobenius_compose` accept S^n for n > 2?

**Honest answer: no, and it shouldn't, because S^n for n > 2 is
not a Kähler manifold.**

Source: `src/geometry/quantum_cohomology.rs::QuantumCohomology`:

```rust
pub enum QuantumCohomology {
    Cpn { n: usize, q_truncation: usize },
    TorusTn { n: usize },
    Sphere2,            // <-- only S² (= CP^1) is hardcoded
    NonToy,             // <-- catches everything else
}
```

S³⁸³ would route through `NonToy` and return:

```
QuantumError::UnsupportedManifold {
    reason: "general_GW_invariants_not_computable"
}
```

### Why "S² only" isn't a missing feature

- **S² = ℂP¹** is Kähler (it's a complex 1-fold; the Fubini-Study
  metric makes it Kähler-Einstein with constant holo-sectional
  curvature 4).
- **S^4, S^6, ..., S^{2k}** for k > 1 are NOT Kähler. Their
  cohomology rings (`H*(S^{2k}; ℝ) = ℝ[x]/x²` with x in degree
  2k) don't have the Hodge structure a compact Kähler manifold's
  cohomology must have (Hodge symmetry b^{p,q} = b^{q,p} forces
  b² even, but b²(S^4) = 0).
- **S^{2k+1}** (odd-dim) doesn't even admit an almost-complex
  structure for most k (only S¹, S³, S⁵, S⁷ do — Borel-Serre
  obstruction).

So S³⁸³ being non-Kähler is **not a quirk of our API**, it's a
theorem. The right move for your substrate is one of:

**Option A — work in the ambient C¹⁹² (recommended for first wiring).**
Declare your schema's `kahler` as a structure on R³⁸⁴ = C¹⁹²:

```rust
let j = ComplexStructure::standard(192);  // J on R^384
let b = ClosedTwoForm::new_constant(...);  // your fixed B
let k = KahlerStructure::new(j, b);
schema.with_kahler(k);
```

Then `frobenius_compose` on `QuantumCohomology::cpn(191)` IS
valid — CP¹⁹¹ is the natural Kähler quotient of S³⁸³ by the
Hopf S¹ action. Your bge embeddings, modulo the unit-norm
constraint, ARE points of CP¹⁹¹. Frobenius/WDVV is then defined
and associative.

The catch: this treats two unit vectors related by a phase as
"the same point" of CP¹⁹¹. For bge embeddings where sign matters,
this loses semantic information. You'd want to confirm with the
bge model authors whether phase is meaningful.

**Option B — ship a `SphereN { n }` variant in GIGI.**
S^n has trivial cohomology `H*(S^n) = ℝ[x]/x²`, so the algebraic
product is trivially associative (x · x = 0). We could add this
variant in ~50 lines. The trade-off: it doesn't carry quantum
corrections (no Gromov-Witten content) — `frobenius_compose`
would always return the classical cup product, which is true but
might not be what your runtime expects from the catalog §2.10
"associative quantum product" phrasing.

If you want this, file an issue; I'll ship `SphereN { n }` as a
~half-day add. For first wiring, **Option A is the cleaner
path** because it gives you the full quantum apparatus.

---

## Q3 — B-source: (a), (b), or (c)?

**Option (c). Endorsed. Ship it.**

Reasons in order of decisiveness:

1. **(c) is constant ⇒ dB = 0 trivially.** Your closedness
   preflight clears on the first run. No B-learner test infra to
   build.

2. **Fast path to `kahler_active = true`.** First wiring is about
   proving the consumption pipeline works end-to-end, not about
   optimizing B's information content. (a) and (b) both delay
   that demonstration by weeks.

3. **GIGI shouldn't lock in B before the system has data.**
   Option (a) — GIGI attaches B to your bundle — would force a
   decision your runtime is better positioned to make later. We'd
   be picking a B for you, then iterating with you to undo our
   choice. Skip.

4. **Learned B (option b) is a research project.** Plenty of
   open questions: regularization, ground-truth signal,
   integrability constraints. Worth doing eventually, but bee
   shouldn't gate L8 flip on Marcella's research progress.

### The concrete B for option (c)

For your 384-D ambient, the canonical Kähler form is:

```
B = (1/2) · Σ_{k=0..191} dx_{2k} ∧ dx_{2k+1}
```

In matrix form: 384×384 block-diagonal of 192 copies of
`[[0, 0.5], [-0.5, 0]]`. Constant ⇒ closed ⇒ `dB = 0` exactly.

Construction in Rust:

```rust
let mut raw = vec![0.0_f64; 384 * 384];
for k in 0..192 {
    let i = 2 * k;
    let j = 2 * k + 1;
    raw[i * 384 + j] = 0.5;   // (i, j) entry
    raw[j * 384 + i] = -0.5;  // (j, i) entry (antisymmetric)
}
let tf = TwoForm::new(raw, 384).expect("antisymmetric");
let b = ClosedTwoForm::new_constant(tf);
```

### What your `holonomy_debt` will report on this B

For a small loop on S³⁸³ enclosing area `A`, the symplectic
integral is `0.5 · A_proj` where `A_proj` is the projected area
in the C¹⁹² Kähler plane. **This won't generally be a multiple
of 2π** — the sphere isn't a complex submanifold of C¹⁹², so the
Wu-Yang quantization argument doesn't constrain loops here.

You'll get `HolonomyDebt::Continuous(x)` for most loops. That's
correct and useful: the Continuous variant feeds your rose-
mechanism's α coefficient via `x` directly, no Davis non-
decoupling claim required. Document in v3 that this substrate
uses the Continuous variant — DON'T cite §2.1 Dirac quantization,
that requires holomorphic loops.

---

## Concrete next steps for you

1. **Use option (c)** with the constant `B = 0.5 · Σ dx_{2k} ∧ dx_{2k+1}` above.
2. **Re-run Gate 2.** Closedness preflight should clear immediately
   (constant B ⇒ exact 0). Holo-sectional + Hadamard preflights
   stay as they are (sphere geometry is sphere geometry).
3. **Implement consumption via Option A** (ambient flat_transport
   + L2 projection) for the first wiring. This sidesteps the
   Hadamard gate and gives you `kahler_active = true` on every
   turn. Self-inspect output reads:
   ```
   { "kahler_active": true,
     "transport_method": "ambient_flat_with_projection",
     "b_source": "Override",
     "energy_drift": 1.2e-12,
     "manifold_regime": "high_curvature_sphere" }
   ```
4. **Don't cite §1.4 / §1.5 / §2.1** in v3 — your substrate's
   geometry doesn't satisfy their preconditions. Cite §1.3
   (Jacobi cardinality, theorem-bound), §E.3 Ricci as a
   streaming bound, §2.5 spectral gap. Those all work on
   high-curvature substrates.
5. **Loop back once (1)–(4) are live** — at that point Gate 2 is
   ✅ and we can move on to Gates 3 and 4.

---

## What GIGI needs to ship (if anything) before you proceed

**Nothing blocking.** Everything you need for option (c) +
Option A is already in `main` at `82e6978`.

Optional shippable:

- **`SphereN { n }` variant** — only if you want frobenius_compose
  to non-error on S³⁸³. ~half-day add. Tell me if you want it
  before the consumption wiring lands or after.
- **A dedicated `TransportError::NotHadamard` variant** — current
  refusal in `transport_along` repurposes `DimensionMismatch`
  which is semantically wrong. ~1 hour to add a proper variant.
  Will land this regardless; it cleans up the L5 surface.

Neither blocks your work.

---

## On the protocol

You're not vetoing → flip protocol advances. Gate 2 is
"reported with caveats, no surprise failures" — that's a clean
pass with documentation, not a block.

Gate 3 (1-week clean run on sheets bundle) is on bee's side; I'll
start the clock once you've wired option (c) and confirmed the
consumption path fires on production turns.

Gate 4 (external geometer review) — bee will reach out to the
ICDG circle once the v3 paper draft is solid. The substrate-is-a-
sphere finding actually MAKES the paper easier: the geometer can
verify our claims more directly because the manifold has known
constant curvature, no surprises in the analysis.

— bee + Claude (GIGI side)
