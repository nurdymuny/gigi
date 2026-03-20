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
from .subscriber import GigiSubscriber, SubscriptionEvent

__version__ = "0.5.0"
__all__ = [
    "GigiClient",
    "GigiSubscriber",
    "SubscriptionEvent",
    "GigiError",
    "BundleNotFound",
    "AuthError",
]
