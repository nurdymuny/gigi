"""Live smoke against gigi-stream.fly.dev.

Skipped unless ``GIGI_URL`` and ``GIGI_API_KEY`` are both set in env.
When set, the test reaches out to the deployed engine, posts the paper's
H5 anchor through the same path the kernel uses, and asserts the
rendered output mentions the right regime + numbers.
"""

import os

import pytest

ipykernel = pytest.importorskip("ipykernel")

from gigi_notebook.kernel import GigiKernel


GIGI_URL = os.environ.get("GIGI_URL", "")
GIGI_API_KEY = os.environ.get("GIGI_API_KEY", "")
LIVE = bool(GIGI_URL and GIGI_API_KEY)


@pytest.mark.skipif(not LIVE, reason="GIGI_URL/GIGI_API_KEY not set")
def test_live_hmm_h5_anchor_renders_smooth_smooth():
    """POST the paper's H5 reference point; expect smooth regime."""
    k = GigiKernel()
    body = """
{
  "a": {"kind": "hmm", "alpha": 0.2, "beta": 0.3, "symbol": 0},
  "b": {"kind": "hmm", "alpha": 0.2, "beta": 0.3, "symbol": 1},
  "base_belief": [0.5, 0.5]
}
"""
    rendered = k._handle_commutator(body)
    text = rendered["text/plain"]
    # Paper §6.3 H5 reference values.
    assert "smooth" in text
    assert "0.106" in text  # TV
    # KL is finite at this anchor.
    assert "bits" in text


@pytest.mark.skipif(not LIVE, reason="GIGI_URL/GIGI_API_KEY not set")
def test_live_even_process_renders_sofic_divergent():
    """POST the Even Process anchor; expect sofic + divergent KL."""
    k = GigiKernel()
    body = """
{
  "a": {"kind": "even_u0"},
  "b": {"kind": "even_u1"},
  "base_belief": [0.6666666666666666, 0.3333333333333333]
}
"""
    rendered = k._handle_commutator(body)
    text = rendered["text/plain"]
    assert "sofic" in text
    assert "divergent" in text
    # TV saturates.
    assert "1.000000" in text
