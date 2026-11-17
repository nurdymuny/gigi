//! RF2A: independently evaluated local Levi-Civita / Yang-Mills correspondence.
//! This is not the lattice force or a fixed-background trajectory equivalence.
use crate::parser::ExecResult;
use crate::types::{Record, Value};
type Triple = [f64;3];

pub(crate) fn connection(g: Triple) -> Triple {
    std::array::from_fn(|i| {
        let (j,k)=((i+1)%3,(i+2)%3);
        (g[j]+g[k]-g[i])/(g[j]*g[k]).sqrt()
    })
}

fn map_derivative(g: Triple, v: Triple, x: Triple) -> Triple {
    std::array::from_fn(|i| {
        let (j,k)=((i+1)%3,(i+2)%3);
        (v[j]+v[k]-v[i])/(g[j]*g[k]).sqrt()
            - 0.5*x[i]*(v[j]/g[j]+v[k]/g[k])
    })
}

fn curvature(x: Triple) -> Triple {
    std::array::from_fn(|i| x[(i+1)%3]*x[(i+2)%3] - 2.*x[i])
}

fn energy_density(g: Triple,x: Triple) -> f64 {
    let f=curvature(x);
    (0..3).map(|i| 0.5*f[i]*f[i]/(g[(i+1)%3]*g[(i+2)%3])).sum()
}

// Derived from E_g and the L2 mass matrix. No Ricci operator or map derivative here.
fn yang_mills(g: Triple,x: Triple) -> Triple {
    let f=curvature(x);
    std::array::from_fn(|i| {
        let (j,k)=((i+1)%3,(i+2)%3);
        let gradient = -2.*f[i]/(g[j]*g[k])
            + x[k]*f[j]/(g[k]*g[i]) + x[j]*f[k]/(g[i]*g[j]);
        -g[i] * gradient
    })
}

fn norm(g: Triple,v: Triple) -> f64 {
    (0..3).map(|i| v[i]*v[i]/g[i]).sum::<f64>().sqrt()
}

fn compare(g: Triple,p: Triple,w: Triple) -> (Triple,f64,f64) {
    let residual: Triple = std::array::from_fn(|i| p[i]-w[i]);
    let absolute = norm(g,residual);
    let denominator = norm(g,p).max(norm(g,w));
    (residual,absolute,if denominator==0. {0.} else {absolute/denominator})
}

pub fn execute_ricci_wilson_audit(g: Triple,normalized: bool) -> Result<ExecResult,String> {
    if g.iter().any(|x| !x.is_finite() || !(1e-6..=1e6).contains(x)) {
        return Err("RICCI_WILSON_AUDIT: METRIC requires finite squared lengths in [1e-6,1e6]; rescale units".into());
    }
    let lo=g.iter().copied().fold(f64::INFINITY,f64::min);
    let hi=g.iter().copied().fold(f64::NEG_INFINITY,f64::max);
    if hi/lo>1e4 {return Err("RICCI_WILSON_AUDIT: METRIC max/min exceeds 10000; this numerical policy does not cover that anisotropy".into());}
    let x=connection(g);
    let v=crate::ricci_flow::rhs(g,normalized,0.)?;
    let p=map_derivative(g,v,x);
    let w=yang_mills(g,x);
    let (residual,absolute,relative)=compare(g,p,w);
    let mut row=Record::new();
    for (key,value) in [("metric",g),("connection",x),("connection_curvature",curvature(x)),
        ("ricci_velocity",v),("mapped_ricci_velocity",p),("yang_mills_velocity",w),("residual",residual)] {
        if value.iter().any(|x|!x.is_finite()) {return Err(format!("RICCI_WILSON_AUDIT: nonfinite {key}; rescale METRIC"));}
        row.insert(key.into(),Value::Vector(value.to_vec()));
    }
    for (key,value) in [("absolute_residual",absolute),("relative_residual",relative),
        ("mapped_velocity_norm",norm(g,p)),("yang_mills_velocity_norm",norm(g,w)),
        ("energy_density",energy_density(g,x)),("time_scale",1.)] {
        if !value.is_finite() {return Err(format!("RICCI_WILSON_AUDIT: nonfinite {key}; rescale METRIC"));}
        row.insert(key.into(),Value::Float(value));
    }
    for (key,value) in [("scope","homogeneous_levi_civita_instantaneous"),
        ("mapping","levi_civita_positive_orthonormal_frame"),
        ("gradient","negative_YM_with_instantaneous_g_held_fixed"),
        ("numerical_policy","ricci_wilson_v1"),
        ("algebra_convention","so3_J_cross_bracket_plus"),
        ("norm_convention","L2_per_unit_volume"),
        ("manifold_model","SU2_left_invariant_diagonal_cover")] {
        row.insert(key.into(),Value::Text(value.into()));
    }
    for (key,value) in [("normalized",normalized),("time_scale_fitted",false),
        ("lattice_correspondence_tested",false),("fixed_background_equivalence_tested",false),
        ("topology_inferred",false)] {row.insert(key.into(),Value::Bool(value));}
    Ok(ExecResult::Rows(vec![row]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cw02_arbitrary_connection_gradient() {
        let (g,x)=([1.,2.,3.],[0.2,-0.4,0.7]);
        let w=yang_mills(g,x);
        for (a,b) in w.iter().zip([-0.742,1.708,-4.764]) {assert!((a-b).abs()<1e-12);}
        for i in 0..3 {
            let (mut plus,mut minus)=(x,x);
            plus[i]+=1e-6; minus[i]-=1e-6;
            let derivative=(energy_density(g,plus)-energy_density(g,minus))/2e-6;
            assert!((derivative+w[i]/g[i]).abs()<1e-9);
        }
        let advanced=std::array::from_fn(|i|x[i]+1e-6*w[i]);
        assert!(energy_density(g,advanced)<energy_density(g,x));
    }
    #[test]
    fn cw07_residual_cannot_be_invented_zero() {
        let (r,a,b)=compare([1.;3],[1.,2.,3.],[0.,2.,1.]);
        assert_eq!(r,[1.,0.,2.]);
        assert!((a-5f64.sqrt()).abs()<1e-14);
        assert!((b-(5f64/14.).sqrt()).abs()<1e-14);
    }
    #[test]
    fn cw00_nonfinite_refuses() {
        for bad in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY] {
            assert!(execute_ricci_wilson_audit([bad,1.,1.],false).is_err());
        }
    }
}
