//! Ask G — Patterns (DEFINE PATTERN / HUNT / DROP PATTERN / SHOW
//! PATTERNS). The GQL surface lives in `crate::parser`; this area holds
//! the HTTP-facing free logic moved out of the gigi-stream binary
//! (stream-extraction phase 2, family 3; see EXTRACTION_MAP.md). Gated
//! on the `patterns` Cargo feature, exactly as the binary items were.

pub mod http;
