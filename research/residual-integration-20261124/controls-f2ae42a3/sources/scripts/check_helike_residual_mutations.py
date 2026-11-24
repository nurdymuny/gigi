"""Deliberate removal gates: assertion failures count, execution errors do not."""
import contextlib
import inspect
import io
from pathlib import Path
import sys
import textwrap
import unittest
from unittest.mock import patch
import helike_residual_moments as rm
from test_helike_residual_moments import Moments
from run_helike_flux_background import write


def main():
    mutations=[
        ('translation',rm,'norm_squared','mu=total/(2*space.w)','mu=np.zeros_like(total)','test_independent_unshifted_quadrature'),
        ('square_prefactor',rm,'norm_squared','*np.exp(exponent)','*1.','test_completed_square_prefactor'),
        ('conjugation',rm,'norm_squared','left.conj()*right','left*right','test_complex_norm_and_cross_terms'),
        ('cross_terms',rm,'norm_squared','1 if i==j else 2','1 if i==j else 0','test_complex_norm_and_cross_terms'),
        ('density_drift',rm,'build_residual','-.5*c*pc.MEASURE_POWERS[i]','0.','test_operator_matches_cartesian_and_reduced'),
        ('derivative_raise',rm.Space,'op','-np.sqrt(w*n/2)','np.zeros_like(n)','test_derivative_keeps_raising_part'),
        ('truncation',rm.Space,'op',"kind!='derivative' and np.any(C[self.top]!=0)",'False','test_nonzero_truncation_refuses'),
        ('false_certificate',rm,'numerical_result','certified=False','certified=True','test_numerical_moments_never_certify')]
    rows=[]
    for name,owner,func,old,new,test in mutations:
        source=textwrap.dedent(inspect.getsource(getattr(owner,func)))
        if source.count(old)!=1: raise ValueError(f'mutation anchor not unique: {name}')
        namespace=dict(vars(rm)); exec(source.replace(old,new),namespace)
        stream=io.StringIO()
        with patch.object(owner,func,namespace[func]),contextlib.redirect_stdout(stream),contextlib.redirect_stderr(stream):
            result=unittest.TextTestRunner(stream=stream,verbosity=2).run(unittest.TestSuite([Moments(test)]))
        caught=bool(result.failures) and not result.errors
        rows.append(dict(name=name,caught_by_assertion=caught,failures=len(result.failures),errors=len(result.errors),log=stream.getvalue()))
        print(name,'CAUGHT' if caught else 'GATE FAILURE',flush=True)
    report=dict(all_caught=all(r['caught_by_assertion'] for r in rows),mutants=rows)
    write(Path(sys.argv[1]),report)
    return 0 if report['all_caught'] else 1


if __name__=='__main__': sys.exit(main())
