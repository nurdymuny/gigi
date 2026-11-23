use gigi::dials::{sphere_parallel_transport as transport, sphere_parallel_transport_records};
use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, FieldType, Record, Value};

type V = [f64; 3];
type M = [[f64; 3]; 3];
const X: V = [1., 0., 0.];
const Y: V = [0., 1., 0.];
const Z: V = [0., 0., 1.];
const I: M = [X, Y, Z];

fn dot(a: V, b: V) -> f64 { a.iter().zip(b).map(|(x,y)| x*y).sum() }
fn norm(a: V) -> f64 { dot(a,a).sqrt() }
fn unit(a: V) -> V { let n=norm(a); a.map(|x| x/n) }
fn apply(a: M, x: V) -> V { a.map(|row| dot(row,x)) }
fn mul(a: M, b: M) -> M {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k|a[i][k]*b[k][j]).sum()))
}
fn close(a: V, b: V) {
    for i in 0..3 { assert!((a[i]-b[i]).abs() < 2e-10, "{a:?} != {b:?}"); }
}
fn matrix_close(a: M, b: M) { for i in 0..3 { close(a[i],b[i]); } }
fn oracle(p: V, q: V, v: V) -> V {
    let scale=dot(v,q)/(1.+dot(p,q));
    std::array::from_fn(|i| v[i]-scale*(p[i]+q[i]))
}
fn error(p: &[f64], q: &[f64], v: &[f64], text: &str) {
    let e=transport(p,q,v).unwrap_err(); assert!(e.contains(text), "{e}");
}

#[test]
fn sphere_quarter_loop_basis_positions_reverse_and_backtrack() {
    let expected=[[1.,0.,0.],[0.,0.,-1.],[0.,1.,0.]];
    let mut h=I;
    for (p,q) in [(X,Y),(Y,Z),(Z,X)] {
        let tangent=if p==Y {X} else {Y};
        let r=transport(&p,&q,&tangent).unwrap();
        close(apply(r.matrix,p),q);
        h=mul(r.matrix,h);
        assert!(r.endpoint_residual<1e-10 && r.orthogonality_residual<1e-10);
    }
    matrix_close(h,expected); close(apply(h,Y),Z); close(apply(h,Z),[0.,-1.,0.]);
    let mut reverse=I;
    for (p,q,v) in [(X,Z,Y),(Z,Y,X),(Y,X,Z)] { reverse=mul(transport(&p,&q,&v).unwrap().matrix,reverse); }
    matrix_close(mul(reverse,h),I); close(apply(reverse,Y),[0.,0.,-1.]);
    let a=transport(&X,&Y,&Y).unwrap();
    let b=transport(&Y,&X,&a.transported).unwrap(); close(b.transported,Y);
    matrix_close(mul(b.matrix,a.matrix),I);
    let mut repeated=I; for _ in 0..21 { repeated=mul(h,repeated); }
    matrix_close(repeated,expected);
}

#[test]
fn sphere_nonaxis_oracle_inverse_and_frame_covariance() {
    let frame=[[0.,0.,1.],[1.,0.,0.],[0.,1.,0.]];
    for i in 1..49 {
        let t=i as f64 * 0.137;
        let p=unit([t.cos(), t.sin(), 0.37]);
        let q=unit([(t+0.8).cos(), (t+0.8).sin(), -0.21]);
        let v=unit([-p[1],p[0],0.]);
        let r=transport(&p,&q,&v).unwrap(); close(r.transported,oracle(p,q,v));
        close(transport(&q,&p,&r.transported).unwrap().transported,v);
        assert!(dot(r.transported,q).abs()<1e-10);
        assert!((norm(r.transported)-1.).abs()<1e-10);
        let s=transport(&apply(frame,p),&apply(frame,q),&apply(frame,v)).unwrap();
        close(s.transported,apply(frame,r.transported));
        matrix_close(mul(s.matrix,frame),mul(frame,r.matrix));
    }
}

#[test]
fn sphere_refuses_dimensions_nonfinite_and_magnitude() {
    error(&[1.,0.],&Y,&Y,"from must have exactly 3");
    error(&X,&[0.,1.],&Y,"to must have exactly 3");
    error(&X,&Y,&[0.,1.],"tangent must have exactly 3");
    error(&[f64::NAN,0.,0.],&Y,&Y,"from component 0");
    error(&X,&[0.,f64::INFINITY,0.],&Y,"to component 1");
    error(&X,&Y,&[0.,f64::NEG_INFINITY,0.],"tangent component 1");
    error(&X,&Y,&[0.,1e101,0.],"tangent norm");
}

#[test]
fn sphere_refuses_nonunit_nontangent_and_antipodal() {
    error(&[0.;3],&Y,&Y,"from norm");
    error(&X,&[0.,2.,0.],&Y,"to norm");
    error(&[1.+2e-12,0.,0.],&Y,&Y,"from norm");
    error(&X,&Y,&[1e-6,1.,0.],"tangent is not tangent");
    error(&X,&Y,&[1e-11,1.,0.],"tangent is not tangent");
    error(&X,&Y,&[1e-30,0.,0.],"tangent is not tangent");
    error(&X,&[-1.,0.,0.],&Y,"antipodal");
    error(&X,&unit([-1.,1e-5,0.]),&Y,"antipodal");
    let q=unit([-1.,1e-3,0.]); close(transport(&X,&q,&Y).unwrap().transported,oracle(X,q,Y));
}

#[test]
fn sphere_identity_small_angle_roundoff_normalization_and_zero() {
    let r=transport(&X,&X,&Y).unwrap(); matrix_close(r.matrix,I); close(r.transported,Y);
    close(transport(&X,&Y,&[0.;3]).unwrap().transported,[0.;3]);
    let p=[1.+5e-13,0.,0.]; let original=p;
    let r=transport(&p,&Y,&Y).unwrap(); assert_eq!(r.from_unit,X);
    assert_eq!(r.endpoint_input_norms[0],p[0]); assert_eq!(p,original);
    let q=unit([1.,1e-13,0.]); close(transport(&X,&q,&Y).unwrap().transported,oracle(X,q,Y));
    assert_eq!(r.numerical_policy,1);
}

fn record(id: &str, p: V) -> Record {
    [("id".into(),Value::Text(id.into())),("x".into(),Value::Float(p[0])),
     ("y".into(),Value::Float(p[1])),("z".into(),Value::Float(p[2]))].into_iter().collect()
}

#[test]
fn sphere_record_adapter_never_invents_components() {
    let a=record("a",X); let b=record("b",Y);
    close(sphere_parallel_transport_records(&a,&b,&["x","y","z"],&Y).unwrap().transported,[-1.,0.,0.]);
    let mut missing=a.clone(); missing.remove("z");
    assert!(sphere_parallel_transport_records(&missing,&b,&["x","y","z"],&Y).unwrap_err().contains("from field 'z'"));
    for value in [Value::Null,Value::Text("0".into()),Value::Bool(false),Value::Timestamp(0)] {
        let mut invalid=b.clone(); invalid.insert("z".into(),value);
        assert!(sphere_parallel_transport_records(&a,&invalid,&["x","y","z"],&Y).unwrap_err().contains("to field 'z'"));
    }
    assert!(sphere_parallel_transport_records(&a,&b,&["x","y"],&Y).unwrap_err().contains("three distinct"));
    assert!(sphere_parallel_transport_records(&a,&b,&["x","x","z"],&Y).unwrap_err().contains("three distinct"));
    let mut bad=b.clone(); bad.insert("z".into(),Value::Float(f64::NAN));
    assert!(sphere_parallel_transport_records(&a,&bad,&["x","y","z"],&Y).unwrap_err().contains("to field 'z'"));
}

#[test]
fn sphere_legacy_antipodal_and_small_angle_behavior_unchanged() {
    assert_eq!(gigi::dials::transport_rotation_matrix(&X,&[-1.,0.,0.],std::f64::consts::PI), I.concat());
    assert_eq!(gigi::dials::transport_rotation_matrix(&X,&Y,1e-14), I.concat());
}

#[test]
fn sphere_embedded_gigi_showcase_persists_derived_transport_and_reopens() {
    let dir=tempfile::tempdir().unwrap();
    let mut engine=Engine::open(dir.path()).unwrap();
    let mut schema=BundleSchema::new("sphere_positions");
    schema.base_fields=vec![FieldDef::categorical("id")];
    schema.fiber_fields=vec![FieldDef::numeric("x"),FieldDef::numeric("y"),FieldDef::numeric("z")];
    engine.create_bundle(schema).unwrap();
    let mut results=BundleSchema::new("sphere_results");
    results.base_fields=vec![FieldDef::categorical("id")];
    results.fiber_fields=vec![FieldDef {field_type:FieldType::Vector{dims:3},..FieldDef::numeric("v")}];
    engine.create_bundle(results).unwrap();
    let mut expected=Vec::new();
    for (name,points) in [("octant",vec![X,Y,Z,X]),("oblique",vec![unit([1.,2.,3.]),unit([-2.,3.,1.]),unit([3.,1.,-2.]),unit([1.,2.,3.])])] {
        for (i,p) in points.iter().enumerate() {engine.insert("sphere_positions",&record(&format!("{name}_{i}"),*p)).unwrap();}
        let p=points[0]; let a=unit([-p[1],p[0],0.]);
        let b=[p[1]*a[2]-p[2]*a[1],p[2]*a[0]-p[0]*a[2],p[0]*a[1]-p[1]*a[0]];
        for (basis,initial) in [a,b].into_iter().enumerate() {
            let mut actual=initial; let mut independent=initial;
            for i in 0..points.len()-1 {
                let get=|j| engine.point_query("sphere_positions",&[("id".into(),Value::Text(format!("{name}_{j}")))].into_iter().collect()).unwrap().unwrap();
                actual=sphere_parallel_transport_records(&get(i),&get(i+1),&["x","y","z"],&actual).unwrap().transported;
                independent=oracle(points[i],points[i+1],independent);
            }
            close(actual,independent);
            if name=="octant" {close(actual,if basis==0 {Z} else {[0.,-1.,0.]});}
            let id=format!("{name}_{basis}");
            engine.insert("sphere_results",&[("id".into(),Value::Text(id.clone())),("v".into(),Value::Vector(actual.to_vec()))].into_iter().collect()).unwrap();
            println!("{id}: actual={actual:?} independent={independent:?}");
            expected.push((id,actual));
        }
    }
    engine.checkpoint().unwrap(); drop(engine);
    let engine=Engine::open(dir.path()).unwrap();
    for (id,v) in expected {
        let row=engine.point_query("sphere_results",&[("id".into(),Value::Text(id))].into_iter().collect()).unwrap().unwrap();
        assert_eq!(row.get("v"),Some(&Value::Vector(v.to_vec())));
    }
}
