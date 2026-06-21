# GIGI → AURORA, Q4 / Q5 / Q6 resolved + Phase 2 unblock acknowledgment (2026-06-21)

**From:** GIGI engine team (Bee + Claude)
**To:** AURORA team
**Subject:** Q4b / Q5 eager / Q6 stability-annotation confirmed. Phase 2 unblocks. Plan + parallel work pending your `ShallowWater` factory sketch.
**In reply to:** AURORA's 2026-06-21 Q4/Q5/Q6 reply (received via Bee).
**Prior letters:**
- `theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY.md` (commit `49afc22`)
- `theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY_2.md` (commit `ad306ec`)

---

## Letter

AURORA —

Three clean answers received. Three OPEN questions closed. Phase 2 unblocks on the substrate side. Acknowledgment + the substrate-side state + what we do next, in that order. Brief on purpose; this is a handoff, not a scope review.

**Q4b — separate host binary, gigi's `Cargo.toml` stays clean.** Accepted as-stated. This makes AURORA the canonical example of the downstream-crate pattern the README references. Halcyon is one of the special-three in-tree-feature-flag systems; everything else (AURORA today, ICARUS later, anyone past that) is the AURORA shape. The substrate-side `hamiltonian_registry` ships as a regular pub API that any downstream crate can call; gigi does not need a `shallow_water` feature flag, does not need to know `ShallowWater` exists, and does not need to ship the trait impl. AURORA's binary owns the registration and the impl; gigi owns the registry.

**Q5 — eager `init()` at top of `main()`, no lazy anything.** Accepted as-stated. Registry exposes:

```rust
pub fn register(name: impl Into<String>, factory: Box<dyn HamiltonianFactory>) -> Result<(), RegistryError>;
```

AURORA's `main()` calls it once at startup. No `lazy_static!`, no `OnceCell`, no thread-local magic, no auto-registration. The WAL replay ordering question that v1 Reply 2 §11 raised under Q5 dissolves because eager registration means the registry is populated before any WAL replay can fire a `HamiltonianDeclare` that needs to resolve a factory. The error message when AURORA forgets to call `register()` is the natural one: "no factory registered for hamiltonian kind `ShallowWater`."

**Q6 — gigi grows a stability annotation convention because of AURORA.** Accepted as-stated. The framing — "AURORA being the reason gigi grows one is accurate and appropriate" — is the right read. Halcyon doesn't need this because it's tight-coupled via feature flag in-tree; refactors land atomically with their consumer. AURORA is the first external pinned consumer; what gigi guarantees about the `HamiltonianFactory` / `HamiltonianForce` / `HamiltonianDrift` / `ProjectionOperator` / `EnergyDecomposition` trait surfaces across versions is now a contract, not an implementation detail.

This is substrate-side design work. We will spec the convention as a Phase 2 deliverable alongside `hamiltonian_registry.rs`. Working draft of what we are likely to land:

- Extend `docs/STABILITY_GUARANTEES.md` from feature-flag stability (already covered: `kahler` production-stable, `sharded` research-grade) to trait-surface stability (new section).
- Marker convention: likely a `#[stable(since = "0.1.x")]` proc-macro attribute on the trait + each method, or — if proc-macro overhead is not justified for this surface — a `// STABILITY: stable since 0.1.x` doc comment at the trait + method level with a CI lint that fails on undocumented `pub trait` items in `src/gauge/` and `src/lattice/dec/`.
- "Stable" means: methods do not get removed, signatures do not get tightened (parameter list, return type, where-clauses), method bodies may change. Trait-default impls may be added. New methods with default impls may be added but each addition gets called out in the changelog as a minor-version event.
- "Research-grade" means: the surface may shape-shift commit to commit; pin a commit hash if you depend on it.
- Phase 2 ships with the convention defined and applied to the four AURORA-consumed traits + `HamiltonianFactory`. Halcyon-only surfaces (the `Hamiltonian` enum, the in-tree `KogutSusskind` variant) remain feature-gated and outside the stability contract.

We will draft this against AURORA's `ShallowWater` factory sketch (your committed next move) so the convention attaches to a known concrete consumer rather than a hypothetical one. If your factory's API ergonomics push back on any of the working-draft choices above, the pushback is welcome and the design adapts.

---

## Substrate-side state right now

**Phase 0 mid-flight.** `Lattice::signed_face_orientations()` is in implementation against the TDD harness as this letter goes out. Workflow `wqxj92qhh` runs the discover → RED → GREEN → verify cycle. Bit-identity gate (`halcyon_part_iv_gold`) is the kill criterion: if the lift drifts any computed face orientation, the work reverts and we surface the design question. Expected outcome: lift is additive (the buckyball constructor already computes signed orientations internally; the new method just exposes them as `pub`).

**Phase 1 next.** After Phase 0 commits cleanly, the A1 CUBED_SPHERE constructor + CC-2 registry refactor + topology_hint table land as a single Phase 1 sprint. Once Phase 1 ships, AURORA can construct a CUBED_SPHERE Lattice through the same registry API as TRUNCATED_ICOSAHEDRON.

**Phase 2 unblocked but not started.** Two prerequisites still pending:

1. Your `ShallowWater` factory sketch (your committed next move per the Q4-resolution message). The substrate-side `hamiltonian_registry.rs` design wants a known consumer at design time. We hold our trait-surface decisions (especially around generic `State` parameterization, `ProjectionOperator` lifetime, and `EnergyDecomposition` return shape) until we see your factory's API ergonomics.
2. The Q3 DEC operator surface (`src/lattice/metric.rs` + `src/lattice/dec/`) lands before your `HamiltonianForce` impl needs it; this is in Phase 2 sprint scope on our side, not blocked.

The Q6 stability annotation convention work happens in parallel with `hamiltonian_registry.rs` design. It does not need to wait for the factory sketch but it benefits from seeing it.

---

## What we are not changing from v1 / v2 commitments

- The hot-path constraint on A2 stands: trait-object dispatch lives off the integrator inner loop; per-step is generic-over-concrete `H: HamiltonianForce + HamiltonianDrift`.
- The DEC operator surface returns free functions consuming `LatticeWithMetric`, not methods on `Lattice` itself (post-`ea50585` general-purpose discipline).
- The `topology_hint` const table reserves both `S2/CUBED_SPHERE` and `S2/TRUNCATED_ICOSAHEDRON` so the buckyball gets metadata symmetrically.
- The A3-workaround six flat scalar names ship now (zero engine LOC); the `[TYPE; N]` arrays and inline structs with `__` desugar are Phase 3 general-purpose lifts, not Phase 2 blockers.

---

## What we owe you back

- Phase 0 commit hash + ASKS_LOG row flipped to DONE (any moment now, when workflow `wqxj92qhh` returns).
- Phase 1 ship: A1 CUBED_SPHERE constructor + CC-2 registry refactor + topology_hint table.
- Phase 2 substrate work, paced by your `ShallowWater` factory sketch arrival: `hamiltonian_registry.rs` + stability annotation convention + DEC operator surface.
- Cross-cutting: any pushback on the stability-annotation working draft above, received and reflected in the spec before Phase 2 ships.

The substrate timeline does not assume your `ShallowWater` factory sketch arrives by any particular date. Take the time you need; the substrate works the topology side (Phase 0 → Phase 1) and the stability-convention spec in parallel.

—Bee + Claude
