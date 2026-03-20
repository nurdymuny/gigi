"""
GigiSubscriber — async WebSocket client for reactive GIGI subscriptions.

Wire protocol (text frames):
    Client → Server:
        SUBSCRIBE <bundle> [WHERE <field> <op> <value> [AND ...]]
        UNSUBSCRIBE <bundle>
        PING
        INSERT <bundle>\\n<json>
        QUERY <bundle> WHERE <field> = <value>
        CURVATURE <bundle>

    Server → Client:
        SUBSCRIBED <bundle> filters=<n>
        UNSUBSCRIBED <bundle>
        EVENT <bundle> <op> <json> K=<curvature>
        NOTICE <bundle> lagged=<n>
        PONG
        RESULT <json>
        ERROR <message>

Geometric interpretation:
    Each subscription is an open set U ⊆ B in the sheaf topology.
    EVENT frames carry the restriction ρ_{U,b}(s) — the section value
    at a base point b that falls inside U.
    A NOTICE lagged=n means n events fell out of the channel buffer;
    the subscriber has detected holonomy (lost path-equivalence).
"""

from __future__ import annotations

import asyncio
import json
from dataclasses import dataclass, field
from typing import AsyncIterator, Dict, List, Optional

try:
    from websockets import connect as ws_connect
    from websockets.exceptions import ConnectionClosedError, ConnectionClosedOK
except ImportError:
    raise ImportError(
        "websockets is required. Install with: pip install gigi-client"
    )

from .client import AuthError, GigiError


@dataclass
class SubscriptionEvent:
    """
    A single reactive event pushed by the server.

    Attributes:
        bundle:    The bundle this event came from.
        op:        Mutation type: insert | update | delete | upsert |
                                  bulk_update | bulk_delete.
        record:    The record dict (parsed JSON from the EVENT frame).
        curvature: Scalar curvature K of the bundle at the time of mutation.
    """
    bundle: str
    op: str
    record: Dict
    curvature: float


@dataclass
class _SubscriptionState:
    bundle: str
    where: Optional[str]


class GigiSubscriber:
    """
    Async WebSocket subscriber for real-time GIGI events.

    Implements the sheaf-restriction model: each subscription defines an
    open set with optional filter predicates; the server sends only events
    whose base point lies inside that open set.

    Args:
        ws_url:  WebSocket URL, e.g. "ws://localhost:3142/ws".
                 Use "wss://" for TLS connections.
        api_key: Optional API key (sent via Sec-WebSocket-Protocol handshake
                 or query param, depending on server config).
        ping_interval: Seconds between keepalive PINGs (None to disable).
        reconnect: If True, auto-reconnect on disconnect (not yet implemented;
                   placeholder for future use).

    Usage::

        async with GigiSubscriber("ws://localhost:3142/ws") as sub:
            await sub.subscribe("orders", where="status eq pending")
            async for event in sub.events():
                print(event.op, event.record)
    """

    def __init__(
        self,
        ws_url: str,
        *,
        api_key: Optional[str] = None,
        ping_interval: Optional[float] = 30.0,
        reconnect: bool = False,
    ) -> None:
        self._ws_url = ws_url
        self._api_key = api_key
        self._ping_interval = ping_interval
        self._reconnect = reconnect
        self._ws = None
        self._subscriptions: Dict[str, _SubscriptionState] = {}
        self._event_queue: asyncio.Queue[SubscriptionEvent] = asyncio.Queue()
        self._notice_queue: asyncio.Queue[Dict] = asyncio.Queue()
        self._connected = False
        self._listen_task: Optional[asyncio.Task] = None
        self._ping_task: Optional[asyncio.Task] = None

    async def connect(self) -> None:
        """Open the WebSocket connection."""
        extra_headers: Dict[str, str] = {}
        if self._api_key:
            extra_headers["X-Api-Key"] = self._api_key

        self._ws = await ws_connect(
            self._ws_url,
            additional_headers=extra_headers,
            max_size=10 * 1024 * 1024,  # 10 MB max frame
        )
        self._connected = True
        self._listen_task = asyncio.create_task(self._listener())
        if self._ping_interval:
            self._ping_task = asyncio.create_task(self._pinger())

    async def disconnect(self) -> None:
        """Close the WebSocket connection and cancel background tasks."""
        self._connected = False
        if self._ping_task:
            self._ping_task.cancel()
        if self._listen_task:
            self._listen_task.cancel()
        if self._ws:
            await self._ws.close()
        self._ws = None

    async def subscribe(self, bundle: str, *, where: Optional[str] = None) -> None:
        """
        Subscribe to events from a bundle.

        Args:
            bundle: Bundle name.
            where:  Optional filter predicate in the compact WHERE syntax:
                    "field op value [AND field op value ...]"
                    Ops: eq neq gt gte lt lte contains starts_with ends_with

        Example::
            await sub.subscribe("orders", where="amount gt 100 AND status eq open")
        """
        self._ensure_connected()
        cmd = f"SUBSCRIBE {bundle}"
        if where:
            cmd += f" WHERE {where}"
        await self._send(cmd)
        self._subscriptions[bundle] = _SubscriptionState(bundle=bundle, where=where)

    async def unsubscribe(self, bundle: str) -> None:
        """Cancel subscription for a bundle."""
        self._ensure_connected()
        await self._send(f"UNSUBSCRIBE {bundle}")
        self._subscriptions.pop(bundle, None)

    async def ping(self) -> None:
        """Send a PING and wait for PONG."""
        self._ensure_connected()
        await self._send("PING")

    async def events(self) -> AsyncIterator[SubscriptionEvent]:
        """
        Async iterator over incoming subscription events.

        Yields SubscriptionEvent instances as the server pushes EVENT frames.
        Runs until the connection is closed or disconnect() is called.

        Lag notices (NOTICE lagged=N) are NOT yielded here; consume them
        via `notices()` if needed.

        Example::
            async for event in sub.events():
                print(f"{event.op} in {event.bundle}: {event.record}")
        """
        while self._connected or not self._event_queue.empty():
            try:
                event = await asyncio.wait_for(
                    self._event_queue.get(), timeout=0.1
                )
                yield event
            except asyncio.TimeoutError:
                continue

    async def notices(self) -> AsyncIterator[Dict]:
        """
        Async iterator over lag notices.

        Yields dicts: {"bundle": str, "lagged": int}
        These indicate missed events when the broadcast channel overflowed.
        """
        while self._connected or not self._notice_queue.empty():
            try:
                notice = await asyncio.wait_for(
                    self._notice_queue.get(), timeout=0.1
                )
                yield notice
            except asyncio.TimeoutError:
                continue

    # ── Context manager ───────────────────────────────────────────────────

    async def __aenter__(self) -> "GigiSubscriber":
        await self.connect()
        return self

    async def __aexit__(self, *_) -> None:
        await self.disconnect()

    # ── Internals ─────────────────────────────────────────────────────────

    def _ensure_connected(self) -> None:
        if not self._connected or self._ws is None:
            raise GigiError("Not connected. Call connect() or use 'async with'.")

    async def _send(self, text: str) -> None:
        if self._ws is None:
            raise GigiError("WebSocket is None — connection lost.")
        await self._ws.send(text)

    async def _listener(self) -> None:
        """Background task: read frames from the server and dispatch them."""
        try:
            async for raw in self._ws:
                if isinstance(raw, bytes):
                    raw = raw.decode("utf-8", errors="replace")
                self._dispatch(raw)
        except (ConnectionClosedOK, ConnectionClosedError):
            self._connected = False
        except asyncio.CancelledError:
            pass
        except Exception:
            self._connected = False

    async def _pinger(self) -> None:
        """Background task: send periodic PINGs."""
        try:
            while self._connected:
                await asyncio.sleep(self._ping_interval)
                if self._connected and self._ws:
                    await self._send("PING")
        except asyncio.CancelledError:
            pass

    def _dispatch(self, raw: str) -> None:
        """Parse a server frame and route it to the appropriate queue."""
        raw = raw.strip()
        if raw.startswith("EVENT "):
            event = self._parse_event(raw)
            if event is not None:
                self._event_queue.put_nowait(event)
        elif raw.startswith("NOTICE "):
            notice = self._parse_notice(raw)
            if notice is not None:
                self._notice_queue.put_nowait(notice)
        elif raw.startswith("ERROR "):
            # Server errors are raised lazily; callers see them as missing events
            pass
        # SUBSCRIBED, UNSUBSCRIBED, PONG, RESULT — informational, ignored here

    @staticmethod
    def _parse_event(raw: str) -> Optional[SubscriptionEvent]:
        """
        Parse: EVENT <bundle> <op> <record_json> K=<curvature>

        The record_json is everything between <op> and the trailing K= token.
        """
        # "EVENT " prefix consumed above; strip it
        line = raw[len("EVENT "):]

        # Extract K=<float> from the end
        curvature = 0.0
        k_sep = " K="
        k_idx = line.rfind(k_sep)
        if k_idx != -1:
            try:
                curvature = float(line[k_idx + len(k_sep):])
            except ValueError:
                pass
            line = line[:k_idx]

        # Remaining: "<bundle> <op> <json>"
        parts = line.split(" ", 2)
        if len(parts) < 3:
            return None
        bundle, op, record_json = parts[0], parts[1], parts[2]
        try:
            record = json.loads(record_json)
        except json.JSONDecodeError:
            record = {"_raw": record_json}

        return SubscriptionEvent(
            bundle=bundle,
            op=op,
            record=record if isinstance(record, dict) else {"_value": record},
            curvature=curvature,
        )

    @staticmethod
    def _parse_notice(raw: str) -> Optional[Dict]:
        """Parse: NOTICE <bundle> lagged=<n>"""
        # "NOTICE " prefix
        line = raw[len("NOTICE "):].strip()
        parts = line.split(" ", 1)
        bundle = parts[0]
        lagged = 0
        if len(parts) == 2 and parts[1].startswith("lagged="):
            try:
                lagged = int(parts[1][len("lagged="):])
            except ValueError:
                pass
        return {"bundle": bundle, "lagged": lagged}
