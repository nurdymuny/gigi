//! Report-only basis support and projected omitted-shell residuals; not a spectral certificate.
use crate::basis::State;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use crate::model::Background;
use crate::operator::{ConeOperator, Mutation, Part};
use crate::ritz::{self, SolveSettings};

#[derive(Debug, Serialize)]
pub struct JointBin { pub occupation: [usize; 4], pub mass: f64 }
#[derive(Debug, Serialize)]
pub struct Distribution {
    pub mass: f64,
    pub shells: Vec<f64>,
    pub joint: Vec<JointBin>,
    pub mean_group_occupation: Option<[f64; 4]>,
    pub occupation_fraction: Option<[f64; 4]>,
}
pub struct ResidualSplit {
    pub residual: Vec<f64>, pub outer: Vec<f64>,
    pub inner_norm: f64, pub outer_norm: f64, pub total_norm: f64,
}

pub fn probabilities(vs: &[Vec<f64>]) -> Result<Vec<f64>, String> {
    if vs.is_empty() || vs[0].is_empty() || vs.iter().any(|v| v.len() != vs[0].len() || v.iter().any(|x| !x.is_finite())) {
        return Err("empty, inconsistent or nonfinite vectors".into());
    }
    let p: Vec<f64> = (0..vs[0].len()).map(|i| vs.iter().map(|v| v[i] * v[i]).sum::<f64>() / vs.len() as f64).collect();
    if p.iter().any(|x| !x.is_finite()) { return Err("overflowed squared coefficients".into()); }
    Ok(p)
}

pub fn summarize(b: &[State], p: &[f64], roles: [usize; 6]) -> Result<Distribution, String> {
    let mut sorted = roles;
    sorted.sort();
    if b.is_empty() || b.len() != p.len() || sorted != [0, 1, 2, 3, 4, 5]
        || b.iter().collect::<HashSet<_>>().len() != b.len() || p.iter().any(|x| !x.is_finite() || *x < 0.0) {
        return Err("invalid occupation distribution".into());
    }
    let max_n = b.iter().map(|n| n.iter().map(|x| *x as usize).sum::<usize>()).max().unwrap();
    let mut shells = vec![0.0; max_n + 1];
    let mut joint: BTreeMap<[usize; 4], f64> = BTreeMap::new();
    let mut mass = 0.0;
    let mut occupied = 0.0;
    let mut means = [0.0; 4];
    let mut fractions = [0.0; 4];
    for (n, &w) in b.iter().zip(p) {
        let g = [n[roles[0]] as usize + n[roles[1]] as usize, n[roles[2]] as usize,
                 n[roles[3]] as usize, n[roles[4]] as usize + n[roles[5]] as usize];
        let total: usize = g.iter().sum();
        mass += w;
        shells[total] += w;
        if w > 0.0 { *joint.entry(g).or_default() += w; }
        for j in 0..4 { means[j] += w * g[j] as f64; }
        if total > 0 {
            occupied += w;
            for j in 0..4 { fractions[j] += w * g[j] as f64 / total as f64; }
        }
    }
    if !mass.is_finite() || means.iter().any(|x| !x.is_finite()) { return Err("overflowed distribution".into()); }
    Ok(Distribution { mass, shells, joint: joint.into_iter().map(|(occupation, mass)| JointBin { occupation, mass }).collect(),
        mean_group_occupation: if mass > 0.0 { Some(means.map(|x| x / mass)) } else { None },
        occupation_fraction: if occupied > 0.0 { Some(fractions.map(|x| x / occupied)) } else { None } })
}

fn positions(a: &[State], b: &[State]) -> Result<Vec<usize>, String> {
    let index: HashMap<State, usize> = b.iter().enumerate().map(|(i, n)| (*n, i)).collect();
    if a.is_empty() || a.iter().collect::<HashSet<_>>().len() != a.len() || index.len() != b.len() {
        return Err("empty or duplicate occupation keys".into());
    }
    a.iter().map(|n| index.get(n).copied().ok_or_else(|| "missing occupation key".into())).collect()
}

pub fn embed(a: &[State], b: &[State], v: &[f64]) -> Result<Vec<f64>, String> {
    if v.len() != a.len() || v.iter().any(|x| !x.is_finite()) { return Err("invalid source coefficients".into()); }
    let mut out = vec![0.0; b.len()];
    for (i, j) in positions(a, b)?.into_iter().enumerate() { out[j] = v[i]; }
    Ok(out)
}

pub fn split_residual(a: &[State], b: &[State], v: &[f64], hv: &[f64], e: f64) -> Result<ResidualSplit, String> {
    if v.len() != b.len() || hv.len() != b.len() || !e.is_finite()
        || v.iter().chain(hv).any(|x| !x.is_finite()) { return Err("invalid operator action".into()); }
    let residual: Vec<f64> = hv.iter().zip(v).map(|(h, c)| h - e * c).collect();
    let total_sq = residual.iter().map(|x| x * x).sum::<f64>();
    let mut outer = residual.clone();
    let mut inner_sq = 0.0;
    for j in positions(a, b)? { inner_sq += outer[j] * outer[j]; outer[j] = 0.0; }
    let outer_sq = outer.iter().map(|x| x * x).sum::<f64>();
    if !total_sq.is_finite() { return Err("overflowed residual".into()); }
    Ok(ResidualSplit { residual, outer, inner_norm: inner_sq.sqrt(), outer_norm: outer_sq.sqrt(), total_norm: total_sq.sqrt() })
}

#[derive(Serialize)]
pub struct StateSupport {
    pub name: &'static str,
    pub level_index: usize,
    pub energy: f64,
    pub reference_weight: f64,
    pub probability: Distribution,
    pub omitted: Distribution,
    pub inner_norm: f64,
    pub outer_norm: f64,
    pub probe_norm: f64,
    pub nesting_error: f64,
    pub coeff_bits: Vec<u64>,
    pub probe_residual_bits: Vec<u64>,
}

#[derive(Serialize)]
pub struct SupportExport {
    pub schema: &'static str,
    pub scope: &'static str,
    pub s: f64, pub t: f64, pub g2: f64,
    pub mq: usize, pub probe_mq: usize,
    pub certified: bool, pub full_grid_released: bool,
    pub roles: [usize; 6], pub frequencies: [f64; 6],
    pub chart_origin: [f64; 6], pub chart_matrix: [[f64; 6]; 6],
    pub basis: Vec<State>, pub probe_basis: Vec<State>,
    pub identification: ritz::HelikeCompetitor,
    pub states: Vec<StateSupport>,
    pub helike_average_probability: Distribution,
    pub helike_average_omitted: Distribution,
    pub seconds: f64,
}

pub fn diagnose(s: f64, t: f64, g2: f64, mq: usize, set: &SolveSettings) -> Result<SupportExport, String> {
    let start = std::time::Instant::now();
    if !(g2 > 0.0 && g2.is_finite()) || !(2..=12).contains(&mq) { return Err("support diagnostic requires finite g2 > 0 and 2 <= Mq <= 12".into()); }
    let bg = Background::maurer_cartan(s, t, g2.sqrt())?;
    let op = ConeOperator::exact(bg.clone(), mq, Mutation::None)?;
    eprintln!("support: solving Mq={mq}, dimension={}", op.dim());
    let eigs = ritz::spectrum(&op, set);
    if eigs.values.len() != set.nev || eigs.residuals.len() != set.nev
        || eigs.residuals.iter().zip(&eigs.values).any(|(r,e)| !r.is_finite() || !e.is_finite() || *r > set.tol * e.abs().max(1.0)) {
        return Err("finite-space Ritz solve unresolved".into());
    }
    eprintln!("support: identifying states, elapsed={:.1}s", start.elapsed().as_secs_f64());
    let wh = ritz::helike_frequency(s, t);
    let identification = ritz::helike_and_competitor(&op, &eigs, wh)?;
    let roles = ritz::mode_roles(&bg.w, wh)?;
    let amps = ritz::reference_amplitudes(&op, &eigs.vectors, identification.overlap_orders[1])?;
    let k = eigs.values.len();
    let weight = |j: usize| (amps[roles[0]*k+j].powi(2) + amps[roles[1]*k+j].powi(2)) / identification.vacuum_weight;
    let mut order: Vec<usize> = (1..k).collect();
    order.sort_by(|a,b| weight(*b).total_cmp(&weight(*a)));
    let mut pair = [order[0], order[1]];
    pair.sort();
    let indices = [0, pair[0], pair[1], identification.competitor.level_index];
    if indices.iter().collect::<HashSet<_>>().len() != 4 || !pair.contains(&identification.helike.level_index) {
        return Err("diagnostic state selection is not distinct".into());
    }
    let weights = [identification.vacuum_weight, weight(pair[0]), weight(pair[1]), identification.competitor.dressed_weight];
    let vectors: Vec<Vec<f64>> = indices.iter().map(|j| eigs.vectors[*j].clone()).collect();
    for v in &vectors {
        if (v.iter().map(|x| x*x).sum::<f64>()-1.0).abs() > 1e-8 { return Err("unnormalized diagnostic state".into()); }
    }
    eprintln!("support: probing shells {} and {}; no larger eigensolve, elapsed={:.1}s", mq+1, mq+2, start.elapsed().as_secs_f64());
    let probe = ConeOperator::exact(bg, mq+2, Mutation::None)?;
    let padded: Vec<Vec<f64>> = vectors.iter().map(|v| embed(&op.basis, &probe.basis, v)).collect::<Result<_,_>>()?;
    let high_action = probe.apply(&padded, Part::Full);
    let low_action = op.apply(&vectors, Part::Full);
    let pos = positions(&op.basis, &probe.basis)?;
    let mut states = Vec::new();
    let mut omitted_vectors = Vec::new();
    for (j, name) in ["ground", "helike_a", "helike_b", "competitor"].iter().enumerate() {
        let e = eigs.values[indices[j]];
        let nesting_error = pos.iter().enumerate().map(|(i,q)| (low_action[j][i]-high_action[j][*q]).powi(2)).sum::<f64>().sqrt();
        if !nesting_error.is_finite() || nesting_error > 1e-9 * e.abs().max(1.0) { return Err("nested-block action mismatch".into()); }
        let residual = split_residual(&op.basis, &probe.basis, &padded[j], &high_action[j], e)?;
        let probability = summarize(&op.basis, &probabilities(&[vectors[j].clone()])?, roles)?;
        let omitted = summarize(&probe.basis, &probabilities(&[residual.outer.clone()])?, roles)?;
        states.push(StateSupport { name, level_index: indices[j], energy: e, reference_weight: weights[j], probability, omitted,
            inner_norm: residual.inner_norm, outer_norm: residual.outer_norm, probe_norm: residual.total_norm, nesting_error,
            coeff_bits: vectors[j].iter().map(|x| x.to_bits()).collect(),
            probe_residual_bits: residual.residual.iter().map(|x| x.to_bits()).collect() });
        omitted_vectors.push(residual.outer);
    }
    let helike_average_probability = summarize(&op.basis, &probabilities(&vectors[1..3])?, roles)?;
    let helike_average_omitted = summarize(&probe.basis, &probabilities(&omitted_vectors[1..3])?, roles)?;
    eprintln!("support: diagnostics complete, elapsed={:.1}s", start.elapsed().as_secs_f64());
    Ok(SupportExport { schema: "helike-lab/cone-support/v1",
        scope: "finite-space occupation and projected omitted-shell residual only; not a full residual, domain certificate or gap error bar",
        s, t, g2, mq, probe_mq: mq+2, certified: false, full_grid_released: false,
        roles, frequencies: op.bg.w, chart_origin: op.chart.u0, chart_matrix: op.chart.s,
        basis: op.basis, probe_basis: probe.basis, identification, states,
        helike_average_probability, helike_average_omitted, seconds: start.elapsed().as_secs_f64() })
}
