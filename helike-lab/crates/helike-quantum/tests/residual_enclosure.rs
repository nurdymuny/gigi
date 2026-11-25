//! Rigorous residual enclosures of the four saved quasimode controls (oracle/quasimode_states_e21087a1.json,
//! exported bit-exactly by oracle/export_quasimode_states.py), against the reference's centred floating-point
//! moments (`helike_residual_moments.norm_squared`, d0297072). Those are not enclosures. The test is that every one
//! lies inside the interval, and that each deliberate fault moves it outside.

use helike_quantum::laurent;
use helike_quantum::residual::{self, ControlEnclosure, Mutation, SavedControl, SavedStates};
use std::collections::BTreeMap;
use std::sync::OnceLock;

fn saved() -> &'static SavedStates {
    static S: OnceLock<SavedStates> = OnceLock::new();
    S.get_or_init(|| {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../oracle/quasimode_states_e21087a1.json");
        residual::load_saved_states(std::path::Path::new(p)).expect("run oracle/export_quasimode_states.py first")
    })
}

fn control(g2: f64, mq: usize) -> &'static SavedControl {
    saved().controls.iter().find(|c| c.g2 == g2 && c.mq == mq).expect("control present")
}

fn check_encloses(r: &ControlEnclosure, c: &SavedControl) {
    assert_eq!(r.group_count, c.reference_centered_moments.as_ref().expect("controls carry reference moments").group_count, "group count");
    assert_eq!(r.workspace_degree, c.reference_centered_moments.as_ref().expect("controls carry reference moments").workspace_degree, "workspace degree");
    for s in &r.states {
        assert!(s.reference_inside == Some(true), "g2 {} Mq {} {}: reference {} outside [{}, {}]", r.g2, r.mq, s.name,
                s.reference_residual_sq.expect("controls carry a reference value"), s.residual_sq[0], s.residual_sq[1]);
        assert!(s.r_lower > 0.0 && s.r_lower <= s.r_upper);
        // the enclosure must be useful, not merely valid: relative width of the residual square below 1e-2
        let w = s.residual_sq[1] - s.residual_sq[0];
        assert!(w < 1e-2 * s.residual_sq[0], "{}: enclosure too wide ({w})", s.name);
        // normalised trial states
        assert!(s.norm_sq[0] <= 1.0 + 1e-11 && s.norm_sq[1] >= 1.0 - 1e-11);
    }
    // the reference's second (independent quadrature-translation) path also lies inside
    for (s, v) in r.states.iter().zip(&c.reference_centered_moments.as_ref().expect("controls carry reference moments").independent) {
        assert!(s.residual_sq[0] <= *v && *v <= s.residual_sq[1], "{}: independent reference {v} outside", s.name);
    }
}

#[test]
fn mq6_controls_enclose_the_reference() {
    for g2 in [0.004, 0.002] {
        let c = control(g2, 6);
        let r = residual::enclose_control(c, Mutation::None).unwrap();
        check_encloses(&r, c);
    }
}

#[test]
fn mq8_controls_enclose_the_reference() {
    for g2 in [0.004, 0.002] {
        let c = control(g2, 8);
        let r = residual::enclose_control(c, Mutation::None).unwrap();
        check_encloses(&r, c);
    }
}

#[test]
fn every_fault_moves_the_reference_outside() {
    let c = control(0.004, 6);
    let clean = residual::enclose_control(c, Mutation::None).unwrap();
    for m in [Mutation::DropMeasureDrift, Mutation::DropCross, Mutation::DropExponential, Mutation::NoConjugate] {
        let r = residual::enclose_control(c, m).unwrap();
        // caught: some state's reference lies outside by more than 100 times its clean enclosure width
        let caught = r.states.iter().zip(&clean.states).any(|(s, k)| {
            let w = k.residual_sq[1] - k.residual_sq[0];
            s.reference_inside == Some(false) && s.reference_outside_by.expect("reference present") > 100.0 * w
        });
        assert!(caught, "{m:?} was not caught");
    }
}

#[test]
fn a_wrong_cometric_term_is_refused() {
    let c = control(0.004, 6);
    let mut pm: Vec<(usize, usize, laurent::Key, f64)> =
        c.python_metric.iter().map(|m| (m.i, m.j, m.key.as_slice().try_into().unwrap(), m.c)).collect();
    assert!(residual::verified_cometric(&pm).is_ok());
    pm[3].3 += 1.0;
    assert!(residual::verified_cometric(&pm).is_err());
    pm[3].3 -= 1.0;
    pm.pop();
    assert!(residual::verified_cometric(&pm).is_err(), "a missing reference term must be refused");
}

#[test]
fn potential_readings_agree_and_a_wrong_coefficient_is_refused() {
    let c = control(0.004, 6);
    let mut py: BTreeMap<laurent::Key, f64> = c
        .python_potential
        .iter()
        .map(|p| (p.key.as_slice().try_into().unwrap(), f64::from_bits(p.c)))
        .collect();
    let g = f64::from_bits(c.g_bits);
    let v = residual::potential_enclosure(c.s, c.t, c.g2, g, &py).unwrap();
    assert_eq!(v.len(), 15);
    for (k, iv) in &v {
        assert!(iv.contains(py[k]));
        // the declared-decimal and binary64 readings of (s, g) differ at about 1e-15 relative; the hull covers both
        assert!(iv.width() < 1e-14 * iv.mag().max(1e-3), "{k:?}: {iv:?}");
    }
    let k = *py.keys().next().unwrap();
    *py.get_mut(&k).unwrap() *= 1.0 + 1e-9;
    assert!(residual::potential_enclosure(c.s, c.t, c.g2, g, &py).is_err());
}
