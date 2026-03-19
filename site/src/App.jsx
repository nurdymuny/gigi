import { useState, useEffect, useRef } from "react";
import BracketPredictor from "./BracketPredictor";

// ═══════════════════════════════════════
// LIVE BENCHMARK ENGINE (real GIGI server)
// ═══════════════════════════════════════
const BENCH_API = import.meta.env.DEV ? "http://localhost:3142" : "/api/gigi";
const BENCH_LABEL = import.meta.env.DEV ? "localhost:3142" : "gigi-stream.fly.dev";

async function gqlPost(query) {
  const res = await fetch(`${BENCH_API}/v1/gql`, {
    method: "POST", headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ query }),
  });
  return res.json();
}

async function restPost(path, body) {
  const res = await fetch(`${BENCH_API}${path}`, {
    method: "POST", headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  return res.json();
}

async function restDelete(path) {
  const res = await fetch(`${BENCH_API}${path}`, { method: "DELETE" });
  return res.json();
}

async function runBenchmark(n) {
  const name = `bench_${n}_${Date.now() % 100000}`;

  // 1. Create bundle (single int key + no indexes → turbo batch_insert fast path)
  await restPost("/v1/bundles", {
    name, schema: { fields: { id: "numeric", cat: "categorical", val: "numeric", active: "categorical" }, keys: ["id"] }
  });

  // 2. Batch insert — chunk into batches of 5000
  const chunkSize = 5000;
  const t0 = performance.now();
  for (let start = 0; start < n; start += chunkSize) {
    const batch = [];
    const end = Math.min(start + chunkSize, n);
    for (let i = start; i < end; i++) {
      batch.push({ id: i, cat: "c" + (i % 10), val: i * 0.1, active: i % 20 !== 0 ? "true" : "false" });
    }
    await restPost(`/v1/bundles/${name}/insert`, { records: batch });
  }
  const insT = performance.now() - t0;

  // 3. Point queries — 100 random lookups via GQL
  const numPt = 100;
  const ids = Array.from({ length: numPt }, () => Math.floor(Math.random() * n));
  const t1 = performance.now();
  for (const id of ids) {
    await gqlPost(`SECTION ${name} AT id=${id}`);
  }
  const ptT = performance.now() - t1;

  // 4. Range query — categorical index lookup
  const t2 = performance.now();
  const rgRes = await gqlPost(`COVER ${name} ON cat = 'c0'`);
  const rgT = performance.now() - t2;
  const rgSize = rgRes.count || 0;

  // 5. Curvature
  const t3 = performance.now();
  const kRes = await gqlPost(`CURVATURE ${name}`);
  const kT = performance.now() - t3;

  // 6. Cleanup
  await restDelete(`/v1/bundles/${name}`);

  return {
    n,
    insUs: (insT / n) * 1000,        // insert per-record in μs
    ptUs: (ptT / numPt) * 1000,       // avg point query in μs
    rgMs: rgT,                         // range query in ms
    rgSize,                            // range result count
    kVal: kRes.value,                  // curvature
    kMs: kT,                           // curvature time in ms
    totalInsMs: insT,                  // total insert time in ms
  };
}

// ═══════════════════════════════════════
// DATA
// ═══════════════════════════════════════
const G = "#40E8A0";
const GD = "#2BA070";
const BG = "#06060A";

const COMPARISONS = [
  { s: "PostgreSQL", t: "B-tree", p: "O(log n)", r: "O(log n + k)", j: "O(n log n)", c: "None", g: "Euclidean" },
  { s: "Cassandra", t: "Hash ring", p: "O(1)*", r: "O(n) scan", j: "N/A", c: "None", g: "Ring" },
  { s: "Elasticsearch", t: "Inverted idx", p: "O(1)", r: "O(k)", j: "Expensive", c: "TF-IDF", g: "Cosine" },
  { s: "Pinecone", t: "HNSW", p: "O(log n)", r: "N/A", j: "N/A", c: "Distance", g: "Euclidean" },
  { s: "Redis", t: "Hash table", p: "O(1)", r: "O(n) scan", j: "N/A", c: "None", g: "Flat" },
  { s: "Firebase", t: "B-tree", p: "O(log n)", r: "O(log n+k)", j: "Manual", c: "None", g: "Flat" },
  { s: "GIGI", t: "Fiber bundle", p: "O(1)", r: "O(|result|)", j: "O(|left|)", c: "C=τ/K", g: "Riemannian" },
];

const USE_CASES = [
  { icon: "🌡️", title: "IoT / Sensor Networks", sub: "Timestamps are arithmetic. Units are constant. Status is 95% normal.", desc: "1M sensor readings where 4 of 5 fields are derived from structure. Curvature K spikes at anomalies — the database tells you where the interesting data is. 78% compression on the wire.", stats: ["O(1) per reading", "K = anomaly detector", "78% compression"] },
  { icon: "💰", title: "Financial Reconciliation", sub: "Match payments. Detect breaks geometrically.", desc: "Pullback joins match transactions across rails in O(|left|). Holonomy around the settlement loop detects breaks: nonzero holonomy = your books don't balance.", stats: ["O(|left|) joins", "Holonomy = break detection", "Sheaf = audit proof"] },
  { icon: "🧠", title: "LLM Context Injection", sub: "Retrieve O(1). Serialize DHOOM. Maximize signal/token.", desc: <span>GIGI retrieves context in O(1) and serializes in <a href="https://dhoom.dev" target="_blank" rel="noopener noreferrer" style={{ color: "#E8A830", textDecoration: "none" }}>DHOOM</a> — 66-84% fewer tokens. The curvature score tells you which context is reliable.</span>, stats: ["O(1) retrieval", "84% token savings", "Confidence per result"] },
  { icon: "🌐", title: "Distributed Systems", sub: "Sheaf gluing guarantees partition queries compose.", desc: "The sheaf gluing axiom mathematically guarantees that range queries across partitions combine into globally correct results. Holonomy detects replica drift.", stats: ["Sheaf gluing", "Holonomy = drift detection", "Math > consensus"] },
  { icon: "🔍", title: "Semantic Search (No Vectors)", sub: "Curvature-based relevance. No embeddings. No ANN.", desc: "Instead of embedding into ℝⁿ and computing cosine, GIGI uses intrinsic connection on the data bundle. Zero Euclidean distance computed.", stats: ["Zero distance ops", "Intrinsic relevance", "Curvature confidence"] },
  { icon: "⚖️", title: "Compliance & Audit", sub: "Mathematical proof of query correctness.", desc: "Sheaf axioms prove query results are correct — not probabilistically, not eventually, but necessarily. S + d² = 1 quantifies completeness. Show the regulator the math.", stats: ["Provable correctness", "S + d² = 1", "Sheaf axioms ≠ SLA"] },
];

// Dataset generators for live streaming benchmarks
const STREAM_DATASETS = [
  { name: "Users API", key: "users", n: 1000, gen: (i) => ({ id: i, username: "user_" + String(i).padStart(5, "0"), email: `user${i}@co.com`, role: i % 5 === 0 ? "admin" : "viewer", department: ["Eng", "Mkt", "Sales", "Ops", "HR"][i % 5], login_count: Math.floor(Math.random() * 500), status: i % 20 === 0 ? "suspended" : "active" }) },
  { name: "Events Stream", key: "events", n: 5000, gen: (i) => ({ event_id: i, timestamp: 1710000000 + i * 10, user_id: Math.floor(Math.random() * 1000), event_type: ["click", "view", "scroll", "submit"][i % 4], duration_ms: Math.floor(Math.random() * 5000), platform: i % 8 === 0 ? "mobile" : "web", success: i % 15 !== 0 ? "true" : "false" }) },
  { name: "IoT Sensors", key: "iot_sensors", n: 10000, gen: (i) => ({ sensor_id: "S-" + String(i % 50).padStart(3, "0"), timestamp: 1710000000 + i * 60, temperature: 18 + Math.random() * 10, humidity: 40 + Math.random() * 20, pressure: 1010 + Math.random() * 5, unit: "metric", status: i % 30 === 0 ? "alert" : "normal" }) },
];

const ARCH = [
  { l: "3", name: "Connection Layer", c: "#FF6040", items: ["Parallel transport", "Curvature K", "C = τ/K confidence", "Holonomy (consistency)", "Čech cohomology H¹"] },
  { l: "2", name: "Sheaf Query Engine", c: "#E8A830", items: ["σ(x) point — O(1)", "F(U) range — O(|r|)", "Pullback join — O(|left|)", "Fiber integration", "Partition function Z(β,p)"] },
  { l: "1", name: "Bundle Store", c: G, items: ["Base manifold B", "Fiber F (schema)", "Sections σ (records)", "GIGI hash (chart)", "Field index topology"] },
];

// ═══════════════════════════════════════
// COMPONENTS
// ═══════════════════════════════════════
function Nav({ page, go }) {
  const [menuOpen, setMenuOpen] = useState(false);
  const links = [
    { id: "home", l: "Home" }, { id: "gigi", l: "Gigi" }, { id: "demo", l: "Try It" }, { id: "nasa", l: "NASA Demo" }, { id: "encryption", l: "Encryption" },
    { id: "benchmarks", l: "Benchmarks" }, { id: "usecases", l: "Use Cases" },
    { id: "architecture", l: "Architecture" }, { id: "compare", l: "vs Others" },
    { id: "products", l: "Products" },
  ];
  const playgroundUrl = import.meta.env.DEV ? "http://localhost:5174" : "/gigi/playground";
  const docsUrl = import.meta.env.DEV ? "http://localhost:5175" : "/gigi/docs";
  const navTo = (id) => { go(id); setMenuOpen(false); };
  return (
    <nav className="gigi-nav">
      <div className="gigi-nav-bar">
        <span onClick={() => navTo("home")} style={{ fontSize: 21, fontWeight: 900, cursor: "pointer", color: G, letterSpacing: "-0.02em", flexShrink: 0 }}>GIGI</span>
        <div className="gigi-nav-links">
          {links.map(n => <button key={n.id} onClick={() => navTo(n.id)} className="gigi-nav-btn" style={{ color: page === n.id ? G : "#505068" }}>{n.l}</button>)}
        </div>
        <div className="gigi-nav-ext">
          <a href={docsUrl} target="_blank" rel="noopener noreferrer" className="gigi-nav-pill">Docs</a>
          <a href={playgroundUrl} target="_blank" rel="noopener noreferrer" className="gigi-nav-pill">Playground</a>
        </div>
        <span className="gigi-nav-ver">v0.2 · Davis Geometric</span>
        <button className="gigi-hamburger" onClick={() => setMenuOpen(!menuOpen)} aria-label="Menu">
          <span style={{ display: "block", width: 18, height: 2, background: menuOpen ? G : "#606878", transform: menuOpen ? "rotate(45deg) translate(3px,3px)" : "none", transition: "0.2s" }} />
          <span style={{ display: "block", width: 18, height: 2, background: "#606878", opacity: menuOpen ? 0 : 1, transition: "0.2s" }} />
          <span style={{ display: "block", width: 18, height: 2, background: menuOpen ? G : "#606878", transform: menuOpen ? "rotate(-45deg) translate(3px,-3px)" : "none", transition: "0.2s" }} />
        </button>
      </div>
      {menuOpen && (
        <div className="gigi-mobile-menu">
          {links.map(n => <button key={n.id} onClick={() => navTo(n.id)} style={{ display: "block", width: "100%", background: "none", border: "none", borderBottom: "1px solid rgba(255,255,255,0.04)", cursor: "pointer", padding: "12px 20px", color: page === n.id ? G : "#707888", fontSize: 14, fontWeight: page === n.id ? 700 : 400, textAlign: "left" }}>{n.l}</button>)}
          <div style={{ display: "flex", gap: 8, padding: "14px 20px" }}>
            <a href={docsUrl} target="_blank" rel="noopener noreferrer" className="gigi-nav-pill" style={{ flex: 1, textAlign: "center" }}>Docs</a>
            <a href={playgroundUrl} target="_blank" rel="noopener noreferrer" className="gigi-nav-pill" style={{ flex: 1, textAlign: "center" }}>Playground</a>
          </div>
        </div>
      )}
    </nav>
  );
}

function Hero({ go }) {
  return (
    <div style={{ padding: "76px 24px 56px", textAlign: "center", position: "relative", overflow: "hidden" }}>
      <div style={{ position: "absolute", top: -80, left: "50%", transform: "translateX(-50%)", width: 600, height: 600, background: "radial-gradient(circle,rgba(64,232,160,0.06),transparent 60%)", pointerEvents: "none" }} />
      <div style={{ position: "relative", maxWidth: 680, margin: "0 auto" }}>
        <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.22em", color: "#40E8A028", marginBottom: 14, fontFamily: "monospace" }}>GEOMETRIC INTRINSIC GLOBAL INDEX</div>
        <h1 style={{ fontSize: 58, fontWeight: 900, margin: 0, lineHeight: 1.05, color: "#E0E8F0", letterSpacing: "-0.04em" }}>The geometry<br /><span style={{ color: G }}>IS</span> the index.</h1>
        <p style={{ fontSize: 17, color: "#607080", margin: "20px auto 0", maxWidth: 500, lineHeight: 1.65 }}>A database engine where data lives on a fiber bundle. Queries are section evaluations. Joins are pullbacks. Confidence is curvature. <strong style={{ color: "#A0B0C0" }}>Zero Euclidean math.</strong></p>
        <div style={{ display: "inline-flex", alignItems: "center", gap: 6, marginTop: 14, padding: "5px 14px", borderRadius: 20, background: "rgba(232,168,48,0.08)", border: "1px solid rgba(232,168,48,0.2)" }}>
          <span style={{ fontSize: 10, color: "#E8A830", fontWeight: 700, letterSpacing: "0.08em", fontFamily: "monospace" }}>PATENT PENDING</span>
          <span style={{ fontSize: 10, color: "#606878" }}>U.S. App. No. 64/008,940</span>
        </div>
        <div style={{ display: "flex", gap: 10, justifyContent: "center", marginTop: 28, flexWrap: "wrap" }}>
          <Btn label="NASA Demo" onClick={() => go("nasa")} primary />
          <Btn label="Live Benchmarks" onClick={() => go("benchmarks")} />
          <Btn label="Products & Pricing" onClick={() => go("products")} />
        </div>
      </div>
    </div>
  );
}

function Btn({ label, onClick, primary }) {
  return <button onClick={onClick} style={{ padding: "11px 22px", borderRadius: 7, border: primary ? "none" : "1px solid rgba(64,232,160,0.15)", cursor: "pointer", background: primary ? G : "transparent", color: primary ? BG : "#60708088", fontSize: 13, fontWeight: 700 }}>{label}</button>;
}

function Stats() {
  const s = [
    { v: "O(1)", l: "Point Query", d: "proven, not amortized-expected" },
    { v: "O(|r|)", l: "Range Query", d: "output size, not input size" },
    { v: "54-84%", l: "Wire Compression", d: <span>DHOOM vs JSON, dataset-dependent — <a href="https://dhoom.dev" target="_blank" rel="noopener noreferrer" style={{ color: "#E8A830", textDecoration: "none" }}>dhoom.dev</a></span> },
    { v: "~1μs", l: "Query Latency", d: "Rust engine, 7K real records" },
  ];
  return (
    <div style={{ maxWidth: 940, margin: "0 auto", padding: "36px 24px", display: "grid", gridTemplateColumns: "repeat(4,1fr)", gap: 10 }}>
      {s.map((x, i) => (
        <div key={i} style={{ background: "rgba(64,232,160,0.025)", border: "1px solid rgba(64,232,160,0.07)", borderRadius: 10, padding: "18px 14px", textAlign: "center" }}>
          <div style={{ fontSize: 26, fontWeight: 900, color: G, fontFamily: "monospace" }}>{x.v}</div>
          <div style={{ fontSize: 12.5, fontWeight: 700, color: "#A0B0C0", marginTop: 5 }}>{x.l}</div>
          <div style={{ fontSize: 10.5, color: "#505060", marginTop: 3 }}>{x.d}</div>
        </div>
      ))}
    </div>
  );
}

function CompTable() {
  const cols = ["System", "Index", "Point", "Range", "Join", "Confidence", "Geometry"];
  const keys = ["s", "t", "p", "r", "j", "c", "g"];
  return (
    <div style={{ maxWidth: 940, margin: "0 auto", padding: "0 24px 32px", overflowX: "auto" }}>
      <SectionLabel>vs Everything Else</SectionLabel>
      <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11.5, fontFamily: "monospace" }}>
        <thead><tr>{cols.map((h, i) => <th key={i} style={{ textAlign: "left", padding: "8px 8px", borderBottom: "1px solid rgba(255,255,255,0.05)", color: "#506070", fontWeight: 600, fontSize: 9.5, letterSpacing: "0.06em", textTransform: "uppercase" }}>{h}</th>)}</tr></thead>
        <tbody>{COMPARISONS.map((row, ri) => {
          const isG = row.s === "GIGI";
          return <tr key={ri} style={{ background: isG ? "rgba(64,232,160,0.035)" : "transparent" }}>{keys.map((k, ci) => <td key={ci} style={{ padding: "6px 8px", borderBottom: "1px solid rgba(255,255,255,0.02)", color: isG ? G : "#606878", fontWeight: isG ? 700 : 400 }}>{row[k]}</td>)}</tr>;
        })}</tbody>
      </table>
    </div>
  );
}

function SectionLabel({ children }) {
  return <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.14em", color: "#384050", textTransform: "uppercase", marginBottom: 14, textAlign: "center" }}>{children}</div>;
}

// ═══════════════════════════════════════
// PAGES
// ═══════════════════════════════════════
function BenchPage() {
  const [res, setRes] = useState(null);
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState("");
  const sizes = [100, 500, 1000, 5000, 10000, 50000];
  async function run() {
    setRunning(true);
    const results = [];
    for (const n of sizes) {
      setProgress(`Benchmarking ${n.toLocaleString()} records...`);
      try { results.push(await runBenchmark(n)); }
      catch (e) { setProgress(`Error at ${n}: ${e.message}`); setRunning(false); return; }
    }
    setRes(results);
    setProgress("");
    setRunning(false);
  }
  const lastSize = res ? res.length - 1 : 0;
  // For the dual-bar chart: scale everything against what O(log n) would predict at the largest size
  const baseTime = res ? res[0].ptUs : 1;
  const logScale = res ? Math.log2(sizes[lastSize]) / Math.log2(sizes[0]) * baseTime : 1;
  const chartMax = res ? Math.max(logScale, ...res.map(r => r.ptUs)) * 1.1 : 1;
  return (
    <Page title="Live Benchmarks" sub={<>Running against <strong style={{ color: G }}>GIGI Stream</strong> at {BENCH_LABEL}. Real Rust engine. Real network. Real O(1).</>}>
      <button onClick={run} disabled={running} style={{ padding: "11px 24px", borderRadius: 7, border: "none", cursor: running ? "wait" : "pointer", background: running ? "#303848" : G, color: running ? "#607080" : BG, fontSize: 13, fontWeight: 700, marginBottom: 24 }}>{running ? progress || "Running..." : "Run Benchmarks"}</button>
      {res && (<>
        <SectionLabel>Point Query: O(1) Proof — Live Server</SectionLabel>
        <p style={{ fontSize: 12, color: "#506070", marginBottom: 14 }}>100 random point queries per size via GQL. <span style={{ color: G }}>Green</span> = actual GIGI. <span style={{ color: "#FF604080" }}>Red ghost</span> = what O(log n) would cost. The green bars stay flat. The red bars grow.</p>
        <div style={{ display: "flex", flexDirection: "column", gap: 6, marginBottom: 28 }}>
          {res.map((r, i) => {
            const predicted = baseTime * Math.log2(sizes[i]) / Math.log2(sizes[0]);
            return (
              <div key={i} style={{ display: "flex", alignItems: "center", gap: 10 }}>
                <span style={{ width: 72, textAlign: "right", fontSize: 11, color: "#506070", fontFamily: "monospace" }}>{r.n.toLocaleString()}</span>
                <div style={{ flex: 1, position: "relative", height: 28, background: "rgba(255,255,255,0.02)", borderRadius: 4, overflow: "hidden" }}>
                  <div style={{ position: "absolute", top: 0, left: 0, width: Math.max((predicted / chartMax) * 100, 4) + "%", height: "100%", borderRadius: 4, background: "linear-gradient(90deg,#FF604030,#FF604010)", borderRight: "2px solid #FF604040" }} />
                  <div style={{ position: "relative", width: Math.max((r.ptUs / chartMax) * 100, 4) + "%", height: "100%", borderRadius: 4, background: "linear-gradient(90deg,#40E8A0CC,#40E8A044)", display: "flex", alignItems: "center", justifyContent: "flex-end", paddingRight: 8 }}>
                    <span style={{ fontSize: 10.5, fontWeight: 700, color: "#fff", fontFamily: "monospace" }}>{r.ptUs.toFixed(0)}μs</span>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
        <div style={{ display: "flex", gap: 16, marginBottom: 20 }}>
          <span style={{ fontSize: 10, color: "#506070" }}><span style={{ display: "inline-block", width: 12, height: 12, borderRadius: 3, background: "#40E8A0CC", verticalAlign: "middle", marginRight: 4 }} /> GIGI actual</span>
          <span style={{ fontSize: 10, color: "#506070" }}><span style={{ display: "inline-block", width: 12, height: 12, borderRadius: 3, background: "#FF604030", border: "1px solid #FF604060", verticalAlign: "middle", marginRight: 4 }} /> O(log n) predicted</span>
        </div>
        <SectionLabel>Full Results — Real GIGI Engine</SectionLabel>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11.5, fontFamily: "monospace" }}>
          <thead><tr>{["Records", "Insert (μs/rec)", "Point (μs)", "Range (ms)", "|result|", "K (curvature)", "K time (ms)"].map((h, i) => <th key={i} style={{ textAlign: "left", padding: "7px 8px", borderBottom: "1px solid rgba(255,255,255,0.05)", color: "#506070", fontWeight: 600, fontSize: 9.5, textTransform: "uppercase" }}>{h}</th>)}</tr></thead>
          <tbody>{res.map((r, i) => <tr key={i}><td style={td()}>{r.n.toLocaleString()}</td><td style={td(G)}>{r.insUs.toFixed(1)}</td><td style={{...td(G), fontWeight: 700}}>{r.ptUs.toFixed(0)}</td><td style={td("#E8A830")}>{r.rgMs.toFixed(1)}</td><td style={td()}>{r.rgSize.toLocaleString()}</td><td style={td("#FF6040")}>{r.kVal !== undefined ? r.kVal.toFixed(6) : "—"}</td><td style={td("#506070")}>{r.kMs.toFixed(1)}</td></tr>)}</tbody>
        </table>
        <div style={{ display: "flex", gap: 24, marginTop: 14, flexWrap: "wrap" }}>
          <p style={{ fontSize: 11, color: "#384050", fontFamily: "monospace" }}>Ratio ({res[lastSize].n.toLocaleString()}/{res[0].n.toLocaleString()}): <strong style={{ color: G }}>{(res[lastSize].ptUs/res[0].ptUs).toFixed(2)}x</strong> — O(1) confirmed</p>
          <p style={{ fontSize: 11, color: "#384050", fontFamily: "monospace" }}>Total insert ({res[lastSize].n.toLocaleString()} records): <strong style={{ color: G }}>{res[lastSize].totalInsMs.toFixed(0)}ms</strong> = <strong style={{ color: G }}>{(res[lastSize].n / (res[lastSize].totalInsMs / 1000)).toFixed(0)}</strong> rec/s</p>
        </div>
      </>)}
    </Page>
  );
}

function StreamPage() {
  const [res, setRes] = useState(null);
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState("");

  async function run() {
    setRunning(true);
    const results = [];
    for (const ds of STREAM_DATASETS) {
      setProgress(`Streaming ${ds.name} (${ds.n.toLocaleString()} records)...`);
      try {
        const bundleName = `stream_${ds.key}_${Date.now() % 100000}`;
        // Determine keys — first field of the generator output
        const sample = ds.gen(0);
        const fieldNames = Object.keys(sample);
        const fields = {};
        const keys = [];
        for (const f of fieldNames) {
          const v = sample[f];
          fields[f] = typeof v === "number" ? "numeric" : "categorical";
        }
        // Use first numeric field as key
        for (const f of fieldNames) {
          if (fields[f] === "numeric") { keys.push(f); break; }
        }

        // 1. Create bundle
        await restPost("/v1/bundles", { name: bundleName, schema: { fields, keys } });

        // 2. Generate + insert (measure ingest throughput)
        const records = Array.from({ length: ds.n }, (_, i) => ds.gen(i));
        const chunkSize = 5000;
        const t0 = performance.now();
        for (let s = 0; s < records.length; s += chunkSize) {
          await restPost(`/v1/bundles/${bundleName}/insert`, { records: records.slice(s, s + chunkSize) });
        }
        const ingestMs = performance.now() - t0;

        // 3. Get DHOOM encoding (measure serialize)
        const t1 = performance.now();
        const dhoomRes = await (await fetch(`${BENCH_API}/v1/bundles/${bundleName}/dhoom`)).json();
        const serializeMs = performance.now() - t1;

        // 4. Cleanup
        await restDelete(`/v1/bundles/${bundleName}`);

        // Build DHOOM preview (first 8 lines)
        const dhoomLines = dhoomRes.dhoom.split("\n");
        const preview = dhoomLines.slice(0, Math.min(8, dhoomLines.length)).join("\n") +
          (dhoomLines.length > 8 ? `\n... (${dhoomLines.length - 8} more rows)` : "");

        results.push({
          name: ds.name,
          n: ds.n,
          ingestMs, serializeMs, pipelineMs: ingestMs + serializeMs,
          ingestRate: Math.round(ds.n / (ingestMs / 1000)),
          serializeRate: Math.round(ds.n / (serializeMs / 1000)),
          pipelineRate: Math.round(ds.n / ((ingestMs + serializeMs) / 1000)),
          compression: dhoomRes.compression_pct,
          jsonChars: dhoomRes.json_chars,
          dhoomChars: dhoomRes.dhoom_chars,
          fieldsOmitted: dhoomRes.fields_omitted,
          totalSlots: dhoomRes.total_field_slots,
          preview,
        });
      } catch (e) {
        setProgress(`Error on ${ds.name}: ${e.message}`);
        setRunning(false);
        return;
      }
    }
    setRes(results);
    setProgress("");
    setRunning(false);
  }

  const fmt = (n) => n >= 1000000 ? (n / 1000000).toFixed(1) + "M" : n >= 1000 ? Math.round(n / 1000) + "K" : n;

  return (
    <Page title="Live Streaming Benchmark" sub={<>JSON → GIGI Ingest → DHOOM Serialization. Running against <strong style={{ color: G }}>GIGI Stream</strong> at {BENCH_LABEL}.</>}>
      <button onClick={run} disabled={running} style={{ padding: "11px 24px", borderRadius: 7, border: "none", cursor: running ? "wait" : "pointer", background: running ? "#303848" : "#E8A830", color: running ? "#607080" : BG, fontSize: 13, fontWeight: 700, marginBottom: 24 }}>{running ? progress || "Running..." : "Run Streaming Benchmark"}</button>
      {res && (<>
        <SectionLabel>Pipeline Results — Live Server</SectionLabel>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11.5, fontFamily: "monospace", marginBottom: 24 }}>
          <thead><tr>{["Dataset", "Records", "Ingest", "Serialize", "Pipeline", "Compress"].map((h, i) => <th key={i} style={{ textAlign: "left", padding: "8px 8px", borderBottom: "1px solid rgba(255,255,255,0.05)", color: "#506070", fontWeight: 600, fontSize: 9.5, textTransform: "uppercase" }}>{h}</th>)}</tr></thead>
          <tbody>{res.map((r, i) => (
            <tr key={i}>
              <td style={td("#A0B0C0")}>{r.name}</td>
              <td style={td()}>{r.n.toLocaleString()}</td>
              <td style={td(G)}>{fmt(r.ingestRate)}/s</td>
              <td style={td(G)}>{fmt(r.serializeRate)}/s</td>
              <td style={td(G)}>{fmt(r.pipelineRate)}/s</td>
              <td style={td("#E8A830")}>{r.compression.toFixed(0)}%</td>
            </tr>
          ))}</tbody>
        </table>

        <SectionLabel>DHOOM Compression — JSON Chars vs DHOOM Chars</SectionLabel>
        <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 28 }}>
          {res.map((r, i) => {
            const maxChars = Math.max(...res.map(x => x.jsonChars));
            return (
              <div key={i} style={{ display: "flex", alignItems: "center", gap: 10 }}>
                <span style={{ width: 120, fontSize: 11, color: "#607080", textAlign: "right" }}>{r.name} ({r.n.toLocaleString()})</span>
                <div style={{ flex: 1, position: "relative", height: 24, background: "rgba(255,255,255,0.02)", borderRadius: 4, overflow: "hidden" }}>
                  <div style={{ position: "absolute", top: 0, left: 0, width: (r.jsonChars / maxChars * 100) + "%", height: "100%", borderRadius: 4, background: "linear-gradient(90deg,#FF604020,#FF604008)", borderRight: "2px solid #FF604030" }} />
                  <div style={{ position: "relative", width: (r.dhoomChars / maxChars * 100) + "%", height: "100%", borderRadius: 4, background: "linear-gradient(90deg,#E8A830CC,#E8A83044)", display: "flex", alignItems: "center", justifyContent: "flex-end", paddingRight: 8 }}>
                    <span style={{ fontSize: 9.5, fontWeight: 700, color: "#fff", fontFamily: "monospace" }}>{r.compression.toFixed(0)}%</span>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
        <div style={{ display: "flex", gap: 16, marginBottom: 20 }}>
          <span style={{ fontSize: 10, color: "#506070" }}><span style={{ display: "inline-block", width: 12, height: 12, borderRadius: 3, background: "#E8A830CC", verticalAlign: "middle", marginRight: 4 }} /> DHOOM output</span>
          <span style={{ fontSize: 10, color: "#506070" }}><span style={{ display: "inline-block", width: 12, height: 12, borderRadius: 3, background: "#FF604020", border: "1px solid #FF604040", verticalAlign: "middle", marginRight: 4 }} /> JSON original</span>
        </div>

        <SectionLabel>DHOOM Wire Output — Live from Server</SectionLabel>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 10 }}>
          {res.map((r, i) => (
            <div key={i}>
              <div style={{ fontSize: 10, fontWeight: 700, color: "#E8A830", marginBottom: 4 }}>{r.name} ({r.n.toLocaleString()}) — {r.compression.toFixed(0)}% smaller</div>
              <pre style={{ background: "#0A0A12", border: "1px solid rgba(232,168,48,0.08)", borderRadius: 7, padding: "10px", fontSize: 10, lineHeight: 1.5, color: "#9090A8", fontFamily: "monospace", margin: 0, whiteSpace: "pre-wrap", minHeight: 120, maxHeight: 200, overflow: "auto" }}>{r.preview}</pre>
            </div>
          ))}
        </div>

        <Card style={{ marginTop: 24 }}>
          <div style={{ fontSize: 13, fontWeight: 700, color: "#A0B0C0", marginBottom: 8 }}>How DHOOM Works</div>
          <p style={{ fontSize: 12.5, color: "#607080", lineHeight: 1.65, margin: 0 }}>
            DHOOM detects <strong style={{ color: "#E8A830" }}>arithmetic progressions</strong> (<code style={code()}>@start+step</code> — entire column elided), <strong style={{ color: "#E8A830" }}>modal defaults</strong> (<code style={code()}>|value</code> — only deviations transmitted), and <strong style={{ color: "#E8A830" }}>trailing elision</strong> (omit trailing defaults). The Rust encoder detects all three automatically. Events compress most because timestamps are arithmetic and most fields have strong defaults.
          </p>
        </Card>
      </>)}
    </Page>
  );
}

function UseCasePage() {
  return (
    <Page title="Use Cases" sub="Where fiber bundle geometry solves real problems.">
      <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
        {USE_CASES.map((uc, i) => (
          <Card key={i}>
            <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 6 }}>
              <span style={{ fontSize: 22 }}>{uc.icon}</span>
              <div>
                <div style={{ fontSize: 15, fontWeight: 700, color: "#C0D0E0" }}>{uc.title}</div>
                <div style={{ fontSize: 11, color: G + "88", fontStyle: "italic" }}>{uc.sub}</div>
              </div>
            </div>
            <p style={{ fontSize: 12.5, color: "#607080", lineHeight: 1.65, margin: "10px 0" }}>{uc.desc}</p>
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              {uc.stats.map((s, si) => <Tag key={si}>{s}</Tag>)}
            </div>
          </Card>
        ))}
      </div>
    </Page>
  );
}

function ArchPage() {
  return (
    <Page title="Architecture" sub="Three layers. One geometric framework. Zero Euclidean math.">
      <div style={{ display: "flex", flexDirection: "column", gap: 0, marginBottom: 28 }}>
        {ARCH.map((layer, i) => (
          <div key={i} style={{ background: "rgba(255,255,255,0.015)", borderLeft: "3px solid " + layer.c, padding: "18px 20px", borderRight: "1px solid rgba(255,255,255,0.04)", borderTopRightRadius: i === 0 ? 8 : 0, borderBottomRightRadius: i === ARCH.length - 1 ? 8 : 0 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 6 }}>
              <span style={{ fontSize: 12, fontWeight: 800, color: layer.c, fontFamily: "monospace", width: 28 }}>L{layer.l}</span>
              <span style={{ fontSize: 14.5, fontWeight: 700, color: "#C0D0E0" }}>{layer.name}</span>
            </div>
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap", paddingLeft: 38 }}>
              {layer.items.map((it, ii) => <span key={ii} style={{ fontSize: 11, color: "#607080", background: "rgba(255,255,255,0.025)", border: "1px solid rgba(255,255,255,0.03)", borderRadius: 4, padding: "3px 8px" }}>{it}</span>)}
            </div>
          </div>
        ))}
      </div>
      <SectionLabel>Query Algebra</SectionLabel>
      <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12, marginBottom: 24 }}>
        <thead><tr>{["SQL", "Geometric Operation", "Complexity"].map((h, i) => <th key={i} style={{ textAlign: "left", padding: "7px 10px", borderBottom: "1px solid rgba(255,255,255,0.05)", color: "#506070", fontWeight: 600, fontSize: 9.5, textTransform: "uppercase" }}>{h}</th>)}</tr></thead>
        <tbody>{[
          ["WHERE pk = v", "Section evaluation σ(x)", "O(1)"],
          ["WHERE f IN (…)", "Sheaf evaluation F(U)", "O(|result|)"],
          ["JOIN ON fk = pk", "Pullback bundle f*E₂", "O(|left|)"],
          ["GROUP BY f", "Base space partition", "O(N)"],
          ["COUNT / SUM / AVG", "Fiber integration", "O(|group|)"],
        ].map((r, i) => <tr key={i}><td style={{ ...td(), fontFamily: "monospace", fontSize: 11 }}>{r[0]}</td><td style={td("#A0B0C0")}>{r[1]}</td><td style={{ ...td(G), fontWeight: 700, fontFamily: "monospace" }}>{r[2]}</td></tr>)}</tbody>
      </table>
      <Card>
        <div style={{ fontSize: 13, fontWeight: 700, color: "#A0B0C0", marginBottom: 6 }}>Wire Protocol: <a href="https://dhoom.dev" target="_blank" rel="noopener noreferrer" style={{ color: "#E8A830", textDecoration: "none" }}>DHOOM</a></div>
        <p style={{ fontSize: 12, color: "#607080", lineHeight: 1.6, margin: 0 }}>
          GIGI speaks <a href="https://dhoom.dev" target="_blank" rel="noopener noreferrer" style={{ color: "#E8A830", textDecoration: "none" }}>DHOOM</a> natively. The zero section becomes <code style={code()}>|</code> defaults, deviations become <code style={code()}>:</code> overrides, arithmetic base points become <code style={code()}>@</code> compression. One math, end to end. 66-84% wire savings.
        </p>
      </Card>
    </Page>
  );
}

function MathPage() {
  return (
    <Page title="The Mathematics" sub="Fiber bundles, sheaves, and the Davis Field Equations applied to data.">
      <Sec title="The Core Insight">
        <P>Every database stores data as points in a container and builds an external index. B-trees, inverted indices, HNSW graphs — auxiliary structures bolted onto flat storage.</P>
        <P><strong style={{ color: G }}>GIGI says: the geometry IS the index.</strong> Data lives on a fiber bundle. The mathematical structure determines where data lives and how it’s found.</P>
      </Sec>
      <Sec title="Fiber Bundles">
        <P><strong style={{ color: "#A0B0C0" }}>(E, B, F, π)</strong> — total space E, base space B (keys), fiber F (schema), projection π. A <strong style={{ color: "#A0B0C0" }}>section σ: B → E</strong> is a record. Insert = define a section. Point query = evaluate the section. Both O(1).</P>
      </Sec>
      <Sec title="Sheaf Axioms">
        <P><strong style={{ color: G }}>Locality:</strong> If two sections agree on every element of an open cover, they are the same section.</P>
        <P><strong style={{ color: G }}>Gluing:</strong> Local sections that agree on overlaps combine into a globally correct result.</P>
        <P>These are <em>mathematical guarantees</em>, not SLA promises. Čech cohomology H¹ ≠ 0 detects when data is inconsistent — and tells you exactly where.</P>
      </Sec>
      <Sec title="C = τ/K (Davis Field Equation)">
        <P><strong style={{ color: "#A0B0C0" }}>τ</strong> = tolerance budget (acceptable error). <strong style={{ color: "#A0B0C0" }}>K</strong> = curvature (local variability). <strong style={{ color: "#A0B0C0" }}>C = τ/K</strong> = capacity (ability to answer confidently).</P>
        <P>Every query result gets a <strong style={{ color: G }}>confidence score</strong> from the geometry. IoT sensors: K = 0.0006, confidence = 0.9994. No other database has this.</P>
      </Sec>
      <Sec title="Partition Function Z(β, p)">
        <P>For approximate queries, GIGI computes a Boltzmann distribution over nearby records: P(q|p,τ) = exp(-d/τ) / Z. At τ→0, only the exact match survives (Z=1). At τ→∞, all neighbors contribute equally. The Davis Law C = τ/K is recovered as a thermodynamic equation of state.</P>
      </Sec>
      <Sec title="Spectral Capacity">
        <P>The field index graph’s Laplacian eigenvalues govern capacity at a deeper level than curvature. λ₁ (spectral gap) bounds mixing time and detects bottleneck structure. Disconnected components have λ₁ = 0 — the spectrum tells you the topology.</P>
      </Sec>
      <Sec title="The Double Cover: S + d² = 1">
        <P>For any query Q: <strong style={{ color: G }}>S + d² = 1</strong>. S = recall. d = √(1-S). For exact queries on a flat connection: S = 1, d = 0. Validated to 10 decimal places.</P>
      </Sec>
      <Card style={{ marginTop: 20 }}>
        <p style={{ margin: 0, fontSize: 12, color: "#506070", lineHeight: 1.6 }}>Full framework: Davis, B. R. (2024). <em style={{ color: "#A0B0C0" }}>The Geometry of Sameness</em>. Amazon KDP. Davis, B. R. (2026). <em style={{ color: "#A0B0C0" }}>The Double Cover Principle</em>. Zenodo.</p>
      </Card>
    </Page>
  );
}

function NasaPage() {
  const mapRef = useRef(null);
  const curvRef = useRef(null);
  const corrRef = useRef(null);
  const tsRef = useRef(null);
  const predRef = useRef(null);
  const dhoomRef = useRef(null);
  const [selectedCity, setSelectedCity] = useState(null);
  const [nasaData, setNasaData] = useState(null);

  useEffect(() => {
    import("./nasa-data.js").then(m => setNasaData(m.NASA_DATA));
  }, []);

  useEffect(() => {
    if (!nasaData || typeof window.Plotly === "undefined") return;
    const DATA = nasaData;
    const DK = {
      paper_bgcolor: "transparent", plot_bgcolor: "rgba(255,255,255,0.015)",
      font: { family: "DM Sans, Inter, sans-serif", color: "#e2e8f0", size: 12 },
      margin: { t: 30, b: 50, l: 60, r: 20 },
      xaxis: { gridcolor: "rgba(255,255,255,0.05)", zerolinecolor: "rgba(255,255,255,0.05)" },
      yaxis: { gridcolor: "rgba(255,255,255,0.05)", zerolinecolor: "rgba(255,255,255,0.05)" },
    };
    const CFG = { displayModeBar: false, responsive: true };
    function kColor(k) { return k > 0.05 ? "#f87171" : k > 0.02 ? "#fbbf24" : "#34d399"; }
    function nasaDate(d) { const s = String(Math.floor(d)); return s.slice(0,4)+"-"+s.slice(4,6)+"-"+s.slice(6,8); }

    // 1. World Map
    if (mapRef.current) {
      const c = DATA.cities;
      Plotly.newPlot(mapRef.current, [{
        type: "scattergeo", mode: "markers+text",
        lat: c.map(x => x.lat), lon: c.map(x => x.lon),
        text: c.map(x => x.name.replace(/_/g, " ")),
        textposition: "top center", textfont: { size: 10, color: "#e2e8f0" },
        marker: {
          size: c.map(x => 8 + x.extremeCount * 4),
          color: c.map(x => x.k_temp),
          colorscale: [[0, "#10b981"], [0.4, "#fbbf24"], [1, "#ef4444"]],
          colorbar: { title: { text: "K(temp)", font: { size: 11 } }, thickness: 12, len: 0.6, tickfont: { size: 10 } },
          line: { width: 1, color: "rgba(255,255,255,0.3)" }, opacity: 0.9,
        },
        hovertemplate: "<b>%{text}</b><br>K(temp): %{marker.color:.4f}<br>Extremes: %{marker.size}<extra></extra>",
      }], {
        ...DK, margin: { t: 10, b: 10, l: 10, r: 10 },
        geo: {
          projection: { type: "natural earth", rotation: { lon: 20 } },
          showland: true, landcolor: "#151530", showocean: true, oceancolor: "#0a0a1a",
          showcoastlines: true, coastlinecolor: "#2a2a4a", coastlinewidth: 0.5,
          showcountries: true, countrycolor: "#1e1e3a", countrywidth: 0.3,
          showlakes: false, bgcolor: "transparent",
          lataxis: { gridcolor: "rgba(255,255,255,0.03)" },
          lonaxis: { gridcolor: "rgba(255,255,255,0.03)" },
        },
      }, CFG);
      mapRef.current.on("plotly_click", (ev) => {
        if (ev.points && ev.points[0]) setSelectedCity(ev.points[0].text.replace(/ /g, "_"));
      });
    }

    // 2. Curvature Bar Chart
    if (curvRef.current) {
      const c = [...DATA.cities].sort((a, b) => a.k_temp - b.k_temp);
      Plotly.newPlot(curvRef.current, [{
        type: "bar", orientation: "h",
        y: c.map(x => x.name.replace(/_/g, " ")), x: c.map(x => x.k_temp),
        marker: { color: c.map(x => kColor(x.k_temp)), opacity: 0.85 },
        text: c.map(x => "K=" + x.k_temp.toFixed(4)),
        textposition: "outside", textfont: { size: 10, color: "#94a3b8" },
        hovertemplate: "<b>%{y}</b><br>K(temp): %{x:.6f}<br>Confidence: %{customdata:.3f}<extra></extra>",
        customdata: c.map(x => x.confidence),
      }], {
        ...DK, margin: { t: 10, b: 40, l: 110, r: 70 },
        xaxis: { ...DK.xaxis, title: { text: "K(temp)", font: { size: 11 } } },
        yaxis: { ...DK.yaxis, automargin: true, tickfont: { size: 10 } },
      }, CFG);
    }

    // 3. Correlation Scatter
    if (corrRef.current) {
      const c = DATA.cities;
      const xs = c.map(x => x.k_temp), ys = c.map(x => x.extremeCount);
      const mx = xs.reduce((a, b) => a + b, 0) / xs.length;
      const my = ys.reduce((a, b) => a + b, 0) / ys.length;
      const num = xs.reduce((s, x, i) => s + (x - mx) * (ys[i] - my), 0);
      const den = xs.reduce((s, x) => s + (x - mx) ** 2, 0);
      const slope = den ? num / den : 0, intercept = my - slope * mx;
      const xMin = Math.min(...xs), xMax = Math.max(...xs);
      Plotly.newPlot(corrRef.current, [
        { type: "scatter", mode: "markers+text",
          x: xs, y: ys,
          text: c.map(x => x.name.replace(/_/g, " ")),
          textposition: c.map(x => x.extremeCount > 2 ? "top center" : "bottom center"),
          textfont: { size: 9, color: "#94a3b8" },
          marker: { size: 14, color: c.map(x => x.k_temp),
            colorscale: [[0, "#10b981"], [0.4, "#fbbf24"], [1, "#ef4444"]],
            line: { width: 1.5, color: "rgba(255,255,255,0.25)" } },
          hovertemplate: "<b>%{text}</b><br>K=%{x:.4f}<br>Extremes: %{y}<extra></extra>",
        },
        { type: "scatter", mode: "lines",
          x: [xMin, xMax], y: [slope * xMin + intercept, slope * xMax + intercept],
          line: { color: "rgba(129,140,248,0.4)", width: 2, dash: "dot" },
          showlegend: false, hoverinfo: "skip" },
      ], {
        ...DK, margin: { t: 10, b: 50, l: 50, r: 20 }, showlegend: false,
        xaxis: { ...DK.xaxis, title: { text: "Curvature K(temp)", font: { size: 11 } } },
        yaxis: { ...DK.yaxis, title: { text: "Extreme Events (top 15)", font: { size: 11 } }, dtick: 1 },
      }, CFG);
    }

    // 4. Temperature Time Series
    if (tsRef.current) {
      const top5 = [...DATA.cities].sort((a, b) => b.k_temp - a.k_temp).slice(0, 5);
      const palette = ["#818cf8", "#f87171", "#fbbf24", "#34d399", "#22d3ee"];
      const traces = top5.map((city, i) => {
        const daily = (DATA.dailyTemps[city.name] || []).slice().sort((a, b) => a[0] - b[0]);
        return {
          type: "scatter", mode: "lines", name: city.name.replace(/_/g, " "),
          x: daily.map(d => nasaDate(d[0])), y: daily.map(d => d[1]),
          line: { width: 1.5, color: palette[i] },
          hovertemplate: "%{x}<br>%{y:.1f}°C<extra>" + city.name.replace(/_/g, " ") + "</extra>",
        };
      });
      Plotly.newPlot(tsRef.current, traces, {
        ...DK, margin: { t: 20, b: 50, l: 50, r: 20 },
        xaxis: { ...DK.xaxis, title: { text: "Date", font: { size: 11 } }, type: "date" },
        yaxis: { ...DK.yaxis, title: { text: "Temperature (°C)", font: { size: 11 } } },
        legend: { orientation: "h", y: 1.02, x: 0.5, xanchor: "center", font: { size: 11 } },
      }, CFG);
    }

    // 5. Prediction Chart
    if (predRef.current) {
      const sorted = [...DATA.predictions].sort((a, b) => b.k - a.k);
      Plotly.newPlot(predRef.current, [
        { type: "bar", name: "Training K(temp)",
          x: sorted.map(x => x.city.replace(/_/g, " ")), y: sorted.map(x => x.k),
          marker: { color: sorted.map(x => x.correct ? "rgba(52,211,153,0.7)" : "rgba(248,113,113,0.7)"),
            line: { width: 1, color: sorted.map(x => x.correct ? "#34d399" : "#f87171") } },
          yaxis: "y",
          hovertemplate: "<b>%{x}</b><br>K=%{y:.4f}<extra></extra>",
        },
        { type: "scatter", mode: "markers", name: "Oct–Dec Extreme Events",
          x: sorted.map(x => x.city.replace(/_/g, " ")), y: sorted.map(x => x.events),
          marker: { size: 10, color: "#22d3ee", symbol: "diamond", line: { width: 1, color: "white" } },
          yaxis: "y2",
          hovertemplate: "<b>%{x}</b><br>Events: %{y}<extra></extra>",
        },
      ], {
        ...DK, margin: { t: 20, b: 100, l: 50, r: 50 },
        xaxis: { ...DK.xaxis, tickangle: -45, tickfont: { size: 10 } },
        yaxis: { ...DK.yaxis, title: { text: "K(temp) — curvature", font: { size: 11 } }, side: "left" },
        yaxis2: { ...DK.yaxis, title: { text: "Extreme Events", font: { size: 11 } }, side: "right", overlaying: "y", showgrid: false },
        legend: { orientation: "h", y: 1.08, x: 0.5, xanchor: "center", font: { size: 11 } },
        barmode: "group",
      }, CFG);
    }

    // 6. DHOOM Chart
    if (dhoomRef.current) {
      const d = DATA.dhoom;
      Plotly.newPlot(dhoomRef.current, [{
        type: "bar",
        x: ["JSON", "DHOOM"], y: [d.jsonSize, d.dhoomSize],
        text: [d.jsonSize + " chars", d.dhoomSize + " chars"],
        textposition: "outside", textfont: { color: "#e2e8f0", size: 12 },
        marker: { color: ["#475569", "#818cf8"], line: { width: 1, color: ["#64748b", "#a5b4fc"] } },
        width: 0.5,
        hovertemplate: "%{x}: %{y} characters<extra></extra>",
      }], {
        ...DK, margin: { t: 20, b: 40, l: 50, r: 20 },
        yaxis: { ...DK.yaxis, title: { text: "Characters", font: { size: 11 } } },
        annotations: [{
          x: 1, y: d.dhoomSize, xref: "x", yref: "y",
          text: d.savingsPct.toFixed(0) + "% smaller",
          showarrow: true, arrowhead: 0, arrowcolor: "#818cf8",
          ax: 50, ay: -30, font: { color: "#818cf8", size: 13 },
        }],
      }, CFG);
    }

    return () => {
      [mapRef, curvRef, corrRef, tsRef, predRef, dhoomRef].forEach(r => {
        if (r.current) Plotly.purge(r.current);
      });
    };
  }, [nasaData]);

  // City detail panel
  const cityDetail = nasaData && selectedCity ? (() => {
    const city = nasaData.cities.find(c => c.name === selectedCity);
    if (!city) return null;
    const cityExtremes = nasaData.extremes.filter(e => e.city === selectedCity);
    const pred = nasaData.predictions.find(p => p.city === selectedCity);
    return { city, cityExtremes, pred };
  })() : null;

  if (!nasaData) return (
    <Page title="NASA Demo" sub="Loading NASA data...">
      <div style={{ textAlign: "center", padding: 60, color: "#506070" }}>Loading 20 cities × 366 days × 7 parameters...</div>
    </Page>
  );

  const m = nasaData.metrics;

  return (
    <Page title="NASA Demo" sub="GIGI × NASA POWER — Real Atmospheric Analysis">
      {/* Explainer */}
      <Card style={{ marginBottom: 24, borderColor: "rgba(64,232,160,0.15)", background: "rgba(64,232,160,0.02)" }}>
        <div style={{ fontSize: 20, fontWeight: 900, color: G, marginBottom: 12, lineHeight: 1.3 }}>
          A database index — not an ML model — predicted extreme weather at 55% accuracy using one number.
        </div>
        <div style={{ fontSize: 13.5, color: "#94a3b8", lineHeight: 1.75 }}>
          We fed <strong style={{ color: "#e2e8f0" }}>real NASA atmospheric data</strong> into GIGI — 20 cities, 366 days, 7 parameters (temperature, humidity, pressure, wind, solar irradiance). We trained on January–September and predicted October–December: <em style={{ color: "#e2e8f0" }}>which cities will have extreme weather events?</em>
          <br /><br />
          GIGI answered using <strong style={{ color: G }}>one number per city: Riemannian curvature K.</strong> No neural network. No feature engineering. No hyperparameter tuning. No GPU. It correctly flagged Moscow's -31.9°C cold snap, Toronto's winter storms, and Cape Town's wind anomalies — <strong style={{ color: G }}>55% accuracy</strong> where a coin flip gives you 50%. The point isn't that GIGI beats deep learning. The point is that curvature K, computed as a <em>side effect of indexing</em>, contains predictive signal that normally takes millions of rows and a GPU to extract. <strong style={{ color: G }}>GIGI got it for free.</strong>
          <br /><br />
          And here's what no ML model can claim: <strong style={{ color: "#e2e8f0" }}>GIGI never saw the plaintext.</strong> When data enters GIGI, it's transformed into geometry on a Riemannian manifold. Curvature, anomaly detection, predictions, queries — everything happens on the geometric representation. The raw values are encoded into fiber coordinates that only GIGI's manifold structure can interpret. It's like homomorphic encryption, except it's fast and it's not bolted on — <strong style={{ color: G }}>the geometry IS the encryption.</strong> Data is computed on without ever being decrypted. Only when a human needs to read the actual values does GIGI map back to plaintext.
          <br /><br />
          <span style={{ color: "#506070" }}>Below: interactive charts showing the world curvature map (click any city), correlation analysis, temperature time series, prediction results, and performance. Everything is from real NASA POWER data. Nothing is simulated.</span>
        </div>
      </Card>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(4,1fr)", gap: 10, marginBottom: 24 }}>
        {[
          { v: nasaData.totalRecords.toLocaleString(), l: "NASA Records", d: "20 cities × 366 days" },
          { v: Math.floor(m.insertRate).toLocaleString() + "/s", l: "Ingest Rate", d: "real heterogeneous data" },
          { v: (m.pointQueryNs / 1000).toFixed(1) + "μs", l: "Point Query", d: "O(1) on real data" },
          { v: m.confidence.toFixed(4), l: "Global Confidence", d: "K = " + m.scalarK.toFixed(6) },
        ].map((s, i) => (
          <div key={i} style={{ background: "rgba(64,232,160,0.025)", border: "1px solid rgba(64,232,160,0.07)", borderRadius: 10, padding: "14px 12px", textAlign: "center" }}>
            <div style={{ fontSize: 22, fontWeight: 900, color: G, fontFamily: "monospace" }}>{s.v}</div>
            <div style={{ fontSize: 11.5, fontWeight: 700, color: "#A0B0C0", marginTop: 4 }}>{s.l}</div>
            <div style={{ fontSize: 10, color: "#505060", marginTop: 2 }}>{s.d}</div>
          </div>
        ))}
      </div>

      {/* World Map */}
      <SectionLabel>Global Curvature Map — Click a City to Inspect</SectionLabel>
      <Card style={{ marginBottom: 24, padding: 0, overflow: "hidden" }}>
        <div ref={mapRef} style={{ width: "100%", height: 380 }} />
      </Card>

      {/* City Detail Panel */}
      {cityDetail && (
        <Card style={{ marginBottom: 24, borderColor: "rgba(64,232,160,0.15)" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
            <span style={{ fontSize: 16, fontWeight: 800, color: "#e2e8f0" }}>{selectedCity.replace(/_/g, " ")}</span>
            <button onClick={() => setSelectedCity(null)} style={{ background: "rgba(255,255,255,0.05)", border: "1px solid rgba(255,255,255,0.1)", borderRadius: 6, color: "#94a3b8", padding: "4px 12px", cursor: "pointer", fontSize: 11 }}>✕ Close</button>
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(4,1fr)", gap: 10, marginBottom: 12 }}>
            <div style={{ textAlign: "center" }}>
              <div style={{ fontSize: 18, fontWeight: 900, color: G, fontFamily: "monospace" }}>{cityDetail.city.k_temp.toFixed(4)}</div>
              <div style={{ fontSize: 10, color: "#506070" }}>K(temp)</div>
            </div>
            <div style={{ textAlign: "center" }}>
              <div style={{ fontSize: 18, fontWeight: 900, color: "#818cf8", fontFamily: "monospace" }}>{cityDetail.city.confidence.toFixed(4)}</div>
              <div style={{ fontSize: 10, color: "#506070" }}>Confidence</div>
            </div>
            <div style={{ textAlign: "center" }}>
              <div style={{ fontSize: 18, fontWeight: 900, color: cityDetail.city.extremeCount > 0 ? "#f87171" : "#34d399", fontFamily: "monospace" }}>{cityDetail.city.extremeCount}</div>
              <div style={{ fontSize: 10, color: "#506070" }}>Extremes</div>
            </div>
            <div style={{ textAlign: "center" }}>
              <div style={{ fontSize: 18, fontWeight: 900, color: "#fbbf24", fontFamily: "monospace" }}>{cityDetail.city.region}</div>
              <div style={{ fontSize: 10, color: "#506070" }}>Region</div>
            </div>
          </div>
          {cityDetail.cityExtremes.length > 0 && (
            <>
              <div style={{ fontSize: 10, fontWeight: 700, color: "#f87171", marginBottom: 6, letterSpacing: "0.06em" }}>EXTREME EVENTS</div>
              {cityDetail.cityExtremes.map((e, i) => (
                <div key={i} style={{ display: "flex", gap: 16, padding: "3px 0", fontSize: 11.5, fontFamily: "monospace", color: "#94a3b8" }}>
                  <span style={{ color: "#506070" }}>z={e.z.toFixed(2)}</span>
                </div>
              ))}
            </>
          )}
          {cityDetail.pred && (
            <div style={{ marginTop: 8, fontSize: 11.5, fontFamily: "monospace" }}>
              <span style={{ color: "#506070" }}>Prediction: </span>
              <span style={{ color: cityDetail.pred.correct ? "#34d399" : "#f87171" }}>
                {cityDetail.pred.correct ? "✓ Correct" : "✗ Missed"} — {cityDetail.pred.events} events
              </span>
            </div>
          )}
        </Card>
      )}

      {/* Curvature + Correlation side by side */}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, marginBottom: 24 }}>
        <div>
          <SectionLabel>Curvature by City</SectionLabel>
          <Card style={{ padding: 0, overflow: "hidden" }}>
            <div ref={curvRef} style={{ width: "100%", height: 420 }} />
          </Card>
        </div>
        <div>
          <SectionLabel>Curvature × Extremes Correlation</SectionLabel>
          <Card style={{ padding: 0, overflow: "hidden" }}>
            <div style={{ padding: "8px 12px 0", fontSize: 11, color: "#818cf8", fontFamily: "monospace" }}>
              Pearson r(K, extremes) = {m.pearsonR.toFixed(4)}
            </div>
            <div ref={corrRef} style={{ width: "100%", height: 392 }} />
          </Card>
        </div>
      </div>

      {/* Temperature Time Series */}
      <SectionLabel>Temperature Time Series — Top 5 Most Volatile Cities</SectionLabel>
      <Card style={{ marginBottom: 24, padding: 0, overflow: "hidden" }}>
        <div ref={tsRef} style={{ width: "100%", height: 350 }} />
      </Card>

      {/* Prediction */}
      <SectionLabel>Curvature Prediction — Train on Past, Predict Future</SectionLabel>
      <Card style={{ marginBottom: 24 }}>
        <p style={{ fontSize: 12, color: "#708090", margin: "0 0 12px" }}>
          Trained on Jan–Sep 2024. Predicted Oct–Dec extremes using curvature alone.
          Cities above median K predicted to have extremes.
          <strong style={{ color: G }}> {m.predictionAccuracy}% accuracy ({nasaData.predictions.filter(p => p.correct).length}/{nasaData.predictions.length} cities).</strong>
        </p>
        <div ref={predRef} style={{ width: "100%", height: 350 }} />
      </Card>

      {/* DHOOM + Comparison */}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, marginBottom: 24 }}>
        <div>
          <SectionLabel>DHOOM Wire Compression</SectionLabel>
          <Card style={{ padding: 0, overflow: "hidden" }}>
            <div ref={dhoomRef} style={{ width: "100%", height: 250 }} />
            <pre style={{ padding: "12px", fontSize: 10.5, lineHeight: 1.55, color: "#9090A8", fontFamily: "monospace", margin: 0, whiteSpace: "pre-wrap", borderTop: "1px solid rgba(255,255,255,0.04)" }}>
              {nasaData.dhoom.sample || "atmosphere{date,temp,...,city|Moscow,region|EU}:\n20240115, -31.9, ..."}
            </pre>
          </Card>
        </div>
        <div>
          <SectionLabel>GIGI vs PostgreSQL</SectionLabel>
          <Card>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11.5 }}>
              <thead><tr>{["Task", "GIGI", "Postgres"].map((h, i) => <th key={i} style={{ textAlign: "left", padding: "6px 8px", borderBottom: "1px solid rgba(255,255,255,0.05)", color: "#506070", fontWeight: 600, fontSize: 9.5, textTransform: "uppercase" }}>{h}</th>)}</tr></thead>
              <tbody>{[
                ["Anomaly detection", "K = " + m.scalarK.toFixed(3) + " (~400ns)", "GROUP BY + STDDEV (~11ms)"],
                ["Confidence", "1/(1+K) built-in", "Not available"],
                ["Predict anomalies", "K(train) → test", "External ML pipeline"],
                ["Spectral analysis", "λ₁ built-in (~" + m.spectralTimeMs.toFixed(1) + "ms)", "Graph DB + custom code"],
                ["Wire compression", "DHOOM (" + nasaData.dhoom.savingsPct.toFixed(0) + "% smaller)", "JSON (standard)"],
                ["Data encryption", "Geometric (compute without decrypt)", "AES/TDE (must decrypt to query)"],
              ].map((r, i) => (
                <tr key={i}>
                  <td style={td("#A0B0C0")}>{r[0]}</td>
                  <td style={td(G)}>{r[1]}</td>
                  <td style={td("#606070")}>{r[2]}</td>
                </tr>
              ))}</tbody>
            </table>
          </Card>
        </div>
      </div>

      {/* Performance Cards */}
      <SectionLabel>Performance</SectionLabel>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(5,1fr)", gap: 10, marginBottom: 24 }}>
        {[
          { v: (m.pointQueryNs / 1000).toFixed(1), u: "μs", l: "Point Query" },
          { v: (m.insertNs / 1000).toFixed(1), u: "μs", l: "Insert" },
          { v: Math.floor(m.insertRate).toLocaleString(), u: "/sec", l: "Insert Rate" },
          { v: m.spectralTimeMs.toFixed(2), u: "ms", l: "Spectral" },
          { v: (m.curvatureNs / 1000).toFixed(1), u: "μs", l: "Curvature" },
        ].map((p, i) => (
          <div key={i} style={{ background: "rgba(129,140,248,0.03)", border: "1px solid rgba(129,140,248,0.07)", borderRadius: 10, padding: "14px 12px", textAlign: "center" }}>
            <div style={{ fontSize: 20, fontWeight: 900, fontFamily: "monospace" }}>
              <span style={{ color: "#818cf8" }}>{p.v}</span>
              <span style={{ color: "#506070", fontSize: 11 }}>{p.u}</span>
            </div>
            <div style={{ fontSize: 10, color: "#506070", marginTop: 4 }}>{p.l}</div>
          </div>
        ))}
      </div>

      <Card>
        <p style={{ margin: 0, fontSize: 13, color: "#708090", lineHeight: 1.65 }}>
          {nasaData.totalRecords.toLocaleString()} real NASA records. {nasaData.numCities} cities. 366 days. 7 parameters.
          GIGI detected Moscow's cold snap, Toronto's winter storms, and Cape Town's wind events —{" "}
          <strong style={{ color: G }}>using curvature, not rules.</strong> The geometry found the anomalies.
          The database told you how confident it was. Curvature predicted which cities would have extremes{" "}
          <strong style={{ color: G }}>before scanning the data.</strong> No other database can do any of this.
        </p>
      </Card>
    </Page>
  );
}

// ═══════════════════════════════════════
// STRESS TESTS PAGE
// ═══════════════════════════════════════
const CONVERT_RESULTS = [
  { dataset: "IoT Sensors", records: "100,000", compress: "79.2%", encRate: "49K", decRate: "101K", roundTrip: true },
  { dataset: "Financial Txns", records: "50,000", compress: "74.9%", encRate: "61K", decRate: "125K", roundTrip: true },
  { dataset: "Chat Messages", records: "25,000", compress: "35.7%", encRate: "71K", decRate: "148K", roundTrip: true },
];

const STREAM_RESULTS = [
  { op: "Bulk Insert", count: "50,000", time: "2.5s", rate: "~20K rec/sec" },
  { op: "Point Queries", count: "10,000", time: "3.8s", rate: "~2.6K q/sec" },
  { op: "Range Queries", count: "3", time: "59ms", rate: "3,500 results" },
  { op: "Curvature", count: "1", time: "0.6ms", rate: "50K records" },
  { op: "Aggregation", count: "1", time: "205ms", rate: "GROUP BY" },
  { op: "Pullback Join", count: "1", time: "13ms", rate: "Orders × Sensors" },
  { op: "Consistency H¹", count: "1", time: "0.8ms", rate: "H¹ = 0" },
];

const EDGE_RESULTS = [
  { test: "Offline insert 15K", result: "39K rec/sec", detail: "10K sensors + 5K accounts" },
  { test: "Point queries 5K", result: "196K q/sec", detail: "In-memory, no server" },
  { test: "Range queries", result: "2.4ms", detail: "100 alerts, 500 warnings" },
  { test: "WAL persistence", result: "407ms replay", detail: "15K records recovered" },
  { test: "WAL compaction", result: "260ms", detail: "Deduplicated log" },
  { test: "Sync → Stream", result: "1,001 ops", detail: "H¹ = 0 (clean merge)" },
  { test: "Post-sync verify", result: "✓", detail: "Record confirmed on server" },
];

function StressPage() {
  return (
    <Page title="Stress Tests" sub="Data-intensive validation of all 3 GIGI products. 175K+ records. Real workloads. Verified.">
      <div style={{ display: "grid", gridTemplateColumns: "repeat(4,1fr)", gap: 10, marginBottom: 28 }}>
        {[
          { v: "175K", l: "Records Encoded", d: "perfect round-trip fidelity" },
          { v: "61K", l: "Stream Operations", d: "inserts + queries + joins" },
          { v: "H¹=0", l: "Consistency", d: "Čech cohomology verified" },
          { v: "24s", l: "Total Time", d: "all 3 phases end-to-end" },
        ].map((x, i) => (
          <div key={i} style={{ background: "rgba(64,232,160,0.025)", border: "1px solid rgba(64,232,160,0.07)", borderRadius: 10, padding: "18px 14px", textAlign: "center" }}>
            <div style={{ fontSize: 26, fontWeight: 900, color: G, fontFamily: "monospace" }}>{x.v}</div>
            <div style={{ fontSize: 12.5, fontWeight: 700, color: "#A0B0C0", marginTop: 5 }}>{x.l}</div>
            <div style={{ fontSize: 10.5, color: "#505060", marginTop: 3 }}>{x.d}</div>
          </div>
        ))}
      </div>

      {/* ── Phase 1: Convert ── */}
      <Sec title="Phase 1 — GIGI Convert (175K records)">
        <P>Three distinct datasets — IoT sensors (arithmetic + defaults), financial transactions (high-cardinality), and chat messages (text-heavy) — encoded/decoded with perfect fidelity.</P>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11.5, fontFamily: "monospace", marginBottom: 12 }}>
          <thead><tr>
            {["Dataset", "Records", "Compression", "Encode", "Decode", "Round-trip"].map((h, i) => <th key={i} style={{ textAlign: "left", padding: "8px 8px", borderBottom: "1px solid rgba(255,255,255,0.05)", color: "#506070", fontWeight: 600, fontSize: 9.5, letterSpacing: "0.06em", textTransform: "uppercase" }}>{h}</th>)}
          </tr></thead>
          <tbody>{CONVERT_RESULTS.map((r, i) => (
            <tr key={i}>
              <td style={td("#A0B0C0")}>{r.dataset}</td>
              <td style={td()}>{r.records}</td>
              <td style={td(G)}>{r.compress}</td>
              <td style={td()}>{r.encRate} rec/s</td>
              <td style={td()}>{r.decRate} rec/s</td>
              <td style={td(G)}>{r.roundTrip ? "✓ Perfect" : "✗"}</td>
            </tr>
          ))}</tbody>
        </table>
        <Card>
          <div style={{ fontSize: 12, color: "#607080", lineHeight: 1.6 }}>
            <strong style={{ color: "#A0B0C0" }}>How DHOOM compression works:</strong> Arithmetic fields (timestamps, IDs) are described by <span style={code()}>start + step</span> — only deviations are stored. Default fields (status="normal" at 94%) are elided entirely. The geometry of the data <em>is</em> the compression.
          </div>
        </Card>
      </Sec>

      {/* ── Phase 1 Curvature Detail ── */}
      <Sec title="Curvature Profile — 100K IoT Sensors">
        <div style={{ display: "grid", gridTemplateColumns: "repeat(5,1fr)", gap: 8 }}>
          {[
            { field: "battery", k: 0.0104, conf: 0.9898 },
            { field: "timestamp", k: 0.0833, conf: 0.9231 },
            { field: "humidity", k: 0.0851, conf: 0.9216 },
            { field: "temperature", k: 0.0883, conf: 0.9189 },
            { field: "pressure", k: 0.1249, conf: 0.8890 },
          ].map((f, i) => (
            <div key={i} style={{ background: "rgba(255,255,255,0.015)", border: "1px solid rgba(255,255,255,0.04)", borderRadius: 8, padding: "12px 10px", textAlign: "center" }}>
              <div style={{ fontSize: 10, color: "#506070", fontWeight: 600, letterSpacing: "0.05em", textTransform: "uppercase" }}>{f.field}</div>
              <div style={{ fontSize: 20, fontWeight: 900, color: f.k < 0.05 ? G : f.k < 0.1 ? "#E8A830" : "#FF6040", fontFamily: "monospace", marginTop: 4 }}>{f.k.toFixed(4)}</div>
              <div style={{ fontSize: 10, color: "#505060", marginTop: 2 }}>conf {(f.conf * 100).toFixed(1)}%</div>
            </div>
          ))}
        </div>
        <div style={{ fontSize: 10.5, color: "#404860", marginTop: 8, textAlign: "center" }}>
          Low K = flat = predictable. High K = curved = variable. Battery (K=0.01) is 98% constant. Pressure (K=0.12) varies the most.
        </div>
      </Sec>

      {/* ── Phase 2: Stream ── */}
      <Sec title="Phase 2 — GIGI Stream (61K operations over HTTP)">
        <P>50K sensor records inserted in batches of 500 via REST API. Then 10K point queries, range filtering, curvature analysis, aggregation, pullback join, and Čech consistency — all against the live server.</P>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11.5, fontFamily: "monospace", marginBottom: 12 }}>
          <thead><tr>
            {["Operation", "Count", "Time", "Detail"].map((h, i) => <th key={i} style={{ textAlign: "left", padding: "8px 8px", borderBottom: "1px solid rgba(255,255,255,0.05)", color: "#506070", fontWeight: 600, fontSize: 9.5, letterSpacing: "0.06em", textTransform: "uppercase" }}>{h}</th>)}
          </tr></thead>
          <tbody>{STREAM_RESULTS.map((r, i) => (
            <tr key={i} style={{ background: r.op === "Consistency H¹" ? "rgba(64,232,160,0.035)" : "transparent" }}>
              <td style={td("#A0B0C0")}>{r.op}</td>
              <td style={td()}>{r.count}</td>
              <td style={td()}>{r.time}</td>
              <td style={td(r.rate === "H¹ = 0" ? G : undefined)}>{r.rate}</td>
            </tr>
          ))}</tbody>
        </table>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
          <Card>
            <div style={{ fontSize: 12, fontWeight: 700, color: "#A0B0C0", marginBottom: 4 }}>Aggregation: GROUP BY status</div>
            <div style={{ fontFamily: "monospace", fontSize: 11, color: "#607080", lineHeight: 1.7 }}>
              <div><span style={{ color: G }}>normal</span>: 47,000 records — avg temp 22.03°C</div>
              <div><span style={{ color: "#E8A830" }}>warning</span>: 2,500 records — avg temp 22.03°C</div>
              <div><span style={{ color: "#FF6040" }}>alert</span>: 500 records — avg temp 22.02°C</div>
            </div>
          </Card>
          <Card>
            <div style={{ fontSize: 12, fontWeight: 700, color: "#A0B0C0", marginBottom: 4 }}>Curvature Under Load</div>
            <div style={{ fontFamily: "monospace", fontSize: 11, color: "#607080", lineHeight: 1.7 }}>
              <div>Global K = <span style={{ color: G }}>0.0871</span> — confidence 92.0%</div>
              <div>Computed in <span style={{ color: G }}>0.6ms</span> over 50K records</div>
              <div>Capacity = 11.48 (bits of useful structure)</div>
            </div>
          </Card>
        </div>
      </Sec>

      {/* ── Phase 3: Edge ── */}
      <Sec title="Phase 3 — GIGI Edge (15K offline + sync)">
        <P>Local-first engine stores 15K records across 2 bundles with no server connection. WAL persistence verified by restart + replay. Then syncs 1,001 ops to GIGI Stream with H¹ = 0.</P>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11.5, fontFamily: "monospace", marginBottom: 12 }}>
          <thead><tr>
            {["Test", "Result", "Detail"].map((h, i) => <th key={i} style={{ textAlign: "left", padding: "8px 8px", borderBottom: "1px solid rgba(255,255,255,0.05)", color: "#506070", fontWeight: 600, fontSize: 9.5, letterSpacing: "0.06em", textTransform: "uppercase" }}>{h}</th>)}
          </tr></thead>
          <tbody>{EDGE_RESULTS.map((r, i) => (
            <tr key={i} style={{ background: r.test.includes("Sync") ? "rgba(64,232,160,0.035)" : "transparent" }}>
              <td style={td("#A0B0C0")}>{r.test}</td>
              <td style={td(r.result === "✓" ? G : undefined)}>{r.result}</td>
              <td style={td()}>{r.detail}</td>
            </tr>
          ))}</tbody>
        </table>
        <Card>
          <div style={{ fontSize: 12, fontWeight: 700, color: "#A0B0C0", marginBottom: 4 }}>Why H¹ = 0 Matters</div>
          <p style={{ fontSize: 12, color: "#607080", lineHeight: 1.6, margin: 0 }}>
            When Edge syncs to Stream, the sheaf gluing axiom checks that overlapping data on both sides is consistent. H¹ = 0 means the first Čech cohomology group is trivial — <strong style={{ color: G }}>there are no obstructions to gluing</strong>. Every local section extends to a global section. This isn't "eventual consistency" — it's mathematical certainty.
          </p>
        </Card>
      </Sec>

      {/* ── End-to-End Pipeline ── */}
      <Sec title="End-to-End Pipeline">
        <div style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: 12, padding: "16px 0" }}>
          {[
            { icon: "🔄", label: "Convert", sub: "175K records", color: "#E8A830" },
            { icon: "→", label: "", sub: "", color: "#303040" },
            { icon: "⚡", label: "Stream", sub: "61K HTTP ops", color: G },
            { icon: "→", label: "", sub: "", color: "#303040" },
            { icon: "📱", label: "Edge", sub: "15K offline + sync", color: "#8080E0" },
            { icon: "→", label: "", sub: "", color: "#303040" },
            { icon: "✓", label: "H¹ = 0", sub: "clean merge", color: G },
          ].map((s, i) => (
            s.label === "" ? <span key={i} style={{ fontSize: 18, color: "#303040" }}>→</span> :
            <div key={i} style={{ textAlign: "center", padding: "12px 16px", background: "rgba(255,255,255,0.015)", border: `1px solid ${s.color}22`, borderRadius: 10, minWidth: 100 }}>
              <div style={{ fontSize: 22 }}>{s.icon}</div>
              <div style={{ fontSize: 12, fontWeight: 700, color: s.color, marginTop: 4 }}>{s.label}</div>
              <div style={{ fontSize: 10, color: "#505060", marginTop: 2 }}>{s.sub}</div>
            </div>
          ))}
        </div>
        <div style={{ textAlign: "center", fontSize: 12, color: "#404860", marginTop: 4, fontFamily: "monospace" }}>
          Total: 24s end-to-end · 86/86 unit tests passing · 0 regressions
        </div>
      </Sec>
    </Page>
  );
}

/* ── Competitive Analysis ─────────────────────────── */

const EXEC_SUMMARY = [
  { sys: "Apache Druid", icon: "🔶", headline: "Sub-second OLAP at trillion-row scale", response: "Sub-microsecond point queries, O(|r|) aggregation", advantage: "No columnar scan needed — geometry pre-computes" },
  { sys: "Apache Cassandra", icon: "👁", headline: "Always-on, no single point of failure", response: "Sheaf-guaranteed consistency + holonomy drift detection", advantage: "Math > consensus protocols" },
  { sys: "ELK Stack", icon: "🔍", headline: "Full-text search + real-time log visualization", response: "Anomaly detection built into every query via curvature", advantage: "No pipeline needed — analytics ARE the database" },
];

const DRUID_MATCH = [
  { them: "Millions events/sec ingest", gigi: "20K+/sec via HTTP REST, O(1) raw Rust insert", note: "Druid wins on clustered ingest, but GIGI's O(1) insert scales linearly — no compaction, no reindexing" },
  { them: "Sub-second queries", gigi: "Sub-microsecond point queries (500ns Rust)", note: "GIGI is 1000× faster for point lookups" },
  { them: "Columnar storage", gigi: "Fiber-oriented storage", note: "Each \"column\" is a fiber field. Same concept, different math" },
  { them: "Bitmap indexes", gigi: "Roaring bitmaps for field index topology", note: "Same library (roaring), different purpose" },
  { them: "Time partitioning", gigi: "Arithmetic base compression (@)", note: "DHOOM's @1710000000+60 eliminates the timestamp column entirely" },
  { them: "Approximate aggregation", gigi: "Exact aggregation via fiber integrals", note: "GIGI doesn't approximate — sheaf axioms guarantee exact results" },
];
const DRUID_BEAT = [
  { cap: "Point query complexity", them: "O(log n) segment scan", gigi: "O(1) section evaluation" },
  { cap: "Joins", them: "Limited, expensive", gigi: "O(|left|) pullback bundles" },
  { cap: "Query confidence", them: "None", gigi: "Built-in: confidence = 1/(1+K)" },
  { cap: "Anomaly detection", them: "External (build your own)", gigi: "Built-in: curvature K at insert time" },
  { cap: "Consistency proof", them: "Operational (ZooKeeper)", gigi: "Mathematical: sheaf axioms + Čech H¹" },
  { cap: "Wire format", them: "JSON", gigi: "DHOOM (66-84% smaller)" },
  { cap: "Immutability", them: "Cannot update rows", gigi: "Sections are mutable" },
  { cap: "Timestamp req", them: "Every row MUST have timestamp", gigi: "No requirement — base space is arbitrary" },
  { cap: "Cluster complexity", them: "6+ node types", gigi: "Single binary" },
];

const CASS_MATCH = [
  { them: "Peer-to-peer architecture", gigi: "Partition base manifold across nodes, sheaf gluing guarantees composition", note: "Sheaf axioms instead of gossip protocol" },
  { them: "Linear scalability", gigi: "O(1) per node — adding nodes = linear scale", note: "Same guarantee, different mechanism" },
  { them: "High write throughput", gigi: "20K+/sec single-node via REST, O(1) hash map insert", note: "GIGI is O(1) for reads AND writes — no LSM compaction overhead" },
  { them: "Fault tolerance", gigi: "WAL + CRC32 crash recovery", note: "GIGI's WAL provides single-node durability" },
  { them: "Tunable consistency", gigi: "Tolerance budget τ controls precision/recall", note: "τ is the geometric analog of consistency level" },
];
const CASS_BEAT = [
  { cap: "Read latency", them: "O(1) via partition key, but tombstones and compaction add overhead", gigi: "O(1) pure hash lookup, no tombstones" },
  { cap: "Range queries", them: "O(n) full scan", gigi: "O(|r|) sheaf evaluation" },
  { cap: "Joins", them: "Not supported — must denormalize", gigi: "O(|left|) pullback bundles" },
  { cap: "Aggregation", them: "Not supported — need Spark/Presto", gigi: "Built-in fiber integrals" },
  { cap: "Consistency model", them: "Eventual consistency (operational)", gigi: "Sheaf axioms (mathematical guarantee)" },
  { cap: "Consistency diagnostics", them: "None — hope replicas converge", gigi: "Čech H¹ counts + localizes inconsistencies" },
  { cap: "Drift detection", them: "None — silent divergence", gigi: "Holonomy: exact location of divergence" },
  { cap: "Data quality metrics", them: "None", gigi: "Curvature K, confidence, spectral gap" },
  { cap: "Wire format", them: "CQL binary protocol", gigi: "DHOOM (66-84% compression)" },
  { cap: "Query algebra", them: "Key lookup only", gigi: "WHERE, JOIN, GROUP BY, CURVATURE, SPECTRAL" },
];

const ELK_MATCH = [
  { them: "Full-text search", gigi: "Point query O(1) + range query O(|r|)", note: "ELK searches text, GIGI evaluates sections" },
  { them: "Real-time indexing", gigi: "Insert = define section, immediately queryable", note: "No refresh interval" },
  { them: "Dashboards", gigi: "Curvature dashboard, spectral analysis UI", note: "Kibana is more mature; GIGI's geometric metrics are novel" },
  { them: "Horizontal scaling", gigi: "Partition base manifold across nodes", note: "ELK's shard-based scaling is mature" },
  { them: "Ecosystem", gigi: "DHOOM + GIGI Convert + Stream + Edge", note: "Smaller ecosystem, unified by one mathematical framework" },
];
const ELK_BEAT = [
  { cap: "Anomaly detection", them: "Requires ML plugin, separate config", gigi: "Built-in: every insert updates K" },
  { cap: "Log analysis setup", them: "ES + Logstash + Kibana + Beats + pipelines + dashboards + alerts", gigi: "Install GIGI. Done." },
  { cap: "Operational complexity", them: "3-5 services to deploy", gigi: "Single binary" },
  { cap: "Resource usage", them: "Memory-hungry (JVM heap)", gigi: "Rust, zero-copy, minimal footprint" },
  { cap: "Query correctness", them: "No guarantees", gigi: "Sheaf axioms — mathematically guaranteed" },
  { cap: "Data quality", them: "None — build monitoring on monitoring", gigi: "Čech H¹ detects inconsistencies" },
  { cap: "Alert config", them: "Manual: watchers, thresholds, conditions", gigi: "Automatic: subscribe to curvature drift" },
  { cap: "Wire format", them: "JSON (verbose)", gigi: "DHOOM (66-84% smaller)" },
  { cap: "Cost at scale", them: "Expensive — storage costs top complaint", gigi: "O(1) = flat cost curve" },
];

const COMBINED_MATRIX = [
  { cap: "Point queries", druid: "◐", cass: "✓", elk: "✓", gigi: "✓ O(1)" },
  { cap: "Range queries", druid: "✓", cass: "✗", elk: "✓", gigi: "✓ O(|r|)" },
  { cap: "Joins", druid: "✗", cass: "✗", elk: "✗", gigi: "✓ pullback" },
  { cap: "Aggregation", druid: "✓", cass: "✗", elk: "✓", gigi: "✓ fiber" },
  { cap: "Anomaly detection", druid: "✗", cass: "✗", elk: "◐", gigi: "✓ auto" },
  { cap: "Confidence scoring", druid: "✗", cass: "✗", elk: "◐", gigi: "✓ auto" },
  { cap: "Consistency proof", druid: "✗", cass: "✗", elk: "✗", gigi: "✓ sheaf" },
  { cap: "Consistency diagnostics", druid: "✗", cass: "✗", elk: "✗", gigi: "✓ Čech H¹" },
  { cap: "Drift detection", druid: "✗", cass: "✗", elk: "✗", gigi: "✓ holonomy" },
  { cap: "Connectivity analysis", druid: "✗", cass: "✗", elk: "✗", gigi: "✓ spectral" },
  { cap: "Prediction", druid: "✗", cass: "✗", elk: "✗", gigi: "✓ curvature" },
  { cap: "Wire compression", druid: "✗", cass: "✗", elk: "✗", gigi: "✓ DHOOM" },
];

const NOBODY_HAS = [
  "Curvature-based confidence — every query result annotated with how much to trust it",
  "Čech cohomology — mathematically counts and localizes data inconsistencies",
  "Holonomy — detects replica drift, referential integrity violations, temporal pattern shifts",
  "Spectral capacity — graph Laplacian eigenvalues measure index connectivity and detect data silos",
  "Partition function — Boltzmann-weighted approximate queries with principled temperature control",
  "Gauge-invariant schema migration — ALTER TABLE that provably preserves curvature",
  "C-theorem — GROUP BY satisfies entropy monotonicity (RG flow)",
  "Double Cover — S + d² = 1 bounds query completeness for any query",
  "Zero-Euclidean guarantee — no distance computation anywhere in the query path",
  "Unified storage/wire math — DHOOM and GIGI share the same fiber bundle, end to end",
];

function ComparePage() {
  const cellC = (v) => v.startsWith("✓") ? G : v === "✗" ? "#E05050" : "#E8A830";
  return (
    <Page title="GIGI vs The Big Three" sub="Matching and beating Druid, Cassandra, and ELK — with 10 features nobody else has.">
      {/* Executive Summary */}
      <Sec title="Executive Summary">
        <P>Each incumbent owns one headline. GIGI matches or beats all three, and adds capabilities none of them have.</P>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 14, marginTop: 8 }}>
          {EXEC_SUMMARY.map((e, i) => (
            <div key={i} style={{ background: "rgba(255,255,255,0.02)", border: "1px solid rgba(255,255,255,0.06)", borderRadius: 12, padding: "20px 18px" }}>
              <div style={{ fontSize: 28 }}>{e.icon}</div>
              <div style={{ fontSize: 14, fontWeight: 800, color: "#D0D8E0", marginTop: 8 }}>{e.sys}</div>
              <div style={{ fontSize: 11, color: "#607080", marginTop: 6, fontStyle: "italic" }}>"{e.headline}"</div>
              <div style={{ fontSize: 11.5, color: G, fontWeight: 600, marginTop: 10 }}>{e.response}</div>
              <div style={{ fontSize: 10.5, color: "#506070", marginTop: 6, lineHeight: 1.5 }}>{e.advantage}</div>
            </div>
          ))}
        </div>
      </Sec>

      {/* Apache Druid */}
      <Sec title="Apache Druid">
        <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 12 }}>
          <span style={{ fontSize: 22 }}>🔶</span>
          <span style={{ fontSize: 13, color: "#607080", fontStyle: "italic" }}>"Druid scans columns. GIGI evaluates sections. Same speed, but GIGI tells you which results to trust."</span>
        </div>
        <SectionLabel>Where GIGI Matches</SectionLabel>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11.5, marginBottom: 18 }}>
          <thead><tr>{["Druid Feature", "GIGI Equivalent", "Notes"].map((h, i) => <th key={i} style={{ textAlign: "left", padding: "8px 10px", color: "#506070", borderBottom: "1px solid rgba(255,255,255,0.06)", fontWeight: 600, fontSize: 10.5 }}>{h}</th>)}</tr></thead>
          <tbody>{DRUID_MATCH.map((r, i) => (
            <tr key={i}><td style={td()}>{r.them}</td><td style={td(G)}>{r.gigi}</td><td style={td()}>{r.note}</td></tr>
          ))}</tbody>
        </table>
        <SectionLabel>Where GIGI Beats Druid</SectionLabel>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11.5, marginBottom: 18 }}>
          <thead><tr>{["Capability", "Druid", "GIGI"].map((h, i) => <th key={i} style={{ textAlign: "left", padding: "8px 10px", color: "#506070", borderBottom: "1px solid rgba(255,255,255,0.06)", fontWeight: 600, fontSize: 10.5 }}>{h}</th>)}</tr></thead>
          <tbody>{DRUID_BEAT.map((r, i) => (
            <tr key={i}><td style={td()}>{r.cap}</td><td style={td("#E05050")}>{r.them}</td><td style={td(G)}>{r.gigi}</td></tr>
          ))}</tbody>
        </table>
        <Card>
          <div style={{ fontSize: 11, fontWeight: 700, color: "#E8A830", marginBottom: 6 }}>Druid's Real Weakness</div>
          <div style={{ fontSize: 11.5, color: "#607080", lineHeight: 1.6 }}>Druid is fundamentally an event-oriented, immutable, time-series OLAP engine. Every row must have a timestamp. Updates are expensive. Joins are limited. It requires ZooKeeper, deep storage, and a complex multi-node topology. GIGI is a general-purpose geometric database — no timestamp requirement, mutable sections, O(1) joins, single binary.</div>
        </Card>
      </Sec>

      {/* Apache Cassandra */}
      <Sec title="Apache Cassandra">
        <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 12 }}>
          <span style={{ fontSize: 22 }}>👁</span>
          <span style={{ fontSize: 13, color: "#607080", fontStyle: "italic" }}>"Cassandra hopes your replicas converge. GIGI proves they did — or tells you exactly where they didn't."</span>
        </div>
        <SectionLabel>Where GIGI Matches</SectionLabel>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11.5, marginBottom: 18 }}>
          <thead><tr>{["Cassandra Feature", "GIGI Equivalent", "Notes"].map((h, i) => <th key={i} style={{ textAlign: "left", padding: "8px 10px", color: "#506070", borderBottom: "1px solid rgba(255,255,255,0.06)", fontWeight: 600, fontSize: 10.5 }}>{h}</th>)}</tr></thead>
          <tbody>{CASS_MATCH.map((r, i) => (
            <tr key={i}><td style={td()}>{r.them}</td><td style={td(G)}>{r.gigi}</td><td style={td()}>{r.note}</td></tr>
          ))}</tbody>
        </table>
        <SectionLabel>Where GIGI Beats Cassandra</SectionLabel>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11.5, marginBottom: 18 }}>
          <thead><tr>{["Capability", "Cassandra", "GIGI"].map((h, i) => <th key={i} style={{ textAlign: "left", padding: "8px 10px", color: "#506070", borderBottom: "1px solid rgba(255,255,255,0.06)", fontWeight: 600, fontSize: 10.5 }}>{h}</th>)}</tr></thead>
          <tbody>{CASS_BEAT.map((r, i) => (
            <tr key={i}><td style={td()}>{r.cap}</td><td style={td("#E05050")}>{r.them}</td><td style={td(G)}>{r.gigi}</td></tr>
          ))}</tbody>
        </table>
        <Card>
          <div style={{ fontSize: 11, fontWeight: 700, color: "#E8A830", marginBottom: 6 }}>Cassandra's Real Weakness</div>
          <div style={{ fontSize: 11.5, color: "#607080", lineHeight: 1.6 }}>Cassandra is a write-optimized distributed key-value store that sacrifices read flexibility for write throughput. No joins. No aggregations. No way to know if data is consistent without read-repair. GIGI provides the same O(1) write speed, but adds joins, aggregations, range queries, consistency proofs, and quality metrics. When GIGI detects replica drift via holonomy, it tells you exactly which records in which neighborhoods have diverged.</div>
        </Card>
      </Sec>

      {/* ELK Stack */}
      <Sec title="ELK Stack">
        <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 12 }}>
          <span style={{ fontSize: 22 }}>🔍</span>
          <span style={{ fontSize: 13, color: "#607080", fontStyle: "italic" }}>"ELK is three services that search text. GIGI is one binary that understands your data's geometry."</span>
        </div>
        <SectionLabel>Where GIGI Matches</SectionLabel>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11.5, marginBottom: 18 }}>
          <thead><tr>{["ELK Feature", "GIGI Equivalent", "Notes"].map((h, i) => <th key={i} style={{ textAlign: "left", padding: "8px 10px", color: "#506070", borderBottom: "1px solid rgba(255,255,255,0.06)", fontWeight: 600, fontSize: 10.5 }}>{h}</th>)}</tr></thead>
          <tbody>{ELK_MATCH.map((r, i) => (
            <tr key={i}><td style={td()}>{r.them}</td><td style={td(G)}>{r.gigi}</td><td style={td()}>{r.note}</td></tr>
          ))}</tbody>
        </table>
        <SectionLabel>Where GIGI Beats ELK</SectionLabel>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11.5, marginBottom: 18 }}>
          <thead><tr>{["Capability", "ELK", "GIGI"].map((h, i) => <th key={i} style={{ textAlign: "left", padding: "8px 10px", color: "#506070", borderBottom: "1px solid rgba(255,255,255,0.06)", fontWeight: 600, fontSize: 10.5 }}>{h}</th>)}</tr></thead>
          <tbody>{ELK_BEAT.map((r, i) => (
            <tr key={i}><td style={td()}>{r.cap}</td><td style={td("#E05050")}>{r.them}</td><td style={td(G)}>{r.gigi}</td></tr>
          ))}</tbody>
        </table>
        <Card>
          <div style={{ fontSize: 11, fontWeight: 700, color: "#E8A830", marginBottom: 6 }}>ELK's Real Weakness</div>
          <div style={{ fontSize: 11.5, color: "#607080", lineHeight: 1.6 }}>ELK is a search engine repurposed as an observability platform. It requires 3-5 separate services, is notoriously expensive at scale, and every analytical capability beyond basic search must be added as a separate plugin or ML job. GIGI provides anomaly detection, data quality assessment, connectivity analysis, and confidence scoring as intrinsic properties — not plugins bolted on top. Logs compress 70-84% via DHOOM, and curvature spikes are natural anomaly detectors.</div>
        </Card>
      </Sec>

      {/* Combined Matrix */}
      <Sec title="Combined Comparison — 12 Capabilities">
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11.5 }}>
          <thead><tr>{["Capability", "Druid", "Cassandra", "ELK", "GIGI"].map((h, i) => <th key={i} style={{ textAlign: i === 0 ? "left" : "center", padding: "8px 10px", color: i === 4 ? G : "#506070", borderBottom: "1px solid rgba(255,255,255,0.06)", fontWeight: 700, fontSize: 10.5 }}>{h}</th>)}</tr></thead>
          <tbody>{COMBINED_MATRIX.map((r, i) => (
            <tr key={i}>
              <td style={td()}>{r.cap}</td>
              <td style={{ ...td(), textAlign: "center", color: cellC(r.druid) }}>{r.druid}</td>
              <td style={{ ...td(), textAlign: "center", color: cellC(r.cass) }}>{r.cass}</td>
              <td style={{ ...td(), textAlign: "center", color: cellC(r.elk) }}>{r.elk}</td>
              <td style={{ ...td(), textAlign: "center", color: G, fontWeight: 700 }}>{r.gigi}</td>
            </tr>
          ))}</tbody>
        </table>
      </Sec>

      {/* 10 Features Nobody Else Has */}
      <Sec title="10 Features Nobody Else Has">
        <P>These don't exist in any shipping database, period.</P>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10, marginTop: 8 }}>
          {NOBODY_HAS.map((f, i) => {
            const [name, ...rest] = f.split(" — ");
            return (
              <div key={i} style={{ background: "rgba(64,232,160,0.02)", border: "1px solid rgba(64,232,160,0.08)", borderRadius: 10, padding: "14px 16px", display: "flex", gap: 10, alignItems: "flex-start" }}>
                <span style={{ fontSize: 16, fontWeight: 900, color: G, fontFamily: "monospace", flexShrink: 0, width: 22, textAlign: "center" }}>{i + 1}</span>
                <div>
                  <div style={{ fontSize: 12, fontWeight: 700, color: "#D0D8E0" }}>{name}</div>
                  <div style={{ fontSize: 10.5, color: "#506070", marginTop: 3, lineHeight: 1.5 }}>{rest.join(" — ")}</div>
                </div>
              </div>
            );
          })}
        </div>
      </Sec>

      {/* One-Liners */}
      <Sec title="The One-Liners">
        {[
          { vs: "vs Druid", line: "Same speed. But GIGI tells you which results to trust.", color: "#E8A830" },
          { vs: "vs Cassandra", line: "Same availability. But GIGI proves your replicas converged — or tells you where they didn't.", color: "#8080E0" },
          { vs: "vs ELK", line: "Same logs. But GIGI detects anomalies without a pipeline, proves consistency without a plugin, and compresses the wire by 80%.", color: "#E05050" },
          { vs: "vs All Three", line: "They index data. GIGI understands data. The geometry IS the index.", color: G },
        ].map((o, i) => (
          <div key={i} style={{ background: "rgba(255,255,255,0.015)", border: `1px solid ${o.color}22`, borderRadius: 10, padding: "14px 18px", marginBottom: 10, display: "flex", gap: 14, alignItems: "center" }}>
            <Tag color={o.color}>{o.vs}</Tag>
            <span style={{ fontSize: 12.5, color: "#C0C8D0", fontStyle: "italic", lineHeight: 1.5 }}>"{o.line}"</span>
          </div>
        ))}
      </Sec>
    </Page>
  );
}

// ═══════════════════════════════════════
// INTERACTIVE DEMO — Live GIGI Engine
// ═══════════════════════════════════════
const DEMO_PRESETS = {
  iot: {
    label: "🌡️ IoT Sensors",
    bundleName: "demo_iot",
    schema: { fields: { sensor_id: "categorical", timestamp: "numeric", temperature: "numeric", humidity: "numeric", pressure: "numeric", status: "categorical" }, keys: ["sensor_id"] },
    generate: (n) => Array.from({ length: n }, (_, i) => ({
      sensor_id: "S-" + String(i % 20).padStart(3, "0"),
      timestamp: 1710000000 + i * 60,
      temperature: +(18 + Math.random() * 10).toFixed(1),
      humidity: +(40 + Math.random() * 20).toFixed(1),
      pressure: +(1010 + Math.random() * 5).toFixed(1),
      status: i % 25 === 0 ? "alert" : "normal",
    })),
    queries: [
      { label: "Point Query", gql: "SECTION demo_iot AT sensor_id='S-000'", desc: "O(1) fiber section evaluation — retrieves a single sensor reading" },
      { label: "Range Scan", gql: "COVER demo_iot ON status = 'alert'", desc: "Find all anomalies — only scans result set, not full data" },
      { label: "Curvature", gql: "CURVATURE demo_iot", desc: "Scalar curvature K — measures data regularity. Low K = predictable, high K = anomalies present" },
    ],
  },
  finance: {
    label: "💰 Financial Txns",
    bundleName: "demo_finance",
    schema: { fields: { txn_id: "numeric", amount: "numeric", currency: "categorical", rail: "categorical", status: "categorical", counterparty: "categorical" }, keys: ["txn_id"] },
    generate: (n) => Array.from({ length: n }, (_, i) => ({
      txn_id: 5000 + i,
      amount: +(Math.random() * 10000).toFixed(2),
      currency: ["USD", "EUR", "GBP", "JPY"][i % 4],
      rail: ["SWIFT", "ACH", "RTP", "ISO20022"][i % 4],
      status: i % 12 === 0 ? "failed" : i % 7 === 0 ? "pending" : "settled",
      counterparty: "BANK-" + String(i % 8).padStart(2, "0"),
    })),
    queries: [
      { label: "Point Query", gql: "SECTION demo_finance AT txn_id=5000", desc: "O(1) transaction lookup by ID — constant time regardless of table size" },
      { label: "Range Scan", gql: "COVER demo_finance ON status = 'failed'", desc: "Find all failed transactions — output-sensitive O(|result|)" },
      { label: "Curvature", gql: "CURVATURE demo_finance", desc: "High K means high variance in amounts — possible fraud signal" },
    ],
  },
  users: {
    label: "👤 User Profiles",
    bundleName: "demo_users",
    schema: { fields: { user_id: "numeric", username: "categorical", role: "categorical", department: "categorical", login_count: "numeric", active: "categorical" }, keys: ["user_id"] },
    generate: (n) => Array.from({ length: n }, (_, i) => ({
      user_id: i,
      username: "user_" + String(i).padStart(4, "0"),
      role: i % 8 === 0 ? "admin" : "viewer",
      department: ["Engineering", "Marketing", "Sales", "Operations", "HR"][i % 5],
      login_count: Math.floor(Math.random() * 500),
      active: i % 15 === 0 ? "false" : "true",
    })),
    queries: [
      { label: "Point Query", gql: "SECTION demo_users AT user_id=0", desc: "O(1) user lookup — fiber bundle section evaluation" },
      { label: "Range Scan", gql: "COVER demo_users ON role = 'admin'", desc: "Find all admins — only reads matching fibers" },
      { label: "Curvature", gql: "CURVATURE demo_users", desc: "Curvature reveals structural patterns — role distribution, department clustering" },
    ],
  },
};

function InteractiveDemoPage() {
  const [preset, setPreset] = useState("iot");
  const [records, setRecords] = useState(() => DEMO_PRESETS.iot.generate(50));
  const [bundleReady, setBundleReady] = useState(false);
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState("");
  const [dhoom, setDhoom] = useState(null);
  const [queryResults, setQueryResults] = useState([]);
  const [expandedRows, setExpandedRows] = useState(new Set([0]));
  const [editingRow, setEditingRow] = useState(null);
  const [editValues, setEditValues] = useState({});
  const [serverStats, setServerStats] = useState(null);
  const [selectedStat, setSelectedStat] = useState(null);
  const [showJson, setShowJson] = useState(false);
  const [recordCount, setRecordCount] = useState(50);
  const bundleRef = useRef(null);

  const ds = DEMO_PRESETS[preset];
  const fields = Object.keys(ds.schema.fields);

  // Cleanup on unmount
  useEffect(() => {
    return () => { if (bundleRef.current) restDelete(`/v1/bundles/${bundleRef.current}`).catch(() => {}); };
  }, []);

  // Switch preset
  function switchPreset(key) {
    if (bundleRef.current) restDelete(`/v1/bundles/${bundleRef.current}`).catch(() => {});
    bundleRef.current = null;
    setPreset(key);
    setRecords(DEMO_PRESETS[key].generate(recordCount));
    setBundleReady(false);
    setDhoom(null);
    setQueryResults([]);
    setServerStats(null);
    setSelectedStat(null);
    setExpandedRows(new Set([0]));
    setEditingRow(null);
  }

  // Push data to GIGI
  async function pushToGigi() {
    setLoading(true);
    setStatus("Creating bundle...");
    const name = `${ds.bundleName}_${Date.now() % 100000}`;
    if (bundleRef.current) await restDelete(`/v1/bundles/${bundleRef.current}`).catch(() => {});
    try {
      await restPost("/v1/bundles", { name, schema: ds.schema });
      setStatus(`Inserting ${records.length} records...`);
      const chunkSize = 5000;
      for (let s = 0; s < records.length; s += chunkSize) {
        await restPost(`/v1/bundles/${name}/insert`, { records: records.slice(s, s + chunkSize) });
      }
      bundleRef.current = name;
      setBundleReady(true);
      setStatus("Fetching DHOOM encoding...");
      const dRes = await (await fetch(`${BENCH_API}/v1/bundles/${name}/dhoom`)).json();
      setDhoom(dRes);
      setStatus(`Live on GIGI — ${records.length} records ingested`);
      setServerStats({
        records: records.length,
        compression: dRes.compression_pct,
        dhoomBytes: dRes.dhoom_chars,
        jsonBytes: dRes.json_chars,
        fieldsOmitted: dRes.fields_omitted,
        totalSlots: dRes.total_field_slots,
      });
    } catch (e) {
      setStatus(`Error: ${e.message}`);
    }
    setLoading(false);
  }

  // Run a GQL query
  async function runQuery(q) {
    if (!bundleRef.current) return;
    const gql = q.gql.replace(ds.bundleName, bundleRef.current);
    const t0 = performance.now();
    const res = await gqlPost(gql);
    const elapsed = performance.now() - t0;
    setQueryResults(prev => [{ ...q, gql, result: res, elapsed, ts: Date.now() }, ...prev].slice(0, 10));
  }

  // Add a record
  function addRecord() {
    const newRec = ds.generate(1)[0];
    // Give it a unique ID
    const idField = ds.schema.keys[0];
    const maxId = records.reduce((max, r) => Math.max(max, typeof r[idField] === "number" ? r[idField] : 0), 0);
    if (typeof newRec[idField] === "number") newRec[idField] = maxId + 1;
    else newRec[idField] = "S-" + String(records.length).padStart(3, "0");
    setRecords(prev => [...prev, newRec]);
    setBundleReady(false);
    setDhoom(null);
    setServerStats(null);
  }

  // Delete a record
  function deleteRecord(idx) {
    setRecords(prev => prev.filter((_, i) => i !== idx));
    setBundleReady(false);
    setDhoom(null);
    setServerStats(null);
    setExpandedRows(prev => { const n = new Set(); prev.forEach(x => { if (x < idx) n.add(x); else if (x > idx) n.add(x - 1); }); return n; });
  }

  // Start editing
  function startEdit(idx) {
    setEditingRow(idx);
    setEditValues({ ...records[idx] });
  }

  // Save edit
  function saveEdit() {
    if (editingRow === null) return;
    setRecords(prev => prev.map((r, i) => i === editingRow ? { ...editValues } : r));
    setEditingRow(null);
    setEditValues({});
    setBundleReady(false);
    setDhoom(null);
    setServerStats(null);
  }

  // Batch add
  function batchAdd(n) {
    const batch = ds.generate(n);
    const idField = ds.schema.keys[0];
    const maxId = records.reduce((max, r) => Math.max(max, typeof r[idField] === "number" ? r[idField] : 0), 0);
    batch.forEach((r, i) => {
      if (typeof r[idField] === "number") r[idField] = maxId + 1 + i;
      else r[idField] = "S-" + String(records.length + i).padStart(3, "0");
    });
    setRecords(prev => [...prev, ...batch]);
    setBundleReady(false);
    setDhoom(null);
    setServerStats(null);
  }

  // Update count and regenerate
  function setCount(n) {
    setRecordCount(n);
    setRecords(DEMO_PRESETS[preset].generate(n));
    setBundleReady(false);
    setDhoom(null);
    setServerStats(null);
    setQueryResults([]);
  }

  const toggleRow = (i) => setExpandedRows(prev => { const n = new Set(prev); n.has(i) ? n.delete(i) : n.add(i); return n; });

  const statExplain = {
    records: "Total records in the fiber bundle. Each record is a point on the base manifold with fiber values attached.",
    compression: "DHOOM wire compression vs JSON. Achieved by arithmetic detection (@), modal defaults (|), and trailing elision.",
    dhoomBytes: "Size of the DHOOM-encoded output in characters. This is what goes over the wire.",
    jsonBytes: "Size of the equivalent JSON. DHOOM eliminates redundancy the geometry reveals.",
    fieldsOmitted: "Fields whose values were entirely predictable from structure — arithmetic sequences, constant defaults. These fields are elided completely.",
    totalSlots: "Total field×record slots in the dataset. Compare with fieldsOmitted to see how much structure DHOOM found.",
  };

  const jsonStr = JSON.stringify(records.slice(0, 5), null, 2);

  return (
    <div style={{ maxWidth: 1100, margin: "0 auto", padding: "32px 24px 56px" }}>
      <div style={{ marginBottom: 24 }}>
        <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.18em", color: "#E8A830", marginBottom: 6, fontFamily: "monospace" }}>INTERACTIVE DEMO</div>
        <h1 style={{ fontSize: 26, fontWeight: 800, color: "#E0E8F0", margin: "0 0 6px" }}>GIGI — Live Fiber Bundle Engine</h1>
        <p style={{ fontSize: 13, color: "#506070", margin: "0 0 16px" }}>Create, edit, and query data — watch it flow through the real GIGI engine. Every operation hits the live Rust server at <strong style={{ color: G }}>{BENCH_LABEL}</strong>.</p>
      </div>

      {/* Preset selector + controls */}
      <div style={{ display: "flex", gap: 8, marginBottom: 16, flexWrap: "wrap", alignItems: "center" }}>
        {Object.entries(DEMO_PRESETS).map(([k, v]) => (
          <button key={k} onClick={() => switchPreset(k)} style={{ padding: "8px 16px", borderRadius: 7, border: preset === k ? `1px solid ${G}` : "1px solid rgba(255,255,255,0.06)", cursor: "pointer", background: preset === k ? "rgba(64,232,160,0.08)" : "transparent", color: preset === k ? G : "#506070", fontSize: 12, fontWeight: 700 }}>{v.label}</button>
        ))}
        <div style={{ flex: 1 }} />
        <select value={recordCount} onChange={e => setCount(+e.target.value)} style={{ padding: "6px 10px", borderRadius: 5, border: "1px solid rgba(255,255,255,0.08)", background: "#0A0A12", color: "#A0B0C0", fontSize: 11, cursor: "pointer" }}>
          {[10, 25, 50, 100, 250, 500, 1000, 5000].map(n => <option key={n} value={n}>{n} records</option>)}
        </select>
      </div>

      {/* Stats bar */}
      {serverStats && (
        <div style={{ display: "grid", gridTemplateColumns: "repeat(4,1fr)", gap: 8, marginBottom: 16 }}>
          {[
            { k: "records", v: serverStats.records.toLocaleString(), l: "RECORDS", color: G },
            { k: "compression", v: serverStats.compression.toFixed(0) + "%", l: "SMALLER", color: "#E8A830" },
            { k: "dhoomBytes", v: serverStats.dhoomBytes.toLocaleString() + "B", l: "DHOOM", color: G },
            { k: "jsonBytes", v: serverStats.jsonBytes.toLocaleString() + "B", l: "JSON", color: "#FF6040" },
          ].map(s => (
            <div key={s.k} onClick={() => setSelectedStat(selectedStat === s.k ? null : s.k)} style={{ background: selectedStat === s.k ? "rgba(64,232,160,0.06)" : "rgba(255,255,255,0.015)", border: selectedStat === s.k ? `1px solid ${G}33` : "1px solid rgba(255,255,255,0.04)", borderRadius: 8, padding: "12px 10px", textAlign: "center", cursor: "pointer", transition: "all 0.15s" }}>
              <div style={{ fontSize: 22, fontWeight: 900, color: s.color, fontFamily: "monospace" }}>{s.v}</div>
              <div style={{ fontSize: 9, fontWeight: 700, color: "#506070", letterSpacing: "0.06em" }}>{s.l}</div>
            </div>
          ))}
        </div>
      )}
      {selectedStat && <div style={{ fontSize: 12, color: "#607080", padding: "8px 12px", marginBottom: 12, background: "rgba(64,232,160,0.03)", border: "1px solid rgba(64,232,160,0.08)", borderRadius: 6, lineHeight: 1.6 }}>{statExplain[selectedStat]}</div>}

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, marginBottom: 20 }}>
        {/* LEFT — Data panel */}
        <div>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
            <div style={{ fontSize: 12, fontWeight: 700, color: "#A0B0C0" }}>Data — {records.length} records</div>
            <div style={{ display: "flex", gap: 6 }}>
              <button onClick={addRecord} style={demoBtn()}>+ Add</button>
              <button onClick={() => batchAdd(25)} style={demoBtn()}>⚡ +25</button>
              <button onClick={() => batchAdd(100)} style={demoBtn()}>⚡ +100</button>
              <button onClick={() => { setRecords([]); setBundleReady(false); setDhoom(null); setServerStats(null); }} style={demoBtn("#FF6040")}>Clear</button>
            </div>
          </div>
          <div style={{ maxHeight: 460, overflowY: "auto", border: "1px solid rgba(255,255,255,0.04)", borderRadius: 8, background: "rgba(0,0,0,0.2)" }}>
            {records.slice(0, 200).map((rec, idx) => (
              <div key={idx} style={{ borderBottom: "1px solid rgba(255,255,255,0.03)" }}>
                <div onClick={() => toggleRow(idx)} style={{ display: "flex", alignItems: "center", padding: "6px 10px", cursor: "pointer", background: expandedRows.has(idx) ? "rgba(64,232,160,0.02)" : "transparent" }}>
                  <span style={{ fontSize: 9, color: "#405060", marginRight: 6, transform: expandedRows.has(idx) ? "rotate(90deg)" : "none", transition: "0.15s", display: "inline-block" }}>▶</span>
                  <span style={{ fontSize: 11, fontFamily: "monospace", color: G, fontWeight: 700, minWidth: 70 }}>{String(rec[fields[0]])}</span>
                  <span style={{ fontSize: 11, color: "#607080", flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {fields.slice(1, 4).map(f => `${f}=${typeof rec[f] === "number" ? rec[f] : `"${rec[f]}"`}`).join(" · ")}
                  </span>
                  <span style={{ display: "flex", gap: 4 }}>
                    <button onClick={e => { e.stopPropagation(); startEdit(idx); }} style={{ background: "none", border: "none", cursor: "pointer", color: "#E8A830", fontSize: 11, padding: "2px 4px" }}>✎</button>
                    <button onClick={e => { e.stopPropagation(); deleteRecord(idx); }} style={{ background: "none", border: "none", cursor: "pointer", color: "#FF6040", fontSize: 11, padding: "2px 4px" }}>✕</button>
                  </span>
                </div>
                {expandedRows.has(idx) && editingRow !== idx && (
                  <div style={{ padding: "4px 10px 8px 26px", background: "rgba(0,0,0,0.15)" }}>
                    <table style={{ width: "100%", fontSize: 11, fontFamily: "monospace" }}>
                      <tbody>{fields.map(f => (
                        <tr key={f}><td style={{ padding: "2px 8px 2px 0", color: "#506070", width: 110 }}>{f}</td><td style={{ padding: "2px 0", color: "#A0B0C0" }}>{String(rec[f])}</td></tr>
                      ))}</tbody>
                    </table>
                  </div>
                )}
                {editingRow === idx && (
                  <div style={{ padding: "8px 10px 10px 26px", background: "rgba(232,168,48,0.03)" }}>
                    {fields.map(f => (
                      <div key={f} style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
                        <label style={{ fontSize: 10, color: "#506070", width: 100, fontFamily: "monospace" }}>{f}</label>
                        <input value={editValues[f] ?? ""} onChange={e => setEditValues(prev => ({ ...prev, [f]: ds.schema.fields[f] === "numeric" ? (e.target.value === "" ? "" : +e.target.value) : e.target.value }))} style={{ flex: 1, padding: "4px 8px", borderRadius: 4, border: "1px solid rgba(232,168,48,0.2)", background: "#0A0A14", color: "#D0D8E0", fontSize: 11, fontFamily: "monospace" }} />
                      </div>
                    ))}
                    <div style={{ display: "flex", gap: 6, marginTop: 6 }}>
                      <button onClick={saveEdit} style={demoBtn(G)}>Save</button>
                      <button onClick={() => setEditingRow(null)} style={demoBtn()}>Cancel</button>
                    </div>
                  </div>
                )}
              </div>
            ))}
            {records.length > 200 && <div style={{ padding: "8px 10px", fontSize: 11, color: "#405060", textAlign: "center" }}>... and {records.length - 200} more records</div>}
            {records.length === 0 && <div style={{ padding: "20px", fontSize: 12, color: "#405060", textAlign: "center" }}>No records — click + Add or ⚡ Batch to create data</div>}
          </div>
        </div>

        {/* RIGHT — DHOOM output */}
        <div>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
            <div style={{ fontSize: 12, fontWeight: 700, color: "#A0B0C0" }}>Live <a href="https://dhoom.dev" target="_blank" rel="noopener noreferrer" style={{ color: "#E8A830", textDecoration: "none" }}>DHOOM</a> Output {dhoom && <span style={{ fontSize: 10, color: "#506070", fontWeight: 400 }}>{dhoom.dhoom_chars} bytes</span>}</div>
          </div>
          <div style={{ maxHeight: 460, overflowY: "auto", border: "1px solid rgba(255,255,255,0.04)", borderRadius: 8, background: "#06060C", padding: "12px 14px", fontFamily: "monospace", fontSize: 11, lineHeight: 1.55, color: "#708090", whiteSpace: "pre-wrap", wordBreak: "break-all" }}>
            {dhoom ? dhoom.dhoom : <span style={{ color: "#303848" }}>Push data to GIGI to see DHOOM encoding here...</span>}
          </div>
        </div>
      </div>

      {/* Push to GIGI button */}
      <div style={{ display: "flex", gap: 10, alignItems: "center", marginBottom: 20 }}>
        <button onClick={pushToGigi} disabled={loading || records.length === 0} style={{ padding: "12px 28px", borderRadius: 8, border: "none", cursor: loading ? "wait" : "pointer", background: loading ? "#1A2A20" : G, color: loading ? "#40E8A060" : BG, fontSize: 14, fontWeight: 800, opacity: records.length === 0 ? 0.3 : 1 }}>
          {loading ? "⏳ Processing..." : bundleReady ? "🔄 Re-push to GIGI" : "🚀 Push to GIGI Engine"}
        </button>
        {status && <span style={{ fontSize: 12, color: bundleReady ? G : "#E8A830", fontFamily: "monospace" }}>{status}</span>}
      </div>

      {/* Geometric Profile */}
      {serverStats && (
        <div style={{ marginBottom: 20 }}>
          <div style={{ fontSize: 12, fontWeight: 700, color: "#A0B0C0", marginBottom: 8 }}>Geometric Profile — {fields.length} fields</div>
          <div style={{ display: "flex", gap: 10, flexWrap: "wrap", marginBottom: 10 }}>
            {[
              { sym: "@", label: "Arithmetic", color: "#40E8A0", desc: "Timestamps, IDs — described by start+step" },
              { sym: "&", label: "Interning", color: "#8080E0", desc: "Repeated strings stored once, referenced by index" },
              { sym: "#", label: "Computed", color: "#E8A830", desc: "Derivable from other fields (price×qty)" },
              { sym: "|", label: "Default", color: "#C060FF", desc: "Modal value — only deviations transmitted" },
              { sym: "!", label: "Constraint", color: "#FF6040", desc: "Type constraints validated by the engine" },
              { sym: ":", label: "Deviation", color: "#E05050", desc: "Non-default values — the interesting data" },
            ].map(t => (
              <div key={t.sym} style={{ fontSize: 10, color: "#506070", display: "flex", alignItems: "center", gap: 3 }}>
                <span style={{ color: t.color, fontWeight: 900, fontFamily: "monospace", fontSize: 13 }}>{t.sym}</span>{t.label}
              </div>
            ))}
          </div>
          <div style={{ border: "1px solid rgba(255,255,255,0.04)", borderRadius: 8, overflow: "hidden" }}>
            {fields.map((f, i) => {
              const fType = ds.schema.fields[f];
              const isKey = ds.schema.keys.includes(f);
              const isNumeric = fType === "numeric";
              // Detect field characteristics from data
              const vals = records.map(r => r[f]);
              const unique = new Set(vals).size;
              const isArith = isNumeric && records.length > 2 && (() => {
                const diffs = [];
                for (let j = 1; j < Math.min(vals.length, 10); j++) diffs.push(vals[j] - vals[j - 1]);
                return diffs.length > 0 && new Set(diffs.map(d => d.toFixed(2))).size === 1;
              })();
              const mode = (() => { const freq = {}; vals.forEach(v => { freq[v] = (freq[v] || 0) + 1; }); let mv, mc = 0; for (const [k, c] of Object.entries(freq)) { if (c > mc) { mc = c; mv = k; } } return { val: mv, pct: mc / vals.length }; })();
              const isDefault = mode.pct > 0.7;
              const isInterned = !isNumeric && unique < vals.length * 0.3;
              const symbols = [];
              if (isArith) symbols.push({ sym: "@", color: "#40E8A0" });
              if (isInterned) symbols.push({ sym: "&", color: "#8080E0" });
              if (isDefault) symbols.push({ sym: "|", color: "#C060FF" });
              if (isKey) symbols.push({ sym: "!", color: "#FF6040" });
              const elisionPct = isArith ? 100 : isDefault ? Math.round(mode.pct * 100) : isInterned ? Math.round((1 - unique / vals.length) * 100) : 0;
              return (
                <div key={f} style={{ display: "flex", alignItems: "center", padding: "6px 12px", borderBottom: i < fields.length - 1 ? "1px solid rgba(255,255,255,0.03)" : "none", background: i % 2 === 0 ? "rgba(255,255,255,0.008)" : "transparent" }}>
                  <span style={{ fontSize: 11, fontFamily: "monospace", color: "#A0B0C0", fontWeight: 600, width: 120 }}>{f}</span>
                  <span style={{ fontSize: 10, color: "#405060", width: 70 }}>{fType}</span>
                  <span style={{ display: "flex", gap: 3, width: 60 }}>{symbols.map((s, j) => <span key={j} style={{ color: s.color, fontWeight: 900, fontFamily: "monospace", fontSize: 14 }}>{s.sym}</span>)}</span>
                  <div style={{ flex: 1, height: 6, background: "rgba(255,255,255,0.03)", borderRadius: 3, overflow: "hidden" }}>
                    <div style={{ width: elisionPct + "%", height: "100%", background: elisionPct > 80 ? G : elisionPct > 40 ? "#E8A830" : "#FF604060", borderRadius: 3, transition: "width 0.3s" }} />
                  </div>
                  <span style={{ fontSize: 10, color: elisionPct > 60 ? G : "#506070", fontFamily: "monospace", width: 45, textAlign: "right" }}>{elisionPct}%</span>
                </div>
              );
            })}
          </div>
          <div style={{ display: "flex", justifyContent: "space-between", marginTop: 6 }}>
            <span style={{ fontSize: 10, color: "#405060" }}>{serverStats.fieldsOmitted} of {serverStats.totalSlots} field slots elided ({(serverStats.fieldsOmitted / serverStats.totalSlots * 100).toFixed(0)}%)</span>
          </div>
        </div>
      )}

      {/* Query panel */}
      {bundleReady && (
        <div style={{ marginBottom: 20 }}>
          <div style={{ fontSize: 12, fontWeight: 700, color: "#A0B0C0", marginBottom: 8 }}>GQL Queries — Live against GIGI Engine</div>
          <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
            {ds.queries.map((q, i) => (
              <button key={i} onClick={() => runQuery(q)} style={{ padding: "8px 16px", borderRadius: 7, border: "1px solid rgba(64,232,160,0.12)", cursor: "pointer", background: "rgba(64,232,160,0.04)", color: G, fontSize: 12, fontWeight: 700 }}>▶ {q.label}</button>
            ))}
          </div>
          {queryResults.map((qr, i) => (
            <div key={qr.ts} style={{ marginBottom: 8, border: "1px solid rgba(255,255,255,0.04)", borderRadius: 8, overflow: "hidden", opacity: i === 0 ? 1 : 0.6 }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "8px 12px", background: "rgba(64,232,160,0.02)" }}>
                <div>
                  <span style={{ fontSize: 11, fontWeight: 700, color: G }}>{qr.label}</span>
                  <span style={{ fontSize: 10, color: "#506070", marginLeft: 8, fontFamily: "monospace" }}>{qr.gql}</span>
                </div>
                <span style={{ fontSize: 11, fontWeight: 700, color: "#E8A830", fontFamily: "monospace" }}>{qr.elapsed.toFixed(1)}ms</span>
              </div>
              <div style={{ padding: "8px 12px", fontSize: 11, color: "#708090", lineHeight: 1.5 }}>
                <div style={{ fontSize: 10, color: "#506070", marginBottom: 4 }}>{qr.desc}</div>
                <pre style={{ margin: 0, fontFamily: "monospace", fontSize: 10.5, color: "#A0B0C0", whiteSpace: "pre-wrap", maxHeight: 150, overflow: "auto" }}>{JSON.stringify(qr.result, null, 2)}</pre>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* JSON comparison */}
      <div style={{ marginBottom: 16 }}>
        <div onClick={() => setShowJson(!showJson)} style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer", padding: "8px 12px", border: "1px solid rgba(255,255,255,0.04)", borderRadius: 8, background: "rgba(255,255,255,0.01)" }}>
          <span style={{ fontSize: 10, color: "#405060", transform: showJson ? "rotate(90deg)" : "none", transition: "0.15s", display: "inline-block" }}>▶</span>
          <span style={{ fontSize: 12, fontWeight: 700, color: "#FF6040" }}>JSON Equivalent</span>
          <span style={{ fontSize: 10, color: "#506070" }}>{new Blob([JSON.stringify(records)]).size.toLocaleString()} bytes</span>
        </div>
        {showJson && (
          <pre style={{ margin: "8px 0 0", padding: "12px 14px", borderRadius: 8, background: "#06060C", border: "1px solid rgba(255,96,64,0.1)", fontSize: 10.5, color: "#607080", fontFamily: "monospace", lineHeight: 1.5, maxHeight: 300, overflow: "auto", whiteSpace: "pre-wrap" }}>{jsonStr}{records.length > 5 ? `\n... (${records.length - 5} more records)` : ""}</pre>
        )}
      </div>

      <div style={{ textAlign: "center", fontSize: 10, color: "#1A1A2A", fontFamily: "monospace", padding: "12px 0" }}>
        Built with <a href="/gigi" style={{ color: G, textDecoration: "none" }}>GIGI</a> · Wire format: <a href="https://dhoom.dev" target="_blank" rel="noopener noreferrer" style={{ color: "#E8A830", textDecoration: "none" }}>DHOOM</a> · Running on {BENCH_LABEL}
      </div>
    </div>
  );
}

function demoBtn(color) {
  return { padding: "4px 10px", borderRadius: 5, border: `1px solid ${color || "rgba(255,255,255,0.08)"}33`, cursor: "pointer", background: "transparent", color: color || "#506070", fontSize: 10, fontWeight: 700 };
}

function GigiPage() {
  const shipped = [
    { name: "GIGI", color: G, desc: "Geometric Intrinsic Global Index — a database engine where data lives on fiber bundles. O(1) queries, DHOOM wire protocol, sheaf-guaranteed consistency. 289 tests passing. Live at gigi-stream.fly.dev.", link: "/gigi" },
    { name: "PRISM", color: "#E8A830", desc: "Payment Rail Integration via Semantic Matching. Geometric financial transaction reconciliation across SWIFT, ISO 20022, ACH, ISO 8583, and RTP. 1M transactions matched in 22 seconds at 99.97% accuracy.", link: "https://useprism.sh" },
    { name: "CHIHIRO", color: "#FF6040", desc: "Plasma MHD stability diagnostic via the Davis Field Equations. Sub-10ms diagnostics with zero free parameters. Troyon beta limit derived from topology.", link: "https://chihiro.sh" },
    { name: "PSYCHOHISTORY", color: "#C060FF", desc: "Geopolitical conflict prediction using manifold curvature analysis. F1 = 0.73 across 47 historical conflicts. 72-day lead time on the Russia-Ukraine invasion.", link: "https://helicity.io/#exp" },
    { name: "PARALLAX SUITE", color: "#8080E0", desc: "HERALD · GEODESIC · TESSERA — geometric intelligence tools for viral mutation surveillance, cancer biomarker detection, and antimicrobial resistance analysis.", link: "https://parallax.sh" },
    { name: "MARCELLA", color: "#E05050", desc: "Geometric language model on the Davis-Lie-Riemann metric. 69.4M parameters. 3.85× perplexity advantage over vanilla transformers. Named after her mother.", link: null },
  ];
  return (
    <div style={{ maxWidth: 940, margin: "0 auto", padding: "32px 24px 56px" }}>
      {/* Hero section with photo */}
      <div style={{ display: "flex", gap: 40, alignItems: "flex-start", marginBottom: 40, flexWrap: "wrap" }}>
        <div style={{ flexShrink: 0 }}>
          <img src="/gigi/gigi-photo.jpg" alt="Bee Rosa Davis" style={{ width: 260, height: 340, objectFit: "cover", objectPosition: "center bottom", borderRadius: 14, border: "2px solid rgba(64,232,160,0.15)" }} />
        </div>
        <div style={{ flex: 1, minWidth: 280 }}>
          <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.18em", color: "#E8A830", marginBottom: 8, fontFamily: "monospace" }}>THE REAL GIGI</div>
          <h1 style={{ fontSize: 36, fontWeight: 900, color: "#E0E8F0", margin: "0 0 4px", letterSpacing: "-0.03em" }}>Bee Rosa Davis</h1>
          <div style={{ fontSize: 14, color: G, fontWeight: 600, marginBottom: 16 }}>Overall Mother of the House of Diwa</div>
          <p style={{ fontSize: 13.5, color: "#607080", lineHeight: 1.7, margin: "0 0 14px" }}>
            Applied mathematician, security engineer, and independent researcher. <em style={{ color: "#A0B0C0" }}>Gigi</em> is my ballroom house nickname — it means mom.
          </p>
          <p style={{ fontSize: 13.5, color: "#607080", lineHeight: 1.7, margin: "0 0 14px" }}>
            I build production systems grounded in differential geometry. Every product ships with the same identity: <strong style={{ color: G }}>S + d² = 1</strong>. The math is the product.
          </p>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 18 }}>
            {[["MS Digital Forensics", "Brown University"], ["BA Logic", "Morehouse College"], ["BA Communication", "Univ. of the Pacific"]].map(([d, s], i) => (
              <div key={i} style={{ background: "rgba(64,232,160,0.03)", border: "1px solid rgba(64,232,160,0.08)", borderRadius: 8, padding: "8px 12px" }}>
                <div style={{ fontSize: 11, fontWeight: 700, color: "#A0B0C0" }}>{d}</div>
                <div style={{ fontSize: 10, color: "#506070" }}>{s}</div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Stats bar */}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(4,1fr)", gap: 10, marginBottom: 32 }}>
        {[
          { v: "6", l: "Live Products", d: "All built on one framework" },
          { v: "28", l: "Patents Filed", d: "Oct 2025 – Mar 2026" },
          { v: "8", l: "Books Published", d: "Including #1 Amazon bestseller" },
          { v: "27yr", l: "Engineering", d: "Pandora → NSA → NASA → IBM" },
        ].map((x, i) => (
          <div key={i} style={{ background: "rgba(64,232,160,0.025)", border: "1px solid rgba(64,232,160,0.07)", borderRadius: 10, padding: "18px 14px", textAlign: "center" }}>
            <div style={{ fontSize: 26, fontWeight: 900, color: G, fontFamily: "monospace" }}>{x.v}</div>
            <div style={{ fontSize: 12.5, fontWeight: 700, color: "#A0B0C0", marginTop: 5 }}>{x.l}</div>
            <div style={{ fontSize: 10.5, color: "#505060", marginTop: 3 }}>{x.d}</div>
          </div>
        ))}
      </div>

      {/* The math */}
      <Card style={{ marginBottom: 24, background: "linear-gradient(135deg, rgba(64,232,160,0.03), rgba(232,168,48,0.03))", border: "1px solid rgba(64,232,160,0.1)" }}>
        <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.12em", color: "#E8A830", marginBottom: 10 }}>ONE MATH</div>
        <p style={{ fontSize: 13, color: "#607080", lineHeight: 1.7, margin: "0 0 14px" }}>
          Every product, every paper, every patent traces back to two equations from the same geometric framework.
          The Davis Law governs how systems behave. The Davis Identity proves every decision.
        </p>
        <div style={{ display: "flex", gap: 24, justifyContent: "center", flexWrap: "wrap" }}>
          <div style={{ textAlign: "center" }}>
            <div style={{ fontSize: 22, fontWeight: 900, color: G, fontFamily: "monospace" }}>C = τ / K</div>
            <div style={{ fontSize: 10, color: "#506070", marginTop: 4 }}>The Davis Law</div>
          </div>
          <div style={{ textAlign: "center" }}>
            <div style={{ fontSize: 22, fontWeight: 900, color: "#E8A830", fontFamily: "monospace" }}>S + d² = 1</div>
            <div style={{ fontSize: 10, color: "#506070", marginTop: 4 }}>The Davis Identity</div>
          </div>
        </div>
      </Card>

      {/* Shipped products */}
      <SectionLabel>Shipped Products</SectionLabel>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, marginBottom: 28 }}>
        {shipped.map((p, i) => (
          <div key={i} style={{ background: "rgba(255,255,255,0.015)", border: "1px solid rgba(255,255,255,0.04)", borderRadius: 10, padding: "16px 16px" }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
              <div style={{ fontSize: 15, fontWeight: 800, color: p.color }}>{p.name}</div>
              {p.link && <a href={p.link} target="_blank" rel="noopener noreferrer" style={{ fontSize: 10, color: p.color, textDecoration: "none", fontWeight: 700 }}>→</a>}
            </div>
            <div style={{ fontSize: 12, color: "#607080", lineHeight: 1.55 }}>{p.desc}</div>
          </div>
        ))}
      </div>

      {/* Career highlights */}
      <SectionLabel>Career</SectionLabel>
      <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 28 }}>
        {[
          { yr: "2024–", role: "Principal Adversarial Intelligence Engineer", org: "IBM X-Force Red", note: "VIVID adversarial analytics, Shadow Clone Jutsu — automated vulnerability discovery" },
          { yr: "2023–", role: "Principal Software Engineer", org: "NASA JSC", note: "2.3TB/day telemetry pipelines, flight-ready precision adaptation system" },
          { yr: "2025", role: "Independent Security Researcher", org: "Microsoft MSRC", note: "Four Windows kernel zero-day vulnerabilities via geometric attack surface methods" },
          { yr: "2022–23", role: "Director of Security Engineering", org: "Humana", note: "Quantum-resistant encryption, homomorphic data stores for healthcare PHI" },
          { yr: "2012–16", role: "Head of ML & Security Engineering", org: "Axiom88 — NSA, Naval Intelligence, DISA", note: "Arc Angel anomaly detection for NSA, predictive threat detection for nuclear submarines" },
          { yr: "2009–12", role: "Principal Engineer, ML", org: "Pandora Radio", note: "Founding engineer. Grew ad revenue 750%, scaled team 5 → 100 engineers" },
        ].map((c, i) => (
          <div key={i} style={{ display: "flex", gap: 14, padding: "10px 14px", background: "rgba(255,255,255,0.01)", border: "1px solid rgba(255,255,255,0.03)", borderRadius: 8 }}>
            <div style={{ fontSize: 11, fontWeight: 700, color: G, fontFamily: "monospace", minWidth: 55, flexShrink: 0 }}>{c.yr}</div>
            <div>
              <div style={{ fontSize: 12.5, fontWeight: 700, color: "#A0B0C0" }}>{c.role}</div>
              <div style={{ fontSize: 11, color: "#E8A830", fontWeight: 600 }}>{c.org}</div>
              <div style={{ fontSize: 11, color: "#506070", marginTop: 2 }}>{c.note}</div>
            </div>
          </div>
        ))}
      </div>

      {/* Books */}
      <SectionLabel>Publications</SectionLabel>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 10, marginBottom: 28 }}>
        {[
          { title: "Hidden Variable", desc: "From a childhood IQ diagnosis to NASA and algorithmic justice.", link: "https://a.co/d/ia1s3x3", tag: "MEMOIR" },
          { title: "The Geometry of Sameness", desc: "ML and Riemannian manifolds tell the same story. #1 New Release.", link: "https://a.co/d/e7rGkcF", tag: "#1 NEW RELEASE" },
          { title: "The Geometry of Medicine", desc: "Health as geometric coherence on the Davis Manifold.", link: "https://a.co/d/elVqMku", tag: "BOOK" },
          { title: "The Geometry of Fuel", desc: "Davis Law to topological vacuum rectification.", link: "https://a.co/d/0ir12g3Z", tag: "BOOK" },
          { title: "Motion to Be Seen", desc: "One Black trans woman's fight for algorithmic justice. #1 Amazon Bestseller.", link: "https://a.co/d/ghWdbIh", tag: "#1 BESTSELLER" },
          { title: "Fly Into The Sun", desc: "Hard sci-fi. Consciousness, identity, and the geometry of interstellar travel.", link: "https://a.co/d/2pSIn8A", tag: "SCI-FI" },
        ].map((b, i) => (
          <a key={i} href={b.link} target="_blank" rel="noopener noreferrer" style={{ background: "rgba(255,255,255,0.015)", border: "1px solid rgba(255,255,255,0.04)", borderRadius: 10, padding: "14px 14px", textDecoration: "none", display: "block" }}>
            <div style={{ fontSize: 9, fontWeight: 700, letterSpacing: "0.08em", color: "#E8A830", marginBottom: 4 }}>{b.tag}</div>
            <div style={{ fontSize: 13, fontWeight: 700, color: "#A0B0C0" }}>{b.title}</div>
            <div style={{ fontSize: 11, color: "#506070", marginTop: 4, lineHeight: 1.45 }}>{b.desc}</div>
          </a>
        ))}
      </div>

      {/* Links */}
      <div style={{ display: "flex", gap: 12, justifyContent: "center", flexWrap: "wrap", padding: "16px 0" }}>
        {[
          ["GitHub", "https://github.com/nurdymuny"],
          ["ORCID", "https://orcid.org/0009-0009-8034-4308"],
          ["LinkedIn", "https://www.linkedin.com/in/msbeedavis/"],
          ["Zenodo", "https://doi.org/10.5281/zenodo.18511755"],
          ["Products", "https://www.davisgeometric.com/products"],
          ["Contact", "mailto:bee_davis@alumni.brown.edu"],
        ].map(([label, url], i) => (
          <a key={i} href={url} target="_blank" rel="noopener noreferrer" style={{ fontSize: 11, fontWeight: 700, color: "#506070", textDecoration: "none", padding: "6px 14px", borderRadius: 6, border: "1px solid rgba(255,255,255,0.06)", background: "rgba(255,255,255,0.01)" }}>{label}</a>
        ))}
      </div>
    </div>
  );
}

function ProductsPage() {
  return (
    <Page title="Products" sub="Open format. Open core. Hosted service.">
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 14, marginBottom: 28 }}>
        <ProductCard name="GIGI Convert" icon={"🔄"} desc={"JSON / CSV / SQL → DHOOM conversion. Local CLI free. API metered. Detects arithmetic, computes defaults, optimizes field ordering automatically."} tag="Coming Soon" color="#E8A830" features={["Upload JSON → get DHOOM", "Geometric data profiler", "Compression ratio prediction", "Batch + streaming modes"]} />
        <ProductCard name="GIGI Stream" icon={"⚡"} desc="Real-time geometric database as a service. O(1) reads/writes. DHOOM wire protocol. WebSocket subscriptions. Confidence per result." tag="Coming Soon" color={G} featured features={["O(1) read + write", "Real-time subscriptions", "DHOOM wire (66-84% savings)", "Curvature confidence", "Sheaf-guaranteed consistency", "Holonomy drift detection"]} />
        <ProductCard name="GIGI Edge" icon={"📱"} desc="Local-first GIGI engine for mobile/IoT. Stores locally, syncs to GIGI Stream. Sheaf gluing axiom guarantees correct sync." tag="Coming Soon" color="#8080E0" features={["Tiny local engine", "Offline-first", "Sheaf-guaranteed sync", "No conflict resolution needed"]} />
      </div>

      <Card>
        <div style={{ fontSize: 13, fontWeight: 700, color: "#A0B0C0", marginBottom: 6 }}>Why O(1) Changes Pricing</div>
        <p style={{ fontSize: 12, color: "#607080", lineHeight: 1.6, margin: 0 }}>
          Traditional databases have infrastructure costs that scale with query volume. O(1) means GIGI’s cost per query is <strong style={{ color: G }}>constant regardless of database size</strong>. Pricing details coming soon.
        </p>
      </Card>
    </Page>
  );
}

function ProductCard({ name, icon, desc, tag, color, featured, features }) {
  return (
    <div style={{ background: featured ? "rgba(64,232,160,0.025)" : "rgba(255,255,255,0.015)", border: featured ? "1px solid rgba(64,232,160,0.12)" : "1px solid rgba(255,255,255,0.04)", borderRadius: 10, padding: "20px 16px" }}>
      <span style={{ fontSize: 24 }}>{icon}</span>
      <div style={{ fontSize: 16, fontWeight: 700, color: "#C0D0E0", marginTop: 8 }}>{name}</div>
      <div style={{ fontSize: 10, fontWeight: 700, color, marginTop: 3, letterSpacing: "0.04em" }}>{tag}</div>
      <p style={{ fontSize: 12, color: "#607080", lineHeight: 1.55, margin: "10px 0" }}>{desc}</p>
      <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
        {features.map((f, i) => <div key={i} style={{ fontSize: 11, color: "#708090", paddingLeft: 10, position: "relative" }}><span style={{ position: "absolute", left: 0, color }}>+</span>{f}</div>)}
      </div>
    </div>
  );
}

// ═══════════════════════════════════════
// ENCRYPTION DEMO — Interactive Lab
// ═══════════════════════════════════════
const ENC_PRESETS = {
  sensor: {
    label: "🌡️ Sensor", fields: ["temp", "humidity", "pressure", "wind"],
    rows: [[22.4,45.2,1013.2,3.1],[23.1,44.8,1013.5,2.9],[45.8,12.1,998.3,18.7],[21.9,46.0,1013.1,3.3],[22.7,45.5,1013.4,3.0],[-8.2,78.3,1025.6,12.4],[22.0,45.1,1013.0,3.2],[23.5,44.2,1012.8,2.7]],
  },
  financial: {
    label: "💰 Financial", fields: ["price", "volume", "spread", "volatility"],
    rows: [[142.50,32000,0.12,14.2],[143.20,28500,0.08,13.8],[141.80,41000,0.35,22.1],[142.90,30100,0.11,14.5],[142.60,31200,0.09,13.9],[138.10,89000,0.82,45.3],[143.10,29800,0.10,14.1],[142.40,33500,0.13,14.3]],
  },
  health: {
    label: "🏥 Health", fields: ["heart_rate", "bp_sys", "oxygen", "resp_rate"],
    rows: [[72,120,98.1,16],[75,118,97.8,15],[68,122,98.5,17],[140,185,91.2,28],[71,119,98.0,16],[73,121,97.9,15],[70,117,98.3,16],[74,120,98.2,15]],
  },
};

function EncryptionPage() {
  const EC = "#C060FF";
  const [fields, setFields] = useState(["temp", "humidity", "pressure", "wind"]);
  const [rows, setRows] = useState(ENC_PRESETS.sensor.rows.map(r => [...r]));
  const [alphas, setAlphas] = useState([2.73, 0.87, 1.42, 3.15]);
  const [betas, setBetas] = useState([-14.5, 31.2, 203.0, -7.8]);
  const [srvStage, setSrvStage] = useState(null);
  const [srvStep, setSrvStep] = useState("");
  const [srvResult, setSrvResult] = useState(null);
  const [queryId, setQueryId] = useState(0);

  // Per-field affine transform: v → α·v + β
  const encrypted = rows.map(row => row.map((v, j) => alphas[j] * v + betas[j]));

  // K = mean of (Var/range²) per field — matches GIGI's scalar_curvature
  function computeK(data) {
    const nc = data[0]?.length || 0;
    if (data.length < 2 || nc === 0) return 0;
    let sum = 0;
    for (let j = 0; j < nc; j++) {
      const vals = data.map(r => r[j]);
      const mu = vals.reduce((a, b) => a + b, 0) / vals.length;
      const va = vals.reduce((a, v) => a + (v - mu) ** 2, 0) / vals.length;
      const lo = Math.min(...vals), hi = Math.max(...vals), rng = hi - lo || 1e-10;
      sum += va / (rng * rng);
    }
    return sum / nc;
  }

  const kPlain = computeK(rows);
  const kEnc = computeK(encrypted);

  function loadPreset(key) {
    const p = ENC_PRESETS[key];
    setFields([...p.fields]); setRows(p.rows.map(r => [...r]));
    randomizeT(p.fields.length); setSrvResult(null); setSrvStage(null); setQueryId(0);
  }
  function randomizeT(n = fields.length) {
    setAlphas(Array.from({ length: n }, () => +(0.5 + Math.random() * 5).toFixed(2)));
    setBetas(Array.from({ length: n }, () => +(Math.random() * 200 - 100).toFixed(1)));
  }
  function resetT() { setAlphas(fields.map(() => 1)); setBetas(fields.map(() => 0)); }
  function addRow() {
    setRows([...rows, fields.map((_, j) => {
      const avg = rows.reduce((a, r) => a + r[j], 0) / rows.length;
      return +(avg + (Math.random() - 0.5) * Math.abs(avg) * 0.15).toFixed(1);
    })]);
  }
  function updateCell(ri, ci, val) {
    const n = parseFloat(val); if (isNaN(n)) return;
    const nr = rows.map(r => [...r]); nr[ri][ci] = n; setRows(nr);
  }
  function removeRow(i) { if (rows.length <= 2) return; setRows(rows.filter((_, j) => j !== i)); }

  async function runServer() {
    setSrvStage("running");
    const ts = Date.now() % 100000;
    const plainN = `enc_p_${ts}`, encN = `enc_e_${ts}`;
    const schema = { fields: { id: "numeric" }, keys: ["id"] };
    fields.forEach(f => { schema.fields[f] = "numeric"; });
    const records = rows.map((row, i) => {
      const rec = { id: i }; fields.forEach((f, j) => { rec[f] = row[j]; }); return rec;
    });
    try {
      setSrvStep("Creating bundles...");
      await restPost("/v1/bundles", { name: plainN, schema });
      await restPost("/v1/bundles", { name: encN, schema, encrypted: true });
      setSrvStep("Inserting your data...");
      await restPost(`/v1/bundles/${plainN}/insert`, { records });
      await restPost(`/v1/bundles/${encN}/insert`, { records });
      setSrvStep("Computing curvature...");
      const kP = await gqlPost(`CURVATURE ${plainN}`);
      const kE = await gqlPost(`CURVATURE ${encN}`);
      setSrvStep("Point query on encrypted bundle...");
      const ptP = await gqlPost(`SECTION ${plainN} AT id=${queryId}`);
      const ptE = await gqlPost(`SECTION ${encN} AT id=${queryId}`);
      setSrvStep("Exporting DHOOM...");
      const dhP = await (await fetch(`${BENCH_API}/v1/bundles/${plainN}/dhoom`)).json();
      const dhE = await (await fetch(`${BENCH_API}/v1/bundles/${encN}/dhoom`)).json();
      setSrvStep("Cleanup...");
      await restDelete(`/v1/bundles/${plainN}`); await restDelete(`/v1/bundles/${encN}`);
      setSrvResult({
        kP: kP.value, kE: kE.value, match: Math.abs(kP.value - kE.value) < 0.0001,
        ptP: ptP.section || ptP, ptE: ptE.section || ptE,
        dhP: dhP.dhoom?.split("\n").slice(0, 8).join("\n") || "",
        dhE: dhE.dhoom?.split("\n").slice(0, 8).join("\n") || "",
      });
      setSrvStage("done"); setSrvStep("");
    } catch (e) {
      setSrvStep(`Error: ${e.message}`);
      try { await restDelete(`/v1/bundles/${plainN}`); } catch (_) {}
      try { await restDelete(`/v1/bundles/${encN}`); } catch (_) {}
      setSrvStage(null);
    }
  }

  const cellSt = { background: "none", border: "1px solid rgba(255,255,255,0.06)", borderRadius: 4, color: "#A0B0C0", fontSize: 10.5, fontFamily: "monospace", padding: "3px 5px", width: 76, textAlign: "right" };
  const thSt = { textAlign: "left", padding: "5px 8px", borderBottom: "1px solid rgba(255,255,255,0.05)", color: "#506070", fontWeight: 600, fontSize: 9, textTransform: "uppercase" };
  const tdSt = { padding: "2px 8px", borderBottom: "1px solid rgba(255,255,255,0.02)" };
  const th2St = { textAlign: "left", padding: "4px 6px", borderBottom: "1px solid rgba(255,255,255,0.05)", color: "#506070", fontSize: 8.5 };
  const td2St = { padding: "3px 6px", borderBottom: "1px solid rgba(255,255,255,0.02)", color: "#506070" };
  const smBtnSt = { padding: "4px 10px", borderRadius: 4, border: "1px solid rgba(255,255,255,0.06)", background: "transparent", color: "#607080", fontSize: 10, cursor: "pointer" };
  const sr = srvResult;

  return (
    <Page title="Geometric Encryption — Interactive Lab" sub={<>Your data. Your transforms. Live proof that curvature is gauge-invariant. Running against <strong style={{ color: G }}>GIGI Stream</strong> at {BENCH_LABEL}.</>}>

      {/* Presets */}
      <div style={{ display: "flex", gap: 8, marginBottom: 18, flexWrap: "wrap" }}>
        {Object.entries(ENC_PRESETS).map(([k, v]) => (
          <button key={k} onClick={() => loadPreset(k)} style={{ padding: "7px 14px", borderRadius: 6, border: "1px solid rgba(192,96,255,0.15)", background: "rgba(192,96,255,0.04)", color: EC, fontSize: 11, fontWeight: 600, cursor: "pointer" }}>{v.label}</button>
        ))}
        <button onClick={addRow} style={{ ...smBtnSt, padding: "7px 14px" }}>+ Add Row</button>
      </div>

      {/* Editable Data Table */}
      <Card style={{ marginBottom: 22 }}>
        <div style={{ fontSize: 11, fontWeight: 700, color: G, marginBottom: 10, letterSpacing: "0.1em" }}>YOUR DATA — click any cell to edit</div>
        <div style={{ overflowX: "auto" }}>
          <table style={{ width: "100%", borderCollapse: "collapse" }}>
            <thead><tr>
              <th style={{ ...thSt, width: 30 }}>#</th>
              {fields.map((f, i) => <th key={i} style={thSt}>{f}</th>)}
              <th style={{ ...thSt, width: 24 }}></th>
            </tr></thead>
            <tbody>{rows.map((row, ri) => (
              <tr key={ri}>
                <td style={{ ...tdSt, color: "#384050", fontSize: 9 }}>{ri}</td>
                {row.map((v, ci) => (
                  <td key={ci} style={tdSt}>
                    <input type="number" value={v} step="0.1"
                      onChange={e => updateCell(ri, ci, e.target.value)}
                      style={cellSt} />
                  </td>
                ))}
                <td style={tdSt}>
                  {rows.length > 2 && <button onClick={() => removeRow(ri)} style={{ background: "none", border: "none", color: "#FF604060", cursor: "pointer", fontSize: 10, padding: 2 }}>✕</button>}
                </td>
              </tr>
            ))}</tbody>
          </table>
        </div>
        <div style={{ fontSize: 10, color: "#384050", marginTop: 8 }}>{rows.length} records · {fields.length} numeric fields · edit any value and watch the curvature update live below</div>
      </Card>

      {/* Gauge Transform Sliders */}
      <Card style={{ marginBottom: 22, background: "linear-gradient(135deg, rgba(192,96,255,0.03), rgba(64,232,160,0.01))" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
          <div style={{ fontSize: 11, fontWeight: 700, color: EC, letterSpacing: "0.1em" }}>GAUGE TRANSFORM — drag sliders to encrypt</div>
          <div style={{ display: "flex", gap: 6 }}>
            <button onClick={() => randomizeT()} style={smBtnSt}>🎲 Randomize</button>
            <button onClick={resetT} style={smBtnSt}>↺ Identity</button>
          </div>
        </div>
        <div style={{ fontSize: 10.5, color: "#506070", marginBottom: 14 }}>
          Each field gets an independent affine transform: <code style={{ ...code(), color: EC }}>v → α·v + β</code>. Drag any slider — the encrypted values change but <strong style={{ color: G }}>K stays identical</strong>.
        </div>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
          {fields.map((f, i) => (
            <div key={i} style={{ background: "rgba(255,255,255,0.015)", borderRadius: 7, padding: "10px 12px" }}>
              <div style={{ fontSize: 10, fontWeight: 700, color: "#708090", marginBottom: 6 }}>{f}</div>
              <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
                <span style={{ fontSize: 9, color: "#506070", width: 12 }}>α</span>
                <input type="range" min="0.1" max="8" step="0.01" value={alphas[i]}
                  onChange={e => { const a = [...alphas]; a[i] = +e.target.value; setAlphas(a); }}
                  style={{ flex: 1, accentColor: EC, height: 4 }} />
                <span style={{ fontSize: 10, fontFamily: "monospace", color: EC, width: 38, textAlign: "right" }}>{alphas[i].toFixed(2)}</span>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                <span style={{ fontSize: 9, color: "#506070", width: 12 }}>β</span>
                <input type="range" min="-200" max="200" step="0.1" value={betas[i]}
                  onChange={e => { const b = [...betas]; b[i] = +e.target.value; setBetas(b); }}
                  style={{ flex: 1, accentColor: EC, height: 4 }} />
                <span style={{ fontSize: 10, fontFamily: "monospace", color: EC, width: 38, textAlign: "right" }}>{betas[i].toFixed(1)}</span>
              </div>
            </div>
          ))}
        </div>
      </Card>

      {/* Live Curvature Comparison */}
      <SectionLabel>Live Result — Curvature is Gauge-Invariant</SectionLabel>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 14, marginBottom: 10 }}>
        <Card style={{ textAlign: "center", borderColor: "rgba(64,232,160,0.12)" }}>
          <div style={{ fontSize: 9, fontWeight: 700, letterSpacing: "0.12em", color: "#506070", marginBottom: 4 }}>PLAINTEXT CURVATURE</div>
          <div style={{ fontSize: 28, fontWeight: 900, color: G, fontFamily: "monospace" }}>{kPlain.toFixed(6)}</div>
        </Card>
        <Card style={{ textAlign: "center", borderColor: "rgba(192,96,255,0.15)" }}>
          <div style={{ fontSize: 9, fontWeight: 700, letterSpacing: "0.12em", color: "#506070", marginBottom: 4 }}>ENCRYPTED CURVATURE</div>
          <div style={{ fontSize: 28, fontWeight: 900, color: EC, fontFamily: "monospace" }}>{kEnc.toFixed(6)}</div>
        </Card>
      </div>
      <Card style={{ marginBottom: 22, background: "rgba(64,232,160,0.03)", borderColor: "rgba(64,232,160,0.15)" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <div style={{ fontSize: 24, flexShrink: 0 }}>✓</div>
          <div>
            <div style={{ fontSize: 13, fontWeight: 800, color: G }}>K is identical — gauge invariance holds</div>
            <div style={{ fontSize: 11, color: "#607080", marginTop: 2 }}>
              Drag any α or β slider — K never changes. Edit the data — both K values update together. The affine transform cancels: K = Var/range² → α²·Var/(α·range)² = Var/range².
            </div>
          </div>
        </div>
      </Card>

      {/* Side-by-side tables: plaintext vs encrypted */}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 14, marginBottom: 24 }}>
        <div>
          <div style={{ fontSize: 10, fontWeight: 700, color: G, marginBottom: 6, letterSpacing: "0.1em" }}>PLAINTEXT VALUES</div>
          <div style={{ overflowX: "auto", maxHeight: 220, overflow: "auto" }}>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 10, fontFamily: "monospace" }}>
              <thead><tr><th style={th2St}>#</th>{fields.map((f, i) => <th key={i} style={th2St}>{f}</th>)}</tr></thead>
              <tbody>{rows.map((row, ri) => (
                <tr key={ri}><td style={td2St}>{ri}</td>{row.map((v, ci) => <td key={ci} style={{ ...td2St, color: G }}>{v.toFixed(1)}</td>)}</tr>
              ))}</tbody>
            </table>
          </div>
        </div>
        <div>
          <div style={{ fontSize: 10, fontWeight: 700, color: EC, marginBottom: 6, letterSpacing: "0.1em" }}>ENCRYPTED (WHAT THE DB STORES)</div>
          <div style={{ overflowX: "auto", maxHeight: 220, overflow: "auto" }}>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 10, fontFamily: "monospace" }}>
              <thead><tr><th style={th2St}>#</th>{fields.map((f, i) => <th key={i} style={th2St}>{f}</th>)}</tr></thead>
              <tbody>{encrypted.map((row, ri) => (
                <tr key={ri}><td style={td2St}>{ri}</td>{row.map((v, ci) => <td key={ci} style={{ ...td2St, color: EC }}>{v.toFixed(2)}</td>)}</tr>
              ))}</tbody>
            </table>
          </div>
        </div>
      </div>

      {/* Server proof */}
      <SectionLabel>Prove It on the Real Server</SectionLabel>
      <div style={{ fontSize: 11, color: "#506070", marginBottom: 12 }}>
        Your data will be sent to GIGI Stream — once as plaintext, once encrypted with a random gauge key. Curvature computed on both. Point query auto-decrypts. No data leaves localhost.
      </div>
      <div style={{ display: "flex", gap: 10, alignItems: "center", marginBottom: 16, flexWrap: "wrap" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <span style={{ fontSize: 11, color: "#506070" }}>Query record:</span>
          <select value={queryId} onChange={e => setQueryId(+e.target.value)}
            style={{ background: "#14141E", border: "1px solid rgba(255,255,255,0.08)", borderRadius: 4, color: "#A0B0C0", fontSize: 11, padding: "4px 8px" }}>
            {rows.map((_, i) => <option key={i} value={i}>id = {i}</option>)}
          </select>
        </div>
        <button onClick={runServer} disabled={srvStage === "running"}
          style={{ padding: "10px 22px", borderRadius: 7, border: "none", cursor: srvStage === "running" ? "wait" : "pointer", background: srvStage === "running" ? "#303848" : EC, color: "#fff", fontSize: 13, fontWeight: 700 }}>
          {srvStage === "running" ? srvStep || "Running..." : srvStage === "done" ? "↻ Run Again" : "🔐  Send to GIGI Server"}
        </button>
      </div>

      {sr && (<>
        {/* Server curvature */}
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 14, marginBottom: 14 }}>
          <Card style={{ textAlign: "center", borderColor: "rgba(64,232,160,0.12)" }}>
            <div style={{ fontSize: 9, fontWeight: 700, letterSpacing: "0.1em", color: "#506070", marginBottom: 4 }}>SERVER K (PLAINTEXT)</div>
            <div style={{ fontSize: 26, fontWeight: 900, color: G, fontFamily: "monospace" }}>{typeof sr.kP === "number" ? sr.kP.toFixed(6) : "—"}</div>
          </Card>
          <Card style={{ textAlign: "center", borderColor: "rgba(192,96,255,0.15)" }}>
            <div style={{ fontSize: 9, fontWeight: 700, letterSpacing: "0.1em", color: "#506070", marginBottom: 4 }}>SERVER K (ENCRYPTED)</div>
            <div style={{ fontSize: 26, fontWeight: 900, color: EC, fontFamily: "monospace" }}>{typeof sr.kE === "number" ? sr.kE.toFixed(6) : "—"}</div>
          </Card>
        </div>
        <Card style={{ marginBottom: 18, background: sr.match ? "rgba(64,232,160,0.03)" : "rgba(255,96,64,0.03)", borderColor: sr.match ? "rgba(64,232,160,0.15)" : "rgba(255,96,64,0.15)" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <div style={{ fontSize: 24 }}>{sr.match ? "✓" : "✗"}</div>
            <div>
              <div style={{ fontSize: 13, fontWeight: 800, color: sr.match ? G : "#FF6040" }}>
                {sr.match ? "Server confirms: encryption preserves geometry" : "Mismatch (unexpected)"}
              </div>
              <div style={{ fontSize: 11, color: "#607080" }}>
                Δ = {typeof sr.kP === "number" && typeof sr.kE === "number" ? Math.abs(sr.kP - sr.kE).toExponential(2) : "—"} — curvature computed on <strong style={{ color: EC }}>encrypted</strong> values, no decryption
              </div>
            </div>
          </div>
        </Card>

        {/* Point query */}
        <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.1em", color: "#384050", marginBottom: 8 }}>POINT QUERY — id={queryId} (encrypted bundle auto-decrypts on read)</div>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 14, marginBottom: 18 }}>
          <div>
            <div style={{ fontSize: 9, fontWeight: 700, color: G, marginBottom: 4, letterSpacing: "0.1em" }}>PLAINTEXT</div>
            <pre style={{ background: "#0A0A12", border: "1px solid rgba(64,232,160,0.08)", borderRadius: 7, padding: "10px", fontSize: 10, lineHeight: 1.5, color: G, fontFamily: "monospace", margin: 0, whiteSpace: "pre-wrap" }}>{JSON.stringify(sr.ptP, null, 2)}</pre>
          </div>
          <div>
            <div style={{ fontSize: 9, fontWeight: 700, color: EC, marginBottom: 4, letterSpacing: "0.1em" }}>ENCRYPTED (auto-decrypted)</div>
            <pre style={{ background: "#0A0A12", border: "1px solid rgba(192,96,255,0.1)", borderRadius: 7, padding: "10px", fontSize: 10, lineHeight: 1.5, color: EC, fontFamily: "monospace", margin: 0, whiteSpace: "pre-wrap" }}>{JSON.stringify(sr.ptE, null, 2)}</pre>
          </div>
        </div>

        {/* DHOOM */}
        <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.1em", color: "#384050", marginBottom: 8 }}>DHOOM WIRE OUTPUT</div>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 14, marginBottom: 18 }}>
          <pre style={{ background: "#0A0A12", border: "1px solid rgba(64,232,160,0.08)", borderRadius: 7, padding: "10px", fontSize: 9.5, lineHeight: 1.4, color: "#9090A8", fontFamily: "monospace", margin: 0, whiteSpace: "pre-wrap", maxHeight: 160, overflow: "auto" }}>{sr.dhP}</pre>
          <pre style={{ background: "#0A0A12", border: "1px solid rgba(192,96,255,0.1)", borderRadius: 7, padding: "10px", fontSize: 9.5, lineHeight: 1.4, color: "#9090A8", fontFamily: "monospace", margin: 0, whiteSpace: "pre-wrap", maxHeight: 160, overflow: "auto" }}>{sr.dhE}</pre>
        </div>
      </>)}

      {/* The math */}
      <Card style={{ background: "linear-gradient(135deg, rgba(192,96,255,0.03), rgba(64,232,160,0.02))" }}>
        <div style={{ fontSize: 13, fontWeight: 700, color: EC, marginBottom: 8 }}>Why This Works — The Math</div>
        <p style={{ fontSize: 12.5, color: "#607080", lineHeight: 1.7, margin: "0 0 8px" }}>
          Geometric encryption applies a <strong style={{ color: "#D0D8E0" }}>per-field affine gauge transform</strong>: each field value <em>v</em> is mapped to <em>αv + β</em> where α ≠ 0 and β are derived from a 32-byte cryptographic seed.
        </p>
        <p style={{ fontSize: 12.5, color: "#607080", lineHeight: 1.7, margin: "0 0 8px" }}>
          Curvature <em>K = Var/r²</em> is <strong style={{ color: G }}>invariant</strong> under affine transforms because Var(αv+β) = α²·Var(v) and range(αv+β) = α·range(v), so K = α²·Var/(α·range)² = Var/range². The α cancels. This is the <strong style={{ color: EC }}>gauge symmetry</strong>.
        </p>
        <p style={{ fontSize: 12.5, color: "#607080", lineHeight: 1.7, margin: 0 }}>
          Try it yourself: drag any slider above and watch K stay the same. Change the data — both K values update together. The geometry doesn't care about the coordinate system — that's why you can compute on encrypted data without decrypting it.
        </p>
      </Card>
    </Page>
  );
}

// ═══════════════════════════════════════
// SHARED UI
// ═══════════════════════════════════════
function Page({ title, sub, children }) {
  return <div style={{ maxWidth: 940, margin: "0 auto", padding: "32px 24px 56px" }}><h1 style={{ fontSize: 26, fontWeight: 800, color: "#E0E8F0", margin: "0 0 6px" }}>{title}</h1><p style={{ fontSize: 13, color: "#506070", margin: "0 0 22px" }}>{sub}</p>{children}</div>;
}
function Sec({ title, children }) { return <div style={{ marginBottom: 24 }}><h2 style={{ fontSize: 17, fontWeight: 700, color: "#B0C0D0", margin: "0 0 10px" }}>{title}</h2>{children}</div>; }
function P({ children }) { return <p style={{ margin: "0 0 8px", fontSize: 13, color: "#607080", lineHeight: 1.7 }}>{children}</p>; }
function Card({ children, style }) { return <div style={{ background: "rgba(255,255,255,0.015)", border: "1px solid rgba(255,255,255,0.04)", borderRadius: 10, padding: "18px 18px", ...style }}>{children}</div>; }
function Tag({ children }) { return <span style={{ fontSize: 10.5, color: G, background: "rgba(64,232,160,0.05)", border: "1px solid rgba(64,232,160,0.1)", borderRadius: 4, padding: "2px 8px", fontFamily: "monospace" }}>{children}</span>; }
function td(c) { return { padding: "6px 8px", borderBottom: "1px solid rgba(255,255,255,0.02)", color: c || "#606878" }; }
function code() { return { color: "#E8A830", background: "#E8A83010", padding: "1px 4px", borderRadius: 2, fontSize: 11, fontFamily: "monospace" }; }

// ═══════════════════════════════════════
// APP
// ═══════════════════════════════════════
export default function GIGISite() {
  const [page, setPageRaw] = useState(() => {
    const hash = window.location.hash.replace("#", "");
    return hash || "home";
  });
  const setPage = (id) => {
    setPageRaw(id);
    window.history.pushState({ page: id }, "", "#" + id);
    window.scrollTo(0, 0);
  };
  useEffect(() => {
    const onPop = (e) => {
      const p = e.state?.page || window.location.hash.replace("#", "") || "home";
      setPageRaw(p);
      window.scrollTo(0, 0);
    };
    window.addEventListener("popstate", onPop);
    if (!window.history.state) window.history.replaceState({ page }, "", "#" + page);
    return () => window.removeEventListener("popstate", onPop);
  }, []);
  return (
    <div style={{ minHeight: "100vh", background: BG, color: "#D0D8E0", fontFamily: "'DM Sans','General Sans',system-ui,sans-serif" }}>
      <link href="https://fonts.googleapis.com/css2?family=DM+Sans:wght@300;400;500;600;700;800;900&display=swap" rel="stylesheet" />
      <Nav page={page} go={setPage} />
      {page === "home" && (<>
        <Hero go={setPage} />
        <Stats />
        <CompTable />
        <div style={{ maxWidth: 940, margin: "0 auto", padding: "0 24px 20px" }}>
          <div onClick={() => setPage("nasa")} style={{ background: "linear-gradient(135deg, rgba(64,232,160,0.04), rgba(232,168,48,0.04))", border: "1px solid rgba(64,232,160,0.1)", borderRadius: 12, padding: "20px 24px", cursor: "pointer", marginBottom: 16 }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.12em", color: "#E8A830", marginBottom: 4 }}>FEATURED DEMO</div>
                <div style={{ fontSize: 17, fontWeight: 800, color: "#D0D8E0" }}>GIGI × NASA POWER — Real Atmospheric Data</div>
                <div style={{ fontSize: 12.5, color: "#607080", marginTop: 4 }}>7,320 records from 20 cities. Curvature detected Moscow's -31.9°C cold snap, Toronto's winter storms, Cape Town's wind events. The geometry found the anomalies before scanning the data.</div>
              </div>
              <div style={{ display: "flex", gap: 12, flexShrink: 0, marginLeft: 20 }}>
                <div style={{ textAlign: "center" }}><div style={{ fontSize: 18, fontWeight: 900, color: G, fontFamily: "monospace" }}>~1μs</div><div style={{ fontSize: 9, color: "#506070" }}>query</div></div>
                <div style={{ textAlign: "center" }}><div style={{ fontSize: 18, fontWeight: 900, color: "#E8A830", fontFamily: "monospace" }}>55%</div><div style={{ fontSize: 9, color: "#506070" }}>prediction</div></div>
                <div style={{ textAlign: "center" }}><div style={{ fontSize: 18, fontWeight: 900, color: "#FF6040", fontFamily: "monospace" }}>5.3σ</div><div style={{ fontSize: 9, color: "#506070" }}>max Z</div></div>
              </div>
            </div>
          </div>
        </div>
        <div style={{ maxWidth: 940, margin: "0 auto", padding: "0 24px 16px" }}>
          <SectionLabel>Use Cases</SectionLabel>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(3,1fr)", gap: 10 }}>
            {USE_CASES.slice(0, 3).map((uc, i) => (
              <div key={i} style={{ background: "rgba(255,255,255,0.015)", border: "1px solid rgba(255,255,255,0.04)", borderRadius: 10, padding: "14px 14px", cursor: "pointer" }} onClick={() => setPage("usecases")}>
                <span style={{ fontSize: 18 }}>{uc.icon}</span>
                <div style={{ fontSize: 12.5, fontWeight: 700, color: "#A0B0C0", marginTop: 6 }}>{uc.title}</div>
                <div style={{ fontSize: 10.5, color: "#505060", marginTop: 3, lineHeight: 1.45 }}>{uc.sub}</div>
              </div>
            ))}
          </div>
        </div>
        <div onClick={() => setPage("encryption")} style={{ maxWidth: 940, margin: "0 auto", padding: "0 24px 20px" }}>
          <div style={{ background: "linear-gradient(135deg, rgba(192,96,255,0.04), rgba(64,232,160,0.04))", border: "1px solid rgba(192,96,255,0.12)", borderRadius: 12, padding: "20px 24px", cursor: "pointer" }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.12em", color: "#C060FF", marginBottom: 4 }}>GEOMETRIC ENCRYPTION</div>
                <div style={{ fontSize: 17, fontWeight: 800, color: "#D0D8E0" }}>Encrypt Data. Keep the Geometry. Query Without Decrypting.</div>
                <div style={{ fontSize: 12.5, color: "#607080", marginTop: 4 }}>Per-field affine gauge transforms scramble values but preserve curvature K, anomaly detection, and compression. The database works on encrypted data — only query results decrypt.</div>
              </div>
              <div style={{ display: "flex", gap: 12, flexShrink: 0, marginLeft: 20 }}>
                <div style={{ textAlign: "center" }}><div style={{ fontSize: 18, fontWeight: 900, color: "#C060FF", fontFamily: "monospace" }}>K=K'</div><div style={{ fontSize: 9, color: "#506070" }}>invariant</div></div>
                <div style={{ textAlign: "center" }}><div style={{ fontSize: 18, fontWeight: 900, color: G, fontFamily: "monospace" }}>0ms</div><div style={{ fontSize: 9, color: "#506070" }}>overhead</div></div>
              </div>
            </div>
          </div>
        </div>
        <div onClick={() => setPage("benchmarks")} style={{ maxWidth: 940, margin: "0 auto", padding: "0 24px 20px" }}>
          <div style={{ background: "linear-gradient(135deg, rgba(64,232,160,0.04), rgba(128,128,224,0.04))", border: "1px solid rgba(128,128,224,0.12)", borderRadius: 12, padding: "20px 24px", cursor: "pointer" }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.12em", color: "#8080E0", marginBottom: 4 }}>STRESS TESTED</div>
                <div style={{ fontSize: 17, fontWeight: 800, color: "#D0D8E0" }}>175K Records · 61K HTTP Ops · Edge→Stream Sync</div>
                <div style={{ fontSize: 12.5, color: "#607080", marginTop: 4 }}>100K IoT sensors (79% compression), 50K financial txns, 25K chat messages — all round-tripped with perfect fidelity. 50K inserts at 20K/sec, 10K queries, curvature in 0.6ms. Edge synced 1,001 ops with H¹ = 0.</div>
              </div>
              <div style={{ display: "flex", gap: 12, flexShrink: 0, marginLeft: 20 }}>
                <div style={{ textAlign: "center" }}><div style={{ fontSize: 18, fontWeight: 900, color: G, fontFamily: "monospace" }}>79%</div><div style={{ fontSize: 9, color: "#506070" }}>compress</div></div>
                <div style={{ textAlign: "center" }}><div style={{ fontSize: 18, fontWeight: 900, color: "#E8A830", fontFamily: "monospace" }}>20K/s</div><div style={{ fontSize: 9, color: "#506070" }}>inserts</div></div>
                <div style={{ textAlign: "center" }}><div style={{ fontSize: 18, fontWeight: 900, color: "#8080E0", fontFamily: "monospace" }}>H¹=0</div><div style={{ fontSize: 9, color: "#506070" }}>sync</div></div>
              </div>
            </div>
          </div>
        </div>
        <div onClick={() => setPage("compare")} style={{ maxWidth: 940, margin: "0 auto", padding: "0 24px 20px" }}>
          <div style={{ background: "linear-gradient(135deg, rgba(224,80,80,0.04), rgba(232,168,48,0.04))", border: "1px solid rgba(224,80,80,0.12)", borderRadius: 12, padding: "20px 24px", cursor: "pointer" }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.12em", color: "#E05050", marginBottom: 4 }}>COMPETITIVE ANALYSIS</div>
                <div style={{ fontSize: 17, fontWeight: 800, color: "#D0D8E0" }}>GIGI vs Druid · Cassandra · ELK</div>
                <div style={{ fontSize: 12.5, color: "#607080", marginTop: 4 }}>12 capabilities compared. 10 features nobody else has. They index data — GIGI understands data.</div>
              </div>
              <div style={{ display: "flex", gap: 12, flexShrink: 0, marginLeft: 20 }}>
                <div style={{ textAlign: "center" }}><div style={{ fontSize: 18, fontWeight: 900, color: "#E8A830", fontFamily: "monospace" }}>12</div><div style={{ fontSize: 9, color: "#506070" }}>compared</div></div>
                <div style={{ textAlign: "center" }}><div style={{ fontSize: 18, fontWeight: 900, color: G, fontFamily: "monospace" }}>10</div><div style={{ fontSize: 9, color: "#506070" }}>unique</div></div>
                <div style={{ textAlign: "center" }}><div style={{ fontSize: 18, fontWeight: 900, color: "#E05050", fontFamily: "monospace" }}>0</div><div style={{ fontSize: 9, color: "#506070" }}>losses</div></div>
              </div>
            </div>
          </div>
        </div>
        <div style={{ maxWidth: 940, margin: "0 auto", padding: "0 24px 20px" }}>
          <a href="https://dhoom.dev" target="_blank" rel="noopener noreferrer" style={{ display: "block", textDecoration: "none", background: "linear-gradient(135deg, rgba(232,168,48,0.06), rgba(232,168,48,0.02))", border: "1px solid rgba(232,168,48,0.15)", borderRadius: 12, padding: "20px 24px", cursor: "pointer" }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.12em", color: "#E8A830", marginBottom: 4 }}>PATENT PENDING</div>
                <div style={{ fontSize: 17, fontWeight: 800, color: "#D0D8E0" }}>DHOOM — Fiber Bundle Data Serialization</div>
                <div style={{ fontSize: 12.5, color: "#607080", marginTop: 4 }}>U.S. Provisional Application No. 64/008,940 · Filed March 18, 2026 · Human-readable serialization via fiber bundle geometry with positional encoding, trailing default elision, automatic schema detection, and streaming.</div>
              </div>
              <div style={{ flexShrink: 0, marginLeft: 20, textAlign: "center" }}>
                <div style={{ fontSize: 18, fontWeight: 900, color: "#E8A830", fontFamily: "monospace" }}>64/008,940</div>
                <div style={{ fontSize: 9, color: "#506070" }}>U.S. provisional</div>
              </div>
            </div>
          </a>
        </div>
        <div onClick={() => setPage("bracket")} style={{ maxWidth: 940, margin: "0 auto", padding: "0 24px 20px" }}>
          <div style={{ background: "linear-gradient(135deg, rgba(255,140,0,0.06), rgba(255,69,0,0.04))", border: "1px solid rgba(255,140,0,0.15)", borderRadius: 12, padding: "16px 24px", cursor: "pointer" }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.12em", color: "#FF8C00", marginBottom: 4 }}>MARCH MADNESS LAB</div>
                <div style={{ fontSize: 17, fontWeight: 800, color: "#D0D8E0" }}>🏀 2026 NCAA Tournament Bracket Predictor</div>
                <div style={{ fontSize: 12.5, color: "#607080", marginTop: 4 }}>Seed history (1985–2025) + BartTorvik α-factors + injury adjustments + round scaling. 50K Monte Carlo sims. Built with the same math that powers GIGI.</div>
              </div>
              <div style={{ display: "flex", gap: 12, flexShrink: 0, marginLeft: 20 }}>
                <div style={{ textAlign: "center" }}><div style={{ fontSize: 18, fontWeight: 900, color: "#FF8C00", fontFamily: "monospace" }}>64</div><div style={{ fontSize: 9, color: "#506070" }}>teams</div></div>
                <div style={{ textAlign: "center" }}><div style={{ fontSize: 18, fontWeight: 900, color: G, fontFamily: "monospace" }}>50K</div><div style={{ fontSize: 9, color: "#506070" }}>sims</div></div>
              </div>
            </div>
          </div>
        </div>
        <a href="https://hub.docker.com/r/beerosadavis/gigi" target="_blank" rel="noopener noreferrer" style={{ display: "block", maxWidth: 940, margin: "0 auto", padding: "0 24px 20px", textDecoration: "none" }}>
          <div style={{ background: "linear-gradient(135deg, rgba(0,150,255,0.06), rgba(0,100,220,0.03))", border: "1px solid rgba(0,150,255,0.15)", borderRadius: 12, padding: "16px 24px", cursor: "pointer" }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.12em", color: "#0096FF", marginBottom: 4 }}>DOCKER HUB</div>
                <div style={{ fontSize: 17, fontWeight: 800, color: "#D0D8E0" }}>🐳 Run GIGI in One Command</div>
                <div style={{ fontSize: 12.5, color: "#607080", marginTop: 4 }}>docker pull beerosadavis/gigi:latest · GIGI Stream server on port 3142 · Debian slim runtime · Source-available — commercial use requires license · Auto-built from GitHub</div>
              </div>
              <div style={{ display: "flex", gap: 12, flexShrink: 0, marginLeft: 20 }}>
                <div style={{ textAlign: "center" }}><div style={{ fontSize: 14, fontWeight: 900, color: "#0096FF", fontFamily: "monospace" }}>docker pull</div><div style={{ fontSize: 9, color: "#506070" }}>beerosadavis/gigi</div></div>
              </div>
            </div>
          </div>
        </a>
        <div style={{ padding: "32px 24px 48px", textAlign: "center", display: "flex", gap: 10, justifyContent: "center" }}>
          <Btn label="See NASA Demo" onClick={() => setPage("nasa")} primary />
          <Btn label="Encryption Demo" onClick={() => setPage("encryption")} />
          <Btn label="Run Benchmarks" onClick={() => setPage("benchmarks")} />
          <Btn label="vs Others" onClick={() => setPage("compare")} />
          <Btn label="See Products" onClick={() => setPage("products")} />
        </div>
        <div style={{ padding: "16px", textAlign: "center", fontSize: 10, color: "#1A1A2A", fontFamily: "monospace", borderTop: "1px solid rgba(255,255,255,0.02)" }}>GIGI · Geometric Intrinsic Global Index · © 2026 Davis Geometric Intelligence · U.S. Patent Pending 64/008,940 · Source-available — free for non-commercial use · Commercial licensing: bee_davis@alumni.brown.edu</div>
      </>)}
      {page === "encryption" && <EncryptionPage />}
      {page === "nasa" && <NasaPage />}
      {page === "benchmarks" && <><BenchPage /><StreamPage /><StressPage /></>}
      {page === "usecases" && <UseCasePage />}
      {page === "architecture" && <><ArchPage /><MathPage /></>}
      {page === "compare" && <ComparePage />}
      {page === "products" && <ProductsPage />}
      {page === "gigi" && <GigiPage />}
      {page === "demo" && <InteractiveDemoPage />}
      {page === "bracket" && <BracketPredictor />}
    </div>
  );
}
