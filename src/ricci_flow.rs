//! Homogeneous SU(2) Ricci flow, not graph Ricci curvature or Wilson flow.
//! Convention and numerical limits: docs/RICCI_FLOW_SU2_SPEC.md.
use crate::parser::ExecResult;
use crate::types::{Record, Value};

type Metric = [f64; 3];

fn ricci(g: Metric) -> Metric {
    std::array::from_fn(|i| {
        let (a, b, c) = (g[i], g[(i + 1) % 3], g[(i + 2) % 3]);
        2. * (a * a - (b - c).powi(2)) / (b * c)
    })
}

fn positive(g: Metric, floor: f64) -> Result<(), String> {
    if g.iter().any(|x| !x.is_finite() || *x <= floor) {
        return Err("RICCI_FLOW: RK stage lost a finite positive metric above the numerical floor. Shorten TIME or increase STEPS; singularity crossing and surgery are not supported.".into());
    }
    Ok(())
}

pub(crate) fn rhs(g: Metric, normalized: bool, floor: f64) -> Result<Metric, String> {
    positive(g, floor)?;
    let r = ricci(g);
    let scalar = (0..3).map(|i| r[i] / g[i]).sum::<f64>();
    let velocity = std::array::from_fn(|i| {
        -2. * r[i] + if normalized { (2. / 3.) * scalar * g[i] } else { 0. }
    });
    if velocity.iter().any(|x: &f64| !x.is_finite()) {
        return Err("RICCI_FLOW: nonfinite curvature derivative; rescale the metric or shorten TIME".into());
    }
    Ok(velocity)
}

fn step(g: Metric, h: f64, normalized: bool, floor: f64) -> Result<Metric, String> {
    let add = |v: Metric, factor: f64| std::array::from_fn(|i| g[i] + factor * v[i]);
    let k1 = rhs(g, normalized, floor)?;
    let k2 = rhs(add(k1, h / 2.), normalized, floor)?;
    let k3 = rhs(add(k2, h / 2.), normalized, floor)?;
    let k4 = rhs(add(k3, h), normalized, floor)?;
    let next = std::array::from_fn(|i| g[i] + h / 6. * (k1[i] + 2. * k2[i] + 2. * k3[i] + k4[i]));
    positive(next, floor)?;
    Ok(next)
}

/// Fixed, bounded-work RK4 with a step-doubling diagnostic, not a rigorous enclosure.
pub fn execute_ricci_flow(
    metric: Metric, time: f64, steps: usize, normalized: bool, tolerance: f64,
) -> Result<ExecResult, String> {
    if metric.iter().any(|x| !x.is_finite() || !(1e-6..=1e6).contains(x)) {
        return Err("RICCI_FLOW: METRIC coefficients must be finite squared lengths in [1e-6,1e6]; rescale units".into());
    }
    let scale = metric.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min = metric.iter().copied().fold(f64::INFINITY, f64::min);
    if scale / min > 1e4 {
        return Err("RICCI_FLOW: METRIC max/min must be <=10000 for this numerical policy".into());
    }
    if !time.is_finite() || time <= 0. || time > 100. * min {
        return Err("RICCI_FLOW: TIME must be finite, positive and <=100*min(initial METRIC); split the requested interval".into());
    }
    if !(1..=4096).contains(&steps) {
        return Err("RICCI_FLOW: STEPS must be an integer in 1..4096".into());
    }
    if !tolerance.is_finite() || !(1e-12..=1e-3).contains(&tolerance) {
        return Err("RICCI_FLOW: TOLERANCE must be finite in [1e-12,1e-3]".into());
    }
    let initial = metric.map(|x| x / scale);
    let floor = min / scale * 1e-10;
    let h = (time / scale) / steps as f64;
    let (mut coarse, mut fine) = (initial, initial);
    let mut samples = vec![(initial, 0.)];
    let mut max_error: f64 = 0.;
    for _ in 0..steps {
        coarse = step(coarse, h, normalized, floor)?;
        fine = step(step(fine, h / 2., normalized, floor)?, h / 2., normalized, floor)?;
        let error = (0..3).map(|i| (coarse[i] - fine[i]).abs() / 15.).fold(0., f64::max);
        max_error = max_error.max(error);
        if error > tolerance {
            return Err(format!("RICCI_FLOW: step-doubling error estimate {error} exceeds TOLERANCE {tolerance}; increase STEPS or shorten TIME. No partial trajectory returned."));
        }
        samples.push((fine, error));
    }
    let mut rows = Vec::with_capacity(samples.len());
    for (i, (g, error)) in samples.into_iter().enumerate() {
        let g = g.map(|x| x * scale);
        let r = ricci(g);
        let eig: Metric = std::array::from_fn(|j| r[j] / g[j]);
        let scalar = eig.iter().sum::<f64>();
        let lo = g.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = g.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let mut row = Record::new();
        for (key, value) in [
            ("t", time * (i as f64 / steps as f64)),
            ("a", g[0]), ("b", g[1]), ("c", g[2]),
            ("ricci_a", r[0]), ("ricci_b", r[1]), ("ricci_c", r[2]),
            ("lambda_a", eig[0]), ("lambda_b", eig[1]), ("lambda_c", eig[2]),
            ("scalar_curvature", scalar),
            ("sectional_ab", (eig[0] + eig[1] - eig[2]) / 2.),
            ("sectional_bc", (eig[1] + eig[2] - eig[0]) / 2.),
            ("sectional_ca", (eig[2] + eig[0] - eig[1]) / 2.),
            ("volume_cover", 2. * std::f64::consts::PI.powi(2) * (g[0] * g[1] * g[2]).sqrt()),
            ("anisotropy", hi / lo), ("estimated_relative_error", error),
            ("max_estimated_relative_error", max_error), ("tolerance", tolerance),
        ] {
            if !value.is_finite() { return Err(format!("RICCI_FLOW: nonfinite output {key}; shorten TIME")); }
            row.insert(key.into(), Value::Float(value));
        }
        for (key, value) in [
            ("manifold_model", "SU2_left_invariant_diagonal_cover"),
            ("equation", if normalized { "dg/dt=-2Ric+(2/3)Rg" } else { "dg/dt=-2Ric" }),
            ("method", "rk4_step_doubling"), ("numerical_policy", "su2_ricci_v1"),
            ("bracket_convention", "[e1,e2]=2e3_cyclic"),
        ] { row.insert(key.into(), Value::Text(value.into())); }
        for (key, value) in [("normalized", normalized), ("surgery_performed", false),
            ("topology_inferred", false), ("error_bound_certified", false)] {
            row.insert(key.into(), Value::Bool(value));
        }
        row.insert("coarse_steps".into(), Value::Integer(steps as i64));
        row.insert("rhs_evaluations".into(), Value::Integer(12 * steps as i64));
        rows.push(row);
    }
    Ok(ExecResult::Rows(rows))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_nonfinite_arguments_refuse() {
        for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(execute_ricci_flow([x,1.,1.],0.1,16,false,1e-8).is_err());
            assert!(execute_ricci_flow([1.;3],x,16,false,1e-8).is_err());
            assert!(execute_ricci_flow([1.;3],0.1,16,false,x).is_err());
        }
    }
    #[test]
    fn stage_positivity_guard_is_load_bearing() {
        assert!(rhs([1.,-1.,1.],false,1e-10).unwrap_err().contains("positive"));
    }
}
