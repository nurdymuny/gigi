//! Shared Engine fixtures for the ML modules' unit tests — ported from the
//! binary's mod-wide test helpers (`tmp_dir` / `cleanup` / `scan_rec` /
//! `scan_env` / `scan_lens`) so the extracted tests don't reach into
//! `src/bin/gigi_stream.rs`. The binary keeps its own copies for the tests
//! that stay behind (e.g. `ml_all_endpoints_regression_smoke`).

use std::path::Path;

use crate::engine::Engine;
use crate::types::{BundleSchema, Record, Value};

pub fn tmp_dir(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("gigi_stream_test_{tag}"))
}

pub fn cleanup(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

pub fn scan_rec(pairs: &[(&str, Value)]) -> Record {
    pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
}

pub fn scan_env(tag: &str, name: &str, schema: BundleSchema, rows: Vec<Record>) -> (std::path::PathBuf, Engine) {
    let dir = tmp_dir(tag);
    cleanup(&dir);
    let mut engine = Engine::open(&dir).unwrap();
    engine.create_bundle(schema).unwrap();
    if !rows.is_empty() { engine.batch_insert(name, &rows).unwrap(); }
    (dir, engine)
}

pub fn scan_lens<'a>(sl: &'a super::scan::ScanLenses, n: &str) -> Option<&'a std::collections::HashMap<String, f64>> {
    sl.lens_names.iter().position(|l| l == n).map(|i| &sl.norm[i])
}
