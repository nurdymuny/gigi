"""
TDD tests for GigiClient and GigiSubscriber.

Run with:
    pip install gigi-client[pandas] responses pytest pytest-asyncio
    pytest sdk/python/tests/ -v
"""

from __future__ import annotations

import json
import pytest
import responses as resp_lib  # alias to avoid shadowing
from unittest.mock import AsyncMock, MagicMock, patch

from gigi.client import (
    GigiClient,
    GigiError,
    BundleNotFound,
    AuthError,
    VersionConflict,
)
from gigi.subscriber import GigiSubscriber, SubscriptionEvent


BASE_URL = "http://gigi-test.local"


# ── Helpers ───────────────────────────────────────────────────────────────────


def client() -> GigiClient:
    return GigiClient(BASE_URL, api_key="test-key")


def mock_json(mocked, method: str, path: str, body: dict, status: int = 200):
    url = f"{BASE_URL}{path}"
    getattr(mocked, method)(url, json=body, status=status)


# ── Construction ──────────────────────────────────────────────────────────────


class TestClientInit:
    def test_base_url_trailing_slash_stripped(self):
        c = GigiClient("http://localhost:3142/")
        assert c._base == "http://localhost:3142"

    def test_api_key_in_header_not_url(self):
        c = GigiClient(BASE_URL, api_key="secret")
        assert c._session.headers.get("X-Api-Key") == "secret"
        assert "secret" not in c._base

    def test_repr(self):
        c = GigiClient(BASE_URL)
        assert "gigi-test.local" in repr(c)


# ── Health ────────────────────────────────────────────────────────────────────


class TestHealth:
    @resp_lib.activate
    def test_health_ok(self):
        resp_lib.add(resp_lib.GET, f"{BASE_URL}/v1/health",
                     json={"status": "ok", "records": 42})
        result = client().health()
        assert result["status"] == "ok"
        assert result["records"] == 42

    @resp_lib.activate
    def test_health_error_raises(self):
        resp_lib.add(resp_lib.GET, f"{BASE_URL}/v1/health",
                     json={"error": "boom"}, status=500)
        with pytest.raises(GigiError) as exc_info:
            client().health()
        assert exc_info.value.status_code == 500


# ── Bundles ───────────────────────────────────────────────────────────────────


class TestBundles:
    @resp_lib.activate
    def test_list_bundles(self):
        resp_lib.add(resp_lib.GET, f"{BASE_URL}/v1/bundles",
                     json=[{"name": "users", "count": 10}])
        bundles = client().list_bundles()
        assert len(bundles) == 1
        assert bundles[0]["name"] == "users"

    @resp_lib.activate
    def test_create_bundle(self):
        resp_lib.add(resp_lib.POST, f"{BASE_URL}/v1/bundles",
                     json={"created": True, "name": "orders"})
        result = client().create_bundle(
            "orders",
            fields={"id": "categorical", "amount": "numeric"},
            keys=["id"],
        )
        assert result["created"] is True
        # Verify request body
        body = json.loads(resp_lib.calls[0].request.body)
        assert body["name"] == "orders"
        assert body["schema"]["keys"] == ["id"]

    @resp_lib.activate
    def test_drop_bundle_ok(self):
        resp_lib.add(resp_lib.DELETE, f"{BASE_URL}/v1/bundles/orders",
                     json={"dropped": True})
        result = client().drop_bundle("orders")
        assert result["dropped"] is True

    @resp_lib.activate
    def test_drop_bundle_not_found(self):
        resp_lib.add(resp_lib.DELETE, f"{BASE_URL}/v1/bundles/missing",
                     json={"error": "not found"}, status=404)
        with pytest.raises(BundleNotFound):
            client().drop_bundle("missing")


# ── Insert ────────────────────────────────────────────────────────────────────


class TestInsert:
    @resp_lib.activate
    def test_insert_ok(self):
        resp_lib.add(resp_lib.POST, f"{BASE_URL}/v1/bundles/events/insert",
                     json={"inserted": 2, "K": 0.012, "confidence": 0.94})
        result = client().insert("events", [{"id": "e1"}, {"id": "e2"}])
        assert result["inserted"] == 2
        assert isinstance(result["K"], float)

    @resp_lib.activate
    def test_insert_auth_error(self):
        resp_lib.add(resp_lib.POST, f"{BASE_URL}/v1/bundles/events/insert",
                     json={"error": "unauthorized"}, status=401)
        with pytest.raises(AuthError):
            client().insert("events", [{"id": "e1"}])

    @resp_lib.activate
    def test_upsert(self):
        resp_lib.add(resp_lib.POST, f"{BASE_URL}/v1/bundles/users/upsert",
                     json={"upserted": True, "op": "insert"})
        result = client().upsert("users", {"id": "u1", "name": "Alice"})
        assert result["upserted"] is True


# ── Query ─────────────────────────────────────────────────────────────────────


class TestQuery:
    @resp_lib.activate
    def test_query_returns_list(self):
        resp_lib.add(resp_lib.POST, f"{BASE_URL}/v1/bundles/events/query",
                     json={"data": [{"id": "e1", "val": 42}]})
        records = client().query("events")
        assert len(records) == 1
        assert records[0]["id"] == "e1"

    @resp_lib.activate
    def test_query_with_filters(self):
        resp_lib.add(resp_lib.POST, f"{BASE_URL}/v1/bundles/events/query",
                     json={"data": []})
        client().query(
            "events",
            filters=[{"field": "val", "op": "gt", "value": 10}],
            sort_by="val",
            sort_desc=True,
            limit=50,
            offset=0,
        )
        body = json.loads(resp_lib.calls[0].request.body)
        assert body["conditions"] == [{"field": "val", "op": "gt", "value": 10}]
        assert body["sort_by"] == "val"
        assert body["sort_desc"] is True
        assert body["limit"] == 50

    @resp_lib.activate
    def test_query_empty_response(self):
        resp_lib.add(resp_lib.POST, f"{BASE_URL}/v1/bundles/events/query",
                     json={})
        records = client().query("events")
        assert records == []

    @resp_lib.activate
    def test_get_found(self):
        resp_lib.add(resp_lib.GET, f"{BASE_URL}/v1/bundles/users/get",
                     json={"data": {"id": "u1", "name": "Alice"}})
        record = client().get("users", user_id="u1")
        assert record["name"] == "Alice"

    @resp_lib.activate
    def test_get_not_found(self):
        resp_lib.add(resp_lib.GET, f"{BASE_URL}/v1/bundles/users/get",
                     json={"data": None})
        record = client().get("users", user_id="missing")
        assert record is None

    @resp_lib.activate
    def test_count(self):
        resp_lib.add(resp_lib.POST, f"{BASE_URL}/v1/bundles/events/count",
                     json={"count": 7})
        assert client().count("events") == 7

    @resp_lib.activate
    def test_exists_true(self):
        resp_lib.add(resp_lib.POST, f"{BASE_URL}/v1/bundles/events/exists",
                     json={"exists": True})
        assert client().exists("events") is True

    @resp_lib.activate
    def test_distinct(self):
        resp_lib.add(resp_lib.GET, f"{BASE_URL}/v1/bundles/events/distinct/status",
                     json={"values": ["open", "closed"]})
        vals = client().distinct("events", "status")
        assert "open" in vals


# ── Pandas ────────────────────────────────────────────────────────────────────


class TestPandas:
    @resp_lib.activate
    def test_query_df(self):
        pd = pytest.importorskip("pandas")
        resp_lib.add(resp_lib.POST, f"{BASE_URL}/v1/bundles/events/query",
                     json={"data": [{"id": "e1", "val": 1}, {"id": "e2", "val": 2}]})
        df = client().query_df("events")
        assert list(df.columns) == ["id", "val"]
        assert len(df) == 2

    def test_query_df_no_pandas(self, monkeypatch):
        import builtins
        real_import = builtins.__import__

        def mock_import(name, *args, **kwargs):
            if name == "pandas":
                raise ImportError("No module named 'pandas'")
            return real_import(name, *args, **kwargs)

        monkeypatch.setattr(builtins, "__import__", mock_import)
        with pytest.raises(ImportError, match="pandas is required"):
            client().query_df("events")


# ── Update ────────────────────────────────────────────────────────────────────


class TestUpdate:
    @resp_lib.activate
    def test_update_ok(self):
        resp_lib.add(resp_lib.POST, f"{BASE_URL}/v1/bundles/users/update",
                     json={"updated": 1})
        result = client().update("users", key={"id": "u1"}, fields={"name": "Bob"})
        assert result["updated"] == 1

    @resp_lib.activate
    def test_update_returning(self):
        resp_lib.add(resp_lib.POST, f"{BASE_URL}/v1/bundles/users/update",
                     json={"updated": 1, "record": {"id": "u1", "name": "Bob"}})
        result = client().update(
            "users", key={"id": "u1"}, fields={"name": "Bob"}, returning=True
        )
        body = json.loads(resp_lib.calls[0].request.body)
        assert body["returning"] is True
        assert result["record"]["name"] == "Bob"

    @resp_lib.activate
    def test_update_version_conflict(self):
        resp_lib.add(resp_lib.POST, f"{BASE_URL}/v1/bundles/users/update",
                     json={"error": "version conflict"}, status=409)
        with pytest.raises(VersionConflict):
            client().update(
                "users",
                key={"id": "u1"},
                fields={"name": "Bob"},
                expected_version=5,
            )

    @resp_lib.activate
    def test_bulk_update(self):
        resp_lib.add(resp_lib.PATCH, f"{BASE_URL}/v1/bundles/events/points",
                     json={"updated": 3})
        result = client().bulk_update(
            "events",
            filters=[{"field": "status", "op": "eq", "value": "pending"}],
            fields={"status": "closed"},
        )
        assert result["updated"] == 3


# ── Delete ────────────────────────────────────────────────────────────────────


class TestDelete:
    @resp_lib.activate
    def test_delete_ok(self):
        resp_lib.add(resp_lib.POST, f"{BASE_URL}/v1/bundles/users/delete",
                     json={"deleted": 1})
        result = client().delete("users", key={"id": "u1"})
        assert result["deleted"] == 1

    @resp_lib.activate
    def test_bulk_delete(self):
        resp_lib.add(resp_lib.POST, f"{BASE_URL}/v1/bundles/events/bulk-delete",
                     json={"deleted": 5})
        result = client().bulk_delete(
            "events",
            filters=[{"field": "status", "op": "eq", "value": "closed"}],
        )
        assert result["deleted"] == 5

    @resp_lib.activate
    def test_truncate(self):
        resp_lib.add(resp_lib.POST, f"{BASE_URL}/v1/bundles/events/truncate",
                     json={"deleted": 100})
        result = client().truncate("events")
        assert result["deleted"] == 100


# ── Analytics ─────────────────────────────────────────────────────────────────


class TestAnalytics:
    @resp_lib.activate
    def test_curvature(self):
        resp_lib.add(resp_lib.GET, f"{BASE_URL}/v1/bundles/events/curvature",
                     json={"K": 0.05, "confidence": 0.92, "capacity": 1024})
        result = client().curvature("events")
        assert isinstance(result["K"], float)
        assert 0.0 <= result["confidence"] <= 1.0

    @resp_lib.activate
    def test_stats(self):
        resp_lib.add(resp_lib.GET, f"{BASE_URL}/v1/bundles/events/stats",
                     json={"count": 50, "fields": {"val": {"mean": 42.5}}})
        result = client().stats("events")
        assert result["count"] == 50

    @resp_lib.activate
    def test_consistency(self):
        resp_lib.add(resp_lib.GET, f"{BASE_URL}/v1/bundles/events/consistency",
                     json={"H1": 0, "consistent": True})
        result = client().consistency("events")
        assert result["consistent"] is True


# ── GQL ───────────────────────────────────────────────────────────────────────


class TestGQL:
    @resp_lib.activate
    def test_gql_ok(self):
        resp_lib.add(resp_lib.POST, f"{BASE_URL}/v1/gql",
                     json={"result": [{"id": "u1"}]})
        result = client().gql("COVER users WHERE score >= 9.0")
        assert "result" in result

    @resp_lib.activate
    def test_gql_sends_query_field(self):
        resp_lib.add(resp_lib.POST, f"{BASE_URL}/v1/gql", json={})
        client().gql("SELECT * FROM events")
        body = json.loads(resp_lib.calls[0].request.body)
        assert body["query"] == "SELECT * FROM events"


# ── Error mapping ─────────────────────────────────────────────────────────────


class TestErrorMapping:
    def _make(self, status: int):
        resp_lib.add(resp_lib.GET, f"{BASE_URL}/v1/health",
                     json={"error": "err"}, status=status)

    @resp_lib.activate
    def test_404_raises_bundle_not_found(self):
        self._make(404)
        with pytest.raises(BundleNotFound):
            client().health()

    @resp_lib.activate
    def test_401_raises_auth_error(self):
        self._make(401)
        with pytest.raises(AuthError):
            client().health()

    @resp_lib.activate
    def test_403_raises_auth_error(self):
        self._make(403)
        with pytest.raises(AuthError):
            client().health()

    @resp_lib.activate
    def test_500_raises_gigi_error(self):
        self._make(500)
        with pytest.raises(GigiError) as exc_info:
            client().health()
        assert exc_info.value.status_code == 500


# ── SubscriptionEvent parsing ─────────────────────────────────────────────────


class TestSubscriptionEventParsing:
    def test_parse_event_basic(self):
        raw = 'EVENT orders insert {"id":"o1","amount":99.5} K=0.031'
        event = GigiSubscriber._parse_event(raw)
        assert event is not None
        assert event.bundle == "orders"
        assert event.op == "insert"


# ── VectorSearch ──────────────────────────────────────────────────────────────


class TestVectorSearch:
    @resp_lib.activate
    def test_vector_search_basic(self):
        results = [
            {"score": 0.99, "record": {"id": "a", "emb": [1.0, 0.0]}},
            {"score": 0.71, "record": {"id": "b", "emb": [0.7, 0.7]}},
        ]
        resp_lib.post(
            f"{BASE_URL}/v1/bundles/docs/vector-search",
            json={"results": results, "meta": {"count": 2, "metric": "cosine", "top_k": 10}},
            status=200,
        )
        hits = client().vector_search("docs", "emb", [1.0, 0.0], top_k=10)
        assert len(hits) == 2
        assert hits[0]["score"] == pytest.approx(0.99)
        assert hits[0]["record"]["id"] == "a"

    @resp_lib.activate
    def test_vector_search_with_filters(self):
        resp_lib.post(
            f"{BASE_URL}/v1/bundles/articles/vector-search",
            json={"results": [], "meta": {"count": 0, "metric": "euclidean", "top_k": 5}},
            status=200,
        )
        hits = client().vector_search(
            "articles", "embedding", [0.1, 0.2, 0.3],
            top_k=5, metric="euclidean",
            filters=[{"field": "category", "op": "eq", "value": "tech"}],
        )
        assert hits == []
        # verify the request body was correct
        req = resp_lib.calls[0].request
        body = json.loads(req.body)
        assert body["metric"] == "euclidean"
        assert body["top_k"] == 5
        assert body["filters"] == [{"field": "category", "op": "eq", "value": "tech"}]

    @resp_lib.activate
    def test_vector_search_default_metric_cosine(self):
        resp_lib.post(
            f"{BASE_URL}/v1/bundles/store/vector-search",
            json={"results": [], "meta": {}},
            status=200,
        )
        client().vector_search("store", "vec", [1.0])
        body = json.loads(resp_lib.calls[0].request.body)
        assert body["metric"] == "cosine"
        assert body["top_k"] == 10

    @resp_lib.activate
    def test_vector_search_404_raises_bundle_not_found(self):
        resp_lib.post(
            f"{BASE_URL}/v1/bundles/ghost/vector-search",
            json={"error": "Bundle 'ghost' not found"},
            status=404,
        )
        with pytest.raises(BundleNotFound):
            client().vector_search("ghost", "emb", [1.0, 0.0])


# ── DropField ─────────────────────────────────────────────────────────────────


class TestDropField:
    @resp_lib.activate
    def test_drop_field_success(self):
        resp_lib.post(
            f"{BASE_URL}/v1/bundles/users/drop-field",
            json={"status": "field_dropped", "field": "temp", "records": 42},
            status=200,
        )
        resp = client().drop_field("users", "temp")
        assert resp["status"] == "field_dropped"
        assert resp["field"] == "temp"
        req_body = json.loads(resp_lib.calls[0].request.body)
        assert req_body == {"field": "temp"}

    @resp_lib.activate
    def test_drop_field_not_found_raises(self):
        resp_lib.post(
            f"{BASE_URL}/v1/bundles/users/drop-field",
            json={"error": "Field 'ghost' not found in bundle 'users'"},
            status=404,
        )
        with pytest.raises(GigiError):
            client().drop_field("users", "ghost")

    @resp_lib.activate
    def test_drop_field_bundle_not_found_raises(self):
        resp_lib.post(
            f"{BASE_URL}/v1/bundles/none/drop-field",
            json={"error": "Bundle 'none' not found"},
            status=404,
        )
        with pytest.raises(BundleNotFound):
            client().drop_field("none", "score")


# ── Aggregate ─────────────────────────────────────────────────────────────────


class TestAggregate:
    @resp_lib.activate
    def test_aggregate_basic(self):
        payload = {"groups": {"Eng": {"count": 20, "sum": 1000.0, "avg": 50.0, "min": 40.0, "max": 60.0}}}
        resp_lib.post(f"{BASE_URL}/v1/bundles/employees/aggregate", json=payload, status=200)
        result = client().aggregate("employees", "dept", "salary")
        assert "Eng" in result["groups"]
        assert result["groups"]["Eng"]["count"] == 20

    @resp_lib.activate
    def test_aggregate_sends_having(self):
        resp_lib.post(f"{BASE_URL}/v1/bundles/employees/aggregate",
                      json={"groups": {}}, status=200)
        client().aggregate("employees", "dept", "salary",
                           having=[{"field": "count", "op": "gt", "value": 10}])
        body = json.loads(resp_lib.calls[0].request.body)
        assert body["having"] == [{"field": "count", "op": "gt", "value": 10}]

    @resp_lib.activate
    def test_aggregate_sends_filters(self):
        resp_lib.post(f"{BASE_URL}/v1/bundles/employees/aggregate",
                      json={"groups": {}}, status=200)
        client().aggregate("employees", "dept", "salary",
                           filters=[{"field": "active", "op": "eq", "value": True}])
        body = json.loads(resp_lib.calls[0].request.body)
        assert body["conditions"] == [{"field": "active", "op": "eq", "value": True}]
        assert body["having"] == []

    @resp_lib.activate
    def test_aggregate_defaults_empty_having(self):
        resp_lib.post(f"{BASE_URL}/v1/bundles/employees/aggregate",
                      json={"groups": {}}, status=200)
        client().aggregate("employees", "dept", "salary")
        body = json.loads(resp_lib.calls[0].request.body)
        assert body["having"] == []
        assert body["conditions"] == []

    @resp_lib.activate
    def test_vector_search_404_raises_bundle_not_found(self):
        resp_lib.post(
            f"{BASE_URL}/v1/bundles/ghost/vector-search",
            json={"error": "Bundle 'ghost' not found"},
            status=404,
        )
        with pytest.raises(BundleNotFound):
            client().vector_search("ghost", "emb", [1.0, 0.0])

    def test_parse_event_no_k(self):
        raw = 'EVENT orders delete {"id":"o1"}'
        event = GigiSubscriber._parse_event(raw)
        assert event is not None
        assert event.op == "delete"
        assert event.curvature == 0.0

    def test_parse_event_bulk_update(self):
        raw = 'EVENT inventory bulk_update {"sku":"ABC","qty":0} K=-0.004'
        event = GigiSubscriber._parse_event(raw)
        assert event.op == "bulk_update"
        assert event.record["sku"] == "ABC"
        assert event.curvature < 0

    def test_parse_event_invalid_json_falls_back(self):
        raw = 'EVENT orders insert NOT_JSON K=0.0'
        event = GigiSubscriber._parse_event(raw)
        assert event is not None
        assert "_raw" in event.record

    def test_parse_event_too_short_returns_none(self):
        raw = 'EVENT orders'
        event = GigiSubscriber._parse_event(raw)
        assert event is None

    def test_parse_notice(self):
        raw = "NOTICE orders lagged=12"
        notice = GigiSubscriber._parse_notice(raw)
        assert notice["bundle"] == "orders"
        assert notice["lagged"] == 12

    def test_parse_notice_no_lag(self):
        raw = "NOTICE orders"
        notice = GigiSubscriber._parse_notice(raw)
        assert notice["lagged"] == 0


# ── GigiSubscriber unit tests (mocked WS) ────────────────────────────────────


@pytest.mark.asyncio
class TestGigiSubscriber:
    async def test_not_connected_raises(self):
        sub = GigiSubscriber("ws://localhost:3142/ws")
        with pytest.raises(GigiError, match="Not connected"):
            await sub.subscribe("events")

    async def test_dispatch_event_queues_event(self):
        sub = GigiSubscriber("ws://localhost:3142/ws")
        sub._connected = True
        raw = 'EVENT orders insert {"id":"o1"} K=0.1'
        sub._dispatch(raw)
        assert sub._event_queue.qsize() == 1
        event = await sub._event_queue.get()
        assert event.bundle == "orders"
        assert event.op == "insert"

    async def test_dispatch_notice_queues_notice(self):
        sub = GigiSubscriber("ws://localhost:3142/ws")
        sub._connected = True
        sub._dispatch("NOTICE orders lagged=5")
        assert sub._notice_queue.qsize() == 1
        notice = await sub._notice_queue.get()
        assert notice["lagged"] == 5

    async def test_dispatch_ignores_unknown(self):
        sub = GigiSubscriber("ws://localhost:3142/ws")
        sub._connected = True
        sub._dispatch("PONG")
        sub._dispatch("SUBSCRIBED orders filters=0")
        assert sub._event_queue.qsize() == 0
        assert sub._notice_queue.qsize() == 0

    async def test_events_iterator_yields_from_queue(self):
        sub = GigiSubscriber("ws://localhost:3142/ws")
        sub._connected = False  # stop after queue drains

        # Pre-populate the queue
        e = SubscriptionEvent(bundle="x", op="insert", record={"a": 1}, curvature=0.0)
        sub._event_queue.put_nowait(e)

        results = []
        async for event in sub.events():
            results.append(event)

        assert len(results) == 1
        assert results[0].record == {"a": 1}

    async def test_context_manager_calls_connect_disconnect(self):
        sub = GigiSubscriber("ws://localhost:3142/ws")

        connect_called = False
        disconnect_called = False

        async def fake_connect():
            nonlocal connect_called
            connect_called = True
            sub._connected = True

        async def fake_disconnect():
            nonlocal disconnect_called
            disconnect_called = True

        sub.connect = fake_connect
        sub.disconnect = fake_disconnect

        async with sub:
            pass

        assert connect_called
        assert disconnect_called


# ── Anomaly Detection v0.8.0 ──────────────────────────────────────────────────


ANOMALY_RESPONSE = {
    "bundle": "weather",
    "threshold_sigma": 2.0,
    "k_mean": 0.12,
    "k_std": 0.04,
    "k_threshold": 0.20,
    "total_records": 7320,
    "anomaly_count": 4,
    "anomalies": [
        {"record": {"id": 14, "city": "Moscow", "temp_c": -65.0},
         "local_curvature": 0.51, "z_score": 9.75, "confidence": 0.66,
         "deviation_norm": 1, "deviation_distance": 1.23, "neighbourhood_size": 366,
         "contributing_fields": ["temp_c"]},
        {"record": {"id": 545, "city": "Dubai", "humidity_pct": 1.5},
         "local_curvature": 0.48, "z_score": 9.0, "confidence": 0.68,
         "deviation_norm": 1, "deviation_distance": 1.18, "neighbourhood_size": 366,
         "contributing_fields": ["humidity_pct"]},
    ],
}


class TestAnomalyDetection:
    @resp_lib.activate
    def test_anomalies_returns_dict(self):
        resp_lib.post(f"{BASE_URL}/v1/bundles/weather/anomalies",
                      json=ANOMALY_RESPONSE, status=200)
        result = client().anomalies("weather")
        assert result["bundle"] == "weather"
        assert isinstance(result["anomalies"], list)
        assert result["anomaly_count"] == 4

    @resp_lib.activate
    def test_anomalies_default_sigma(self):
        resp_lib.post(f"{BASE_URL}/v1/bundles/weather/anomalies",
                      json=ANOMALY_RESPONSE, status=200)
        client().anomalies("weather")
        body = json.loads(resp_lib.calls[0].request.body)
        assert body["threshold_sigma"] == 2.0

    @resp_lib.activate
    def test_anomalies_custom_sigma(self):
        resp_lib.post(f"{BASE_URL}/v1/bundles/weather/anomalies",
                      json=ANOMALY_RESPONSE, status=200)
        client().anomalies("weather", threshold_sigma=3.0)
        body = json.loads(resp_lib.calls[0].request.body)
        assert body["threshold_sigma"] == 3.0

    @resp_lib.activate
    def test_anomalies_with_filters(self):
        resp_lib.post(f"{BASE_URL}/v1/bundles/weather/anomalies",
                      json=ANOMALY_RESPONSE, status=200)
        f = [{"field": "city", "op": "eq", "value": "Moscow"}]
        client().anomalies("weather", filters=f)
        body = json.loads(resp_lib.calls[0].request.body)
        assert body["filters"] == f

    @resp_lib.activate
    def test_anomalies_limit_sent(self):
        resp_lib.post(f"{BASE_URL}/v1/bundles/weather/anomalies",
                      json=ANOMALY_RESPONSE, status=200)
        client().anomalies("weather", limit=50)
        body = json.loads(resp_lib.calls[0].request.body)
        assert body["limit"] == 50

    @resp_lib.activate
    def test_bundle_health_returns_dict(self):
        health = {
            "bundle": "weather", "record_count": 7320,
            "k_global": 0.14, "k_mean": 0.12, "k_std": 0.04,
            "k_threshold_2s": 0.20, "k_threshold_3s": 0.24,
            "confidence": 0.88, "anomaly_rate_2s": 0.0005,
            "per_field": [{"field": "temp_c", "k": 0.18, "variance": 110.0, "range": 65.0}],
        }
        resp_lib.add(resp_lib.GET, f"{BASE_URL}/v1/bundles/weather/health", json=health)
        result = client().bundle_health("weather")
        assert result["record_count"] == 7320
        assert isinstance(result["per_field"], list)

    @resp_lib.activate
    def test_predict_returns_predictions(self):
        resp = {
            "bundle": "weather", "group_by": "city", "field": "temp_c",
            "predictions": [{"group": "Moscow", "count": 366, "mean": -5.0, "std_dev": 6.5, "volatility_index": 1.3}],
        }
        resp_lib.post(f"{BASE_URL}/v1/bundles/weather/predict", json=resp, status=200)
        result = client().predict("weather", "city", "temp_c")
        assert result["group_by"] == "city"
        assert len(result["predictions"]) >= 1

    @resp_lib.activate
    def test_predict_sends_correct_body(self):
        resp_lib.post(f"{BASE_URL}/v1/bundles/weather/predict",
                      json={"bundle": "weather", "group_by": "city", "field": "temp_c", "predictions": []},
                      status=200)
        client().predict("weather", "city", "temp_c")
        body = json.loads(resp_lib.calls[0].request.body)
        assert body == {"group_by": "city", "field": "temp_c"}

    @resp_lib.activate
    def test_field_anomalies_returns_dict(self):
        resp = {
            "bundle": "weather", "field": "temp_c",
            "threshold_sigma": 2.0, "anomaly_count": 1,
            "anomalies": [ANOMALY_RESPONSE["anomalies"][0]],
        }
        resp_lib.post(f"{BASE_URL}/v1/bundles/weather/anomalies/field", json=resp, status=200)
        result = client().field_anomalies("weather", "temp_c")
        assert result["field"] == "temp_c"
        assert result["anomaly_count"] == 1

    @resp_lib.activate
    def test_field_anomalies_sends_correct_body(self):
        resp_lib.post(f"{BASE_URL}/v1/bundles/weather/anomalies/field",
                      json={"bundle": "weather", "field": "temp_c", "threshold_sigma": 2.0, "anomaly_count": 0, "anomalies": []},
                      status=200)
        client().field_anomalies("weather", "temp_c", threshold_sigma=3.0, limit=25)
        body = json.loads(resp_lib.calls[0].request.body)
        assert body["field"] == "temp_c"
        assert body["threshold_sigma"] == 3.0
        assert body["limit"] == 25
