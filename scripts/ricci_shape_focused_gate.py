"""Focused RF2C checks only. Never runs Cargo's full suites or grants release."""
import argparse
import json
import os
from pathlib import Path
import sys
import time
import unittest

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'e2e/probes'))
from monodromy_evidence import sha
from ricci_shape_experiment import BINARY,check_manifest
from ricci_lattice_evidence import read

MODULES=['test_ricci_shape_regular_mesh','test_ricci_mesh_orbits','test_ricci_shape_certificate',
    'test_ricci_shape_native','test_ricci_operator_native','test_ricci_shape_guards',
    'test_ricci_shape_evidence','test_ricci_shape_attacks']


def main(directory):
    os.environ['GIGI_RF2B_OPERATOR_BINARY']=str(BINARY)
    os.environ['GIGI_RF2C_EVIDENCE']=str(directory)
    path=directory/'FOCUSED_TESTS.json'
    receipt={'status':'IN_PROGRESS','release_gate':'OPEN','full_regressions':'DEFERRED',
        'modules':MODULES,'sources':{str(ROOT/'e2e/probes'/(name+'.py')):sha(ROOT/'e2e/probes'/(name+'.py')) for name in MODULES},
        'runner_sha256':sha(Path(__file__))}
    path.write_text(json.dumps(receipt,indent=2))
    start=time.perf_counter()
    try:
        check_manifest(read(directory/'manifest.json'))
        suite=unittest.defaultTestLoader.loadTestsFromNames(MODULES)
        result=unittest.TextTestRunner(verbosity=2).run(suite)
        receipt.update(status='PASS' if result.wasSuccessful() and not result.skipped else 'FAIL',
            tests=result.testsRun,failures=[(str(t),trace) for t,trace in result.failures],
            errors=[(str(t),trace) for t,trace in result.errors],skipped=[(str(t),reason) for t,reason in result.skipped])
        check_manifest(read(directory/'manifest.json'))
    except BaseException as error:
        receipt['status']='FAIL';receipt['error']=str(error);raise
    finally:
        receipt['elapsed_seconds']=time.perf_counter()-start
        path.write_text(json.dumps(receipt,indent=2,allow_nan=False))
    if receipt['status']!='PASS':raise SystemExit('focused checks failed; no release PASS')


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('directory',type=Path);a=p.parse_args();main(a.directory.resolve())
