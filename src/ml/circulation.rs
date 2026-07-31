//! CIRCULATION — Hodge potential / magnetic-Laplacian directed-flow analysis.
//!
//! Extracted mechanically from `src/bin/gigi_stream.rs` (stream-extraction
//! phase 1). The HTTP handler stays in the binary as a thin wrapper.

use axum::http::StatusCode;
use serde::Deserialize;

use crate::engine::Engine;

/// Request for `POST /v1/bundles/{name}/circulation`.
#[derive(Deserialize)]
pub struct CirculationRequest {
    /// Categorical field naming each edge's source node.
    pub source: String,
    /// Categorical field naming each edge's target node.
    pub target: String,
    /// Numeric field for the directed flow value (default: 1.0 per edge).
    #[serde(default)]
    pub weight: Option<String>,
    /// Magnetic charge γ — flux sensitivity of the Hermitian Laplacian (default 1.0).
    #[serde(default = "default_circulation_charge")]
    pub charge: f64,
    /// How many top cyclic edges to return (default 10).
    #[serde(default = "default_circulation_max_cycles")]
    pub max_cycles: usize,
}
pub fn default_circulation_charge() -> f64 { 1.0 }
pub fn default_circulation_max_cycles() -> usize { 10 }

/// Result of a directed-flow circulation analysis.
#[derive(Debug)]
pub struct CirculationResult {
    pub n_nodes: usize,
    pub n_edges: usize,
    /// Fraction of flow energy that is cyclic — unexplainable by ANY ranking (0 =
    /// perfectly rankable gradient, 1 = pure circulation).
    pub circulation_ratio: f64,
    /// Smallest eigenvalue of the normalized magnetic Laplacian at charge γ — the
    /// spectral flux witness (0 = balanced/gradient, > 0 = net directed circulation).
    pub magnetic_gap: f64,
    pub charge: f64,
    /// Per-node potential (the gradient/ranking part), sorted high→low.
    pub potential: Vec<(String, f64)>,
    /// Top cyclic edges by residual flow (source, target, flow, residual).
    pub cyclic_edges: Vec<(String, String, f64, f64)>,
    pub reads: Vec<String>,
    pub notes: Vec<String>,
}

/// Directed-flow circulation analysis (rung two: the magnetic-Laplacian / Hodge lens).
///
/// Given directed weighted edges (a flow, pairwise comparisons, money movement…), split
/// the flow into the part a global **ranking/potential** explains (the gradient) and the
/// part **no** ranking can (the **circulation** — directed loops). Reports:
/// - `potential`: each node's score — a HodgeRank-style global ranking (least-squares
///   solve of the graph Laplacian `L φ = div`, i.e. the discrete Dirichlet/harmonic
///   extension of the flow),
/// - `circulation_ratio`: residual cyclic energy `‖flow − ∇φ‖² / ‖flow‖²`,
/// - `magnetic_gap`: smallest eigenvalue of the normalized **magnetic (Hermitian)
///   Laplacian** at charge γ — the U(1)-gauge flux witness (cousin of the production
///   `spectral_gauge_gap`; the spectrum depends on the phases only through cycle fluxes),
/// - `cyclic_edges`: the loop-carrying edges (fraud rings, feedback, inconsistencies).
///
/// Why this is uncopyable: undirected diffusion (`/infer method=diffusion`) symmetrizes
/// direction away and is **provably blind** to circulation. This is the holonomy/flux
/// signal — the directed, phase-carrying rung above bounded-harmonic label propagation.
pub fn circulation_flow(
    engine: &Engine,
    name: &str,
    source: &str,
    target: &str,
    weight: Option<String>,
    charge: f64,
    max_cycles: usize,
) -> Result<CirculationResult, (StatusCode, String)> {
    let store = engine.bundle(name).ok_or_else(|| (
        StatusCode::NOT_FOUND, format!("Bundle '{}' not found", name)))?;
    let schema = store.schema();
    let has_field = |f: &str| schema.base_fields.iter().chain(schema.fiber_fields.iter()).any(|d| d.name == f);
    if !has_field(source) { return Err((StatusCode::UNPROCESSABLE_ENTITY, format!("source field '{}' not found", source))); }
    if !has_field(target) { return Err((StatusCode::UNPROCESSABLE_ENTITY, format!("target field '{}' not found", target))); }
    if let Some(w) = &weight { if !has_field(w) {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!("weight field '{}' not found", w))); } }
    let records: Vec<crate::types::Record> = store.records().collect();

    // ── build the directed edge list + node index ──
    let mut idx: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut labels: Vec<String> = Vec::new();
    let mut node_of = |labels: &mut Vec<String>, idx: &mut std::collections::HashMap<String, usize>, s: String| -> usize {
        if let Some(&k) = idx.get(&s) { k } else { let k = labels.len(); idx.insert(s.clone(), k); labels.push(s); k }
    };
    let mut edges: Vec<(usize, usize, f64)> = Vec::new();
    let mut skipped = 0usize;
    for r in &records {
        let (s, t) = match (r.get(source), r.get(target)) {
            (Some(a), Some(b)) => (format!("{}", a), format!("{}", b)),
            _ => { skipped += 1; continue; }
        };
        if s == t { skipped += 1; continue; }   // self-loops carry no flow
        let w = weight.as_ref().and_then(|wf| r.get(wf)).and_then(|v| v.as_f64()).unwrap_or(1.0);
        let (i, j) = (node_of(&mut labels, &mut idx, s), node_of(&mut labels, &mut idx, t));
        edges.push((i, j, w));
    }
    // net reciprocal flows: flow is antisymmetric, so a→b (w₁) and b→a (w₂) combine into
    // ONE oriented edge with net w₁−w₂. Required for a well-defined magnetic Laplacian /
    // Hodge split (otherwise a pair's |Hᵢⱼ| ≠ 1 and the degree normalization breaks).
    {
        let mut net: std::collections::HashMap<(usize, usize), f64> = std::collections::HashMap::new();
        for &(i, j, w) in &edges {
            let (lo, hi, sgn) = if i < j { (i, j, 1.0) } else { (j, i, -1.0) };
            *net.entry((lo, hi)).or_insert(0.0) += sgn * w;
        }
        edges = net.into_iter().filter_map(|((lo, hi), f)|
            if f.abs() < 1e-12 { None } else if f > 0.0 { Some((lo, hi, f)) } else { Some((hi, lo, -f)) }
        ).collect();
        edges.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));   // deterministic order
    }
    let n = labels.len();
    let m = edges.len();
    if n < 2 || m < 1 {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "circulation needs >= 2 distinct nodes and >= 1 directed edge (got {n} nodes, {m} edges)")));
    }
    const MAX_N: usize = 20000;
    const MAX_M: usize = 200000;
    if n > MAX_N || m > MAX_M {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "circulation: {n} nodes / {m} edges exceeds the limit ({MAX_N}/{MAX_M}) in this version")));
    }
    let mut notes: Vec<String> = Vec::new();
    if skipped > 0 { notes.push(format!("skipped {skipped} record(s) with a missing endpoint or a self-loop")); }

    // ── Hodge potential: least-squares solve of the graph Laplacian L φ = div ──
    // (each directed edge contributes one undirected Laplacian edge; div_k = net inflow)
    let mut nbr: Vec<Vec<usize>> = vec![Vec::new(); n];   // undirected adjacency w/ multiplicity
    let mut div = vec![0.0f64; n];
    for &(i, j, w) in &edges {
        nbr[i].push(j); nbr[j].push(i);
        div[j] += w; div[i] -= w;
    }
    let deg: Vec<f64> = nbr.iter().map(|a| a.len() as f64).collect();
    let lap = |v: &[f64], out: &mut [f64]| {
        for k in 0..n { out[k] = deg[k] * v[k] - nbr[k].iter().map(|&j| v[j]).sum::<f64>(); }
    };
    // conjugate gradient on the SPD-on-mean-zero-complement system (div ⟂ constants)
    let mut phi = vec![0.0f64; n];
    let mut r = div.clone();                       // r = div − L·0
    let mut p = r.clone();
    let mut ap = vec![0.0f64; n];
    let mut rs: f64 = r.iter().map(|x| x * x).sum();
    for _ in 0..(n + 50).min(2000) {
        if rs.sqrt() < 1e-10 { break; }
        lap(&p, &mut ap);
        let denom: f64 = p.iter().zip(&ap).map(|(a, b)| a * b).sum();
        if denom.abs() < 1e-18 { break; }
        let alpha = rs / denom;
        for k in 0..n { phi[k] += alpha * p[k]; r[k] -= alpha * ap[k]; }
        let rs_new: f64 = r.iter().map(|x| x * x).sum();
        let beta = rs_new / rs.max(1e-300);
        for k in 0..n { p[k] = r[k] + beta * p[k]; }
        rs = rs_new;
    }
    let mean = phi.iter().sum::<f64>() / n as f64;
    for v in &mut phi { *v -= mean; }
    // residual (cyclic) flow per edge
    let residual: Vec<f64> = edges.iter().map(|&(i, j, w)| w - (phi[j] - phi[i])).collect();
    let flow_energy: f64 = edges.iter().map(|&(_, _, w)| w * w).sum::<f64>().max(1e-12);
    let circulation_ratio = (residual.iter().map(|x| x * x).sum::<f64>() / flow_energy).clamp(0.0, 1.0);

    // ── magnetic (Hermitian) Laplacian flux gap: smallest eigenvalue of the normalized
    //    L_sym = I − D^{-½} H D^{-½}, phases θ_e = γ·(w_e/max|w|). Balanced (gradient)
    //    ⇒ gap 0; frustrated (cyclic flux) ⇒ gap > 0. Cousin of spectral_gauge_gap. ──
    let wmax = edges.iter().map(|&(_, _, w)| w.abs()).fold(0.0f64, f64::max).max(1e-12);
    let dsqrt: Vec<f64> = deg.iter().map(|d| if *d > 0.0 { d.sqrt() } else { 1.0 }).collect();
    let theta: Vec<f64> = edges.iter().map(|&(_, _, w)| charge * (w / wmax)).collect();
    // S·v where S = D^{-½} H D^{-½} (complex); B = I + S is PSD, λ_max(B) = 2 − gap
    let s_apply = |vr: &[f64], vi: &[f64], or: &mut [f64], oi: &mut [f64]| {
        or.iter_mut().for_each(|x| *x = 0.0); oi.iter_mut().for_each(|x| *x = 0.0);
        for (e, &(i, j, _)) in edges.iter().enumerate() {
            let (c, s) = (theta[e].cos(), theta[e].sin());
            let scale = 1.0 / (dsqrt[i] * dsqrt[j]);
            // H_ij = e^{+iθ} acting on v_j → contributes to node i
            or[i] += scale * (c * vr[j] - s * vi[j]);
            oi[i] += scale * (s * vr[j] + c * vi[j]);
            // H_ji = e^{−iθ} acting on v_i → contributes to node j
            or[j] += scale * (c * vr[i] + s * vi[i]);
            oi[j] += scale * (-s * vr[i] + c * vi[i]);
        }
    };
    let mut lcg_seed: u64 = 0xC12C0FF1CE_u64 ^ (n as u64).wrapping_mul(2654435761);
    let mut lcg = || { lcg_seed = lcg_seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (lcg_seed >> 33) as f64 / (1u64 << 31) as f64 - 1.0 };
    let (mut vr, mut vi): (Vec<f64>, Vec<f64>) = ((0..n).map(|_| lcg()).collect(), (0..n).map(|_| lcg()).collect());
    let (mut sr, mut si) = (vec![0.0f64; n], vec![0.0f64; n]);
    let normalize = |vr: &mut [f64], vi: &mut [f64]| {
        let nrm = (vr.iter().map(|x| x * x).sum::<f64>() + vi.iter().map(|x| x * x).sum::<f64>()).sqrt();
        if nrm > 1e-18 { for x in vr.iter_mut() { *x /= nrm; } for x in vi.iter_mut() { *x /= nrm; } }
    };
    normalize(&mut vr, &mut vi);
    for _ in 0..400 {
        s_apply(&vr, &vi, &mut sr, &mut si);
        for k in 0..n { vr[k] += sr[k]; vi[k] += si[k]; }   // B·v = v + S·v
        normalize(&mut vr, &mut vi);
    }
    s_apply(&vr, &vi, &mut sr, &mut si);
    // Rayleigh quotient ⟨v, Bv⟩ = 1 + Re⟨v, Sv⟩ (real for a Hermitian operator)
    let re_vsv: f64 = (0..n).map(|k| vr[k] * sr[k] + vi[k] * si[k]).sum();
    let lambda_max_b = 1.0 + re_vsv;
    let magnetic_gap = (2.0 - lambda_max_b).max(0.0);

    // ── assemble outputs ──
    let round = |v: f64| (v * 100000.0).round() / 100000.0;
    let mut potential: Vec<(String, f64)> = (0..n).map(|k| (labels[k].clone(), round(phi[k]))).collect();
    potential.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut cyc: Vec<(usize, f64)> = (0..m).map(|e| (e, residual[e].abs())).collect();
    cyc.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let cyclic_edges: Vec<(String, String, f64, f64)> = cyc.iter().take(max_cycles)
        .filter(|(_, r)| *r > 1e-9)
        .map(|&(e, _)| { let (i, j, w) = edges[e]; (labels[i].clone(), labels[j].clone(), round(w), round(residual[e])) })
        .collect();

    let mut reads: Vec<String> = Vec::new();
    let pct = (circulation_ratio * 100.0).round();
    if circulation_ratio < 0.05 {
        reads.push(format!("Rankable flow: only {pct:.0}% is cyclic — a consistent global ranking (the potential) explains the directed flow. Use `potential` as a scoring/ordering."));
    } else if circulation_ratio > 0.5 {
        reads.push(format!("Frustrated flow: {pct:.0}% of the flow is circulation that NO ranking can explain — dominant directed loops. Inspect `cyclic_edges`."));
    } else {
        reads.push(format!("Mostly rankable with circulation pockets: {pct:.0}% cyclic. The ranking holds globally; `cyclic_edges` localizes the loops (feedback / rings / inconsistencies)."));
    }
    reads.push(format!("Magnetic flux gap = {} (charge γ={}): 0 = balanced/gradient, > 0 = net directed circulation. This is the holonomy/flux signal.", round(magnetic_gap), round(charge)));
    reads.push("Undirected diffusion symmetrizes direction away and is blind to this circulation — it is the rung-two, phase-carrying signal above bounded-harmonic label propagation.".into());
    notes.push("potential = HodgeRank-style least-squares ranking (discrete Dirichlet/harmonic extension of the flow); magnetic_gap = normalized magnetic-Laplacian smallest eigenvalue (U(1) gauge flux witness, cousin of spectral_gauge_gap).".into());
    notes.push(format!("charge γ controls flux sensitivity; very large γ × long cycles can alias past 2π — sweep γ if the gap looks saturated (γ={} used).", round(charge)));

    Ok(CirculationResult {
        n_nodes: n, n_edges: m, circulation_ratio: round(circulation_ratio),
        magnetic_gap: round(magnetic_gap), charge, potential, cyclic_edges, reads, notes,
    })
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::*;
    use crate::ml::test_support::{cleanup, scan_env, scan_rec};
    use crate::types::{BundleSchema, FieldDef, Value as V};

    /// CIRCULATION (rung two — directed flux). A pure-gradient DAG chain 0→1→…→5 is
    /// perfectly rankable: circulation ≈ 0, magnetic flux gap ≈ 0, and the recovered
    /// potential is monotone in the chain order.
    #[test]
    fn circulation_pure_gradient_is_rankable() {
        let rows: Vec<_> = (0..5).map(|i| scan_rec(&[
            ("id", V::Text(format!("e{i}"))),
            ("src", V::Text(format!("{i}"))), ("dst", V::Text(format!("{}", i + 1))),
            ("w", V::Float(1.0)),
        ])).collect();
        let schema = BundleSchema::new("chain")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::categorical("src")).fiber(FieldDef::categorical("dst"))
            .fiber(FieldDef::numeric("w"));
        let (dir, engine) = scan_env("circ_chain", "chain", schema, rows);
        let r = circulation_flow(&engine, "chain", "src", "dst", Some("w".into()), 1.0, 10).expect("circulation");
        assert!(r.circulation_ratio < 0.02, "a gradient chain is rankable, circ={}", r.circulation_ratio);
        assert!(r.magnetic_gap < 0.05, "a tree has no flux, gap={}", r.magnetic_gap);
        let pot: std::collections::HashMap<String, f64> = r.potential.iter().cloned().collect();
        assert!(pot["0"] < pot["5"], "potential must increase along the chain: {:?}", r.potential);
        assert!(pot["1"] < pot["4"], "potential monotone: {:?}", r.potential);
        cleanup(&dir);
    }

    /// CIRCULATION: a rock-paper-scissors 3-cycle (A→B→C→A) has NO consistent ranking —
    /// it is pure directed circulation. Circulation ≈ 1, magnetic flux gap > 0 (the
    /// signal the undirected graph is blind to), and the potentials collapse (unrankable).
    #[test]
    fn circulation_detects_directed_cycle() {
        let edges = [("A", "B"), ("B", "C"), ("C", "A")];
        let rows: Vec<_> = edges.iter().enumerate().map(|(i, (s, d))| scan_rec(&[
            ("id", V::Text(format!("e{i}"))),
            ("src", V::Text(s.to_string())), ("dst", V::Text(d.to_string())), ("w", V::Float(1.0)),
        ])).collect();
        let schema = BundleSchema::new("rps")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::categorical("src")).fiber(FieldDef::categorical("dst"))
            .fiber(FieldDef::numeric("w"));
        let (dir, engine) = scan_env("circ_rps", "rps", schema, rows);
        let r = circulation_flow(&engine, "rps", "src", "dst", Some("w".into()), 1.0, 10).expect("circulation");
        assert!(r.circulation_ratio > 0.9, "RPS is pure circulation, circ={}", r.circulation_ratio);
        assert!(r.magnetic_gap > 0.05, "a frustrated cycle has flux, gap={}", r.magnetic_gap);
        cleanup(&dir);
    }

    /// CIRCULATION localizes a fraud ring: a rankable chain with one small directed loop
    /// bolted on. The global ranking still holds (low-moderate circulation), and the ring
    /// edges surface as the top cyclic (highest-residual) edges.
    #[test]
    fn circulation_localizes_fraud_ring() {
        let mut rows: Vec<crate::types::Record> = Vec::new();
        let mut k = 0;
        let mut push = |rows: &mut Vec<crate::types::Record>, k: &mut usize, s: &str, d: &str| {
            rows.push(scan_rec(&[
                ("id", V::Text(format!("e{k}"))),
                ("src", V::Text(s.to_string())), ("dst", V::Text(d.to_string())), ("w", V::Float(1.0)),
            ])); *k += 1;
        };
        for i in 0..8 { push(&mut rows, &mut k, &format!("{i}"), &format!("{}", i + 1)); }  // chain
        push(&mut rows, &mut k, "4", "r0");                                                 // link in
        for (s, d) in [("r0", "r1"), ("r1", "r2"), ("r2", "r0")] { push(&mut rows, &mut k, s, d); }  // ring
        let schema = BundleSchema::new("fraud")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::categorical("src")).fiber(FieldDef::categorical("dst"))
            .fiber(FieldDef::numeric("w"));
        let (dir, engine) = scan_env("circ_fraud", "fraud", schema, rows);
        let r = circulation_flow(&engine, "fraud", "src", "dst", Some("w".into()), 1.0, 5).expect("circulation");
        assert!(r.circulation_ratio > 0.05, "the ring adds circulation, circ={}", r.circulation_ratio);
        assert!(r.magnetic_gap > 0.005, "the ring has flux, gap={}", r.magnetic_gap);
        let top: std::collections::HashSet<(String, String)> =
            r.cyclic_edges.iter().take(3).map(|(s, d, _, _)| (s.clone(), d.clone())).collect();
        for e in [("r0", "r1"), ("r1", "r2"), ("r2", "r0")] {
            assert!(top.contains(&(e.0.to_string(), e.1.to_string())),
                "ring edge {:?} should be a top cyclic edge; got {:?}", e, r.cyclic_edges);
        }
        cleanup(&dir);
    }

    /// CIRCULATION gives actionable errors, not panics.
    #[test]
    fn circulation_guards_are_actionable() {
        let rows: Vec<_> = (0..3).map(|i| scan_rec(&[
            ("id", V::Text(format!("e{i}"))),
            ("src", V::Text("A".into())), ("dst", V::Text("B".into())), ("w", V::Float(1.0)),
        ])).collect();
        let schema = BundleSchema::new("g")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::categorical("src")).fiber(FieldDef::categorical("dst"))
            .fiber(FieldDef::numeric("w"));
        let (dir, engine) = scan_env("circ_guard", "g", schema, rows);
        // unknown source field → clear error
        assert!(circulation_flow(&engine, "g", "nope", "dst", None, 1.0, 5).unwrap_err().1.contains("nope"));
        // only 1 distinct node (A→B is 2, but A→A collapses)… use missing bundle → 404
        assert_eq!(circulation_flow(&engine, "no", "src", "dst", None, 1.0, 5).unwrap_err().0, StatusCode::NOT_FOUND);
        cleanup(&dir);
    }
}
