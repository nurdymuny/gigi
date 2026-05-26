//! GIGI Observability — structured logging and live metrics.
//!
//! Primary storage format: DHOOM bundles (`_gigi_*`) — schema-once, delta-encoded
//! timestamps, interned strings, native GQL queryable. Phase 2 will wire the
//! LogIngester to write into the Engine. Phase 1 delivers stdout JSON + metrics.
//!
//! Architecture:
//!   Logger      — cheap Clone, fire-and-forget emit via unbounded mpsc channel
//!   LogIngester — tokio task: reads channel, writes JSON to stdout
//!   Metrics     — atomic counters + rolling percentile window for /v1/metrics

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

// ── Constants ─────────────────────────────────────────────────────────────────

pub const GIGI_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Default slow-query threshold: 1 second.
pub const DEFAULT_SLOW_QUERY_US: u64 = 1_000_000;

// ── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogCategory {
    Query,
    Ingest,
    Wal,
    Connection,
    Stream,
    Bundle,
    Anomaly,
    Audit,
    System,
    Slow,
}

// ── Geometric Fields ──────────────────────────────────────────────────────────

/// Geometric analysis fields attached to query and ingest events.
/// Fields are `Option<f64>` — `null` when not computed, not omitted, so operators
/// can count "queries without geometric analysis" in dashboards.
#[derive(Debug, Clone, Default, Serialize)]
pub struct GeometricFields {
    pub kl_forward:      Option<f64>,
    pub kl_reverse:      Option<f64>,
    pub jensen_shannon:  Option<f64>,
    pub fields_compared: Option<u32>,
    pub ricci:           Option<f64>,
    pub k_global:        Option<f64>,
    pub coherence:       Option<f64>,
    // ingest-specific — only serialized when present
    #[serde(skip_serializing_if = "Option::is_none")]
    pub k_before:           Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub k_after:            Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub k_delta:            Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anomaly_triggered:  Option<bool>,
}

// ── Log Event ─────────────────────────────────────────────────────────────────

/// A single structured log event. All events share the base fields; event-specific
/// fields are merged flat into the JSON output via `#[serde(flatten)]`.
#[derive(Debug, Clone, Serialize)]
pub struct LogEvent {
    /// ISO-8601 UTC timestamp with microsecond precision.
    pub ts:       String,
    /// Epoch microseconds — skipped from JSON output, used by Phase 2 bundle inserts.
    #[serde(skip)]
    pub ts_us:    u64,
    pub level:    LogLevel,
    pub category: LogCategory,
    /// Dot-namespaced event name, e.g. "query.complete".
    pub event:    &'static str,
    pub instance: String,
    pub version:  &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id:  Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_us: Option<u64>,
    /// Event-specific fields merged flat into the JSON output (no "payload" wrapper).
    #[serde(flatten)]
    pub payload: serde_json::Map<String, serde_json::Value>,
}

impl LogEvent {
    pub fn new(
        level:    LogLevel,
        category: LogCategory,
        event:    &'static str,
        instance: &str,
    ) -> Self {
        let ts_us = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;
        Self {
            ts:          now_iso8601(),
            ts_us,
            level,
            category,
            event,
            instance:    instance.to_string(),
            version:     GIGI_VERSION,
            request_id:  None,
            duration_us: None,
            payload:     serde_json::Map::new(),
        }
    }

    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    pub fn with_duration_us(mut self, us: u64) -> Self {
        self.duration_us = Some(us);
        self
    }

    /// Set a payload field. `v` must impl `Into<serde_json::Value>`.
    pub fn field(mut self, k: &str, v: impl Into<serde_json::Value>) -> Self {
        self.payload.insert(k.to_string(), v.into());
        self
    }

    /// Serialize to a single-line JSON string.
    pub fn serialize_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

// ── Log Config ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LogConfig {
    pub level:                  LogLevel,
    pub slow_query_threshold_us: u64,
    pub stdout_enabled:         bool,
    pub internal_bundles_enabled: bool,
    // per-category toggles
    pub cat_query:      bool,
    pub cat_ingest:     bool,
    pub cat_wal:        bool,
    pub cat_connection: bool,
    pub cat_stream:     bool,
    pub cat_bundle:     bool,
    pub cat_anomaly:    bool,
    pub cat_audit:      bool,
    pub cat_system:     bool,
    /// Per-bundle retention in days. Key = bundle name (e.g. "_gigi_query_log").
    /// Missing key → use the category default (see `retention_days()`).
    pub retention_overrides: std::collections::HashMap<String, u32>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level:                    LogLevel::Info,
            slow_query_threshold_us:  DEFAULT_SLOW_QUERY_US,
            stdout_enabled:           true,
            internal_bundles_enabled: true,
            cat_query:      true,
            cat_ingest:     true,
            cat_wal:        true,
            cat_connection: false, // off by default: high volume, low signal
            cat_stream:     true,
            cat_bundle:     true,
            cat_anomaly:    true,
            cat_audit:      true,  // audit is always emitted regardless of this flag
            cat_system:     true,
            retention_overrides: std::collections::HashMap::new(),
        }
    }
}

impl LogConfig {
    pub fn category_enabled(&self, cat: LogCategory) -> bool {
        match cat {
            LogCategory::Query | LogCategory::Slow => self.cat_query,
            LogCategory::Ingest     => self.cat_ingest,
            LogCategory::Wal        => self.cat_wal,
            LogCategory::Connection => self.cat_connection,
            LogCategory::Stream     => self.cat_stream,
            LogCategory::Bundle     => self.cat_bundle,
            LogCategory::Anomaly    => self.cat_anomaly,
            LogCategory::Audit      => true, // audit is always on
            LogCategory::System     => self.cat_system,
        }
    }

    /// Retention period in days for a `_gigi_*` bundle.
    /// Spec §6 defaults: audit=365, anomaly/slow=90, query/ingest/wal/bundle/system=30,
    /// stream/connection=7.
    pub fn retention_days(&self, bundle: &str) -> u32 {
        if let Some(&days) = self.retention_overrides.get(bundle) {
            return days;
        }
        match bundle {
            "_gigi_audit_log"                       => 365,
            "_gigi_anomaly_log" | "_gigi_slow_log"  => 90,
            "_gigi_stream_log" | "_gigi_conn_log"   => 7,
            "_gigi_wal_log" | "_gigi_ingest_log"    => 14,
            _                                       => 30, // query, bundle, system
        }
    }

    pub fn set_retention(&mut self, bundle: &str, days: u32) {
        self.retention_overrides.insert(bundle.to_string(), days);
    }
}

// ── Metrics ───────────────────────────────────────────────────────────────────

/// Atomic counters exposed at `GET /v1/metrics`.
pub struct Metrics {
    pub queries_total:          AtomicU64,
    pub queries_error:          AtomicU64,
    pub queries_slow:           AtomicU64,
    pub records_ingested:       AtomicU64,
    pub bytes_ingested:         AtomicU64,
    pub anomalies_total:        AtomicU64,
    pub http_connections_total: AtomicU64,
    pub ws_connections_total:   AtomicU64,
    // ── Brain/fit cache observability (S1 wave 1 §E, per
    // Marcella's 2026-05-27 caveat: "make sure the cache exports
    // hit/miss/eviction counters so we can spot the LRU-switch
    // signal"). Lock-free counters on the hot path.
    pub brain_cache_hits:       AtomicU64,
    pub brain_cache_misses:     AtomicU64,
    pub brain_cache_evictions:  AtomicU64,
    /// Microseconds spent in fit_full / fit_diagonal / fit_isotropic
    /// across all brain calls. Lets us read "how much fit work has
    /// been done across the process lifetime" — a cold-path
    /// indicator. Per-call fit_ms is also surfaced in response
    /// headers (X-Brain-Fit-Us).
    pub brain_fit_total_us:     AtomicU64,
    /// Microseconds spent in the per-call brain compute (Langevin
    /// trajectory, kernel density, etc.) post-fit. Tracks the
    /// hot-path work after a cache hit.
    pub brain_compute_total_us: AtomicU64,
    /// Sliding window of recent query durations for p50/p95/p99.
    pub durations: Mutex<DurationWindow>,
    /// Per-statement-type query counts.
    pub by_type: Mutex<std::collections::HashMap<String, u64>>,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            queries_total:          AtomicU64::new(0),
            queries_error:          AtomicU64::new(0),
            queries_slow:           AtomicU64::new(0),
            records_ingested:       AtomicU64::new(0),
            bytes_ingested:         AtomicU64::new(0),
            anomalies_total:        AtomicU64::new(0),
            http_connections_total: AtomicU64::new(0),
            ws_connections_total:   AtomicU64::new(0),
            brain_cache_hits:       AtomicU64::new(0),
            brain_cache_misses:     AtomicU64::new(0),
            brain_cache_evictions:  AtomicU64::new(0),
            brain_fit_total_us:     AtomicU64::new(0),
            brain_compute_total_us: AtomicU64::new(0),
            durations: Mutex::new(DurationWindow::new(4096)),
            by_type:   Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Record a brain cache hit (lock-free, hot-path).
    pub fn record_brain_cache_hit(&self) {
        self.brain_cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a brain cache miss (lock-free).
    pub fn record_brain_cache_miss(&self) {
        self.brain_cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a brain cache eviction (when insert past capacity).
    pub fn record_brain_cache_eviction(&self) {
        self.brain_cache_evictions.fetch_add(1, Ordering::Relaxed);
    }

    /// Record fit + compute timing for a single brain call.
    /// `fit_us` is 0 on a cache hit (no fit performed).
    pub fn record_brain_timing(&self, fit_us: u64, compute_us: u64) {
        if fit_us > 0 {
            self.brain_fit_total_us.fetch_add(fit_us, Ordering::Relaxed);
        }
        self.brain_compute_total_us.fetch_add(compute_us, Ordering::Relaxed);
    }

    pub fn record_query(&self, duration_us: u64, stmt_type: &str, is_slow: bool, is_error: bool) {
        self.queries_total.fetch_add(1, Ordering::Relaxed);
        if is_error { self.queries_error.fetch_add(1, Ordering::Relaxed); }
        if is_slow  { self.queries_slow.fetch_add(1, Ordering::Relaxed); }
        if let Ok(mut d) = self.durations.lock() { d.push(duration_us); }
        if let Ok(mut bt) = self.by_type.lock() {
            *bt.entry(stmt_type.to_string()).or_insert(0) += 1;
        }
    }

    pub fn record_ingest(&self, records: u64, bytes: u64) {
        self.records_ingested.fetch_add(records, Ordering::Relaxed);
        self.bytes_ingested.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn record_anomaly(&self) {
        self.anomalies_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Returns (p50_us, p95_us, p99_us).
    pub fn percentiles(&self) -> (u64, u64, u64) {
        self.durations.lock().map(|d| d.percentiles()).unwrap_or((0, 0, 0))
    }

    /// Snapshot the by_type map for serialization.
    pub fn by_type_snapshot(&self) -> std::collections::HashMap<String, u64> {
        self.by_type.lock().map(|m| m.clone()).unwrap_or_default()
    }
}

impl Default for Metrics {
    fn default() -> Self { Self::new() }
}

/// Fixed-capacity sliding window for query durations (used for p50/p95/p99).
pub struct DurationWindow {
    buf:      Vec<u64>,
    capacity: usize,
    pos:      usize,
    count:    usize,
}

impl DurationWindow {
    pub fn new(capacity: usize) -> Self {
        Self { buf: vec![0u64; capacity], capacity, pos: 0, count: 0 }
    }

    pub fn push(&mut self, us: u64) {
        self.buf[self.pos % self.capacity] = us;
        self.pos += 1;
        self.count = (self.count + 1).min(self.capacity);
    }

    /// Returns (p50, p95, p99) in microseconds.
    pub fn percentiles(&self) -> (u64, u64, u64) {
        if self.count == 0 { return (0, 0, 0); }
        let mut sorted: Vec<u64> = self.buf[..self.count].to_vec();
        sorted.sort_unstable();
        let p = |pct: f64| -> u64 {
            let idx = ((pct / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
            sorted[idx.min(sorted.len() - 1)]
        };
        (p(50.0), p(95.0), p(99.0))
    }
}

// ── Logger ────────────────────────────────────────────────────────────────────

/// Cheap-to-clone log emitter. `emit` is fire-and-forget — never blocks the caller.
#[derive(Clone)]
pub struct Logger {
    tx:     UnboundedSender<LogEvent>,
    config: Arc<RwLock<LogConfig>>,
    pub instance: String,
}

impl Logger {
    /// Create a paired (Logger, LogIngester). Spawn the ingester as a tokio task.
    pub fn new(config: LogConfig, instance: impl Into<String>) -> (Self, LogIngester) {
        let (tx, rx) = mpsc::unbounded_channel();
        let cfg = Arc::new(RwLock::new(config));
        let logger = Logger { tx, config: cfg.clone(), instance: instance.into() };
        let ingester = LogIngester { rx, config: cfg, bundle_tx: None };
        (logger, ingester)
    }

    /// Like `new`, but also returns a receiver for log events to be written into
    /// `_gigi_*` system bundles. Wire the receiver to `log_bundle_writer` in
    /// gigi_stream.rs after `Arc<StreamState>` is constructed.
    pub fn new_with_bundle_channel(
        config:   LogConfig,
        instance: impl Into<String>,
    ) -> (Self, LogIngester, UnboundedReceiver<LogEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let (btx, brx) = mpsc::unbounded_channel();
        let cfg = Arc::new(RwLock::new(config));
        let logger = Logger { tx, config: cfg.clone(), instance: instance.into() };
        let ingester = LogIngester { rx, config: cfg, bundle_tx: Some(btx) };
        (logger, ingester, brx)
    }

    /// Emit a log event. Non-blocking. Drops event silently if category is disabled
    /// or channel is closed.
    pub fn emit(&self, event: LogEvent) {
        let enabled = self.config.read()
            .map(|c| c.category_enabled(event.category))
            .unwrap_or(true);
        if enabled {
            let _ = self.tx.send(event);
        }
    }

    /// Current slow-query threshold in microseconds.
    pub fn slow_threshold_us(&self) -> u64 {
        self.config.read()
            .map(|c| c.slow_query_threshold_us)
            .unwrap_or(DEFAULT_SLOW_QUERY_US)
    }

    /// Read a snapshot of the current LogConfig.
    pub fn get_config(&self) -> LogConfig {
        self.config.read().map(|c| c.clone()).unwrap_or_default()
    }

    /// Replace the current LogConfig atomically. Audit category cannot be disabled.
    pub fn update_config(&self, mut new: LogConfig) {
        new.cat_audit = true; // audit is always on
        if let Ok(mut cfg) = self.config.write() {
            *cfg = new;
        }
    }

    // ── Event Builders ────────────────────────────────────────────────────────

    /// Build and emit a `query.complete` event. Returns the event for testing.
    #[allow(clippy::too_many_arguments)]
    pub fn query_complete(
        &self,
        request_id:      &str,
        source:          &str,
        stmt_type:       &str,
        raw_gql:         &str,
        duration_us:     u64,
        parse_us:        u64,
        exec_us:         u64,
        bundles:         &[String],
        records_scanned: u64,
        records_returned: u64,
        bytes_read:      u64,
        bytes_returned:  u64,
        cache_hit:       bool,
        geo:             Option<GeometricFields>,
        error:           Option<&str>,
    ) -> LogEvent {
        let slow = duration_us >= self.slow_threshold_us();
        let level = if error.is_some() { LogLevel::Error } else { LogLevel::Info };

        let e = LogEvent::new(level, LogCategory::Query, "query.complete", &self.instance)
            .with_request_id(request_id)
            .with_duration_us(duration_us)
            .field("source",           source)
            .field("statement_type",   stmt_type)
            .field("raw_gql",          raw_gql)
            .field("parse_us",         parse_us)
            .field("exec_us",          exec_us)
            .field("bundles_accessed", serde_json::Value::Array(
                bundles.iter().map(|b| serde_json::Value::String(b.clone())).collect()
            ))
            .field("records_scanned",   records_scanned)
            .field("records_returned",  records_returned)
            .field("bytes_read",        bytes_read)
            .field("bytes_returned",    bytes_returned)
            .field("cache_hit",         cache_hit)
            .field("slow",              slow);

        // Spec §3.1: geometric fields nested under "geometric" block — GIGI's exclusive differentiator
        let geo_block = match geo {
            Some(g) => serde_json::json!({
                "kl_forward":      g.kl_forward,
                "kl_reverse":      g.kl_reverse,
                "jensen_shannon":  g.jensen_shannon,
                "fields_compared": g.fields_compared,
                "ricci":           g.ricci,
                "k_global":        g.k_global,
                "coherence":       g.coherence,
            }),
            None => serde_json::json!({
                "kl_forward":      null,
                "kl_reverse":      null,
                "jensen_shannon":  null,
                "fields_compared": null,
                "ricci":           null,
                "k_global":        null,
                "coherence":       null,
            }),
        };
        let e = e.field("geometric", geo_block);

        match error {
            Some(err) => e.field("error", err),
            None      => e.field("error", serde_json::Value::Null),
        }
    }

    pub fn query_error(
        &self,
        request_id:  &str,
        raw_gql:     &str,
        duration_us: u64,
        error_class: &str,
        error_msg:   &str,
        http_status: u16,
    ) -> LogEvent {
        LogEvent::new(LogLevel::Error, LogCategory::Query, "query.error", &self.instance)
            .with_request_id(request_id)
            .with_duration_us(duration_us)
            .field("raw_gql",     raw_gql)
            .field("error_class", error_class)
            .field("error_msg",   error_msg)
            .field("http_status", http_status)
    }

    /// Spec §3.1: emit before parsing. Lets operators detect queries that started
    /// but never completed (timeout / OOM / crash).
    pub fn query_start(
        &self,
        request_id: &str,
        source:     &str,
        raw_gql:    &str,
        client_ip:  &str,
        user_agent: &str,
    ) -> LogEvent {
        LogEvent::new(LogLevel::Info, LogCategory::Query, "query.start", &self.instance)
            .with_request_id(request_id)
            .field("source",     source)
            .field("raw_gql",    raw_gql)
            .field("client_ip",  client_ip)
            .field("user_agent", user_agent)
    }

    // ── Stream event builders (§3.5) ──────────────────────────────────────────

    pub fn stream_subscribe(
        &self,
        connection_id: &str,
        bundle:        &str,
        client_ip:     &str,
        mode:          &str,
    ) -> LogEvent {
        LogEvent::new(LogLevel::Info, LogCategory::Stream, "stream.subscribe", &self.instance)
            .field("connection_id", connection_id)
            .field("bundle",        bundle)
            .field("client_ip",     client_ip)
            .field("mode",          mode)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn stream_push(
        &self,
        connection_id: &str,
        bundle:        &str,
        message_seq:   u64,
        duration_us:   u64,
        bytes_sent:    u64,
        k_global:      f64,
        is_anomaly:    bool,
        z_score:       f64,
    ) -> LogEvent {
        LogEvent::new(LogLevel::Info, LogCategory::Stream, "stream.push", &self.instance)
            .with_duration_us(duration_us)
            .field("connection_id", connection_id)
            .field("bundle",        bundle)
            .field("message_seq",   message_seq)
            .field("bytes_sent",    bytes_sent)
            .field("k_global",      k_global)
            .field("is_anomaly",    is_anomaly)
            .field("z_score",       z_score)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn stream_anomaly_push(
        &self,
        connection_id:       &str,
        bundle:              &str,
        message_seq:         u64,
        k_global:            f64,
        k_threshold_3s:      f64,
        z_score:             f64,
        contributing_fields: &[String],
        duration_us:         u64,
    ) -> LogEvent {
        LogEvent::new(LogLevel::Warn, LogCategory::Stream, "stream.anomaly_push", &self.instance)
            .with_duration_us(duration_us)
            .field("connection_id",      connection_id)
            .field("bundle",             bundle)
            .field("message_seq",        message_seq)
            .field("k_global",           k_global)
            .field("k_threshold_3s",     k_threshold_3s)
            .field("z_score",            z_score)
            .field("is_anomaly",         true)
            .field("contributing_fields", serde_json::json!(contributing_fields))
    }

    pub fn stream_disconnect(
        &self,
        connection_id:      &str,
        bundle:             &str,
        session_duration_us: u64,
        messages_sent:      u64,
        anomalies_sent:     u64,
        reason:             &str,
    ) -> LogEvent {
        LogEvent::new(LogLevel::Info, LogCategory::Stream, "stream.disconnect", &self.instance)
            .field("connection_id",      connection_id)
            .field("bundle",             bundle)
            .field("session_duration_us", session_duration_us)
            .field("messages_sent",      messages_sent)
            .field("anomalies_sent",     anomalies_sent)
            .field("reason",             reason)
    }

    // ── Audit event builders (§3.8) ───────────────────────────────────────────

    pub fn audit_bundle_drop(
        &self,
        bundle:          &str,
        records_deleted: u64,
        bytes_freed:     u64,
        triggered_by:    &str,
        client_ip:       &str,
    ) -> LogEvent {
        LogEvent::new(LogLevel::Info, LogCategory::Audit, "audit.bundle_drop", &self.instance)
            .field("bundle",          bundle)
            .field("records_deleted", records_deleted)
            .field("bytes_freed",     bytes_freed)
            .field("triggered_by",    triggered_by)
            .field("client_ip",       client_ip)
            .field("outcome",         "success")
    }

    pub fn audit_log_level_change(
        &self,
        old_level: &str,
        new_level: &str,
        actor:     &str,
        outcome:   &str,
    ) -> LogEvent {
        LogEvent::new(LogLevel::Info, LogCategory::Audit, "audit.log_level_change", &self.instance)
            .field("old_level", old_level)
            .field("new_level", new_level)
            .field("actor",     actor)
            .field("outcome",   outcome)
    }

    pub fn query_slow(
        &self,
        request_id:       &str,
        stmt_type:        &str,
        raw_gql:          &str,
        duration_us:      u64,
        cache_warm_before: bool,
        cache_warm_after:  bool,
        note:             &str,
    ) -> LogEvent {
        let threshold = self.slow_threshold_us();
        LogEvent::new(LogLevel::Warn, LogCategory::Slow, "query.slow", &self.instance)
            .with_request_id(request_id)
            .with_duration_us(duration_us)
            .field("slow_threshold_us",  threshold)
            .field("statement_type",     stmt_type)
            .field("raw_gql",            raw_gql)
            .field("cache_warm_before",  cache_warm_before)
            .field("cache_warm_after",   cache_warm_after)
            .field("note",               note)
    }

    pub fn ingest_complete(
        &self,
        bundle:          &str,
        records_written: u64,
        bytes_written:   u64,
        duration_us:     u64,
        wal_synced:      bool,
        schema_changed:  bool,
        fields_added:    &[String],
        k_before:        Option<f64>,
        k_after:         Option<f64>,
    ) -> LogEvent {
        let k_delta = match (k_before, k_after) {
            (Some(b), Some(a)) => Some(a - b),
            _ => None,
        };
        LogEvent::new(LogLevel::Info, LogCategory::Ingest, "ingest.complete", &self.instance)
            .with_duration_us(duration_us)
            .field("bundle",          bundle)
            .field("records_written", records_written)
            .field("bytes_written",   bytes_written)
            .field("wal_synced",      wal_synced)
            .field("schema_changed",  schema_changed)
            .field("fields_added",    serde_json::json!(fields_added))
            .field("k_before",        serde_json::json!(k_before))
            .field("k_after",         serde_json::json!(k_after))
            .field("k_delta",         serde_json::json!(k_delta))
            .field("anomaly_triggered", false)
    }

    pub fn bundle_stats_cache_warm(
        &self,
        bundle:          &str,
        records_scanned: u64,
        fields_cached:   usize,
        duration_us:     u64,
        triggered_by:    &str,
    ) -> LogEvent {
        LogEvent::new(LogLevel::Info, LogCategory::Bundle, "bundle.stats_cache_warm", &self.instance)
            .with_duration_us(duration_us)
            .field("bundle",          bundle)
            .field("records_scanned", records_scanned)
            .field("fields_cached",   fields_cached)
            .field("triggered_by",    triggered_by)
    }

    pub fn bundle_create(
        &self,
        bundle:       &str,
        field_count:  usize,
        storage_type: &str,
        source:       &str,
    ) -> LogEvent {
        LogEvent::new(LogLevel::Info, LogCategory::Bundle, "bundle.create", &self.instance)
            .field("bundle",       bundle)
            .field("field_count",  field_count)
            .field("storage_type", storage_type)
            .field("source",       source)
    }

    pub fn bundle_drop(
        &self,
        bundle:          &str,
        records_deleted: u64,
        triggered_by:    &str,
        client_ip:       &str,
    ) -> LogEvent {
        LogEvent::new(LogLevel::Info, LogCategory::Bundle, "bundle.drop", &self.instance)
            .field("bundle",          bundle)
            .field("records_deleted", records_deleted)
            .field("triggered_by",    triggered_by)
            .field("client_ip",       client_ip)
    }

    pub fn system_startup(
        &self,
        data_path:        &str,
        bundles_loaded:   usize,
        wal_replayed:     usize,
        records_recovered: u64,
        duration_us:      u64,
        listen_addr:      &str,
    ) -> LogEvent {
        LogEvent::new(LogLevel::Info, LogCategory::System, "system.startup", &self.instance)
            .with_duration_us(duration_us)
            .field("data_path",        data_path)
            .field("bundles_loaded",   bundles_loaded)
            .field("wal_replayed",     wal_replayed)
            .field("records_recovered",records_recovered)
            .field("listen_addr",      listen_addr)
    }

    pub fn system_shutdown(
        &self,
        uptime_us:          u64,
        queries_served:     u64,
        records_ingested:   u64,
        anomalies_detected: u64,
        reason:             &str,
    ) -> LogEvent {
        LogEvent::new(LogLevel::Info, LogCategory::System, "system.shutdown", &self.instance)
            .field("uptime_us",          uptime_us)
            .field("queries_served",     queries_served)
            .field("records_ingested",   records_ingested)
            .field("anomalies_detected", anomalies_detected)
            .field("reason",             reason)
    }

    pub fn anomaly_detected(
        &self,
        bundle:              &str,
        record_id:           &str,
        k_record:            f64,
        k_mean:              f64,
        k_std:               f64,
        z_score:             f64,
        threshold_2s:        f64,
        threshold_3s:        f64,
        contributing_fields: &[String],
        detection_source:    &str,
        duration_us:         u64,
    ) -> LogEvent {
        let sigma_level: u8 = if z_score.abs() >= 3.0 { 3 }
                              else if z_score.abs() >= 2.0 { 2 }
                              else { 1 };
        LogEvent::new(LogLevel::Warn, LogCategory::Anomaly, "anomaly.detected", &self.instance)
            .with_duration_us(duration_us)
            .field("bundle",              bundle)
            .field("record_id",           record_id)
            .field("k_record",            k_record)
            .field("k_mean",              k_mean)
            .field("k_std",               k_std)
            .field("z_score",             z_score)
            .field("threshold_2s",        threshold_2s)
            .field("threshold_3s",        threshold_3s)
            .field("sigma_level",         sigma_level)
            .field("contributing_fields", serde_json::json!(contributing_fields))
            .field("detection_source",    detection_source)
    }

    pub fn audit(
        &self,
        event_name: &'static str,
        actor:      &str,
        client_ip:  &str,
        details:    serde_json::Map<String, serde_json::Value>,
        outcome:    &str,
    ) -> LogEvent {
        let mut e = LogEvent::new(LogLevel::Info, LogCategory::Audit, event_name, &self.instance)
            .field("actor",     actor)
            .field("client_ip", client_ip)
            .field("outcome",   outcome);
        for (k, v) in details { e.payload.insert(k, v); }
        e
    }

    pub fn wal_checkpoint(
        &self,
        bundle:          &str,
        records_flushed: u64,
        bytes_flushed:   u64,
        wal_size_before: u64,
        wal_size_after:  u64,
        duration_us:     u64,
    ) -> LogEvent {
        LogEvent::new(LogLevel::Info, LogCategory::Wal, "wal.checkpoint", &self.instance)
            .with_duration_us(duration_us)
            .field("bundle",          bundle)
            .field("records_flushed", records_flushed)
            .field("bytes_flushed",   bytes_flushed)
            .field("wal_size_before", wal_size_before)
            .field("wal_size_after",  wal_size_after)
    }

    pub fn wal_replay(
        &self,
        bundle:            &str,
        records_recovered: u64,
        duration_us:       u64,
        triggered_by:      &str,
    ) -> LogEvent {
        LogEvent::new(LogLevel::Info, LogCategory::Wal, "wal.replay", &self.instance)
            .with_duration_us(duration_us)
            .field("bundle",            bundle)
            .field("records_recovered", records_recovered)
            .field("triggered_by",      triggered_by)
    }

    pub fn wal_compaction(
        &self,
        bundle:            &str,
        segments_merged:   usize,
        size_before:       u64,
        size_after:        u64,
        compression_ratio: f64,
        duration_us:       u64,
    ) -> LogEvent {
        LogEvent::new(LogLevel::Info, LogCategory::Wal, "wal.compaction", &self.instance)
            .with_duration_us(duration_us)
            .field("bundle",            bundle)
            .field("segments_merged",   segments_merged)
            .field("size_before",       size_before)
            .field("size_after",        size_after)
            .field("compression_ratio", compression_ratio)
    }

    pub fn ingest_bulk(
        &self,
        bundle:        &str,
        records:       u64,
        bytes:         u64,
        duration_us:   u64,
        throughput_rps: f64,
        wal_synced:    bool,
        batches:       usize,
    ) -> LogEvent {
        LogEvent::new(LogLevel::Info, LogCategory::Ingest, "ingest.bulk", &self.instance)
            .with_duration_us(duration_us)
            .field("bundle",         bundle)
            .field("records_written",records)
            .field("bytes_written",  bytes)
            .field("throughput_rps", throughput_rps)
            .field("wal_synced",     wal_synced)
            .field("batches",        batches)
    }

    pub fn connection_open(
        &self,
        protocol:  &str,
        client_ip: &str,
        user_agent: &str,
    ) -> LogEvent {
        LogEvent::new(LogLevel::Info, LogCategory::Connection, "connection.open", &self.instance)
            .field("protocol",   protocol)
            .field("client_ip",  client_ip)
            .field("user_agent", user_agent)
    }

    pub fn connection_close(
        &self,
        protocol:            &str,
        client_ip:           &str,
        session_duration_us: u64,
        requests_served:     u64,
        bytes_sent:          u64,
        bytes_received:      u64,
    ) -> LogEvent {
        LogEvent::new(LogLevel::Info, LogCategory::Connection, "connection.close", &self.instance)
            .field("protocol",            protocol)
            .field("client_ip",           client_ip)
            .field("session_duration_us", session_duration_us)
            .field("requests_served",     requests_served)
            .field("bytes_sent",          bytes_sent)
            .field("bytes_received",      bytes_received)
    }
}

// ── LogIngester ───────────────────────────────────────────────────────────────

/// Async log consumer. Receives events from the Logger channel and writes them
/// to configured destinations. Spawn as a dedicated tokio task.
pub struct LogIngester {
    rx:        UnboundedReceiver<LogEvent>,
    config:    Arc<RwLock<LogConfig>>,
    /// Optional forwarding channel for Phase 2 bundle inserts.
    bundle_tx: Option<UnboundedSender<LogEvent>>,
}

impl LogIngester {
    /// Consume all events until the Logger is dropped. Run as `tokio::spawn(ingester.run())`.
    pub async fn run(mut self) {
        while let Some(event) = self.rx.recv().await {
            let stdout_enabled = self.config.read()
                .map(|c| c.stdout_enabled)
                .unwrap_or(true);
            if stdout_enabled {
                println!("{}", event.serialize_json());
            }
            // Phase 2: forward to log_bundle_writer task in gigi_stream.
            if let Some(ref btx) = self.bundle_tx {
                let _ = btx.send(event);
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Format SystemTime as ISO-8601 UTC with microsecond precision. No external deps.
pub fn now_iso8601() -> String {
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs   = d.as_secs();
    let micros = d.subsec_micros();
    let (y, mo, day, h, mi, s) = secs_to_ymd_hms(secs);
    format!("{y:04}-{mo:02}-{day:02}T{h:02}:{mi:02}:{s:02}.{micros:06}Z")
}

fn secs_to_ymd_hms(secs: u64) -> (u32, u32, u32, u32, u32, u32) {
    let sec  = (secs % 60)      as u32;
    let min  = ((secs / 60) % 60) as u32;
    let hour = ((secs / 3600) % 24) as u32;
    let days = (secs / 86400) as u32;

    let mut y = 1970u32;
    let mut rem = days;
    loop {
        let dy = if is_leap(y) { 366 } else { 365 };
        if rem < dy { break; }
        rem -= dy;
        y += 1;
    }
    let months = if is_leap(y) {
        [31u32, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31u32, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut mo = 1u32;
    for &dm in &months {
        if rem < dm { break; }
        rem -= dm;
        mo += 1;
    }
    (y, mo, rem + 1, hour, min, sec)
}

fn is_leap(y: u32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Generate a monotonic request ID (24 hex chars). No UUID dep required.
static REQ_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn new_request_id() -> String {
    let ts  = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_micros() as u64;
    let seq = REQ_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{ts:016x}{seq:08x}")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn inst() -> &'static str { "test-instance" }

    // ── Serialization: base fields ────────────────────────────────────────────

    #[test]
    fn test_base_fields_present() {
        let e = LogEvent::new(LogLevel::Info, LogCategory::Query, "query.complete", inst());
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        assert_eq!(j["level"],    "INFO");
        assert_eq!(j["category"], "query");
        assert_eq!(j["event"],    "query.complete");
        assert_eq!(j["version"],  GIGI_VERSION);
        assert!(j["ts"].is_string());
        assert!(j["ts"].as_str().unwrap().ends_with('Z'));
        assert_eq!(j["instance"], "test-instance");
    }

    #[test]
    fn test_all_log_levels_serialize_uppercase() {
        for (level, expected) in [
            (LogLevel::Trace, "TRACE"), (LogLevel::Debug, "DEBUG"),
            (LogLevel::Info,  "INFO"),  (LogLevel::Warn,  "WARN"),
            (LogLevel::Error, "ERROR"), (LogLevel::Fatal, "FATAL"),
        ] {
            let j: Value = serde_json::from_str(
                &LogEvent::new(level, LogCategory::System, "test", inst()).serialize_json()
            ).unwrap();
            assert_eq!(j["level"], expected, "level {level:?}");
        }
    }

    #[test]
    fn test_all_categories_serialize_snake_case() {
        for (cat, expected) in [
            (LogCategory::Query,      "query"),
            (LogCategory::Ingest,     "ingest"),
            (LogCategory::Wal,        "wal"),
            (LogCategory::Connection, "connection"),
            (LogCategory::Stream,     "stream"),
            (LogCategory::Bundle,     "bundle"),
            (LogCategory::Anomaly,    "anomaly"),
            (LogCategory::Audit,      "audit"),
            (LogCategory::System,     "system"),
            (LogCategory::Slow,       "slow"),
        ] {
            let j: Value = serde_json::from_str(
                &LogEvent::new(LogLevel::Info, cat, "test", inst()).serialize_json()
            ).unwrap();
            assert_eq!(j["category"], expected, "cat {cat:?}");
        }
    }

    #[test]
    fn test_duration_us_present_when_set() {
        let j: Value = serde_json::from_str(
            &LogEvent::new(LogLevel::Info, LogCategory::Query, "test", inst())
                .with_duration_us(221_043)
                .serialize_json()
        ).unwrap();
        assert_eq!(j["duration_us"], 221_043);
    }

    #[test]
    fn test_duration_us_absent_when_not_set() {
        let j: Value = serde_json::from_str(
            &LogEvent::new(LogLevel::Info, LogCategory::Query, "test", inst()).serialize_json()
        ).unwrap();
        assert!(j.get("duration_us").is_none(), "duration_us must be absent");
    }

    #[test]
    fn test_request_id_present_when_set() {
        let j: Value = serde_json::from_str(
            &LogEvent::new(LogLevel::Info, LogCategory::Query, "test", inst())
                .with_request_id("abc-123")
                .serialize_json()
        ).unwrap();
        assert_eq!(j["request_id"], "abc-123");
    }

    #[test]
    fn test_request_id_absent_when_not_set() {
        let j: Value = serde_json::from_str(
            &LogEvent::new(LogLevel::Info, LogCategory::Query, "test", inst()).serialize_json()
        ).unwrap();
        assert!(j.get("request_id").is_none(), "request_id must be absent");
    }

    // ── Payload flattening ────────────────────────────────────────────────────

    #[test]
    fn test_payload_fields_flatten_no_nesting() {
        let j: Value = serde_json::from_str(
            &LogEvent::new(LogLevel::Info, LogCategory::Query, "query.complete", inst())
                .field("statement_type",   "DIVERGENCE")
                .field("records_returned", 1u64)
                .serialize_json()
        ).unwrap();
        assert_eq!(j["statement_type"],   "DIVERGENCE");
        assert_eq!(j["records_returned"], 1);
        assert!(j.get("payload").is_none(), "payload must NOT be nested");
    }

    #[test]
    fn test_geometric_null_fields_serialize_as_null_not_absent() {
        // null must be PRESENT inside "geometric" (not omitted, not at root)
        // enables counting queries without geo analysis in dashboards
        let j: Value = serde_json::from_str(
            &LogEvent::new(LogLevel::Info, LogCategory::Query, "query.complete", inst())
                .field("geometric", serde_json::json!({
                    "kl_forward":     null,
                    "kl_reverse":     null,
                    "jensen_shannon": null,
                    "fields_compared":null,
                    "ricci":          null,
                    "k_global":       null,
                    "coherence":      null
                }))
                .serialize_json()
        ).unwrap();
        let geo = &j["geometric"];
        assert!(geo.is_object(),                       "geometric must be an object");
        assert!(geo.get("kl_forward").is_some(),       "kl_forward must be present inside geometric");
        assert!(geo.get("jensen_shannon").is_some(),    "jensen_shannon must be present inside geometric");
        assert_eq!(geo["kl_forward"], Value::Null);
        // Root must be clean
        assert!(j.get("kl_forward").is_none(),         "kl_forward must NOT be at root");
    }

    #[test]
    fn test_geometric_values_round_trip() {
        let j: Value = serde_json::from_str(
            &LogEvent::new(LogLevel::Info, LogCategory::Query, "test", inst())
                .field("geometric", serde_json::json!({
                    "kl_forward":     0.3944f64,
                    "jensen_shannon": 0.0647f64,
                    "fields_compared": 1u32
                }))
                .serialize_json()
        ).unwrap();
        let geo = &j["geometric"];
        assert!((geo["kl_forward"].as_f64().unwrap() - 0.3944).abs() < 1e-9);
        assert_eq!(geo["fields_compared"], 1);
    }

    // ── Timestamp ─────────────────────────────────────────────────────────────

    #[test]
    fn test_now_iso8601_format() {
        let ts = now_iso8601();
        assert_eq!(ts.len(), 27, "expected 27 chars, got: {ts}");
        assert!(ts.contains('T'));
        assert!(ts.ends_with('Z'));
        let year: u32 = ts[..4].parse().unwrap();
        assert!(year >= 2024 && year <= 2100);
    }

    #[test]
    fn test_now_iso8601_monotonic() {
        let t1 = now_iso8601();
        let t2 = now_iso8601();
        assert!(t2 >= t1, "not monotonic: {t1} vs {t2}");
    }

    #[test]
    fn test_is_leap_year() {
        assert!(is_leap(2000)); assert!(is_leap(2024)); assert!(is_leap(2400));
        assert!(!is_leap(1900)); assert!(!is_leap(2023)); assert!(!is_leap(2100));
    }

    // ── Request ID ────────────────────────────────────────────────────────────

    #[test]
    fn test_request_id_unique() {
        let ids: std::collections::HashSet<String> = (0..100).map(|_| new_request_id()).collect();
        assert_eq!(ids.len(), 100, "request IDs must be unique");
    }

    #[test]
    fn test_request_id_hex_only() {
        let r = new_request_id();
        assert!(r.chars().all(|c| c.is_ascii_hexdigit()), "not hex: {r}");
    }

    #[test]
    fn test_request_id_length() {
        // 16 hex (ts) + 8 hex (counter) = 24
        assert_eq!(new_request_id().len(), 24);
    }

    // ── DurationWindow ────────────────────────────────────────────────────────

    #[test]
    fn test_duration_window_empty() {
        assert_eq!(DurationWindow::new(100).percentiles(), (0, 0, 0));
    }

    #[test]
    fn test_duration_window_single() {
        let mut w = DurationWindow::new(100);
        w.push(1_000);
        assert_eq!(w.percentiles(), (1_000, 1_000, 1_000));
    }

    #[test]
    fn test_duration_window_known_distribution() {
        let mut w = DurationWindow::new(1000);
        for i in 1u64..=100 { w.push(i * 1_000); }
        let (p50, p95, p99) = w.percentiles();
        assert!(p50 >= 49_000 && p50 <= 51_000, "p50={p50}");
        assert!(p95 >= 93_000 && p95 <= 97_000, "p95={p95}");
        assert!(p99 >= 97_000 && p99 <= 100_000, "p99={p99}");
    }

    #[test]
    fn test_duration_window_wraps() {
        let mut w = DurationWindow::new(4);
        w.push(1); w.push(2); w.push(3); w.push(4);
        w.push(100); // overwrites slot 0
        let (_, _, p99) = w.percentiles();
        assert_eq!(p99, 100);
    }

    // ── LogConfig ─────────────────────────────────────────────────────────────

    #[test]
    fn test_log_config_defaults() {
        let c = LogConfig::default();
        assert_eq!(c.level, LogLevel::Info);
        assert_eq!(c.slow_query_threshold_us, DEFAULT_SLOW_QUERY_US);
        assert!(c.stdout_enabled);
        assert!(c.cat_query); assert!(c.cat_ingest); assert!(c.cat_wal);
        assert!(!c.cat_connection, "connection must be OFF by default");
        assert!(c.cat_stream); assert!(c.cat_bundle); assert!(c.cat_anomaly);
        assert!(c.cat_audit); assert!(c.cat_system);
    }

    #[test]
    fn test_audit_always_enabled_even_when_off() {
        let mut c = LogConfig::default();
        c.cat_audit = false; // not actually used — audit overrides
        // category_enabled(Audit) always returns true regardless
        assert!(c.category_enabled(LogCategory::Audit));
    }

    #[test]
    fn test_connection_category_disabled_by_default() {
        let c = LogConfig::default();
        assert!(!c.category_enabled(LogCategory::Connection));
    }

    // ── Slow query detection ──────────────────────────────────────────────────

    #[test]
    fn test_slow_flag_above_threshold() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e = log.query_complete(
            "r", "gql", "SELECT", "SELECT 1",
            2_000_000, 0, 2_000_000, &[], 0, 0, 0, 0, false, None, None,
        );
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        assert_eq!(j["slow"], true);
    }

    #[test]
    fn test_slow_flag_below_threshold() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e = log.query_complete(
            "r", "gql", "SELECT", "SELECT 1",
            500_000, 0, 500_000, &[], 0, 0, 0, 0, false, None, None,
        );
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        assert_eq!(j["slow"], false);
    }

    #[test]
    fn test_slow_flag_at_exact_threshold() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e = log.query_complete(
            "r", "gql", "SELECT", "SELECT 1",
            1_000_000, 0, 1_000_000, &[], 0, 0, 0, 0, false, None, None,
        );
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        assert_eq!(j["slow"], true);
    }

    // ── Logger / emit ─────────────────────────────────────────────────────────

    #[test]
    fn test_logger_emit_does_not_panic() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        log.emit(LogEvent::new(LogLevel::Info, LogCategory::Query, "test", inst()));
    }

    #[test]
    fn test_disabled_category_silently_dropped() {
        let mut cfg = LogConfig::default();
        cfg.cat_connection = false;
        let (log, _) = Logger::new(cfg, inst());
        // Should not panic even though category is disabled
        log.emit(LogEvent::new(LogLevel::Info, LogCategory::Connection, "connection.open", inst()));
    }

    // ── Event builder shapes ──────────────────────────────────────────────────

    #[test]
    fn test_query_complete_shape() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e = log.query_complete(
            "req-1", "gql", "DIVERGENCE", "DIVERGENCE FROM a TO b",
            221_043, 89, 220_954,
            &["sensor_das".to_string(), "sensor_sonar".to_string()],
            0, 1, 0, 412, true,
            Some(GeometricFields {
                kl_forward:     Some(0.3944),
                kl_reverse:     Some(0.3762),
                jensen_shannon: Some(0.0647),
                fields_compared: Some(1),
                ..Default::default()
            }),
            None,
        );
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        assert_eq!(j["event"],          "query.complete");
        assert_eq!(j["statement_type"], "DIVERGENCE");
        assert_eq!(j["duration_us"],    221_043);
        assert_eq!(j["parse_us"],       89);
        assert_eq!(j["records_returned"], 1);
        assert_eq!(j["cache_hit"],      true);
        assert_eq!(j["slow"],           false);
        assert_eq!(j["error"],          Value::Null);
        assert_eq!(j["bundles_accessed"].as_array().unwrap().len(), 2);
        // No payload nesting
        assert!(j.get("payload").is_none());
        // Spec §3.1: geometric fields MUST be in a nested "geometric" block
        assert!(j.get("kl_forward").is_none(),     "kl_forward must NOT be at root");
        assert!(j.get("jensen_shannon").is_none(),  "jensen_shannon must NOT be at root");
        let geo = &j["geometric"];
        assert!(geo.is_object(), "geometric must be an object");
        assert!((geo["kl_forward"].as_f64().unwrap() - 0.3944).abs() < 1e-9);
        assert!((geo["kl_reverse"].as_f64().unwrap() - 0.3762).abs() < 1e-9);
        assert!((geo["jensen_shannon"].as_f64().unwrap() - 0.0647).abs() < 1e-9);
        assert_eq!(geo["fields_compared"], 1);
        assert_eq!(geo["ricci"],    Value::Null);
        assert_eq!(geo["k_global"], Value::Null);
        assert_eq!(geo["coherence"], Value::Null);
    }

    #[test]
    fn test_query_complete_no_geo_has_null_geometric_block() {
        // Spec §1.1 §5: geometric fields null when not computed — still PRESENT as null nested block
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e = log.query_complete("r", "rest", "SELECT", "q", 100, 0, 100, &[], 0, 0, 0, 0, false, None, None);
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        // Root must NOT have flat kl_forward etc.
        assert!(j.get("kl_forward").is_none(),     "kl_forward must NOT be at root");
        assert!(j.get("jensen_shannon").is_none(),  "jensen_shannon must NOT be at root");
        // geometric block must exist and contain nulls
        let geo = &j["geometric"];
        assert!(geo.is_object(), "geometric block must be present even when null");
        assert_eq!(geo["kl_forward"],     Value::Null);
        assert_eq!(geo["kl_reverse"],     Value::Null);
        assert_eq!(geo["jensen_shannon"], Value::Null);
        assert_eq!(geo["fields_compared"],Value::Null);
        assert_eq!(geo["ricci"],          Value::Null);
        assert_eq!(geo["k_global"],       Value::Null);
        assert_eq!(geo["coherence"],      Value::Null);
    }

    #[test]
    fn test_query_error_shape() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e = log.query_error("r", "DIVERGENCE FROM x TO y", 312, "BundleNotFound", "msg", 404);
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        assert_eq!(j["event"],       "query.error");
        assert_eq!(j["level"],       "ERROR");
        assert_eq!(j["error_class"], "BundleNotFound");
        assert_eq!(j["http_status"], 404);
    }

    #[test]
    fn test_query_slow_shape() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e = log.query_slow("r", "DIVERGENCE", "DIVERGENCE FROM a TO b",
            10_000_000, false, true, "cold cache");
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        assert_eq!(j["event"],           "query.slow");
        assert_eq!(j["level"],           "WARN");
        assert_eq!(j["category"],        "slow");
        assert_eq!(j["slow_threshold_us"], DEFAULT_SLOW_QUERY_US);
        assert_eq!(j["cache_warm_before"], false);
        assert_eq!(j["cache_warm_after"],  true);
    }

    #[test]
    fn test_ingest_k_delta_computed() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e = log.ingest_complete("b", 500, 24_680, 3_841, true, false, &[], Some(0.018), Some(0.021));
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        assert_eq!(j["event"], "ingest.complete");
        assert_eq!(j["records_written"], 500);
        let delta = j["k_delta"].as_f64().unwrap();
        assert!((delta - 0.003).abs() < 1e-9, "k_delta={delta}");
    }

    #[test]
    fn test_ingest_k_delta_null_when_no_k() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e = log.ingest_complete("b", 100, 0, 0, true, false, &[], None, None);
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        assert_eq!(j["k_delta"], Value::Null);
    }

    #[test]
    fn test_anomaly_sigma_level_3() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e = log.anomaly_detected("b","rec",0.089,0.021,0.008,8.5,0.037,0.045,
            &["pressure".to_string()],"stream",441);
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        assert_eq!(j["sigma_level"], 3);
        assert_eq!(j["level"], "WARN");
    }

    #[test]
    fn test_anomaly_sigma_level_2() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e = log.anomaly_detected("b","r",0.04,0.02,0.005,2.4,0.03,0.035,&[],"query",10);
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        assert_eq!(j["sigma_level"], 2);
    }

    #[test]
    fn test_bundle_stats_cache_warm_shape() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e = log.bundle_stats_cache_warm("chembl_activities", 4_900_000, 4, 9_841_022, "DIVERGENCE FROM a TO b");
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        assert_eq!(j["event"],           "bundle.stats_cache_warm");
        assert_eq!(j["category"],        "bundle");
        assert_eq!(j["records_scanned"], 4_900_000);
        assert_eq!(j["fields_cached"],   4);
    }

    #[test]
    fn test_system_startup_shape() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e = log.system_startup("/data/gigi", 14, 3, 1_247, 2_841_022, "0.0.0.0:3142");
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        assert_eq!(j["event"],           "system.startup");
        assert_eq!(j["bundles_loaded"],  14);
        assert_eq!(j["listen_addr"],     "0.0.0.0:3142");
    }

    #[test]
    fn test_wal_checkpoint_shape() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e = log.wal_checkpoint("sensor_das", 50_000, 2_460_800, 14_400_000, 0, 184_022);
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        assert_eq!(j["event"],           "wal.checkpoint");
        assert_eq!(j["category"],        "wal");
        assert_eq!(j["records_flushed"], 50_000);
        assert_eq!(j["wal_size_after"],  0);
    }

    // ── Metrics ───────────────────────────────────────────────────────────────

    #[test]
    fn test_metrics_counters_start_at_zero() {
        let m = Metrics::new();
        assert_eq!(m.queries_total.load(Ordering::Relaxed),    0);
        assert_eq!(m.queries_error.load(Ordering::Relaxed),    0);
        assert_eq!(m.records_ingested.load(Ordering::Relaxed), 0);
        assert_eq!(m.anomalies_total.load(Ordering::Relaxed),  0);
    }

    #[test]
    fn test_metrics_record_query() {
        let m = Metrics::new();
        m.record_query(500_000,   "SELECT",    false, false);
        m.record_query(2_000_000, "DIVERGENCE",true,  false);
        m.record_query(100,       "SELECT",    false, true);
        assert_eq!(m.queries_total.load(Ordering::Relaxed), 3);
        assert_eq!(m.queries_slow.load(Ordering::Relaxed),  1);
        assert_eq!(m.queries_error.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_metrics_by_type() {
        let m = Metrics::new();
        m.record_query(100, "SELECT",    false, false);
        m.record_query(200, "SELECT",    false, false);
        m.record_query(300, "DIVERGENCE",false, false);
        let bt = m.by_type.lock().unwrap();
        assert_eq!(bt["SELECT"],    2);
        assert_eq!(bt["DIVERGENCE"],1);
    }

    #[test]
    fn test_metrics_record_ingest() {
        let m = Metrics::new();
        m.record_ingest(500,    24_680);
        m.record_ingest(50_000, 2_460_800);
        assert_eq!(m.records_ingested.load(Ordering::Relaxed), 50_500);
        assert_eq!(m.bytes_ingested.load(Ordering::Relaxed),   2_485_480);
    }

    #[test]
    fn test_metrics_anomaly() {
        let m = Metrics::new();
        m.record_anomaly();
        m.record_anomaly();
        assert_eq!(m.anomalies_total.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_metrics_percentiles_from_queries() {
        let m = Metrics::new();
        for i in 1u64..=100 { m.record_query(i * 10_000, "SELECT", false, false); }
        let (p50, p95, p99) = m.percentiles();
        assert!(p50 >= 490_000 && p50 <= 510_000, "p50={p50}");
        assert!(p95 >= 930_000 && p95 <= 970_000, "p95={p95}");
    }

    // ── Logger::get_config / update_config ────────────────────────────────────

    #[test]
    fn test_get_config_returns_defaults() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let cfg = log.get_config();
        assert_eq!(cfg.slow_query_threshold_us, DEFAULT_SLOW_QUERY_US);
        assert!(cfg.cat_query);
        assert!(cfg.cat_audit);
        assert!(!cfg.cat_connection); // off by default
        assert!(cfg.stdout_enabled);
    }

    #[test]
    fn test_update_config_applies_new_values() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let mut new_cfg = log.get_config();
        new_cfg.slow_query_threshold_us = 500_000;
        new_cfg.cat_connection = true;
        new_cfg.stdout_enabled = false;
        log.update_config(new_cfg);
        let updated = log.get_config();
        assert_eq!(updated.slow_query_threshold_us, 500_000);
        assert!(updated.cat_connection);
        assert!(!updated.stdout_enabled);
    }

    #[test]
    fn test_update_config_cannot_disable_audit() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let mut cfg = log.get_config();
        cfg.cat_audit = false; // attempt to disable audit
        log.update_config(cfg);
        assert!(log.get_config().cat_audit, "audit must always be enabled");
    }

    #[test]
    fn test_slow_threshold_us_reflects_update() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        assert_eq!(log.slow_threshold_us(), DEFAULT_SLOW_QUERY_US);
        let mut cfg = log.get_config();
        cfg.slow_query_threshold_us = 250_000;
        log.update_config(cfg);
        assert_eq!(log.slow_threshold_us(), 250_000);
    }

    // ── query.start ───────────────────────────────────────────────────────────

    #[test]
    fn test_query_start_shape() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e = log.query_start("req-42", "gql", "DIVERGENCE FROM a TO b", "10.0.0.1", "curl/8.1");
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        assert_eq!(j["event"],      "query.start");
        assert_eq!(j["category"],   "query");
        assert_eq!(j["level"],      "INFO");
        assert_eq!(j["request_id"], "req-42");
        assert_eq!(j["source"],     "gql");
        assert_eq!(j["raw_gql"],    "DIVERGENCE FROM a TO b");
        assert_eq!(j["client_ip"],  "10.0.0.1");
        assert_eq!(j["user_agent"], "curl/8.1");
        // No duration_us — query hasn't completed yet
        assert!(j.get("duration_us").is_none() || j["duration_us"].is_null());
        // No geometric block — haven't executed yet
        assert!(j.get("geometric").is_none() || j["geometric"].is_null());
    }

    #[test]
    fn test_query_start_different_request_ids() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e1 = log.query_start("req-1", "gql", "SELECT * FROM a", "127.0.0.1", "");
        let e2 = log.query_start("req-2", "gql", "SELECT * FROM b", "127.0.0.1", "");
        let j1: Value = serde_json::from_str(&e1.serialize_json()).unwrap();
        let j2: Value = serde_json::from_str(&e2.serialize_json()).unwrap();
        assert_ne!(j1["request_id"], j2["request_id"]);
    }

    // ── stream events ─────────────────────────────────────────────────────────

    #[test]
    fn test_stream_subscribe_shape() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e = log.stream_subscribe("ws_abc123", "sensor_das", "10.0.0.1", "dashboard");
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        assert_eq!(j["event"],         "stream.subscribe");
        assert_eq!(j["category"],      "stream");
        assert_eq!(j["level"],         "INFO");
        assert_eq!(j["connection_id"], "ws_abc123");
        assert_eq!(j["bundle"],        "sensor_das");
        assert_eq!(j["client_ip"],     "10.0.0.1");
        assert_eq!(j["mode"],          "dashboard");
    }

    #[test]
    fn test_stream_push_shape() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e = log.stream_push("ws_abc123", "sensor_das", 142, 884, 312, 0.021, false, 0.41);
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        assert_eq!(j["event"],         "stream.push");
        assert_eq!(j["category"],      "stream");
        assert_eq!(j["connection_id"], "ws_abc123");
        assert_eq!(j["bundle"],        "sensor_das");
        assert_eq!(j["message_seq"],   142);
        assert_eq!(j["bytes_sent"],    312);
        assert_eq!(j["is_anomaly"],    false);
        assert!((j["k_global"].as_f64().unwrap() - 0.021).abs() < 1e-9);
        assert!((j["z_score"].as_f64().unwrap() - 0.41).abs() < 1e-9);
    }

    #[test]
    fn test_stream_anomaly_push_shape() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e = log.stream_anomaly_push(
            "ws_abc123", "sensor_das", 198, 0.089, 0.045, 3.72,
            &["pressure".to_string(), "temp".to_string()], 1022,
        );
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        assert_eq!(j["event"],        "stream.anomaly_push");
        assert_eq!(j["category"],     "stream");
        assert_eq!(j["level"],        "WARN");
        assert_eq!(j["is_anomaly"],   true);
        assert_eq!(j["message_seq"],  198);
        assert!((j["z_score"].as_f64().unwrap() - 3.72).abs() < 1e-9);
        let fields = j["contributing_fields"].as_array().unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0], "pressure");
    }

    #[test]
    fn test_stream_disconnect_shape() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e = log.stream_disconnect("ws_abc123", "sensor_das", 1_802_441, 201, 3, "client_close");
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        assert_eq!(j["event"],              "stream.disconnect");
        assert_eq!(j["category"],           "stream");
        assert_eq!(j["connection_id"],      "ws_abc123");
        assert_eq!(j["session_duration_us"], 1_802_441);
        assert_eq!(j["messages_sent"],      201);
        assert_eq!(j["anomalies_sent"],     3);
        assert_eq!(j["reason"],             "client_close");
    }

    // ── audit events ──────────────────────────────────────────────────────────

    #[test]
    fn test_audit_bundle_drop_shape() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e = log.audit_bundle_drop("test_bundle", 1247, 62350, "api", "10.0.0.1");
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        assert_eq!(j["event"],           "audit.bundle_drop");
        assert_eq!(j["category"],        "audit");
        assert_eq!(j["level"],           "INFO");
        assert_eq!(j["bundle"],          "test_bundle");
        assert_eq!(j["records_deleted"], 1247);
        assert_eq!(j["bytes_freed"],     62350);
        assert_eq!(j["triggered_by"],    "api");
        assert_eq!(j["client_ip"],       "10.0.0.1");
        assert_eq!(j["outcome"],         "success");
    }

    #[test]
    fn test_audit_log_level_change_shape() {
        let (log, _) = Logger::new(LogConfig::default(), inst());
        let e = log.audit_log_level_change("INFO", "DEBUG", "api_key:test", "success");
        let j: Value = serde_json::from_str(&e.serialize_json()).unwrap();
        assert_eq!(j["event"],     "audit.log_level_change");
        assert_eq!(j["category"],  "audit");
        assert_eq!(j["level"],     "INFO");
        assert_eq!(j["old_level"], "INFO");
        assert_eq!(j["new_level"], "DEBUG");
        assert_eq!(j["actor"],     "api_key:test");
        assert_eq!(j["outcome"],   "success");
    }

    // ── TTL retention config ──────────────────────────────────────────────────

    #[test]
    fn test_retention_defaults() {
        let cfg = LogConfig::default();
        // query_log = 30 days default
        assert_eq!(cfg.retention_days("_gigi_query_log"), 30);
        // audit_log = 365 days default
        assert_eq!(cfg.retention_days("_gigi_audit_log"), 365);
        // anomaly_log = 90 days default
        assert_eq!(cfg.retention_days("_gigi_anomaly_log"), 90);
        // slow_log = 90 days default
        assert_eq!(cfg.retention_days("_gigi_slow_log"), 90);
        // stream_log = 7 days default
        assert_eq!(cfg.retention_days("_gigi_stream_log"), 7);
    }

    #[test]
    fn test_retention_custom_override() {
        let mut cfg = LogConfig::default();
        cfg.set_retention("_gigi_query_log", 14);
        assert_eq!(cfg.retention_days("_gigi_query_log"), 14);
        // other bundles unchanged
        assert_eq!(cfg.retention_days("_gigi_audit_log"), 365);
    }
}
