//! Shared helpers for integration-test binaries (`mod common;`).
//!
//! INGEST env root: the GIGI_INGEST_DIR gate (src/ingest.rs,
//! 2026-07-03 hardening) requires every source path to be RELATIVE to
//! an allowlisted root. Test fixtures are all `tempfile::tempdir()`
//! children — which live under `std::env::temp_dir()` — so the root is
//! set to the system temp dir exactly ONCE per test process (a
//! process-global `Once`; no per-test set_var races on parallel test
//! threads) and fixture paths are rewritten relative to it.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::Once;

static SET_INGEST_ROOT: Once = Once::new();

/// Point GIGI_INGEST_DIR at `std::env::temp_dir()`, exactly once per
/// test process. Returns the root every ingest fixture must live under.
pub fn ensure_ingest_root() -> PathBuf {
    let root = std::env::temp_dir();
    SET_INGEST_ROOT.call_once(|| {
        std::env::set_var("GIGI_INGEST_DIR", &root);
    });
    root
}

/// Rewrite an absolute fixture path (under the system temp dir) into
/// the root-relative `PathBuf` the INGEST gate requires. Also
/// guarantees the env root is exported.
pub fn ingest_rel(p: &Path) -> PathBuf {
    let root = ensure_ingest_root();
    p.strip_prefix(&root)
        .unwrap_or_else(|_| {
            panic!("ingest fixture {} must live under the temp root {}", p.display(), root.display())
        })
        .to_path_buf()
}

/// Same, as a forward-slash string ready to splice into a GQL
/// statement's quoted source literal.
pub fn ingest_rel_str(p: &Path) -> String {
    ingest_rel(p).to_string_lossy().replace('\\', "/")
}
