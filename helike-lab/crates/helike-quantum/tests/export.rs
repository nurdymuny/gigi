//! The cone-state export (flux-grid amendment §3.3): selection and chiral mixing at a light control.
use helike_quantum::export;
use helike_quantum::ritz::SolveSettings;

#[test]
fn control_export_selects_and_mixes() {
    let e = export::export_point(1.15, 3.0, 0.004, 4, &SolveSettings::default()).unwrap();
    assert_eq!(e.level_indices[0], 0);
    assert!(e.pair_weights.iter().all(|&w| w >= 0.95) && e.competitor_weight >= 0.95);
    // the pair compresses to charges near −2 and +2
    println!("compressed charges {:?}", e.compressed_charges);
    assert!((e.compressed_charges[0] + 2.0).abs() < 0.05 && (e.compressed_charges[1] - 2.0).abs() < 0.05);
    // the mixing is unitary on the pair block
    let m = |i: usize, j: usize| (f64::from_bits(e.mixing_re[i * 4 + j]), f64::from_bits(e.mixing_im[i * 4 + j]));
    for (a, b) in [(1usize, 1usize), (2, 2)] {
        let n = (1..3).map(|r| { let z = m(r, a); z.0 * z.0 + z.1 * z.1 }).sum::<f64>();
        assert!((n - 1.0).abs() < 1e-12, "column {a}-{b} not normalised");
    }
    let dot = (1..3).fold((0.0, 0.0), |acc, r| {
        let (x, y) = (m(r, 1), m(r, 2));
        (acc.0 + x.0 * y.0 + x.1 * y.1, acc.1 + x.0 * y.1 - x.1 * y.0)
    });
    assert!(dot.0.abs() < 1e-12 && dot.1.abs() < 1e-12, "pair columns not orthogonal");
    // centres: the Helike pair shares its midpoint
    let c: Vec<f64> = e.centers.iter().map(|&b| f64::from_bits(b)).collect();
    assert_eq!(c[1], c[2]);
}
