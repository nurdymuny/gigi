//! HELICITY — discrete fluid helicity as the lattice Chern-Simons
//! functional (Navier-Stokes Tier 1, Ask 1; 2026-07-17).
//!
//! Fluid helicity `H = ∫ u·ω dV` (with vorticity `ω = ∇×u`) is the
//! central topological invariant of a flow — conserved by the Euler
//! equations and measuring the linking of vortex lines (Moffatt 1969).
//! Reading the velocity as a connection 1-form `A` (one signed real
//! `a_e` per edge), the invariant is the discrete Chern-Simons
//! functional `H = ∫ A∧dA` — a *scalar* contraction of real edge
//! 1-forms, metric-light (the `h`-factors cancel), NOT the SU(2)
//! `walk_loop` quaternion product used by CHERN_CLASS / Wilson.
//!
//! ## Substrate
//!
//! The input is a periodic cubic `L³` edge-endpoint bundle — the same
//! `(vertex_a, vertex_b, a_e)` shape the gauge / MODE MATRIX bundles
//! use — with edges emitted FORWARD (exactly three per site: `+x`,
//! `+y`, `+z`, so `3·L³` records total). Sites use Hallie's vertex-id
//! convention
//!
//! ```text
//! vid(i,j,k) = (i·L + j)·L + k          (x = i slowest, z = k fastest)
//! ```
//!
//! `L` is inferred as `round(∛(max_vid + 1))` and validated
//! (`L³ == max_vid+1`, `L ≥ 2`); a non-cubic / non-3D / partial bundle
//! is rejected with a typed error.
//!
//! ## Pinned discrete formula (turnkey, deterministic)
//!
//! With `A_d(s)` the edge fiber of the `+d` edge at site `s`,
//!
//! ```text
//! H = Σ_s [ A_x(s)·Ω_x(s) + A_y(s)·Ω_y(s) + A_z(s)·Ω_z(s) ]
//!
//! Ω_x(s) = A_y(s) + A_z(s+ŷ) − A_y(s+ẑ) − A_z(s)   # yz-plaquette ⟂ x
//! Ω_y(s) = A_z(s) + A_x(s+ẑ) − A_z(s+x̂) − A_x(s)   # zx-plaquette ⟂ y
//! Ω_z(s) = A_x(s) + A_y(s+x̂) − A_x(s+ŷ) − A_y(s)   # xy-plaquette ⟂ z
//! ```
//!
//! the shifts `s+ê` wrap periodically. `Ω_d(s)` is the co-located
//! plaquette circulation perpendicular to `d` — the same four-edge
//! index structure CHERN_CLASS / Wilson enumerate, contracted here with
//! real weights. The per-cell density is the bracketed term; it sums to
//! `H` exactly.
//!
//! ## Golden anchors (Hallie, ABC Beltrami A=B=C=1)
//!
//! `u = (sin z + cos y, sin x + cos z, sin y + cos x)`, `a_e = u_d·h`,
//! `h = 2π/L`. Then `∇×u = u` (Beltrami, eigenvalue +1), the density is
//! `|u|²`, and the closed form of the discrete sum is
//!
//! ```text
//! H(L) = 12·π²·L·sin(2π/L)   →   24π³ ≈ 744.1506   as L → ∞
//! ```
//!
//! reproducing `L=16 → 725.171`, `L=24 → 735.679`, `L=32 → 739.378`,
//! `L=48 → 742.027`. This measures `∫A∧dA` / vortex linking; it is
//! tooling + evidence, not a proof of Navier-Stokes regularity.

use crate::bundle::BundleStore;
use crate::types::Record;

/// Result of `HELICITY <bundle> ON FIBER (<a_field>) [DENSITY]`.
#[derive(Debug, Clone)]
pub struct HelicityResult {
    /// The scalar helicity `H = Σ A∧dA` (discrete Chern-Simons).
    pub helicity: f64,
    /// Number of forward edge records consumed (`3·L³` for a well-formed
    /// cubic bundle).
    pub n_edges_used: usize,
    /// Number of lattice cells `L³` (= `n_cells`, the density length).
    pub n_cells: usize,
    /// Solver/mode label — always `"chern_simons"`.
    pub mode_used: &'static str,
    /// Per-cell helicity density (length `n_cells`, sums to `helicity`)
    /// when `DENSITY` was requested; `None` otherwise.
    pub density: Option<Vec<f64>>,
}

// ── Site index helpers (Hallie's vid; reimplemented, not imported —
//    cubic.rs uses the reversed stride order and its closures are
//    private) ────────────────────────────────────────────────────────

/// Decode `vid` into `(i, j, k)` under `vid = (i·L + j)·L + k`.
#[inline]
fn decode(vid: i64, l: i64) -> (i64, i64, i64) {
    let k = vid % l;
    let t = vid / l;
    let j = t % l;
    let i = t / l;
    (i, j, k)
}

/// Encode `(i, j, k)` → `vid`.
#[inline]
fn encode(i: i64, j: i64, k: i64, l: i64) -> i64 {
    (i * l + j) * l + k
}

/// The `+ê` neighbor of `vid` along `axis` (0 = x/i, 1 = y/j, 2 = z/k),
/// periodic wrap.
#[inline]
fn shift_plus(vid: i64, axis: u8, l: i64) -> i64 {
    let (mut i, mut j, mut k) = decode(vid, l);
    match axis {
        0 => i = (i + 1) % l,
        1 => j = (j + 1) % l,
        _ => k = (k + 1) % l,
    }
    encode(i, j, k, l)
}

/// Float-tolerant vertex-id decode (mirrors `spectral::decode_vertex_id`
/// — numpy/torch emit ids as floats, so the tolerance is load-bearing).
fn decode_vertex_id(rec: &Record, field: &str) -> Result<i64, String> {
    match rec.get(field) {
        Some(v) => {
            if let Some(id) = v.as_i64() {
                Ok(id)
            } else if let Some(f) = v.as_f64() {
                Ok(f.round() as i64)
            } else {
                Err(format!(
                    "HELICITY: edge-endpoint field `{field}` is non-numeric ({v:?}) — \
                     vertex ids must be integers (or integer-valued floats)"
                ))
            }
        }
        None => Err(format!(
            "HELICITY: record missing edge-endpoint field `{field}` — every edge record \
             must carry both vertex_a and vertex_b"
        )),
    }
}

/// Compute the discrete Chern-Simons helicity of an edge-endpoint bundle.
///
/// Reuses the MODE MATRIX bundle-read pattern (endpoint-column check +
/// float-tolerant `decode_vertex_id` + scalar fiber read), but decodes
/// the raw `vid` into `(i,j,k)` (Hallie's convention) and routes each
/// forward edge into `A_x/A_y/A_z[site]` by the axis it steps along —
/// it does NOT re-index vertices densely. Returns a typed `String`
/// error on any non-cubic / non-3D / partial / malformed bundle.
pub fn helicity_chern_simons(
    store: &BundleStore,
    a_field: &str,
    want_density: bool,
) -> Result<HelicityResult, String> {
    // ── Step 1: confirm the edge-endpoint columns exist.
    let endpoint_a = "vertex_a";
    let endpoint_b = "vertex_b";
    let has_a = store.schema.base_fields.iter().any(|f| f.name == endpoint_a);
    let has_b = store.schema.base_fields.iter().any(|f| f.name == endpoint_b);
    if !has_a || !has_b {
        return Err(format!(
            "HELICITY: bundle missing edge-endpoint fields {endpoint_a}/{endpoint_b} — the \
             edge-endpoint schema requires explicit vertex_a/vertex_b in base_fields"
        ));
    }

    // ── Step 2: single pass — collect (va, vb, a_e) + track max vid.
    let mut edges: Vec<(i64, i64, f64)> = Vec::new();
    let mut max_vid: i64 = -1;
    for rec in store.records() {
        let va = decode_vertex_id(&rec, endpoint_a)?;
        let vb = decode_vertex_id(&rec, endpoint_b)?;
        if va < 0 || vb < 0 {
            return Err(format!(
                "HELICITY: negative vertex id ({va} → {vb}) — vertex ids must be ≥ 0"
            ));
        }
        let a_e = rec.get(a_field).and_then(|v| v.as_f64()).unwrap_or(0.0);
        max_vid = max_vid.max(va).max(vb);
        edges.push((va, vb, a_e));
    }
    if edges.is_empty() {
        return Err(
            "HELICITY: empty edge set — the bundle carries no edge records".to_string(),
        );
    }

    // ── Step 3: infer L, validate cubic 3D (N4).
    let v_count_i = max_vid + 1;
    let l = (v_count_i as f64).cbrt().round() as i64;
    if l < 2 {
        return Err(format!(
            "HELICITY: inferred lattice side L = {l} < 2 (max vertex id {max_vid}) — need a \
             periodic cubic L³ lattice with L ≥ 2"
        ));
    }
    if l * l * l != v_count_i {
        return Err(format!(
            "HELICITY: bundle is not a cubic 3D lattice — max vertex id {max_vid} gives V = \
             {v_count_i}, but ⌊∛V⌉³ = {} ≠ V. HELICITY requires a periodic cubic L³ \
             edge bundle (vid(i,j,k) = (i·L+j)·L+k)",
            l * l * l
        ));
    }
    let v_count = v_count_i as usize;

    // ── Step 4: route each forward edge into A_x/A_y/A_z by direction.
    //   The site index is the raw tail vid (0..L³), NOT a first-seen
    //   counter. Direction d = the single axis whose wrap-aware delta is
    //   +1; anything else is a malformed (non-forward-unit) edge.
    let mut ax = vec![0.0f64; v_count];
    let mut ay = vec![0.0f64; v_count];
    let mut az = vec![0.0f64; v_count];
    let mut seen_x = vec![false; v_count];
    let mut seen_y = vec![false; v_count];
    let mut seen_z = vec![false; v_count];

    for (va, vb, a_e) in edges.iter().copied() {
        let (ia, ja, ka) = decode(va, l);
        let (ib, jb, kb) = decode(vb, l);
        // wrap-aware per-axis delta in 0..L
        let dx = ((ib - ia) % l + l) % l;
        let dy = ((jb - ja) % l + l) % l;
        let dz = ((kb - ka) % l + l) % l;
        let s = va as usize;
        match (dx, dy, dz) {
            (1, 0, 0) => {
                if seen_x[s] {
                    return Err(format!(
                        "HELICITY: duplicate +x edge at site {va} — each site carries exactly \
                         one forward edge per axis"
                    ));
                }
                seen_x[s] = true;
                ax[s] = a_e;
            }
            (0, 1, 0) => {
                if seen_y[s] {
                    return Err(format!(
                        "HELICITY: duplicate +y edge at site {va} — each site carries exactly \
                         one forward edge per axis"
                    ));
                }
                seen_y[s] = true;
                ay[s] = a_e;
            }
            (0, 0, 1) => {
                if seen_z[s] {
                    return Err(format!(
                        "HELICITY: duplicate +z edge at site {va} — each site carries exactly \
                         one forward edge per axis"
                    ));
                }
                seen_z[s] = true;
                az[s] = a_e;
            }
            _ => {
                return Err(format!(
                    "HELICITY: edge ({va} → {vb}) is not a unit forward step along exactly one \
                     axis (Δ = {dx},{dy},{dz} under vid(i,j,k)=(i·L+j)·L+k, L={l}) — the bundle \
                     must be a forward-edge cubic lattice (3 edges/site: +x,+y,+z)"
                ));
            }
        }
    }

    // ── Step 5: every site must carry all three forward edges (a 2D or
    //   partial bundle fails here even when V is a perfect cube).
    let n_x = seen_x.iter().filter(|&&b| b).count();
    let n_y = seen_y.iter().filter(|&&b| b).count();
    let n_z = seen_z.iter().filter(|&&b| b).count();
    if n_x != v_count || n_y != v_count || n_z != v_count {
        return Err(format!(
            "HELICITY: incomplete forward-edge bundle — populated (+x,+y,+z) = \
             ({n_x},{n_y},{n_z}) of {v_count} sites each. A periodic cubic L³ lattice emits \
             exactly 3·L³ = {} forward edges (one per axis per site); a 2D or partial lattice \
             is rejected",
            3 * v_count
        ));
    }
    let n_edges_used = edges.len();

    // ── Step 6: contract H = Σ_s A_d(s)·Ω_d(s) with the pinned co-located
    //   plaquette circulations. density[s] = the bracketed per-cell term.
    let mut helicity = 0.0f64;
    let mut density = if want_density {
        vec![0.0f64; v_count]
    } else {
        Vec::new()
    };
    for s in 0..v_count {
        let sv = s as i64;
        let sx = shift_plus(sv, 0, l) as usize;
        let sy = shift_plus(sv, 1, l) as usize;
        let sz = shift_plus(sv, 2, l) as usize;
        // Ω_x = yz-plaquette; Ω_y = zx-plaquette; Ω_z = xy-plaquette.
        let omega_x = ay[s] + az[sy] - ay[sz] - az[s];
        let omega_y = az[s] + ax[sz] - az[sx] - ax[s];
        let omega_z = ax[s] + ay[sx] - ax[sy] - ay[s];
        let cell = ax[s] * omega_x + ay[s] * omega_y + az[s] * omega_z;
        helicity += cell;
        if want_density {
            density[s] = cell;
        }
    }

    Ok(HelicityResult {
        helicity,
        n_edges_used,
        n_cells: v_count,
        mode_used: "chern_simons",
        density: if want_density { Some(density) } else { None },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::BundleStore;
    use crate::types::{BundleSchema, FieldDef, Value};
    use std::f64::consts::PI;

    fn edge_store() -> BundleStore {
        let schema = BundleSchema::new("h")
            .base(FieldDef::numeric("vertex_a"))
            .base(FieldDef::numeric("vertex_b"))
            .fiber(FieldDef::numeric("a_e"));
        BundleStore::new(schema)
    }

    fn push(store: &mut BundleStore, va: i64, vb: i64, ae: f64) {
        let mut r = Record::new();
        r.insert("vertex_a".into(), Value::Integer(va));
        r.insert("vertex_b".into(), Value::Integer(vb));
        r.insert("a_e".into(), Value::Float(ae));
        store.insert(&r);
    }

    fn vid(i: i64, j: i64, k: i64, l: i64) -> i64 {
        (i * l + j) * l + k
    }

    /// Build a full periodic cubic L³ forward-edge store from a per-site
    /// closure returning (a_x, a_y, a_z).
    fn build(l: i64, a_of: impl Fn(i64, i64, i64) -> (f64, f64, f64)) -> BundleStore {
        let mut s = edge_store();
        for i in 0..l {
            for j in 0..l {
                for k in 0..l {
                    let site = vid(i, j, k, l);
                    let (x, y, z) = a_of(i, j, k);
                    push(&mut s, site, vid((i + 1) % l, j, k, l), x);
                    push(&mut s, site, vid(i, (j + 1) % l, k, l), y);
                    push(&mut s, site, vid(i, j, (k + 1) % l, l), z);
                }
            }
        }
        s
    }

    fn abc(l: i64) -> impl Fn(i64, i64, i64) -> (f64, f64, f64) {
        let h = 2.0 * PI / (l as f64);
        move |i, j, k| {
            let (x, y, z) = (i as f64 * h, j as f64 * h, k as f64 * h);
            ((z.sin() + y.cos()) * h, (x.sin() + z.cos()) * h, (y.sin() + x.cos()) * h)
        }
    }

    #[test]
    fn ut_abc_l8_closed_form() {
        let s = build(8, abc(8));
        let r = helicity_chern_simons(&s, "a_e", false).unwrap();
        let expected = 12.0 * PI * PI * 8.0 * (2.0 * PI / 8.0).sin();
        assert!((r.helicity - expected).abs() < 1e-6, "{} vs {expected}", r.helicity);
        assert_eq!(r.n_cells, 512);
        assert_eq!(r.n_edges_used, 3 * 512);
        assert_eq!(r.mode_used, "chern_simons");
        assert!(r.density.is_none());
    }

    #[test]
    fn ut_chirality_sign_exact_mirror() {
        // right-handed (∇×A=+A): A=(0, sin x, cos x)·h ⇒ +4π²L sin(2π/L)
        let h4 = 2.0 * PI / 4.0;
        let rh = build(4, move |i, _j, _k| {
            let x = i as f64 * h4;
            (0.0, x.sin() * h4, x.cos() * h4)
        });
        let lh = build(4, move |i, _j, _k| {
            let x = i as f64 * h4;
            (0.0, x.cos() * h4, x.sin() * h4)
        });
        let hr = helicity_chern_simons(&rh, "a_e", false).unwrap().helicity;
        let hl = helicity_chern_simons(&lh, "a_e", false).unwrap().helicity;
        assert!(hr > 0.0 && hl < 0.0, "right +, left −: {hr}, {hl}");
        assert!((hr - 16.0 * PI * PI).abs() < 1e-9, "L=4 right = +16π², got {hr}");
        assert!((hr + hl).abs() < 1e-9, "exact mirror antisymmetry");
    }

    #[test]
    fn ut_zero_field() {
        let s = build(4, |_i, _j, _k| (0.0, 0.0, 0.0));
        assert!(helicity_chern_simons(&s, "a_e", false).unwrap().helicity.abs() < 1e-15);
    }

    #[test]
    fn ut_non_cubic_errors() {
        let mut s = edge_store();
        push(&mut s, 0, 1, 1.0);
        push(&mut s, 2, 3, 1.0);
        push(&mut s, 4, 5, 1.0); // V = 6, not a cube
        assert!(helicity_chern_simons(&s, "a_e", false).is_err());
    }

    #[test]
    fn ut_density_sums_to_scalar() {
        let s = build(6, abc(6));
        let scalar = helicity_chern_simons(&s, "a_e", false).unwrap().helicity;
        let r = helicity_chern_simons(&s, "a_e", true).unwrap();
        let d = r.density.expect("density present");
        assert_eq!(d.len(), r.n_cells);
        let sum: f64 = d.iter().sum();
        assert!((sum - scalar).abs() < 1e-9, "Σ density {sum} vs H {scalar}");
    }
}
