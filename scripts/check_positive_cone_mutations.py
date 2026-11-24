"""Mechanism removal controls for the positive-cone form; assertions must catch each defect."""
import contextlib
import io
import json
from pathlib import Path
import sys
import unittest
import helike_positive_cone as pc
import test_helike_positive_cone as tests

MUTANTS=[
    ('positive_diagonal_removed',[('r = np.exp(u[:3])','r = u[:3]')],tests.Geometry,'test_old_negative_continuation_point_stays_inside'),
    ('jacobian_measure_wrong',[('MEASURE_POWERS = np.array([3., 2., 1., 0., 0., 0.])','MEASURE_POWERS = np.zeros(6)')],tests.Geometry,'test_roundtrip_metric_and_measure'),
    ('euclidean_metric_substituted',[('B = (inv*A*inv.T).applyfunc(sp.cancel)','B = sp.eye(6)')],tests.Geometry,'test_metric_and_potential_match_independent_q_space'),
    ('source_term_sign_flipped',[('V -= sp.Float(a/2)','V += sp.Float(a/2)')],tests.Geometry,'test_metric_and_potential_match_independent_q_space'),
    ('gaussian_completion_factor_removed',[('np.exp(alpha*alpha/(4*w))','1.')],tests.Moments,'test_completed_square_against_unshifted_quadrature'),
    ('derivative_shell_removed',[('if o[j] < 0:','if o[j] < 0 or sum(o) > Mq:')],tests.Moments,'test_harmonic_single_and_double_controls'),
    ('reference_volume_factor_removed',[('logdet+u[:,:3]@np.array([4.,3.,2.])','0.*(logdet+u[:,:3]@np.array([4.,3.,2.]))')],tests.Moments,'test_reference_amplitudes_against_direct_pullback'),
    ('kinetic_measure_derivative_removed',[('- .5*MEASURE_POWERS[i]*embedding','- 0.*MEASURE_POWERS[i]*embedding')],tests.PhysicalControls,'test_same_weak_coupling_coefficients'),
]


def main():
    report={}
    original=pc.__dict__.copy()
    source=Path(pc.__file__).read_text(encoding='utf-8')
    for name,replacements,cls,test in MUTANTS:
        code=source
        for old,new in replacements:
            if code.count(old)!=1: raise AssertionError(f'mutation anchor drift: {name}')
            code=code.replace(old,new,1)
        sink=io.StringIO()
        try:
            exec(compile(code,pc.__file__,'exec'),pc.__dict__)
            with contextlib.redirect_stdout(sink),contextlib.redirect_stderr(sink):
                r=unittest.TextTestRunner(stream=sink).run(unittest.TestSuite([cls(test)]))
            report[name]=dict(failures=len(r.failures),errors=len(r.errors),
                              caught_by_assertion=bool(r.failures) and not r.errors,log=sink.getvalue())
        finally:
            pc.__dict__.clear(); pc.__dict__.update(original)
        print(name,{k:v for k,v in report[name].items() if k!='log'},flush=True)
    report['all_caught']=all(v['caught_by_assertion'] for v in report.values())
    if len(sys.argv)>1: Path(sys.argv[1]).write_text(json.dumps(report,indent=2),encoding='utf-8')
    return 0 if report['all_caught'] else 1


if __name__=='__main__': sys.exit(main())
