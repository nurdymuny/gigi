"""Targeted mathematical and evidence removals; each must fail by assertion, never an unrelated error."""
import inspect
import io
import types
import unittest
from unittest.mock import patch

import ym_helike_j0 as j
import run_ym_helike_sectors as r
import test_sector_review as tests
import check_ym_helike_j0_mutations as m0
import check_ym_helike_j2_mutations as m2
import test_ym_helike_j0 as t0
import test_ym_helike_j2 as t2


def test_result(name,module,test):
    stream=io.StringIO()
    result=unittest.TextTestRunner(stream=stream,verbosity=2).run(unittest.defaultTestLoader.loadTestsFromName(test,module))
    return dict(mutation=name,test=test,assertion_rejected=bool(result.failures) and not result.errors,
                failures=len(result.failures),errors=len(result.errors),output=stream.getvalue())


def run():
    out=[]
    targets0=('GeneratorIdentities.test_laplacians','GeneratorIdentities.test_gradient_inner_products',
              'GeneratorIdentities.test_gradient_inner_products','GeneratorIdentities.test_gradient_inner_products',
              'Spectrum.test_matches_nine_variable_gpu_controls','OrderingInputs.test_temple_quantities_and_labels')
    targets2=('Tables.test_laplacians','Tables.test_directional_derivatives','Tables.test_directional_derivatives',
              'Tables.test_directional_derivatives','Tables.test_contractions','Tables.test_contractions',
              'Spectrum.test_branch_carries_measured_tensor_residual')
    for mutations,module,targets in ((m0.MUTANTS,t0,targets0),(m2.MUTANTS,t2,targets2)):
        for (name,apply),test in zip(mutations.items(),targets):
            j.moment.cache_clear(); undo=apply()
            try:
                out.append(test_result(name,module,test))
            finally:
                undo(); j.moment.cache_clear()
    changes=(
        ('omit_projected_residual',j,'residuals_from_matrices','+(projected.T*projected)[0]','+0','ResidualTests.test_projected_eigensolver_error_is_part_of_full_residual'),
        ('allow_invalid_exterior',j,'finish','exterior_min < -1e-12','False','ResidualTests.test_invalid_unreported_direction_refused'),
        ('loosen_channel_allowance',r,'make_row','min(gs,gt)','max(gs,gt)','RunnerTests.test_common_allowance_catches_false_channel_pass'),
        ('overwrite_evidence',r,'initialize','exist_ok=False','exist_ok=True','RunnerTests.test_existing_evidence_cannot_be_overwritten'),
        ('skip_residual_replay',r,'replay_solution',"np.testing.assert_allclose(residual,a['R'],atol=1e-14,rtol=1e-10)",'pass','RunnerTests.test_saved_matrices_reject_forged_residuals'),
        ('leave_stale_status',r,'verify_run',"dict(status='FAIL',error=str(exc)","dict(status='PASS',error=str(exc)",'RunnerTests.test_failed_receipt_replaces_old_pass'),
    )
    for name,module,fn,old,new,test in changes:
        source=inspect.getsource(getattr(module,fn))
        if old not in source:
            raise RuntimeError('mutation target drift: '+name)
        ns=dict(module.__dict__); exec(compile(source.replace(old,new),'<mutation:'+name+'>','exec'),ns)
        f=ns[fn]; mutant=types.FunctionType(f.__code__,module.__dict__,f.__name__,f.__defaults__,f.__closure__)
        with patch.object(module,fn,mutant):
            out.append(test_result(name,tests,test))
    return out
