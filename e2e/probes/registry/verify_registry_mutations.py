"""Deliberately remove safeguards and require the corresponding test to fail.

Patches are confined to this process; source files and databases are untouched.
An exception in the test is not counted as a detected mutant: an assertion must
fail. Run the ordinary test suite first to establish its unmutated baseline.
"""

import unittest
from unittest.mock import patch

import registry_bootstrap as db
import registry_paths as paths
import registry_review as review
from test_registry import RegistryTests


def unchecked_write(name, record):
    db.call(f"/v1/bundles/{name}/insert", {"records": [record]})


def warning_free_view(rid):
    result = ORIGINAL_VIEW(rid)
    return dict(result, needs_review=False, flags=[])


ORIGINAL_VIEW = paths.view


def main():
    mutations = [
        ("promotion performs no write", review, "promote", review.requirement_met,
         "test_promotion_changes_stored_altitude"),
        ("evidence requirements removed", review, "requirement_met", lambda *a: (True, "unchecked"),
         "test_missing_sources_refuse_and_flag"),
        ("locator requirement removed", review, "requirement_met", lambda *a: (True, "unchecked"),
         "test_blank_locators_refuse_and_flag"),
        ("promotion ignores failed writes", review, "write_verified", unchecked_write,
         "test_failed_promotion_write_cannot_succeed"),
        ("promotion ignores no-op writes", review, "write_verified", unchecked_write,
         "test_noop_promotion_write_cannot_succeed"),
        ("reviews ignore failed writes", paths, "write_verified", unchecked_write,
         "test_review_write_failure_cannot_succeed"),
        ("randomized review ID restored", paths, "review_id", lambda sig: "PR-" + str(abs(hash(sig)) % 10**8),
         "test_review_key_stable_across_processes"),
        ("arbitrary first review restored", paths, "_review",
         lambda sig: db.query("research_path_reviews", [db.eq("path_signature", sig)])[0],
         "test_replacement_ruling_wins_over_legacy"),
        ("path evidence warnings removed", paths, "view", warning_free_view,
         "test_path_retraction_warning_survives_hydration"),
        ("string inequality treated as proof", paths, "_comparison",
         lambda a, b: "contradiction" if a["value"] != b["value"] else "equal",
         "test_symbolic_inequality_is_not_a_contradiction"),
        ("ruling validation removed", paths, "_validate_ruling", lambda *a: None,
         "test_invalid_rulings_refused"),
        ("path validation removed", paths, "_validate_path", lambda *a: None,
         "test_invalid_paths_refused"),
    ]
    failed = []
    for label, module, attribute, replacement, test_name in mutations:
        with patch.object(module, attribute, replacement):
            result = unittest.TestResult()
            RegistryTests(test_name).run(result)
        detected = bool(result.failures) and not result.errors
        print(f"[{'PASS' if detected else 'FAIL'}] removed safeguard detected: {label}")
        if not detected:
            failed.append((label, result.errors))
    if failed:
        raise SystemExit(f"Mutants escaped or tests errored: {failed}")
    print(f"All {len(mutations)} deliberately broken variants were caught by assertions.")


if __name__ == "__main__":
    main()
