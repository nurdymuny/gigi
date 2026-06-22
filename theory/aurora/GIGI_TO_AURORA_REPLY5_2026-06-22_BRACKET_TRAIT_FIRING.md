# GIGI → AURORA  |  Reply 5: Bracket spec accepted, workflow firing  |  2026-06-22

Dear Rory & Bee,

Your bracket spec landed and substrate-side reads it as a complete deliverable: Q1-back answered (Option B — state-mutating step + substrate-side receipt), Q2-back answered (AFTER — design against concrete spec), §4 the concrete Lie-Poisson math, §5 the trait shape, §6 the capabilities dispatch. **Everything needed to ship is here.** Firing the implementation workflow now; this reply is acknowledgment + one substrate-side design choice for backwards-compat + your one open question answered.

## §1 — `Engine::open_memory()` shipping

Yes. The tempdir-boilerplate argument is real — substrate-side has the same hidden cost on its own test harnesses. Shipping `Engine::open_memory()` as an explicit constructor in the same workflow (`wwhx0aljf`). ~30 LOC, tempdir held in an `Option<TempDir>` field on the Engine struct (None for file-backed, Some for in-memory), automatic cleanup on Drop. Backwards-compat for the other three constructors is preserved (they continue to initialize `_tempdir: None`).

After ship: `Engine::open_memory()` is the recommended dev/CI/test constructor; tempdir-by-hand stays available for cases needing explicit path access.

## §2 — `bracket_step` shape: Option B accepted

Your error-boundary framing is precisely right: **physics invalidity (negative depth, CFL, NaN) returns Err; Casimir drift is the substrate's receipt responsibility, measured externally via `evaluate()` against `RECEIPT_TOL`.** Same referee contract as the existing Euler path. One place for physics, one place for conservation accounting.

The trait sketch is the design spec verbatim:

```rust
/// Stability: EVOLVING until gigi 0.1.0 tag.
pub trait HamiltonianPoissonBracket: HamiltonianHandle {
    fn bracket_step(
        &self,
        state: &mut [f64],
        dt: f64,
    ) -> Result<(), BracketPhysicsError>;
}

pub enum BracketPhysicsError {
    NegativeDepth { i: usize, j: usize, h: f64 },
    CflViolation { courant: f64, max_courant: f64 },
    Other(String),
}
```

Both ship in `src/gauge/action.rs` alongside the existing AURORA Phase 2 trait surface. EVOLVING markers per `Q6`.

## §3 — `HamiltonianCapabilities` + `capabilities()` with backwards-compat default

Substrate-side made one concrete design choice that needs surfacing: **`capabilities()` ships with a default impl** so existing `HamiltonianFactory` implementations (KogutSusskindFactory + any others) inherit `{ force_drift: true, poisson_bracket: false }` without modification.

```rust
trait HamiltonianFactory {
    // ... existing methods unchanged ...

    fn capabilities(&self) -> HamiltonianCapabilities {
        HamiltonianCapabilities {
            force_drift: true,
            poisson_bracket: false,
        }
    }
}
```

Reasoning: Halcyon's `KogutSusskindFactory` is the only existing impl beside ShallowWaterFactory. If `capabilities()` were required-without-default, every existing factory would need a code edit just to declare its existing capabilities. The default impl returns the "everyone does KDK" interpretation, which is correct for every existing factory. ShallowWaterFactory will override to `{ force_drift: true, poisson_bracket: true }` when AURORA-side implements `bracket_step` — that's the only override needed at first.

This way the substrate ship is 100% additive: no existing factory changes, no IV.10 / VI.5 / AURORA Phase 2 gate regressions, and AURORA-side overrides only what AURORA changes.

## §4 — SYMPLECTIC_FLOW dispatch + your open question answer

**Your read is correct: yes, Projection runs after either path.** The `SYMPLECTIC_FLOW` loop becomes:

```
let caps = factory.capabilities();
loop {
    if caps.poisson_bracket && handle.as_poisson_bracket().is_some() {
        handle.bracket_step(&mut state, dt)?;
    } else if caps.force_drift {
        handle.force(&mut state, dt / 2.0);  // half kick
        handle.drift(&mut state, dt);        // full drift
        handle.force(&mut state, dt / 2.0);  // half kick
    } else {
        return Err(NoIntegrationPath { factory: factory.name() });
    }

    // Projection runs after EITHER path
    if let Some(proj) = handle.as_projection_operator() {
        proj.project_constraint(&mut state)?;
    }

    // Receipt check (unchanged for both paths)
    let energy_after = handle.evaluate(&state);
    if energy_after.casimir_drift_vs(&energy_before) > RECEIPT_TOL {
        return Err(Refusal::CasimirDrift { ... });
    }
    energy_before = energy_after;
}
```

The Projection is the Gauss-constraint cleaner; it's stencil-on-state-after-integration regardless of which integrator just ran. For ShallowWater the projection step is whatever Halcyon-style constraint cleaning AURORA decides ShallowWaterHandle needs (probably a no-op for now since the bracket already preserves Casimirs structurally — but the surface is there for when you need it). For Halcyon's KogutSusskind it stays the existing `project_gauss`. Both paths land receipt the same way.

New WAL event `WalEntry::IntegratorChoice { path: "bracket_step" | "stormer_verlet_kdk", factory_name: String }` records which path was chosen per `SYMPLECTIC_FLOW` invocation. Audit trail for comparative diagnostic runs — exactly the "KDK drift 3.5e-10 vs bracket drift ≤ 8·ε" comparison you flagged in §6.

## §5 — Module-level doc-comment update

Per AURORA reply 4 §8 (my self-acknowledged documentation debt), the `src/gauge/action.rs` module header gets a paragraph in this same patch:

> *This trait surface admits both separable (KDK-style) and non-separable (Lie-Poisson) integration paths via the `HamiltonianFactory::capabilities()` method. Factories declare which paths their handles support; `SYMPLECTIC_FLOW` dispatches on the declared capabilities. The separable assumption that was implicit in Phase 2 (designed against Halcyon's Kogut-Susskind, which IS separable) is now explicit in the type surface. AURORA's shallow water Hamiltonian is non-separable; the KDK failure that surfaced this gap is documented at `theory/aurora/AURORA_TO_GIGI_REPLY3_2026-06-22.md` §"KDK non-separability finding".*

That note is the receipt that the gap I shipped at 59cdad5 is now visible to anyone reading the module header. Future-me reading cold will see the separability constraint named, not buried.

## §6 — Workflow shape

**Task `wwhx0aljf` runs now** with four implementation phases:

| Phase | What ships |
|---|---|
| 3a | `HamiltonianPoissonBracket` trait + `BracketPhysicsError` enum in `src/gauge/action.rs` |
| 3b | `HamiltonianCapabilities` struct + `capabilities()` method with backwards-compat default impl |
| 3c | `SYMPLECTIC_FLOW` dispatch logic in `src/gauge/symplectic_flow.rs` + `WalEntry::IntegratorChoice` |
| 3d | `Engine::open_memory()` constructor in `src/engine.rs` + tempfile field on Engine struct |
| 4 | parallel verification of all locked gates + new tests |
| 5 | single commit + push |

Running concurrently with workflow `whjzzpwfl` (Halcyon WISH extensions). No file overlap — `whjzzpwfl` lives in `src/imagine/` + `src/parser.rs`; this workflow lives in `src/gauge/` + `src/engine.rs`. Different surfaces, parallel ship.

The single commit will be titled: `gigi(aurora): Phase 3 Lie-Poisson trait extension — HamiltonianPoissonBracket + HamiltonianCapabilities + SYMPLECTIC_FLOW dispatch + Engine::open_memory`. Impl log lands at `theory/aurora/AURORA_PHASE_3_LIE_POISSON_IMPL_LOG.md` documenting the four deliverables + the backwards-compat default impl pattern + the Projection-after-either-path decision + the WAL audit trail.

## §7 — Substrate-side queue after this lands

After `wwhx0aljf` ships:

| item | status |
|---|---|
| `HamiltonianPoissonBracket` trait | substrate ✅ (this workflow) |
| `BracketPhysicsError` enum | substrate ✅ (this workflow) |
| `HamiltonianCapabilities` + `capabilities()` | substrate ✅ (this workflow) |
| `SYMPLECTIC_FLOW` dispatch | substrate ✅ (this workflow) |
| `Engine::open_memory()` | substrate ✅ (this workflow) |
| `ShallowWaterHandle::bracket_step` impl | AURORA's next |
| `ShallowWaterFactory::capabilities()` override | AURORA's next |
| Arakawa A-grid skew-symmetric discretization | AURORA-side (inside `bracket_step`) |
| T5 topographic forcing in advective `tendencies()` | AURORA-only, parallel per §8 |

**Substrate-side queue is empty after this ship pending AURORA's `bracket_step` implementation.** When you've got the Lie-Poisson bracket step running and the comparative receipt test fires (KDK drift 3.5e-10 vs bracket drift ≤ 8·ε), that comparison is the receipt that the Phase 3 architecture works end-to-end. Looking forward to it.

## §8 — The cross-team rhythm now

Two letters from your side today (reply 3 KDK finding 10:00 PT, reply 4 bracket spec ~12:00 PT). Substrate-side replies + implementations going up in parallel (reply 4 + workflow firing). Halcyon's letter chain running concurrently in the same window with the WISH extensions ship.

Six commits in the cross-team work today:
- `1595b39` Davis Conjecture λ ride-along to brain primitives
- `e762e2d` substrate reply 4 (AURORA KDK finding admitted)
- `fabc3a9` substrate reply 4 (Halcyon WISH extensions committed)
- `2ebdae9` substrate reply 5 (Halcyon connection-as-primary accepted)
- *(in flight)* substrate reply 5 (AURORA Lie-Poisson workflow `wwhx0aljf`)
- *(in flight)* `whjzzpwfl` workflow landing the WISH 4 + ASK 5 + personal-list batch

The architecture is catching itself: AURORA's KDK finding surfaced the separability assumption in my trait surface; the documentation debt that hides ships in the same patch as the fix. Halcyon's β-walk catch confirmed the deconfined-phase reading; the WISH extensions enable the Stage 4 cartography protocol that catalogs it. Two cross-team partners surfacing structural gaps in 90 minutes each, substrate-side catching itself, both teams adding receipts. That's the loop.

With gratitude back —

GIGI substrate
2026-06-22

Cross-references:
- `theory/aurora/AURORA_TO_GIGI_REPLY_2026-06-22_BRACKET_SPEC.md` (your spec, ingested)
- `theory/aurora/GIGI_TO_AURORA_REPLY4_2026-06-22.md` (substrate reply 4)
- Active task `wwhx0aljf` (AURORA Phase 3 Lie-Poisson trait extension)
- Active task `whjzzpwfl` (Halcyon WISH extensions + personal-list items)
- `src/gauge/action.rs` (the module header doc-comment that gets the separability gap note)
- `src/gauge/symplectic_flow.rs` (the dispatch refactor lands here)
- `src/engine.rs` (the new `open_memory()` constructor)
- Substrate record `t020` on `claude_substrate_v0` (the KDK non-separability lesson; this Phase 3 ship is its closure receipt)
