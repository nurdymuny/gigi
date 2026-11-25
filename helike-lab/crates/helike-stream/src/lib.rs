//! Receipts and frames.
//!
//! A receipt records what was computed (command, parameters, results), from which source (git revision, dirty flag,
//! SHA-256 of the solver sources), where (platform), and with which status label. `verify` replays a receipt's
//! command and compares results. Frames (for the viewer) carry the same provenance and a status label; the viewer
//! draws frames and never computes physics.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

/// How much a number may be trusted. Display-grade numbers are never quoted.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    /// Double precision on the CPU, parity-tested against the Python reference.
    Computed,
    /// For display only (e.g. GPU batched); any claim is recomputed with status Computed.
    DisplayGrade,
    /// A floating-point certificate passed.
    PassCertifiedFp,
    /// An interval certificate passed.
    PassCertifiedInterval,
    /// Outward-rounded interval bounds on quantities. Rigorous bounds, but not by themselves the pass or fail of any
    /// gate or certificate.
    IntervalEnclosure,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Provenance {
    pub tool: String,
    pub tool_version: String,
    pub git_revision: Option<String>,
    pub git_dirty: Option<bool>,
    /// SHA-256 over the solver source files (sorted by path), so a receipt names the code that produced it even
    /// outside a git checkout.
    pub source_sha256: Option<String>,
    /// How `source_sha256` was computed. Receipts without this field used raw bytes, so their hash depends on the
    /// checkout's line endings (CRLF on Windows, LF elsewhere) and is not comparable across platforms.
    #[serde(default)]
    pub source_hash_policy: Option<String>,
    pub platform: String,
    pub threads: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub schema: String,
    pub created_unix: u64,
    pub provenance: Provenance,
    pub command: Vec<String>,
    pub status: Status,
    pub params: Value,
    pub results: Value,
}

fn git(args: &[&str], dir: &Path) -> Option<String> {
    let out = Command::new("git").args(args).current_dir(dir).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// SHA-256 over every `.rs` and `Cargo.toml` file under `root` (sorted by relative path; path and content hashed).
pub fn hash_sources(root: &Path) -> Option<String> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if p.is_dir() {
                    if name != "target" && !name.starts_with('.') {
                        walk(&p, out);
                    }
                } else if name.ends_with(".rs") || name == "Cargo.toml" {
                    out.push(p);
                }
            }
        }
    }
    let mut files = vec![];
    walk(root, &mut files);
    if files.is_empty() {
        return None;
    }
    files.sort();
    let mut h = Sha256::new();
    for f in &files {
        let rel = f.strip_prefix(root).unwrap_or(f).to_string_lossy().replace('\\', "/");
        h.update(rel.as_bytes());
        h.update([0u8]);
        // universal newlines: CRLF -> LF, so a Windows checkout and a Mac checkout of the same commit agree
        let bytes = std::fs::read(f).ok()?;
        let mut norm = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == 13 && bytes.get(i + 1) == Some(&10) {
                i += 1;
                continue;
            }
            norm.push(bytes[i]);
            i += 1;
        }
        h.update(&norm);
        h.update([0u8]);
    }
    Some(format!("{:x}", h.finalize()))
}

/// SHA-256 of one text file with CRLF normalised to LF (the same policy as `hash_sources`).
pub fn text_sha256(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let mut norm = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 13 && bytes.get(i + 1) == Some(&10) {
            i += 1;
            continue;
        }
        norm.push(bytes[i]);
        i += 1;
    }
    Some(format!("{:x}", Sha256::digest(&norm)))
}

impl Provenance {
    /// `source_root` is the helike-lab workspace (for hashing and git).
    pub fn collect(tool: &str, tool_version: &str, source_root: &Path, threads: usize) -> Self {
        let rev = git(&["rev-parse", "HEAD"], source_root);
        let dirty = git(&["status", "--porcelain", "--", "."], source_root).map(|s| !s.is_empty());
        Self {
            tool: tool.into(),
            tool_version: tool_version.into(),
            git_revision: rev,
            git_dirty: dirty,
            source_sha256: hash_sources(source_root),
            source_hash_policy: Some(SOURCE_HASH_POLICY.into()),
            platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            threads,
        }
    }
}

impl Receipt {
    pub fn new(provenance: Provenance, command: Vec<String>, status: Status, params: Value, results: Value) -> Self {
        let created_unix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        Self { schema: "helike-lab/receipt/v1".into(), created_unix, provenance, command, status, params, results }
    }

    /// Write atomically: to `path.tmp`, then rename.
    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)? + "\n")?;
        std::fs::rename(tmp, path)
    }

    pub fn read(path: &Path) -> std::io::Result<Self> {
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }
}

/// Source-hash policy: SHA-256 over sorted (relative path, content with CRLF normalised to LF).
pub const SOURCE_HASH_POLICY: &str = "sha256-universal-newline-v1";

/// Wall-clock fields: recorded in receipts, never compared on replay.
pub const TIMING_KEYS: [&str; 1] = ["seconds"];

/// Compare two JSON values: numbers within `rel_tol * max(1, |a|)`, everything else exactly, skipping the wall-clock
/// fields in `TIMING_KEYS`. Returns the paths where they differ.
pub fn compare(a: &Value, b: &Value, rel_tol: f64) -> Vec<String> {
    fn go(a: &Value, b: &Value, tol: f64, path: String, out: &mut Vec<String>) {
        match (a, b) {
            (Value::Number(x), Value::Number(y)) => {
                let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
                if (x - y).abs() > tol * x.abs().max(1.0) {
                    out.push(format!("{path}: {x} vs {y}"));
                }
            }
            (Value::Array(x), Value::Array(y)) => {
                if x.len() != y.len() {
                    out.push(format!("{path}: length {} vs {}", x.len(), y.len()));
                } else {
                    for (k, (u, v)) in x.iter().zip(y).enumerate() {
                        go(u, v, tol, format!("{path}[{k}]"), out);
                    }
                }
            }
            (Value::Object(x), Value::Object(y)) => {
                for (k, u) in x {
                    if TIMING_KEYS.contains(&k.as_str()) {
                        continue;
                    }
                    match y.get(k) {
                        Some(v) => go(u, v, tol, format!("{path}.{k}"), out),
                        None => out.push(format!("{path}.{k}: missing")),
                    }
                }
                for k in y.keys().filter(|k| !x.contains_key(*k)) {
                    out.push(format!("{path}.{k}: extra"));
                }
            }
            _ => {
                if a != b {
                    out.push(format!("{path}: {a} vs {b}"));
                }
            }
        }
    }
    let mut out = vec![];
    go(a, b, rel_tol, "$".into(), &mut out);
    out
}

/// A frame for the viewer: provenance-bearing computed arrays with a status label.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub scene: String,
    pub seq: u64,
    pub status: Status,
    pub params: Value,
    pub data: Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn compare_tolerates_rounding_only() {
        let a = json!({"x": 1.0, "v": [1.0, 2.0], "s": "ok"});
        assert!(compare(&a, &json!({"x": 1.0 + 1e-13, "v": [1.0, 2.0], "s": "ok"}), 1e-10).is_empty());
        assert_eq!(compare(&a, &json!({"x": 1.1, "v": [1.0], "s": "no"}), 1e-10).len(), 3);
    }
}
