"""Each interface defect must be caught by an assertion, not an incidental error."""
import contextlib
import io
import json
from pathlib import Path
import sys
import unittest
import helike_qspace_boundary as qb
import test_helike_qspace_boundary as tests

MUTANTS=[
    ('orientation_erased', [('q[2]@e2,a,','abs(q[2]@e2),a,')],
     tests.Coordinates,'test_both_orientations_and_regular_rank_two'),
    ('density_jacobian_wrong', [('8*r0**3*r1**2','8*r0**3*r1**2*(1+w*w)')],
     tests.Coordinates,'test_agreement_with_existing_positive_chart'),
    ('metric_replaced_by_flat', [('B=(K*K.T).applyfunc(sp.simplify)','B=sp.eye(6)')],
     tests.Coordinates,'test_metric_from_independent_cartesian_derivatives'),
    ('density_drift_removed', [('LOG_DENSITY_GRADIENT = (3,2,0,0,0,0)','LOG_DENSITY_GRADIENT = (0,0,0,0,0,0)')],
     tests.Operator,'test_reduced_laplacian_matches_cartesian_at_and_across_interface'),
    ('signed_cubic_replaced_by_absolute', [('-g*om.sum()*d','-g*om.sum()*abs(d)')],
     tests.Operator,'test_signed_flux_cubic_at_same_control_backgrounds'),
    ('value_jump_ignored', [('value_jump<=MATCH_TOLERANCE and conormal_sum<=MATCH_TOLERANCE','conormal_sum<=MATCH_TOLERANCE')],
     tests.Matching,'test_jump_rejected_even_when_fluxes_match'),
    ('half_normal_sign_reversed', [('abs(positive_conormal+negative_conormal)','abs(positive_conormal-negative_conormal)')],
     tests.Matching,'test_cusp_and_wrong_half_normal_sign_rejected'),
    ('dirichlet_wall_imposed', [('matches=value_jump<=MATCH_TOLERANCE','matches=abs(plus_value)<=MATCH_TOLERANCE and value_jump<=MATCH_TOLERANCE')],
     tests.Matching,'test_nonzero_value_and_derivative_are_allowed'),
    ('neumann_wall_imposed', [('matches=value_jump<=MATCH_TOLERANCE','matches=abs(positive_conormal)<=MATCH_TOLERANCE and value_jump<=MATCH_TOLERANCE')],
     tests.Matching,'test_complex_current_crosses_not_reflects'),
]


def main():
    original=qb.__dict__.copy(); source=Path(qb.__file__).read_text(encoding='utf-8')
    report={}
    for name,replacements,cls,test in MUTANTS:
        code=source
        for old,new in replacements:
            if code.count(old)!=1: raise AssertionError(f'mutation anchor drift: {name}')
            code=code.replace(old,new,1)
        stream=io.StringIO()
        try:
            exec(compile(code,qb.__file__,'exec'),qb.__dict__)
            with contextlib.redirect_stdout(stream),contextlib.redirect_stderr(stream):
                suite=unittest.TestSuite([cls(test)])
                result=unittest.TextTestRunner(stream=stream).run(suite)
            report[name]=dict(failures=len(result.failures),errors=len(result.errors),
                              caught_by_assertion=bool(result.failures) and not result.errors,log=stream.getvalue())
        finally:
            qb.__dict__.clear(); qb.__dict__.update(original)
        print(name,{k:v for k,v in report[name].items() if k!='log'},flush=True)
    report['all_caught']=all(r['caught_by_assertion'] for r in report.values())
    if len(sys.argv)>1: Path(sys.argv[1]).write_text(json.dumps(report,indent=2),encoding='utf-8')
    return 0 if report['all_caught'] else 1


if __name__=='__main__': sys.exit(main())
