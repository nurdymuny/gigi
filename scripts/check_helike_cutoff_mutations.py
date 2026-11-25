"""Remove bound mechanisms deliberately. Only assertion failures catch mutants."""
import contextlib
import inspect
import io
from pathlib import Path
import sys
import textwrap
import unittest
from unittest.mock import patch
import helike_cutoff_bounds as cb
import run_helike_cutoff_bounds as run
from test_helike_cutoff_bounds import Bounds
from run_helike_flux_background import write


def main():
    mutations=[
        ('radial_normalization','radial_tail','/(2*decay**(k+3))','/(decay**(k+3))','test_radial_tail_independent_gamma'),
        ('finite_exterior','radial_tail','y=decay*rho*rho','\n    if rho>30: return arb(0)\n    y=decay*rho*rho','test_infinite_tail_does_not_underflow'),
        ('inward_endpoint','endpoint',"bound=x.upper() if side=='upper' else x.lower()","bound=x.mid()",'test_outward_endpoint_serialization'),
        ('complex_part','polynomial_bounds','c2=re*re+im*im','c2=re*re','test_complex_polynomial_majorant_and_normalization'),
        ('hermite_lowering','polynomial_bounds','arb(int(2*n[j])).sqrt()','arb(0)','test_complex_polynomial_majorant_and_normalization'),
        ('plateau_condition','plateau','if dmin>=2*Ds and nmax<=R:','if True:','test_plateau_whole_ball_and_refusal'),
        ('density_derivative','derivative_bounds','+POWERS[i]*aP/(2*rho)','+0','test_derivative_density_gaussian_and_inverse'),
        ('inverse_chart','derivative_bounds','inv=matrix.inv()','inv=matrix','test_derivative_density_gaussian_and_inverse'),
        ('gaussian_derivative','derivative_bounds','*(aP+b[j]/rho**2)','*(b[j]/rho**2)','test_derivative_density_gaussian_and_inverse'),
        ('cutoff_mixed_term','cutoff_constants','+128*ga*gb','+0','test_cutoff_constants_keep_mixed_term'),
        ('metric_cross_terms','metric_tail','total+=aD[i]*aD[j]*F','total+=(1 if i==j else 0)*aD[i]*aD[j]*F','test_metric_tail_keeps_ordered_terms_and_exponent'),
        ('metric_exponential','metric_tail','exponent+=sum((v*v for v in beta),arb(0))/(4*EPSILON)','exponent+=0','test_metric_tail_keeps_ordered_terms_and_exponent'),
        ('gradient_commutator','combine','Geta*gradient_sq.sqrt()','arb(0)','test_combination_keeps_both_terms_and_norm'),
        ('laplacian_commutator','combine','Leta*mass.sqrt()/2','arb(0)','test_combination_keeps_both_terms_and_norm'),
        ('normalization','combine','tail_sq=mass/norm.lower()','tail_sq=mass','test_combination_keeps_both_terms_and_norm'),
        ('looser_threshold','classify',"arb('1e-20')","arb('1e-12')",'test_status_is_only_partial_and_thresholds_fixed'),
        ('full_certificate','classify','certified=False','certified=True','test_status_is_only_partial_and_thresholds_fixed'),
        ('missing_controls','finalize',"if len(controls)!=4 or {(r['g2'],r['Mq']) for r in controls}!={(g,m) for g in (.004,.002) for m in (6,8)}:",
         'if False:','test_finalization_requires_all_controls_and_verified_flags'),
        ('forged_pass','finalize',"if state['cutoff_tail_gate_passed'] is not passed:",
         'if False:','test_finalization_requires_all_controls_and_verified_flags'),
        ('stale_pass','failure',"cutoff_tail_gate_passed=False","cutoff_tail_gate_passed=True",'test_failure_clears_stale_pass')]
    rows=[]
    for name,func,old,new,test in mutations:
        owner=run if func in ('finalize','failure') else cb
        source=textwrap.dedent(inspect.getsource(getattr(owner,func)))
        if source.count(old)!=1: raise ValueError('nonunique mutation '+name)
        namespace=dict(vars(owner)); exec(source.replace(old,new),namespace)
        stream=io.StringIO()
        with patch.object(owner,func,namespace[func]),contextlib.redirect_stdout(stream),contextlib.redirect_stderr(stream):
            result=unittest.TextTestRunner(stream=stream,verbosity=2).run(unittest.TestSuite([Bounds(test)]))
        caught=bool(result.failures) and not result.errors
        rows.append(dict(name=name,caught_by_assertion=caught,failures=len(result.failures),errors=len(result.errors),log=stream.getvalue()))
        print(name,'CAUGHT' if caught else 'GATE FAILURE',flush=True)
    report=dict(all_caught=all(r['caught_by_assertion'] for r in rows),mutants=rows)
    write(Path(sys.argv[1]),report)
    return 0 if report['all_caught'] else 1


if __name__=='__main__': sys.exit(main())
