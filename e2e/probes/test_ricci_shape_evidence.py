import tempfile
from pathlib import Path
import unittest
from ricci_shape_evidence import numerical_gate,verify


class GateTests(unittest.TestCase):
    def rows(self, errors, relative=.02):
        return [{'full_mass_error':e,'relative_error':relative} for e in errors]

    def test_planted_convergence(self):
        r=numerical_gate(self.rows([1.,.5,.25,.125]),[1.,.5,.25,.125],[.001]*4,[1e-12]*4)
        self.assertTrue(r['pass'])
        self.assertEqual(len(r['observed_orders']),3)

    def test_each_frozen_failure_is_separate(self):
        fixtures=[([1.,.5,.5,.5],.02,[0.]*4,[0.]*4),
            ([1.,.5,.25,.125],.05,[0.]*4,[0.]*4),
            ([1.,.5,.25,.125],.02,[1.]*4,[0.]*4),
            ([1.,.5,.25,.125],.02,[0.]*4,[1.]*4),
            ([1.,.5,0.,0.],.0,[0.]*4,[0.]*4)]
        for errors,relative,q,linear in fixtures:
            with self.subTest(errors=errors,relative=relative,q=q,linear=linear):
                self.assertFalse(numerical_gate(self.rows(errors,relative),[1.,.5,.25,.125],q,linear)['pass'])

    def test_partial_nonfinite_and_wrong_spacing_refuse(self):
        for errors,h in [([1.,.5,.25],[1.,.5,.25]),([1.,.5,float('nan'),.1],[1.,.5,.25,.125]),
            ([1.,.5,.25,.1],[1.,.5,.6,.1])]:
            with self.subTest(errors=errors,h=h),self.assertRaises(ValueError):
                numerical_gate(self.rows(errors),h,[0.]*len(h),[0.]*len(h))

    def test_no_stale_pass_after_bad_or_missing_evidence(self):
        for raw in [None,'{','{}']:
            with self.subTest(raw=raw),tempfile.TemporaryDirectory() as d:
                p=Path(d)
                for name in ['gate.PASS.json','VERIFIED_DIAGNOSTIC.json']:(p/name).write_text('{}')
                if raw is not None:(p/'diagnostic.json').write_text(raw)
                with self.assertRaises((ValueError,FileNotFoundError)):verify(p)
                self.assertFalse((p/'gate.PASS.json').exists())
                self.assertFalse((p/'VERIFIED_DIAGNOSTIC.json').exists())


if __name__=='__main__':unittest.main()
