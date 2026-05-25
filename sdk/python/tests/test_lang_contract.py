"""
Contract tests for gigi.lang skeleton.

These tests pin the SHAPE that GIGI_LANG_SPEC.md (v0.1.1) promises —
error hierarchy, dataclass defaults, enum values, deferred top-level
export. They fire if silent drift happens during implementation
(a default value changed, an error class renamed, the top-level
export added prematurely).

They do NOT test implementations. All methods on GigiLang raise
NotImplementedError by design; that's the v0.0 skeleton contract.

Run with:
    pytest sdk/python/tests/test_lang_contract.py -v
"""

from __future__ import annotations

import pytest

from gigi.client import GigiError
from gigi.lang import (
    ErrorCategory,
    FiberResponse,
    GigiLang,
    GigiLangError,
    GQLExecutionError,
    LicenseRequiredError,
    TranslationError,
    TranslationResult,
)


# ── Error hierarchy ──────────────────────────────────────────────────────────


def test_gigilang_error_subclasses_gigi_error():
    """GigiLangError must root in the existing GigiError hierarchy."""
    assert issubclass(GigiLangError, GigiError)


def test_specific_errors_subclass_gigilang_error():
    """All specific error types must subclass GigiLangError."""
    assert issubclass(TranslationError, GigiLangError)
    assert issubclass(GQLExecutionError, GigiLangError)
    assert issubclass(LicenseRequiredError, GigiLangError)


# ── Dataclass defaults ───────────────────────────────────────────────────────


def test_fiber_response_defaults():
    """Spec §6 — FiberResponse defaults: fiber={}, format='dhoom', raw=None."""
    r = FiberResponse(data=None)
    assert r.fiber == {}
    assert r.format == "dhoom"
    assert r.raw is None


def test_translation_result_defaults():
    """Spec §6 — TranslationResult defaults: alternates=[], confidence=1.0, notes=None."""
    t = TranslationResult(gql="{ x }")
    assert t.alternates == []
    assert t.confidence == 1.0
    assert t.notes is None


# ── ErrorCategory enum stability ─────────────────────────────────────────────


def test_error_category_values_stable():
    """Spec §8a Q7 — enum string values are part of the wire contract."""
    assert ErrorCategory.TRANSLATION.value == "translation"
    assert ErrorCategory.GQL.value == "gql"
    assert ErrorCategory.TRANSPORT.value == "transport"
    assert ErrorCategory.INTEGRALITY.value == "integrality"
    assert ErrorCategory.QUANTUM.value == "quantum"
    assert ErrorCategory.AUTH.value == "auth"
    assert ErrorCategory.LICENSE_REQUIRED.value == "license_required"
    assert ErrorCategory.NOT_FOUND.value == "not_found"
    assert ErrorCategory.SERVER.value == "server"
    assert ErrorCategory.UNKNOWN.value == "unknown"


# ── Skeleton state ───────────────────────────────────────────────────────────


def test_gigilang_init_raises_with_spec_pointer():
    """Spec §11 — skeleton raises NotImplementedError pointing back to the spec."""
    with pytest.raises(NotImplementedError) as exc_info:
        GigiLang(url="http://test.local")
    assert "GIGI_LANG_SPEC" in str(exc_info.value)


def test_gigilang_not_exported_at_top_level():
    """Spec §11 — top-level gigi package must NOT expose GigiLang until implementation."""
    import gigi
    assert not hasattr(gigi, "GigiLang"), (
        "GigiLang must not be exported from top-level gigi package "
        "while the skeleton is unimplemented. See lang.py 'Status' section "
        "and GIGI_LANG_SPEC.md §11 for when the export becomes appropriate."
    )
