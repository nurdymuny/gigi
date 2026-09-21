//! The engine reports one version, and it is the real one.
//!
//! Three different strings were reachable and none was tied to the binary:
//! `/v1/health` returned a hardcoded literal, `/v1/metrics` returned the crate
//! version, and the published API document said a third thing. We told
//! HELICITY engineering that a recorded version nothing compares is a comment,
//! and then could not tell from the outside which binary was serving.
//!
//! That mattered immediately: the 2026-09-20 deploy had no way to prove itself
//! live, because the field that would have shown it was a constant.
use std::fs;

#[test]
fn health_metrics_and_the_api_document_report_the_same_version() {
    let crate_version = env!("CARGO_PKG_VERSION");

    // What /v1/health and /v1/metrics both serve.
    assert_eq!(
        gigi::observability::GIGI_VERSION,
        crate_version,
        "the served version must come from the crate, not a literal"
    );

    let doc: serde_json::Value =
        serde_json::from_str(&fs::read_to_string("openapi.json").expect("openapi.json"))
            .expect("valid json");
    assert_eq!(
        doc["info"]["version"].as_str(),
        Some(crate_version),
        "the published API document must not advertise a different version"
    );
}

/// The literal is gone, not merely shadowed.
#[test]
fn no_hardcoded_version_literal_remains_in_the_health_response() {
    let src = fs::read_to_string("src/bin/gigi_stream.rs").expect("source");
    assert!(
        !src.contains("version: \"0."),
        "a hardcoded version literal is back in the health response"
    );
}
