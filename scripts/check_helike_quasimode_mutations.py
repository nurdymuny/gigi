"""Assertion-caught construction mutants, with no control-spectrum run."""
import contextlib
import io
from pathlib import Path
import sys
import unittest
from unittest.mock import patch
import helike_quasimode as qm
from test_helike_quasimode import Construction
from check_helike_convergence_mutations import changed
from run_helike_flux_background import write


def main():
    mutations=[
        ('zero_extension','outside_support','return (det<=0 or det*det<=DELTA*np.linalg.det(chart.G0)',
         'return (False or det*det<=DELTA*np.linalg.det(chart.G0)',
         'test_cutoff_state_has_zero_action_off_positive_support'),
        ('mixed_shell','geometry','-2*dh*dk*mixed','','test_overlapping_shells_keep_mixed_laplacian_term'),
        ('gradient_commutator','cutoff_action','-gradient_contraction','','test_commutator_keeps_both_contributions'),
        ('laplacian_commutator','cutoff_action','-.5*lap_eta*P','','test_commutator_keeps_both_contributions'),
        ('measure_drift','geometry','drift=driftu@chart.invS.T','drift=np.zeros_like(driftu)',
         'test_full_operator_matches_independent_cartesian_differentiation'),
        ('charge_sign','geometry','velocity=vu@chart.invS.T','velocity=-vu@chart.invS.T',
         'test_generator_sign_and_metric_on_known_covariant'),
        ('complex_square','square_integral','np.abs(values)**2','values**2','test_complex_norm_and_nonfinite_refusals'),
        ('false_certificate','integration_status',"status='UNRESOLVED_INTEGRATION',certified=False",
         "status='PASS_CERTIFIED_INTERVAL',certified=True",'test_numerical_agreement_never_certifies'),
        ('projector_defect','projection_bound','(1-tail)**2-charge**2','(1-tail)**2','test_projector_bound_and_refusals')]
    rows=[]
    for name,func,old,new,test in mutations:
        stream=io.StringIO()
        with patch.object(qm,func,changed(qm,func,old,new)),contextlib.redirect_stdout(stream),contextlib.redirect_stderr(stream):
            result=unittest.TextTestRunner(stream=stream,verbosity=2).run(unittest.TestSuite([Construction(test)]))
        caught=bool(result.failures) and not result.errors
        rows.append(dict(name=name,caught_by_assertion=caught,failures=len(result.failures),errors=len(result.errors),log=stream.getvalue()))
        print(name,'CAUGHT' if caught else 'GATE FAILURE',flush=True)
    report=dict(all_caught=all(r['caught_by_assertion'] for r in rows),mutants=rows)
    write(Path(sys.argv[1]),report)
    return 0 if report['all_caught'] else 1


if __name__=='__main__': sys.exit(main())
