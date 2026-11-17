#![cfg(feature = "gauge")]

use gigi::gauge::{
    e_field::{EFieldInit, SU2EField},
    group_element::GroupElement,
    holonomy::walk_loop,
    lie_exp::drift_step,
    ricci_link::levi_civita_link,
    staple::{build_edge_face_incidence, build_face_edges_cache},
    su2_gauge_field::{GaugeFieldInit, SU2GaugeField},
    wilson_force::wilson_force_per_edge,
};
use gigi::lattice::Lattice;

fn components(q: GroupElement) -> [f64; 4] {
    match q {
        GroupElement::SU2 { q0, q1, q2, q3 } => [q0, q1, q2, q3],
        _ => panic!("SU2 required"),
    }
}
fn tetra() -> Lattice {
    Lattice::new(
        "bw00",
        4,
        vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)],
        vec![vec![0, 2, 1], vec![0, 1, 3], vec![1, 2, 3], vec![0, 3, 2]],
        None,
    )
}

#[test]
fn bw00_native_force_is_action_derivative_in_actual_drift_coordinates() {
    let lat = tetra();
    let u = SU2GaugeField::new(
        "bw00".into(),
        &lat,
        GaugeFieldInit::HaarRandom,
        Some(20261117),
    )
    .unwrap();
    let inc = build_edge_face_incidence(&lat);
    let faces = build_face_edges_cache(&lat);
    let beta = 2.5;
    let force = wilson_force_per_edge(&u, &lat, &inc, &faces, beta).unwrap();
    let action = |v: &SU2GaugeField| {
        beta / 4.
            * faces
                .iter()
                .map(|f| 1. - walk_loop(&lat, f, v).re_trace_half())
                .sum::<f64>()
    };
    let mut max_error = 0f64;
    for edge in 0..lat.n_edges() {
        for axis in 0..3 {
            let mut e = SU2EField::new("bw00_e".into(), &u, EFieldInit::Zero, None).unwrap();
            let mut row = [0.; 4];
            row[axis + 1] = 1.;
            e.write_element_q(edge, row);
            let h = 1e-5;
            let (mut plus, mut minus) = (u.clone(), u.clone());
            drift_step(&mut plus, &e, h, 1.).unwrap();
            drift_step(&mut minus, &e, -h, 1.).unwrap();
            let derivative = (action(&plus) - action(&minus)) / (2. * h);
            max_error = max_error.max((derivative + 2. * force[edge][axis + 1]).abs());
        }
    }
    println!("BW00 native directional derivative max error={max_error:e}");
    assert!(
        max_error < 2e-9,
        "native action/force mismatch {max_error:e}"
    );
    assert!(
        force.iter().any(|r| r[1].abs() > 0.01),
        "noncritical fixture"
    );
}

#[test]
fn bw02_exact_axis_transport_and_reverse() {
    let h = 0.3_f64;
    let (p, q) = ([1., 0., 0., 0.], [h.cos(), h.sin(), 0., 0.]);
    let u = levi_civita_link([3., 1., 1.], p, q).unwrap();
    let got = components(u);
    for (a, b) in got
        .into_iter()
        .zip([(h / 2.).cos(), (h / 2.).sin(), 0., 0.])
    {
        assert!((a - b).abs() < 2e-14, "transport {a} != {b}");
    }
    let reverse = levi_civita_link([3., 1., 1.], q, p).unwrap();
    for (a, b) in components(reverse).into_iter().zip(components(u.inverse())) {
        assert!((a - b).abs() < 2e-14);
    }
    assert_eq!(
        levi_civita_link([3., 1., 1.], p, p).unwrap(),
        GroupElement::su2_identity()
    );
}

#[test]
fn bw02_oriented_vector_curvature_converges_not_only_trace() {
    let mut errors = Vec::new();
    for h in [0.08_f64, 0.04, 0.02, 0.01] {
        let vertices = [
            [1., 0., 0., 0.],
            [h.cos(), h.sin(), 0., 0.],
            [h.cos(), 0., h.sin(), 0.],
        ];
        let mut hol = GroupElement::su2_identity();
        for (a, b) in [(0, 1), (1, 2), (2, 0)] {
            hol = hol.compose(&levi_civita_link([3., 1., 1.], vertices[a], vertices[b]).unwrap());
        }
        let q = components(hol);
        let expected = [0., 0., -3. * 3f64.sqrt()];
        let error = (0..3)
            .map(|i| (-4. * q[i + 1] / (h * h) - expected[i]).abs())
            .fold(0., f64::max);
        errors.push(error);
    }
    println!("BW02 local curvature errors {errors:?}");
    assert!(errors[3] < 0.1, "{errors:?}");
    for i in 0..3 {
        assert!(errors[i] / errors[i + 1] > 1.8, "{errors:?}");
    }
}

#[test]
fn bw02_input_refusals() {
    let p = [1., 0., 0., 0.];
    for g in [
        [0., 1., 1.],
        [-1., 1., 1.],
        [f64::NAN, 1., 1.],
        [1e-6, 1e6, 1.],
    ] {
        assert!(levi_civita_link(g, p, p).is_err());
    }
    for q in [
        [2., 0., 0., 0.],
        [-1., 0., 0., 0.],
        [f64::NAN, 0., 0., 0.],
        [0., 0., 0., 0.],
    ] {
        assert!(levi_civita_link([1.; 3], p, q).is_err());
    }
}

#[test]
fn bw02_scale_does_not_change_connection_links() {
    let p = [1., 0., 0., 0.];
    let h = 0.3_f64;
    let q = [h.cos(), 0., h.sin(), 0.];
    let a = components(levi_civita_link([3., 1., 1.], p, q).unwrap());
    let b = components(levi_civita_link([21., 7., 7.], p, q).unwrap());
    for i in 0..4 {
        assert!((a[i] - b[i]).abs() < 1e-14);
    }
    assert!(a[2].abs() > 0.1, "nontrivial mapped connection");
}
