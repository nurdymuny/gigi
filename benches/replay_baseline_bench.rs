//! Sequential WAL replay baseline — measures Engine::open wall-time on
//! a representative fixture and emits a deterministic post-replay state
//! hash. The hash is the bit-identity contract a future parallel
//! replay must match byte-for-byte.
//!
//! Run with:
//!   cargo run --release --bin replay_baseline_bench -- <data_dir>
//!
//! Default <data_dir> is gigi_data (4-bundle local fixture, ~280MB WAL).

use std::env;
use std::path::PathBuf;
use std::time::Instant;

use gigi::engine::Engine;
use sha2::{Digest, Sha256};

fn main() -> std::io::Result<()> {
    let data_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("gigi_data"));

    eprintln!("== Sequential WAL replay baseline ==");
    eprintln!("data_dir: {}", data_dir.display());

    let wal = data_dir.join("gigi.wal");
    let wal_bytes = std::fs::metadata(&wal).map(|m| m.len()).unwrap_or(0);
    eprintln!("wal size: {} bytes ({:.1} MB)", wal_bytes, wal_bytes as f64 / (1024.0 * 1024.0));

    let runs: usize = 5;
    let mut wall_ms: Vec<f64> = Vec::with_capacity(runs);
    let mut last_hash = String::new();

    for run in 1..=runs {
        let start = Instant::now();
        let engine = Engine::open(&data_dir)?;
        let elapsed = start.elapsed();
        wall_ms.push(elapsed.as_secs_f64() * 1000.0);

        // Deterministic post-replay state hash.
        let hash = state_hash(&engine);
        if last_hash.is_empty() {
            last_hash = hash.clone();
        } else if hash != last_hash {
            eprintln!("  WARN: state hash diverged across runs: {} vs {}", last_hash, hash);
        }
        eprintln!(
            "  run {}/{}: {:.1} ms — {} bundles, {} records, state_hash={}",
            run, runs, wall_ms[run - 1], engine.bundle_names().len(), engine.total_records(), &hash[..16]
        );
    }

    let mean = wall_ms.iter().sum::<f64>() / wall_ms.len() as f64;
    let min = wall_ms.iter().cloned().fold(f64::MAX, f64::min);
    eprintln!("");
    eprintln!("== Summary ==");
    eprintln!("runs:        {}", runs);
    eprintln!("wall_ms_min: {:.1}", min);
    eprintln!("wall_ms_mean: {:.1}", mean);
    eprintln!("state_hash:  {}", last_hash);
    eprintln!("");
    // Machine-readable single line for the harness.
    println!(
        "{{\"baseline_wall_ms_mean\":{:.3},\"baseline_wall_ms_min\":{:.3},\"baseline_state_hash\":\"{}\"}}",
        mean, min, last_hash
    );
    Ok(())
}

/// Deterministic post-replay state hash.
///
/// Hash inputs (sorted to remove HashMap iteration nondeterminism):
///   for each bundle name (sorted):
///     bundle name UTF-8 bytes
///     LE u64 record count
///
/// This is the cheap bit-identity gate. The parallel replay scheme
/// must reproduce this exact hash — same bundle set, same record
/// counts per bundle — across many runs and against the sequential
/// baseline.
fn state_hash(engine: &Engine) -> String {
    let mut names: Vec<&str> = engine.bundle_names();
    names.sort();

    let mut hasher = Sha256::new();
    for name in &names {
        hasher.update(name.as_bytes());
        let count = engine.bundle(name).map(|b| b.len()).unwrap_or(0) as u64;
        hasher.update(count.to_le_bytes());
    }
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for b in digest.iter() {
        out.push_str(&format!("{:02x}", b));
    }
    out
}
