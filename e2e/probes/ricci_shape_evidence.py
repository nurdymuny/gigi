"""RF2C staged diagnostic evidence, never a correspondence or release proof."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import numpy as np
from scipy.sparse.linalg import cg
from ricci_lattice_evidence import read,same
from ricci_lattice_convergence import solve,measures
from ricci_shape_experiment import CONTRACT,check_manifest,case_keys,write
from ricci_shape_regular_mesh import mesh
from ricci_shape_certificate import certify
from ricci_mesh_orbits import orbit_fixture_for_mesh
from monodromy_evidence import sha


def numerical_gate(rows, h, quadrature, linear):
    if any(len(a)!=4 for a in [rows,h,quadrature,linear]):raise ValueError('all four levels required')
    errors=[r['full_mass_error'] for r in rows];relative=[r['relative_error'] for r in rows]
    if not np.isfinite([errors,relative,h,quadrature,linear]).all():raise ValueError('finite evidence required')
    if any(x<=0 for x in h) or any(a<=b for a,b in zip(h,h[1:])):raise ValueError('strictly decreasing measured h required')
    if any(x<0 for x in errors+relative+quadrature+linear):raise ValueError('nonnegative errors required')
    if any(x==0 for x in errors):
        return {'pass':False,'observed_orders':[],'reason':'zero errors do not establish the prescribed observed orders'}
    orders=[float(np.log(errors[i]/errors[i+1])/np.log(h[i]/h[i+1])) for i in range(3)]
    accepted=(all(rate>=.5 for rate in orders[-2:]) and relative[-1]<.05
        and all(q<e/10 for q,e in zip(quadrature,errors)) and all(l<e/10 for l,e in zip(linear,errors)))
    return {'pass':bool(accepted),'errors':errors,'relative_errors':relative,'observed_orders':orders,
        'h_max':h,'quadrature_mass_differences':quadrature,'linear_correction_mass_estimates':linear}


def linear_estimate(b,w,raw):
    residual=b@w+np.ravel(raw['gradient'])
    correction,status=cg(b,residual,rtol=1e-12,atol=1e-25,maxiter=4000)
    if status or np.linalg.norm(b@correction-residual)>3*max(1e-12*np.linalg.norm(residual),1e-25):
        raise ValueError('linear correction estimate did not converge')
    return float(np.sqrt(max(0.,correction@(b@correction))/raw['volume']))


def verify(directory):
    directory=Path(directory)
    for name in ['gate.PASS.json','VERIFIED_DIAGNOSTIC.json']:(directory/name).unlink(missing_ok=True)
    receipt=read(directory/'diagnostic.json')
    if (receipt.get('schema')!='rf2c_berger_diagnostic_v1' or receipt.get('status')!='BERGER_COMPLETE_GATE_OPEN'
        or receipt.get('contract')!=CONTRACT or receipt.get('remaining_metrics')!='NOT_RUN'
        or receipt.get('full_regressions')!='DEFERRED' or set(receipt.get('cases',{}))!=case_keys()
        or set(receipt.get('geometry',{}))!={'0','1','2','3'}):raise ValueError('incomplete/altered staged contract')
    if sha(directory/'manifest.json')!=receipt['manifest_sha256']:raise ValueError('manifest hash mismatch')
    manifest=read(directory/'manifest.json');check_manifest(manifest);binary=manifest['binary']['path']
    for key,case in receipt['cases'].items():
        if sha(directory/(key+'.input.json'))!=case['input_sha256'] or sha(directory/(key+'.output.json'))!=case['output_sha256']:
            raise ValueError('raw evidence hash mismatch')
    checked={};hs=[];quadrature=[];linear=[]
    with tempfile.TemporaryDirectory(prefix='rf2c-replay-') as temp,ThreadPoolExecutor(max_workers=4) as pool:
        replays={key:pool.submit(subprocess.run,[binary,str(directory/(key+'.input.json')),str(Path(temp)/(key+'.json'))],
            check=True,timeout=1800,capture_output=True) for key in sorted(case_keys())}
        for level in range(4):
            m=mesh(level);geometry=certify(m,level);same(receipt['geometry'][str(level)],geometry)
            hs.append(geometry['h_max']);fixture=orbit_fixture_for_mesh(m);velocities={};high=None
            for order in [4,6]:
                key=f'level{level}_berger_q{order}';case=receipt['cases'][key]
                expected=dict(metric=CONTRACT['metric'],order=order,edge_points=fixture['edge_points'],cells=fixture['cells'])
                if hashlib.sha256(json.dumps(expected,allow_nan=False).encode()).hexdigest()!=case['input_sha256']:
                    raise ValueError('native request differs from regenerated mesh/metric')
                replays[key].result()
                if sha(Path(temp)/(key+'.json'))!=case['output_sha256']:raise ValueError('native replay differs')
                raw=read(directory/(key+'.output.json'));numbers=measures(fixture,CONTRACT['metric'],raw)
                same(case['measures'],numbers);checked[key]=numbers
                b,w,_=solve(raw);velocities[order]=w
                if order==6:high=(b,raw)
                print('verified',key,flush=True)
            b,raw=high;delta=velocities[6]-velocities[4]
            quadrature.append(float(np.sqrt(delta@(b@delta)/raw['volume'])))
            linear.append(linear_estimate(b,velocities[6],raw));mesh.cache_clear()
    check_manifest(manifest)
    result=numerical_gate([checked[f'level{i}_berger_q6'] for i in range(4)],hs,quadrature,linear)
    report={'evidence_valid':True,'native_replays':8,'berger_numeric_gate':'PASS' if result['pass'] else 'FAIL',
        'summary':result,'remaining_metrics':'NOT_RUN','full_regressions':'DEFERRED','release_gate':'OPEN',
        'correspondence':'NOT_ESTABLISHED','diagnostic_sha256':sha(directory/'diagnostic.json'),
        'verifier_sha256':sha(Path(__file__))}
    write(directory/'VERIFIED_DIAGNOSTIC.json',report)
    return report


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('directory',type=Path);p.add_argument('--assert-operator',action='store_true')
    a=p.parse_args();r=verify(a.directory);print(json.dumps(r,indent=2))
    if a.assert_operator and r['berger_numeric_gate']!='PASS':raise SystemExit('BW05 FAILED; correspondence NOT ESTABLISHED')
