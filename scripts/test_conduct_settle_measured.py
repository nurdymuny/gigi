import sys
from pathlib import Path
import unittest
import hashlib
import io
import json
import types

sys.path.insert(0, str(Path(__file__).resolve().parents[1]/"e2e/probes"))
import conduct_settle_measured as measured

def fixture(times=(0, .1, .2)):
    rows=["Resistance (ohms),,time (s),square wave (V),Vcapacitor (V)",
          f"1000,,{times[0]},1,0",f",,{times[1]},1,.1",
          f"Capacitance (Farads),,{times[2]},1,.2","1e-6,,,,"]
    return "\n".join(rows)

class MeasuredTests(unittest.TestCase):
    def test_valid_scope_preserves_measurements(self):
        t,u,v=measured.parse_scope(fixture(),1000,1e-6)
        self.assertEqual(list(t),[0,.1,.2])
        self.assertEqual(list(u),[1,1,1])
        self.assertEqual(list(v),[0,.1,.2])
    def test_duplicate_time_refuses_by_name(self):
        with self.assertRaisesRegex(ValueError,"strictly increasing"):
            measured.parse_scope(fixture((0,0,.2)),1000,1e-6)
    def test_nonfinite_refuses_by_name(self):
        with self.assertRaisesRegex(ValueError,"finite"):
            measured.parse_scope(fixture().replace(",1,.1",",1,nan"),1000,1e-6)
    def test_component_mismatch_refuses(self):
        with self.assertRaisesRegex(ValueError,"component"):
            measured.parse_scope(fixture(),2000,1e-6)
    def test_partial_row_refuses(self):
        with self.assertRaisesRegex(ValueError,"incomplete"):
            measured.parse_scope(fixture().replace(",1,.1",",1,"),1000,1e-6)
    def test_published_short_headers_require_explicit_mapping(self):
        text=fixture().replace('time (s),square wave (V),Vcapacitor (V)', 'time,source voltage,Vcapacitor')
        with self.assertRaisesRegex(ValueError, 'header'):
            measured.parse_scope(text,1000,1e-6)
        t,u,v=measured.parse_scope(text,1000,1e-6,headers=('time','source voltage','Vcapacitor'))
        self.assertEqual(list(t),[0,.1,.2])
        self.assertEqual(list(v),[0,.1,.2])
    def test_source_event_is_midpoint(self):
        phases=measured.source_phases([0,1,2,3,4],[0,0,2,2,2])
        self.assertEqual(phases,[dict(start=0.,end=1.5,source=0.,event_uncertainty=0.),
                                 dict(start=1.5,end=4.,source=2.,event_uncertainty=.5)])
    def test_false_crossing_uses_entire_remaining_suffix(self):
        self.assertEqual(measured.observed_settling(list(range(8)),[0,2,0,0,0,0,0,0],1),2)
    def test_short_followup_is_censored(self):
        self.assertIsNone(measured.observed_settling([0,1,2,3],[0,0,0,0],1))
    def test_late_departure_is_censored(self):
        self.assertIsNone(measured.observed_settling(list(range(6)),[0,0,0,0,0,2],1))
    def test_missing_calibration_cannot_be_rescued_by_perfect_fit(self):
        self.assertFalse(measured.hardware_qualified({},True))
    def test_failed_prediction_is_not_qualified(self):
        self.assertFalse(measured.hardware_qualified({k:True for k in measured.REQUIRED_EVIDENCE},False))

def audit(output):
    global measured
    original = measured
    source = Path(measured.__file__).read_text(encoding='utf-8')
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(
        unittest.defaultTestLoader.loadTestsFromTestCase(MeasuredTests))
    assert result.wasSuccessful(), stream.getvalue()
    mutations = [
        ('time_order', 'if np.any(np.diff(t) <= 0):', 'if False:', 'test_duplicate_time_refuses_by_name'),
        ('settling_suffix', 'suffix = np.logical_and.accumulate(good[::-1])[::-1]', 'suffix = good',
         'test_false_crossing_uses_entire_remaining_suffix'),
    ]
    receipts = []
    try:
        for name, before, after, test in mutations:
            assert before in source
            changed = source.replace(before, after, 1)
            module = types.ModuleType('measured_mutant')
            module.__file__ = original.__file__
            exec(compile(changed, original.__file__, 'exec'), module.__dict__)
            measured = module
            log = io.StringIO()
            failure = unittest.TextTestRunner(stream=log, verbosity=2).run(MeasuredTests(test))
            assert len(failure.failures) == 1 and not failure.errors, log.getvalue()
            receipts.append(dict(mechanism=name, caught=True, log=log.getvalue()))
    finally:
        measured = original
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(dict(tests_passed=result.testsRun, log=stream.getvalue(), mutations=receipts,
        harness_sha256=hashlib.sha256(source.encode()).hexdigest()), indent=2), encoding='utf-8')
    print(f'{result.testsRun} tests passed; {len(receipts)} mechanism removals caught; {output}')


if __name__=='__main__':
    if len(sys.argv) == 3 and sys.argv[1] == '--audit':
        audit(Path(sys.argv[2]))
    else:
        unittest.main()
