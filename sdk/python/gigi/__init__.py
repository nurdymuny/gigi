"""
gigi-client — Python client for the GIGI geometric database.

Exports:
    GigiClient       — synchronous REST client
    GigiSubscriber   — async WebSocket subscription client
    GigiError        — base exception
    BundleNotFound   — 404 exception
    AuthError        — 401/403 exception
"""

from .client import GigiClient, GigiError, BundleNotFound, AuthError

# The subscriber needs the optional `websockets` dependency. A user who
# only wants the REST client must not need it — importing the package
# used to raise ImportError here, which made `pip install requests` +
# copy-the-sdk workflows (and CI smoke tests) fail before the first
# request. Found by scripts/sdk_smoke.py.
try:
    from .subscriber import GigiSubscriber, SubscriptionEvent
except ImportError:  # websockets not installed — REST-only mode
    GigiSubscriber = None  # type: ignore[assignment]
    SubscriptionEvent = None  # type: ignore[assignment]

__version__ = "0.5.0"
__all__ = [
    "GigiClient",
    "GigiSubscriber",
    "SubscriptionEvent",
    "GigiError",
    "BundleNotFound",
    "AuthError",
]
