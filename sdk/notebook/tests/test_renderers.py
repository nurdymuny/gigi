"""Pure-Python unit tests for the renderer + dispatch helpers.

Doesn't spin up a Jupyter kernel — just tests the static helpers on
``GigiKernel`` that don't need the kernel runtime.
"""

import json

import pytest

# Skip if ipykernel isn't available — these tests need it for the import.
ipykernel = pytest.importorskip("ipykernel")

from gigi_notebook.kernel import GigiKernel


# ── _render_commutator ────────────────────────────────────────────────────


def test_render_commutator_smooth_regime_shape():
    response = {
        "forward": [0.4469, 0.5531],
        "backward": [0.5531, 0.4469],
        "tv": 0.1062,
        "hellinger": 0.0752,
        "kl": {"kind": "finite", "value": 0.0327},
        "regime": "smooth",
    }
    rendered = GigiKernel._render_commutator(response)
    assert "text/plain" in rendered
    assert "application/json" in rendered

    text = rendered["text/plain"]
    # Should contain the headline regime.
    assert "smooth" in text
    # Should contain TV / Hel / KL labels.
    assert "TV" in text
    assert "Hel" in text
    assert "KL" in text
    # KL should show as bits.
    assert "0.032700 bits" in text or "0.0327" in text
    # Should print both arms.
    assert "forward" in text
    assert "backward" in text


def test_render_commutator_sofic_regime_shows_divergent():
    response = {
        "forward": [0.0, 1.0],
        "backward": [1.0, 0.0],
        "tv": 1.0,
        "hellinger": 1.0,
        "kl": {"kind": "divergent"},
        "regime": "sofic",
    }
    rendered = GigiKernel._render_commutator(response)
    text = rendered["text/plain"]
    assert "sofic" in text
    assert "divergent" in text
    # TV saturates at 1.
    assert "1.000000" in text


def test_render_commutator_handles_missing_fields_gracefully():
    response = {
        "regime": "borderline",
        # Missing tv / hellinger / kl / forward / backward — should NOT crash.
    }
    rendered = GigiKernel._render_commutator(response)
    text = rendered["text/plain"]
    assert "borderline" in text


# ── _render_json ──────────────────────────────────────────────────────────


def test_render_json_pretty_prints():
    rendered = GigiKernel._render_json({"a": 1, "b": [2, 3]})
    parsed = json.loads(rendered["text/plain"])
    assert parsed == {"a": 1, "b": [2, 3]}


# ── _apply_env_line ───────────────────────────────────────────────────────


def test_apply_env_line_updates_gigi_url(monkeypatch):
    monkeypatch.setenv("GIGI_URL", "http://localhost:3142")
    monkeypatch.setenv("GIGI_API_KEY", "")
    k = GigiKernel()
    assert k._gigi_url == "http://localhost:3142"
    k._apply_env_line("GIGI_URL=https://example.org")
    assert k._gigi_url == "https://example.org"


def test_apply_env_line_updates_api_key(monkeypatch):
    monkeypatch.setenv("GIGI_API_KEY", "")
    k = GigiKernel()
    assert k._api_key == ""
    k._apply_env_line("GIGI_API_KEY=secret123")
    assert k._api_key == "secret123"


def test_apply_env_line_rejects_malformed():
    k = GigiKernel()
    with pytest.raises(ValueError, match="expected KEY=value"):
        k._apply_env_line("not-a-pair")
