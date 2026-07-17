//! HOLONOMY AROUND CYCLE — Poincaré Tier 1 readout verb (SU(2)).
//!
//! Davis–Poincaré Thm 3.6: holonomy-trivial ⟺ π₁ = 0. On a lens space
//! `L(p, q) = S³/ℤ_p` realized as a periodic cubic lattice with a
//! twisted boundary condition `Ω = exp(2πi·q·σ₃/p)` on the z-wrap links,
//! `π₁ = ℤ/p` lives in the SU(2) holonomy around the non-contractible
//! z-cycle. This module is the readout: it builds the ordered edge list
//! of a named lattice cycle (AXIS form) or an explicit edge-id list
//! (EDGES form) and hands it to the existing group-erased walker
//! [`crate::gauge::holonomy::walk_loop`] — NO new group math. The walker
//! is untouched; we only adapt at the call site.
//!
//! ── DIRECTION CONVENTION (load-bearing; pinned by test H3) ────────────
//!
//! AXIS walks **+axis order**: the ordered vertex cycle is
//! `[site(t=0), site(t=1), …, site(t=L-1)]` with the axis coordinate
//! running 0..L and the two transverse coordinates fixed, closing via
//! the wrap pair `site(L-1) → site(0)`. Each consecutive pair is
//! resolved through `Lattice::resolve_edge`, which returns
//! `EdgeOrientation::Forward` when the stored lattice edge already runs
//! in the walk direction and `EdgeOrientation::Reverse` otherwise. A
//! Forward edge contributes the canonical link `U`; a Reverse edge
//! contributes `U†` (the walker reads `edge_element(eid, Reverse) =
//! U.inverse()`). On a periodic cubic the axis links are stored
//! `(s, s+ê_axis)` in +axis order, so a +axis walk matches every stored
//! edge and the z-wrap link is read **Forward** — the loop product is
//! `Ω`, NOT `Ω†`. A reversed convention would silently read every class
//! `p` as its inverse (`re_trace` is even in the axis, so the order is
//! unchanged, but the quaternion axis sign flips). H3 pins this by
//! showing that reversing a loop (reverse order + inverted links) yields
//! the conjugate quaternion `(q0, −q1, −q2, −q3)`.
//!
//! ── order_estimate ───────────────────────────────────────────────────
//!
//! Best-effort: the nearest integer `p` such that the SU(2) element has
//! order `p`. For `g = (cos φ, sin φ·n̂)`, `gⁿ = (cos nφ, sin nφ·n̂) =
//! identity` iff `nφ ≡ 0 (mod 2π)`, so the order is the denominator of
//! `x = arccos(q0)/(2π)` in lowest terms. We recover it by
//! continued-fraction (Stern–Brocot) rational approximation of `x` with
//! an absolute tolerance `TOL = 1e-9` and a denominator cap
//! `Q_MAX = 512`; identity (`q0 ≥ 1 − TOL`) returns 1. Branch-robust:
//! `arccos` folds `φ ↔ 2π−φ`, but the denominators of `x` and `1−x` are
//! identical, so a clean lens wrap `Ω = (cos 2πq/p, 0, 0, sin 2πq/p)`
//! yields `order = p/gcd(p, q) = p` when `gcd(q, p) = 1`. The client can
//! re-derive this from `re_trace`; the field is a convenience.
//!
//! Only meaningful on a clean lens wrap (a pure σ-twist). A *generic*
//! SU(2) holonomy still returns a bounded integer (`≤ Q_MAX`) rather than
//! a distinguished sentinel — there is no in-row flag separating an
//! order-`p` element from a generic one beyond `re_trace` itself, so read
//! `order_estimate` only alongside a `re_trace` you already expect to be
//! `cos(2πq/p)`.

#![cfg(feature = "gauge")]

use crate::gauge::edge_connection::EdgeConnection;
use crate::gauge::group::Group;
use crate::gauge::group_element::GroupElement;
use crate::gauge::holonomy::walk_loop;
use crate::lattice::{EdgeId, EdgeOrientation, Lattice};
use crate::parser::{CycleSpec, ExecResult};
use crate::types::{Record, Value};

/// Absolute tolerance for the order-estimate rational approximation.
/// The lens fixtures pin `re_trace` to 1e-12, so 1e-9 leaves generous
/// margin while rejecting numerical noise.
const ORDER_TOL: f64 = 1e-9;
/// Denominator cap for the order-estimate rational approximation —
/// comfortably above the fixture `p ∈ {2, 3, 5, 7}` and the probe p=5.
const ORDER_Q_MAX: u64 = 512;

/// Parsed `(L, D)` of a CUBIC lattice, pulled off its topology hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CubicHint {
    /// Per-axis vertex count `L`.
    pub l: usize,
    /// Dimension `D`.
    pub d: usize,
}

/// Read the leading digit run immediately following `marker` in `s` and
/// parse it as a `usize`. Returns `None` when the marker is absent or is
/// not followed by a digit.
fn digits_after(s: &str, marker: &str) -> Option<usize> {
    let idx = s.find(marker)?;
    let rest = &s[idx + marker.len()..];
    let end = rest
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit())
        .map(|(i, _)| i)
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    rest[..end].parse::<usize>().ok()
}

/// Detect a CUBIC lattice and extract `(L, D)` from its topology hint.
///
/// The cubic constructor sets `topology = "CUBIC_L{L}_D{D}"` (periodic),
/// `"CUBIC_L{L}_D{D}_OBC_AXIS{k}"` (single-axis OBC), or
/// `"CUBIC_L{L}_D{D}_OPEN"` (fully-open). All three carry the `CUBIC_L`
/// prefix and the `_D` marker, so the same parse serves every case.
/// Non-cubic hints (`"S2"`, `"T2"`, …) return `None`.
pub fn parse_cubic_hint(topology: Option<&str>) -> Option<CubicHint> {
    let t = topology?;
    if !t.starts_with("CUBIC_L") {
        return None;
    }
    let l = digits_after(t, "CUBIC_L")?;
    let d = digits_after(t, "_D")?;
    Some(CubicHint { l, d })
}

/// Best-effort integer order of an SU(2) element from its scalar part.
///
/// See the module docstring for the math + tolerance contract. Returns
/// 1 for the identity (`q0 ≥ 1 − ORDER_TOL`).
pub fn order_estimate(q0: f64) -> i64 {
    let q0c = q0.clamp(-1.0, 1.0);
    if q0c >= 1.0 - ORDER_TOL {
        return 1;
    }
    let phi = q0c.acos(); // ∈ [0, π]
    let x = phi / (2.0 * std::f64::consts::PI); // ∈ [0, 0.5]
    nearest_rational_denominator(x, ORDER_TOL, ORDER_Q_MAX) as i64
}

/// Denominator of the best rational approximation of `x` via the
/// continued-fraction (convergent) expansion, stopping at the first
/// convergent within `tol` or when the denominator would exceed
/// `q_max`. Convergent denominators are already in lowest terms.
fn nearest_rational_denominator(x: f64, tol: f64, q_max: u64) -> u64 {
    let mut xx = x;
    // Convergent recurrence: h_i / k_i with seeds h_{-1}=1,h_{-2}=0,
    // k_{-1}=0,k_{-2}=1.
    let mut h_prev1: i64 = 1;
    let mut h_prev2: i64 = 0;
    let mut k_prev1: i64 = 0;
    let mut k_prev2: i64 = 1;
    let mut best_k: i64 = 1;
    for _ in 0..64 {
        if !xx.is_finite() {
            break;
        }
        let ai = xx.floor();
        let ai_i = ai as i64;
        let h = ai_i.saturating_mul(h_prev1).saturating_add(h_prev2);
        let k = ai_i.saturating_mul(k_prev1).saturating_add(k_prev2);
        if k > q_max as i64 {
            break;
        }
        if k > 0 {
            best_k = k;
            let approx = h as f64 / k as f64;
            if (x - approx).abs() < tol {
                return k as u64;
            }
        }
        let frac = xx - ai;
        if frac.abs() < 1e-15 {
            // x is (numerically) exactly h/k — this convergent is final.
            return best_k.max(1) as u64;
        }
        xx = 1.0 / frac;
        h_prev2 = h_prev1;
        h_prev1 = h;
        k_prev2 = k_prev1;
        k_prev1 = k;
    }
    best_k.max(1) as u64
}

/// Build the ordered `(EdgeId, EdgeOrientation)` list for the AXIS-form
/// cycle: all links along `axis` at the fixed `transverse` coordinates,
/// in +axis order, closing via the wrap edge.
///
/// `l` / `d` come from the bound lattice's CUBIC topology hint; the two
/// transverse coordinates fill the `d - 1` non-axis axes in ascending
/// index order. Each consecutive vertex pair (including the wrap pair)
/// is resolved through `Lattice::resolve_edge`, so the returned
/// orientation is correct by construction (Forward when the stored edge
/// already runs +axis). See the module docstring for the convention.
pub fn axis_cycle_edges(
    lat: &Lattice,
    l: usize,
    d: usize,
    axis: usize,
    transverse: &[usize],
) -> Result<Vec<(EdgeId, EdgeOrientation)>, String> {
    if axis >= d {
        return Err(format!(
            "HOLONOMY AROUND CYCLE AXIS: axis index {axis} out of range for \
             DIM={d} (valid axes are 0..{d})"
        ));
    }
    if transverse.len() != d - 1 {
        return Err(format!(
            "HOLONOMY AROUND CYCLE AXIS: a DIM={d} cubic cycle needs {} fixed \
             transverse coordinate(s), got {} — the AT (c0, c1) form pins a \
             single non-contractible loop on a 3D lattice (DIM=3)",
            d - 1,
            transverse.len()
        ));
    }
    // Row-major strides: stride[k] = L^k.
    let mut stride = vec![1usize; d];
    for k in 1..d {
        stride[k] = stride[k - 1] * l;
    }
    // Non-axis axes in ascending index order — the transverse coords map
    // onto these positionally.
    let non_axis: Vec<usize> = (0..d).filter(|&k| k != axis).collect();
    for (i, &c) in transverse.iter().enumerate() {
        if c >= l {
            return Err(format!(
                "HOLONOMY AROUND CYCLE AXIS: transverse coordinate {c} on axis \
                 {} out of range for L={l} (valid 0..{l})",
                non_axis[i]
            ));
        }
    }
    // Site id for axis coordinate `t` with the transverse coords fixed.
    let site_of = |t: usize| -> usize {
        let mut s = t * stride[axis];
        for (i, &k) in non_axis.iter().enumerate() {
            s += transverse[i] * stride[k];
        }
        s
    };
    let cycle: Vec<usize> = (0..l).map(site_of).collect();
    let mut edges = Vec::with_capacity(l);
    for i in 0..l {
        let a = cycle[i];
        let b = cycle[(i + 1) % l];
        let (eid, orient) = lat.resolve_edge(a, b).ok_or_else(|| {
            format!(
                "HOLONOMY AROUND CYCLE AXIS: no lattice edge between sites {a} \
                 and {b} along axis {axis} — the wrap link is absent (is the \
                 lattice periodic on this axis? single-axis OBC drops it)"
            )
        })?;
        edges.push((eid, orient));
    }
    Ok(edges)
}

/// Build the ordered `(EdgeId, EdgeOrientation)` list for the EDGES form:
/// the explicit edge-id list, each taken Forward, product in list order.
/// Validates every id against the field's edge count so an out-of-range
/// id surfaces as a typed error instead of an OOB panic in the walker.
pub fn edges_cycle_edges(
    edge_ids: &[usize],
    n_edges: usize,
) -> Result<Vec<(EdgeId, EdgeOrientation)>, String> {
    if edge_ids.is_empty() {
        return Err(
            "HOLONOMY AROUND CYCLE EDGES: the edge list is empty — supply the \
             ordered edge ids of a closed loop"
                .to_string(),
        );
    }
    let mut out = Vec::with_capacity(edge_ids.len());
    for &e in edge_ids {
        if e >= n_edges {
            return Err(format!(
                "HOLONOMY AROUND CYCLE EDGES: edge id {e} out of range — the \
                 gauge field has {n_edges} edges (valid ids 0..{n_edges})"
            ));
        }
        out.push((e, EdgeOrientation::Forward));
    }
    Ok(out)
}

/// Execute a `HOLONOMY <field> AROUND CYCLE …` statement.
///
/// Resolves the gauge field through the process registry, gates the
/// group to SU(2) BEFORE walking (a non-SU(2) field would panic inside
/// the walker's `compose`/`read_element`), builds the ordered edge list
/// per the cycle spec, walks it with the untouched
/// [`walk_loop`], and returns a single row
/// `{ q0, q1, q2, q3, re_trace, order_estimate, group_used }`.
///
/// Uses the process-global gauge + lattice registries (the same source
/// of truth CHERN_CLASS reads), so it is independent of any `Engine`
/// handle — callable identically from `parser::execute`, the
/// `/v1/gql` topology dispatcher, and the streaming executor.
pub fn execute_holonomy_cycle(
    field: &str,
    spec: &CycleSpec,
) -> Result<ExecResult, String> {
    // 1. Resolve the gauge field handle by name.
    let handle = crate::gauge::registry::get(field).ok_or_else(|| {
        format!(
            "HOLONOMY AROUND CYCLE: gauge field '{field}' not declared \
             (use GAUGE_FIELD {field} ON LATTICE ... first, or INGEST ... \
             AS GAUGE_FIELD)"
        )
    })?;

    // 2. Group gate — SU(2)-only this phase. MUST precede walk_loop: a
    //    non-SU(2) buffer panics inside compose / read_element, so the
    //    gate turns that programming-error path into a clean typed error
    //    (H6 + live probe P6).
    let group = handle.group();
    if group != Group::SU2 {
        return Err(format!(
            "HOLONOMY AROUND CYCLE requires GROUP SU(2) in this phase \
             (quaternion readout); got {}",
            group.label()
        ));
    }

    // 3. Build the ordered edge list per the cycle spec.
    let edges: Vec<(EdgeId, EdgeOrientation)> = match spec {
        CycleSpec::Edges(ids) => {
            let n_edges = handle.as_dense_buffer().n_edges;
            edges_cycle_edges(ids, n_edges)?
        }
        CycleSpec::Axis { axis, c0, c1 } => {
            // AXIS form needs the field's bound lattice to enumerate the
            // loop's ordered edges.
            let lat_name = handle.lattice_name().to_string();
            let lat = crate::lattice::registry::get(&lat_name).ok_or_else(|| {
                format!(
                    "HOLONOMY AROUND CYCLE AXIS: lattice '{lat_name}' bound to \
                     gauge field '{field}' not found — the AXIS form needs the \
                     bound lattice to enumerate the loop (was the lattice \
                     declared? use the EDGES form for a lattice-free loop)"
                )
            })?;
            let hint = parse_cubic_hint(lat.topology.as_deref()).ok_or_else(|| {
                format!(
                    "HOLONOMY AROUND CYCLE AXIS: gauge field '{field}' is bound \
                     to lattice '{lat_name}' whose topology '{}' is not CUBIC — \
                     the AXIS form requires a CUBIC lattice binding to walk a \
                     named axis cycle (use the EDGES form for an arbitrary loop)",
                    lat.topology.as_deref().unwrap_or("<none>")
                )
            })?;
            axis_cycle_edges(&lat, hint.l, hint.d, *axis, &[*c0, *c1])?
        }
    };

    // 4. Walk the ordered loop through the UNTOUCHED group-erased walker.
    //    `walk_loop` ignores its `_lattice` argument (present only for
    //    API symmetry with `face_edges`), so a throwaway lattice is safe
    //    and avoids re-resolving one for the EDGES form.
    let throwaway = Lattice::new("", 0, Vec::new(), Vec::new(), None);
    let conn: &dyn EdgeConnection = handle.as_ref();
    let holonomy = walk_loop(&throwaway, &edges, conn);

    // 5. Extract the SU(2) quaternion row. re_trace = ½·Tr(U) = q0.
    let (q0, q1, q2, q3) = match holonomy {
        GroupElement::SU2 { q0, q1, q2, q3 } => (q0, q1, q2, q3),
        _ => {
            return Err(
                "HOLONOMY AROUND CYCLE: walker returned a non-SU(2) element \
                 (internal invariant violated — the group gate should have \
                 rejected this)"
                    .to_string(),
            )
        }
    };
    let order = order_estimate(q0);

    let mut row = Record::new();
    row.insert("q0".to_string(), Value::Float(q0));
    row.insert("q1".to_string(), Value::Float(q1));
    row.insert("q2".to_string(), Value::Float(q2));
    row.insert("q3".to_string(), Value::Float(q3));
    row.insert("re_trace".to_string(), Value::Float(q0));
    row.insert("order_estimate".to_string(), Value::Integer(order));
    row.insert(
        "group_used".to_string(),
        Value::Text(group.label().to_string()),
    );
    Ok(ExecResult::Rows(vec![row]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_estimate_identity_is_one() {
        assert_eq!(order_estimate(1.0), 1);
        // Just inside the identity tolerance.
        assert_eq!(order_estimate(1.0 - 1e-10), 1);
    }

    #[test]
    fn order_estimate_nails_lens_fixtures() {
        use std::f64::consts::PI;
        let cases = [(2usize, 1usize), (3, 1), (5, 1), (5, 2), (7, 1), (7, 3)];
        for (p, q) in cases {
            let q0 = (2.0 * PI * q as f64 / p as f64).cos();
            assert_eq!(
                order_estimate(q0),
                p as i64,
                "order for (p={p}, q={q}) with q0={q0}"
            );
        }
    }

    #[test]
    fn parse_cubic_hint_reads_l_and_d() {
        assert_eq!(
            parse_cubic_hint(Some("CUBIC_L5_D3")),
            Some(CubicHint { l: 5, d: 3 })
        );
        assert_eq!(
            parse_cubic_hint(Some("CUBIC_L24_D4_OBC_AXIS0")),
            Some(CubicHint { l: 24, d: 4 })
        );
        assert_eq!(parse_cubic_hint(Some("S2")), None);
        assert_eq!(parse_cubic_hint(None), None);
    }

    #[test]
    fn edges_form_rejects_out_of_range_and_empty() {
        assert!(edges_cycle_edges(&[], 8).is_err());
        assert!(edges_cycle_edges(&[0, 1, 8], 8).is_err());
        assert!(edges_cycle_edges(&[0, 1, 7], 8).is_ok());
    }
}
