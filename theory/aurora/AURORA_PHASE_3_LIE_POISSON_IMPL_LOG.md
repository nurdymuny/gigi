# AURORA Phase 3 — Lie-Poisson Trait Extension Implementation Log

**Date:** 2026-06-22
**Author:** Bee (GIGI engine side)
**Incoming spec:** [`AURORA_TO_GIGI_REPLY_2026-06-22_BRACKET_SPEC.md`](./AURORA_TO_GIGI_REPLY_2026-06-22_BRACKET_SPEC.md)
**Phase 2 root commit:** `59cdad5` (HamiltonianFactory + HamiltonianForce + HamiltonianDrift + ProjectionOperator + EnergyDecomposition + HamiltonianHandle + HamiltonianRegistry)
**Davis Conjecture lambda ride-along baseline:** `1595b39` (must not regress; verified 25/0)

## 1. What Rory's Spec Asked For

The AURORA team's Phase 2 retrospective produced one critical finding:

> KDK non-separability finding (AURORA reply 3, 2026-06-22): Stormer-Verlet on
> ShallowWater produces **7x WORSE** Casimir drift than forward Euler. Shallow
> water Hamiltonian is non-separable; KDK fails by CONSTRUCTION (not truncation).
> Lie-Poisson integrator is the structure-preserving alternative.

Rory's reply answered my §5 Q1-back (Option A vs Option B for the bracket
contract) and Q2-back (capability dispatch shape), then handed back a concrete
trait surface.

### §4 — The Separability Gap

Stormer-Verlet KDK assumes the Hamiltonian splits as `H(q,p) = T(p) + V(q)`,
making kick/drift sub-steps individually integrable. Non-separable Hamiltonians
(shallow water, full Yang-Mills with gauge-coupled momentum) cannot be expressed
in that form. KDK applied anyway accumulates `O(dt)` Casimir drift instead of
`O(dt^2)` — worse than forward Euler in the AURORA experiment.

The structure-preserving alternative for non-separable systems is a Lie-Poisson
integrator that advances state via the Poisson bracket directly:
`dF/dt = {F, H}`. The substrate cannot author the bracket — the bracket
structure is system-specific (shallow water is `so(3)*`-style; Yang-Mills is
gauge-group-valued). But the substrate CAN expose a uniform contract for
consumers to plug in their own bracket-preserving step.

### §5 — HamiltonianPoissonBracket Trait (verbatim)

```rust
/// Lie-Poisson bracket integrator for non-separable Hamiltonians.
/// Stability: EVOLVING until gigi 0.1.0 tag.
pub trait HamiltonianPoissonBracket: HamiltonianHandle {
    /// Advance state by one Lie-Poisson bracket step of duration dt.
    ///
    /// Consumer owns the bracket-preserving integration (skew-symmetric
    /// vorticity flux, consistent divergence, vector-invariant momentum).
    /// Substrate calls this in the SYMPLECTIC_FLOW loop, then checks receipt.
    ///
    /// Returns Err for physics invalidity (negative depth, CFL breach).
    /// Does NOT check Casimir drift — that is the substrate's receipt responsibility.
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

Rory's Q1-back answer: **Option B (state-mutating step, substrate orchestrates).**
The bracket step returns `Err` for PHYSICS INVALIDITY only; Casimir drift is the
substrate's receipt responsibility (compare `evaluate()` before/after the step
against `RECEIPT_TOL` — same referee contract as the existing Euler path).

### §6 — HamiltonianCapabilities (verbatim)

```rust
pub struct HamiltonianCapabilities {
    pub force_drift:      bool,   // Stormer-Verlet KDK (Force + Drift path)
    pub poisson_bracket:  bool,   // Lie-Poisson (bracket_step path)
}
```

Plus a `capabilities()` method on `HamiltonianFactory` with a default
returning `{ force_drift: true, poisson_bracket: false }` so existing
factories don't break.

## 2. What This Commit Ships

Four deliverables, all on top of the AURORA Phase 2 trait surface at `59cdad5`,
all gated by the existing receipt + WAL infrastructure.

### D1 — `HamiltonianPoissonBracket` Trait + `BracketPhysicsError` Enum

Added to `src/gauge/action.rs` as two pure additions:

- `BracketPhysicsError` — three-variant enum (`NegativeDepth { i, j, h }`,
  `CflViolation { courant, max_courant }`, `Other(String)`); derives
  `Debug + Clone + PartialEq` (no `Eq` — f64 fields); hand-written
  `Display` + `std::error::Error` impls matching the lowercase
  "subsystem: detail" convention used by `FactoryError`/`EnergyError`/
  `ProjectionError`. No `thiserror` dependency added.
- `HamiltonianPoissonBracket` — sub-trait of `HamiltonianHandle`, carries
  the `EVOLVING` stability marker, references the AURORA reply 3 finding
  in its doc-comment, and explicitly documents that Casimir drift detection
  is the substrate's receipt responsibility (not the trait's).

### D2 — `HamiltonianCapabilities` + `capabilities()` Default Method

Added to `src/gauge/action.rs`:

- Module-level header doc-comment gained a "Separability gap (Phase 3)"
  section explaining why KDK assumes separable `H(q,p) = T(p) + V(q)`,
  why ShallowWater fails by construction (the 7x worse Casimir drift),
  and how `capabilities()` dispatches between the KDK and Lie-Poisson paths.
- `HamiltonianCapabilities` struct — `Debug + Clone + Copy + PartialEq + Eq`,
  two pub bool fields, `EVOLVING` stability marker.
- `capabilities()` default method on `HamiltonianFactory` returning
  `HamiltonianCapabilities { force_drift: true, poisson_bracket: false }` —
  the byte-identical contract every pre-Phase-3 factory already satisfies
  via `HamiltonianForce + HamiltonianDrift`.

**The backwards-compat receipt:** all existing `HamiltonianFactory` impls
inherit the default. KogutSusskindFactory (and every other downstream factory
that may exist after this lands) keeps working WITHOUT modification. AURORA's
ShallowWaterFactory will override to `{ force_drift: true, poisson_bracket: true }`
— keeping `force_drift: true` preserves the A17/A18 diagnostic gates and
allows comparative receipt tests (KDK drift 3.5e-10 vs bracket drift
≤ 8 × machine_eps).

Object-safety preserved: `capabilities()` takes `&self`, returns a plain
`Copy` struct, no associated types / generics / `Self` in return position.
`Box<dyn HamiltonianFactory>` stays constructible.

### D3 — SYMPLECTIC_FLOW Dispatch on Capabilities

**Critical design decision:** the existing SU(2) Kogut-Susskind
`symplectic_flow(u_name, e_name, ...)` function in
`src/gauge/symplectic_flow.rs` was NOT modified. It remains byte-identical,
which preserves the IV.10 (4/0 + 1 ignored) and VI.5 (3 gates ignored by
design) gold-gate kill criteria.

Instead, a NEW capability-dispatched function `symplectic_flow_dispatch` was
added that operates on the group-erased AURORA Phase 2 trait surface
(`factory_name` + `params` + state `Vec<f64>`), reads `factory.capabilities()`
from the `hamiltonian_registry`, and picks `IntegratorPath::LiePoissonBracket`
/ `IntegratorPath::StormerVerletKdk` / errors with `NoIntegrationPath`.

**Dispatch logic in `pick_integrator_path()`:**

| factory.capabilities()                       | handle.as_poisson_bracket() | path                            |
| -------------------------------------------- | --------------------------- | ------------------------------- |
| `{ poisson_bracket: true,  force_drift: _ }` | `Some(_)`                   | `LiePoissonBracket`             |
| `{ poisson_bracket: true,  force_drift: true }` | `None`                   | `StormerVerletKdk` (lie-safety) |
| `{ poisson_bracket: false, force_drift: true }` | _                        | `StormerVerletKdk`              |
| `{ poisson_bracket: true,  force_drift: false }` | `None`                  | `NoIntegrationPath`             |
| `{ poisson_bracket: false, force_drift: false }` | _                       | `NoIntegrationPath`             |

The "capability lies" row is the safety net: a factory declaring
`poisson_bracket: true` but providing a handle whose `as_poisson_bracket()`
returns `None` falls back to KDK if `force_drift: true` is also declared,
or errors out cleanly otherwise.

**The Projection-runs-after-either-path decision (Rory's open Q answer):**
Projection is the Gauss constraint cleaner, independent of integrator choice.
The SYMPLECTIC_FLOW loop becomes:

```
if poisson_bracket: bracket_step -> (optional) project_constraint -> receipt
if force_drift:     kick -> drift -> kick -> (optional) project_constraint -> receipt
```

The receipt contract is unchanged: compare `evaluate()` at end vs start; if
Casimir drift > `RECEIPT_TOL`, emit `Refusal`.

The load-bearing backward-compat lever is a new `as_poisson_bracket(&self) ->
Option<&dyn HamiltonianPoissonBracket>` default method on `HamiltonianHandle`
that returns `None` by default. Every existing handle keeps compiling
unmodified; only handles that implement `HamiltonianPoissonBracket` override
to return `Some(self)`.

### WAL Audit Trail — `IntegratorChoice`

A new WAL event records which integrator path was selected at each
`symplectic_flow_dispatch` invocation:

- `OP_INTEGRATOR_CHOICE = 0x0D` (gated on `feature = "gauge"`) in `src/wal.rs`
- `WalWriter::log_integrator_choice(...)` writer method
- `WalEntry::IntegratorChoice { ... }` reader variant
- Engine replay arm in `src/engine.rs`: `WalEntry::IntegratorChoice { .. } => {}`
  (audit-only, replay does not re-execute — same policy as `HamiltonianDeclare`)

Append-only: pre-Phase-3 WAL bundles replay unchanged (no `IntegratorChoice`
events; the entry is skip-unknown forward-compat). Encoding mirrors the
existing `OP_HAMILTONIAN_DECLARE` pattern exactly: length-prefixed UTF-8
strings, op byte `0x0D`.

The IntegratorChoice WAL writer accepts an `Option<&mut WalWriter>` on
`symplectic_flow_dispatch` (`None` means audit-trail-off, matching test
ergonomics). Production callers (HTTP path, host binary) supply
`Some(writer)` and get the audit event. The event is emitted once per
invocation, after dispatch resolves and before the integrator loop opens —
never per step.

### D4 — `Engine::open_memory()` Constructor

`src/engine.rs` gained:

- `_tempdir: Option<tempfile::TempDir>` as the LAST field on `pub struct Engine`
  (Windows file-handle correctness via field-drop-order).
- `Engine::open_memory() -> io::Result<Self>` — creates a fresh tempdir,
  opens an empty engine inside it, holds the `TempDir` for the engine's
  lifetime, cleans up via `Drop`. ~16 LOC.
- Existing struct-literal returns in `open_inner` and `open_mmap` gained
  `_tempdir: None,` — existing callers see zero behavior change.

`tempfile = "3"` was promoted from `[dev-dependencies]` to `[dependencies]`
in `Cargo.toml`. Build cost negligible — `tempfile` was already in the
compile graph for every test run.

Suitable for dev, CI, tests, and aurora-server-style host binaries that
have no persistence needs.

## 3. Backwards-Compat Default-Impl Pattern

The single most important load-bearing design choice in this commit is the
**default impl of `capabilities()` returning `{ force_drift: true,
poisson_bracket: false }`**. This makes the entire Phase 3 extension a pure
addition:

1. Every pre-Phase-3 `HamiltonianFactory` impl in the tree (or in any
   downstream consumer) inherits the default automatically.
2. The default matches the legacy contract: existing factories provide
   `HamiltonianForce + HamiltonianDrift`, so `force_drift: true` is the
   honest declaration; they do NOT provide `HamiltonianPoissonBracket`,
   so `poisson_bracket: false` is the honest declaration.
3. `SYMPLECTIC_FLOW` dispatch picks `StormerVerletKdk` for any factory
   reporting `{ force_drift: true, poisson_bracket: false }`, which is
   exactly the existing behavior.

Same pattern as the `as_poisson_bracket() -> Option<&dyn ...>` default-None
on `HamiltonianHandle` — handles opt in by overriding, never by changing
existing trait bounds.

## 4. AURORA's Next Step

The Phase 3 trait surface is now stable enough for AURORA to implement on
their side:

1. Implement `ShallowWaterHandle: HamiltonianHandle + HamiltonianForce
   + HamiltonianDrift + HamiltonianPoissonBracket`.
   - `bracket_step` carries the skew-symmetric vorticity flux,
     consistent divergence, and vector-invariant momentum that the
     shallow water Poisson structure requires.
   - `as_poisson_bracket(&self) -> Option<&dyn HamiltonianPoissonBracket>`
     overrides to `Some(self)`.
2. Override `ShallowWaterFactory::capabilities()` to return
   `HamiltonianCapabilities { force_drift: true, poisson_bracket: true }`.
3. Keeping `force_drift: true` preserves A17/A18 diagnostic gates and
   enables the comparative receipt test (KDK drift 3.5e-10 vs bracket
   drift ≤ 8 × machine_eps) — the canonical proof that the Phase 3 path
   solves the §4 separability gap.

Substrate side: nothing more needed until AURORA exercises both paths
in production and the receipt envelope tightens.

## 5. Verification (kill criteria)

| Gate                                                            | Result                              |
| --------------------------------------------------------------- | ----------------------------------- |
| `cargo test --no-default-features --lib`                        | 870/0 (byte-identical baseline)     |
| `cargo test --features kahler --lib`                            | 1150/0                              |
| `cargo test --features halcyon --lib`                           | 1031/0                              |
| `cargo test --features halcyon --test halcyon_part_iv_gold`     | 4/0 + 1 ignored (IV.10 byte-identical) |
| `cargo test --features kahler --test davis_conjecture_lambda_brain_ridealong` | 25/0 (1595b39 ride-along intact) |
| `cargo test --features halcyon --test aurora_lie_poisson_trait` | 12/0 (D1+D2+D3 trait surface)       |
| `cargo test --no-default-features --test engine_open_memory`    | 6/0 (D4 constructor)                |

All kill criteria for this workflow are met. The 1595b39 Davis Conjecture
lambda-budget ride-along is unchanged (no modification to `src/curvature.rs`).
The Kogut-Susskind SU(2) symplectic flow is byte-identical (no modification
to the existing `symplectic_flow` function — the new `symplectic_flow_dispatch`
is a parallel entry point operating on the group-erased trait surface).

## 6. Cross-References

- Spec: [`AURORA_TO_GIGI_REPLY_2026-06-22_BRACKET_SPEC.md`](./AURORA_TO_GIGI_REPLY_2026-06-22_BRACKET_SPEC.md)
- Phase 2 trait surface root: `59cdad5`
- Davis Conjecture lambda ride-along baseline: `1595b39`
- KDK non-separability finding: AURORA reply 3 (2026-06-22)
- AURORA Phase 2 trait surface tests:
  `tests/aurora_phase_2_trait_surface.rs`,
  `tests/aurora_phase_2_hamiltonian_registry.rs`
- New trait surface tests: `tests/aurora_lie_poisson_trait.rs`
- New Engine constructor tests: `tests/engine_open_memory.rs`
- Modified source: `src/gauge/action.rs`, `src/gauge/error.rs`,
  `src/gauge/symplectic_flow.rs`, `src/engine.rs`, `src/wal.rs`, `Cargo.toml`

## 7. What This Commit Does NOT Touch

Active in parallel workflow `whjzzpwfl` (Halcyon + WISH extensions), explicitly
out of scope here:

- `src/imagine/wish.rs`
- `src/imagine/provenance.rs`
- `src/parser.rs`
- `src/gauge/loop_transport.rs`
- `src/gauge/wilson_force.rs`
- `src/gauge/project_gauss.rs`
- `src/gauge/holonomy.rs`
- `src/curvature.rs` (1595b39 ride-along — must not regress; verified intact)

Deferred per Rory's §8:

- Buckyball SU(2) `WishBundle` impl (Halcyon side, workflow `whjzzpwfl`)
- GIBBS_SAMPLE auto-correlation investigation (separate workflow)
- `tau_pin` documentation paragraph in `loop_transport.rs` (workflow `whjzzpwfl`)
- T5 topographic forcing in AURORA tendencies (AURORA-only, parallel)
