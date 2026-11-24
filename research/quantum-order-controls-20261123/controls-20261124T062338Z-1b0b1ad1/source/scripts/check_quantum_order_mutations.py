"""Deliberately remove each new safeguard; require assertion failures, not crashes."""
import inspect
import io
import json
import types
import unittest
from unittest.mock import patch

import quantum_ordering as o
import run_quantum_order_controls as r
import test_quantum_ordering as t
import test_quantum_order_controls as rt


def run():
    cases = [
        ('promote_assumed_separation',o,'ordering',"'separation_verified': False","'separation_verified': True",t,
         'BoundTests.test_missing_exact_level_never_becomes_verified'),
        ('promote_conditional_order',o,'ordering',"'unconditional_order': 'ORDER_UNRESOLVED'","'unconditional_order': conditional",t,
         'BoundTests.test_missing_exact_level_never_becomes_verified'),
        ('omit_neighbor_residual',o,'_branch',"data['next_low']-data['next_residual']","data['next_low']",t,
         'BoundTests.test_next_residual_is_in_denominator'),
        ('pad_energy_with_spin_tolerance',o,'_branch',"beta = data['next_low']", "data = dict(data, residual=data['residual']+1e-6)\n    beta = data['next_low']",t,
         'BoundTests.test_spin_tolerance_does_not_change_energy_residual'),
        ('omit_spin_gate',o,'_branch',"if data['spin_error'] < 0 or data['spin_error'] > 1e-6:", 'if False:',t,
         'BoundTests.test_spin_failure_refuses_instead_of_padding_energy'),
        ('weaken_separation_guard',o,'lower_estimate','if beta <= rho + residual:','if beta <= rho:',t,
         'BoundTests.test_separation_guard_is_strict'),
        ('omit_half_decision',o,'ordering',"conditional = base if base == half else 'ORDER_UNRESOLVED'",'conditional = base',t,
         'BoundTests.test_half_separation_can_change_the_order'),
        ('omit_half_guard',o,'lower_estimate','if beta <= rho + residual:','if beta <= rho:',t,
         'BoundTests.test_half_separation_guard_cannot_be_skipped'),
        ('same_quintet_as_next_level',o,'select_levels',"by_spin[0]+by_spin[2]","by_spin[0]+[by_spin[2][0],by_spin[2][0]]",t,
         'SectorTests.test_next_means_distinct_complete_multiplet'),
        ('omit_scaled_trace',o,'full_residual','(1/(2*alpha)-alpha/2)*c','0.*c',t,
         'ResidualTests.test_full_action_matches_independent_ladders'),
        ('omit_exterior_action',o,'full_residual','gram = action.conj().T@action','action = action[pos]\n    gram = action.conj().T@action',t,
         'ResidualTests.test_full_action_matches_independent_ladders'),
        ('omit_complex_conjugation',o,'full_residual','gram = action.conj().T@action','gram = action.T@action',t,
         'ResidualTests.test_full_action_matches_independent_ladders'),
        ('mix_width_baselines',o,'width_summary','((1.,1.7),(2.,2.3))','((1.,1.7),(2.,2.))',t,
         'WidthTests.test_saved_width_baselines_are_distinct'),
        ('leave_verification_running_on_failure',r,'verify_run',"dict(status='FAIL',error=", "dict(status='RUNNING',error=",rt,
         'EvidenceTests.test_incomplete_replay_invalidates_stale_pass'),
        ('omit_current_source_check',r,'verify_run',"if receipt.get('source_hashes')!=hashes():",'if False:',rt,
         'EvidenceTests.test_source_and_snapshot_corruption_clear_pass'),
        ('omit_array_identity_check',r,'verify_run',"if c['arrays']!=expected:",'if False:',rt,
         'EvidenceTests.test_wrong_array_identity_is_refused_before_read'),
        ('omit_report_recomputation',r,'replay_case',"base.compare_report(c['levels'][name],measured,name)",'pass',rt,
         'ReplayTests.test_saved_states_and_report_are_recomputed'),
        ('omit_host_budget',r,'supervise',"if rss>base.HOST_BUDGET_BYTES or elapsed>base.TIME_BUDGET_SECONDS:",'if False:',rt,
         'SupervisorTests.test_memory_stop_is_not_complete'),
    ]
    results=[]
    for name,module,function,old,new,tests,test in cases:
        source=inspect.getsource(getattr(module,function))
        if old not in source:
            raise RuntimeError('mutation target drifted: '+name)
        namespace=dict(module.__dict__)
        exec(compile(source.replace(old,new),'<mutation:'+name+'>','exec'),namespace)
        compiled=namespace[function]
        mutant=types.FunctionType(compiled.__code__,module.__dict__,compiled.__name__,compiled.__defaults__,compiled.__closure__)
        mutant.__kwdefaults__=compiled.__kwdefaults__
        stream=io.StringIO()
        with patch.object(module,function,mutant):
            result=unittest.TextTestRunner(stream=stream,verbosity=2).run(unittest.defaultTestLoader.loadTestsFromName(test,tests))
        results.append(dict(mutation=name,test=test,assertion_rejected=bool(result.failures) and not result.errors,
                            failures=len(result.failures),errors=len(result.errors),output=stream.getvalue()))
    return results


if __name__=='__main__':
    results=run()
    print(json.dumps(results,indent=2))
    raise SystemExit(0 if all(x['assertion_rejected'] for x in results) else 1)
