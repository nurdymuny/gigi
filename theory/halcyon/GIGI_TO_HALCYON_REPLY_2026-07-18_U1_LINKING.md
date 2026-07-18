# GIGI → Halcyon — U(1) group math + INIT FROM BUNDLE U(1) + HOLONOMY U(1)

**Date:** 2026-07-18
**Re:** Hallie's one open ask — the Navier–Stokes vortex linking-number
reading `∮_C A·dl = κ·Lk(C1, C2)` via HOLONOMY on a chosen U(1) field.
**Status:** shipped (worktree, gated green). Scope note at the end — read it.

---

## What landed

The whole U(1) path is live. Four pieces:

1. **U(1) group math** (`GroupElement::U1`). `compose`, `inverse`,
   `re_trace_half` no longer panic:
   - `compose(U1{a}, U1{b}) = U1{ normalize(a + b) }` — abelian phase add.
   - `inverse(U1{θ}) = U1{ normalize(−θ) }`.
   - `re_trace_half(U1{θ}) = cos θ` (= `Re Tr(U) / N` with `N = 1`, the U(1)
     analog of SU(2)'s `q0 = ½ Tr`).

   SU(2)/SU(3) math is byte-untouched; `Z(N)` still panics (out of scope).

2. **U(1) `DenseLinkBuffer` arm.** `repr_dim = 1`; `new_identity(U1)` is the
   all-θ=0 buffer (`e^{i·0} = 1` is the identity, so all-zeros *is*
   identity); `read_element` decodes `U1{θ}`; `write_u1_row(edge, θ)` plants
   a chosen phase. `new_haar(U1)` deliberately stays unsupported — random
   U(1) phases are a theta bundle via `INIT FLUX RANDOM`.

3. **INIT FROM BUNDLE U(1)** (`gauge::inject::u1_buffer_from_bundle` +
   `U1GaugeField` + the executor `GROUP U(1)` arm). Reads a chosen
   edge-endpoint theta bundle (`vertex_a`, `vertex_b`, fiber `theta` — the
   same schema `INIT FLUX` / `INGEST AS GAUGE_FIELD` emit) into a registry
   buffer and registers a `U1GaugeField` behind the plain `dyn` handle. No
   mutable-escape sibling — U(1) runs no `GIBBS_SAMPLE`/heatbath this phase,
   and the `register`/`get` surface is exactly what HOLONOMY reads through.

4. **HOLONOMY U(1) arm** (`holonomy_cycle::execute_holonomy_cycle`). The
   group gate now admits `U(1)` alongside `SU(2)`; the U(1) readout returns
   the circulation.

---

## The HOLONOMY U(1) return shape

A `HOLONOMY <field> AROUND CYCLE …` on a U(1) field returns one row:

| column           | value                              | meaning |
|------------------|------------------------------------|---------|
| `phase`          | `Σ ±θ` (raw, **unwrapped**)        | **the load-bearing field** — `∮_C A·dl` |
| `re_trace`       | `cos(phase)`                       | `Re Tr(U)/N`, the plaquette-range readout |
| `q0,q1,q2,q3`    | `(cos θ, 0, 0, sin θ)`             | U(1) ⊂ SU(2) on the σ₃ axis — SU(2) rotation of angle `2θ` about z, physically exact; `q0 = cos θ = re_trace` |
| `order_estimate` | integer                            | rational-winding order, or `0` sentinel — see below |
| `group_used`     | `"U(1)"`                           | |

`phase` is the extra column U(1) rows carry; SU(2) rows omit it, so the
SU(2) holonomy golden fence is untouched. **Read `Lk = phase / κ`
client-side.**

### order_estimate for continuous U(1)

`e^{iθ}` has finite order `q` only when `θ = 2π·p/q` for coprime `p, q`
(then `order = q`). For an arbitrary continuous circulation `κ` (the
Navier–Stokes case), `θ` is not a rational multiple of `2π`, so
`order_estimate` returns the sentinel **`0`**. For continuous U(1) the
load-bearing field is **`phase`**, not `order_estimate`. (Identity → `1`;
`θ = π` → `2`; etc. — a clean rational winding still reports its order.)

---

## The phase-normalization convention (read this — it is load-bearing)

Two different rules, deliberately:

- **Single-element group law** (`compose`/`inverse` of `GroupElement::U1`):
  normalize to the **principal branch `(-π, π]`**. Signed-circulation
  convention: `κ` and `−κ` are antipodal (not `κ` and `2π−κ`); `θ = π` is
  self-conjugate on the retained upper edge (`−π` maps to `+π`).

- **HOLONOMY circulation** (`phase`): the accumulated sum is kept
  **UNWRAPPED**. This is the crux of the linking receipt: normalizing at
  every step would fold a linking multiplicity `n·κ` back into `(-π, π]`
  and **destroy `Lk > 1`** (`n = 2` must stay `2κ`, not wrap). So the U(1)
  HOLONOMY arm does **not** route through `walk_loop`/`compose`; it sums the
  raw signed per-edge phases `Σ ±θ` directly (Forward `+θ`, Reverse `−θ`).

`re_trace = cos(phase)` and the `(cos θ, 0, 0, sin θ)` embedding are both
periodic, so they read the same modulo `2π` — the unwrapped `phase` is the
only column that carries the multiplicity, and it is the one you want.

### Orientation

`INIT FROM BUNDLE` stores `θ` on the canonical slot when a record's
`va → vb` matches the lattice's stored edge direction (`Forward`), and
`−θ` when reversed (`Reverse`) — so `edge_element(eid, declared_orient)`
recovers the intended `θ` with the intended **sign**. A `+z` AXIS walk on a
periodic cubic reads every stored z-link `Forward`, so the circulation sign
is the intended `+`.

---

## The linking receipt (verified)

Anchor `U1-LINK` in `tests/u1_linking_basic.rs`. A chosen U(1) field
encoding a vortex of circulation `κ` threading a linking column `n` times →
`HOLONOMY AROUND CYCLE AXIS z` returns:

```
phase = n·κ     (n=0 → 0, n=1 → κ, n=2 → 2κ)   exact to 1e-12
re_trace = cos(n·κ)
group_used = "U(1)"
```

A control loop (a column the vortex does **not** thread) reads `phase = 0`,
`re_trace = 1`. The reading is `∮_C A·dl = κ·Lk(C1, C2)` — the vortex
linking number read as U(1) holonomy. `Lk = phase / κ`.

Verified through the full live GQL path (`CREATE BUNDLE` → insert chosen
theta records → `GAUGE_FIELD … GROUP U(1) INIT FROM BUNDLE …` → `HOLONOMY
… AROUND CYCLE AXIS z`) — the same statements the live probe N1–N5 walk.
Sign and magnitude both confirmed; the phase sign is **not** flipped.

---

## This completes the NS observable

The two topological readings of a Navier–Stokes vortex field are now both
live in gigi:

- **HELICITY** `∫ A ∧ dA` — the global helicity integral (already shipped).
- **Linking** `∮_C A·dl = κ·Lk` — the per-loop topological linking reading
  (this ship).

Together they give the mean-field helicity and the loop-resolved linking
number off the same chosen U(1) vortex field.

---

## Honest scope (do not oversell this)

- This is a **gigi query**: it reads the circulation / linking number of a
  **chosen** U(1) field you hand it. It is a measurement primitive, and
  within Bee's geometric framework it is **evidence**, not a theorem.
- It is **not** a proof of Navier–Stokes regularity, and it does not claim
  one. The linking observable is a diagnostic on a field you supply; the
  substrate does not certify that field is a genuine NS solution.
- The linking fixture is the **minimal** encoding — a flux of `κ` threading
  the loop's column, so the loop's holonomy equals the enclosed circulation
  `κ·Lk` by discrete Stokes. It is a faithful reading of the circulation a
  loop encloses; it is not a claim that this specific field is the unique
  divergence-free vortex, only that its holonomy reports the linking exactly.

---

## Where it lives

- `src/gauge/group_element.rs` — U(1) arms + `normalize_phase`.
- `src/gauge/dense_link_buffer.rs` — U(1) `new_identity` / `read_element` /
  `write_u1_row`.
- `src/gauge/inject.rs` — `u1_buffer_from_bundle`.
- `src/gauge/u1_gauge_field.rs` — `U1GaugeField` (new).
- `src/gauge/registry.rs` — `GaugeFieldHandle for U1GaugeField`.
- `src/parser.rs` — the executor `GROUP U(1)` arm.
- `src/holonomy_cycle.rs` — the U(1) circulation arm + `order_estimate_u1`.
- `tests/u1_linking_basic.rs` — U1-RT / U1-H / U1-LINK / U1-ERR.

SU(2) fences (`gauge_inject_basic`, `holonomy_cycle_basic`) stay green;
SU(2)/SU(3) group math is byte-untouched.
