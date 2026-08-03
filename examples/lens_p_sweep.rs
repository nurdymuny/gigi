//! lens_p_sweep — Poincaré Tier 1 live receipt (Davis–Poincaré Thm 3.6).
//!
//! Runs gigi's real `HOLONOMY <field> AROUND CYCLE AXIS z` verb over the
//! lens-space family L(p,q) = S³/ℤ_p, realized as a periodic cubic lattice
//! whose single +z wrap edge at (x0,y0) carries the σ₃-twist
//!   Ω = (cos 2πq/p, 0, 0, sin 2πq/p).
//! The distinguishing observable is the SU(2) holonomy around the
//! non-contractible z-cycle and its ORDER, which equals |π₁(L(p,q))| = p.
//!
//! Why an example (not GQL): the `GAUGE_FIELD ... INIT` surface only offers
//! IDENTITY / HAAR / FROM(field) / FLUX(U1) — there is no GQL path to inject
//! a CHOSEN SU(2) link. So the p=1 control is reachable over HTTP, but the
//! p>1 twist must be planted through the public `SU2GaugeField::from_buffer`
//! factory (the same one the H2 acceptance test uses). The holonomy walk
//! itself is 100% gigi's `execute_holonomy_cycle` executor — this file only
//! constructs the input field and prints the verb's rows as JSON.
//!
//! Run:
//!   cargo run --example lens_p_sweep \
//!     --features halcyon,kahler,wish,imagine,gauge,lattice
//!
//! Emits a JSON array on stdout: one object per (p,q) with the verb's
//! {q0,q3,re_trace,order_estimate} and the closed-form expectation.

use std::sync::Arc;

use gigi::engine::Engine;
use gigi::gauge::registry as gauge_registry;
use gigi::gauge::{DenseLinkBuffer, GaugeFieldHandle, GaugeFieldInit, Group, SU2GaugeField};
use gigi::lattice::registry as lattice_registry;
use gigi::lattice::Lattice;
use gigi::parser::{execute, parse, ExecResult};
use gigi::types::{Record, Value};

/// Row-major cubic site id: v(x,y,z) = x + y·L + z·L².
fn site(x: usize, y: usize, z: usize, l: usize) -> usize {
    x + y * l + z * l * l
}

/// Lens wrap quaternion Ω = (cos 2πq/p, 0, 0, sin 2πq/p) — a σ₃ twist.
fn omega(p: usize, q: usize) -> [f64; 4] {
    let ang = 2.0 * std::f64::consts::PI * q as f64 / p as f64;
    [ang.cos(), 0.0, 0.0, ang.sin()]
}

fn declare_cubic(eng: &mut Engine, name: &str, l: usize, d: usize) -> Lattice {
    let gql = format!("LATTICE {name} FROM CUBIC L={l} DIM={d} PERIODIC;");
    let stmt = parse(&gql).expect("parse LATTICE");
    execute(eng, &stmt).expect("exec LATTICE");
    lattice_registry::get(name).expect("lattice registered")
}

/// Identity SU(2) field on `lat` with the listed (edge_id, quaternion)
/// overrides planted via the public `from_buffer` factory.
fn register_su2(name: &str, lat: &Lattice, overrides: &[(usize, [f64; 4])]) {
    let n_edges = lat.n_edges();
    let mut buf = DenseLinkBuffer::new_identity(Group::SU2, n_edges).expect("identity SU2 buffer");
    for &(eid, q) in overrides {
        let base = 4 * eid;
        buf.data[base] = q[0];
        buf.data[base + 1] = q[1];
        buf.data[base + 2] = q[2];
        buf.data[base + 3] = q[3];
    }
    let field = SU2GaugeField::from_buffer(
        name.to_string(),
        lat.name.clone(),
        buf,
        GaugeFieldInit::Identity,
        None,
    );
    let h: Arc<dyn GaugeFieldHandle> = Arc::new(field);
    gauge_registry::register(h);
}

fn run_cycle(eng: &mut Engine, gql: &str) -> Record {
    let stmt = parse(gql).unwrap_or_else(|e| panic!("parse '{gql}': {e}"));
    let res = execute(eng, &stmt).unwrap_or_else(|e| panic!("exec '{gql}': {e}"));
    match res {
        ExecResult::Rows(mut rs) => rs.pop().expect("one row"),
        other => panic!("expected Rows, got {other:?}"),
    }
}

fn ff(row: &Record, k: &str) -> f64 {
    match row.get(k) {
        Some(Value::Float(v)) => *v,
        other => panic!("field '{k}' not Float: {other:?}"),
    }
}
fn ii(row: &Record, k: &str) -> i64 {
    match row.get(k) {
        Some(Value::Integer(v)) => *v,
        other => panic!("field '{k}' not Integer: {other:?}"),
    }
}

fn main() {
    gauge_registry::clear();
    lattice_registry::clear();
    let dir = tempfile::tempdir().expect("tempdir");
    let mut eng = Engine::open(dir.path()).expect("engine open");

    let l = 7usize;
    let (x0, y0) = (1usize, 2usize);
    let lat = declare_cubic(&mut eng, "lens_lat", l, 3);
    let wrap = lat
        .resolve_edge(site(x0, y0, l - 1, l), site(x0, y0, 0, l))
        .expect("periodic z-wrap edge exists")
        .0;

    // (p, q): p=1 is the S³ control (identity, no twist); the rest are the
    // lens family whose π₁ = ℤ/p. q chosen coprime to p.
    let cases: &[(usize, usize)] = &[(1, 0), (2, 1), (3, 1), (5, 1), (5, 2), (7, 1), (7, 3)];

    let mut objs: Vec<String> = Vec::new();
    for &(p, q) in cases {
        gauge_registry::remove("lens_field");
        if p == 1 {
            register_su2("lens_field", &lat, &[]); // identity everywhere
        } else {
            register_su2("lens_field", &lat, &[(wrap, omega(p, q))]);
        }
        let row = run_cycle(&mut eng, "HOLONOMY lens_field AROUND CYCLE AXIS z AT (1, 2);");
        let expected = (2.0 * std::f64::consts::PI * q as f64 / p as f64).cos();
        let re_trace = ff(&row, "re_trace");
        let order = ii(&row, "order_estimate");
        let matches = (re_trace - expected).abs() < 1e-10 && order == p as i64;
        objs.push(format!(
            "{{\"p\":{p},\"q\":{q},\"q0\":{:.12},\"q3\":{:.12},\"re_trace\":{:.12},\
             \"order_estimate\":{order},\"expected_re_trace\":{:.12},\
             \"expected_order\":{p},\"match\":{matches}}}",
            ff(&row, "q0"),
            ff(&row, "q3"),
            re_trace,
            expected,
        ));
    }
    println!("[\n  {}\n]", objs.join(",\n  "));
}
