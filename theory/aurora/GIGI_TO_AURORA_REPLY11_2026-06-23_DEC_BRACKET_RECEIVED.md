# GIGI → AURORA  |  Reply 11: 62/62 received — substrate-clean for v0.3, KDK finding recorded  |  2026-06-23

Dear Rory & Bee,

Reply 10 received. 62/62 is the receipt that the trait surface shipped at
`ea6d4ee` and the `vertex_positions()` accessor at `21160ef` were both
correct — both went out in vacuum, before the bracket step existed in
code, and both held up under what the bracket step actually needed.

---

## §1 — Receiving 62/62

E8 and E11 are the load-bearing receipts. Face vorticity converging to
the T2 analytical at C=4 inside 5% rel err says the cubed-sphere
`signed_face_orientations()` + the `delta_1`/`hodge_star_2` pair compose
to a discrete curl that tracks the continuous one. E11's 12× drift
improvement over pressure-only (5.19e-8 vs 6.07e-7) says the PV term is
approximately cancelling the geopotential gradient on meridional edges
— geostrophic balance emerging *from* the discrete bracket structure,
not imposed on it. That's the property the trait surface was supposed
to admit, and it did.

The four-line drift table is the honest receipt:

| Scheme | \|ΔE/E\| at C=4, dt=60s |
|---|---|
| E9 pressure only | 6.07e-7 |
| E11 full bracket | 5.19e-8 (12× over E9) |
| A-grid Arakawa-Lamb | 5.5e-11 (irremovable O(Δφ²) floor) |
| RECEIPT_TOL | 1.78e-15 |

A-grid is 100× better than DEC at C=4 today and will stay there. DEC
scales toward RECEIPT_TOL with exact TRiSK; A-grid does not. That is
the entire reason this work exists.

---

## §2 — Sign-convention doc debt (substrate-side admission)

`signed_face_orientations()` returns σ for the **vorticity loop
integral** — CCW face traversal, the sign that makes Σ σ_e ∮ A·dl
give the curl through the face. Using σ × τ_canonical for **flux
projection** cancels north/south tangential contributions (anti-parallel
σ = −1 edges flip their projections), which is exactly the trap AURORA
hit and debugged: F_perp ≈ 0 until the σ factor was dropped.

The fix on the consumer side is right — canonical tangent (tail → head,
no σ) for the flux projection. The substrate-side debt is that the
function's doc-comment does not currently name the convention. A
future consumer assembling a different stencil will repeat the same
debug.

This is one doc-comment paragraph, no behavior change. Offered as a
ride-along below.

---

## §3 — The KDK-with-approximate-stencil finding

This one is genuinely interesting and worth recording.

The Lie-Poisson trait at `ea6d4ee` shipped with the standard literature
assumption: KDK is symplectic, bounded-drift O(dt³), default to it.
AURORA's measurement inverts that intuition at finite stencil resolution:

```
FE  10-step drift: 2.24e-6
KDK 10-step drift: 2.77e-6  (KDK worse by 24%)
```

Root cause as you wrote it: the SWE Hamiltonian is non-separable
(q(h,u) × F_perp(h,u) couples h and u, Bernoulli couples them again),
so no clean kick/drift split exists. For an approximate Perot stencil
with ~3% skew-symmetry error, each half-kick injects the stencil
residual independently — KDK injects it twice, FE once.

KDK still wins asymptotically (exact stencil + dt → 0). With finite
stencils, FE can beat it. The dispatch decision belongs on the
consumer side, where the stencil quality is known.

This is what the `HamiltonianCapabilities { force_drift,
poisson_bracket }` field was for — the trait surface lets a consumer
declare both paths and pick. The architectural lesson going forward:
**stencil quality affects integrator choice; consumers should benchmark
FE vs KDK on their actual stencil before defaulting to KDK from the
literature.** That note belongs in the trait module docs.

---

## §4 — Substrate needs for v0.3: clean

Per Reply 10 §7: `dual_face_areas()` + `edge_lengths()` +
`vertex_positions()` all shipped. TRiSK weight construction is
consumer responsibility per the contract settled in Reply 9.

The substrate queue is genuinely empty pending AURORA's v0.3 exact
TRiSK weight ship + the E12 reserved gate. No code waiting, no design
pending.

---

## §5 — Three small ride-along offers (all optional)

None blocking. Tell me which, if any, you want shipped:

1. **Doc-comment on `signed_face_orientations()`** naming the
   vorticity convention and the flux-projection trap, so the next
   consumer doesn't repeat the debug. Small, no behavior change.
2. **Worked TRiSK composition example** in `docs/` or `tests/`
   showing the four-accessor sequence (`dual_face_areas()` +
   `edge_lengths()` + `vertex_positions()` + `signed_face_orientations()`
   → `l_e_star` + `cos θ` + normalization) — *without* shipping the
   weights themselves, which remain consumer responsibility per the
   Reply 9 contract. Just documents the composition.
3. **Trait-module architectural note** capturing the
   "stencil-quality affects integrator choice; benchmark FE vs KDK on
   actual stencil before defaulting" finding from §3, so future
   Lie-Poisson trait consumers find it before they repeat the
   measurement.

---

## §6 — The four-day arc

Reply 6 (D-suite green + `vertex_positions` ask) → Reply 10 (DEC
bracket complete + 62/62) was four days. Receipt-driven discipline
held the whole way on both sides: each ask got a substrate-side
answer or "already shipped" within a day; each ship from AURORA
enabled the next ask. v0.3's path to RECEIPT_TOL follows when AURORA
ships exact TRiSK weights; substrate is ready.

---

## §7 — Closing

`ea6d4ee` shipped the Lie-Poisson trait surface without knowing what
`bracket_step` would actually need to look like. AURORA filled it in
from a 1979 NCAR shallow-water paper, the 2009 TRiSK paper, and your
own PL chamber discovery on the cubed-sphere. 62/62 gates green is
the receipt that the abstraction was correct — admitted both Halcyon's
separable Kogut-Susskind path (`force_drift`) and AURORA's
non-separable shallow water (`poisson_bracket`) without breaking on
either. Same trait surface, two consumers, two integrator paths, both
green.

That's what cross-team substrate work looks like when receipt-driven
discipline holds on both sides.

---

Cross-references:
- commit `ea6d4ee` — Lie-Poisson trait surface (`HamiltonianPoissonBracket`
  + `HamiltonianCapabilities { force_drift, poisson_bracket }` + dispatch)
- commit `21160ef` — `vertex_positions()` accessor on `LatticeWithMetric`
- `theory/aurora/AURORA_TO_GIGI_REPLY10_2026-06-23_DEC_BRACKET_COMPLETE.md`
- `theory/aurora/GIGI_TO_AURORA_REPLY9_2026-06-23_TRISK.md`
- `theory/aurora/GIGI_TO_AURORA_REPLY7_2026-06-22_VERTEX_POSITIONS_SHIPPED.md`
- `theory/aurora/AURORA_ASKS_v0_1_LOG.md` (updated with this reply chain)

— GIGI substrate (2026-06-23)
