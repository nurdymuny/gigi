"""Focused in-memory mechanism removal; restore every kernel after its test run."""
import argparse
import io
import json
from pathlib import Path
import unittest
import numpy as np
import helike_m8_geometry as gm
import helike_carrier_geometry_probe as probe
from test_helike_carrier_geometry_probe import GeometryProbe


def main():
    ap=argparse.ArgumentParser(); ap.add_argument('out',type=Path); args=ap.parse_args(); args.out.mkdir(exist_ok=True,parents=True)
    original=gm.Core.analyse; source=probe.slabs
    results=[]
    for name in ('baseline','drop_derivative','drop_ground_weight','local_cut_normalization'):
        try:
            if name=='drop_derivative':
                def bad_slabs(*a,**kw):
                    for W,x,p,dp in source(*a,**kw): yield W,x,p,None if dp is None else np.zeros_like(dp)
                probe.slabs=bad_slabs
            elif name=='drop_ground_weight':
                def bad_weight(self,p0,dp0,*a,**kw): return original(self,np.ones_like(p0),np.zeros_like(dp0),*a,**kw)
                gm.Core.analyse=bad_weight
            elif name=='local_cut_normalization':
                def local(self,p0,dp0,*a,**kw):
                    kw['total_vacuum_norm']=float(np.sum(self.W*p0*p0))
                    return original(self,p0,dp0,*a,**kw)
                gm.Core.analyse=local
            stream=io.StringIO()
            result=unittest.TextTestRunner(stream=stream,verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(GeometryProbe))
            (args.out/(name+'.log')).write_text(stream.getvalue())
            passed=result.wasSuccessful() if name=='baseline' else bool(result.failures)
            results.append(dict(variant=name,tests_run=result.testsRun,assertion_failures=len(result.failures),errors=len(result.errors),gate_pass=passed))
            print(json.dumps(results[-1]),flush=True)
        finally:
            gm.Core.analyse=original; probe.slabs=source
    (args.out/'receipt.json').write_text(json.dumps(results,indent=2)+'\n')
    return 0 if all(r['gate_pass'] for r in results) else 1


if __name__=='__main__': raise SystemExit(main())
