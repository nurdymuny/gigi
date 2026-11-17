"""Fail-closed verification of RF2B diagnostics; cannot issue a release PASS."""
import argparse
import json
from pathlib import Path
import re
import subprocess
import tempfile
from concurrent.futures import ThreadPoolExecutor
import numpy as np
from monodromy_evidence import sha
from ricci_lattice_operator import orbit_fixture
from ricci_lattice_mesh import mesh,validate
from ricci_lattice_convergence import CASES,measures,solve,ROOT


def read(path):
    def nonfinite(value):raise ValueError('nonfinite JSON: '+value)
    def unique(pairs):
        out={}
        for key,value in pairs:
            if key in out:raise ValueError('duplicate JSON key: '+key)
            out[key]=value
        return out
    return json.loads(path.read_text(),parse_constant=nonfinite,object_pairs_hook=unique)


def same(actual,expected):
    if isinstance(expected,dict):
        if not isinstance(actual,dict) or actual.keys()!=expected.keys():raise ValueError('evidence keys mismatch')
        for key in expected:same(actual[key],expected[key])
    elif isinstance(expected,list):
        if not isinstance(actual,list) or len(actual)!=len(expected):raise ValueError('evidence list mismatch')
        for a,b in zip(actual,expected):same(a,b)
    elif isinstance(expected,float):
        if isinstance(actual,bool) or not isinstance(actual,(int,float)) or not np.isfinite(actual):raise ValueError('finite numerical evidence required')
        if not np.isclose(actual,expected,rtol=2e-10,atol=2e-12):raise ValueError(f'computed evidence mismatch: {actual} versus {expected}')
    elif actual!=expected or type(actual)!=type(expected):raise ValueError('evidence value mismatch')


def verify(directory,binary_override=None):
    directory=Path(directory)
    for name in ['gate.PASS.json','VERIFIED_DIAGNOSTIC.json']:(directory/name).unlink(missing_ok=True)
    receipt=read(directory/'diagnostic.json')
    if (receipt.get('schema')!='rf2b_operator_diagnostic_v1' or receipt.get('gate')!='OPEN'
        or receipt.get('continuum_convergence_proven') is not False or receipt.get('time_scale')!=1
        or receipt.get('fitted_constants')!=[] or receipt.get('status')!='DIAGNOSTIC_COMPLETE_FULL_GATE_OPEN'):
        raise ValueError('partial diagnostic cannot claim correspondence, fitted clocks or completed release')
    if not receipt.get('sources') or not receipt.get('cases'):raise ValueError('source and case evidence required')
    for name,digest in receipt['sources'].items():
        p=Path(name).resolve()
        if not p.is_relative_to(ROOT) or sha(p)!=digest:raise ValueError('source mismatch: '+name)
    binary=Path(binary_override) if binary_override is not None else Path(receipt['binary']['path'])
    if sha(binary)!=receipt['binary']['sha256']:raise ValueError('binary mismatch')
    checked={};geometry={};velocities={}
    with tempfile.TemporaryDirectory(prefix='rf2b-replay-') as temporary,ThreadPoolExecutor(max_workers=4) as pool:
        replays={}
        for key,case in receipt['cases'].items():
            if not re.fullmatch(r'level([0-3])_([a-z_]+)_q([46])',key):raise ValueError('unknown case key')
            source=directory/(key+'.input.json');result=directory/(key+'.output.json')
            if sha(source)!=case['input_sha256'] or sha(result)!=case['output_sha256']:raise ValueError('raw evidence hash mismatch')
            replay=Path(temporary)/(key+'.json')
            future=pool.submit(subprocess.run,[str(binary),str(source),str(replay)],check=True,timeout=1200,capture_output=True)
            replays[key]=(future,replay)
        for key,case in receipt['cases'].items():
            match=re.fullmatch(r'level([0-3])_([a-z_]+)_q([46])',key)
            if not match or match[2] not in CASES:raise ValueError('unknown case key')
            level,name,order=int(match[1]),match[2],int(match[3]);g=CASES[name]
            if case['level']!=level or case['metric']!=g or case['quadrature_order']!=order:raise ValueError('case identity mismatch')
            source=directory/(key+'.input.json');result=directory/(key+'.output.json')
            if sha(source)!=case['input_sha256'] or sha(result)!=case['output_sha256']:raise ValueError('raw evidence hash mismatch')
            fixture=orbit_fixture(level)
            same(read(source),dict(metric=g,order=order,edge_points=fixture['edge_points'],cells=fixture['cells']))
            future,replay=replays[key];future.result()
            if sha(replay)!=sha(result):raise ValueError('native replay differs from captured result')
            output=read(result);numbers=measures(fixture,g,output)
            same(case['measures'],numbers)
            if level not in geometry:geometry[level]=validate(mesh(level))
            same(case['mesh'],geometry[level])
            checked[key]=numbers
            b,w,_=solve(output);velocities[key]=(b,w)
            print('verified',key,flush=True)
    expected={f'level{level}_{name}_q{order}' for level in range(4) for name in CASES for order in [4,6]}
    complete=checked.keys()==expected
    summaries={};operator_gate='OPEN'
    if complete:
        for name in CASES:
            rows=[checked[f'level{i}_{name}_q6'] for i in range(4)]
            errors=[r['full_mass_error'] for r in rows]
            orders=[float(np.log(errors[i]/errors[i+1])/np.log(geometry[i]['h_max']/geometry[i+1]['h_max'])) for i in range(3)]
            quadrature=[]
            for i,row in enumerate(rows):
                b,w=velocities[f'level{i}_{name}_q6'];_,low=velocities[f'level{i}_{name}_q4']
                delta=w-low;quadrature.append(float(np.sqrt(delta@(b@delta)/row['volume'])))
            if name=='round':passes=False  # no unprescribed round tolerance may close a gate
            else:passes=all(rate>=.5 for rate in orders[-2:]) and rows[-1]['relative_error']<.05 and all(q<e/10 for q,e in zip(quadrature,errors))
            summaries[name]={'errors':errors,'observed_orders':orders,'relative_errors':[r['relative_error'] for r in rows],
                'quadrature_mass_differences':quadrature,'noncritical_acceptance':passes if name!='round' else None}
        operator_gate='FAIL' if any(not r['noncritical_acceptance'] for n,r in summaries.items() if n!='round') else 'NUMERICAL_CRITERIA_ONLY'
    report={'evidence_valid':True,'correspondence':'NOT_ESTABLISHED','release_gate':'OPEN',
        'operator_numeric_gate':operator_gate,'four_level_data_complete':complete,'native_replays':len(checked),
        'summary':summaries,'diagnostic_sha256':sha(directory/'diagnostic.json'),
        'verifier_sources':{str(p.resolve()):sha(p) for p in [Path(__file__),ROOT/'e2e/probes/ricci_lattice_convergence.py',
            ROOT/'e2e/probes/ricci_wilson_oracle.py',ROOT/'e2e/probes/ricci_oracle.py',ROOT/'e2e/probes/test_ricci_operator_native.py']}}
    (directory/'VERIFIED_DIAGNOSTIC.json').write_text(json.dumps(report,indent=2,allow_nan=False))
    return report


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('directory',type=Path)
    p.add_argument('--assert-operator',action='store_true')
    p.add_argument('--binary',type=Path)
    a=p.parse_args();report=verify(a.directory,a.binary);print(json.dumps(report,indent=2))
    if a.assert_operator:
        assert report['four_level_data_complete'] and report['operator_numeric_gate']=='NUMERICAL_CRITERIA_ONLY', 'BW05 full all-link convergence acceptance FAILED; release gate remains OPEN'
