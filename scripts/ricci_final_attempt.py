"""Bounded RF2G tests, cached-matrix experiment and evidence replay."""
import argparse
import io
import json
from pathlib import Path
import subprocess
import sys
import time
import unittest
import numpy as np
from scipy.sparse import csr_matrix, eye, kron, load_npz

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'e2e/probes'))
from monodromy_evidence import sha
from ricci_lattice_evidence import read,same
from ricci_shape_experiment import write
from ricci_shape_evidence import numerical_gate
from ricci_variational_experiment import manifest as prior_manifest,measure
from ricci_variational_math import solve
from ricci_minimum_energy import recover

BASE=ROOT/'output/ricci-variational-20261117/abelian'
SOURCES=[Path(__file__),ROOT/'docs/RICCI_FINAL_BOUNDED_ATTEMPT.md',
         ROOT/'e2e/probes/ricci_minimum_energy.py',ROOT/'e2e/probes/test_ricci_minimum_energy.py']


def frozen():
    if read(BASE/'manifest.json')!=prior_manifest(): raise ValueError('prior source drift')
    verified=read(BASE/'VERIFIED_DIAGNOSTIC.json')
    if not verified['evidence_valid'] or verified['diagnostic_sha256']!=sha(BASE/'diagnostic.json'):
        raise ValueError('verified RF2F evidence required')
    prior=read(BASE/'diagnostic.json')
    for row in prior['cases'].values():
        for name,digest in row['artifacts'].items():
            if sha(BASE/name)!=digest: raise ValueError('cached matrix/array drift')
    return {'sources':{str(p.resolve()):sha(p) for p in SOURCES},
            'prior_manifest':sha(BASE/'manifest.json'),'prior_diagnostic':sha(BASE/'diagnostic.json'),
            'prior_verified':sha(BASE/'VERIFIED_DIAGNOSTIC.json'),
            'mapping':'UNCHANGED','thresholds':'BW05_UNCHANGED','clock':1,
            'candidate':'minimum_physical_energy_exact_face_flux','parameters':[]}


def test(directory,label):
    import test_ricci_minimum_energy as tests
    output=io.StringIO()
    result=unittest.TextTestRunner(stream=output,verbosity=2).run(
        unittest.defaultTestLoader.loadTestsFromTestCase(tests.MinimumEnergyTests))
    receipt={'passed':result.wasSuccessful(),'tests':result.testsRun,'failures':len(result.failures),
        'errors':len(result.errors),'output':output.getvalue(),
        'sources':{str(p.resolve()):sha(p) for p in SOURCES}}
    path=directory/(label+'.json')
    if path.exists(): raise ValueError('preserve prior receipt')
    write(path,receipt);print(output.getvalue())
    return 0 if result.wasSuccessful() else 1


def compute():
    prior=read(BASE/'diagnostic.json');cases={};hs=[];quad=[];linear=[]
    for level in range(4):
        velocities={};estimates=[]
        for order in [4,6]:
            key=f'level{level}_q{order}';old=prior['cases'][key]
            h=load_npz(BASE/(key+'.h.npz'));m=load_npz(BASE/(key+'.m.npz'));d=load_npz(BASE/(key+'.d.npz'))
            nf=d.shape[0];j=kron(eye(nf,format='csr'),np.ones((3,1))/3,format='csr')
            z=kron(eye(nf,format='csr'),np.array([[1.,0.],[0.,1.],[-1.,-1.]]),format='csr')
            with np.load(BASE/(key+'.arrays.npz'),allow_pickle=False) as saved:
                a=saved['a'][:,0];load=saved['load'][:,0]
            flux=d@a;actual=recover(h,j,z,flux);v,solve_receipt=solve(m,-d.T@actual['gradient'])
            volume=old['volume'];norm=lambda x:float(np.sqrt(max(0.,x@(m@x))/volume))
            row,other=measure(h,csr_matrix((h.shape[0],nf)),m,j,z,d,a,volume,load,direct=level==0)
            disagreement=norm(v-other)
            if disagreement>1e-8: raise ValueError('independent reduced-adjoint implementation disagrees')
            error=norm(v+4*a)
            if abs(error-row['full_mass_error'])>1e-8: raise ValueError('full norm comparison disagrees')
            stationarity=float(np.linalg.norm(z.T@(h@actual['moments'])))
            constraint=float(np.max(abs(actual['moments'].reshape(-1,3).sum(axis=1)-flux)))
            if constraint>1e-10: raise ValueError('original face flux changed')
            row.update({'full_mass_error':error,'relative_error':error/norm(-4*a),
                'independent_velocity_difference':disagreement,'constraint_max_error':constraint,
                'auxiliary_stationarity_norm':stationarity,'candidate_solves':actual['solves']+[solve_receipt]})
            cases[key]=row;velocities[order]=v;estimates.append(row['linear_correction_mass_estimate']+disagreement)
            print(key+' '+json.dumps({'relative':row['relative_error'],'full':error}),flush=True)
        hs.append(old['h_max']);quad.append(norm(velocities[6]-velocities[4]));linear.append(max(estimates))
    return {'cases':cases,'gate':numerical_gate([cases[f'level{i}_q6'] for i in range(4)],hs,quad,linear)}


def run(directory):
    path=directory/'diagnostic.json'
    if path.exists(): raise ValueError('preserve prior experiment')
    snapshot=frozen();write(directory/'manifest.json',snapshot)
    receipt={'status':'IN_PROGRESS','berger':'NOT_RUN','release_gate':'OPEN','full_regressions':'DEFERRED',
             'manifest_sha256':sha(directory/'manifest.json')}
    start=time.perf_counter()
    try:
        receipt.update(compute())
        if snapshot!=frozen(): raise ValueError('source drift during final experiment')
        receipt['status']='COMPLETE_UNVERIFIED'
    except BaseException as error:
        receipt['status']='FAILED';receipt['error']=str(error);raise
    finally:
        receipt['elapsed_seconds']=time.perf_counter()-start;write(path,receipt)


def verify(directory):
    for name in ['VERIFIED.json','gate.PASS.json']:(directory/name).unlink(missing_ok=True)
    receipt=read(directory/'diagnostic.json')
    if (set(receipt)!={'status','berger','release_gate','full_regressions','manifest_sha256','elapsed_seconds','cases','gate'}
        or receipt['status']!='COMPLETE_UNVERIFIED' or receipt['berger']!='NOT_RUN'
        or receipt['release_gate']!='OPEN' or receipt['full_regressions']!='DEFERRED'
        or set(receipt['cases'])!={f'level{i}_q{q}' for i in range(4) for q in [4,6]}):
        raise ValueError('incomplete or forged final-attempt scope')
    snapshot=frozen()
    if read(directory/'manifest.json')!=snapshot or sha(directory/'manifest.json')!=receipt['manifest_sha256']:
        raise ValueError('manifest mismatch')
    replay=compute();same(receipt['cases'],replay['cases']);same(receipt['gate'],replay['gate'])
    if frozen()!=snapshot: raise ValueError('source drift during replay')
    result={'evidence_valid':True,'recomputed_cases':8,'abelian_gate':'PASS' if replay['gate']['pass'] else 'FAIL',
        'gate':replay['gate'],'berger':'NOT_RUN','correspondence':'NOT_ESTABLISHED','release_gate':'OPEN',
        'diagnostic_sha256':sha(directory/'diagnostic.json'),'verifier_sha256':sha(Path(__file__))}
    write(directory/'VERIFIED.json',result);print(json.dumps(result,indent=2))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('mode',choices=['test','run','verify','worker'])
    p.add_argument('directory',type=Path);p.add_argument('--label',default='TESTS');args=p.parse_args()
    args.directory.mkdir(parents=True,exist_ok=True)
    if args.mode=='test':sys.exit(test(args.directory,args.label))
    elif args.mode=='run':
        subprocess.run([sys.executable,__file__,'worker',str(args.directory)],check=True,timeout=180)
    elif args.mode=='worker':run(args.directory)
    else:verify(args.directory)
