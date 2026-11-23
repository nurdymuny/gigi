"""Mesh-only RF2C experiment: certify four meshes, then eight Berger runs."""
import argparse
from concurrent.futures import ThreadPoolExecutor,as_completed
import json
from pathlib import Path
import subprocess
import time
from monodromy_evidence import sha
from ricci_lattice_evidence import read
from ricci_lattice_convergence import ROOT,measures
from ricci_shape_regular_mesh import mesh,RESOLUTIONS
from ricci_mesh_orbits import orbit_fixture_for_mesh
from ricci_shape_certificate import certify

BINARY=ROOT/'output/ricci-lattice-20261117/bin/ricci_operator_probe.exe'
BINARY_SHA='71fb30dbbb5fbfcd01e7a516f72849bef9303cbae8b046f7fbd2fe7824b196ad'
BASELINE=ROOT/'output/ricci-lattice-20261117/four-level-operator'
SPEC=ROOT/'docs/RICCI_SHAPE_REGULAR_SPEC.md'
EXPERIMENT_FILES=[SPEC,*[ROOT/'e2e/probes'/name for name in [
    'ricci_shape_regular_mesh.py','ricci_mesh_orbits.py','ricci_shape_certificate.py',
    'ricci_shape_experiment.py','ricci_shape_evidence.py']]]
CONTRACT={'resolutions':[1,2,3,5],'metric':[3.,1.,1.],'orders':[4,6],
    'minimum_finest_two_orders':.5,'maximum_final_relative_error':.05,
    'error_separation':10.,'shape_cap':14.,'time_scale':1,'fitted_constants':[],
    'cg_rtol':1e-12,'cg_atol':1e-14,'gate':'OPEN','continuum_convergence_proven':False}


def write(path,value):
    path.write_text(json.dumps(value,indent=2,allow_nan=False))


def make_manifest():
    prior=read(BASELINE/'diagnostic.json');verified=read(BASELINE/'VERIFIED_DIAGNOSTIC.json')
    sources=dict(prior['sources'],**verified['verifier_sources'])
    for name,digest in sources.items():
        if sha(Path(name))!=digest:raise ValueError('RF2B mathematical source drift: '+name)
    if sha(BINARY)!=BINARY_SHA or prior['binary']['sha256']!=BINARY_SHA:raise ValueError('native binary drift')
    return {'schema':'rf2c_frozen_mapping_v1','contract':CONTRACT,
        'binary':{'path':str(BINARY),'sha256':BINARY_SHA},'frozen_sources':sources,
        'baseline':{str(BASELINE/name):sha(BASELINE/name) for name in ['diagnostic.json','VERIFIED_DIAGNOSTIC.json']},
        'experiment_sources':{str(p):sha(p) for p in EXPERIMENT_FILES},
        'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
        'freeze_scope':'RF2B hashes checked after initial mesh unit tests, before four-level certification and any RF2C flow run'}


def check_manifest(manifest):
    if manifest!=make_manifest():raise ValueError('frozen contract/source/identity drift')


def case_keys():
    return {f'level{level}_berger_q{order}' for level in range(4) for order in [4,6]}


def run(directory):
    directory.mkdir(parents=True,exist_ok=True)
    for name in ['gate.PASS.json','VERIFIED_DIAGNOSTIC.json']:(directory/name).unlink(missing_ok=True)
    if (directory/'diagnostic.json').exists():raise ValueError('use a fresh directory; do not overwrite evidence')
    manifest=make_manifest();write(directory/'manifest.json',manifest)
    receipt={'schema':'rf2c_berger_diagnostic_v1','status':'IN_PROGRESS','manifest_sha256':sha(directory/'manifest.json'),
        'contract':CONTRACT,'geometry':{},'cases':{},'remaining_metrics':'NOT_RUN','full_regressions':'DEFERRED'}
    write(directory/'diagnostic.json',receipt)
    start=time.perf_counter()
    try:
        for level,n in enumerate(RESOLUTIONS):
            m=mesh(level);geometry=certify(m,level);fixture=orbit_fixture_for_mesh(m)
            receipt['geometry'][str(level)]=geometry
            for order in [4,6]:
                payload=dict(metric=CONTRACT['metric'],order=order,edge_points=fixture['edge_points'],cells=fixture['cells'])
                (directory/f'level{level}_berger_q{order}.input.json').write_text(json.dumps(payload,allow_nan=False))
            write(directory/'diagnostic.json',receipt)
            print('certified',level,geometry,flush=True)
            mesh.cache_clear()
        check_manifest(manifest)
        def calculate(key):
            source=directory/(key+'.input.json');output=directory/(key+'.output.json');t=time.perf_counter()
            subprocess.run([str(BINARY),str(source),str(output)],check=True,timeout=1800,capture_output=True)
            payload=read(source);numbers=measures(payload,CONTRACT['metric'],read(output))
            return key,dict(measures=numbers,input_sha256=sha(source),output_sha256=sha(output),
                elapsed_seconds=time.perf_counter()-t)
        with ThreadPoolExecutor(max_workers=4) as pool:
            jobs=[pool.submit(calculate,key) for key in sorted(case_keys())]
            for future in as_completed(jobs):
                key,result=future.result();receipt['cases'][key]=result
                write(directory/'diagnostic.json',receipt)
                print(key,result['measures'],flush=True)
        check_manifest(manifest)
        receipt['status']='BERGER_COMPLETE_GATE_OPEN'
    except Exception as error:
        receipt['status']='FAILED';receipt['error']=str(error);raise
    finally:
        receipt['elapsed_seconds']=time.perf_counter()-start
        write(directory/'diagnostic.json',receipt)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('directory',type=Path);a=p.parse_args();run(a.directory.resolve())
