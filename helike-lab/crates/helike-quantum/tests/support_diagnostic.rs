use helike_quantum::{basis, support};
use helike_quantum::model::Background;
use helike_quantum::operator::{ConeOperator, Mutation, Part};

const ROLES: [usize; 6] = [0, 1, 2, 3, 4, 5];

#[test]
fn planted_probabilities_and_joint_bins() {
    let b = vec![[0; 6], [1, 0, 0, 0, 0, 0], [0, 0, 2, 0, 0, 0]];
    let p = support::probabilities(&[vec![0.0, -0.6, 0.8]]).unwrap();
    let d = support::summarize(&b, &p, ROLES).unwrap();
    assert!((d.mass - 1.0).abs() < 1e-14);
    assert!((d.shells[1] - 0.36).abs() < 1e-14);
    assert!((d.shells[2] - 0.64).abs() < 1e-14);
    let means = d.mean_group_occupation.unwrap();
    assert!((means[0] - 0.36).abs() < 1e-14);
    assert!((means[1] - 1.28).abs() < 1e-14);
    assert!(d.joint.iter().any(|x| x.occupation == [0, 2, 0, 0] && (x.mass - 0.64).abs() < 1e-14));
    assert!((d.occupation_fraction.unwrap().iter().sum::<f64>() - 1.0).abs() < 1e-14);
}

#[test]
fn zero_distribution_has_no_normalized_summary() {
    let d = support::summarize(&[[0; 6]], &[0.0], ROLES).unwrap();
    assert_eq!(d.mass, 0.0);
    assert!(d.mean_group_occupation.is_none());
    assert!(d.occupation_fraction.is_none());
}

#[test]
fn residual_distribution_normalizes_by_its_own_mass() {
    let b = [[1,0,0,0,0,0], [0,0,2,0,0,0]];
    let d = support::summarize(&b, &[2.0, 6.0], ROLES).unwrap();
    assert_eq!(d.mass, 8.0);
    assert_eq!(d.mean_group_occupation.unwrap(), [0.25, 1.5, 0.0, 0.0]);
    assert_eq!(d.occupation_fraction.unwrap(), [0.25, 0.75, 0.0, 0.0]);
}

#[test]
fn sign_and_subspace_rotation_do_not_change_average_support() {
    let a = support::probabilities(&[vec![1.0, 0.0], vec![0.0, 1.0]]).unwrap();
    let r = 0.5f64.sqrt();
    let b = support::probabilities(&[vec![r, r], vec![-r, r]]).unwrap();
    for (x, y) in a.iter().zip(b) { assert!((x - y).abs() < 1e-14); }
    assert_eq!(support::probabilities(&[vec![-0.6, -0.8]]).unwrap(),
               support::probabilities(&[vec![0.6, 0.8]]).unwrap());
}

#[test]
fn embedding_uses_occupation_identity_not_order() {
    let a = [[0; 6], [1, 0, 0, 0, 0, 0]];
    let b = [a[1], [0, 1, 0, 0, 0, 0], a[0]];
    assert_eq!(support::embed(&a, &b, &[0.6, -0.8]).unwrap(), vec![-0.8, 0.0, 0.6]);
    assert!(support::embed(&a, &[a[0]], &[0.6, -0.8]).is_err());
    assert!(support::embed(&a, &[a[0], a[0]], &[0.6, -0.8]).is_err());
}

#[test]
fn coupled_matrix_exposes_omitted_residual_despite_zero_inner_residual() {
    // H = [[2,3],[3,5]], trial vector (1,0), Ritz value 2 in the first coordinate.
    let a = [[0; 6]];
    let b = [[0; 6], [1, 0, 0, 0, 0, 0]];
    let r = support::split_residual(&a, &b, &[1.0, 0.0], &[2.0, 3.0], 2.0).unwrap();
    assert_eq!(r.inner_norm, 0.0);
    assert_eq!(r.outer_norm, 3.0);
    assert_eq!(r.residual, vec![0.0, 3.0]);
    assert!((r.total_norm.powi(2) - r.inner_norm.powi(2) - r.outer_norm.powi(2)).abs() < 1e-14);
}

#[test]
fn bad_measurements_refuse() {
    assert!(support::probabilities(&[]).is_err());
    assert!(support::probabilities(&[vec![f64::NAN]]).is_err());
    assert!(support::probabilities(&[vec![0.0], vec![]]).is_err());
    assert!(support::summarize(&[[0; 6]], &[-1.0], ROLES).is_err());
    assert!(support::summarize(&[[0; 6]], &[1.0], [0; 6]).is_err());
    assert!(support::summarize(&[[0; 6]], &[], ROLES).is_err());
}

#[test]
fn nested_operator_actions_agree_on_both_weak_controls() {
    for g2 in [0.002f64, 0.004] {
        let bg = Background::maurer_cartan(1.15, 3.0, g2.sqrt()).unwrap();
        let a = ConeOperator::exact(bg.clone(), 2, Mutation::None).unwrap();
        let b = ConeOperator::exact(bg, 4, Mutation::None).unwrap();
        let v: Vec<f64> = (0..a.dim()).map(|i| (i as f64 + 1.0).sin()).collect();
        let padded = support::embed(&a.basis, &b.basis, &v).unwrap();
        let av = a.apply(&[v], Part::Full);
        let bv = b.apply(&[padded], Part::Full);
        let idx = basis::index_of(&b.basis);
        let error = a.basis.iter().enumerate().map(|(i, n)| (av[0][i] - bv[0][idx[n]]).powi(2)).sum::<f64>().sqrt();
        assert!(error < 1e-10, "g2={g2}, nesting error={error}");
    }
}
