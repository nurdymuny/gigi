import math
from pathlib import Path
import sys
import unittest
import io
import json
import types
import hashlib
import numpy as np
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'e2e/probes'))
import settling_diagnosis as diagnostic

class DiagnosisTests(unittest.TestCase):
    def test_constant_source_analytic(self):
        t=np.linspace(0,3,21)
        for mode in ['linear','left','right']:
            np.testing.assert_allclose(diagnostic.propagate(t,2+0*t,0,.7,mode),2*(1-np.exp(-t/.7)),atol=1e-13)
    def test_ramp_analytic(self):
        t=np.linspace(0,2,17)
        np.testing.assert_allclose(diagnostic.propagate(t,t,0,.4),t-.4*(1-np.exp(-t/.4)),atol=1e-13)
    def test_hold_sides_are_not_interchangeable(self):
        self.assertEqual(diagnostic.propagate([0,1],[0,2],0,1,'left')[-1],0)
        self.assertAlmostEqual(diagnostic.propagate([0,1],[0,2],0,1,'right')[-1],2*(1-math.exp(-1)))
    def test_origin_shift(self):
        np.testing.assert_allclose(diagnostic.propagate([0,.5,1],[1,0,-1],.2,.3),
                                   diagnostic.propagate([100,100.5,101],[1,0,-1],.2,.3),atol=1e-13)
    def test_tiny_ramp_does_not_cancel(self):
        self.assertAlmostEqual(diagnostic.propagate([0,1e-10],[0,1e-10],0,1)[-1]/5e-21,1,places=9)
    def test_duplicate_clock_refuses(self):
        with self.assertRaisesRegex(ValueError,'increasing'):
            diagnostic.propagate([0,0],[1,1],0,1)
    def test_nonfinite_refuses(self):
        with self.assertRaisesRegex(ValueError,'finite'):
            diagnostic.propagate([0,1],[1,float('nan')],0,1)
    def test_tau_bounds_cover_extremes(self):
        lo,hi=diagnostic.tolerance_enclosure([0,1],[1,1],0,.5,2)
        self.assertAlmostEqual(lo[-1],1-math.exp(-.5))
        self.assertAlmostEqual(hi[-1],1-math.exp(-2))
    def test_input_bounds_cover_ramp_and_both_holds(self):
        t=[0,.1,.4,1];u=[0,2,-1,3]
        lo,hi=diagnostic.tolerance_enclosure(t,u,.5,.2,.7)
        for tau in [.2,.3,.7]:
            for mode in ['linear','left','right']:
                v=diagnostic.propagate(t,u,.5,tau,mode)
                self.assertTrue(np.all(v>=lo-1e-14) and np.all(v<=hi+1e-14))
    def test_signed_decomposition(self):
        forcing,residual=diagnostic.decompose(np.array([2.,-3.]),np.array([1.,1.]),np.array([3.,-1.]))
        np.testing.assert_array_equal(forcing,[2.,-2.])
        np.testing.assert_array_equal(residual,[-1.,-2.])
        np.testing.assert_array_equal(forcing+residual,[1.,-4.])

def audit(path):
    global diagnostic
    original=diagnostic;source=Path(original.__file__).read_text()
    log=io.StringIO()
    result=unittest.TextTestRunner(stream=log,verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(DiagnosisTests))
    assert result.wasSuccessful(),log.getvalue()
    receipts=[]
    try:
        for name,before,after,test in [
            ('source_forcing','value=result[-1]+gain*(source_value-result[-1])','value=result[-1]','test_constant_source_analytic'),
            ('tolerance_widening','-math.expm1(-dt/tau_high)','-math.expm1(-dt/tau_low)','test_tau_bounds_cover_extremes')]:
            assert before in source
            module=types.ModuleType('diagnostic_mutant');module.__file__=original.__file__
            exec(compile(source.replace(before,after,1),original.__file__,'exec'),module.__dict__)
            diagnostic=module;mutlog=io.StringIO()
            killed=unittest.TextTestRunner(stream=mutlog).run(DiagnosisTests(test))
            assert len(killed.failures)==1 and not killed.errors,mutlog.getvalue()
            receipts.append(dict(name=name,caught=True,log=mutlog.getvalue()))
    finally:diagnostic=original
    Path(path).write_text(json.dumps(dict(tests=result.testsRun,log=log.getvalue(),mutations=receipts,
        harness_sha256=hashlib.sha256(Path(original.__file__).read_bytes()).hexdigest()),indent=2))
    print(f'{result.testsRun} tests passed; {len(receipts)} mechanism removals caught')

if __name__=='__main__':
    if len(sys.argv)==3 and sys.argv[1]=='--audit':audit(sys.argv[2])
    else:unittest.main()
