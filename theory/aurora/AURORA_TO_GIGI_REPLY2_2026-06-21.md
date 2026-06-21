# AURORA → GIGI reply 2, 2026-06-21

**Re:** Q4 (aurora_crate repo/deploy), Q5 (init-hook lifecycle), Q6 (binary-stability contract)
**In reply to:** `theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY_2.md`
**Author:** Bee Rosa Davis, with Claude (Anthropic)

---

## 0. Letter

Gigi —

Q4, Q5, Q6 answered below. All three answers point the same direction: AURORA ships as its own host binary, registers eagerly, and pins hard for v0.1. Phase 2 is unblocked on our end.

One editorial note before the answers. The second reply is the clearest framing we've received of why DEC does not land on `Lattice` directly — metric-awareness is a property of the topology constructor's output, not of the bare graph. The `LatticeWithMetric` wrapper shape is right; AURORA will call `lattice::dec` free functions, not methods. This resolves Q3 cleanly and we won't re-raise it.

—Bee + Claude

---

## 1. Q4 — aurora_crate repo location and deploy topology

**Q4a: repo location and visibility**

`aurora_crate` lives at `~/Documents/aurora` — a separate git repo from gigi. Private for v0.1. Cargo dependency: path dep for local development (`gigi = { path = "../gigi" }`), switching to a pinned git ref once gigi tags a v0.1 release (`gigi = { git = "...", rev = "<sha>" }`). Not on crates.io for v0.1.

**Q4b: feature-flag-in-gigi vs separate host binary**

AURORA ships as its own host binary. The `aurora` crate (`~/Documents/aurora/`) builds a binary that depends on both `gigi` and `aurora_crate` directly. gigi's `Cargo.toml` does NOT grow an `[features] aurora` flag. Reasons:

- gigi stays pure substrate. AURORA's physics does not belong in gigi's CI surface, test suite, or compile graph.
- The separate-binary shape is already what the codebase reflects: `aurora` is a standalone Cargo workspace with `gigi` as a library dep, not a subdirectory.
- fly.io deploy: the `aurora` binary is its own `Dockerfile`; it links gigi-the-library + aurora_crate at build time. No `--features` flag needed on gigi's side. gigi-server and aurora-server are different binaries with different deploy units.

If a future multi-Gi-System deployment needs aurora and halcyon sharing a single gigi process, we can revisit a feature-flag approach then. For v0.1 the separate binary is cleaner.

---

## 2. Q5 — init-hook lifecycle and WAL replay ordering

**Q5a: where `aurora_crate::init()` lives**

Top of `main()`, before `Engine::open()`. Exactly:

```rust
fn main() -> anyhow::Result<()> {
    aurora_crate::init();          // registers ShallowWater + CUBED_SPHERE
    let engine = Engine::open(config)?;
    // ...
}
```

No lazy registration. We agree: a `HamiltonianDeclare` WAL entry referencing `SHALLOW_WATER` with no registered factory should fail fast at replay, not silently produce a missing-factory panic somewhere downstream.

**Q5b: eager vs lazy registration**

Eager. All factories registered at `init()` time. v0.1 has exactly one Hamiltonian (`ShallowWater`) and one topology (`CUBED_SPHERE`); lazy registration would add complexity with no benefit. If AURORA grows a second Hamiltonian later (3D baroclinic, primitive equations), it registers in the same `init()` call. The caller doesn't opt in per-type.

---

## 3. Q6 — binary-stability contract

**Q6a: pinned vs floating versions**

Pinned for v0.1. `aurora_crate/Cargo.toml` will lock to a specific gigi git ref (or `=0.1.0` once tagged). Breaking changes in `HamiltonianHandle` or the four sub-traits require a deliberate dep bump in aurora_crate. This is the correct policy while the surface is still evolving.

**Q6b: stability annotation on the trait surface**

Yes, and we want it. Requested convention:

```
// src/gauge/action.rs
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
```

The "stability: EVOLVING" comment plus the version-bump rule is the minimum contract we need to safely pin. It does not require Rust's `#[stable]` machinery (which doesn't exist in stable Rust for library crates); a documented comment + changelog discipline is sufficient for v0.1.

gigi has no convention for this today — AURORA is fine being the reason it grows one.

---

## 4. Phase 2 gate status

With Q4/Q5/Q6 answered:

| Gate | Status |
|---|---|
| Q4 answered | ✓ |
| Q5 answered | ✓ |
| Q6 answered | ✓ |
| Phase 2 unblocked on AURORA side | ✓ |

Engine can begin `src/gauge/hamiltonian_registry.rs` and the four-trait refactor. AURORA will scope the `ShallowWater` factory shape against the §8 trait sketch from the second reply and send a concrete factory skeleton before the A2 refactor lands, so the interface has a known consumer at design time.

—Bee + Claude
