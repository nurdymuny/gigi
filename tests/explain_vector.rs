//! EXPLAIN SECTION … AT — vector κ rows (Marcella EXPLAIN-family ask 1).
//!
//! Definition under test (kappa_v, a NEW quantity, separate from the
//! scalar per-field kappa):
//!
//!     kappa_v = |1 − cos(v, mu_v)| / R_cos
//!
//!   - mu_v   = per-component mean vector of the field across the
//!              bundle (computed on demand from the record scan, NOT
//!              from insert-time FieldStats)
//!   - cos    = dot(v, mu) / sqrt(dot(v,v) · dot(mu,mu)) — cosine
//!              self-normalizes both operands; NO separate
//!              unit-normalization step is applied (kappa_v is
//!              direction-only by construction)
//!   - R_cos  = effective range of (1 − cos) observed across the
//!              bundle in the same EXPLAIN call: max − min, floored
//!              to f64::EPSILON (mirrors bundle::effective_range)
//!
//! INVARIANT DISCIPLINE: vector rows are ADDITIVE (kind='vector') and
//! are EXCLUDED from the mean(kappa) == record_kappa invariant —
//! record_kappa remains compute_record_k over scalar numeric fibers,
//! untouched. Every test that has scalar fibers re-asserts that.
//!
//! Two supported shapes:
//!   (a) true Value::Vector fiber fields → automatic kappa_v row;
//!   (b) scalar-family assembly via the explicit clause
//!       `VECTOR (v0..v383)` / `VECTOR (f1, f2, …)` (both forms).

use gigi::engine::Engine;
use gigi::parser::{self, ExecResult};
use gigi::types::{BundleSchema, EncryptionMode, FieldDef, FieldType, Record, Value};

fn run(e: &mut Engine, stmt: &str) -> Result<ExecResult, String> {
    let ast = parser::parse(stmt)?;
    parser::execute(e, &ast)
}

fn rows(e: &mut Engine, stmt: &str) -> Vec<Record> {
    match run(e, stmt).unwrap() {
        ExecResult::Rows(rows) => rows,
        other => panic!("expected Rows from `{stmt}`, got {other:?}"),
    }
}

fn vector_field_def(name: &str, dims: usize) -> FieldDef {
    FieldDef {
        name: name.to_string(),
        field_type: FieldType::Vector { dims },
        default: Value::Null,
        range: None,
        weight: 1.0,
        encryption: EncryptionMode::None,
        encryption_group: None,
    }
}

/// True-Vector fixture: base id, one Vector{2} fiber `emb`, no scalar
/// fibers. Records a=(1,0), b=(3,0), c=(0,1) — a ∥ b at 3× scale.
fn vector_engine() -> (tempfile::TempDir, Engine) {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    let mut schema = BundleSchema::new("vb");
    schema.base_fields.push(FieldDef::categorical("id"));
    schema.fiber_fields.push(vector_field_def("emb", 2));
    e.create_bundle(schema).unwrap();
    for (id, v) in [
        ("a", vec![1.0, 0.0]),
        ("b", vec![3.0, 0.0]),
        ("c", vec![0.0, 1.0]),
    ] {
        let mut rec = Record::new();
        rec.insert("id".into(), Value::Text(id.into()));
        rec.insert("emb".into(), Value::Vector(v));
        e.insert("vb", &rec).unwrap();
    }
    (dir, e)
}

/// Scalar-family fixture (the marcella_source_embeddings_bge_v2 shape
/// in miniature): v0, v1 scalar numeric fibers assembled by the
/// VECTOR clause. a=(2,0) ⊥ b=(0,1), deliberately asymmetric norms so
/// R_cos > 0.
fn family_engine() -> (tempfile::TempDir, Engine) {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    run(&mut e, "BUNDLE st BASE (id TEXT) FIBER (v0 NUMERIC, v1 NUMERIC);").unwrap();
    run(&mut e, "SECTION st (id='a', v0=2.0, v1=0.0);").unwrap();
    run(&mut e, "SECTION st (id='b', v0=0.0, v1=1.0);").unwrap();
    (dir, e)
}

fn find_vector_row<'a>(rows: &'a [Record], label: &str) -> &'a Record {
    rows.iter()
        .find(|r| {
            r.get("kind").and_then(|v| v.as_str()) == Some("vector")
                && r.get("field").and_then(|v| v.as_str()) == Some(label)
        })
        .unwrap_or_else(|| panic!("no vector row labeled '{label}' in {rows:?}"))
}

fn scalar_rows(rows: &[Record]) -> Vec<&Record> {
    rows.iter()
        .filter(|r| r.get("kind").and_then(|v| v.as_str()) != Some("vector"))
        .collect()
}

/// mean(kappa over SCALAR rows) == record_kappa — the pre-existing
/// invariant, re-asserted with vector rows present and excluded.
fn assert_scalar_invariant(all: &[Record]) {
    let scal = scalar_rows(all);
    assert!(!scal.is_empty(), "invariant needs scalar rows");
    let record_kappa = scal[0]["record_kappa"].as_f64().unwrap();
    let mean = scal
        .iter()
        .map(|r| r["kappa"].as_f64().unwrap())
        .sum::<f64>()
        / scal.len() as f64;
    assert!(
        (mean - record_kappa).abs() < 1e-9,
        "mean(scalar kappa) {mean} == record_kappa {record_kappa}"
    );
}

// ── (a) true Value::Vector fields: automatic row ────────────────────

#[test]
fn vector_field_gets_automatic_kappa_v_row() {
    // mu = ((1+3+0)/3, (0+0+1)/3) = (4/3, 1/3), ‖mu‖ = √17/3.
    // cos(a,mu) = cos(b,mu) = 4/√17  (a ∥ b — direction only)
    // cos(c,mu) = 1/√17
    // R_cos = (1 − 1/√17) − (1 − 4/√17) = 3/√17
    // kappa_v(a) = (1 − 4/√17)/(3/√17) = (√17 − 4)/3
    // kappa_v(c) = (√17 − 1)/3
    let (_d, mut e) = vector_engine();
    let s17 = 17f64.sqrt();

    let ra = rows(&mut e, "EXPLAIN SECTION vb AT id='a';");
    let va = find_vector_row(&ra, "emb");
    let ka = va["kappa"].as_f64().unwrap();
    assert!(
        (ka - (s17 - 4.0) / 3.0).abs() < 1e-9,
        "kappa_v(a) hand-computed (√17−4)/3: {ka}"
    );
    assert!(
        (va["cos"].as_f64().unwrap() - 4.0 / s17).abs() < 1e-9,
        "cos(a,mu) = 4/√17"
    );
    assert!(
        (va["r_cos"].as_f64().unwrap() - 3.0 / s17).abs() < 1e-9,
        "R_cos = 3/√17"
    );
    assert_eq!(va["dim"], Value::Integer(2));
    assert_eq!(va["n"], Value::Integer(3));
    // No scalar numeric fibers here: record_kappa (compute_record_k's
    // total over scalar fibers) is 0.0 — and kappa_v visibly does NOT
    // participate in it.
    assert_eq!(va["record_kappa"].as_f64().unwrap(), 0.0);

    let rc = rows(&mut e, "EXPLAIN SECTION vb AT id='c';");
    let vc = find_vector_row(&rc, "emb");
    let kc = vc["kappa"].as_f64().unwrap();
    assert!(
        (kc - (s17 - 1.0) / 3.0).abs() < 1e-9,
        "kappa_v(c) hand-computed (√17−1)/3: {kc}"
    );
}

#[test]
fn kappa_v_is_direction_only_scale_invariant() {
    // b = 3·a (same direction, 3× the norm). cos(v, mu) is invariant
    // in the scale of v by construction — NO pre-normalization step is
    // applied; the cosine denominator carries ‖v‖ — so a and b get the
    // SAME kappa_v against the same mu (up to f64 rounding).
    let (_d, mut e) = vector_engine();
    let ra = rows(&mut e, "EXPLAIN SECTION vb AT id='a';");
    let rb = rows(&mut e, "EXPLAIN SECTION vb AT id='b';");
    let ka = find_vector_row(&ra, "emb")["kappa"].as_f64().unwrap();
    let kb = find_vector_row(&rb, "emb")["kappa"].as_f64().unwrap();
    assert!(
        (ka - kb).abs() < 1e-12,
        "scaling a record's vector 10×/3× changes nothing but mu's \
         contribution: kappa_v(a) {ka} vs kappa_v(b) {kb}"
    );
}

#[test]
fn record_equal_to_mean_vector_has_kappa_v_zero_exactly() {
    // (1,0), (0,1), (0.5,0.5): mu = (0.5, 0.5) — exactly representable
    // and exactly equal to record m. cos(m,mu) = 1.0 EXACTLY via the
    // dot/sqrt(dot·dot) formulation (sqrt(x·x) == x in correctly-
    // rounded f64), so kappa_v = 0 / R_cos = 0.0 exactly.
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    let mut schema = BundleSchema::new("vz");
    schema.base_fields.push(FieldDef::categorical("id"));
    schema.fiber_fields.push(vector_field_def("emb", 2));
    e.create_bundle(schema).unwrap();
    for (id, v) in [
        ("p", vec![1.0, 0.0]),
        ("q", vec![0.0, 1.0]),
        ("m", vec![0.5, 0.5]),
    ] {
        let mut rec = Record::new();
        rec.insert("id".into(), Value::Text(id.into()));
        rec.insert("emb".into(), Value::Vector(v));
        e.insert("vz", &rec).unwrap();
    }
    let rm = rows(&mut e, "EXPLAIN SECTION vz AT id='m';");
    let vm = find_vector_row(&rm, "emb");
    assert_eq!(
        vm["kappa"].as_f64().unwrap(),
        0.0,
        "record == bundle mean vector → kappa_v exactly 0.0"
    );
    assert_eq!(vm["cos"].as_f64().unwrap(), 1.0, "cos(mu,mu) exactly 1.0");
}

// ── (b) scalar-family assembly: the VECTOR (…) clause ───────────────

#[test]
fn vector_clause_assembles_scalar_family_one_kappa_v_row() {
    // a=(2,0), b=(0,1): mu=(1, 0.5), ‖mu‖²=1.25.
    // cos(a,mu) = 2/√5, cos(b,mu) = 1/√5, R_cos = 1/√5.
    // kappa_v(a) = √5−2, kappa_v(b) = √5−1.
    let (_d, mut e) = family_engine();
    let s5 = 5f64.sqrt();

    let ra = rows(&mut e, "EXPLAIN SECTION st AT id='a' VECTOR (v0..v1);");
    let va = find_vector_row(&ra, "vector(v0..v1)");
    let ka = va["kappa"].as_f64().unwrap();
    assert!(
        (ka - (s5 - 2.0)).abs() < 1e-9,
        "kappa_v(a) hand-computed √5−2: {ka}"
    );
    assert!((va["cos"].as_f64().unwrap() - 2.0 / s5).abs() < 1e-9);
    assert_eq!(va["n"], Value::Integer(2));
    assert_eq!(va["dim"], Value::Integer(2));

    let rb = rows(&mut e, "EXPLAIN SECTION st AT id='b' VECTOR (v0..v1);");
    let vb = find_vector_row(&rb, "vector(v0..v1)");
    let kb = vb["kappa"].as_f64().unwrap();
    assert!(
        (kb - (s5 - 1.0)).abs() < 1e-9,
        "kappa_v(b) hand-computed √5−1: {kb}"
    );
}

#[test]
fn vector_rows_are_additive_and_excluded_from_record_kappa_invariant() {
    // Scalar rows and record_kappa are UNCHANGED by the VECTOR clause:
    //   v0 over {2,0}: mean 1, range 2 → kappa(a.v0) = 0.5
    //   v1 over {0,1}: mean 0.5, range 1 → kappa(a.v1) = 0.5
    //   record_kappa = 0.5; kappa_v(a) = √5−2 ≈ 0.236 plays NO part.
    let (_d, mut e) = family_engine();

    let plain = rows(&mut e, "EXPLAIN SECTION st AT id='a';");
    let with_vec = rows(&mut e, "EXPLAIN SECTION st AT id='a' VECTOR (v0, v1);");

    // The clause adds exactly one row…
    assert_eq!(with_vec.len(), plain.len() + 1);
    // …and the scalar rows are byte-for-byte the same ones.
    let scal: Vec<&Record> = scalar_rows(&with_vec);
    assert_eq!(scal.len(), plain.len());
    for (a, b) in plain.iter().zip(scal.iter()) {
        assert_eq!(&a, b, "scalar rows unchanged by the VECTOR clause");
    }
    // Invariant over scalar rows only.
    assert_scalar_invariant(&with_vec);
    let record_kappa = scal[0]["record_kappa"].as_f64().unwrap();
    assert!((record_kappa - 0.5).abs() < 1e-12);
    // The vector row carries the same record_kappa stamp (it rides the
    // same record) but its kappa is a different, separately-defined
    // quantity — including it would break the invariant, which is
    // exactly why consumers must filter kind='vector'.
    let v = find_vector_row(&with_vec, "vector(v0,v1)");
    assert_eq!(v["record_kappa"].as_f64().unwrap(), record_kappa);
    let kv = v["kappa"].as_f64().unwrap();
    assert!((kv - (5f64.sqrt() - 2.0)).abs() < 1e-9);
    assert!(
        (kv - record_kappa).abs() > 0.1,
        "fixture chosen so kappa_v visibly differs from record_kappa"
    );
}

#[test]
fn range_sugar_and_explicit_list_agree() {
    let (_d, mut e) = family_engine();
    let sugar = rows(&mut e, "EXPLAIN SECTION st AT id='a' VECTOR (v0..v1);");
    let listed = rows(&mut e, "EXPLAIN SECTION st AT id='a' VECTOR (v0, v1);");
    let ks = find_vector_row(&sugar, "vector(v0..v1)")["kappa"]
        .as_f64()
        .unwrap();
    let kl = find_vector_row(&listed, "vector(v0,v1)")["kappa"]
        .as_f64()
        .unwrap();
    assert_eq!(ks, kl, "v0..v1 is pure sugar for v0, v1");
}

#[test]
fn vector_clause_unknown_field_is_loud() {
    let (_d, mut e) = family_engine();
    let err = run(&mut e, "EXPLAIN SECTION st AT id='a' VECTOR (v0, nope);").unwrap_err();
    assert!(
        err.contains("nope"),
        "unknown VECTOR field must be named: {err}"
    );
}

#[test]
fn vector_clause_miss_still_typed_not_found() {
    let (_d, mut e) = family_engine();
    let err = run(
        &mut e,
        "EXPLAIN SECTION st AT id='ghost' VECTOR (v0..v1);",
    )
    .unwrap_err();
    assert!(err.starts_with("NOT_FOUND: "), "{err}");
    assert!(err.contains("id='ghost'"), "{err}");
}
