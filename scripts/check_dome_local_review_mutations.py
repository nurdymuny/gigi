"""Require assertion-based rejection when the reviewed safeguards are removed."""
import inspect
import io
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch
import ym_helike_dome_truncation as dm
import test_ym_helike_dome_truncation as tests
import test_ym_helike_second_vacuum as local
import test_dome_local_review as audit


def main():
    mutations=[
        ('negative_tail',dm,'checked_remainder','if raw < -tol:','if False:',tests.TruncationDiagnostic,'test_negative_projected_tail_refuses'),
        ('order_direction',dm,'validate','not 0 <= k <= K_ref','k < 0',tests.TruncationDiagnostic,'test_reversed_orders_refuse'),
        ('absolute_roundoff',dm,'checked_remainder','max(raw, mp.mpf(0))','abs(raw)',audit.AuditTests,'test_tiny_negative_is_disclosed_and_zero_not_absolute'),
        ('drop_exterior_projection',dm,'_rows','projected = mp.fsum(x*x for x in y)','projected = mp.mpf(0)',tests.TruncationDiagnostic,'test_gaussian_moment_oracle'),
        ('locking_sign',local,'colour_space_coupling','(-sum(T @','(sum(T @',local.SecondVacuum,'test_second_vacuum_hessian_is_colour_space_coupling'),
        ('failure_receipt',audit.run,'verify',"shared.write(out/'verification.json',dict(status='FAIL',error=str(exc)))","pass",audit.AuditTests,'test_empty_complete_receipt_clears_stale_pass'),
        ('cancellation_disclosure',dm,'replay_batch','if not mp.isfinite(actual) or abs(actual-expected)>tol:','if False:',audit.AuditTests,'test_changed_cancellation_disclosure_rejected'),
        ('relative_precision',audit.run,'precision','da<=1e-12 and dr<=1e-8','da<=1e-12 or dr<=1e-8',audit.AuditTests,'test_precision_checks_small_fractions_relatively'),
    ]
    results=[]
    for name,module,fn,old,new,cls,test in mutations:
        source=inspect.getsource(getattr(module,fn))
        if old not in source:raise ValueError('mutation target changed: '+name)
        namespace=dict(module.__dict__)
        exec(compile(source.replace(old,new),'<mutation>','exec'),namespace)
        log=io.StringIO()
        with patch.object(module,fn,namespace[fn]):
            r=unittest.TextTestRunner(stream=log).run(cls(test))
        if not r.failures or r.errors:raise AssertionError('mutation not rejected by assertion: '+name+'\n'+log.getvalue())
        results.append(dict(name=name,assertion_rejected=True,output=log.getvalue()))
    Path(sys.argv[1]).write_text(json.dumps(results,indent=2),encoding='utf-8')
    print(len(results),'deliberate defects rejected by assertions')


if __name__=='__main__':main()
