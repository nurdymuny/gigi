//! In-process lattice registry. The executor materializes declared
//! `LATTICE` statements here; `SHOW LATTICE name` reads back through
//! it.
//!
//! Closes the storage half of TDD-HAL-I.8. The registry is a
//! per-process `HashMap<String, Lattice>` guarded by a `Mutex` —
//! a `BundleStore`-grade persistence layer is a Part-II follow-up
//! once `GAUGE_FIELD` lands on top.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use super::metric::LatticeWithMetric;
use super::Lattice;

/// Global registry. Singleton; the engine is single-tenant per
/// process for Part I.
fn registry() -> &'static Mutex<HashMap<String, Lattice>> {
    static REG: OnceLock<Mutex<HashMap<String, Lattice>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register a Lattice under its `name`. Overwrites any previous
/// registration with the same name.
pub fn register(lat: Lattice) {
    let mut g = registry().lock().expect("lattice registry mutex poisoned");
    g.insert(lat.name.clone(), lat);
}

/// Look up a Lattice by name. Returns a clone for round-trip
/// stability — the caller never sees the in-registry Mutex guard.
pub fn get(name: &str) -> Option<Lattice> {
    let g = registry().lock().expect("lattice registry mutex poisoned");
    g.get(name).cloned()
}

/// Clear the registry. Convenience for tests + for the engine's
/// `do_replay` path which rebuilds the registry from WAL on every
/// `Engine::open` (TDD-HAL-II.4b durability gate).
pub fn clear() {
    let mut g = registry().lock().expect("lattice registry mutex poisoned");
    g.clear();
}

/// Snapshot every registered Lattice for compaction. The engine's
/// `compact_wal_to_schemas` re-emits one `WalEntry::LatticeDeclare`
/// per snapshot entry so the durable lattice set survives WAL rewrite.
/// Ordering is unspecified — the gauge-field emit consults the
/// resolved name, not iteration order.
pub fn all() -> Vec<Lattice> {
    let g = registry().lock().expect("lattice registry mutex poisoned");
    g.values().cloned().collect()
}

// ── CC-2 constructor-dispatch surface ─────────────────────────────────
//
// The instance registry above stores fully-realized `Lattice` values
// the executor materialized. The *constructor* registry below is the
// CC-2 dispatch surface: a lazily-initialized table from canonical
// constructor identifier (`"TRUNCATED_ICOSAHEDRON"`, `"CUBED_SPHERE"`,
// …) to a `fn` pointer that produces a fresh [`LatticeWithMetric`]. The
// two registries are intentionally separate — instances are
// per-process state the engine WAL-replays into; constructors are
// pure functions of their `ConstructorArgs`.
//
// The CC-2 refactor is *additive*: the GQL `LATTICE name FROM
// CANONICAL_ID` executor arm in `src/parser.rs` continues to use its
// hard-coded match for Phase 1 so the bit-identity contract on the
// buckyball constructor cannot regress while the two paths coexist.
// Downstream callers (Halcyon's CUBED_SPHERE consumer, the AURORA
// `aurora_lattice_registry_dispatch` byte-equality guards) reach the
// constructors through [`get_constructor`].

/// Arguments handed to every constructor. New per-topology parameters
/// land here as additional `Option` fields so adding a knob never
/// breaks existing constructors.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConstructorArgs {
    /// Cubed-sphere panel resolution `C` (cells per panel side).
    /// Defaults to `1` when `None` (the degenerate cube case). The
    /// constructor validates `1 <= C <= 256`.
    pub panel_size: Option<usize>,
    /// CUBIC lattice size `L` (per-axis vertex count). Defaults to
    /// `1` when `None` (the degenerate point case). The constructor
    /// validates `1 <= L <= 1024`.
    pub l: Option<usize>,
    /// CUBIC lattice dimension `D` (2, 3, or 4 supported in Phase 1).
    /// Defaults to `2` when `None` (the flat 2-torus). The constructor
    /// validates `1 <= D <= 4`.
    pub dim: Option<usize>,
    /// CUBIC boundary condition. `Some(true)` (or `None`, the default)
    /// = PERIODIC; `Some(false)` = fully OPEN. Phase 1 ships PERIODIC
    /// plus single-axis OBC (via `obc_axis`); fully-open (`Some(false)`
    /// with `obc_axis = None`) routes to the cubic constructor's
    /// deferred-to-Phase-2 assertion.
    pub periodic: Option<bool>,
    /// CUBIC single-axis open-boundary. `None` (default) = fully
    /// periodic on every axis. `Some(k)` with `k ∈ 0..dim` = axis `k`
    /// is open (wrap edges and boundary-crossing plaquettes at the
    /// `L-1` slice are omitted); all other axes remain periodic.
    /// Vertex count is unchanged. Required for Hallie's SU(2) 4D L=24
    /// β=2.3 OBC sectoral SPECTRAL_GAUGE workflow (`LATTICE l24 FROM
    /// CUBIC L=24 DIM=4 OBC AXIS 0;`).
    pub obc_axis: Option<usize>,
}

/// Constructor-side error. Carries the canonical id at the call site so
/// the caller need not stitch it back on.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstructorError {
    /// `panel_size` (or another per-topology argument) was outside the
    /// constructor's accepted range.
    InvalidArgument(String),
}

impl std::fmt::Display for ConstructorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstructorError::InvalidArgument(msg) => {
                write!(f, "invalid constructor argument: {msg}")
            }
        }
    }
}

impl std::error::Error for ConstructorError {}

/// Function-pointer type of every registered constructor. Constructors
/// take a shared `&ConstructorArgs` so adding new arg fields is a
/// non-breaking change for existing constructors.
pub type Constructor =
    fn(&ConstructorArgs) -> Result<LatticeWithMetric, ConstructorError>;

/// Lazy table from canonical-id (upper-case) to constructor `fn`.
/// Populated by [`init_builtin_constructors`] at first access.
fn constructors() -> &'static HashMap<&'static str, Constructor> {
    static TABLE: OnceLock<HashMap<&'static str, Constructor>> = OnceLock::new();
    TABLE.get_or_init(init_builtin_constructors)
}

/// Idempotently register the Phase 1 built-in constructors. Called from
/// inside [`constructors`]; exposed `pub` so out-of-tree callers can
/// assert the table is materialized (the result is identical to calling
/// [`get_constructor`] on a known key).
pub fn init_builtin_constructors() -> HashMap<&'static str, Constructor> {
    let mut t: HashMap<&'static str, Constructor> = HashMap::new();
    t.insert("TRUNCATED_ICOSAHEDRON", build_truncated_icosahedron as Constructor);
    t.insert("CUBED_SPHERE", build_cubed_sphere as Constructor);
    t.insert("CUBIC", build_cubic as Constructor);
    t
}

/// Resolve a canonical constructor identifier to its constructor `fn`.
///
/// Lookup is case-insensitive — `"cubed_sphere"`, `"Cubed_Sphere"` and
/// `"CUBED_SPHERE"` all resolve to the same constructor. Unknown
/// identifiers return `None`; no silent default is provided, so a typo
/// surfaces immediately rather than dispatching to the wrong topology.
///
/// Phase 1 built-ins (registered by [`init_builtin_constructors`]):
///
/// - `TRUNCATED_ICOSAHEDRON` — wraps
///   [`crate::lattice::topology::truncated_icosahedron::buckyball`] in a
///   zero-metric [`LatticeWithMetric`]. Phase 1 does not assign a real
///   metric to the buckyball; Phase 2 owns that.
/// - `CUBED_SPHERE` — calls
///   [`crate::lattice::topology::cubed_sphere::cubed_sphere`] with
///   `panel_size` from [`ConstructorArgs`] (default `1`, validated
///   `1..=256`).
/// - `CUBIC` — calls [`crate::lattice::topology::cubic::cubic`] with
///   `l` / `dim` / `periodic` from [`ConstructorArgs`] (defaults `l=1`,
///   `dim=2`, `periodic=true`). Validated `1 <= L <= 1024` and
///   `1 <= D <= 4`. Halcyon §3.3 substrate (4D pure-gauge target:
///   L=12, D=4 → V=20736, E=82944, F=124416). PERIODIC only in Phase
///   1; OPEN is deferred to Phase 2 by the underlying constructor.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
pub fn get_constructor(canonical: &str) -> Option<Constructor> {
    let needle = canonical.to_ascii_uppercase();
    constructors().get(needle.as_str()).copied()
}

/// CC-2 wrapper around the existing
/// [`crate::lattice::topology::truncated_icosahedron::buckyball`]
/// constructor. Zero-metric Phase 1 placeholder — the buckyball gets a
/// real metric in Phase 2 when DEC operators land.
///
/// The existing `buckyball()` and `buckyball_with_signed_faces()`
/// public functions remain unchanged; this wrapper dispatches into
/// `buckyball()` so byte-identical equality with the legacy executor
/// path is straightforward to assert.
fn build_truncated_icosahedron(
    _args: &ConstructorArgs,
) -> Result<LatticeWithMetric, ConstructorError> {
    let lat = crate::lattice::topology::truncated_icosahedron::buckyball();
    // AURORA Reply 6: attach the 60 unit-sphere fullerene-cage
    // coordinates so downstream DEC consumers can project analytical
    // initial conditions onto buckyball vertices/edges/faces without
    // recomputing the cage geometry locally.
    let positions =
        crate::lattice::topology::truncated_icosahedron::buckyball_unit_sphere_positions();
    Ok(LatticeWithMetric::from_lattice_and_metric(
        lat,
        Vec::new(),
        Vec::new(),
        None,
    )
    .with_vertex_positions(positions))
}

/// CC-2 wrapper around
/// [`crate::lattice::topology::cubed_sphere::cubed_sphere`]. Reads
/// `panel_size` from [`ConstructorArgs`] (default `1`), validates
/// `1 <= C <= 256`, then delegates. The wrapper supplies the
/// default name `"cubed_sphere"`; the parser executor renames before
/// registering.
fn build_cubed_sphere(
    args: &ConstructorArgs,
) -> Result<LatticeWithMetric, ConstructorError> {
    let c = args.panel_size.unwrap_or(1);
    if !(1..=256).contains(&c) {
        return Err(ConstructorError::InvalidArgument(format!(
            "CUBED_SPHERE: panel_size must be in 1..=256, got {c}"
        )));
    }
    Ok(crate::lattice::topology::cubed_sphere::cubed_sphere(
        "cubed_sphere",
        c,
    ))
}

/// CC-2 wrapper around [`crate::lattice::topology::cubic::cubic`].
/// Reads `l` / `dim` / `periodic` from [`ConstructorArgs`] (defaults
/// `l=1`, `dim=2`, `periodic=true`), validates `1 <= L <= 1024` and
/// `1 <= D <= 4`, then delegates. The wrapper supplies the default
/// name `"cubic"`; the parser executor renames before registering.
///
/// Phase 1 scope: PERIODIC only. The underlying `cubic()` constructor
/// panics on `periodic = false` with a deferred-to-Phase-2 message;
/// this wrapper does NOT short-circuit that panic into an
/// `InvalidArgument` because the test gate `test_open_boundary_not_yet_supported`
/// asserts the panic propagates as-is. Phase 2 will lift the
/// PERIODIC-only restriction in the constructor itself.
fn build_cubic(args: &ConstructorArgs) -> Result<LatticeWithMetric, ConstructorError> {
    let l = args.l.unwrap_or(1);
    let d = args.dim.unwrap_or(2);
    let periodic = args.periodic.unwrap_or(true);
    let obc_axis = args.obc_axis;
    if !(1..=1024).contains(&l) {
        return Err(ConstructorError::InvalidArgument(format!(
            "CUBIC: L must be in 1..=1024, got {l}"
        )));
    }
    if !(1..=4).contains(&d) {
        return Err(ConstructorError::InvalidArgument(format!(
            "CUBIC: DIM must be in 1..=4 (Phase 1 ships 2D/3D/4D), got {d}"
        )));
    }
    if let Some(k) = obc_axis {
        if k >= d {
            return Err(ConstructorError::InvalidArgument(format!(
                "CUBIC: OBC AXIS {k} out of range for DIM={d} (must be 0..{d})"
            )));
        }
    }
    Ok(crate::lattice::topology::cubic::cubic(
        "cubic", l, d, periodic, obc_axis,
    ))
}

#[cfg(test)]
mod tests {
    use super::super::topology::truncated_icosahedron::buckyball;
    use super::*;

    #[test]
    fn register_and_get_round_trip() {
        clear();
        let bb = buckyball();
        register(bb.clone());
        let got = get("buckyball").expect("buckyball was just registered");
        assert_eq!(got, bb);
    }

    /// TDD-HAL-I.8 — executor + SHOW LATTICE round-trip.
    /// Declare a lattice via the shorthand form; issue SHOW LATTICE
    /// name; re-parse the SHOW output's `gql` column; assert
    /// structural equality with the original.
    #[test]
    fn tdd_hal_i_8_lattice_register_and_show() {
        use crate::engine::Engine;
        use crate::parser;
        use crate::types::Value;

        clear();
        // Use an isolated name to avoid colliding with other tests
        // racing for the singleton registry.
        let name = "tdd_hal_i_8_bb";

        let dir = tempfile::tempdir().expect("tempdir");
        let mut engine = Engine::open(dir.path()).expect("engine open");

        // 1. Declare via shorthand. Executor materializes via
        // truncated_icosahedron::buckyball() and renames.
        let decl = format!(
            "LATTICE {name} FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';"
        );
        let stmt = parser::parse(&decl).expect("parse LATTICE decl");
        match parser::execute(&mut engine, &stmt).expect("exec LATTICE decl") {
            parser::ExecResult::Ok => {}
            other => panic!("expected Ok, got {other:?}"),
        }

        // 2. SHOW LATTICE name; — read out the registered lattice.
        let show = format!("SHOW LATTICE {name};");
        let stmt = parser::parse(&show).expect("parse SHOW LATTICE");
        let rows = match parser::execute(&mut engine, &stmt)
            .expect("exec SHOW LATTICE")
        {
            parser::ExecResult::Rows(r) => r,
            other => panic!("expected Rows, got {other:?}"),
        };
        assert_eq!(rows.len(), 1, "SHOW LATTICE returns exactly one row");
        let row = &rows[0];
        let gql_emitted = match row.get("gql") {
            Some(Value::Text(s)) => s.clone(),
            other => panic!("missing/wrong-typed gql column: {other:?}"),
        };

        // 3. Round-trip — re-parse the emitted GQL via the Lattice
        //    algebra's own from_gql (the canonical re-emit
        //    parser).
        let lat = super::super::Lattice::from_gql(&gql_emitted)
            .expect("re-parse SHOW output");
        assert_eq!(lat.name, name);
        assert_eq!(lat.n_vertices, 60);
        assert_eq!(lat.n_edges(), 90);
        assert_eq!(lat.n_faces(), 32);
        assert_eq!(lat.topology.as_deref(), Some("S2"));

        // 4. Structural equality with the registered lattice
        //    (after the rename — the buckyball constructor names
        //    itself "buckyball" by default; the executor rebinds
        //    to the declared name).
        let registered = get(name).expect("registered lattice");
        assert_eq!(lat, registered);
    }

    /// Bit-identity contract: re-emitting an explicit-form
    /// declaration through SHOW LATTICE produces the same canonical
    /// re-emit body twice in a row.
    #[test]
    fn tdd_hal_i_8_explicit_form_round_trip() {
        use crate::engine::Engine;
        use crate::parser;
        use crate::types::Value;

        clear();
        let name = "tdd_hal_i_8_explicit";
        let dir = tempfile::tempdir().expect("tempdir");
        let mut engine = Engine::open(dir.path()).expect("engine open");
        let decl = format!(
            "LATTICE {name} \
             VERTICES 4 \
             EDGES ((0,1),(1,2),(2,0)) \
             FACES ((0,1,2));"
        );
        let stmt = parser::parse(&decl).expect("parse explicit LATTICE");
        parser::execute(&mut engine, &stmt).expect("exec explicit LATTICE");

        let show = format!("SHOW LATTICE {name};");
        let stmt = parser::parse(&show).expect("parse SHOW LATTICE");
        let rows = match parser::execute(&mut engine, &stmt)
            .expect("exec SHOW LATTICE")
        {
            parser::ExecResult::Rows(r) => r,
            other => panic!("expected Rows, got {other:?}"),
        };
        let gql_first = match rows[0].get("gql") {
            Some(Value::Text(s)) => s.clone(),
            other => panic!("missing/wrong gql: {other:?}"),
        };

        // Round-trip the emitted form back through the parser +
        // executor.
        let stmt = parser::parse(&gql_first).expect("re-parse SHOW output");
        parser::execute(&mut engine, &stmt).expect("re-exec re-parsed LATTICE");

        // Issue SHOW LATTICE again — the canonical form is
        // bit-identical to the first emission.
        let stmt = parser::parse(&show).expect("parse SHOW LATTICE 2");
        let rows = match parser::execute(&mut engine, &stmt)
            .expect("exec SHOW LATTICE 2")
        {
            parser::ExecResult::Rows(r) => r,
            other => panic!("expected Rows, got {other:?}"),
        };
        let gql_second = match rows[0].get("gql") {
            Some(Value::Text(s)) => s.clone(),
            other => panic!("missing/wrong gql: {other:?}"),
        };
        assert_eq!(gql_first, gql_second);
    }

    // ── CC-2 constructor-dispatch tests ──────────────────────────────

    /// CC-2 bit-identity guard: the registry-dispatched
    /// TRUNCATED_ICOSAHEDRON constructor produces a `Lattice` byte-equal
    /// to the legacy `buckyball()` direct path.
    #[test]
    fn cc2_truncated_icosahedron_via_registry_matches_direct() {
        let ctor = get_constructor("TRUNCATED_ICOSAHEDRON")
            .expect("TRUNCATED_ICOSAHEDRON registered");
        let lwm = ctor(&ConstructorArgs::default())
            .expect("buckyball constructor returns Ok");
        let direct = buckyball();
        assert_eq!(lwm.lattice(), &direct);
        // Phase 1 zero-metric placeholder for the buckyball.
        assert!(lwm.cell_areas().is_empty());
        assert!(lwm.edge_lengths().is_empty());
        assert!(lwm.dual_face_areas().is_none());
    }

    /// CC-2: CUBED_SPHERE constructor wires through with the default
    /// panel size (C = 1, the degenerate cube). 6 faces, 8 vertices,
    /// 12 edges; Euler χ = 2.
    #[test]
    fn cc2_cubed_sphere_via_registry_default_panel_size() {
        let ctor = get_constructor("CUBED_SPHERE")
            .expect("CUBED_SPHERE registered");
        let lwm = ctor(&ConstructorArgs::default())
            .expect("cubed_sphere with default panel_size");
        let lat = lwm.lattice();
        assert_eq!(lat.n_faces(), 6);
        assert_eq!(lat.n_vertices, 8);
        assert_eq!(lat.n_edges(), 12);
        assert_eq!(lat.topology.as_deref(), Some("S2"));
    }

    /// CC-2: CUBED_SPHERE constructor with an explicit panel size of
    /// `C = 3` produces the locked combinatorial counts F = 54, V = 56,
    /// E = 108.
    #[test]
    fn cc2_cubed_sphere_via_registry_panel_size_three() {
        let ctor = get_constructor("CUBED_SPHERE")
            .expect("CUBED_SPHERE registered");
        let args = ConstructorArgs { panel_size: Some(3), ..Default::default() };
        let lwm = ctor(&args).expect("cubed_sphere with C = 3");
        let lat = lwm.lattice();
        assert_eq!(lat.n_faces(), 6 * 3 * 3);
        assert_eq!(lat.n_vertices, 6 * 3 * 3 + 2);
        assert_eq!(lat.n_edges(), 12 * 3 * 3);
    }

    /// CC-2: lookup is case-insensitive on the canonical id.
    #[test]
    fn cc2_get_constructor_is_case_insensitive() {
        assert!(get_constructor("truncated_icosahedron").is_some());
        assert!(get_constructor("Truncated_Icosahedron").is_some());
        assert!(get_constructor("TRUNCATED_ICOSAHEDRON").is_some());
        assert!(get_constructor("cubed_sphere").is_some());
        assert!(get_constructor("Cubed_Sphere").is_some());
        assert!(get_constructor("CUBED_SPHERE").is_some());
    }

    /// CC-2: unknown canonical ids return `None`, never a silent
    /// default.
    #[test]
    fn cc2_get_constructor_unknown_returns_none() {
        assert!(get_constructor("NOPE").is_none());
        assert!(get_constructor("").is_none());
        assert!(get_constructor("cube").is_none());
    }

    /// CC-2: CUBED_SPHERE rejects out-of-range panel sizes.
    #[test]
    fn cc2_cubed_sphere_rejects_out_of_range_panel_size() {
        let ctor = get_constructor("CUBED_SPHERE").unwrap();
        let too_big = ConstructorArgs { panel_size: Some(257), ..Default::default() };
        assert!(matches!(
            ctor(&too_big),
            Err(ConstructorError::InvalidArgument(_))
        ));
        let zero = ConstructorArgs { panel_size: Some(0), ..Default::default() };
        assert!(matches!(
            ctor(&zero),
            Err(ConstructorError::InvalidArgument(_))
        ));
    }

    /// CC-2: `init_builtin_constructors` is idempotent in shape — every
    /// successful materialization contains exactly the Phase 1 keys.
    #[test]
    fn cc2_init_builtin_constructors_lists_phase1_keys() {
        let t = init_builtin_constructors();
        assert_eq!(t.len(), 3);
        assert!(t.contains_key("TRUNCATED_ICOSAHEDRON"));
        assert!(t.contains_key("CUBED_SPHERE"));
        assert!(t.contains_key("CUBIC"));
    }
}
