//! Transactions Phase-A HTTP wire types + free helpers
//! (stream-extraction phase 2, family 4; see EXTRACTION_MAP.md).
//! The request/response structs and the free functions (`parse_tx_id`,
//! `sys_time_to_iso`) for the `/v1/transactions/*` endpoints, moved
//! verbatim out of `src/bin/gigi_stream.rs`.
//!
//! The five handlers themselves stay whole in the binary: their bodies
//! ARE the shared-state logic (`StreamState.tx_registry` /
//! `tx_snap_counter` / the interleaved engine-lock discipline in
//! `tx_commit`), which EXTRACTION_MAP.md family 11 keeps at the binary
//! root — exactly the "OpenTx / StreamState.open_txs live on the shared
//! state" seam the map flags for this family.

use crate::stream_shared::ErrorResponse;
use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct TxBeginRequest {
    #[serde(default)]
    pub isolation: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TxBeginResponse {
    pub tx_id: String,
    pub snap_id: u64,
    pub opened_at: String,
    pub isolation: String,
}

#[derive(Debug, Deserialize)]
pub struct TxWriteRequest {
    pub bundle: String,
    pub records: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct TxWriteResponse {
    pub staged: usize,
    pub total_in_tx: usize,
    pub touched_bundles: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct TxCommitResponse {
    pub committed_at: String,
    pub new_snap_id: u64,
    pub bundles_committed: Vec<String>,
    pub records_committed: usize,
}

#[derive(Debug, Serialize)]
pub struct TxRollbackResponse {
    pub aborted: bool,
    pub discarded_records: usize,
}

#[derive(Debug, Serialize)]
pub struct TxStatusResponse {
    pub tx_id: String,
    pub snap_id: u64,
    pub state: String,
    pub isolation: String,
    pub opened_at: String,
    pub age_secs: u64,
    pub touched_bundles: Vec<String>,
    pub pending_writes: usize,
}

pub fn parse_tx_id(
    s: &str,
) -> Result<crate::transactions::TransactionId, (StatusCode, Json<ErrorResponse>)> {
    let stripped = s.strip_prefix("tx_").unwrap_or(s);
    let uuid = uuid::Uuid::parse_str(stripped).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("invalid tx_id '{}': expected 'tx_<uuid>'", s),
            }),
        )
    })?;
    Ok(crate::transactions::TransactionId(uuid))
}

pub fn sys_time_to_iso(t: std::time::SystemTime) -> String {
    let dur = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    format!("epoch:{}", dur.as_secs())
}
