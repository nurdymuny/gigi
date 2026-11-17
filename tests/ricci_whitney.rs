#![cfg(feature = "gauge")]
use gigi::gauge::ricci_whitney::tetra_mass;

fn orthant() -> [[f64; 4]; 4] {
    [
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., 1.],
    ]
}

#[test]
fn bw03_physical_volume_and_positive_full_mass() {
    let m = tetra_mass(orthant(), [1.; 3], 6).unwrap();
    println!(
        "orthant volume {} exact {}",
        m.volume,
        std::f64::consts::PI.powi(2) / 8.
    );
    assert!((m.volume - std::f64::consts::PI.powi(2) / 8.).abs() < 0.002);
    assert!(m.edge_mass.cholesky().is_some());
    assert!(m.face_mass.cholesky().is_some());
    assert!(m.edge_mass[(0, 1)].abs() > 1e-3);
    assert!(m.face_mass[(0, 1)].abs() > 1e-3);
}

#[test]
fn bw03_metric_tensor_scaling_and_anisotropy() {
    let a = tetra_mass(orthant(), [3., 1., 1.], 4).unwrap();
    let b = tetra_mass(orthant(), [21., 7., 7.], 4).unwrap();
    assert!(a.volume > 1.);
    assert!((b.volume / a.volume - 7f64.powf(1.5)).abs() < 1e-12);
    assert!((b.edge_mass - a.edge_mass * 7f64.sqrt()).norm() < 1e-12);
    assert!((b.face_mass - a.face_mass / 7f64.sqrt()).norm() < 1e-12);
    let c = tetra_mass(orthant(), [1., 3., 1.], 4).unwrap();
    assert!((a.edge_mass - c.edge_mass).norm() > 0.01);
    assert!((a.face_mass - c.face_mass).norm() > 0.01);
}

#[test]
fn bw03_quadrature_refinement_not_fitted_normalization() {
    let exact = std::f64::consts::PI.powi(2) / 8.;
    let a = tetra_mass(orthant(), [1.; 3], 2).unwrap();
    let b = tetra_mass(orthant(), [1.; 3], 6).unwrap();
    assert!((b.volume - exact).abs() < (a.volume - exact).abs() / 20.);
}

#[test]
fn bw03_missing_metric_degenerate_cell_and_budget_refuse() {
    assert!(tetra_mass(orthant(), [0., 1., 1.], 4).is_err());
    assert!(tetra_mass(orthant(), [f64::NAN, 1., 1.], 4).is_err());
    assert!(tetra_mass(orthant(), [1.; 3], 0).is_err());
    assert!(tetra_mass(orthant(), [1.; 3], 7).is_err());
    let mut degenerate = orthant();
    degenerate[3] = degenerate[2];
    assert!(tetra_mass(degenerate, [1.; 3], 4).is_err());
}
