use helike_quantum::enrichment::{Lift,Block};
use helike_quantum::model::Background;
use helike_quantum::operator::{ConeOperator,Mutation,Part};

fn norm(v: &[f64]) -> f64 { v.iter().map(|x| x*x).sum::<f64>().sqrt() }
fn error(a: &[f64],b: &[f64]) -> f64 { norm(&a.iter().zip(b).map(|(x,y)|x-y).collect::<Vec<_>>()) }
fn canonical() -> Lift { Lift::new(vec![0],vec![vec![0.,1.,0.],vec![0.,0.,1.]]).unwrap() }

#[test]
fn lift_retains_old_space_and_is_isometric_even_when_positions_reorder() {
    let l=Lift::new(vec![2,0],vec![vec![0.,1.,0.,0.],vec![0.,0.,0.,1.]]).unwrap();
    let x=[2.,3.,4.,5.];
    let v=l.expand(&x).unwrap();
    assert_eq!(v,vec![3.,4.,2.,5.]);
    assert_eq!(l.compress(&v).unwrap(),x);
    assert_eq!(norm(&v),norm(&x));
}

#[test]
fn analytic_block_includes_both_couplings_and_adjoint() {
    // Ambient H=[[2,3,4],[3,5,6],[4,6,9]], with the first coordinate retained as the old space.
    let b=Block::new(canonical(),vec![vec![3.,5.,6.],vec![4.,6.,9.]]).unwrap();
    assert_eq!(b.apply(&[1.,2.,3.],&[2.]).unwrap(),vec![20.,31.,43.]);
    assert_eq!(b.apply(&[1.,0.,0.],&[2.]).unwrap(),vec![2.,3.,4.]);
}

#[test]
fn residual_whitening_normalizes_and_rotation_preserves_projector() {
    let r=vec![vec![0.,2.,0.,0.],vec![0.,1.,3.,0.]];
    let q=Lift::from_residuals(vec![0],r.clone()).unwrap();
    let s=0.5f64.sqrt();
    let rot=vec![(0..4).map(|i|s*(r[0][i]+r[1][i])).collect(),(0..4).map(|i|s*(r[0][i]-r[1][i])).collect()];
    let z=Lift::from_residuals(vec![0],rot).unwrap();
    for v in [vec![0.2,0.4,0.6,0.8],vec![1.,2.,3.,4.]] {
        let a=q.expand(&q.compress(&v).unwrap()).unwrap();
        let b=z.expand(&z.compress(&v).unwrap()).unwrap();
        assert!(error(&a,&b)<1e-12);
    }
    for c in &q.columns { assert!((norm(c)-1.).abs()<1e-12); }
}

#[test]
fn projection_does_not_confuse_in_space_and_complement() {
    let l=Lift::new(vec![0],vec![vec![0.,1.,0.]]).unwrap();
    let r=[2.,3.,4.];
    let p=l.expand(&l.compress(&r).unwrap()).unwrap();
    assert_eq!(p,vec![2.,3.,0.]);
    assert_eq!(error(&r,&p),4.);
}

#[test]
fn bad_directions_and_asymmetric_block_refuse() {
    assert!(Lift::new(vec![0,0],vec![vec![0.,1.]]).is_err());
    assert!(Lift::new(vec![0],vec![vec![1.,0.]]).is_err());
    assert!(Lift::new(vec![0],vec![vec![0.,2.]]).is_err());
    assert!(Lift::new(vec![0],vec![vec![0.,f64::NAN]]).is_err());
    assert!(Lift::from_residuals(vec![0],vec![vec![0.,1.],vec![0.,2.]]).is_err());
    assert!(Block::new(canonical(),vec![vec![3.,5.,7.],vec![4.,6.,9.]]).is_err());
    assert!(canonical().expand(&[1.]).is_err());
}

#[test]
fn zero_coupling_does_not_change_old_action() {
    let b=Block::new(canonical(),vec![vec![0.,5.,0.],vec![0.,0.,9.]]).unwrap();
    assert_eq!(b.apply(&[2.,0.,0.],&[4.]).unwrap(),vec![4.,0.,0.]);
}

#[test]
fn both_controls_match_direct_projected_operator() {
    for g2 in [0.002f64,0.004] {
        let bg=Background::maurer_cartan(1.15,3.,g2.sqrt()).unwrap();
        let low=ConeOperator::exact(bg.clone(),2,Mutation::None).unwrap();
        let high=ConeOperator::exact(bg,4,Mutation::None).unwrap();
        let index=helike_quantum::basis::index_of(&high.basis);
        let positions:Vec<usize>=low.basis.iter().map(|n|index[n]).collect();
        let u:Vec<Vec<f64>>=(0..4).map(|j| {let mut v=vec![0.;high.dim()];v[low.dim()+j]=1.;v}).collect();
        let hu=high.apply(&u,Part::Full);
        let b=Block::new(Lift::new(positions,u).unwrap(),hu).unwrap();
        for seed in 1..=3 {
            let x:Vec<f64>=(0..low.dim()+4).map(|i|((i+seed) as f64).sin()).collect();
            let direct=high.apply(&[b.lift.expand(&x).unwrap()],Part::Full);
            let expected=b.lift.compress(&direct[0]).unwrap();
            let hx=low.apply(&[x[..low.dim()].to_vec()],Part::Full);
            let got=b.apply(&x,&hx[0]).unwrap();
            assert!(error(&got,&expected)<1e-10*norm(&expected).max(1.));
        }
    }
}
