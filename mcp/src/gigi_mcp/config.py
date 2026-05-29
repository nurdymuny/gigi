"""Configuration for gigi-mcp.

Reads connection settings from environment variables; caches the
constructed GigiClient so repeated tool calls share one connection.
"""

from __future__ import annotations

import os
from functools import lru_cache

from gigi.client import GigiClient


DEFAULT_GIGI_URL = "https://gigi-stream.fly.dev"
"""Public read-only GIGI instance used by default.

Lets `uvx gigi-mcp` work on first run without any setup. Users with their
own GIGI deployment override via the GIGI_URL environment variable.
"""

DEFAULT_TIMEOUT = 30
"""Request timeout in seconds. Override with GIGI_TIMEOUT env var."""


@lru_cache(maxsize=1)
def get_client() -> GigiClient:
    """Return a process-cached GigiClient configured from environment variables.

    Environment variables
    ---------------------
    GIGI_URL : str
        Base URL of the GIGI server. Defaults to the public read-only
        instance (see DEFAULT_GIGI_URL).
    GIGI_API_KEY : str, optional
        API key sent in the X-Api-Key header. Required for write operations
        and for any commercial-tier reads.
    GIGI_TIMEOUT : int, optional
        Request timeout in seconds. Defaults to 30.

    Returns
    -------
    GigiClient
        Cached client instance. Subsequent calls within the same process
        return the same object.
    """
    url = os.environ.get("GIGI_URL", DEFAULT_GIGI_URL)
    api_key = os.environ.get("GIGI_API_KEY")
    timeout = int(os.environ.get("GIGI_TIMEOUT", str(DEFAULT_TIMEOUT)))
    return GigiClient(url=url, api_key=api_key, timeout=timeout)
