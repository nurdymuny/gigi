"""Fail-closed RF2D evidence validation; cannot close the flow gate."""
import argparse
import json
from pathlib import Path
import subprocess
import numpy as np
from scipy.sparse import load_npz
from monodromy_evidence import sha
from ricci_lattice_evidence import read,same
from ricci_lattice_convergence import reference_velocity
from test_ricci_operator_native import sparse_matrix
from ricci_shape_experiment import write
from ricci_consistency_diagnostic import (BASE,ROOT,frozen,assemble,scalar_control,hhe_matrix_control)
from ricci_consistency_math import base_log,residual_split,solve_rhs


def extract_hhe(stdout):
    return [json.loads(line.split('OC08_HHE ',1)[1]) for line in stdout.splitlines() if 'OC08_HHE ' in line]


def capture_native(directory,binary):
    binary=Path(binary).resolve()
    result=subprocess.run([str(binary),'--nocapture','--test-threads=1'],capture_output=True,text=True,check=True,timeout=120)
    if '6 passed; 0 failed' not in result.stdout:raise ValueError('all six native tests including HHE must run')
    polynomial=extract_hhe(result.stdout)
    if len(polynomial)!=1 or polynomial[0]['status']!='bounded_holomorphic':raise ValueError('native HHE receipt missing')
    path=directory/'native-tests.stdout.txt';path.write_text(result.stdout)
    receipt={'binary':str(binary),'binary_sha256':sha(binary),'tests_passed':6,
        'test_source_sha256':sha(ROOT/'tests/ricci_operator_consistency.rs'),
        'stdout_sha256':sha(path),'hhe':polynomial[0]}
    write(directory/'NATIVE_RECEIPT.json',receipt)


def verify(directory):
    directory=Path(directory)
    for name in ['gate.PASS.json','VERIFIED_DIAGNOSTIC.json']:(directory/name).unlink(missing_ok=True)
    r=read(directory/'diagnostic.json')
    keys={f'level{i}_q{q}' for i in range(4) for q in [4,6]}
    if (r.get('schema')!='rf2d_diagnostic_v1' or r.get('status')!='DIAGNOSTIC_COMPLETE_FULL_GATE_OPEN'
        or r.get('release_gate')!='OPEN' or r.get('correspondence')!='NOT_ESTABLISHED'
        or r.get('operator_changed') is not False or r.get('full_regressions')!='DEFERRED'
        or set(r.get('cases',{}))!=keys):raise ValueError('incomplete/altered RF2D contract')
    if sha(directory/'manifest.json')!=r['manifest_sha256']:raise ValueError('manifest digest mismatch')
    manifest=read(directory/'manifest.json')
    if manifest!=frozen():raise ValueError('mapping, source, threshold or prior evidence drift')
    for key,row in r['cases'].items():
        if sha(directory/(key+'.arrays.npz'))!=row['arrays_sha256']:raise ValueError('array evidence changed')
    checked={};quadrature=[]
    for level in range(4):
        values={}
        for order in [4,6]:
            key=f'level{level}_q{order}';native=f'level{level}_berger_q{order}'
            payload=read(BASE/(native+'.input.json'));raw=read(BASE/(native+'.output.json'))
            load,m,k,vol,phase=assemble(payload,raw,order);b=sparse_matrix(raw)
            reference=reference_velocity(payload['edge_points'],[3.,1.,1.])
            split=residual_split(b,np.ravel(raw['gradient']),reference,load,raw['volume'])
            edges=np.array(payload['edge_points']);a=base_log(edges[:,0],edges[:,1])[:,0]
            control,velocity=scalar_control(m,k,a,vol)
            expected={'level':level,'order':order,'berger_split':split,'double_cover':phase,'abelian_control':control}
            if level==0:
                expected['hhe_matrix_control']=hhe_matrix_control(m,k)
                for name,matrix in [('mass',m),('stiffness',k)]:
                    saved=load_npz(directory/f'coarse_q{order}_{name}.npz')
                    np.testing.assert_array_equal(saved.toarray(),matrix.toarray())
            same({k:v for k,v in r['cases'][key].items() if k!='arrays_sha256'},expected)
            with np.load(directory/(key+'.arrays.npz'),allow_pickle=False) as saved:
                if set(saved.files)!={'load','abelian_velocity','abelian_edge_integrals'}:raise ValueError('array keys mismatch')
                for name,value in [('load',load),('abelian_velocity',velocity),('abelian_edge_integrals',a)]:
                    np.testing.assert_array_equal(saved[name],value)
            values[order]=(load,velocity);checked[key]=expected
            print('verified',key,flush=True)
        load_delta=values[6][0]-values[4][0];correction=solve_rhs(b,load_delta)
        scalar_delta=values[6][1]-values[4][1]
        difference={'level':level,'load_dual_mass_difference':float(np.sqrt(load_delta@correction/raw['volume'])),
            'abelian_velocity_mass_difference':float(np.sqrt(scalar_delta@(m@scalar_delta)/vol))}
        if (difference['load_dual_mass_difference']>=split['total_error']/10
            or difference['abelian_velocity_mass_difference']>=control['full_mass_error']/10):
            raise ValueError('quadrature not separated from measured discrepancy')
        quadrature.append(difference)
    native=read(directory/'NATIVE_RECEIPT.json')
    if (native['binary_sha256']!=sha(Path(native['binary'])) or native['tests_passed']!=6
        or native['test_source_sha256']!=sha(ROOT/'tests/ricci_operator_consistency.rs')
        or native['stdout_sha256']!=sha(directory/'native-tests.stdout.txt')):raise ValueError('native evidence drift')
    rerun=subprocess.run([native['binary'],'--nocapture','--test-threads=1'],capture_output=True,text=True,check=True,timeout=120)
    if '6 passed; 0 failed' not in rerun.stdout:raise ValueError('native test replay failed')
    hhe=extract_hhe(rerun.stdout)
    if len(hhe)!=1:raise ValueError('HHE replay missing')
    same(native['hhe'],hhe[0])
    if manifest!=frozen():raise ValueError('source drift during verification')
    report={'evidence_valid':True,'recomputed_cases':8,'native_tests_replayed':6,
        'full_flow_gate':'FAIL_RETAINED_FROM_RF2C','release_gate':'OPEN','correspondence':'NOT_ESTABLISHED',
        'quadrature':quadrature,'diagnostic_sha256':sha(directory/'diagnostic.json'),
        'native_receipt_sha256':sha(directory/'NATIVE_RECEIPT.json'),
        'verifier_sha256':sha(Path(__file__)),'full_regressions':'DEFERRED'}
    write(directory/'VERIFIED_DIAGNOSTIC.json',report)
    return report


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('directory',type=Path);p.add_argument('--native-binary',type=Path)
    a=p.parse_args();d=a.directory.resolve()
    if a.native_binary:capture_native(d,a.native_binary)
    print(json.dumps(verify(d),indent=2))
