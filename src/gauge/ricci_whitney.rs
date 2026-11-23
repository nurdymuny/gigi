//! RF2B metric mass matrices on a radial spherical tetrahedron.
use nalgebra::{DMatrix, Matrix3, Matrix4, SMatrix, Vector3, Vector4};

const EDGES: [(usize, usize); 6] = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
const FACES: [(usize, usize, usize); 4] = [(0, 1, 2), (0, 1, 3), (0, 2, 3), (1, 2, 3)];

#[derive(Clone, Debug)]
pub struct TetraMass {
    pub edge_mass: SMatrix<f64, 6, 6>,
    pub face_mass: SMatrix<f64, 4, 4>,
    pub volume: f64,
}

pub fn tetra_mass(
    vertices: [[f64; 4]; 4],
    metric: [f64; 3],
    quadrature_order: usize,
) -> Result<TetraMass, String> {
    if !(2..=6).contains(&quadrature_order) {
        return Err("Whitney mass: quadrature order must be in [2,6]".into());
    }
    for v in vertices {
        super::ricci_link::levi_civita_link(metric, v, v)?;
    }
    let points = vertices.map(|v| Vector4::from_row_slice(&v));
    let p = Matrix4::from_columns(&points);
    if p.determinant() <= 1e-16 {
        return Err("Whitney mass: degenerate or negatively oriented radial tetrahedron".into());
    }
    let b = SMatrix::<f64, 4, 3>::from_columns(&[
        points[1] - points[0],
        points[2] - points[0],
        points[3] - points[0],
    ]);
    let gradients = [
        Vector3::new(-1., -1., -1.),
        Vector3::new(1., 0., 0.),
        Vector3::new(0., 1., 0.),
        Vector3::new(0., 0., 1.),
    ];
    let diagonal = Matrix3::from_diagonal(&Vector3::from_row_slice(&metric));
    // Golub-Welsch Gauss-Legendre quadrature, using the existing symmetric eigensolver.
    let n = quadrature_order;
    let mut jacobi = DMatrix::<f64>::zeros(n, n);
    for i in 1..n {
        let off = i as f64 / ((4 * i * i - 1) as f64).sqrt();
        jacobi[(i - 1, i)] = off;
        jacobi[(i, i - 1)] = off;
    }
    let eigen = jacobi.symmetric_eigen();
    let nodes: Vec<_> = (0..n)
        .map(|i| {
            (
                (eigen.eigenvalues[i] + 1.) / 2.,
                eigen.eigenvectors[(0, i)].powi(2),
            )
        })
        .collect();
    let mut result = TetraMass {
        edge_mass: SMatrix::zeros(),
        face_mass: SMatrix::zeros(),
        volume: 0.,
    };
    for &(u, wu) in &nodes {
        for &(v, wv) in &nodes {
            for &(w, ww) in &nodes {
                let (a, beta, c) = (u, (1. - u) * v, (1. - u) * (1. - v) * w);
                let lambda = [1. - a - beta - c, a, beta, c];
                let y = p * Vector4::from_row_slice(&lambda);
                let length = y.norm();
                if length < 1e-10 {
                    return Err("Whitney mass: radial map approaches zero; refine the cell".into());
                }
                let x = y / length;
                let dx = (b - x * (x.transpose() * b)) / length;
                let mut maurer = Matrix3::zeros();
                for i in 0..3 {
                    for axis in 0..3 {
                        let (j, k) = ((i + 1) % 3, (i + 2) % 3);
                        maurer[(i, axis)] = x[0] * dx[(i + 1, axis)]
                            - dx[(0, axis)] * x[i + 1]
                            - x[j + 1] * dx[(k + 1, axis)]
                            + x[k + 1] * dx[(j + 1, axis)];
                    }
                }
                let g = maurer.transpose() * diagonal * maurer;
                let determinant = g.determinant();
                if !determinant.is_finite() || determinant <= 0. {
                    return Err("Whitney mass: nonpositive metric determinant".into());
                }
                let inverse = g.try_inverse().ok_or("Whitney mass: singular metric")?;
                let weight = wu * wv * ww * (1. - u).powi(2) * (1. - v) * determinant.sqrt();
                result.volume += weight;
                let one = EDGES.map(|(i, j)| lambda[i] * gradients[j] - lambda[j] * gradients[i]);
                let two = FACES.map(|(i, j, k)| {
                    2. * (lambda[i] * gradients[j].cross(&gradients[k])
                        - lambda[j] * gradients[i].cross(&gradients[k])
                        + lambda[k] * gradients[i].cross(&gradients[j]))
                });
                for i in 0..6 {
                    for j in 0..6 {
                        result.edge_mass[(i, j)] += weight * one[i].dot(&(inverse * one[j]));
                    }
                }
                for i in 0..4 {
                    for j in 0..4 {
                        result.face_mass[(i, j)] +=
                            weight * two[i].dot(&(g * two[j])) / determinant;
                    }
                }
            }
        }
    }
    if !result.volume.is_finite()
        || result
            .edge_mass
            .iter()
            .chain(result.face_mass.iter())
            .any(|v| !v.is_finite())
    {
        return Err("Whitney mass: nonfinite result; rescale or refine".into());
    }
    Ok(result)
}
