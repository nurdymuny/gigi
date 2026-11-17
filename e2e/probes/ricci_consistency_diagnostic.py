"""RF2D exact residual accounting, double cover audit, and linear control."""
import argparse
import json
from pathlib import Path
import time
import numpy as np
from scipy.linalg import eigh,solve_triangular
from scipy.sparse import coo_matrix,save_npz
from monodromy_evidence import sha
from ricci_lattice_evidence import read,same
from ricci_lattice_convergence import ROOT,reference_velocity
from ricci_shape_experiment import check_manifest,write
from test_ricci_operator_native import sparse_matrix
from ricci_consistency_math import (EDGES,INCIDENCE,base_log,link_quaternions,compose,inverse,
    rotations,phase_summary,cell_integrals,solve_rhs,residual_split)

BASE=ROOT/'output/ricci-shape-regular-20261117/berger'
SPEC=ROOT/'docs/RICCI_OPERATOR_CONSISTENCY_SPEC.md'
SOURCES=[Path(__file__),ROOT/'e2e/probes/ricci_consistency_math.py',SPEC,
    ROOT/'tests/ricci_operator_consistency.rs',ROOT/'src/geometry/harmonic_conjugate.rs',
    ROOT/'src/gauge/group_element.rs']


def frozen():
    m=read(BASE/'manifest.json');check_manifest(m)
    r=read(BASE/'VERIFIED_DIAGNOSTIC.json')
    if not r['evidence_valid'] or r['native_replays']!=8 or r['diagnostic_sha256']!=sha(BASE/'diagnostic.json'):
        raise ValueError('verified native RF2C baseline required')
    d=read(BASE/'diagnostic.json')
    for key,row in d['cases'].items():
        for kind in ['input','output']:
            if sha(BASE/(key+'.'+kind+'.json'))!=row[kind+'_sha256']:raise ValueError('prior raw data drift')
    return {'baseline_manifest_sha256':sha(BASE/'manifest.json'),
        'baseline_diagnostic_sha256':sha(BASE/'diagnostic.json'),
        'baseline_verified_sha256':sha(BASE/'VERIFIED_DIAGNOSTIC.json'),
        'sources':{str(p.resolve()):sha(p) for p in SOURCES},
        'mapping':'RF2C_UNCHANGED','thresholds':'BW05_UNCHANGED','time_scale':1,
        'resolutions':[1,2,3,5],'orders':[4,6]}


def assemble(payload,raw,order):
    n=len(raw['links']);links=np.array(raw['links']);cells=payload['cells']
    load=np.zeros((n,3));rowidx=[];colidx=[];massvals=[];stiffvals=[];vol=0.;face_hol=[]
    for start in range(0,len(cells),128):
        batch=cells[start:start+128]
        points=np.array([c['vertices'] for c in batch]);ids=np.array([c['edges'] for c in batch])
        reverse=np.array([c['reverse'] for c in batch]);weight=np.array([c['weight'] for c in batch])
        local=links[ids].copy();local[...,1:]*=np.where(reverse,-1.,1.)[...,None]
        berg=cell_integrals(points,local,[3.,1.,1.],order)
        lift=np.where(reverse[...,None,None],-rotations(links[ids]),np.eye(3))
        part=np.einsum('ceij,cei,c->cej',lift,berg['load'],weight)
        np.add.at(load,ids.ravel(),part.reshape(-1,3))
        round_links=link_quaternions([1.,1.,1.],points[:,[a for a,b in EDGES]],points[:,[b for a,b in EDGES]])
        control=cell_integrals(points,round_links,[1.,1.,1.],order,velocity=np.zeros((3,3)))
        signs=np.where(reverse,-1.,1.);factor=weight[:,None,None]*signs[:,:,None]*signs[:,None,:]
        massvals.append((control['edge_mass']*factor).ravel())
        stiffness=np.einsum('fe,cfg,gh->ceh',INCIDENCE,control['face_mass'],INCIDENCE,optimize=True)
        stiffvals.append((stiffness*factor).ravel())
        rowidx.append(np.broadcast_to(ids[:,:,None],(len(batch),6,6)).ravel())
        colidx.append(np.broadcast_to(ids[:,None,:],(len(batch),6,6)).ravel())
        vol+=float(weight@control['volume'])
        hol=np.stack([compose(compose(local[:,a],local[:,b]),inverse(local[:,c]))
            for a,b,c in [(0,3,1),(0,4,2),(1,5,2),(3,5,4)]],axis=1)
        face_hol.append(hol.reshape(-1,4))
    indices=(np.concatenate(rowidx),np.concatenate(colidx))
    mass=coo_matrix((np.concatenate(massvals),indices),shape=(n,n)).tocsr()
    stiffness=coo_matrix((np.concatenate(stiffvals),indices),shape=(n,n)).tocsr()
    return load.ravel(),mass,stiffness,vol,{'links':phase_summary(links),
        'face_occurrences':phase_summary(np.concatenate(face_hol)),
        'scope':'signed chosen-frame links; gauge-invariant face classes; face occurrences not unique faces'}


def scalar_control(mass,stiffness,a,volume):
    discrete=solve_rhs(mass,-stiffness@a);reference=-4*a;error=discrete-reference
    norm=lambda v:float(np.sqrt(max(0.,v@(mass@v))/volume))
    measured=norm(error);ref=norm(reference)
    return {'full_mass_error':measured,'reference_mass_norm':ref,'relative_error':measured/ref,
        'rayleigh_quotient':float(a@(stiffness@a)/(a@(mass@a))),
        'energy_density':float(.5*a@(stiffness@a)/volume),'continuum_energy_density':2.,
        'mass_density':float(a@(mass@a)/volume),'continuum_mass_density':1.,
        'scalar_degrees_of_freedom':len(a),'volume':volume},discrete


def hhe_matrix_control(mass,stiffness):
    m=mass.toarray();k=stiffness.toarray();lower=np.linalg.cholesky(m)
    left=solve_triangular(lower,k,lower=True)
    h=solve_triangular(lower,left.T,lower=True).T
    lam,v=eigh(h);maximum=float(lam[-1]);z=2*h/maximum-np.eye(len(m));p=(np.eye(len(m))-z)/2
    raw=np.linalg.solve(m,k);raw_p=np.eye(len(m))-raw/maximum
    asym=float(np.linalg.norm(h-h.T)/np.linalg.norm(h));norm=float(np.linalg.norm(p,2))
    if asym>2e-12 or lam[0]<-1e-10 or norm>1+2e-10:raise ValueError('mass-whitened contraction control failed')
    return {'polynomial':'(1-z)/2','domain':'unit disk for mass-whitened Z; spectrum lies in [-1,1]',
        'dimension':len(m),'whitened_symmetry_relative_error':asym,
        'minimum_eigenvalue':float(lam[0]),'maximum_eigenvalue':maximum,
        'eigen_residual_relative':float(np.linalg.norm(h@v-v*lam)/np.linalg.norm(h)),
        'mass_norm_polynomial':norm,'unwhitened_euclidean_polynomial_norm':float(np.linalg.norm(raw_p,2)),
        'unwhitened_commutator_relative':float(np.linalg.norm(raw.T@raw-raw@raw.T)/np.linalg.norm(raw)**2),
        'scope':'finite coarse Abelian operator; no nonlinear stability or repaired flow claim'}


def compute(level,order,directory=None):
    key=f'level{level}_berger_q{order}';payload=read(BASE/(key+'.input.json'));raw=read(BASE/(key+'.output.json'))
    load,mass,stiffness,volume,phase=assemble(payload,raw,order)
    b=sparse_matrix(raw);reference=reference_velocity(payload['edge_points'],[3.,1.,1.])
    split=residual_split(b,np.ravel(raw['gradient']),reference,load,raw['volume'])
    prior=read(BASE/'diagnostic.json')['cases'][key]['measures']['full_mass_error']
    if abs(split['total_error']-prior)>2e-9:raise ValueError('full error no longer agrees with frozen RF2C')
    if split['recombination_error']>2e-9:raise ValueError('residual recombination failed')
    identity=split['action_error']**2+split['mass_error']**2+2*split['cross_inner']
    if abs(identity-split['total_error']**2)>2e-7:raise ValueError('norm identity failed')
    edges=np.array(payload['edge_points']);a=base_log(edges[:,0],edges[:,1])[:,0]
    control,velocity=scalar_control(mass,stiffness,a,volume)
    result={'level':level,'order':order,'berger_split':split,'double_cover':phase,'abelian_control':control}
    if level==0:result['hhe_matrix_control']=hhe_matrix_control(mass,stiffness)
    if directory is not None:
        p=directory/f'level{level}_q{order}.arrays.npz'
        np.savez_compressed(p,load=load,abelian_velocity=velocity,abelian_edge_integrals=a)
        result['arrays_sha256']=sha(p)
        if level==0:
            save_npz(directory/f'coarse_q{order}_mass.npz',mass)
            save_npz(directory/f'coarse_q{order}_stiffness.npz',stiffness)
    return result


def run(directory):
    directory.mkdir(parents=True,exist_ok=True)
    if (directory/'diagnostic.json').exists():raise ValueError('fresh directory required; preserve prior evidence')
    manifest=frozen();write(directory/'manifest.json',manifest)
    receipt={'schema':'rf2d_diagnostic_v1','status':'IN_PROGRESS','release_gate':'OPEN',
        'correspondence':'NOT_ESTABLISHED','operator_changed':False,'full_regressions':'DEFERRED',
        'manifest_sha256':sha(directory/'manifest.json'),'cases':{}}
    start=time.perf_counter()
    try:
        for level in range(4):
            for order in [4,6]:
                row=compute(level,order,directory);receipt['cases'][f'level{level}_q{order}']=row
                write(directory/'diagnostic.json',receipt);print('measured',json.dumps(row),flush=True)
        if manifest!=frozen():raise ValueError('source drift during diagnosis')
        receipt['status']='DIAGNOSTIC_COMPLETE_FULL_GATE_OPEN'
    except BaseException as e:receipt['status']='FAILED';receipt['error']=str(e);raise
    finally:
        receipt['elapsed_seconds']=time.perf_counter()-start;write(directory/'diagnostic.json',receipt)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('directory',type=Path);a=p.parse_args();run(a.directory.resolve())
