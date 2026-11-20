//! RF2B covariant cell action. Not the legacy Hamiltonian flow.
use super::group_element::GroupElement;
use nalgebra::{SMatrix, SVector};
use std::ops::{Add, Mul, Sub};

pub struct CellAction {
    pub energy: f64,
    pub gradient: SVector<f64, 18>,
}

/// Right link tangents live at their target vertices. Transport them to root 0
/// before contracting the full Whitney matrix; physical angular velocity is -2 eta.
pub fn cell_tangent_mass(
    links: [[f64; 4]; 6],
    mass: SMatrix<f64, 6, 6>,
) -> Result<SMatrix<f64, 18, 18>, String> {
    if links.iter().any(|q| {
        q.iter().any(|x| !x.is_finite())
            || (q.iter().map(|x| x * x).sum::<f64>() - 1.).abs() > 1e-12
    }) {
        return Err("simplicial tangent mass: finite unit links required".into());
    }
    if mass.iter().any(|x| !x.is_finite())
        || (mass - mass.transpose()).norm() > 1e-13 * mass.norm().max(1.)
        || mass.cholesky().is_none()
    {
        return Err(
            "simplicial tangent mass: finite symmetric positive definite edge mass required".into(),
        );
    }
    let roots = [links[0], links[1], links[2], links[1], links[2], links[2]];
    let rotations: [SMatrix<f64, 3, 3>; 6] = roots.map(adjoint);
    let mut result = SMatrix::<f64, 18, 18>::zeros();
    for e in 0..6 {
        for f in 0..6 {
            let block = 4. * mass[(e, f)] * rotations[e].transpose() * rotations[f];
            result
                .fixed_view_mut::<3, 3>(3 * e, 3 * f)
                .copy_from(&block);
        }
    }
    if result.iter().any(|x| !x.is_finite()) {
        return Err("simplicial tangent mass: nonfinite result".into());
    }
    Ok(result)
}

/// Adjoint in GIGI's native minus-cross quaternion convention.
pub fn adjoint(q: [f64; 4]) -> SMatrix<f64, 3, 3> {
    SMatrix::from_fn(|row, column| {
        let mut axis = [0.; 4];
        axis[column + 1] = 1.;
        components(group(q).compose(&group(axis)).compose(&group(q).inverse()))[row + 1]
    })
}

#[derive(Clone, Copy)]
struct Jet {
    v: f64,
    d: [f64; 18],
}
impl Jet {
    fn constant(v: f64) -> Self {
        Self { v, d: [0.; 18] }
    }
}
impl Add for Jet {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self {
            v: self.v + b.v,
            d: std::array::from_fn(|i| self.d[i] + b.d[i]),
        }
    }
}
impl Sub for Jet {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self {
            v: self.v - b.v,
            d: std::array::from_fn(|i| self.d[i] - b.d[i]),
        }
    }
}
impl Mul for Jet {
    type Output = Self;
    fn mul(self, b: Self) -> Self {
        Self {
            v: self.v * b.v,
            d: std::array::from_fn(|i| self.d[i] * b.v + self.v * b.d[i]),
        }
    }
}
type Q = [Jet; 4];

fn product(a: Q, b: Q) -> Q {
    [
        a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
        a[0] * b[1] + a[1] * b[0] - a[2] * b[3] + a[3] * b[2],
        a[0] * b[2] + a[2] * b[0] - a[3] * b[1] + a[1] * b[3],
        a[0] * b[3] + a[3] * b[0] - a[1] * b[2] + a[2] * b[1],
    ]
}
fn inverse(a: Q) -> Q {
    [
        a[0],
        Jet::constant(0.) - a[1],
        Jet::constant(0.) - a[2],
        Jet::constant(0.) - a[3],
    ]
}
fn group(a: [f64; 4]) -> GroupElement {
    GroupElement::SU2 {
        q0: a[0],
        q1: a[1],
        q2: a[2],
        q3: a[3],
    }
}
fn components(q: GroupElement) -> [f64; 4] {
    match q {
        GroupElement::SU2 { q0, q1, q2, q3 } => [q0, q1, q2, q3],
        _ => unreachable!(),
    }
}

/// The four face defects of a cell (holonomy − 1, face 123 carried to root 0 by
/// u₀, exactly as in [`cell_action`]) and their derivatives with respect to the
/// 18 right-trivialized link variations. Used by the two-ring Petrov–Galerkin
/// force (`ricci_pg_force`), which pairs the defect derivatives with a recovered
/// field instead of the native face mass. `cell_action` is unchanged.
pub struct CellDefects {
    /// `value[face][component]`.
    pub value: [[f64; 4]; 4],
    /// `derivative[face][component][dof]`, dof = 3 · edge + axis.
    pub derivative: [[[f64; 18]; 4]; 4],
}

pub fn cell_defects(links: [[f64; 4]; 6]) -> Result<CellDefects, String> {
    if links.iter().any(|q| {
        q.iter().any(|x| !x.is_finite())
            || (q.iter().map(|x| x * x).sum::<f64>() - 1.).abs() > 1e-12
    }) {
        return Err("simplicial defects: finite unit links required".into());
    }
    let u: [Q; 6] = std::array::from_fn(|edge| {
        let derivatives: [[f64; 4]; 3] = std::array::from_fn(|axis| {
            let mut eta = [0.; 4];
            eta[axis + 1] = 1.;
            components(group(links[edge]).compose(&group(eta)))
        });
        std::array::from_fn(|component| {
            let mut q = Jet::constant(links[edge][component]);
            for axis in 0..3 {
                q.d[3 * edge + axis] = derivatives[axis][component];
            }
            q
        })
    });
    let mut defects: [Q; 4] = std::array::from_fn(|face| {
        let (a, b, c) = [(0, 3, 1), (0, 4, 2), (1, 5, 2), (3, 5, 4)][face];
        let mut h = product(product(u[a], u[b]), inverse(u[c]));
        h[0] = h[0] - Jet::constant(1.);
        h
    });
    defects[3] = product(product(u[0], defects[3]), inverse(u[0]));
    Ok(CellDefects {
        value: std::array::from_fn(|f| std::array::from_fn(|c| defects[f][c].v)),
        derivative: std::array::from_fn(|f| std::array::from_fn(|c| defects[f][c].d)),
    })
}

/// Full cell action and all 18 right-trivialized derivatives, including the
/// root-to-face transport. Edges: 01,02,03,12,13,23. Faces: 012,013,023,123.
pub fn cell_action(links: [[f64; 4]; 6], mass: SMatrix<f64, 4, 4>) -> Result<CellAction, String> {
    if links.iter().any(|q| {
        q.iter().any(|x| !x.is_finite())
            || (q.iter().map(|x| x * x).sum::<f64>() - 1.).abs() > 1e-12
    }) {
        return Err("simplicial action: finite unit links required".into());
    }
    if mass.iter().any(|x| !x.is_finite())
        || (mass - mass.transpose()).norm() > 1e-13 * mass.norm().max(1.)
        || mass.cholesky().is_none()
    {
        return Err(
            "simplicial action: finite symmetric positive definite face mass required".into(),
        );
    }
    let u: [Q; 6] = std::array::from_fn(|edge| {
        let derivatives: [[f64; 4]; 3] = std::array::from_fn(|axis| {
            let mut eta = [0.; 4];
            eta[axis + 1] = 1.;
            components(group(links[edge]).compose(&group(eta)))
        });
        std::array::from_fn(|component| {
            let mut q = Jet::constant(links[edge][component]);
            for axis in 0..3 {
                q.d[3 * edge + axis] = derivatives[axis][component];
            }
            q
        })
    });
    let mut defects: [Q; 4] = std::array::from_fn(|face| {
        let (a, b, c) = [(0, 3, 1), (0, 4, 2), (1, 5, 2), (3, 5, 4)][face];
        let mut h = product(product(u[a], u[b]), inverse(u[c]));
        h[0] = h[0] - Jet::constant(1.);
        h
    });
    // Face 123 is based at vertex 1; compare it to the others at cell root 0.
    defects[3] = product(product(u[0], defects[3]), inverse(u[0]));
    let mut energy = Jet::constant(0.);
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                energy = energy + Jet::constant(2. * mass[(i, j)]) * defects[i][k] * defects[j][k];
            }
        }
    }
    if !energy.v.is_finite() || energy.d.iter().any(|x| !x.is_finite()) {
        return Err("simplicial action: nonfinite energy or derivative; rescale the mass".into());
    }
    Ok(CellAction {
        energy: energy.v,
        gradient: SVector::from_row_slice(&energy.d),
    })
}
