//! Behavioral gates: initially red at parse(), not missing symbols.
#[cfg(feature = "gauge")]
use gigi::parser::ExecResult;
use gigi::parser::{execute, parse};

#[test]
fn mg00_grammar() {
    for q in [
        "MONODROMY_GROUP_ORDER f FROM EDGES (0, 3);",
        "MONODROMY_GROUP_ORDER f FROM EDGES (0) COMMUTATOR CAP 120 TOLERANCE 0.00000001;",
        "MONODROMY_GROUP_ORDER f FROM EDGES (0) TOLERANCE 0.000001 CAP 1 COMMUTATOR;",
    ] {
        assert!(parse(q).is_ok(), "{q}: {:?}", parse(q));
    }
}

#[test]
fn mg00_bad_grammar() {
    let too_many = vec!["0"; 33].join(",");
    assert!(parse(&format!("MONODROMY_GROUP_ORDER f FROM EDGES ({too_many});")).is_err());
    for tail in [
        "()",
        "(-1)",
        "(0.5)",
        "(0) CAP 0",
        "(0) CAP 257",
        "(0) CAP 1.5",
        "(0) CAP 2 CAP 3",
        "(0) COMMUTATOR COMMUTATOR",
        "(0) TOLERANCE 0",
        "(0) TOLERANCE 0.1",
        "(0) TOLERANCE 0.00000001 TOLERANCE 0.00000001",
        "(0) WHOOPS 1",
    ] {
        assert!(
            parse(&format!("MONODROMY_GROUP_ORDER f FROM EDGES {tail};")).is_err(),
            "{tail}"
        );
    }
}

#[cfg(not(feature = "gauge"))]
#[test]
fn mg07_feature_off_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = gigi::engine::Engine::open(dir.path()).unwrap();
    let stmt = parse("MONODROMY_GROUP_ORDER f FROM EDGES (0);").unwrap();
    assert!(execute(&mut engine, &stmt).unwrap_err().contains("gauge"));
}

#[cfg(feature = "gauge")]
mod enabled {
    use super::*;
    use gigi::gauge::group_element::GroupElement;
    use gigi::gauge::{registry, DenseLinkBuffer, GaugeFieldInit, Group, SU2GaugeField};
    use gigi::types::{Record, Value};
    use std::sync::{Arc, RwLock};

    type Q = [f64; 4];
    const ID: Q = [1., 0., 0., 0.];
    const S: Q = [0.5, 0.5, 0.5, 0.5];
    fn t() -> Q {
        let phi = (1. + 5f64.sqrt()) / 2.;
        [phi / 2., 0.5, (phi - 1.) / 2., 0.]
    }
    fn ge(q: Q) -> GroupElement {
        GroupElement::SU2 {
            q0: q[0],
            q1: q[1],
            q2: q[2],
            q3: q[3],
        }
    }
    fn unpack(g: GroupElement) -> Q {
        if let GroupElement::SU2 { q0, q1, q2, q3 } = g {
            [q0, q1, q2, q3]
        } else {
            panic!()
        }
    }
    fn register(qs: &[Q]) {
        let mut buf = DenseLinkBuffer::new_identity(Group::SU2, qs.len()).unwrap();
        for (i, q) in qs.iter().enumerate() {
            buf.data[i * 4..i * 4 + 4].copy_from_slice(q);
        }
        registry::register(Arc::new(SU2GaugeField::from_buffer(
            "mg_fixture".into(),
            "supplied_generators".into(),
            buf,
            GaugeFieldInit::Identity,
            None,
        )));
    }
    fn run(qs: &[Q], tail: &str) -> Record {
        register(qs);
        let dir = tempfile::tempdir().unwrap();
        let mut engine = gigi::engine::Engine::open(dir.path()).unwrap();
        let ids = (0..qs.len())
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let stmt = parse(&format!(
            "MONODROMY_GROUP_ORDER mg_fixture FROM EDGES ({ids}) {tail};"
        ))
        .unwrap();
        match execute(&mut engine, &stmt).unwrap() {
            ExecResult::Rows(mut rs) => {
                assert_eq!(rs.len(), 1);
                rs.remove(0)
            }
            other => panic!("not a computed row: {other:?}"),
        }
    }
    fn int(r: &Record, k: &str) -> i64 {
        if let Some(Value::Integer(n)) = r.get(k) {
            *n
        } else {
            panic!("{k}: {r:?}")
        }
    }
    fn closed(r: &Record, n: i64, derived: i64) {
        assert_eq!(
            r.get("status"),
            Some(&Value::Text("numerically_closed".into()))
        );
        assert_eq!(int(r, "group_order"), n);
        assert_eq!(int(r, "commutator_subgroup_order"), derived);
        assert_eq!(int(r, "abelianization_order"), n / derived);
        assert_eq!(r.get("is_perfect"), Some(&Value::Bool(n == derived)));
        assert_eq!(r.get("table_verified"), Some(&Value::Bool(true)));
        assert_eq!(r.get("exact_subgroup_certified"), Some(&Value::Bool(false)));
        assert_eq!(int(r, "associativity_checks"), n * n * n);
        assert!(int(r, "products_evaluated") >= n * n);
    }
    fn histogram(r: &Record, orders: &[f64], counts: &[f64]) {
        assert_eq!(
            r.get("element_orders"),
            Some(&Value::Vector(orders.to_vec()))
        );
        assert_eq!(
            r.get("element_order_counts"),
            Some(&Value::Vector(counts.to_vec()))
        );
    }

    #[test]
    fn mg01_identity_central_and_cyclic() {
        let _guard = registry::test_serial_lock();
        let r = run(&[ID], "COMMUTATOR CAP 1");
        closed(&r, 1, 1);
        histogram(&r, &[1.], &[1.]);
        let r = run(&[[-1., 0., 0., 0.]], "COMMUTATOR");
        closed(&r, 2, 1);
        histogram(&r, &[1., 2.], &[1., 1.]);
        let a = std::f64::consts::TAU / 5.;
        let r = run(&[[a.cos(), 0., 0., a.sin()]], "COMMUTATOR CAP 5");
        closed(&r, 5, 1);
        histogram(&r, &[1., 5.], &[1., 4.]);
    }
    #[test]
    fn mg01_quaternion_and_tetrahedral() {
        let _guard = registry::test_serial_lock();
        let i = [0., 1., 0., 0.];
        let j = [0., 0., 1., 0.];
        let r = run(&[i, j], "COMMUTATOR");
        closed(&r, 8, 2);
        histogram(&r, &[1., 2., 4.], &[1., 1., 6.]);
        let r = run(&[i, S], "COMMUTATOR");
        closed(&r, 24, 8);
        histogram(&r, &[1., 2., 3., 4., 6.], &[1., 1., 8., 6., 8.]);
    }
    #[test]
    fn mg02_icosahedral_120_and_perfection() {
        let _guard = registry::test_serial_lock();
        let r = run(&[S, t()], "COMMUTATOR CAP 120");
        closed(&r, 120, 120);
        histogram(
            &r,
            &[1., 2., 3., 4., 5., 6., 10.],
            &[1., 1., 20., 30., 24., 20., 24.],
        );
    }
    #[test]
    fn mg03_invariance_and_generator_removal() {
        let _guard = registry::test_serial_lock();
        let h = ge([0.6, 0., 0.8, 0.]);
        let conjugate = |q| unpack(h.compose(&ge(q)).compose(&h.inverse()));
        for qs in [
            vec![t(), S],
            vec![S, t(), S, ID],
            vec![unpack(ge(S).inverse()), t()],
            vec![conjugate(S), conjugate(t())],
        ] {
            for tol in ["0.000000000001", "0.00000001", "0.000001"] {
                closed(&run(&qs, &format!("COMMUTATOR TOLERANCE {tol}")), 120, 120);
            }
        }
        closed(&run(&[S], "COMMUTATOR"), 6, 1);
        closed(&run(&[t()], "COMMUTATOR"), 10, 1);
    }
    #[test]
    fn mg04_cap_and_irrational_are_not_finite_orders() {
        let _guard = registry::test_serial_lock();
        for (qs, tail) in [
            (vec![S, t()], "COMMUTATOR CAP 119"),
            (
                vec![[2f64.sqrt().cos(), 0., 0., 2f64.sqrt().sin()]],
                "COMMUTATOR CAP 32",
            ),
        ] {
            let r = run(&qs, tail);
            assert_eq!(r.get("status"), Some(&Value::Text("unresolved".into())));
            for k in [
                "group_order",
                "commutator_subgroup_order",
                "is_perfect",
                "abelianization_order",
            ] {
                assert_eq!(r.get(k), Some(&Value::Null), "{k}");
            }
            assert_eq!(r.get("table_verified"), Some(&Value::Bool(false)));
        }
    }
    #[test]
    fn mg05_invalid_elements_and_edges_refuse() {
        let _guard = registry::test_serial_lock();
        let dir = tempfile::tempdir().unwrap();
        let mut engine = gigi::engine::Engine::open(dir.path()).unwrap();
        let stmt = parse("MONODROMY_GROUP_ORDER mg_fixture FROM EDGES (0);").unwrap();
        for q in [
            [0.; 4],
            [2., 0., 0., 0.],
            [f64::NAN, 0., 0., 0.],
            [f64::INFINITY, 0., 0., 0.],
        ] {
            register(&[q]);
            assert!(execute(&mut engine, &stmt)
                .unwrap_err()
                .contains("generator"));
        }
        register(&[ID]);
        let bad = parse("MONODROMY_GROUP_ORDER mg_fixture FROM EDGES (2);").unwrap();
        assert!(execute(&mut engine, &bad).unwrap_err().contains("edge"));
        let missing = parse("MONODROMY_GROUP_ORDER mg_missing FROM EDGES (0);").unwrap();
        assert!(execute(&mut engine, &missing)
            .unwrap_err()
            .contains("not declared"));
    }
    #[test]
    fn mg05_wrong_group_malformed_buffer_and_direct_options_refuse() {
        let _guard = registry::test_serial_lock();
        use gigi::monodromy::execute_monodromy_group_order as direct;
        let buf = DenseLinkBuffer::new_identity(Group::U1, 1).unwrap();
        registry::register(Arc::new(gigi::gauge::U1GaugeField::from_buffer(
            "mg_fixture".into(),
            "supplied_generators".into(),
            buf,
            GaugeFieldInit::Identity,
            None,
        )));
        assert!(direct("mg_fixture", &[0], true, 256, 1e-8)
            .unwrap_err()
            .contains("requires GROUP SU(2)"));
        let mut buf = DenseLinkBuffer::new_identity(Group::SU2, 1).unwrap();
        buf.data.pop();
        registry::register(Arc::new(SU2GaugeField::from_buffer(
            "mg_fixture".into(),
            "supplied_generators".into(),
            buf,
            GaugeFieldInit::Identity,
            None,
        )));
        assert!(direct("mg_fixture", &[0], true, 256, 1e-8)
            .unwrap_err()
            .contains("malformed"));
        register(&[ID]);
        for (edges, cap, tol) in [
            (vec![], 256, 1e-8),
            (vec![0; 33], 256, 1e-8),
            (vec![0], 0, 1e-8),
            (vec![0], 257, 1e-8),
            (vec![0], 256, f64::NAN),
            (vec![0], 256, f64::INFINITY),
        ] {
            assert!(direct("mg_fixture", &edges, true, cap, tol)
                .unwrap_err()
                .contains("require 1..32"));
        }
    }
    #[test]
    fn mg05_inconsistent_buffer_metadata_refuses() {
        let _guard = registry::test_serial_lock();
        for wrong_group in [true, false] {
            let mut buf = DenseLinkBuffer::new_identity(Group::SU2, 1).unwrap();
            if wrong_group {
                buf.group = Group::U1;
            } else {
                buf.repr_dim = 8;
            }
            registry::register(Arc::new(SU2GaugeField::from_buffer(
                "mg_fixture".into(),
                "supplied_generators".into(),
                buf,
                GaugeFieldInit::Identity,
                None,
            )));
            let err =
                gigi::monodromy::execute_monodromy_group_order("mg_fixture", &[0], true, 256, 1e-8)
                    .unwrap_err();
            let expected = if wrong_group {
                "requires GROUP SU(2)"
            } else {
                "malformed"
            };
            assert!(err.contains(expected), "{err}");
        }
    }
    #[test]
    fn mg07_shared_dispatch_and_optional_commutator() {
        let _guard = registry::test_serial_lock();
        register(&[S, t()]);
        let dir = tempfile::tempdir().unwrap();
        let engine = RwLock::new(gigi::engine::Engine::open(dir.path()).unwrap());
        let stmt = parse("MONODROMY_GROUP_ORDER mg_fixture FROM EDGES (0,1);").unwrap();
        let a = execute(&mut engine.write().unwrap(), &stmt).unwrap();
        let b =
            gigi::halcyon_gql_dispatch::try_dispatch_topology_statement(&engine, &stmt).unwrap();
        if let (ExecResult::Rows(a), ExecResult::Rows(b)) = (a, b) {
            assert_eq!(a, b);
            assert_eq!(int(&a[0], "group_order"), 120);
            assert_eq!(a[0].get("is_perfect"), Some(&Value::Null));
        } else {
            panic!("dispatch did not execute")
        }
    }
}
