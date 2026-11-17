#![cfg(feature = "gauge")]
use gigi::gauge::{
    group_element::GroupElement,
    holonomy::walk_loop,
    ricci_simplicial::{cell_action, cell_tangent_mass},
    staple::{build_edge_face_incidence, build_face_edges_cache},
    su2_gauge_field::{GaugeFieldInit, SU2GaugeField},
    wilson_force::wilson_force_per_edge,
};
use gigi::lattice::Lattice;
use nalgebra::SMatrix;

fn fixture() -> (Lattice, SU2GaugeField, [[f64; 4]; 6]) {
    let lat = Lattice::new(
        "simplicial",
        4,
        vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)],
        vec![vec![0, 1, 2], vec![0, 1, 3], vec![0, 2, 3], vec![1, 2, 3]],
        None,
    );
    let u = SU2GaugeField::new(
        "simplicial".into(),
        &lat,
        GaugeFieldInit::HaarRandom,
        Some(20261117),
    )
    .unwrap();
    let links = std::array::from_fn(|i| std::array::from_fn(|j| u.buffer.data[4 * i + j]));
    (lat, u, links)
}

#[test]
fn bw04_diagonal_mass_reduces_to_actual_native_wilson_action_and_force() {
    let (lat, u, links) = fixture();
    let inc = build_edge_face_incidence(&lat);
    let faces = build_face_edges_cache(&lat);
    let beta = 2.5;
    let force = wilson_force_per_edge(&u, &lat, &inc, &faces, beta).unwrap();
    let action = beta / 4.
        * faces
            .iter()
            .map(|f| 1. - walk_loop(&lat, f, &u).re_trace_half())
            .sum::<f64>();
    let result = cell_action(links, SMatrix::<f64, 4, 4>::identity() * (beta / 16.)).unwrap();
    assert!(
        (result.energy - action).abs() < 2e-13,
        "{} != {action}",
        result.energy
    );
    for i in 0..6 {
        for a in 0..3 {
            assert!((result.gradient[3 * i + a] + 2. * force[i][a + 1]).abs() < 2e-13);
        }
    }
}

#[test]
fn bw04_plant_one_link_and_correct_physical_factor() {
    let h = 0.3_f64;
    let mut links = [[1., 0., 0., 0.]; 6];
    links[0] = [h.cos(), h.sin(), 0., 0.];
    let result = cell_action(links, SMatrix::identity()).unwrap();
    assert!((result.energy - 8. * (1. - h.cos())).abs() < 1e-13);
    assert!((result.gradient[0] - 8. * h.sin()).abs() < 1e-13);
}

#[test]
fn bw04_cross_face_coupling_is_part_of_energy() {
    let h=0.3_f64;
    let mut links=[[1.,0.,0.,0.];6];links[0]=[h.cos(),h.sin(),0.,0.];
    let mut mass=SMatrix::<f64,4,4>::identity();
    mass[(0,1)]=0.25;mass[(1,0)]=0.25;
    let result=cell_action(links,mass).unwrap();
    assert!((result.energy-10.*(1.-h.cos())).abs()<1e-13);
    assert!((result.gradient[0]-10.*h.sin()).abs()<1e-13);
}

fn q(a: [f64; 4]) -> GroupElement {
    GroupElement::SU2 {
        q0: a[0],
        q1: a[1],
        q2: a[2],
        q3: a[3],
    }
}
fn parts(a: GroupElement) -> [f64; 4] {
    match a {
        GroupElement::SU2 { q0, q1, q2, q3 } => [q0, q1, q2, q3],
        _ => unreachable!(),
    }
}

#[test]
fn bw04_full_mass_directional_derivatives_and_gauge_invariance() {
    let (_, _, links) = fixture();
    let mut mass = SMatrix::<f64, 4, 4>::identity();
    mass[(0, 3)] = 0.2;
    mass[(3, 0)] = 0.2;
    let result = cell_action(links, mass).unwrap();
    assert!(result.energy > 1.);
    for edge in 0..6 {
        for axis in 0..3 {
            let h = 1e-5_f64;
            let mut exp = [h.cos(), 0., 0., 0.];
            exp[axis + 1] = h.sin();
            let (mut plus, mut minus) = (links, links);
            plus[edge] = parts(q(links[edge]).compose(&q(exp)));
            minus[edge] = parts(q(links[edge]).compose(&q(exp).inverse()));
            let derivative = (cell_action(plus, mass).unwrap().energy
                - cell_action(minus, mass).unwrap().energy)
                / (2. * h);
            assert!((derivative - result.gradient[3 * edge + axis]).abs() < 2e-9);
        }
    }
    let gauge = [q(links[0]), q(links[1]), q(links[2]), q(links[3])];
    let edges = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    let transformed = std::array::from_fn(|i| {
        let (a, b) = edges[i];
        parts(gauge[a].compose(&q(links[i])).compose(&gauge[b].inverse()))
    });
    let changed = cell_action(transformed, mass).unwrap();
    assert!((changed.energy - result.energy).abs() < 1e-12);
    for (e, (_, target)) in edges.into_iter().enumerate() {
        let v = q([
            0.,
            result.gradient[3 * e],
            result.gradient[3 * e + 1],
            result.gradient[3 * e + 2],
        ]);
        let rotated = parts(gauge[target].compose(&v).compose(&gauge[target].inverse()));
        for a in 0..3 {
            assert!((changed.gradient[3 * e + a] - rotated[a + 1]).abs() < 2e-12);
        }
    }
}

#[test]
fn bw04_invalid_mass_and_nonunit_link_refuse() {
    let (_, _, links) = fixture();
    assert!(cell_action(links, SMatrix::zeros()).is_err());
    let mut mass = SMatrix::identity();
    mass[(0, 1)] = 1.;
    assert!(cell_action(links, mass).is_err());
    let mut invalid = links;
    invalid[0] = [2., 0., 0., 0.];
    assert!(cell_action(invalid, SMatrix::identity()).is_err());
}

#[test]
fn bw04_tangent_mass_physical_norm_spd_and_coupling() {
    let (_, _, links) = fixture();
    let mut m = SMatrix::<f64, 6, 6>::identity();
    m[(0, 5)] = 0.2;
    m[(5, 0)] = 0.2;
    let b = cell_tangent_mass(links, m).unwrap();
    assert!(b.cholesky().is_some());
    assert!((b - b.transpose()).norm() < 1e-13);
    assert!(
        (b.fixed_view::<3, 3>(0, 0).into_owned() - SMatrix::<f64, 3, 3>::identity() * 4.).norm()
            < 1e-13
    );
    assert!((b.fixed_view::<3, 3>(0, 15).norm() - 0.8 * 3_f64.sqrt()).abs() < 1e-13);
    let identity = cell_tangent_mass([[1., 0., 0., 0.]; 6], m).unwrap();
    for e in 0..6 {
        for f in 0..6 {
            for a in 0..3 {
                for c in 0..3 {
                    assert!(
                        (identity[(3 * e + a, 3 * f + c)]
                            - if a == c { 4. * m[(e, f)] } else { 0. })
                        .abs()
                            < 1e-13
                    );
                }
            }
        }
    }
    assert!((cell_tangent_mass(links, 3. * m).unwrap() - 3. * b).norm() < 1e-12);
}

#[test]
fn bw04_tangent_mass_vertex_gauge_covariance() {
    let (_, _, links) = fixture();
    let gauge = [q(links[0]), q(links[1]), q(links[2]), q(links[3])];
    let edges = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    let changed = std::array::from_fn(|e| {
        let (a, b) = edges[e];
        parts(gauge[a].compose(&q(links[e])).compose(&gauge[b].inverse()))
    });
    let mut m = SMatrix::<f64, 6, 6>::identity();
    m[(0, 5)] = 0.2;
    m[(5, 0)] = 0.2;
    let b = cell_tangent_mass(links, m).unwrap();
    let transformed = cell_tangent_mass(changed, m).unwrap();
    let mut r = SMatrix::<f64, 18, 18>::zeros();
    for (e, (_, target)) in edges.into_iter().enumerate() {
        for a in 0..3 {
            let mut v = [0.; 4];
            v[a + 1] = 1.;
            let rotated = parts(
                gauge[target]
                    .compose(&q(v))
                    .compose(&gauge[target].inverse()),
            );
            for c in 0..3 {
                r[(3 * e + c, 3 * e + a)] = rotated[c + 1];
            }
        }
    }
    assert!(b.norm() > 1.);
    assert!((transformed - r * b * r.transpose()).norm() < 3e-12);
}

#[test]
fn bw04_tangent_mass_refuses_invalid_input() {
    let (_, _, links) = fixture();
    assert!(cell_tangent_mass(links, SMatrix::zeros()).is_err());
    let mut bad = links;
    bad[5][0] = f64::NAN;
    assert!(cell_tangent_mass(bad, SMatrix::identity()).is_err());
}
