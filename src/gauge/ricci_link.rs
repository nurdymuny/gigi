//! RF2B exact edge adapter. See docs/RICCI_LATTICE_CORRESPONDENCE_SPEC.md.
use super::group_element::GroupElement;

/// Inverse parallel transport along the short round great-circle edge.
/// Base-space products are Hamilton/plus; stored matrix links are minus-cross.
pub fn levi_civita_link(
    metric: [f64; 3],
    source: [f64; 4],
    target: [f64; 4],
) -> Result<GroupElement, String> {
    if metric
        .iter()
        .any(|x| !x.is_finite() || !(1e-6..=1e6).contains(x))
    {
        return Err("RICCI link: metric requires finite squared lengths in [1e-6,1e6]".into());
    }
    let low = metric.iter().copied().fold(f64::INFINITY, f64::min);
    let high = metric.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if high / low > 1e4 {
        return Err("RICCI link: metric anisotropy exceeds 10000".into());
    }
    for (name, point) in [("source", source), ("target", target)] {
        if point.iter().any(|x| !x.is_finite())
            || (point.iter().map(|x| x * x).sum::<f64>() - 1.).abs() > 1e-12
        {
            return Err(format!(
                "RICCI link: {name} must be a finite unit quaternion; no implicit normalization"
            ));
        }
    }
    let real = source.iter().zip(target).map(|(a, b)| a * b).sum::<f64>();
    let imag: [f64; 3] = std::array::from_fn(|i| {
        let (j, k) = ((i + 1) % 3, (i + 2) % 3);
        source[0] * target[i + 1] - target[0] * source[i + 1] - source[j + 1] * target[k + 1]
            + source[k + 1] * target[j + 1]
    });
    let length = imag.iter().map(|x| x * x).sum::<f64>().sqrt();
    let angle = length.atan2(real);
    if angle >= std::f64::consts::PI - 1e-8 {
        return Err("RICCI link: antipodal or nearly antipodal edge has an ambiguous short path; refine mesh".into());
    }
    if length == 0. {
        return Ok(GroupElement::su2_identity());
    }
    let x = crate::ricci_wilson::connection(metric);
    let a: [f64; 3] = std::array::from_fn(|i| x[i] * imag[i] * angle / length);
    let half_angle = 0.5 * a.iter().map(|v| v * v).sum::<f64>().sqrt();
    let factor = if half_angle < 1e-8 {
        -0.5 * (1. - half_angle * half_angle / 6.)
    } else {
        -0.5 * half_angle.sin() / half_angle
    };
    Ok(GroupElement::SU2 {
        q0: half_angle.cos(),
        q1: factor * a[0],
        q2: factor * a[1],
        q3: factor * a[2],
    })
}
