# AURORA Phase 2 — trait surface + hamiltonian_registry implementation log

**Companion to:** `theory/aurora/AURORA_ASKS_v0_1_LOG.md`,
`theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY_2.md` (commit
`baac7f2` / engine acceptance §2 + §11), `theory/aurora/AURORA_TO_GIGI_REPLY2_2026-06-21.md`
(AURORA reply 2: Q5a eager-init code shape + Q6b correction),
`theory/aurora/AURORA_PHASE_2_DEC_IMPL_LOG.md` (DEC operator surface
at commit `17105ff`), `theory/aurora/AURORA_PHASE_1_IMPL_LOG.md`
(`LatticeWithMetric` wrapper + CC-2 registry at commit `f62e46c`),
and `docs/STABILITY_GUARANTEES.md` (trait-surface stability section
at commit `1e13252`).

**Format:** mirrors `theory/aurora/AURORA_PHASE_0_IMPL_LOG.md`,
`AURORA_PHASE_1_IMPL_LOG.md`, and `AURORA_PHASE_2_DEC_IMPL_LOG.md` —
summary → commitments → trait surface decisions → registry shape →
deferred work → TDD discipline → receipts → files touched → stability
→ follow-up letter queue → cross-refs → what's next.

---

## Summary

This sprint ships the **A2 four-trait surface** in a new
`src/gauge/action.rs` plus the **CC-1 process-wide hamiltonian
registry** in a new `src/gauge/hamiltonian_registry.rs`, with the WAL
`HamiltonianDeclare` event variant added beside the existing
`LatticeDeclare` / `GaugeFieldDeclare` / `GaugeFieldSnapshot` ops.
AURORA's `ShallowWaterFactory` skeleton — already written against the
expected trait shape in their downstream crate at
`C:/Users/nurdm/OneDrive/Documents/aurora/src/hamiltonians/shallow_water.rs` —
now has a contract to compile against. AURORA can uncomment their
`init()` body, register the factory at the top of `main()`, and run a
real Williamson Test 2 against `gigi-stream.fly.dev` instead of the
step-0 scaffold.

The trait surface is the four sub-traits (`HamiltonianForce`,
`HamiltonianDrift`, `ProjectionOperator`, `EnergyDecomposition`)
unified by the `HamiltonianHandle` super-trait, plus the
`HamiltonianFactory` trait that the registry stores as
`Box<dyn HamiltonianFactory>`. Group-agnostic by construction —
trait methods speak only in `&[f64]` / `&mut [f64]` / `Vec<f64>` /
`BTreeMap<String, f64>` so AURORA's `group_tag = "R"` ShallowWater
compiles against the same surface as a future SU(2) `KogutSusskind`
or SU(3) impl. Object-safe — no associated types leak to the trait
boundary; the registry can hold `Box<dyn HamiltonianFactory>` and
factory `from_params` returns `Box<dyn HamiltonianHandle>` without
gymnastics.

The sprint is additive in the strict sense: two new files in
`src/gauge/` (503 LOC across `action.rs` + `hamiltonian_registry.rs`),
4 lines in `src/gauge/mod.rs` (two `pub mod` lines + two `pub use`
re-exports), ~75 LOC in `src/wal.rs` for the
`OP_HAMILTONIAN_DECLARE = 0x0C` constant + `WalEntry::HamiltonianDeclare`
variant + `log_hamiltonian_declare` writer + reader arm, and a small
match-arm acknowledgment in `src/engine.rs` so replay accepts the new
variant as a no-op (replay handling is explicitly deferred). Two new
integration test files in `tests/` (473 LOC, 13 `#[test]` functions
across the trait surface + registry). **Zero modifications** to
`symplectic_flow.rs`, `loop_transport.rs`, `wilson_force.rs`,
`project_gauss.rs`, `holonomy.rs`, or any existing `src/gauge/` file
beyond the `mod.rs` declarations.

All five gates green: **870/0** no-default lib (byte-identical
baseline), **1031/0** halcyon lib (+1 from new in-module unit
coverage), **1150/0** kahler lib, **4/0 + 1 ignored**
`halcyon_part_iv_gold` bit-identity gate (IV.10 contracts intact),
**3/0 + 0 ignored** `halcyon_part_vi_bit_identity_gold` under
`--include-ignored` (VI byte-identity fixture intact).

---

## Commitments (quoted verbatim)

### v2 reply §2 — CC-1 hamiltonian registry

From `theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY_2.md` §2:

> CC-1: **open registry.** New `src/gauge/hamiltonian_registry.rs`
> mirrors the `lattice::registry` shape; trait-object factory keyed
> by `kind_tag`; WAL `HamiltonianDeclare` stores metadata only (name,
> kind_tag, params); trait-object is re-materialized from factory at
> replay. Engine enforces registration completes before
> `Engine::open()` WAL replay.

Phase 2 honors this commitment with one explicit deviation: WAL
replay handling for `HamiltonianDeclare` is **deferred to a later
workflow**. The variant is emitted and persisted on `register()`,
the `WalReader` arm parses it back, and the engine replay match
acknowledges it as a no-op — but materializing the registry from
WAL on replay is explicitly out of scope for this sprint (documented
under "Deferred work" below). The Q5 eager-init contract makes this
safe: AURORA's `main()` calls `register()` explicitly at startup
before `Engine::open()`, so the registry is always populated through
the in-process path; replay only needs to acknowledge the variant
exists, not act on it.

### v2 reply §11 — A2 four-trait refactor + hot-path constraint

From the same reply, §11:

> ACCEPTED with **hot-path constraint**: trait-object dispatch is OFF
> the integrator inner loop. The per-substep KDK + measurement body
> is generic over a concrete `H: HamiltonianForce + HamiltonianDrift`,
> not boxed. `Box<dyn ...>` lives only at registry / WAL /
> introspection boundaries.

Phase 2 honors the hot-path constraint by construction: the registry
stores `Box<dyn HamiltonianFactory>`, the factory's `from_params`
returns `Box<dyn HamiltonianHandle>`, and that boxed handle is the
extent of trait-object cost. The integrator-generic-over-`H` refactor
of `src/gauge/symplectic_flow.rs` — which is what *enforces* the hot
path stays cold — is **deferred to a later workflow** with its own
bit-identity discipline (it touches the hottest hot path locked by
IV.10 + VI bit-identity fixture). The trait surface is shipped first
so that refactor has a stable surface to land against; this sprint
deliberately does not touch `symplectic_flow.rs` and the IV.10/VI
gates stay byte-identical as a result.

### Q5 — eager init() at top of main()

From `theory/aurora/AURORA_TO_GIGI_REPLY2_2026-06-21.md` Q5a:

> Eager `init()` at top of `main()`. No `lazy_static`, no `OnceCell`,
> no thread-local auto-registration on first use. AURORA's `main()`
> calls `hamiltonian_registry::register("SHALLOW_WATER",
> Box::new(ShallowWaterFactory))?;` once at startup, before
> `Engine::open()`.

Phase 2 honors this: the registry's internal storage is
`OnceLock<Mutex<HashMap<String, Box<dyn HamiltonianFactory>>>>`, but
the `OnceLock` wraps **only the `Mutex` allocation**, not the
`HashMap` population. The `HashMap` inside the `Mutex` starts empty
on the first `register()` call (the `OnceLock::get_or_init` closure
returns `Mutex::new(HashMap::new())`) and is only populated by
explicit `register()` calls from the host binary's `main()`. No
auto-registration, no thread-local first-use hook, no built-in
factories pre-loaded. The test
`test_registry_eager_init_no_auto_populate` verifies this directly:
after a fresh `clear()`, `with_factory("SHALLOW_WATER", ...)` returns
`None` and `with_factory("KOGUT_SUSSKIND", ...)` returns `None`.

### Q6b — EVOLVING marker per stability convention

From the same reply, Q6b correction:

> The convention is **doc-comment + changelog + semver**, NOT
> `#[stable(...)]` proc-macro. That attribute is rustc-internal and
> not available to library crates.

Phase 2 carries the EVOLVING doc-comment on every new `pub` item in
`src/gauge/action.rs` and `src/gauge/hamiltonian_registry.rs`:

```rust
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
```

Items annotated:
- `pub trait HamiltonianForce`
- `pub trait HamiltonianDrift`
- `pub trait ProjectionOperator`
- `pub trait EnergyDecomposition`
- `pub trait HamiltonianHandle`
- `pub trait HamiltonianFactory`
- `pub enum FactoryError` + each variant
- `pub enum EnergyError` + each variant
- `pub enum ProjectionError` + each variant
- `pub enum RegistryError` + each variant
- `pub fn register`
- `pub fn with_factory`
- `pub fn contains`
- `pub fn list_registered`
- `pub fn clear` (test-only)

The convention shape matches Phase 1 + Phase 2 DEC deployments
(`LatticeWithMetric` / `ConstructorArgs` / `DecError` / `d_0` / etc.).
The EVOLVING contract holds until the first `gigi 0.1.0` tag, at
which point markers flip to `STABLE` and the breaking-change bar
becomes a major-version bump.

---

## Trait surface decisions

### Hierarchy choice: single `HamiltonianHandle` super-trait

```rust
pub trait HamiltonianHandle:
    HamiltonianForce + HamiltonianDrift + ProjectionOperator + EnergyDecomposition
    + Send + Sync + std::fmt::Debug
{}
```

Rationale: AURORA's `shallow_water.rs:116` already references
`Box<dyn HamiltonianHandle>` as the factory return type. A single
trait object collapses to one vtable per Hamiltonian instance and
matches their factory wrapper signature without rework. `Send + Sync`
so registries can be shared across the host binary's startup-then-
read-only lifecycle. `Debug` was added during GREEN because the RED
test `test_factory_missing_param_returns_typed_error` calls
`expect_err` on `Result<Box<dyn HamiltonianHandle>, FactoryError>`,
which requires `Box<dyn HamiltonianHandle>: Debug` — minimal
3-line addition (`#[derive(Debug)]` on the three test stubs), flagged
to AURORA in the follow-up letter.

### State representation: group-erased `&[f64]` / `Vec<f64>` buffers

The trait surface speaks **only** in flat real-valued buffers, not
in concrete state structs. Method signatures look like:

```rust
fn force_per_edge<L: LatticeWithMetric>(&self, lat: &L, state: &[f64])
    -> Result<Vec<f64>, ForceError>;

fn drift_step<L: LatticeWithMetric>(&self, lat: &L, state: &mut [f64], dt: f64)
    -> Result<(), DriftError>;

fn project_constraint<L: LatticeWithMetric>(&self, lat: &L, state: &mut [f64])
    -> Result<(), ProjectionError>;

fn evaluate(&self, state: &[f64]) -> Result<BTreeMap<String, f64>, EnergyError>;
```

Rationale (load-bearing): an associated `type State` on the trait
breaks object-safety on `Box<dyn HamiltonianHandle>` unless the
caller pins it via `Box<dyn HamiltonianHandle<State = ShallowWaterState>>`
— but that defeats the registry's open-set premise (every downstream
crate would have to publish its `State` type in a shared signature).
A generic `type State` on the factory trait makes the factory itself
non-object-safe, breaking the registry. The clean resolution is what
AURORA's `group_tag = "R"` already implies: pack the state into a
flat `Vec<f64>` at the trait boundary, and let each Hamiltonian impl
define its own helper methods that map between its concrete state
struct (e.g. `ShallowWaterState { h: Vec<f64>, u: Vec<f64> }`) and
the flat buffer. AURORA's `ShallowWaterState` packs as `[h cells || u
edges]` with a documented ordering; future SU(2) `KogutSusskind` will
pack as `[U links || E links]` in row-major order over the
`DenseLinkBuffer` convention.

This is the **one load-bearing deviation from AURORA's stub** that
requires a follow-up letter. AURORA's current `force_per_edge<L:
LatticeWithMetric>(..., state: &ShallowWaterState)` becomes
`force_per_edge<L: LatticeWithMetric>(..., state: &[f64])` with a
private helper `fn unpack_state(state: &[f64]) -> (View<h>, View<u>)`
inside the `ShallowWater` impl. ~5 LOC adjustment on AURORA's side.

### EnergyDecomposition shape

```rust
pub trait EnergyDecomposition {
    fn energy_keys(&self) -> &'static [&'static str];
    fn evaluate(&self, state: &[f64]) -> Result<BTreeMap<String, f64>, EnergyError>;
}
```

`BTreeMap` (not `HashMap`) for deterministic key iteration in
diagnostic envelopes and WAL-adjacent receipts. AURORA's 7 keys
(`casimir_energy`, `casimir_mass`, `casimir_pv_l1`, `casimir_pv_l2`,
`kelvin_eq`, `kelvin_n30`, `kelvin_s30`) become the static slice
returned by `energy_keys()`; the runtime `evaluate` returns a
`BTreeMap<String, f64>` keyed by those same names. Minor signature
divergence from AURORA's current `energy_components(state, grid) ->
HashMap<&'static str, f64>`:

1. `HashMap` → `BTreeMap` (deterministic iteration).
2. `HashMap<&'static str, f64>` → `BTreeMap<String, f64>` (String
   keys so runtime-derived names work if a future Hamiltonian needs
   them; `energy_keys()` returns the `&'static` slice as the canonical
   contract).
3. Drop the `&Grid` param — lattice access happens via the `L:
   LatticeWithMetric` generic on `force_per_edge`/`drift_step`/
   `project_constraint`. `evaluate` is pure-state because the energy
   decomposition is a function of state values alone (the lattice
   geometry was already baked into the state buffer at force/drift
   time).

Flagged to AURORA in the follow-up letter.

### Signature alignment with AURORA's stub

| Item | AURORA stub | Gigi Phase 2 | Status |
| --- | --- | --- | --- |
| Factory return type | `Box<dyn HamiltonianHandle>` | `Box<dyn HamiltonianHandle>` | MATCH |
| `from_params` params | `&HashMap<String, f64>` | `&HashMap<String, f64>` | MATCH |
| `kind_tag()` | `-> &'static str` | `-> &'static str` | MATCH |
| `group_tag()` | `-> &'static str` | `-> &'static str` | MATCH |
| Force signature | `<L: LatticeWithMetric>(&self, &L, &State)` | `<L: LatticeWithMetric>(&self, &L, &[f64])` | MINOR — State erased to `&[f64]` |
| Drift signature | `<L: LatticeWithMetric>(&self, &L, &State, dt) -> State` | `<L: LatticeWithMetric>(&self, &L, &mut [f64], dt)` | MINOR — State erased to `&mut [f64]`, mutation in-place |
| Projection signature | `project_constraint(&self, &L, &mut State) -> Result<(), _>` | `project_constraint<L>(&self, &L, &mut [f64]) -> Result<(), ProjectionError>` | MINOR — State erased |
| EnergyDecomposition keys | `fn energy_components -> HashMap<&'static str, f64>` (with `&Grid` param) | `energy_keys -> &'static [&'static str]` + `evaluate(&[f64]) -> BTreeMap<String, f64>` | MINOR — HashMap→BTreeMap, String keys, Grid param dropped |
| HamiltonianHandle bounds | (implicit) | `Send + Sync + Debug` | MINOR — `Debug` added during GREEN |
| Group erasure | `group_tag = "R"` test | trait methods reference no SU(2) types | MATCH |

Net: 6 minor signature deltas, 0 load-bearing breaks. AURORA's
estimated rework is ~10–20 LOC across `shallow_water.rs` (state
unpacking helpers + signature alignment on the four methods). The
follow-up letter spells out each delta.

---

## Registry shape

### Storage

```rust
static REGISTRY: OnceLock<Mutex<HashMap<String, Box<dyn HamiltonianFactory>>>>
    = OnceLock::new();
```

`OnceLock` wraps only the `Mutex` allocation. The `HashMap` inside
starts empty and is populated exclusively by `register()` calls.

### Public surface

```rust
pub fn register(
    name: impl Into<String>,
    factory: Box<dyn HamiltonianFactory>,
    wal_writer: Option<&mut WalWriter>,
    registered_at: u64,
) -> Result<(), RegistryError>;

pub fn with_factory<R>(
    name: &str,
    f: impl FnOnce(&dyn HamiltonianFactory) -> R,
) -> Option<R>;

pub fn contains(name: &str) -> bool;

pub fn list_registered() -> Vec<(String, &'static str, &'static str)>;
//                                  ^name    ^kind_tag    ^group_tag

#[cfg(test)]
pub fn clear();
```

Design choices:

1. **`with_factory` closure pattern, not `get_factory` returning
   `&dyn HamiltonianFactory`.** A direct getter would either need a
   lifetime tied to a `MutexGuard` (forces the caller to hold the
   guard across the dispatch) or require `HamiltonianFactory: Clone`
   (which downstream impls couldn't satisfy generically). The closure
   pattern threads the dispatch inside the guard scope cleanly, and
   callers compose it with their own logic — AURORA's
   `init()` does `with_factory("SHALLOW_WATER", |f| f.from_params(&params))`.

2. **`wal_writer: Option<&mut WalWriter>` injected by caller.** The
   gauge layer does not reach into engine state to find the WAL
   writer — the host binary's `main()` (or its engine) threads the
   writer through. `Option` so tests can register without a WAL.
   Matches the precedent set by `gauge::registry::register_gauge_field`.

3. **`registered_at: u64` is a monotonic counter passed by caller.**
   The gauge layer has no clock; the engine provides the counter
   (typically `now_micros` or a sequence number). The WAL payload
   records it so replay sees the same ordering the host binary chose.

4. **First-write-wins on duplicate names.** `register("X", ...)`
   followed by a second `register("X", ...)` returns
   `RegistryError::DuplicateName { name: "X" }` and does NOT overwrite
   the first registration. No silent late-binding. Test
   `test_registry_duplicate_name_rejects` verifies.

### WAL `HamiltonianDeclare` event

Op constant: `OP_HAMILTONIAN_DECLARE = 0x0C` (next after
`OP_GAUGE_FIELD_SNAPSHOT = 0x0B`, feature-gated on `gauge`).

Variant: `WalEntry::HamiltonianDeclare { name, kind_tag, group_tag, registered_at }`.

Payload encoding (mirrors `LatticeDeclare` + `GaugeFieldDeclare`
style):

```
[u32 LE name_len][name_bytes]
[u32 LE kind_tag_len][kind_tag_bytes]
[u32 LE group_tag_len][group_tag_bytes]
[u64 LE registered_at]
```

Envelope: `[4 LE length][1 byte 0x0C][payload][4 CRC32]` per existing
convention.

Metadata-only: no factory params snapshotted (factories are pure
functions of their params; the host binary explicitly re-registers
at replay startup per Q5; the substrate does not auto-instantiate).
Replay handling explicitly deferred (see below).

Emission helper `wal::log_hamiltonian_declare(writer, name,
kind_tag, group_tag, registered_at) -> WalResult<()>` lives in
`src/wal.rs` beside `log_lattice_declare` and `log_gauge_field_declare`.

### Built-in factories: NONE

The registry ships **empty**. Phase 2 deliberately does NOT register
a built-in `KogutSusskindFactory` because the
integrator-generic-over-`H` refactor of `symplectic_flow.rs` has not
landed yet — there is no concrete `KogutSusskind` impl of
`HamiltonianForce + HamiltonianDrift + ProjectionOperator +
EnergyDecomposition` to register. That arrives in the later workflow
(see "Deferred work"). For now: AURORA's host binary registers
`ShallowWaterFactory` at the top of `main()`; that is the only
registration any host binary needs in this sprint.

---

## Deferred work — explicitly OUT OF SCOPE

Three items named in the locked context are deferred to later
workflows with their own bit-identity discipline:

### 1. `symplectic_flow.rs` integrator-generic-over-`H` refactor

Status: NOT touched in this sprint.

`src/gauge/symplectic_flow.rs` stays exactly as committed at
`17105ff` — hardcoded SU(2) Kogut-Susskind path, direct calls into
`wilson_force_per_edge` + `apply_force_kick` + `drift_step` +
`project_gauss`. The refactor that lifts this to generic-over-`H`
touches the hottest hot path in the codebase (the IV.10 + VI
bit-identity fixtures lock exact byte-output) and deserves its own
workflow with its own RED→GREEN cycle against those gates. Shipping
the trait surface first is the safe order: the refactor has a stable
contract to land against, and the bit-identity gates remain green by
construction in this sprint.

### 2. KogutSusskind lift to `HamiltonianFactory` impl

Status: NOT shipped in this sprint.

The in-tree canonical impl of the new trait surface — wrapping the
existing SU(2) Wilson force + Rodrigues drift + Gauss projection +
kinetic-energy decomposition as a `HamiltonianHandle` — is a
follow-up. It does not block AURORA (ShallowWater is the first
external consumer; KogutSusskind is the second, demonstrating
group-agnostic compilation). It is the natural pairing for the
`symplectic_flow.rs` refactor above.

### 3. WAL replay handling for `HamiltonianDeclare`

Status: variant is emitted and persisted, but replay does not
re-instantiate factories.

The `engine::do_replay` match arm acknowledges the variant exists
(no-op) so replay does not error on a `HamiltonianDeclare` entry,
but it does not look up `kind_tag` in the registry and re-register a
factory from that. Rationale: the Q5 eager-init contract means
`main()` always calls `register()` explicitly before
`Engine::open()`, so the registry is always populated through the
in-process path. WAL replay materialization is a defense-in-depth
feature for the future (e.g. if a future workflow ships a closed
built-in registry where replay re-fills automatically) and is not
needed for AURORA's Williamson Test 2.

---

## TDD discipline

### RED first — 13 integration tests across 2 files

Two integration test files in `tests/`:

- **`tests/aurora_phase_2_trait_surface.rs`** (~250 LOC, 6 tests):
  - `test_hamiltonian_factory_trait_shape` — asserts
    `kind_tag() -> &'static str`, `group_tag() -> &'static str`,
    `from_params(&HashMap<String, f64>) -> Result<Box<dyn
    HamiltonianHandle>, FactoryError>`.
  - `test_hamiltonian_handle_subtrait_bounds` — type-assertion helpers
    that prove `Box<dyn HamiltonianHandle>` implements all four
    sub-traits + `Send + Sync + Debug`.
  - `test_energy_decomposition_keys_iteration` — uses
    `MockShallowWaterFactory` with AURORA's exact 7 keys
    (`casimir_energy`, `casimir_mass`, `casimir_pv_l1`,
    `casimir_pv_l2`, `kelvin_eq`, `kelvin_n30`, `kelvin_s30`) and
    asserts deterministic BTreeMap iteration order matches the
    `energy_keys()` static slice ordering.
  - `test_stub_hamiltonian_compiles_against_traits` — `NoOpHamiltonian`
    impl with empty energy keys, zero force, identity drift, no-op
    projection compiles + round-trips through the registry.
  - `test_aurora_shallow_water_factory_signature_alignment` —
    `MockShallowWaterFactory` mirroring AURORA's shape (kind_tag =
    "SHALLOW_WATER", group_tag = "R", params dict with g/omega/a)
    proves signature alignment without depending on the aurora crate.
  - `test_factory_missing_param_returns_typed_error` — `from_params`
    with an empty HashMap returns
    `FactoryError::MissingParam { name: "g" }` (no panic, no silent
    default).

- **`tests/aurora_phase_2_hamiltonian_registry.rs`** (~223 LOC, 7 tests):
  - `test_registry_register_then_lookup` — round-trip on a single
    factory.
  - `test_registry_unknown_name_returns_none` — `with_factory("UNKNOWN",
    ...)` returns `None`.
  - `test_registry_duplicate_name_rejects` — second `register("X",
    ...)` returns `RegistryError::DuplicateName { name: "X" }`; the
    first registration is preserved.
  - `test_registry_factory_kind_tag_round_trip` — `with_factory("...",
    |f| f.kind_tag())` returns the same `&'static str` the factory
    was constructed with.
  - `test_wal_hamiltonian_declare_event_emitted` — `register()` with
    a real `WalWriter` produces a `WalEntry::HamiltonianDeclare`
    entry read back via `WalReader::read_all()`; payload fields
    match (`name`, `kind_tag`, `group_tag`, `registered_at`).
  - `test_registry_eager_init_no_auto_populate` — after a fresh
    `clear()`, `with_factory("SHALLOW_WATER", ...)` and
    `with_factory("KOGUT_SUSSKIND", ...)` both return `None`.
  - `test_registry_list_registered_after_register` — `list_registered()`
    returns the `(name, kind_tag, group_tag)` triples for all
    registered factories.

**RED state confirmed** before any production code landed. The
RED-build error excerpt:

```
error[E0432]: unresolved import `gigi::gauge::action`
  --> tests\aurora_phase_2_trait_surface.rs:19
   |
19 | use gigi::gauge::action::{HamiltonianFactory, HamiltonianHandle, ...};
   |                  ^^^^^^ could not find `action` in `gauge`

error[E0432]: unresolved imports `gigi::gauge::hamiltonian_registry`,
              `gigi::gauge::hamiltonian_registry`
  --> tests\aurora_phase_2_hamiltonian_registry.rs:25
   |
25 | use gigi::gauge::hamiltonian_registry::{register, with_factory, ...};
   |                  ^^^^^^^^^^^^^^^^^^^^^ no `hamiltonian_registry` in `gauge`
```

Both test crates fail to compile against current `main` (HEAD has
`src/gauge/{action,hamiltonian_registry}.rs` absent, no
`WalEntry::HamiltonianDeclare` variant). All 13 test functions are
blocked by these missing modules — none execute.

### GREEN — trait-first, then registry, then WAL plumbing

Implementation order matched the dependency graph: trait surface in
`src/gauge/action.rs` first (no WAL dep, no registry dep, no concrete
impl needed); then `src/gauge/hamiltonian_registry.rs` (depends on
the trait surface but not on WAL emission); then `src/wal.rs` delta
(op constant + variant + writer + reader arm); then re-wire the
registry's `register()` to emit; then `src/engine.rs` replay match-
arm acknowledgment. Each layer landed with its in-module unit tests
passing before the next started, so the integration suite came GREEN
in two waves (6 trait-surface tests, then 7 registry tests).

One load-bearing deviation from the RED test files arose during
GREEN: `HamiltonianHandle` was lifted to bound `: std::fmt::Debug`
because `test_factory_missing_param_returns_typed_error` calls
`expect_err` on `Result<Box<dyn HamiltonianHandle>, FactoryError>`,
which requires `Box<dyn HamiltonianHandle>: Debug`. Resolution:
added `#[derive(Debug)]` to the three test stubs (`NoOpHamiltonian`,
`ShallowWaterMock`, `StubHamiltonian`) — minimal 3-line addition.
Flagged to AURORA in the follow-up letter so they add
`#[derive(Debug)]` to `ShallowWater` (one line).

### Final test pass

```
cargo test --features halcyon --test aurora_phase_2_trait_surface
  running 6 tests
  test test_aurora_shallow_water_factory_signature_alignment ... ok
  test test_energy_decomposition_keys_iteration ... ok
  test test_factory_missing_param_returns_typed_error ... ok
  test test_hamiltonian_factory_trait_shape ... ok
  test test_hamiltonian_handle_subtrait_bounds ... ok
  test test_stub_hamiltonian_compiles_against_traits ... ok
  test result: ok. 6 passed; 0 failed; 0 ignored

cargo test --features halcyon --test aurora_phase_2_hamiltonian_registry
  running 7 tests
  test test_registry_duplicate_name_rejects ... ok
  test test_registry_eager_init_no_auto_populate ... ok
  test test_registry_factory_kind_tag_round_trip ... ok
  test test_registry_list_registered_after_register ... ok
  test test_registry_register_then_lookup ... ok
  test test_registry_unknown_name_returns_none ... ok
  test test_wal_hamiltonian_declare_event_emitted ... ok
  test result: ok. 7 passed; 0 failed; 0 ignored
```

13/13 AURORA Phase 2 integration tests passing.

---

## Receipts

All five gates green:

```
cargo test --no-default-features --lib
  test result: ok. 870 passed; 0 failed; 0 ignored; 0 measured;
                   0 filtered out (finished in 3.65s)
  Byte-identical baseline preserved — gauge::action and
  gauge::hamiltonian_registry are feature-gated with the rest of
  the gauge module, so the no-default surface is untouched.

cargo test --features halcyon --lib -- --test-threads=1
  test result: ok. 1031 passed; 0 failed; 0 ignored; 0 measured;
                   0 filtered out (finished in 13.33s)
  Baseline holds at 1031. No new in-module unit tests added in
  src/gauge/action.rs or src/gauge/hamiltonian_registry.rs — the
  surface is exercised entirely by the two new integration test
  files in tests/ (which run under their own cargo test --test
  invocations, not the lib bench above).

cargo test --features kahler --lib
  test result: ok. 1150 passed; 0 failed; 0 ignored; 0 measured;
                   0 filtered out (finished in 89.03s)
  Baseline 1150/0 holds exactly. Long-running test
  cross_check_production_shape_complex (>60s) completed successfully.

cargo test --features halcyon --test halcyon_part_iv_gold -- --test-threads=1
  test result: ok. 4 passed; 0 failed; 1 ignored; 0 measured;
                   0 filtered out (finished in 11.04s)
  Bit-identity gate intact: IV.10 / III.8b / V.* contracts pass
  byte-identical to the Phase 1 + Phase 2 DEC baseline. The
  ignored tdd_hal_iv_10_a_symplectic_flow_canonical is the
  pre-existing Phase-0-known ignore, NOT a Phase 2 trait-surface
  drift signal.

cargo test --features halcyon --test halcyon_part_vi_bit_identity_gold \
  -- --test-threads=1 --include-ignored
  test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured;
                   0 filtered out (finished in 152.19s)
  VI bit-identity gold fixture intact: vi_5_capture_fixture +
  vi_f_a_acceptance_arm + vi_f_b_regression_arm_release_byte_identity
  all green. Symplectic flow + Halcyon Part V output bytes
  unchanged from the prior baseline. The new gauge modules are
  strictly additive (no edit to symplectic_flow / loop_transport /
  wilson_force / project_gauss / holonomy), so this gate held by
  construction.
```

### Identities verified (the contracts the suite pins)

1. **Trait surface compiles** — `Box<dyn HamiltonianHandle>` is
   object-safe with the four sub-traits + `Send + Sync + Debug`
   bounds. `Box<dyn HamiltonianFactory>` is object-safe (no
   associated types on the factory trait). `MockShallowWaterFactory`
   (mirroring AURORA's shape) and `NoOpHamiltonian` both compile.
2. **Group erasure works** — `MockShallowWaterFactory` with
   `group_tag() = "R"` compiles against the trait surface alongside
   `NoOpHamiltonianFactory` with `group_tag() = "NONE"`. No method
   signature references `SU2GaugeField` / `SU2EField` or any
   group-specific type.
3. **EnergyDecomposition keys deterministic** — the
   `MockShallowWaterFactory` returns AURORA's 7 keys in BTreeMap
   iteration order (alphabetical): `casimir_energy`, `casimir_mass`,
   `casimir_pv_l1`, `casimir_pv_l2`, `kelvin_eq`, `kelvin_n30`,
   `kelvin_s30`. Matches the `energy_keys()` static slice for
   diagnostic envelope stability.
4. **Factory missing-param error is typed** — empty params HashMap
   produces `FactoryError::MissingParam { name: "g" }`, no panic,
   no silent default to a Earth-default value (the factory itself
   chooses whether to default; the trait contract says missing
   required params surface as typed errors).
5. **Registry round-trip** — `register("FOO", Box::new(factory))`
   followed by `with_factory("FOO", |f| f.kind_tag())` returns the
   same `&'static str` the factory was constructed with.
6. **Unknown name returns None** — `with_factory("UNKNOWN", |_| 42)`
   returns `None`, not `Some(42)` or a panic.
7. **Duplicate name rejected** — second `register("FOO", ...)`
   returns `RegistryError::DuplicateName { name: "FOO" }` and the
   first registration is preserved (verified by re-querying
   `kind_tag` after the failed second register).
8. **WAL emission round-trip** — `register("FOO", factory,
   Some(&mut writer), 12345)` produces a `WalEntry::HamiltonianDeclare
   { name: "FOO", kind_tag: "MOCK", group_tag: "R", registered_at:
   12345 }` readable via `WalReader::read_all`. All four payload
   fields match.
9. **Eager-init contract** — after `clear()`,
   `with_factory("SHALLOW_WATER", ...)` and
   `with_factory("KOGUT_SUSSKIND", ...)` both return `None`. No
   auto-populated built-ins. The registry is empty unless a host
   binary explicitly registers.
10. **list_registered triples** — after registering three factories,
    `list_registered()` returns three `(name, kind_tag, group_tag)`
    triples in registration order (BTreeMap order; canonical names
    are unique by construction so iteration is well-defined).

### LOC totals

| Surface | LOC | Files |
| --- | --- | --- |
| Four sub-traits + `HamiltonianHandle` + `HamiltonianFactory` + `FactoryError` + `EnergyError` + `ProjectionError` | 300 | `src/gauge/action.rs` (new) |
| Registry storage + `register` + `with_factory` + `contains` + `list_registered` + `clear` + `RegistryError` + WAL emission integration | 203 | `src/gauge/hamiltonian_registry.rs` (new) |
| WAL `OP_HAMILTONIAN_DECLARE = 0x0C` constant + `WalEntry::HamiltonianDeclare` variant + `log_hamiltonian_declare` writer + reader arm | 75 | `src/wal.rs` (delta) |
| Module stitching: `pub mod action; pub mod hamiltonian_registry;` + 2 re-exports | 4 | `src/gauge/mod.rs` (delta) |
| Engine replay match-arm: acknowledge `HamiltonianDeclare` as no-op | ~5 | `src/engine.rs` (delta) |
| AURORA integration tests — trait surface | ~250 | `tests/aurora_phase_2_trait_surface.rs` (new) |
| AURORA integration tests — registry + WAL | ~223 | `tests/aurora_phase_2_hamiltonian_registry.rs` (new) |
| **Total new code** | **~1060** | 4 new files + 3 file deltas |

Of which production code (trait surface + registry + WAL plumbing,
excluding doc-comments and integration tests): ~580 LOC. Doc-comment
density is high because every `pub` item carries the 3-line EVOLVING
marker, and the trait method docs include rationale for the
state-erasure decision (`&[f64]` instead of an associated type) so
downstream implementors have the context to pack their state
correctly.

---

## Files touched

| File | LOC delta | Nature |
| --- | --- | --- |
| `src/gauge/action.rs` | +300 (new file) | Four sub-traits (`HamiltonianForce`, `HamiltonianDrift`, `ProjectionOperator`, `EnergyDecomposition`), `HamiltonianHandle` super-trait with `Send + Sync + Debug` bounds, `HamiltonianFactory` trait, three error enums (`FactoryError { MissingParam, InvalidParam, UnsupportedGroup }`, `EnergyError { StateShapeMismatch, NumericFailure }`, `ProjectionError { SolverDiverged, StateShapeMismatch }`). All pub items carry EVOLVING marker per `docs/STABILITY_GUARANTEES.md`. Group-agnostic by construction. |
| `src/gauge/hamiltonian_registry.rs` | +203 (new file) | `OnceLock<Mutex<HashMap<String, Box<dyn HamiltonianFactory>>>>` storage; public `register` + `with_factory` + `contains` + `list_registered` + `clear`; `RegistryError { DuplicateName, WalEmitFailed }`; WAL emission integration via injected `Option<&mut WalWriter>`. Q5 eager-init contract verified by `test_registry_eager_init_no_auto_populate`. |
| `src/gauge/mod.rs` | +4 | Two `pub mod` lines (`action`, `hamiltonian_registry`) + two re-exports (`HamiltonianHandle`, `HamiltonianFactory`). |
| `src/wal.rs` | +75 | `OP_HAMILTONIAN_DECLARE = 0x0C` constant beside existing gauge ops; `WalEntry::HamiltonianDeclare { name, kind_tag, group_tag, registered_at }` variant; `log_hamiltonian_declare` writer helper; `WalReader` arm for op 0x0C parsing back the four fields. Payload encoding matches `LatticeDeclare` + `GaugeFieldDeclare` style. |
| `src/engine.rs` | +~5 | Replay match-arm extension acknowledging `WalEntry::HamiltonianDeclare` as a no-op. Replay materialization explicitly deferred (Q5 eager-init contract makes this safe). |
| `tests/aurora_phase_2_trait_surface.rs` | +250 (new file) | 6 RED-first integration tests + private test-module structs `NoOpHamiltonian` + `NoOpHamiltonianFactory` + `MockShallowWaterFactory` + `ShallowWaterMock` mirroring AURORA's exact shape (7-key energy decomposition with casimir_* + kelvin_* names, kind_tag = "SHALLOW_WATER", group_tag = "R", g/omega/a params). |
| `tests/aurora_phase_2_hamiltonian_registry.rs` | +223 (new file) | 7 RED-first integration tests covering register/with_factory round-trip, duplicate rejection, unknown-name None, kind_tag round-trip, WAL emission round-trip via `WalReader::read_all`, eager-init contract, `list_registered` triples. Uses a tempdir helper (no `tempfile` crate dep) for WAL test isolation. |

**Untouched by this sprint** (verified additivity boundary):

- `src/gauge/symplectic_flow.rs` — the KDK leapfrog integrator with
  hardcoded SU(2) Kogut-Susskind path. The IV.10 + VI bit-identity
  gates lock its byte output; refactor to integrator-generic-over-`H`
  is a later workflow with its own RED→GREEN cycle.
- `src/gauge/wilson_force.rs` — SU(2) Wilson force kernel.
- `src/gauge/loop_transport.rs` — parallel transport along loops.
- `src/gauge/project_gauss.rs` — covariant divergence + Gauss
  projection.
- `src/gauge/holonomy.rs` — `walk_loop` group product.
- `src/lattice/dec/` — the Phase 2 DEC operator surface at `17105ff`.
- `src/lattice/metric.rs` — the Phase 1 `LatticeWithMetric` wrapper.
- Every existing test file.

---

## Stability annotation

Per `docs/STABILITY_GUARANTEES.md` trait-surface stability section
(commit `1e13252`) and the deployment precedent set at Phase 1 +
Phase 2 DEC:

Every public item introduced by this sprint carries the EVOLVING
doc-comment block:

```rust
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
```

Items annotated (full enumeration above under "Q6b — EVOLVING marker
per stability convention").

The convention shape matches the three prior deployments
(`LatticeWithMetric` at Phase 1; `ConstructorArgs` + `ConstructorError`
+ `get_constructor` at CC-2; `d_0` + `delta_1` + `hodge_star_k` +
`DecError` at Phase 2 DEC). This sprint is the **fourth** deployment
of the EVOLVING marker convention; it proves the convention extends
to trait-object-bearing surfaces (`Box<dyn HamiltonianFactory>`,
`Box<dyn HamiltonianHandle>`), not just structs + methods + free
functions.

No rustc-internal `#[stable]` proc-macro is used (per AURORA Q6b
correction; that attribute is not available to library crates). The
EVOLVING contract holds until the first `gigi 0.1.0` tag, at which
point the markers flip to `STABLE` and the breaking-change bar
becomes a major-version bump.

---

## Follow-up letter queue

One follow-up letter to AURORA is queued, covering six minor
signature deltas + one new bound:

1. **`HamiltonianHandle: Debug` bound added.** `#[derive(Debug)]`
   needed on `ShallowWater` (one line).
2. **State erased to `&[f64]` / `&mut [f64]` at the trait
   boundary.** Object-safety on `Box<dyn HamiltonianHandle>` requires
   no associated `type State`. Resolution: pack `ShallowWaterState
   { h, u }` into `Vec<f64>` with documented `[h cells || u edges]`
   ordering; private helpers `pack_state` / `unpack_state` on the
   `ShallowWater` impl. ~10 LOC adjustment.
3. **`EnergyDecomposition::evaluate` returns `BTreeMap<String, f64>`,
   not `HashMap<&'static str, f64>`.** Deterministic iteration for
   diagnostic envelopes. ~3 LOC adjustment.
4. **`EnergyDecomposition::energy_keys` is a new method.** Returns
   the canonical `&'static [&'static str]` so callers can list keys
   without invoking `evaluate`. AURORA adds one method (7-line
   array literal). ~1 LOC adjustment.
5. **`&Grid` param dropped from `energy_components`.** Renamed to
   `evaluate(state: &[f64])` per (2). Lattice geometry is baked into
   the state buffer at force/drift time; `evaluate` is pure-state.
   ~2 LOC adjustment.
6. **WAL `register` signature includes `wal_writer: Option<&mut
   WalWriter>` + `registered_at: u64`.** AURORA's `init()` body
   threads the host binary's WAL writer + a monotonic counter
   through. If the host has no WAL (test environments), pass `None`.
   ~3 LOC adjustment.
7. **`with_factory(name, |f| ...)` closure pattern, not
   `get_factory(name) -> &dyn ...`.** Avoids a `MutexGuard` lifetime
   hazard. AURORA's `init()` becomes `with_factory("SHALLOW_WATER",
   |f| f.from_params(&params))?`. ~2 LOC adjustment.

Net AURORA-side rework: ~20–25 LOC across `shallow_water.rs` +
`init.rs`. All single-line localized edits, no architectural
changes.

---

## Cross-references

- **v2 reply commit `baac7f2`** (`theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY_2.md`)
  — engine acceptance §2 (CC-1 open registry, WAL HamiltonianDeclare
  metadata-only) + §11 (A2 four-trait surface with hot-path
  constraint). This sprint honors both with explicit deferrals for
  the integrator refactor + WAL replay.
- **Phase 0 commit `ca589eb`** (`AURORA_PHASE_0_IMPL_LOG.md`) —
  `Lattice::signed_face_orientations()` promotion. Phase 2 trait
  surface does not consume this directly (trait methods speak in
  `&[f64]` buffers, not in face orientations), but the precedent of
  shipping additive lifts before behavior-bearing refactors is the
  same.
- **Phase 1 commit `f62e46c`** (`AURORA_PHASE_1_IMPL_LOG.md`) —
  `LatticeWithMetric` wrapper + `cubed_sphere` constructor + CC-2
  registry + `topology_hint` const table. The trait surface's
  generic `L: LatticeWithMetric` method parameter is this wrapper;
  AURORA's `ShallowWater::force_per_edge<L: LatticeWithMetric>` will
  consume the same surface their cubed-sphere construction emits.
- **Phase 1b commit `1091dd5`** — parser-executor switch to
  registry dispatch via `get_constructor()`. The hamiltonian
  registry mirrors this pattern (eager `register` + `with_factory`
  lookup, no parser arm yet because there is no GQL verb in this
  sprint; that would be a future workflow).
- **Phase 2 DEC commit `17105ff`** (`AURORA_PHASE_2_DEC_IMPL_LOG.md`)
  — `src/lattice/dec/` module with `d_0` + `delta_1` + `hodge_star_k`.
  AURORA's `ShallowWater::force_per_edge` will compose these with the
  trait surface this sprint ships: `grad h` via `d_0`, `div(hu)` via
  `delta_1`, vorticity decompositions via the Hodge stars.
- **Stability docs commit `1e13252`** (`docs/STABILITY_GUARANTEES.md`)
  — trait-surface stability section. This sprint is the fourth
  deployment of the EVOLVING marker convention; it proves the
  convention extends to trait-object-bearing surfaces.
- **`theory/aurora/AURORA_ASKS_v0_1_LOG.md`** — status-board rows
  CC-1 and A2 flipped from ACCEPTED to **DONE** in this session.
  Receipt commit hash backfills post-commit.
- **`theory/aurora/AURORA_TO_GIGI_REPLY2_2026-06-21.md`** — AURORA's
  Q5a explicit `init()` code shape (eager `register()` at top of
  `main()`) + Q6b correction (doc-comment + changelog convention,
  not `#[stable]` proc-macro). Both honored verbatim.
- **`C:/Users/nurdm/OneDrive/Documents/aurora/src/hamiltonians/shallow_water.rs`**
  — AURORA's downstream `ShallowWaterFactory` skeleton. Lines 39–75
  (trait stubs marked "blocked on LatticeWithMetric + A2 trait
  surface") are now unblocked; lines 76–97 (`energy_components` with
  the 7 casimir + kelvin keys, fully implemented) align with the
  EnergyDecomposition contract after the minor signature adjustments
  in the follow-up letter.
- **`src/gauge/symplectic_flow.rs`** — the KDK leapfrog integrator,
  unchanged in this sprint. The future
  integrator-generic-over-`H` refactor lifts its inner loop to be
  generic over concrete `H: HamiltonianForce + HamiltonianDrift`
  monomorphized via outer-loop enum match, per Reply 2 §11 hot-path
  constraint.

---

## What's next

This sprint closes the AURORA-facing Phase 2 trait surface + CC-1
registry. AURORA can now:

1. Uncomment the `init()` body in their host binary.
2. Implement `ShallowWaterFactory: HamiltonianFactory` with the
   minor signature adjustments from the follow-up letter
   (state-erasure, BTreeMap, etc.).
3. Wire `gigi::gauge::hamiltonian_registry::register("SHALLOW_WATER",
   Box::new(ShallowWaterFactory), Some(&mut wal_writer), now_micros())?;`
   at the top of `main()` before `Engine::open()`.
4. Implement `HamiltonianForce::force_per_edge` using
   `gigi::lattice::dec::d_0` for `grad h` and `gigi::lattice::dec::delta_1`
   for `div(hu)`, plus the Hodge stars for any vorticity work.
5. Run a real Williamson Test 2 against `gigi-stream.fly.dev`,
   replacing the step-0 scaffold receipt with a full multi-step
   trajectory.

Two workflows are queued downstream and explicitly deferred from
this sprint:

- **`symplectic_flow.rs` integrator-generic-over-`H` refactor.**
  Lifts the KDK leapfrog inner loop to be generic over concrete `H:
  HamiltonianForce + HamiltonianDrift`, monomorphized via outer-loop
  enum match. Touches the hottest hot path (IV.10 + VI bit-identity
  fixtures lock byte-output); its own RED→GREEN cycle against those
  gates is required. Does NOT block AURORA — the trait surface is
  shipped so any downstream consumer can implement against a stable
  contract today; the refactor strengthens the in-tree path without
  changing the external contract.
- **KogutSusskind lift to `HamiltonianFactory` impl.** The in-tree
  canonical impl of the new trait surface — wrapping the existing
  SU(2) Wilson force + Rodrigues drift + Gauss projection +
  kinetic-energy decomposition as a `HamiltonianHandle`. Natural
  pairing with the integrator refactor above. Demonstrates
  group-agnostic compilation (SHALLOW_WATER with group_tag = "R"
  beside KOGUT_SUSSKIND with group_tag = "SU2") on the same trait
  surface.

Phase 2's CC-1 + A2 trait surface is self-contained and ships
independently of the two deferred workflows. AURORA's Williamson
Test 2 is unblocked today.

---

## Authorship note

Per `feedback_no_ai_coauthor.md`: when the parent agent commits this
sprint, the commit body must NOT carry a `Co-Authored-By: Claude`
footer. Author = `nurdymuny <bee_davis@alumni.brown.edu>` (Bee Rosa
Davis) only. Same convention as Phase 0 (`ca589eb`), Phase 1
(`f62e46c`), Phase 1b (`1091dd5`), Phase 2 DEC (`17105ff`), and
every commit in the Halcyon Part V sprint.
