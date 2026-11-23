//! Checked unit-sphere connection adapter. See docs/SPHERE_TRANSPORT_SPEC.md.
use crate::types::{Record, Value};

const UNIT_TOL: f64 = 1e-12;
const RESIDUAL_TOL: f64 = 1e-10;
const ANTIPODAL_GAP: f64 = 1e-8;
type V = [f64; 3];
type M = [[f64; 3]; 3];

#[derive(Debug, Clone)]
pub struct SphereTransport {
    pub numerical_policy: u32,
    pub from_unit: [f64; 3],
    pub to_unit: [f64; 3],
    pub endpoint_input_norms: [f64; 2],
    pub angle_radians: f64,
    pub matrix: [[f64; 3]; 3],
    pub transported: [f64; 3],
    pub endpoint_residual: f64,
    pub orthogonality_residual: f64,
    pub tangency_relative_residual: f64,
    pub norm_relative_residual: f64,
}

fn dot(a: V, b: V) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn norm(v: V) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}

fn vector(values: &[f64], label: &str) -> Result<V, String> {
    let result: V = values.try_into().map_err(|_| {
        format!("{label} must have exactly 3 components; use one orthonormal Cartesian frame")
    })?;
    for (i, v) in result.iter().enumerate() {
        if !v.is_finite() {
            return Err(format!("{label} component {i} must be finite; supply a measured numeric value"));
        }
    }
    Ok(result)
}

fn unit(v: V, label: &str) -> Result<(V, f64), String> {
    let n = norm(v);
    if !n.is_finite() || (n - 1.).abs() > UNIT_TOL {
        return Err(format!("{label} norm {n} must be within 1e-12 of 1; declare unit-sphere positions before calling"));
    }
    Ok((v.map(|x| x / n), n))
}

/// Shortest-arc Levi-Civita transport on the unit sphere in an orthonormal frame.
/// Refuses ambiguous antipodal paths. Only endpoint roundoff normalization is
/// allowed and disclosed. Residuals are measured checks, not certified intervals.
pub fn sphere_parallel_transport(
    from: &[f64], to: &[f64], tangent: &[f64],
) -> Result<SphereTransport, String> {
    let (p, pn) = unit(vector(from, "from")?, "from")?;
    let (q, qn) = unit(vector(to, "to")?, "to")?;
    let v = vector(tangent, "tangent")?;
    let vn = norm(v);
    if !vn.is_finite() || vn > 1e100 {
        return Err("tangent norm must be finite and at most 1e100; rescale explicitly".into());
    }
    let direction = if vn == 0. { [0.; 3] } else { v.map(|x| x / vn) };
    if dot(p, direction).abs() > UNIT_TOL {
        return Err("tangent is not tangent at from within relative 1e-12; supply a measured tangent, not a normal component".into());
    }
    let cosine = dot(p, q).clamp(-1., 1.);
    if 1. + cosine <= ANTIPODAL_GAP {
        return Err("antipodal or numerically unresolved arc: 1 + from dot to must exceed 1e-8; supply an intermediate waypoint away from the antipode".into());
    }
    let cross = [p[1]*q[2]-p[2]*q[1], p[2]*q[0]-p[0]*q[2], p[0]*q[1]-p[1]*q[0]];
    let angle = norm(cross).atan2(cosine);
    let flat = super::transport_rotation_matrix(&p, &q, angle);
    let matrix = std::array::from_fn(|i| std::array::from_fn(|j| flat[3*i+j]));
    finish(p, q, v, [pn, qn], angle, matrix)
}

fn finish(p: V, q: V, v: V, input_norms: [f64; 2], angle: f64, matrix: M) -> Result<SphereTransport, String> {
    let transported = matrix.map(|row| dot(row, v));
    let endpoint_residual = norm(std::array::from_fn(|i| dot(matrix[i], p) - q[i]));
    let mut orthogonality_residual = 0f64;
    for i in 0..3 {
        for j in 0..3 {
            let entry: f64 = (0..3).map(|k| matrix[k][i] * matrix[k][j]).sum();
            orthogonality_residual = orthogonality_residual.hypot(entry - if i == j { 1. } else { 0. });
        }
    }
    let vn = norm(v);
    let out_direction = if vn == 0. { [0.; 3] } else { transported.map(|x| x / vn) };
    let tangency_relative_residual = dot(q, out_direction).abs();
    let norm_relative_residual = if vn == 0. { norm(transported) } else { (norm(out_direction)-1.).abs() };
    let diagnostics = [endpoint_residual, orthogonality_residual, tangency_relative_residual, norm_relative_residual];
    if transported.iter().any(|x| !x.is_finite())
        || diagnostics.iter().any(|x| !x.is_finite() || *x > RESIDUAL_TOL) {
        return Err(format!("numerically unresolved sphere transport: residuals {diagnostics:?} must be finite and <= 1e-10; use better-resolved intermediate waypoints"));
    }
    Ok(SphereTransport {
        numerical_policy: 1, from_unit: p, to_unit: q,
        endpoint_input_norms: input_norms, angle_radians: angle, matrix, transported,
        endpoint_residual, orthogonality_residual, tangency_relative_residual, norm_relative_residual,
    })
}

/// Read exactly three distinct numeric measurements, never missing-value zeros.
pub fn sphere_parallel_transport_records(
    from: &Record, to: &Record, fields: &[&str], tangent: &[f64],
) -> Result<SphereTransport, String> {
    if fields.len() != 3 || fields.iter().enumerate().any(|(i, f)| fields[..i].contains(f)) {
        return Err("sphere transport requires three distinct numeric fields in declared x,y,z order".into());
    }
    let read = |record: &Record, label: &str| -> Result<V, String> {
        let mut v = [0.; 3];
        for (i, field) in fields.iter().enumerate() {
            v[i] = match record.get(*field) {
                Some(Value::Float(x)) => *x,
                Some(Value::Integer(x)) => *x as f64,
                _ => return Err(format!("{label} field '{field}' must contain a numeric measurement; missing, null and nonnumeric values cannot be replaced by zero")),
            };
            if !v[i].is_finite() {
                return Err(format!("{label} field '{field}' must contain a finite numeric measurement"));
            }
        }
        Ok(v)
    };
    sphere_parallel_transport(&read(from, "from")?, &read(to, "to")?, tangent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sphere_output_guard_rejects_bad_kernel_results() {
        let p = [1., 0., 0.];
        let q = [0., 1., 0.];
        for matrix in [
            [[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],
            [[0.,-2.,0.],[1.,0.,0.],[0.,0.,1.]],
            [[0.,-1.,0.],[1.,0.,0.],[0.,0.,f64::NAN]],
        ] {
            assert!(finish(p, q, q, [1.,1.], std::f64::consts::FRAC_PI_2, matrix)
                .unwrap_err().contains("numerically unresolved"));
        }
    }
}
