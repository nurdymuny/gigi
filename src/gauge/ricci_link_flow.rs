//! LINK_FLOW (YM4, docs/YANG_MILLS_ENGINE_SPEC.md): the SU(2) lattice in time,
//! B(U, g(t)) ξ′ = −a(U; g(t)), integrated with the Cayley midpoint step of S3b
//! (scripts/ricci_su2_pg_flow_cayley.py), engine form.
//!
//!   U_{k+1} = U_k · cay(Ω),   Ω = h ξ(U_k · cay(Ω/2), t_k + h/2)
//!
//! solved by quasi-Newton with the frozen preconditioner A = B₀ + (h/2) K₀
//! (dense LU; n ≤ 3 by decision, larger meshes refuse), stopping when the relative
//! increment reaches 10⁻⁸ (S3b Amendment 1). The velocity solves the native mass
//! B(U, g) by Jacobi-preconditioned conjugate gradients to 10⁻¹² (as the research
//! script). The metric along the flow is an input, sampled at the times the step
//! needs; a missing time refuses, and nothing is ever substituted. Exact links of
//! the reference metric are comparison inputs only: they never enter a step.
use super::curvature_recovery::Cell;
use super::group_element::GroupElement;
use super::ricci_link::levi_civita_link;
use super::ricci_pg_force::{ForceContext, ForceMethod};
use nalgebra::{DMatrix, DVector, Vector3};

/// Largest number of unknowns the dense-LU preconditioner accepts.
pub const DENSE_LU_MAX: usize = 12_000;
pub const NEWTON_TOL: f64 = 1e-8;
pub const NEWTON_MAX: usize = 40;

/// Compressed sparse rows, duplicates summed.
pub struct Csr {
    pub n: usize,
    indptr: Vec<usize>,
    indices: Vec<usize>,
    data: Vec<f64>,
}

impl Csr {
    pub fn from_triplets(n: usize, r: &[usize], c: &[usize], v: &[f64]) -> Csr {
        let mut order: Vec<usize> = (0..v.len()).collect();
        order.sort_unstable_by_key(|&i| (r[i], c[i]));
        let (mut indptr, mut indices, mut data) = (vec![0usize; n + 1], Vec::new(), Vec::new());
        let mut last: Option<(usize, usize)> = None;
        for &i in &order {
            if last == Some((r[i], c[i])) {
                *data.last_mut().unwrap() += v[i];
            } else {
                indices.push(c[i]);
                data.push(v[i]);
                indptr[r[i] + 1] += 1;
                last = Some((r[i], c[i]));
            }
        }
        for i in 0..n {
            indptr[i + 1] += indptr[i];
        }
        Csr { n, indptr, indices, data }
    }

    pub fn matvec(&self, x: &[f64]) -> Vec<f64> {
        (0..self.n)
            .map(|i| (self.indptr[i]..self.indptr[i + 1]).map(|p| self.data[p] * x[self.indices[p]]).sum())
            .collect()
    }

    fn diagonal(&self) -> Vec<f64> {
        (0..self.n)
            .map(|i| (self.indptr[i]..self.indptr[i + 1]).find(|&p| self.indices[p] == i).map_or(0.0, |p| self.data[p]))
            .collect()
    }

    fn add_dense_into(&self, scale: f64, m: &mut DMatrix<f64>) {
        for i in 0..self.n {
            for p in self.indptr[i]..self.indptr[i + 1] {
                m[(i, self.indices[p])] += scale * self.data[p];
            }
        }
    }
}

fn norm(x: &[f64]) -> f64 {
    x.iter().map(|v| v * v).sum::<f64>().sqrt()
}

/// Jacobi-preconditioned conjugate gradients; stops at ‖r‖ ≤ max(rtol ‖b‖, atol)
/// (scipy's rule), then requires a true relative residual ≤ 10⁻¹⁰.
fn cg(b_mat: &Csr, rhs: &[f64]) -> Result<Vec<f64>, String> {
    let (rtol, atol, maxiter) = (1e-12, 1e-14, 5000);
    let d = b_mat.diagonal();
    let n = rhs.len();
    let mut x = vec![0.0; n];
    let mut r = rhs.to_vec();
    let mut z: Vec<f64> = r.iter().zip(&d).map(|(a, b)| a / b).collect();
    let mut p = z.clone();
    let mut rz: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
    let stop = (rtol * norm(rhs)).max(atol);
    for _ in 0..maxiter {
        if norm(&r) <= stop {
            break;
        }
        let ap = b_mat.matvec(&p);
        let alpha = rz / p.iter().zip(&ap).map(|(a, b)| a * b).sum::<f64>();
        for i in 0..n {
            x[i] += alpha * p[i];
            r[i] -= alpha * ap[i];
        }
        z = r.iter().zip(&d).map(|(a, b)| a / b).collect();
        let rz_new: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
        let beta = rz_new / rz;
        rz = rz_new;
        for i in 0..n {
            p[i] = z[i] + beta * p[i];
        }
    }
    let bx = b_mat.matvec(&x);
    let res = norm(&bx.iter().zip(rhs).map(|(a, b)| a - b).collect::<Vec<_>>()) / norm(rhs).max(1e-300);
    if !(res <= 1e-10) {
        return Err(format!("LINK_FLOW: mass solve failed (relative residual {res:.2e})"));
    }
    Ok(x)
}

/// Link product, stored-link convention.
fn lmul(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    let (av, bv) = (Vector3::new(a[1], a[2], a[3]), Vector3::new(b[1], b[2], b[3]));
    let v = bv * a[0] + av * b[0] - av.cross(&bv);
    [a[0] * b[0] - av.dot(&bv), v[0], v[1], v[2]]
}

fn inv(q: [f64; 4]) -> [f64; 4] {
    [q[0], -q[1], -q[2], -q[3]]
}

fn qlog(q: [f64; 4]) -> [f64; 3] {
    let s = (q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if s < 1e-300 {
        return [0.0; 3];
    }
    let f = s.atan2(q[0]) / s;
    [q[1] * f, q[2] * f, q[3] * f]
}

/// Cayley map of a pure quaternion.
fn cay(v: [f64; 3]) -> [f64; 4] {
    let s = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]) / 4.0;
    [(1.0 - s) / (1.0 + s), v[0] / (1.0 + s), v[1] / (1.0 + s), v[2] / (1.0 + s)]
}

fn step_links(u: &[[f64; 4]], omega: &[f64], scale: f64) -> Vec<[f64; 4]> {
    u.iter()
        .enumerate()
        .map(|(e, q)| lmul(*q, cay([scale * omega[3 * e], scale * omega[3 * e + 1], scale * omega[3 * e + 2]])))
        .collect()
}

/// The metric along the flow, as samples (t, g). Lookup refuses a missing time.
pub struct MetricSamples(pub Vec<(f64, [f64; 3])>);

impl MetricSamples {
    pub fn at(&self, t: f64) -> Result<[f64; 3], String> {
        self.0
            .iter()
            .find(|(s, _)| (s - t).abs() <= 1e-12 * t.abs().max(1.0))
            .map(|(_, g)| *g)
            .ok_or_else(|| format!("LINK_FLOW: no metric sample at t = {t}; none is ever substituted"))
    }
}

#[derive(Clone, Debug)]
pub struct FlowRow {
    pub t: f64,
    pub e: f64,
    pub d: f64,
    pub r: f64,
    pub newton_iterations: usize,
}

pub struct FlowResult {
    pub status: String,
    pub rows: Vec<FlowRow>,
    pub newton_iterations: Vec<usize>,
    pub worst_final_increment: f64,
    pub force_evaluations: usize,
    pub final_links: Vec<[f64; 4]>,
    pub variational: bool,
}

fn exact_links(edge_points: &[[[f64; 4]; 2]], g: [f64; 3]) -> Result<Vec<[f64; 4]>, String> {
    edge_points
        .iter()
        .map(|p| {
            levi_civita_link(g, p[0], p[1]).map(|q| match q {
                GroupElement::SU2 { q0, q1, q2, q3 } => [q0, q1, q2, q3],
                _ => unreachable!(),
            })
        })
        .collect()
}

/// E = ‖log(L⁻¹ U)‖_{B(L, g)} / √Vol, as the research script's error().
fn mass_error(ctx: &ForceContext, cells: &[Cell], u: &[[f64; 4]], l: &[[f64; 4]]) -> Result<f64, String> {
    let (r, c, v) = ctx.tangent_mass(cells, l)?;
    let b = Csr::from_triplets(3 * l.len(), &r, &c, &v);
    let d: Vec<f64> = u.iter().zip(l).flat_map(|(ue, le)| qlog(lmul(inv(*le), *ue))).collect();
    let bd = b.matvec(&d);
    let q: f64 = d.iter().zip(&bd).map(|(a, b)| a * b).sum();
    Ok((q.max(0.0) / ctx.volume()).sqrt())
}

/// Where the step evaluates the velocity. `Midpoint` is the Cayley step;
/// `Endpoint` (Ω = h ξ(U · cay(Ω), t_k + h)) exists only as the YM6 mechanism
/// removal and is never used by the verb.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StepEvaluation {
    Midpoint,
    Endpoint,
}

/// Integrate the flow. `k0` is the preconditioner's Jacobian as triplets
/// (rows, cols, vals); `metric_step` the metric the step uses (the frozen-metric
/// control passes g(0) at every time); `metric_ref` the reference metric for the
/// exact links; `edge_points` the edge endpoints for those exact links.
#[allow(clippy::too_many_arguments)]
pub fn link_flow(
    cells: &[Cell],
    weights: &[f64],
    links0: &[[f64; 4]],
    edge_points: &[[[f64; 4]; 2]],
    order: usize,
    method: ForceMethod,
    n_steps: usize,
    t_end: f64,
    metric_step: &MetricSamples,
    metric_ref: &MetricSamples,
    k0: (&[usize], &[usize], &[f64]),
) -> Result<FlowResult, String> {
    link_flow_with(cells, weights, links0, edge_points, order, method, n_steps, t_end, metric_step, metric_ref, k0,
                   StepEvaluation::Midpoint)
}

/// [`link_flow`] with the YM6 removal switch.
#[allow(clippy::too_many_arguments)]
pub fn link_flow_with(
    cells: &[Cell],
    weights: &[f64],
    links0: &[[f64; 4]],
    edge_points: &[[[f64; 4]; 2]],
    order: usize,
    method: ForceMethod,
    n_steps: usize,
    t_end: f64,
    metric_step: &MetricSamples,
    metric_ref: &MetricSamples,
    k0: (&[usize], &[usize], &[f64]),
    evaluation: StepEvaluation,
) -> Result<FlowResult, String> {
    let n = 3 * links0.len();
    if n > DENSE_LU_MAX {
        return Err(format!(
            "LINK_FLOW: {n} unknowns exceed the dense-LU preconditioner's limit {DENSE_LU_MAX}; a sparse solver is not built yet"
        ));
    }
    if n_steps == 0 || !(t_end > 0.0) {
        return Err("LINK_FLOW: positive step count and end time required".into());
    }
    if edge_points.len() != links0.len() {
        return Err("LINK_FLOW: one edge-point pair per link required (for the exact reference)".into());
    }
    let h = t_end / n_steps as f64;
    let g0 = metric_step.at(0.0)?;
    let mut ctx = ForceContext::new(cells, weights, links0.len(), g0, order, method)?;
    let (r, c, v) = ctx.tangent_mass(cells, links0)?;
    let b0 = Csr::from_triplets(n, &r, &c, &v);
    let mut a = DMatrix::<f64>::zeros(n, n);
    b0.add_dense_into(1.0, &mut a);
    Csr::from_triplets(n, k0.0, k0.1, k0.2).add_dense_into(0.5 * h, &mut a);
    let lu = a.lu();

    let velocity = |ctx: &mut ForceContext, u: &[[f64; 4]], g: [f64; 3]| -> Result<Vec<f64>, String> {
        ctx.set_metric(cells, g, order)?;
        let grad = ctx.force(cells, u)?;
        let (r, c, v) = ctx.tangent_mass(cells, u)?;
        let bm = Csr::from_triplets(n, &r, &c, &v);
        let rhs: Vec<f64> = grad.iter().flat_map(|x| [-x[0], -x[1], -x[2]]).collect();
        cg(&bm, &rhs)
    };

    let samples: Vec<usize> = {
        let mut s = vec![n_steps / 4, n_steps / 2, 3 * n_steps / 4, n_steps];
        s.sort_unstable();
        s.dedup();
        s
    };
    let mut u = links0.to_vec();
    let mut om = vec![0.0; n];
    let (mut rows, mut iters) = (Vec::new(), Vec::new());
    let (mut worst, mut evals) = (0.0_f64, 0usize);
    let mut status = "ok".to_string();
    for k in 0..n_steps {
        let (tm, frac) = match evaluation {
            StepEvaluation::Midpoint => ((k as f64 + 0.5) * h, 0.5),
            StepEvaluation::Endpoint => ((k as f64 + 1.0) * h, 1.0),
        };
        let g = metric_step.at(tm)?;
        let (mut ok, mut inc, mut it) = (false, f64::NAN, 0);
        for i in 1..=NEWTON_MAX {
            it = i;
            let xi = velocity(&mut ctx, &step_links(&u, &om, frac), g)?;
            evals += 1;
            let f: Vec<f64> = om.iter().zip(&xi).map(|(o, x)| o - h * x).collect();
            let rhs = DVector::from_vec(b0.matvec(&f).into_iter().map(|x| -x).collect());
            let d = lu.solve(&rhs).ok_or("LINK_FLOW: singular preconditioner")?;
            for (o, di) in om.iter_mut().zip(d.iter()) {
                *o += di;
            }
            inc = d.norm() / norm(&om).max(1e-300);
            if !inc.is_finite() {
                break;
            }
            if inc <= NEWTON_TOL {
                ok = true;
                break;
            }
        }
        iters.push(it);
        worst = worst.max(inc);
        if !ok {
            status = format!("failed at step {}: Newton increment {inc:.2e} after {it} iterations", k + 1);
            break;
        }
        u = step_links(&u, &om, 1.0)
            .into_iter()
            .map(|q| {
                let s = (q.iter().map(|x| x * x).sum::<f64>()).sqrt();
                q.map(|x| x / s)
            })
            .collect();
        if samples.contains(&(k + 1)) {
            let t = (k + 1) as f64 * h;
            let gt = metric_ref.at(t)?;
            let l = exact_links(edge_points, gt)?;
            ctx.set_metric(cells, gt, order)?;
            let e = mass_error(&ctx, cells, &u, &l)?;
            let d = mass_error(&ctx, cells, links0, &l)?;
            rows.push(FlowRow { t, e, d, r: e / d, newton_iterations: it });
        }
    }
    Ok(FlowResult {
        status,
        rows,
        newton_iterations: iters,
        worst_final_increment: worst,
        force_evaluations: evals,
        final_links: u,
        variational: method.variational(),
    })
}
