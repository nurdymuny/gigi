"""Require assertion failures when dome execution protections are removed."""
import inspect
import io
import json
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

import run_ym_helike_dome as run
import test_dome_execution as tests


def main():
    mutations = [
        ("truncation_verdict", "decide", 'all(v for k, v in gates.items() if k != "D04")',
         'all(gates.values())', "test_truncation_has_its_registered_verdict"),
        ("complete_primary", "decide", 'len(primary) != expected or ', '',
         "test_incomplete_primary_cannot_be_supported"),
        ("all_domes_repeated", "repeated_dome_sizes", 'list(design["dome_sizes"])',
         '[design["primary_dome_size"]]', "test_all_top_cutoff_domes_are_repeated"),
        ("receipt_contract", "verify", 'verify_contract(rec)', 'pass',
         "test_empty_complete_receipt_rejected_and_stale_pass_cleared"),
    ]
    results = []
    for name,fn,old,new,test in mutations:
        source = inspect.getsource(getattr(run,fn))
        if old not in source:
            raise ValueError("mutation target changed")
        namespace = dict(run.__dict__)
        exec(compile(source.replace(old,new),"<dome-mutation>","exec"),namespace)
        log = io.StringIO()
        with patch.object(run,fn,namespace[fn]):
            result = unittest.TextTestRunner(stream=log).run(tests.ExecutionTests(test))
        if not result.failures or result.errors:
            raise AssertionError("mutation not rejected by assertion: " + name)
        results.append(dict(name=name,assertion_rejected=True,output=log.getvalue()))
    Path(sys.argv[1]).write_text(json.dumps(results,indent=2),encoding="utf-8")
    print("4 execution mutations rejected by assertion")


if __name__ == "__main__":
    main()
