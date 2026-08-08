//! SBF-6 — a slow graph verb must not freeze the engine.
//!
//! Before TDD-SBF, `/v1/gql` ran SPECTRAL and BETTI to completion while
//! holding the engine-wide read guard.  Ingest takes the write guard, so it
//! blocked for the whole computation; and because both the Linux futex and
//! the Windows SRWLOCK queue *new readers* behind a waiting writer, one
//! queued insert turned "readers run concurrently" into "everything waits."
//! That is what a 180-second SPECTRAL did to the whole process.
//!
//! This gate reproduces the shape of that failure against the real server and
//! asserts it no longer happens: while SPECTRAL is in flight, point queries
//! stay fast and an insert completes.
//!
//! It is an end-to-end test — it boots `gigi-stream` on a scratch dir and
//! talks HTTP — so it is `#[ignore]` by default and run explicitly:
//!
//!     cargo test --release --test spectral_no_wedge -- --ignored --nocapture

use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const PORT: u16 = 3167;

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Minimal blocking HTTP/1.1 POST — no reqwest dependency in dev-deps.
fn post(path: &str, body: &str, ctype: &str) -> (u16, String) {
    let mut s = TcpStream::connect(("127.0.0.1", PORT)).expect("connect");
    s.set_read_timeout(Some(Duration::from_secs(120))).unwrap();
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: {ctype}\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    s.write_all(req.as_bytes()).expect("write");
    let mut raw = String::new();
    let _ = s.read_to_string(&mut raw);
    let code = raw
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let body = raw.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (code, body)
}

fn gql(q: &str) -> (u16, String) {
    let payload = serde_json::json!({ "query": q }).to_string();
    post("/v1/gql", &payload, "application/json")
}

fn boot() -> Server {
    let exe = env!("CARGO_BIN_EXE_gigi-stream");
    let dir = std::env::temp_dir().join(format!("gigi-sbf6-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let child = Command::new(exe)
        .env("PORT", PORT.to_string())
        .env("GIGI_DATA_DIR", &dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn gigi-stream");
    let srv = Server(child);
    // Wait for the port to answer.
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(30) {
        if TcpStream::connect(("127.0.0.1", PORT)).is_ok() {
            std::thread::sleep(Duration::from_millis(300));
            return srv;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("server did not come up on {PORT}");
}

#[test]
#[ignore = "boots a server; run with --ignored"]
fn sbf6_slow_verb_does_not_block_reads_or_ingest() {
    let _srv = boot();

    // A bundle shaped like the one that wedged: low-cardinality indexed
    // fields ⇒ giant near-cliques ⇒ the worst case for the graph verbs.
    let (c, b) = gql(
        "CREATE BUNDLE wedge (id INT BASE, sym TEXT FIBER INDEX, \
         side TEXT FIBER INDEX, px NUMERIC FIBER);",
    );
    assert_eq!(c, 200, "create failed: {b}");

    const N: usize = 40_000;
    let mut nd = String::with_capacity(N * 60);
    for i in 0..N {
        nd.push_str(&format!(
            "{{\"id\":{i},\"sym\":\"S{}\",\"side\":\"{}\",\"px\":{}}}\n",
            i % 4,
            if i % 2 == 0 { "B" } else { "S" },
            100.0 + (i % 977) as f64 * 0.01
        ));
    }
    let (c, b) = post("/v1/bundles/wedge/ingest", &nd, "application/x-ndjson");
    assert_eq!(c, 200, "ingest failed: {}", &b[..b.len().min(300)]);

    // Fire SPECTRAL on a background thread and hammer the engine while it runs.
    let spectral = std::thread::spawn(|| {
        let t = Instant::now();
        let (c, b) = gql("SPECTRAL wedge;");
        (c, b, t.elapsed())
    });
    std::thread::sleep(Duration::from_millis(30)); // let it get under way

    let mut read_ms: Vec<u128> = Vec::new();
    for _ in 0..40 {
        let t = Instant::now();
        let (c, _) = gql("CURVATURE wedge;");
        assert_eq!(c, 200, "read failed while SPECTRAL was in flight");
        read_ms.push(t.elapsed().as_millis());
    }

    let t = Instant::now();
    let (ci, _) = post(
        "/v1/bundles/wedge/ingest",
        "{\"id\":999999,\"sym\":\"S0\",\"side\":\"B\",\"px\":1.0}\n",
        "application/x-ndjson",
    );
    let insert_ms = t.elapsed().as_millis();
    assert_eq!(ci, 200, "insert failed while SPECTRAL was in flight");

    let (sc, sb, sdur) = spectral.join().unwrap();
    assert_eq!(sc, 200, "SPECTRAL failed: {sb}");

    read_ms.sort_unstable();
    let p99 = read_ms[read_ms.len() * 99 / 100];
    let max = *read_ms.last().unwrap();
    println!(
        "SBF-6: SPECTRAL {} ms · concurrent reads p99 {p99} ms (max {max}) \
         · insert during flight {insert_ms} ms",
        sdur.as_millis()
    );

    assert!(p99 < 500, "reads queued behind SPECTRAL: p99 {p99} ms");
    assert!(
        insert_ms < 2_000,
        "ingest blocked behind SPECTRAL: {insert_ms} ms"
    );
}

#[test]
#[ignore = "boots a server; run with --ignored"]
fn sbf6b_betti_returns_same_numbers_over_http() {
    let _srv = boot();
    // Two indexed fields: `sym` alone would give 7 DISJOINT cliques (β₀=7),
    // so `side` is what cross-links them into one component — and it is also
    // what makes the inclusion–exclusion overlap term non-trivial, which is
    // the part of the operator worth exercising end-to-end.
    let (c, _) = gql(
        "CREATE BUNDLE parity (id INT BASE, sym TEXT FIBER INDEX, \
         side TEXT FIBER INDEX, px NUMERIC FIBER);",
    );
    assert_eq!(c, 200);
    let mut nd = String::new();
    for i in 0..2_000 {
        nd.push_str(&format!(
            "{{\"id\":{i},\"sym\":\"S{}\",\"side\":\"{}\",\"px\":{}}}\n",
            i % 7,
            if i % 2 == 0 { "B" } else { "S" },
            (i % 101) as f64
        ));
    }
    assert_eq!(post("/v1/bundles/parity/ingest", &nd, "application/x-ndjson").0, 200);

    let (c, b) = gql("BETTI parity;");
    assert_eq!(c, 200, "betti failed: {b}");
    let v: serde_json::Value = serde_json::from_str(&b).expect("json");
    let b0 = v["betti_0"].as_u64().expect("betti_0 present");
    let b1 = v["betti_1"].as_u64().expect("betti_1 present");
    let scalar = v["value"].as_f64().expect("value present");
    assert_eq!(scalar as u64, b0 + b1, "scalar envelope must stay β₀+β₁");
    // Two 1000-member side buckets alone give 2·C(1000,2) ≈ 999k edges;
    // the sym cliques add more, the overlap subtracts. One component.
    assert_eq!(b0, 1, "expected a connected graph, got β₀={b0}");
    assert!(b1 > 900_000, "expected near-clique edge count, got β₁={b1}");
    println!("SBF-6b: BETTI over HTTP → β₀={b0} β₁={b1} scalar={scalar}");

    let (c, b) = gql("SPECTRAL parity;");
    assert_eq!(c, 200, "spectral failed: {b}");
    let v: serde_json::Value = serde_json::from_str(&b).unwrap();
    let lam = v["value"].as_f64().expect("value present");
    assert!(lam.is_finite() && lam >= 0.0, "λ₁={lam}");
    println!("SBF-6b: SPECTRAL over HTTP → λ₁={lam}");
}
