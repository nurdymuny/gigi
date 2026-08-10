//! Wire-contract gates for the three shape verbs, over real HTTP.
//!
//! Two properties that only exist at the wire, and that a customer's SDK
//! depends on. Both were defects found by customer acceptance testing
//! (2026-08-10) and fixed in `28dd5bd`; nothing in the Rust suite covered
//! either, so both could silently regress.
//!
//! 1. **Byte determinism.** `Record` is a `HashMap`, so two identical GQL calls
//!    returned semantically equal but byte-DIFFERENT bodies — iteration order
//!    varies per process. That breaks response hashing, cache keys, golden
//!    files and client-side diffs. `record_to_json` now sorts its keys.
//!
//! 2. **One error envelope.** Axum's `Json<T>` extractor rejects with PLAIN
//!    TEXT, so a malformed body or a missing required field returned a
//!    non-JSON 400/422 while every domain refusal from the same endpoint was
//!    `{"error": ...}`. A generated client that assumes one envelope breaks.
//!    The three shape handlers now use an `ApiJson` extractor.
//!
//! Neither can be tested in-process: the router, `record_to_json` and
//! `ApiJson` all live in `src/bin/gigi_stream.rs`, not the library. So this
//! spawns the real binary and speaks HTTP/1.1 to it over a socket. That is
//! also the honest level — the claim is about bytes on the wire.
//!
//! Deliberately ONE test function. The server takes seconds to boot and every
//! assertion here shares that one instance; splitting it would either boot the
//! binary repeatedly or leak a process across parallel test threads.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Kills the spawned server even if an assertion panics.
struct ServerGuard(Child);

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// An ephemeral port the OS has just confirmed is free.
fn free_port() -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    let p = l.local_addr().expect("local_addr").port();
    drop(l);
    p
}

/// One HTTP/1.1 request over a fresh socket. Returns (status, content_type,
/// body bytes). `Connection: close` lets us read the body to EOF without
/// implementing chunked decoding.
///
/// Uses 127.0.0.1 explicitly — `localhost` resolves ::1 first on Windows and
/// costs about 2 seconds per request against 11 ms here.
fn http(port: u16, method: &str, path: &str, ctype: Option<&str>, body: Option<&[u8]>) -> (u16, String, Vec<u8>) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream.set_read_timeout(Some(Duration::from_secs(120))).ok();

    let body = body.unwrap_or(&[]);
    let mut req = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    if let Some(ct) = ctype {
        req.push_str(&format!("Content-Type: {ct}\r\n"));
    }
    req.push_str("\r\n");

    stream.write_all(req.as_bytes()).expect("write head");
    if !body.is_empty() {
        stream.write_all(body).expect("write body");
    }
    stream.flush().ok();

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).expect("read response");

    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("no header/body separator");
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let body_bytes = raw[split + 4..].to_vec();

    let status: u16 = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .expect("status line");

    let ct = head
        .lines()
        .find(|l| l.to_ascii_lowercase().starts_with("content-type:"))
        .map(|l| l[l.find(':').unwrap() + 1..].trim().to_string())
        .unwrap_or_default();

    (status, ct, body_bytes)
}

fn post_json(port: u16, path: &str, body: &str) -> (u16, String, Vec<u8>) {
    http(port, "POST", path, Some("application/json"), Some(body.as_bytes()))
}

fn gql(port: u16, query: &str) -> (u16, String, Vec<u8>) {
    let payload = serde_json::json!({ "query": query }).to_string();
    post_json(port, "/v1/gql", &payload)
}

/// The body must be `application/json` AND parse to an object carrying `error`.
/// Checking only the status code would have passed against the plain-text
/// rejections this gate exists to prevent.
fn assert_error_envelope(label: &str, status: u16, ctype: &str, body: &[u8]) {
    assert!(
        status >= 400,
        "{label}: expected a 4xx/5xx, got {status}"
    );
    assert!(
        ctype.to_ascii_lowercase().contains("json"),
        "{label}: content-type was '{ctype}', not JSON — body: {}",
        String::from_utf8_lossy(&body[..body.len().min(160)])
    );
    let v: serde_json::Value = serde_json::from_slice(body).unwrap_or_else(|e| {
        panic!(
            "{label}: body was not JSON ({e}) — {}",
            String::from_utf8_lossy(&body[..body.len().min(160)])
        )
    });
    let msg = v.get("error").and_then(|e| e.as_str()).unwrap_or_else(|| {
        panic!("{label}: no `error` key in {v}")
    });
    assert!(!msg.trim().is_empty(), "{label}: `error` was empty");
}

#[test]
fn shape_verbs_hold_their_wire_contract() {
    let port = free_port();
    let dir = tempfile::tempdir().expect("tempdir");

    let child = Command::new(env!("CARGO_BIN_EXE_gigi-stream"))
        .env("PORT", port.to_string())
        .env("GIGI_DATA_DIR", dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn gigi-stream");
    let _guard = ServerGuard(child);

    // Boot. The engine replays any WAL and opens its listener; on a fresh
    // scratch dir that is fast, but give it real headroom on a loaded CI box.
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        assert!(Instant::now() < deadline, "gigi-stream never opened port {port}");
        std::thread::sleep(Duration::from_millis(100));
    }

    // ── fixture ────────────────────────────────────────────────────────────
    let (c, _, _) = gql(
        port,
        "CREATE BUNDLE wire (row_id INT BASE, seq NUMERIC FIBER, ts NUMERIC FIBER, \
         v NUMERIC FIBER, w NUMERIC FIBER, lbl TEXT FIBER);",
    );
    assert_eq!(c, 200, "CREATE BUNDLE failed");

    let mut nd = String::new();
    let (mut a, mut b, mut t) = (0.0f64, 0.0f64, 0.0f64);
    let mut s: u64 = 0x2545_F491_4F6C_DD1D;
    let mut rnd = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s >> 11) as f64 / (1u64 << 53) as f64
    };
    for i in 0..2048 {
        let (u1, u2) = (rnd().max(1e-12), rnd());
        let g = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        a += g;
        b += g * 0.5 + rnd() - 0.5;
        t += -rnd().max(1e-12).ln() * 0.05;
        nd.push_str(&format!(
            "{{\"row_id\":{i},\"seq\":{}.0,\"ts\":{:.9},\"v\":{:.6},\"w\":{:.6},\"lbl\":\"x\"}}\n",
            i, t, a, b
        ));
    }
    let (c, _, _) = http(
        port,
        "POST",
        "/v1/bundles/wire/ingest",
        Some("application/x-ndjson"),
        Some(nd.as_bytes()),
    );
    assert_eq!(c, 200, "ingest failed");

    // ── PROPERTY 1: identical GQL calls return identical BYTES ─────────────
    for query in [
        "TEXTURE wire ON v ALONG seq;",
        "PRECEDENCE wire ON v, w ALONG seq;",
        "CADENCE wire ON ts;",
    ] {
        let mut bodies: Vec<Vec<u8>> = Vec::new();
        for _ in 0..6 {
            let (st, _, body) = gql(port, query);
            assert_eq!(st, 200, "{query} -> {st}");
            bodies.push(body);
        }
        let first = &bodies[0];
        for (i, other) in bodies.iter().enumerate().skip(1) {
            assert_eq!(
                first,
                other,
                "call {i} of `{query}` differed BYTE-WISE from call 0.\n  \
                 first: {}\n  other: {}",
                String::from_utf8_lossy(&first[..first.len().min(200)]),
                String::from_utf8_lossy(&other[..other.len().min(200)])
            );
        }

        // Determinism here comes from sorted keys, so assert the mechanism too:
        // a HashMap that happened to hash identically six times would pass the
        // check above while leaving the defect in place.
        let v: serde_json::Value = serde_json::from_slice(first).expect("row json");
        let row = v["rows"][0].as_object().expect("one row object");
        let keys: Vec<&String> = row.keys().collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "`{query}` row keys are not sorted: {keys:?}");
    }

    // ── PROPERTY 2: every failure uses ONE envelope ────────────────────────
    // Extractor-level rejections — these were plain text before `ApiJson`.
    let (st, ct, body) = post_json(port, "/v1/bundles/wire/texture", "{not json");
    assert_error_envelope("malformed JSON body", st, &ct, &body);

    let (st, ct, body) = post_json(port, "/v1/bundles/wire/texture", r#"{"order":"seq"}"#);
    assert_error_envelope("missing required field", st, &ct, &body);

    let (st, ct, body) = post_json(port, "/v1/bundles/wire/texture", r#"{"field":123}"#);
    assert_error_envelope("wrong field type", st, &ct, &body);

    let (st, ct, body) = post_json(
        port,
        "/v1/bundles/wire/texture",
        r#"{"field":"v","order":"seq","maxLag":32}"#,
    );
    assert_error_envelope("unknown request key", st, &ct, &body);

    // Domain refusals from all three verbs — these were already JSON, and must
    // stay the same shape as the extractor rejections above.
    for (label, path, payload) in [
        ("texture: missing field", "/v1/bundles/wire/texture", r#"{"field":"nope"}"#),
        ("texture: missing bundle", "/v1/bundles/ghost/texture", r#"{"field":"v"}"#),
        ("precedence: x == y", "/v1/bundles/wire/precedence", r#"{"x":"v","y":"v"}"#),
        ("cadence: text clock", "/v1/bundles/wire/cadence", r#"{"time":"lbl"}"#),
        ("cadence: counter clock", "/v1/bundles/wire/cadence", r#"{"time":"seq"}"#),
    ] {
        let (st, ct, body) = post_json(port, path, payload);
        assert_error_envelope(label, st, &ct, &body);
    }

    // The GQL surface uses the same envelope for its own failures.
    let (st, ct, body) = gql(port, "TEXTURE ghost ON v;");
    assert_error_envelope("gql: missing bundle", st, &ct, &body);

    let (st, ct, body) = gql(port, "CADENCE wire ON ts ALONG seq;");
    assert_error_envelope("gql: parse error", st, &ct, &body);

    // ── The message a customer sees must match across surfaces ─────────────
    let (_, _, rest) = post_json(port, "/v1/bundles/ghost/texture", r#"{"field":"v"}"#);
    let (_, _, over_gql) = gql(port, "TEXTURE ghost ON v;");
    let rest_msg = serde_json::from_slice::<serde_json::Value>(&rest).unwrap()["error"].clone();
    let gql_msg = serde_json::from_slice::<serde_json::Value>(&over_gql).unwrap()["error"].clone();
    assert_eq!(
        rest_msg, gql_msg,
        "missing-bundle message differs between surfaces: REST {rest_msg} vs GQL {gql_msg}"
    );
}
