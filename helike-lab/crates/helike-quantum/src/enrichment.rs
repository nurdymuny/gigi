//! Nested residual enrichment. The ambient probe is finite and is not a domain certificate.
use faer::{Mat, Side};
use std::collections::HashSet;
use crate::{basis, lanczos, model::Background, operator::{ConeOperator,Mutation,Part}, ritz::{self,SolveSettings}, support};
use serde::{Deserialize,Serialize};

fn dot(a: &[f64], b: &[f64]) -> f64 { a.iter().zip(b).map(|(x,y)| x*y).sum() }
fn norm(a: &[f64]) -> f64 { dot(a,a).sqrt() }
fn difference(a: &[f64], b: &[f64]) -> f64 { norm(&a.iter().zip(b).map(|(x,y)| x-y).collect::<Vec<_>>()) }
fn finite(v: &[f64], n: usize) -> bool { v.len()==n && v.iter().all(|x| x.is_finite()) }

pub struct Lift { pub positions: Vec<usize>, pub columns: Vec<Vec<f64>>, pub ambient: usize }
pub struct Block { pub lift: Lift, pub c: Vec<Vec<f64>>, pub d: Vec<Vec<f64>> }
impl Lift {
    fn check(p: &[usize], u: &[Vec<f64>]) -> Result<usize,String> {
        let n=u.first().map_or(0,Vec::len);
        if p.is_empty() || u.is_empty() || n==0 || p.iter().collect::<HashSet<_>>().len()!=p.len()
            || p.iter().any(|i| *i>=n) || u.iter().any(|v| !finite(v,n) || p.iter().any(|i| v[*i]!=0.0)) {
            return Err("invalid dimensions, old-space contamination or nonfinite directions".into());
        }
        Ok(n)
    }
    pub fn new(p: Vec<usize>, u: Vec<Vec<f64>>) -> Result<Self,String> {
        let ambient=Self::check(&p,&u)?;
        for i in 0..u.len() { for j in 0..u.len() {
            let e=(dot(&u[i],&u[j])-if i==j {1.0} else {0.0}).abs();
            if !e.is_finite() || e>1e-10 { return Err("directions are not orthonormal".into()); }
        }}
        Ok(Self{positions:p,columns:u,ambient})
    }
    pub fn from_residuals(p: Vec<usize>, r: Vec<Vec<f64>>) -> Result<Self,String> {
        let n=Self::check(&p,&r)?;
        let k=r.len();
        let g=Mat::from_fn(k,k,|i,j| dot(&r[i],&r[j]));
        if (0..k).any(|i| (0..k).any(|j| !g[(i,j)].is_finite())) { return Err("overflowed Gram matrix".into()); }
        let eig=g.self_adjoint_eigen(Side::Lower).map_err(|e| format!("Gram eigensolve: {e:?}"))?;
        let lambda:Vec<f64>=(0..k).map(|i| eig.S().column_vector()[i]).collect();
        let lo=lambda.iter().copied().fold(f64::INFINITY,f64::min);
        let hi=lambda.iter().copied().fold(0.0,f64::max);
        if !(hi.is_finite() && hi>0.0 && lo>1e-12*hi) { return Err("dependent residual directions".into()); }
        let u:Vec<Vec<f64>>=(0..k).map(|j| (0..n).map(|i|
            (0..k).map(|a| r[a][i]*eig.U()[(a,j)]).sum::<f64>()/lambda[j].sqrt()).collect()).collect();
        Self::new(p,u)
    }
    pub fn expand(&self, v: &[f64]) -> Result<Vec<f64>,String> {
        let n=self.positions.len();
        if !finite(v,n+self.columns.len()) { return Err("invalid trial vector".into()); }
        let mut out=vec![0.0;self.ambient];
        for (i,&p) in self.positions.iter().enumerate() { out[p]=v[i]; }
        for (j,u) in self.columns.iter().enumerate() { for (a,b) in out.iter_mut().zip(u) { *a+=v[n+j]*b; }}
        Ok(out)
    }
    pub fn compress(&self, v: &[f64]) -> Result<Vec<f64>,String> {
        if !finite(v,self.ambient) { return Err("invalid ambient vector".into()); }
        let mut out:Vec<f64>=self.positions.iter().map(|p| v[*p]).collect();
        out.extend(self.columns.iter().map(|u| dot(u,v)));
        Ok(out)
    }
}
impl Block {
    pub fn new(l: Lift, hu: Vec<Vec<f64>>) -> Result<Self,String> {
        let k=l.columns.len();
        if hu.len()!=k || hu.iter().any(|v| !finite(v,l.ambient)) { return Err("invalid direction actions".into()); }
        let c=hu.iter().map(|v| l.positions.iter().map(|p| v[*p]).collect()).collect();
        let mut d:Vec<Vec<f64>>=l.columns.iter().map(|u| hu.iter().map(|v| dot(u,v)).collect()).collect();
        if d.iter().flatten().any(|v| !v.is_finite()) { return Err("nonfinite projected block".into()); }
        let scale=d.iter().flatten().map(|x| x.abs()).fold(1.0,f64::max);
        for i in 0..k { for j in 0..i {
            if (d[i][j]-d[j][i]).abs()>1e-10*scale { return Err("projected block is asymmetric".into()); }
            let a=0.5*(d[i][j]+d[j][i]); d[i][j]=a; d[j][i]=a;
        }}
        Ok(Self{lift:l,c,d})
    }
    pub fn apply(&self, v: &[f64], hx: &[f64]) -> Result<Vec<f64>,String> {
        let n=self.lift.positions.len(); let k=self.c.len();
        if !finite(v,n+k) || !finite(hx,n) { return Err("invalid block action input".into()); }
        let mut out=hx.to_vec();
        for (j,c) in self.c.iter().enumerate() { for i in 0..n { out[i]+=c[i]*v[n+j]; }}
        for j in 0..k { out.push(dot(&self.c[j],&v[..n])+dot(&self.d[j],&v[n..])); }
        if out.iter().any(|x| !x.is_finite()) { return Err("nonfinite block action".into()); }
        Ok(out)
    }
}

#[derive(Deserialize)]
struct SavedState {
    name: String, level_index: usize, energy: f64,
    coeff_bits: Vec<u64>, probe_residual_bits: Vec<u64>,
}
#[derive(Deserialize)]
struct OldLevels { levels: Vec<f64> }
#[derive(Deserialize)]
struct Saved {
    schema: String, s: f64, t: f64, g2: f64, mq: usize, probe_mq: usize,
    certified: bool, full_grid_released: bool, basis: Vec<basis::State>, probe_basis: Vec<basis::State>,
    identification: OldLevels, states: Vec<SavedState>,
}
fn decode(bits: &[u64], n: usize) -> Result<Vec<f64>,String> {
    let v:Vec<f64>=bits.iter().map(|b| f64::from_bits(*b)).collect();
    if !finite(&v,n) { return Err("invalid saved bit patterns".into()); }
    Ok(v)
}
fn bits(v: &[f64]) -> Vec<u64> { v.iter().map(|x| x.to_bits()).collect() }

#[derive(Serialize)]
pub struct EnrichedState {
    name: &'static str, level_index: usize, energy: f64,
    inside_norm: f64, outside_norm: f64, ambient_norm: f64,
    probe_residual_bits: Vec<u64>,
}
#[derive(Serialize)]
pub struct EnrichmentExport {
    schema: &'static str, scope: &'static str,
    s: f64, t: f64, g2: f64, base_mq: usize, probe_mq: usize,
    base_dimension: usize, dimension: usize, ambient_dimension: usize,
    certified: bool, full_grid_released: bool, domain_gate: &'static str,
    solver: SolveSettings, identification_residual_space: &'static str,
    pub identification: ritz::HelikeCompetitor,
    input_action_errors: Vec<f64>, block_action_relative_errors: Vec<f64>,
    lift_positions: Vec<usize>, lift_columns_bits: Vec<Vec<u64>>, vectors_bits: Vec<Vec<u64>>,
    ambient_residual_norms: Vec<f64>, inside_via_ambient_norms: Vec<f64>,
    states: Vec<EnrichedState>, applications: usize, krylov_dimension: usize,
    pub seconds: f64,
}

pub fn enrich(text: &str) -> Result<EnrichmentExport,String> {
    let start=std::time::Instant::now();
    let saved:Saved=serde_json::from_str(text).map_err(|e| format!("invalid support export: {e}"))?;
    let set=SolveSettings::default();
    if saved.schema!="helike-lab/cone-support/v1" || saved.certified || saved.full_grid_released
        || !(2..=12).contains(&saved.mq) || saved.probe_mq!=saved.mq+2
        || !(saved.g2.is_finite() && saved.g2>0.0) || saved.states.len()!=4
        || !finite(&saved.identification.levels,set.nev)
        || saved.identification.levels.windows(2).any(|v| v[0]>v[1]) {
        return Err("unsupported support export contract".into());
    }
    let bg=Background::maurer_cartan(saved.s,saved.t,saved.g2.sqrt())?;
    eprintln!("enrichment: rebuilding unchanged operators; no ambient eigensolve");
    let low=ConeOperator::exact(bg.clone(),saved.mq,Mutation::None)?;
    let high=ConeOperator::exact(bg,saved.probe_mq,Mutation::None)?;
    if low.basis!=saved.basis || high.basis!=saved.probe_basis { return Err("saved occupation keys differ".into()); }
    let index=basis::index_of(&high.basis);
    let positions:Vec<usize>=low.basis.iter().map(|s| index[s]).collect();
    let names=["ground","helike_a","helike_b","competitor"];
    if saved.states.iter().map(|s| s.level_index).collect::<HashSet<_>>().len()!=4 || saved.states[0].level_index!=0 {
        return Err("duplicate or missing diagnostic states".into());
    }
    let mut old=Vec::new();
    for (j,s) in saved.states.iter().enumerate() {
        if s.name!=names[j] || s.level_index>=set.nev || !s.energy.is_finite()
            || (s.energy-saved.identification.levels[s.level_index]).abs()>1e-10 {
            return Err("state identity or energy mismatch".into());
        }
        let v=decode(&s.coeff_bits,low.dim())?;
        if (dot(&v,&v)-1.0).abs()>1e-8 { return Err("unnormalized saved state".into()); }
        old.push(v);
    }
    let padded:Vec<Vec<f64>>=old.iter().map(|v| support::embed(&low.basis,&high.basis,v)).collect::<Result<_,_>>()?;
    let ha=high.apply(&padded,Part::Full);
    let la=low.apply(&old,Part::Full);
    let mut residuals=Vec::new(); let mut input_action_errors=Vec::new();
    for j in 0..4 {
        let state=&saved.states[j]; let e=state.energy;
        let recorded=decode(&state.probe_residual_bits,high.dim())?;
        let fresh:Vec<f64>=ha[j].iter().zip(&padded[j]).map(|(h,v)| h-e*v).collect();
        let inside:Vec<f64>=la[j].iter().zip(&old[j]).map(|(h,v)| h-e*v).collect();
        let error=difference(&fresh,&recorded);
        if !error.is_finite() || error>set.tol*e.abs().max(1.0) || norm(&inside)>set.tol*e.abs().max(1.0) {
            return Err("saved state does not replay under the unchanged operator".into());
        }
        input_action_errors.push(error);
        let mut r=recorded;
        for &p in &positions { r[p]=0.0; }
        residuals.push(r);
    }
    let lift=Lift::from_residuals(positions,residuals)?;
    let hu=high.apply(&lift.columns,Part::Full);
    let block=Block::new(lift,hu)?;
    let n=low.dim()+4;
    let mut block_action_relative_errors=Vec::new();
    for seed in 1..=3 {
        let v=lanczos::seeded(n,seed);
        let direct=high.apply(&[block.lift.expand(&v)?],Part::Full);
        let expected=block.lift.compress(&direct[0])?;
        let hx=low.apply(&[v[..low.dim()].to_vec()],Part::Full);
        let actual=block.apply(&v,&hx[0])?;
        let error=difference(&actual,&expected)/norm(&expected).max(1.0);
        if !error.is_finite() || error>1e-9 { return Err("block/direct operator disagreement".into()); }
        block_action_relative_errors.push(error);
    }
    eprintln!("enrichment: replay and block gates passed; solving dimension={n}, elapsed={:.1}s",start.elapsed().as_secs_f64());
    let initial:Vec<Vec<f64>>=(0..set.block).map(|j| {let mut v=vec![0.0;n];v[j]=1.0;v}).collect();
    let eigs=lanczos::lowest(n,set.nev,set.block,&initial,set.tol,set.max_dim,|vs| {
        let xs:Vec<Vec<f64>>=vs.iter().map(|v| v[..low.dim()].to_vec()).collect();
        let hx=low.apply(&xs,Part::Full);
        vs.iter().zip(&hx).map(|(v,h)| block.apply(v,h).expect("validated finite block action")).collect()
    });
    if eigs.values.len()!=set.nev || eigs.residuals.len()!=set.nev || eigs.vectors.len()!=set.nev
        || eigs.values.iter().zip(&eigs.residuals).any(|(e,r)| !e.is_finite() || !r.is_finite() || *r>set.tol*e.abs().max(1.0))
        || eigs.values.iter().zip(&saved.identification.levels).any(|(e,old)| *e>old+1e-8) {
        return Err("enriched solve unresolved or nested variational check failed".into());
    }
    let expanded:Vec<Vec<f64>>=eigs.vectors.iter().map(|v| block.lift.expand(v)).collect::<Result<_,_>>()?;
    let hv=high.apply(&expanded,Part::Full);
    let mut ambient_residuals=Vec::new(); let mut ambient_residual_norms=Vec::new(); let mut inside_via_ambient_norms=Vec::new();
    for j in 0..set.nev {
        let r:Vec<f64>=hv[j].iter().zip(&expanded[j]).map(|(h,v)| h-eigs.values[j]*v).collect();
        let inside=norm(&block.lift.compress(&r)?);
        if !inside.is_finite() || inside>set.tol*eigs.values[j].abs().max(1.0) { return Err("ambient projection contradicts converged solve".into()); }
        inside_via_ambient_norms.push(inside); ambient_residual_norms.push(norm(&r)); ambient_residuals.push(r);
    }
    eprintln!("enrichment: solve complete; identifying expanded states, elapsed={:.1}s",start.elapsed().as_secs_f64());
    let ambient_eigs=lanczos::Eigs{values:eigs.values.clone(),vectors:expanded,residuals:eigs.residuals.clone(),
        applications:eigs.applications,krylov_dimension:eigs.krylov_dimension};
    let wh=ritz::helike_frequency(saved.s,saved.t);
    let identification=ritz::helike_and_competitor(&high,&ambient_eigs,wh)?;
    let roles=ritz::mode_roles(&high.bg.w,wh)?;
    let amps=ritz::reference_amplitudes(&high,&ambient_eigs.vectors,identification.overlap_orders[1])?;
    let weight=|j:usize| amps[roles[0]*set.nev+j].powi(2)+amps[roles[1]*set.nev+j].powi(2);
    let mut order:Vec<usize>=(1..set.nev).collect(); order.sort_by(|a,b| weight(*b).total_cmp(&weight(*a)));
    let mut pair=[order[0],order[1]]; pair.sort();
    let selected=[0,pair[0],pair[1],identification.competitor.level_index];
    if selected.iter().collect::<HashSet<_>>().len()!=4 || !pair.contains(&identification.helike.level_index) {
        return Err("ambiguous enriched state selection".into());
    }
    let mut states=Vec::new();
    for (i,&j) in selected.iter().enumerate() {
        let r=&ambient_residuals[j];
        let projection=block.lift.expand(&block.lift.compress(r)?)?;
        states.push(EnrichedState{name:names[i],level_index:j,energy:eigs.values[j],inside_norm:inside_via_ambient_norms[j],
            outside_norm:difference(r,&projection),ambient_norm:norm(r),probe_residual_bits:bits(r)});
    }
    eprintln!("enrichment: diagnostic complete, elapsed={:.1}s",start.elapsed().as_secs_f64());
    Ok(EnrichmentExport{schema:"helike-lab/cone-enrichment/v1",scope:"one nested residual enrichment; finite Mq+2 residual, not a domain certificate or gap error bound",
        s:saved.s,t:saved.t,g2:saved.g2,base_mq:saved.mq,probe_mq:saved.probe_mq,base_dimension:low.dim(),dimension:n,ambient_dimension:high.dim(),
        certified:false,full_grid_released:false,domain_gate:"NOT_RUN",solver:set,identification_residual_space:"W",
        identification,input_action_errors,block_action_relative_errors,lift_positions:block.lift.positions,
        lift_columns_bits:block.lift.columns.iter().map(|v| bits(v)).collect(),vectors_bits:eigs.vectors.iter().map(|v| bits(v)).collect(),
        ambient_residual_norms,inside_via_ambient_norms,states,applications:eigs.applications,krylov_dimension:eigs.krylov_dimension,seconds:start.elapsed().as_secs_f64()})
}
