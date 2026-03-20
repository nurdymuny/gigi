"""
GigiClient — synchronous REST client for the GIGI geometric database.

Geometric model:
    Bundles are fiber bundles π: E → B over a base manifold B.
    Each INSERT adds a section σ: B → E.
    Curvature K measures deviation from the flat (arithmetic) connection.

Security:
    - API key passed via X-Api-Key header (never in URL)
    - TLS enforced when url starts with https://
    - No credentials logged
"""

from __future__ import annotations

import json
from typing import Any, Dict, Iterator, List, Optional, Sequence

import requests
from requests import Session


class GigiError(Exception):
    """Base exception for all GIGI client errors."""
    def __init__(self, message: str, status_code: Optional[int] = None):
        super().__init__(message)
        self.status_code = status_code


class BundleNotFound(GigiError):
    """Raised when the requested bundle does not exist (HTTP 404)."""


class AuthError(GigiError):
    """Raised on authentication failure (HTTP 401/403)."""


class VersionConflict(GigiError):
    """Raised when an optimistic concurrency check fails (HTTP 409)."""


def _raise_for(resp: requests.Response) -> None:
    """Raise an appropriate GigiError for non-2xx responses."""
    if resp.ok:
        return
    try:
        body = resp.json()
        msg = body.get("error", resp.text)
    except Exception:
        msg = resp.text
    if resp.status_code == 404:
        raise BundleNotFound(msg, status_code=404)
    if resp.status_code in (401, 403):
        raise AuthError(msg, status_code=resp.status_code)
    if resp.status_code == 409:
        raise VersionConflict(msg, status_code=409)
    raise GigiError(msg, status_code=resp.status_code)


class GigiClient:
    """
    Synchronous REST client for GIGI Stream.

    Args:
        url: Base URL of the GIGI server, e.g. "http://localhost:3142".
        api_key: Optional API key (sent as X-Api-Key header).
        timeout: Request timeout in seconds (default 30).
        session: Optional pre-configured requests.Session to use.

    Example::

        db = GigiClient("https://gigi-stream.fly.dev", api_key="my-key")
        db.create_bundle("events", fields={"id": "categorical", "ts": "timestamp",
                                           "val": "numeric"}, keys=["id"])
        db.insert("events", [{"id": "e1", "ts": 1700000000000, "val": 42.0}])
        records = db.query("events", filters=[{"field": "val", "op": "gt", "value": 10}])
    """

    def __init__(
        self,
        url: str,
        *,
        api_key: Optional[str] = None,
        timeout: float = 30.0,
        session: Optional[Session] = None,
    ) -> None:
        self._base = url.rstrip("/")
        self._timeout = timeout
        self._session = session or Session()
        if api_key:
            # API key in header — never in URL to avoid log leakage
            self._session.headers.update({"X-Api-Key": api_key})
        self._session.headers.update({"Content-Type": "application/json"})

    def _url(self, path: str) -> str:
        return f"{self._base}{path}"

    def _get(self, path: str, **kwargs) -> Any:
        resp = self._session.get(self._url(path), timeout=self._timeout, **kwargs)
        _raise_for(resp)
        return resp.json()

    def _post(self, path: str, body: Any) -> Any:
        resp = self._session.post(self._url(path), json=body, timeout=self._timeout)
        _raise_for(resp)
        return resp.json()

    def _patch(self, path: str, body: Any) -> Any:
        resp = self._session.patch(self._url(path), json=body, timeout=self._timeout)
        _raise_for(resp)
        return resp.json()

    def _delete(self, path: str, body: Optional[Any] = None) -> Any:
        kwargs: Dict[str, Any] = {"timeout": self._timeout}
        if body is not None:
            kwargs["json"] = body
        resp = self._session.delete(self._url(path), **kwargs)
        _raise_for(resp)
        return resp.json()

    # ── Health ─────────────────────────────────────────────────────────────

    def health(self) -> Dict[str, Any]:
        """GET /v1/health — server status and record counts."""
        return self._get("/v1/health")

    # ── Bundle management ──────────────────────────────────────────────────

    def list_bundles(self) -> List[Dict[str, Any]]:
        """List all bundles with record counts."""
        return self._get("/v1/bundles")

    def create_bundle(
        self,
        name: str,
        fields: Dict[str, str],
        *,
        keys: Optional[List[str]] = None,
        indexed: Optional[List[str]] = None,
        defaults: Optional[Dict[str, Any]] = None,
        encrypted: bool = False,
    ) -> Dict[str, Any]:
        """
        Create a new bundle (table).

        Args:
            name: Bundle name.
            fields: Mapping of field_name → type string.
                    Types: "numeric", "categorical", "timestamp".
            keys: Base field names (form the primary key / base point).
                  Defaults to first field if omitted.
            indexed: Field names to build an index on (for fast range queries).
            defaults: Default values for fields, e.g. {"status": "active"}.
            encrypted: Enable GaugeKey encryption for fiber values.
        """
        body: Dict[str, Any] = {
            "name": name,
            "schema": {
                "fields": fields,
                "keys": keys or [],
                "indexed": indexed or [],
                "defaults": defaults or {},
            },
            "encrypted": encrypted,
        }
        return self._post("/v1/bundles", body)

    def drop_bundle(self, name: str) -> Dict[str, Any]:
        """Drop (delete) a bundle and all its data."""
        return self._delete(f"/v1/bundles/{name}")

    def schema(self, name: str) -> Dict[str, Any]:
        """GET bundle schema: base fields, fiber fields, indexed fields."""
        return self._get(f"/v1/bundles/{name}/schema")

    # ── Inserts ────────────────────────────────────────────────────────────

    def insert(self, bundle: str, records: List[Dict[str, Any]]) -> Dict[str, Any]:
        """
        Insert records into a bundle.

        Returns curvature K and confidence after insertion.
        WAL-logged: survives server restarts.
        """
        return self._post(f"/v1/bundles/{bundle}/insert", {"records": records})

    def upsert(self, bundle: str, record: Dict[str, Any]) -> Dict[str, Any]:
        """Insert if not exists, update if exists (by key)."""
        return self._post(f"/v1/bundles/{bundle}/upsert", {"record": record})

    def stream_ndjson(self, bundle: str, ndjson: str) -> Dict[str, Any]:
        """
        Bulk-ingest newline-delimited JSON (NDJSON).
        Efficient for large dataset uploads.
        """
        resp = self._session.post(
            self._url(f"/v1/bundles/{bundle}/stream"),
            data=ndjson.encode(),
            headers={"Content-Type": "application/x-ndjson"},
            timeout=self._timeout,
        )
        _raise_for(resp)
        return resp.json()

    # ── Queries ────────────────────────────────────────────────────────────

    def get(self, bundle: str, **key_fields) -> Optional[Dict[str, Any]]:
        """
        Point query — O(1) lookup by key field(s).

        Example::

            record = db.get("users", user_id=42)
        """
        result = self._get(f"/v1/bundles/{bundle}/get", params=key_fields)
        if result.get("data") is None:
            return None
        return result["data"]

    def query(
        self,
        bundle: str,
        *,
        filters: Optional[List[Dict[str, Any]]] = None,
        sort_by: Optional[str] = None,
        sort_desc: bool = False,
        limit: Optional[int] = None,
        offset: Optional[int] = None,
        fields: Optional[List[str]] = None,
        search: Optional[str] = None,
        or_conditions: Optional[List[List[Dict[str, Any]]]] = None,
    ) -> List[Dict[str, Any]]:
        """
        Filtered query with optional sort, pagination, and projection.

        Args:
            filters: List of filter dicts: {"field": "x", "op": "gt", "value": 10}.
                     Supported ops: eq, neq, gt, gte, lt, lte, contains, starts_with,
                     ends_with, regex, in, not_in, is_null, is_not_null.
            sort_by:  Field name to sort by.
            sort_desc: Sort descending if True.
            limit:    Max records to return.
            offset:   Records to skip (for pagination).
            fields:   Field projection — only return these fields.
            search:   Full-text search across all text fields.
            or_conditions: OR-grouped conditions (list of AND-groups).

        Returns:
            List of record dicts.
        """
        body: Dict[str, Any] = {
            "conditions": filters or [],
        }
        if sort_by:
            body["sort_by"] = sort_by
            body["sort_desc"] = sort_desc
        if limit is not None:
            body["limit"] = limit
        if offset is not None:
            body["offset"] = offset
        if fields:
            body["fields"] = fields
        if search:
            body["search"] = search
        if or_conditions:
            body["or_conditions"] = or_conditions

        result = self._post(f"/v1/bundles/{bundle}/query", body)
        return result.get("data", [])

    def count(
        self,
        bundle: str,
        filters: Optional[List[Dict[str, Any]]] = None,
    ) -> int:
        """Count records matching filters."""
        body = {"conditions": filters or []}
        return self._post(f"/v1/bundles/{bundle}/count", body)["count"]

    def exists(
        self,
        bundle: str,
        filters: Optional[List[Dict[str, Any]]] = None,
    ) -> bool:
        """Check if any record matches filters."""
        body = {"conditions": filters or []}
        return self._post(f"/v1/bundles/{bundle}/exists", body)["exists"]

    def distinct(self, bundle: str, field: str) -> List[Any]:
        """Get all distinct values for a field."""
        return self._get(f"/v1/bundles/{bundle}/distinct/{field}")["values"]

    def all(
        self,
        bundle: str,
        *,
        limit: Optional[int] = None,
        offset: Optional[int] = None,
    ) -> List[Dict[str, Any]]:
        """Return all records, optionally paginated."""
        params: Dict[str, Any] = {}
        if limit is not None:
            params["limit"] = limit
        if offset is not None:
            params["offset"] = offset
        result = self._get(f"/v1/bundles/{bundle}/points", params=params)
        return result.get("data", [])

    # ── Pandas integration ────────────────────────────────────────────────

    def query_df(
        self,
        bundle: str,
        *,
        filters: Optional[List[Dict[str, Any]]] = None,
        **kwargs,
    ):
        """
        Query and return results as a pandas DataFrame.
        Requires pandas to be installed: pip install gigi-client[pandas]
        """
        try:
            import pandas as pd
        except ImportError:
            raise ImportError(
                "pandas is required for query_df(). "
                "Install with: pip install gigi-client[pandas]"
            )
        records = self.query(bundle, filters=filters, **kwargs)
        return pd.DataFrame(records)

    def all_df(self, bundle: str, **kwargs):
        """Return all records as a pandas DataFrame."""
        try:
            import pandas as pd
        except ImportError:
            raise ImportError(
                "pandas is required for all_df(). "
                "Install with: pip install gigi-client[pandas]"
            )
        return pd.DataFrame(self.all(bundle, **kwargs))

    # ── Updates ────────────────────────────────────────────────────────────

    def update(
        self,
        bundle: str,
        key: Dict[str, Any],
        fields: Dict[str, Any],
        *,
        returning: bool = False,
        expected_version: Optional[int] = None,
    ) -> Dict[str, Any]:
        """
        Update a record by key (partial patch).

        Args:
            key: Primary key field(s) identifying the record.
            fields: Fields to update.
            returning: Return the updated record in the response.
            expected_version: For optimistic concurrency — the current _version
                              value. Raises VersionConflict if stale.
        """
        body: Dict[str, Any] = {"key": key, "fields": fields, "returning": returning}
        if expected_version is not None:
            body["expected_version"] = expected_version
        return self._post(f"/v1/bundles/{bundle}/update", body)

    def bulk_update(
        self,
        bundle: str,
        filters: List[Dict[str, Any]],
        fields: Dict[str, Any],
    ) -> Dict[str, Any]:
        """Update all records matching filters."""
        return self._patch(
            f"/v1/bundles/{bundle}/points",
            {"filter": filters, "fields": fields},
        )

    def increment(
        self,
        bundle: str,
        key: Dict[str, Any],
        field: str,
        amount: float = 1.0,
    ) -> Dict[str, Any]:
        """Atomic increment/decrement of a numeric field."""
        return self._post(
            f"/v1/bundles/{bundle}/increment",
            {"key": key, "field": field, "amount": amount},
        )

    # ── Deletes ────────────────────────────────────────────────────────────

    def delete(
        self,
        bundle: str,
        key: Dict[str, Any],
        *,
        returning: bool = False,
    ) -> Dict[str, Any]:
        """Delete a record by key."""
        body: Dict[str, Any] = {"key": key, "returning": returning}
        return self._post(f"/v1/bundles/{bundle}/delete", body)

    def bulk_delete(
        self,
        bundle: str,
        filters: List[Dict[str, Any]],
    ) -> Dict[str, Any]:
        """Delete all records matching filters."""
        return self._post(f"/v1/bundles/{bundle}/bulk-delete", {"conditions": filters})

    def truncate(self, bundle: str) -> Dict[str, Any]:
        """Delete all records from a bundle (schema preserved)."""
        return self._post(f"/v1/bundles/{bundle}/truncate", {})

    # ── Transactions ──────────────────────────────────────────────────────

    def transaction(
        self,
        bundle: str,
        ops: List[Dict[str, Any]],
    ) -> Dict[str, Any]:
        """
        Execute multiple operations atomically (all-or-nothing).

        Args:
            ops: List of operation dicts. Each op has "op" key:
                 {"op": "insert", "record": {...}}
                 {"op": "update", "key": {...}, "fields": {...}}
                 {"op": "delete", "key": {...}}
                 {"op": "increment", "key": {...}, "field": "n", "amount": 1}

        Raises:
            GigiError: If the transaction is rolled back.
        """
        return self._post(f"/v1/bundles/{bundle}/transaction", {"ops": ops})

    # ── GQL interface ─────────────────────────────────────────────────────

    def gql(self, query: str) -> Dict[str, Any]:
        """
        Execute a GQL (GIGI Query Language) statement.

        Example::

            db.gql("CREATE BUNDLE users BASE (id CATEGORICAL) FIBER (name CATEGORICAL, score NUMERIC)")
            db.gql("INSERT INTO users (id, name, score) VALUES ('u1', 'Alice', 9.5)")
            result = db.gql("COVER users WHERE score >= 9.0")
        """
        return self._post("/v1/gql", {"query": query})

    # ── Analytics ─────────────────────────────────────────────────────────

    def curvature(self, bundle: str) -> Dict[str, Any]:
        """
        Get scalar curvature K for the bundle.

        Geometric interpretation:
            K ≈ 0  → flat (arithmetic patterns dominate, high compressibility)
            K > 0  → positive curvature (bounded/categorical)
            K < 0  → negative curvature (heavy-tailed)

        Returns dict with: K, confidence, capacity, per_field list.
        """
        return self._get(f"/v1/bundles/{bundle}/curvature")

    def spectral(self, bundle: str) -> Dict[str, Any]:
        """
        Get spectral gap λ₁ of the bundle's connection Laplacian.
        λ₁ > 0 guarantees rapid mixing (expander graph structure).
        """
        return self._get(f"/v1/bundles/{bundle}/spectral")

    def consistency(self, bundle: str) -> Dict[str, Any]:
        """
        Check Čech cohomology H¹ — holonomy-based consistency check.
        H¹ = 0 means fully consistent (flat connection, path-independent).
        Non-zero H¹ detects conflicting records (data integrity issues).
        """
        return self._get(f"/v1/bundles/{bundle}/consistency")

    def stats(self, bundle: str) -> Dict[str, Any]:
        """Full bundle statistics including per-field stats and cardinalities."""
        return self._get(f"/v1/bundles/{bundle}/stats")

    def explain(
        self,
        bundle: str,
        filters: Optional[List[Dict[str, Any]]] = None,
        *,
        limit: Optional[int] = None,
        offset: Optional[int] = None,
    ) -> Dict[str, Any]:
        """Explain the query execution plan (index scans vs full scan)."""
        body: Dict[str, Any] = {"conditions": filters or []}
        if limit is not None:
            body["limit"] = limit
        if offset is not None:
            body["offset"] = offset
        return self._post(f"/v1/bundles/{bundle}/explain", body)

    # ── Import / Export ───────────────────────────────────────────────────

    def export(self, bundle: str) -> List[Dict[str, Any]]:
        """Export all records as a list of dicts."""
        return self._get(f"/v1/bundles/{bundle}/export")["records"]

    def export_dhoom(self, bundle: str) -> str:
        """Export bundle in DHOOM wire format (geometric compression)."""
        return self._get(f"/v1/bundles/{bundle}/dhoom")["dhoom"]

    def import_records(self, bundle: str, records: List[Dict[str, Any]]) -> Dict[str, Any]:
        """Import a list of records (alias for import endpoint)."""
        return self._post(f"/v1/bundles/{bundle}/import", {"records": records})

    def add_field(
        self,
        bundle: str,
        name: str,
        field_type: str = "categorical",
        default: Optional[Any] = None,
    ) -> Dict[str, Any]:
        """Add a new fiber field to an existing bundle."""
        body: Dict[str, Any] = {"name": name, "type": field_type}
        if default is not None:
            body["default"] = default
        return self._post(f"/v1/bundles/{bundle}/add-field", body)

    def add_index(self, bundle: str, field: str) -> Dict[str, Any]:
        """Add an index on a field for faster range queries."""
        return self._post(f"/v1/bundles/{bundle}/add-index", {"field": field})

    def __repr__(self) -> str:
        return f"GigiClient({self._base!r})"
