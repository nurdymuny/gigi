"""In-memory mechanism removals; never edits the numerical solver or starts it."""
import contextlib
import io
import json
from pathlib import Path
import types
import unittest
from unittest.mock import patch

import test_helike_flux_basis_smoke as tests

source = Path(tests.smoke.__file__).read_text()
mutants = {
    "ignore_basis_step": ("max(steps.values()) <= STEP_TOL", "True"),
    "substitute_dressed_overlap": ('b[n]["dressed_weight"] for n in steps', 'b[n]["dressed_overlap"] for n in steps'),
    "weaken_export_threshold": ('b["competitor"]["dressed_weight"] < EXPORT_MIN', 'b["competitor"]["dressed_weight"] < 0.9'),
    "ignore_reproduction": ('if abs(q[name][key] - finite(old[name][key])) > tol:', 'if False:'),
    "ignore_residual": ('if not 0 <= finite(residual) <= SOLVER["tol"] * max(1.0, abs(finite(value))):', 'if False:'),
    "release_grid": ('"full_grid_released": False', '"full_grid_released": True'),
    "reuse_evidence": ('exist_ok=False', 'exist_ok=True'),
    "hide_execution_failure": ('event.update(status="STOPPED",', 'event.update(status="COMPLETED",'),
}
results = []
for name, (old, new) in mutants.items():
    if old not in source:
        raise RuntimeError(f"mutation target missing: {name}")
    module = types.ModuleType("smoke_mutant")
    module.__file__ = tests.smoke.__file__
    exec(compile(source.replace(old, new), module.__file__, "exec"), module.__dict__)
    stream = io.StringIO()
    with patch.object(tests, "smoke", module), contextlib.redirect_stdout(io.StringIO()):
        result = unittest.TextTestRunner(stream=stream).run(unittest.defaultTestLoader.loadTestsFromTestCase(tests.Smoke))
    killed = not result.wasSuccessful()
    results.append({"mutation": name, "rejected": killed,
                    "failing_tests": [str(t) for t, _ in result.failures + result.errors]})
print(json.dumps(results, indent=2))
raise SystemExit(0 if all(r["rejected"] for r in results) else 1)
