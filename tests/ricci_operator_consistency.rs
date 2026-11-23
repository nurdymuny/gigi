#![cfg(feature = "gauge")]
use gigi::gauge::ricci_simplicial::{adjoint, cell_action, cell_tangent_mass};
use gigi::gauge::ricci_whitney::tetra_mass;
use nalgebra::{SMatrix, SVector};

fn link(angle: f64, axis: usize) -> [f64; 4] {
    let mut q = [angle.cos(), 0., 0., 0.];
    q[axis + 1] = angle.sin();
    q
}
fn points() -> [[f64; 4]; 4] {
    let mut p = [[1.,0.,0.,0.],[1.,0.2,0.,0.],[1.,0.,0.2,0.],[1.,0.,0.,0.2]];
    for v in &mut p { let n=v.iter().map(|x| x*x).sum::<f64>().sqrt(); for x in v { *x/=n; } }
    p
}

#[test]
fn oc02_center_holonomy_not_erased_by_equal_adjoint() {
    let id=[1.,0.,0.,0.];let center=[-1.,0.,0.,0.];
    assert!((adjoint(id)-adjoint(center)).norm()<1e-15);
    let mut u=[id;6];let mass=SMatrix::<f64,4,4>::identity();
    assert_eq!(cell_action(u,mass).unwrap().energy,0.);
    u[0]=center;
    assert!((cell_action(u,mass).unwrap().energy-16.).abs()<1e-14);
}

#[test]
fn oc02_vertex_center_gauge_preserves_energy_gradient_and_mass() {
    let u=std::array::from_fn(|e| link(0.21+e as f64*0.39,e%3));
    let pairs=[(0,1),(0,2),(0,3),(1,2),(1,3),(2,3)];let signs=[-1.,1.,-1.,1.];
    let changed=std::array::from_fn(|e| u[e].map(|x|x*signs[pairs[e].0]*signs[pairs[e].1]));
    let m=tetra_mass(points(),[3.,1.,1.],6).unwrap();
    let a=cell_action(u,m.face_mass).unwrap();let b=cell_action(changed,m.face_mass).unwrap();
    assert!((a.energy-b.energy).abs()<1e-12);
    assert!((a.gradient-b.gradient).norm()<1e-12);
    assert!((cell_tangent_mass(u,m.edge_mass).unwrap()-cell_tangent_mass(changed,m.edge_mass).unwrap()).norm()<1e-12);
}

#[test]
fn oc02_smooth_signed_path_and_canonicalization_mutant() {
    let mut u=[[1.,0.,0.,0.];6];u[3]=link(0.31,0);
    let mass=SMatrix::<f64,4,4>::identity();let h=1e-6;
    for alpha in [std::f64::consts::FRAC_PI_2,std::f64::consts::PI] {
        let mut values=Vec::new();
        for side in [-1.,1.] {u[0]=link(alpha+side*h,0);values.push(cell_action(u,mass).unwrap().energy);}
        assert!((values[1]-values[0]).abs()<1e-3);
        u[0]=link(alpha,0);let derivative=cell_action(u,mass).unwrap().gradient[0];
        assert!(((values[1]-values[0])/(2.*h)-derivative).abs()<2e-8);
    }
    let mut folded=Vec::new();
    for side in [-1.,1.] {
        u[0]=link(std::f64::consts::FRAC_PI_2+side*h,0);
        if u[0][0]<0. {u[0]=u[0].map(|x|-x);}
        folded.push(cell_action(u,mass).unwrap().energy);
    }
    assert!((folded[1]-folded[0]).abs()>1.,"sign erasure must be detectable");
}

#[test]
fn oc03_native_small_amplitude_hessian_is_whitney_stiffness() {
    let m=tetra_mass(points(),[1.;3],6).unwrap();
    let d=SMatrix::<f64,4,6>::from_row_slice(&[
        1.,-1.,0.,1.,0.,0., 1.,0.,-1.,0.,1.,0.,
        0.,1.,-1.,0.,0.,1., 0.,0.,0.,1.,-1.,1.]);
    let a=SVector::<f64,6>::from_row_slice(&[0.2,-0.3,0.1,0.4,-0.2,0.7]);
    let stiffness=d.transpose()*m.face_mass*d;let exact=-2.*stiffness*a;
    let mut errors=Vec::new();
    for eps in [0.02,0.01,0.005] {
        let links=std::array::from_fn(|e|link(-0.5*eps*a[e],0));
        let result=cell_action(links,m.face_mass).unwrap();
        let measured=SVector::<f64,6>::from_fn(|e,_|result.gradient[3*e]/eps);
        errors.push((measured-exact).norm());
        assert!((result.energy/(eps*eps)-0.5*a.dot(&(stiffness*a))).abs()<0.002);
        let tangent=cell_tangent_mass(links,m.edge_mass).unwrap();
        for e in 0..6 {for f in 0..6 {assert!((tangent[(3*e,3*f)]-4.*m.edge_mass[(e,f)]).abs()<1e-14);}}
    }
    for pair in errors.windows(2) {assert!(pair[0]/pair[1]>3.9 && pair[0]/pair[1]<4.1);}
    println!("OC03 normalized gradient errors: {:?}",errors);
}

#[test]
fn oc02_simultaneous_base_antipodes_are_not_link_sign_erasure() {
    use gigi::gauge::ricci_link::levi_civita_link;
    let p=points();
    let a=levi_civita_link([3.,1.,1.],p[0],p[1]).unwrap();
    let b=levi_civita_link([3.,1.,1.],p[0].map(|x|-x),p[1].map(|x|-x)).unwrap();
    assert_eq!(a,b);
}

#[cfg(feature = "kahler")]
#[test]
fn oc08_hhe_normalized_relaxation_polynomial_and_conjugate() {
    use gigi::geometry::harmonic_conjugate::{recover,RealTerm};
    let terms=[RealTerm{x_power:0,y_power:0,coefficient:0.5},
        RealTerm{x_power:1,y_power:0,coefficient:-0.5}];
    let r=recover(&terms,0.,1.+1e-10,&[[0.2,0.3]]).unwrap();
    assert_eq!(r.status,"bounded_holomorphic");
    let p=r.representation.as_ref().unwrap();
    assert_eq!(p.coefficients,vec![[0.5,0.],[-0.5,0.]]);
    assert!((r.queries[0].value[0]-0.4).abs()<1e-15);
    assert!((r.queries[0].value[1]+0.15).abs()<1e-15);
    assert_eq!(r.queries[0].first_derivative,[-0.5,0.]);
    assert_eq!(r.queries[0].second_derivative,[0.,0.]);
    println!("OC08_HHE {}",serde_json::to_string(&r).unwrap());
}
