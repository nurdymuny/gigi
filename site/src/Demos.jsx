// site/src/Demos.jsx
// Live Demo Gallery — Bitcoin Crash Detector, Music DNA, USGS Earthquakes
// All math runs on GIGI's Rust backend. This file is a display shell only.
import { useState, useEffect, useRef } from "react";

const GIGI_API = import.meta.env.DEV ? "http://localhost:3142" : "https://gigi-stream.fly.dev";
const G = "#40E8A0";
const BG = "#06060A";

// ─────────────────────────────────────────
// API helpers
// ─────────────────────────────────────────
async function gq(query) {
  const res = await fetch(`${GIGI_API}/v1/gql`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ query }),
  });
  if (!res.ok) {
    const e = await res.json().catch(() => ({}));
    throw new Error(e.error || `GQL HTTP ${res.status}`);
  }
  return res.json();
}
async function rp(path, body) {
  const res = await fetch(`${GIGI_API}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    const e = await res.json().catch(() => ({}));
    throw new Error(e.error || `HTTP ${res.status}`);
  }
  return res.json();
}
async function rd(path) {
  const res = await fetch(`${GIGI_API}${path}`, { method: "DELETE" });
  return res.json().catch(() => ({}));
}

// ─────────────────────────────────────────
// Shared UI atoms
// ─────────────────────────────────────────
function DemoLog({ lines }) {
  const ref = useRef(null);
  useEffect(() => { if (ref.current) ref.current.scrollTop = ref.current.scrollHeight; }, [lines]);
  return (
    <div ref={ref} style={{
      background: "#06060E", border: "1px solid rgba(255,255,255,0.05)", borderRadius: 8,
      padding: "12px 14px", fontFamily: "monospace", fontSize: 11.5, lineHeight: 1.7,
      maxHeight: 200, overflowY: "auto", marginBottom: 16,
    }}>
      {lines.map((l, i) => (
        <div key={i} style={{ color: l.color || "#607080" }}>
          {l.text === "" ? <br /> : <span><span style={{ color: "#383848", userSelect: "none" }}>$ </span>{l.text}</span>}
        </div>
      ))}
      <span style={{ color: G, animation: "blink 1s step-end infinite" }}>▊</span>
    </div>
  );
}

function GqlBox({ queries }) {
  return (
    <div style={{ background: "rgba(64,232,160,0.02)", border: "1px solid rgba(64,232,160,0.07)", borderRadius: 8, padding: "12px 14px", marginTop: 16 }}>
      <div style={{ fontSize: 9, fontWeight: 700, letterSpacing: "0.12em", color: "#384050", marginBottom: 8 }}>GQL QUERIES — running live on GIGI's Rust engine</div>
      {queries.map((q, i) => (
        <div key={i} style={{ fontFamily: "monospace", fontSize: 11.5, color: G, padding: "2px 0" }}>{q}</div>
      ))}
    </div>
  );
}

function StatBox({ v, l, color, sub }) {
  return (
    <div style={{ background: "rgba(255,255,255,0.015)", border: "1px solid rgba(255,255,255,0.04)", borderRadius: 10, padding: "14px 12px", textAlign: "center" }}>
      <div style={{ fontSize: 24, fontWeight: 900, color: color || G, fontFamily: "monospace" }}>{v}</div>
      <div style={{ fontSize: 11, fontWeight: 700, color: "#A0B0C0", marginTop: 4 }}>{l}</div>
      {sub && <div style={{ fontSize: 10, color: "#505060", marginTop: 2 }}>{sub}</div>}
    </div>
  );
}

function RunBtn({ onClick, stage, labels }) {
  const disabled = stage === "running";
  const label = stage === "running" ? "Running..." : stage === "done" ? labels[2] : labels[0];
  return (
    <button onClick={onClick} disabled={disabled} style={{
      padding: "11px 26px", borderRadius: 8, border: "none",
      cursor: disabled ? "wait" : "pointer",
      background: disabled ? "#1A2030" : G,
      color: disabled ? "#40E8A060" : BG,
      fontSize: 13, fontWeight: 800,
    }}>{label}</button>
  );
}

// ─────────────────────────────────────────
// DEMO 1 — Bitcoin Crash Detector
// ─────────────────────────────────────────
function BitcoinDemo() {
  const chartRef = useRef(null);
  const [log, setLog] = useState([]);
  const [stage, setStage] = useState("idle");
  const [results, setResults] = useState(null);
  const bundleRef = useRef(null);
  const add = (text, color) => setLog(p => [...p, { text, color }]);

  useEffect(() => {
    return () => { if (bundleRef.current) rd(`/v1/bundles/${bundleRef.current}`).catch(() => {}); };
  }, []);

  async function run() {
    setStage("running");
    setLog([]);
    setResults(null);
    if (chartRef.current && window.Plotly) Plotly.purge(chartRef.current);

    try {
      // 1. Fetch
      add("Fetching 365 days BTC price data from CoinGecko…", "#607080");
      const res = await fetch(
        "https://api.coingecko.com/api/v3/coins/bitcoin/market_chart?vs_currency=usd&days=365"
      );
      if (!res.ok) throw new Error(`CoinGecko: HTTP ${res.status} — try again in a moment`);
      const raw = await res.json();
      add(`Got ${raw.prices.length} data points`, G);

      // 2. Build records
      const records = raw.prices.map(([ts_ms, price], i) => {
        const prev = i > 0 ? raw.prices[i - 1][1] : price;
        const dr = i > 0 ? (price - prev) / prev : 0;
        return {
          day_id: i,
          timestamp_ns: ts_ms * 1000000,
          close_price: +price.toFixed(2),
          volume: +(raw.total_volumes[i]?.[1] || 0),
          daily_return: +dr.toFixed(4),
          crash_signal: dr < -0.05 ? "crash" : dr > 0.05 ? "pump" : "normal",
        };
      });

      const nCrash = records.filter(r => r.crash_signal === "crash").length;
      const nPump = records.filter(r => r.crash_signal === "pump").length;
      add(`Classified: ${nCrash} crash days (>5% drop)  ${nPump} pump days (>5% rise)`, "#E8A830");

      // 3. Create bundle
      const name = "btc_live_demo";
      add(`Creating GIGI bundle '${name}'…`, "#607080");
      await rd(`/v1/bundles/${name}`).catch(() => {});
      await rp("/v1/bundles", {
        name,
        schema: {
          fields: {
            day_id: "numeric",
            timestamp_ns: "numeric",
            close_price: "numeric",
            volume: "numeric",
            daily_return: "numeric",
            crash_signal: "categorical",
          },
          keys: ["day_id"],
        },
      });
      bundleRef.current = name;

      // 4. Ingest
      add(`Ingesting ${records.length} records…`, "#607080");
      const t0 = performance.now();
      const CHUNK = 200;
      for (let i = 0; i < records.length; i += CHUNK) {
        await rp(`/v1/bundles/${name}/insert`, { records: records.slice(i, i + CHUNK) });
      }
      const ms = performance.now() - t0;
      add(`Ingest done: ${records.length} records in ${ms.toFixed(0)}ms  (${(records.length / ms * 1000).toFixed(0)} rec/s)`, G);

      // 5. GQL
      add("", "");
      add("Running GQL queries…", "#606878");

      add(`→ CURVATURE ${name}`, "#E8A830");
      const kRes = await gq(`CURVATURE ${name}`);
      add(`  K = ${kRes.value?.toFixed(6)}   confidence = ${kRes.confidence?.toFixed(4)}`, G);

      add(`→ COVER ${name} ON crash_signal = 'crash'`, "#E8A830");
      const crashRes = await gq(`COVER ${name} ON crash_signal = 'crash'`);
      add(`  Found ${crashRes.count ?? crashRes.records?.length ?? 0} crash days`, G);

      add(`→ SECTION ${name} AT day_id=0`, "#E8A830");
      const sec0 = await gq(`SECTION ${name} AT day_id=0`);
      const price0 = sec0.section?.close_price ?? sec0.close_price;
      add(`  Day 0 BTC price: $${price0 != null ? (+price0).toLocaleString(undefined, { minimumFractionDigits: 2 }) : "—"}`, G);

      const maxCrash = [...(crashRes.records || [])].sort((a, b) => a.daily_return - b.daily_return)[0];

      add("", "");
      add(`DEMO COMPLETE — bundle '${name}' live on GIGI Stream`, G);

      setResults({ records, kVal: kRes.value, kConf: kRes.confidence, crashCount: crashRes.count ?? (crashRes.records?.length ?? 0), maxCrash, queries: [`CURVATURE ${name}`, `COVER ${name} ON crash_signal = 'crash'`, `SECTION ${name} AT day_id=0`] });
      setStage("done");
    } catch (e) {
      add(`ERROR: ${e.message}`, "#FF6040");
      setStage("error");
    }
  }

  useEffect(() => {
    if (!results || !chartRef.current || !window.Plotly) return;
    const { records } = results;
    const dates = records.map(r => new Date(r.timestamp_ns / 1000000).toISOString().split("T")[0]);
    const prices = records.map(r => r.close_price);
    const returns = records.map(r => r.daily_return * 100);

    const crashIdx = records.map((r, i) => r.crash_signal === "crash" ? i : null).filter(i => i !== null);
    const pumpIdx = records.map((r, i) => r.crash_signal === "pump" ? i : null).filter(i => i !== null);

    const DK = {
      paper_bgcolor: "transparent", plot_bgcolor: "rgba(255,255,255,0.015)",
      font: { family: "DM Sans, sans-serif", color: "#e2e8f0", size: 11 },
      margin: { t: 24, b: 50, l: 70, r: 60 },
    };
    Plotly.newPlot(chartRef.current, [
      { type: "scatter", mode: "lines", name: "BTC Close", x: dates, y: prices, line: { color: "#F7931A", width: 2 }, yaxis: "y" },
      { type: "scatter", mode: "markers", name: "▼ Crash (>5% drop)", x: crashIdx.map(i => dates[i]), y: crashIdx.map(i => prices[i]), marker: { size: 9, color: "#FF4040", symbol: "triangle-down", line: { width: 1, color: "#fff" } }, yaxis: "y" },
      { type: "scatter", mode: "markers", name: "▲ Pump (>5% rise)", x: pumpIdx.map(i => dates[i]), y: pumpIdx.map(i => prices[i]), marker: { size: 9, color: G, symbol: "triangle-up", line: { width: 1, color: "#fff" } }, yaxis: "y" },
      { type: "bar", name: "Daily Return %", x: dates, y: returns, marker: { color: returns.map(v => v < -5 ? "#FF404099" : v > 5 ? `${G}88` : "#2020308A") }, yaxis: "y2", opacity: 0.8 },
    ], {
      ...DK,
      xaxis: { ...DK.xaxis, gridcolor: "rgba(255,255,255,0.04)", type: "date" },
      yaxis: { gridcolor: "rgba(255,255,255,0.04)", tickprefix: "$", title: { text: "BTC Price", font: { size: 10 } } },
      yaxis2: { title: { text: "Daily Return %", font: { size: 10 } }, side: "right", overlaying: "y", showgrid: false, zeroline: true, zerolinecolor: "rgba(255,255,255,0.07)" },
      legend: { orientation: "h", y: 1.05, x: 0.5, xanchor: "center", font: { size: 10 } },
      hovermode: "x unified",
    }, { displayModeBar: false, responsive: true });
    return () => { if (chartRef.current) Plotly.purge(chartRef.current); };
  }, [results]);

  const worst = results?.maxCrash;

  return (
    <div>
      <div style={{ marginBottom: 20 }}>
        <div style={{ display: "flex", gap: 12, alignItems: "flex-start", flexWrap: "wrap", marginBottom: 14 }}>
          <div style={{ flex: 1, minWidth: 260 }}>
            <div style={{ fontSize: 20, fontWeight: 800, color: "#E0E8F0", marginBottom: 6 }}>₿ Bitcoin Crash Detector</div>
            <div style={{ fontSize: 12.5, color: "#607080", lineHeight: 1.6 }}>
              Fetches 365 days of live BTC price data, classifies every day as <span style={{ color: "#FF4040" }}>crash</span> / <span style={{ color: G }}>pump</span> / normal, ingests into GIGI's fiber bundle, then queries curvature. The crash days are detected by the geometry — no anomaly detection code, no thresholds written in the engine. <strong style={{ color: "#A0B0C0" }}>Curvature K measures how "curved" the price manifold is.</strong>
            </div>
          </div>
          <RunBtn onClick={run} stage={stage} labels={["▶ Run Live Demo", "", "↻ Re-run"]} />
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          {[{ label: "CoinGecko API", color: "#E8A830" }, { label: "365 days BTC/USD", color: "#507080" }, { label: "GIGI ingest → CURVATURE", color: "#507080" }, { label: "Crash = daily_return < −5%", color: "#507080" }].map((t, i) => (
            <span key={i} style={{ fontSize: 10, color: t.color, background: "rgba(255,255,255,0.03)", border: "1px solid rgba(255,255,255,0.06)", borderRadius: 12, padding: "3px 10px", fontFamily: "monospace" }}>{t.label}</span>
          ))}
        </div>
      </div>

      {log.length > 0 && <DemoLog lines={log} />}

      {results && (<>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(4,1fr)", gap: 10, marginBottom: 16 }}>
          <StatBox v={results.records.length} l="Days Ingested" sub="daily BTC close" />
          <StatBox v={`K = ${results.kVal?.toFixed(4)}`} l="Curvature" color={results.kVal > 0.05 ? "#FF6040" : G} sub={`conf ${(results.kConf * 100)?.toFixed(1)}%`} />
          <StatBox v={results.crashCount} l="Crash Days" color="#FF4040" sub=">5% single-day drop" />
          <StatBox v={worst ? `${(worst.daily_return * 100).toFixed(1)}%` : "—"} l="Worst Drop" color="#FF4040" sub={worst ? `$${(+worst.close_price).toLocaleString(undefined, { maximumFractionDigits: 0 })} BTC` : ""} />
        </div>
        <div ref={chartRef} style={{ width: "100%", height: 380, marginBottom: 12 }} />
        <GqlBox queries={results.queries} />
      </>)}
    </div>
  );
}

// ─────────────────────────────────────────
// DEMO 2 — Music DNA
// ─────────────────────────────────────────
const MUSIC_DATA = [
  { artist: "Radiohead",         rock: 0.90, electronic: 0.75, pop: 0.35, hip_hop: 0.05, jazz: 0.20, classical: 0.25, metal: 0.10, folk: 0.05, primary_genre: "art_rock" },
  { artist: "Pink Floyd",        rock: 0.92, electronic: 0.55, pop: 0.30, hip_hop: 0.00, jazz: 0.10, classical: 0.25, metal: 0.10, folk: 0.10, primary_genre: "progressive" },
  { artist: "Portishead",        rock: 0.40, electronic: 0.85, pop: 0.30, hip_hop: 0.20, jazz: 0.40, classical: 0.15, metal: 0.05, folk: 0.00, primary_genre: "trip_hop" },
  { artist: "Massive Attack",    rock: 0.30, electronic: 0.88, pop: 0.25, hip_hop: 0.30, jazz: 0.35, classical: 0.10, metal: 0.00, folk: 0.00, primary_genre: "trip_hop" },
  { artist: "Aphex Twin",        rock: 0.05, electronic: 0.99, pop: 0.05, hip_hop: 0.10, jazz: 0.10, classical: 0.20, metal: 0.05, folk: 0.00, primary_genre: "electronic" },
  { artist: "Daft Punk",         rock: 0.20, electronic: 0.95, pop: 0.65, hip_hop: 0.15, jazz: 0.10, classical: 0.10, metal: 0.00, folk: 0.00, primary_genre: "electronic" },
  { artist: "Boards of Canada",   rock: 0.15, electronic: 0.92, pop: 0.15, hip_hop: 0.05, jazz: 0.15, classical: 0.15, metal: 0.00, folk: 0.10, primary_genre: "electronic" },
  { artist: "Kendrick Lamar",    rock: 0.10, electronic: 0.35, pop: 0.45, hip_hop: 0.97, jazz: 0.40, classical: 0.10, metal: 0.00, folk: 0.00, primary_genre: "hip_hop" },
  { artist: "J. Cole",           rock: 0.05, electronic: 0.25, pop: 0.40, hip_hop: 0.95, jazz: 0.25, classical: 0.05, metal: 0.00, folk: 0.00, primary_genre: "hip_hop" },
  { artist: "Frank Ocean",       rock: 0.15, electronic: 0.55, pop: 0.70, hip_hop: 0.60, jazz: 0.45, classical: 0.15, metal: 0.00, folk: 0.05, primary_genre: "rnb" },
  { artist: "Miles Davis",       rock: 0.05, electronic: 0.10, pop: 0.05, hip_hop: 0.05, jazz: 0.99, classical: 0.30, metal: 0.00, folk: 0.05, primary_genre: "jazz" },
  { artist: "John Coltrane",     rock: 0.00, electronic: 0.00, pop: 0.00, hip_hop: 0.00, jazz: 0.99, classical: 0.25, metal: 0.00, folk: 0.00, primary_genre: "jazz" },
  { artist: "Thelonious Monk",   rock: 0.00, electronic: 0.00, pop: 0.05, hip_hop: 0.00, jazz: 0.97, classical: 0.05, metal: 0.00, folk: 0.00, primary_genre: "jazz" },
  { artist: "The Beatles",       rock: 0.90, electronic: 0.15, pop: 0.85, hip_hop: 0.00, jazz: 0.20, classical: 0.20, metal: 0.05, folk: 0.35, primary_genre: "rock" },
  { artist: "David Bowie",       rock: 0.75, electronic: 0.55, pop: 0.65, hip_hop: 0.05, jazz: 0.20, classical: 0.15, metal: 0.10, folk: 0.10, primary_genre: "glam_rock" },
  { artist: "Björk",             rock: 0.30, electronic: 0.85, pop: 0.40, hip_hop: 0.05, jazz: 0.20, classical: 0.45, metal: 0.10, folk: 0.25, primary_genre: "art_rock" },
  { artist: "PJ Harvey",         rock: 0.85, electronic: 0.20, pop: 0.30, hip_hop: 0.00, jazz: 0.15, classical: 0.10, metal: 0.25, folk: 0.35, primary_genre: "art_rock" },
  { artist: "LCD Soundsystem",   rock: 0.65, electronic: 0.82, pop: 0.40, hip_hop: 0.15, jazz: 0.10, classical: 0.00, metal: 0.05, folk: 0.00, primary_genre: "art_rock" },
  { artist: "Vampire Weekend",   rock: 0.60, electronic: 0.25, pop: 0.70, hip_hop: 0.05, jazz: 0.30, classical: 0.20, metal: 0.00, folk: 0.45, primary_genre: "indie" },
  { artist: "Animal Collective",  rock: 0.50, electronic: 0.65, pop: 0.25, hip_hop: 0.00, jazz: 0.10, classical: 0.10, metal: 0.05, folk: 0.50, primary_genre: "art_rock" },
  { artist: "Taylor Swift",      rock: 0.30, electronic: 0.25, pop: 0.98, hip_hop: 0.10, jazz: 0.00, classical: 0.05, metal: 0.00, folk: 0.40, primary_genre: "pop" },
  { artist: "Beyoncé",           rock: 0.10, electronic: 0.45, pop: 0.95, hip_hop: 0.60, jazz: 0.20, classical: 0.10, metal: 0.00, folk: 0.00, primary_genre: "pop" },
  { artist: "Ariana Grande",     rock: 0.05, electronic: 0.50, pop: 0.97, hip_hop: 0.35, jazz: 0.15, classical: 0.10, metal: 0.00, folk: 0.00, primary_genre: "pop" },
  { artist: "The Weeknd",        rock: 0.15, electronic: 0.60, pop: 0.85, hip_hop: 0.55, jazz: 0.10, classical: 0.05, metal: 0.00, folk: 0.00, primary_genre: "rnb" },
  { artist: "Burial",            rock: 0.05, electronic: 0.97, pop: 0.10, hip_hop: 0.05, jazz: 0.20, classical: 0.10, metal: 0.00, folk: 0.00, primary_genre: "electronic" },
  { artist: "Flying Lotus",      rock: 0.05, electronic: 0.90, pop: 0.10, hip_hop: 0.55, jazz: 0.75, classical: 0.15, metal: 0.00, folk: 0.00, primary_genre: "electronic" },
  { artist: "Nine Inch Nails",   rock: 0.70, electronic: 0.80, pop: 0.20, hip_hop: 0.05, jazz: 0.00, classical: 0.10, metal: 0.60, folk: 0.00, primary_genre: "industrial" },
  { artist: "Portishead",        rock: 0.40, electronic: 0.85, pop: 0.30, hip_hop: 0.20, jazz: 0.40, classical: 0.15, metal: 0.05, folk: 0.00, primary_genre: "trip_hop" },
  { artist: "Bon Iver",          rock: 0.45, electronic: 0.35, pop: 0.55, hip_hop: 0.00, jazz: 0.10, classical: 0.20, metal: 0.00, folk: 0.85, primary_genre: "indie" },
  { artist: "Nick Drake",        rock: 0.25, electronic: 0.00, pop: 0.20, hip_hop: 0.00, jazz: 0.30, classical: 0.45, metal: 0.00, folk: 0.95, primary_genre: "folk" },
  { artist: "Joni Mitchell",     rock: 0.30, electronic: 0.00, pop: 0.45, hip_hop: 0.00, jazz: 0.55, classical: 0.20, metal: 0.00, folk: 0.90, primary_genre: "folk" },
  { artist: "Elliott Smith",     rock: 0.65, electronic: 0.05, pop: 0.45, hip_hop: 0.00, jazz: 0.10, classical: 0.15, metal: 0.00, folk: 0.60, primary_genre: "indie" },
  { artist: "Sigur Rós",         rock: 0.55, electronic: 0.40, pop: 0.25, hip_hop: 0.00, jazz: 0.05, classical: 0.70, metal: 0.10, folk: 0.20, primary_genre: "post_rock" },
  { artist: "Explosions in Sky", rock: 0.80, electronic: 0.25, pop: 0.10, hip_hop: 0.00, jazz: 0.05, classical: 0.45, metal: 0.30, folk: 0.10, primary_genre: "post_rock" },
  { artist: "Brian Eno",         rock: 0.20, electronic: 0.90, pop: 0.15, hip_hop: 0.00, jazz: 0.10, classical: 0.55, metal: 0.00, folk: 0.05, primary_genre: "ambient" },
  { artist: "Bill Evans",        rock: 0.00, electronic: 0.00, pop: 0.10, hip_hop: 0.00, jazz: 0.99, classical: 0.50, metal: 0.00, folk: 0.05, primary_genre: "jazz" },
  { artist: "Charles Mingus",    rock: 0.00, electronic: 0.00, pop: 0.00, hip_hop: 0.05, jazz: 0.99, classical: 0.30, metal: 0.00, folk: 0.00, primary_genre: "jazz" },
  { artist: "Sade",              rock: 0.05, electronic: 0.20, pop: 0.60, hip_hop: 0.10, jazz: 0.65, classical: 0.10, metal: 0.00, folk: 0.05, primary_genre: "rnb" },
  { artist: "Erykah Badu",       rock: 0.05, electronic: 0.35, pop: 0.45, hip_hop: 0.55, jazz: 0.70, classical: 0.05, metal: 0.00, folk: 0.10, primary_genre: "rnb" },
  { artist: "D'Angelo",          rock: 0.10, electronic: 0.30, pop: 0.50, hip_hop: 0.60, jazz: 0.65, classical: 0.10, metal: 0.00, folk: 0.05, primary_genre: "rnb" },
  { artist: "Kanye West",        rock: 0.20, electronic: 0.55, pop: 0.65, hip_hop: 0.95, jazz: 0.30, classical: 0.10, metal: 0.10, folk: 0.00, primary_genre: "hip_hop" },
];
const GENRE_FIELDS = ["rock", "electronic", "pop", "hip_hop", "jazz", "classical", "metal", "folk"];
const GENRE_COLORS = ["#FF6040", "#40E8A0", "#E8A830", "#C060FF", "#818cf8", "#22d3ee", "#f87171", "#a3e635"];
const SEED_ARTISTS = ["Radiohead", "Aphex Twin", "Miles Davis", "Kendrick Lamar", "Nick Drake", "The Beatles"];

function MusicDNADemo() {
  const radarRef = useRef(null);
  const mapRef = useRef(null);
  const [log, setLog] = useState([]);
  const [stage, setStage] = useState("idle");
  const [results, setResults] = useState(null);
  const [seedArtist, setSeedArtist] = useState("Radiohead");
  const bundleRef = useRef(null);
  const add = (text, color) => setLog(p => [...p, { text, color }]);

  useEffect(() => {
    return () => { if (bundleRef.current) rd(`/v1/bundles/${bundleRef.current}`).catch(() => {}); };
  }, []);

  async function run() {
    setStage("running");
    setLog([]);
    setResults(null);
    if (radarRef.current && window.Plotly) Plotly.purge(radarRef.current);
    if (mapRef.current && window.Plotly) Plotly.purge(mapRef.current);

    try {
      const dedupedData = MUSIC_DATA.filter((a, i, arr) => arr.findIndex(b => b.artist === a.artist) === i);

      add(`Using curated dataset: ${dedupedData.length} artists × ${GENRE_FIELDS.length} genre dimensions`, "#607080");
      const name = "music_dna_demo";
      add(`Creating GIGI bundle '${name}'…`, "#607080");
      await rd(`/v1/bundles/${name}`).catch(() => {});

      const schemaFields = { artist: "categorical" };
      GENRE_FIELDS.forEach(f => { schemaFields[f] = "numeric"; });
      schemaFields.primary_genre = "categorical";

      await rp("/v1/bundles", {
        name,
        schema: { fields: schemaFields, keys: ["artist"] },
      });
      bundleRef.current = name;

      add(`Ingesting ${dedupedData.length} artist records…`, "#607080");
      const t0 = performance.now();
      const records = dedupedData.map(a => {
        const rec = {};
        Object.keys(schemaFields).forEach(k => { rec[k] = a[k] ?? 0; });
        return rec;
      });
      await rp(`/v1/bundles/${name}/insert`, { records });
      const ms = performance.now() - t0;
      add(`Ingest done in ${ms.toFixed(0)}ms`, G);

      add("", "");
      add("Running GQL queries…", "#606878");

      add(`→ CURVATURE ${name}`, "#E8A830");
      const kRes = await gq(`CURVATURE ${name}`);
      add(`  K = ${kRes.value?.toFixed(6)}   confidence = ${kRes.confidence?.toFixed(4)}`, G);

      const safeSeed = seedArtist.replace(/'/g, "''");
      add(`→ SECTION ${name} AT artist='${safeSeed}'`, "#E8A830");
      const secRes = await gq(`SECTION ${name} AT artist='${safeSeed}'`);
      const dna = secRes.section || secRes;
      const topGenres = GENRE_FIELDS
        .map(f => ({ f, v: +(dna[f] ?? 0) }))
        .sort((a, b) => b.v - a.v)
        .slice(0, 3)
        .map(x => `${x.f}=${(x.v * 100).toFixed(0)}%`)
        .join("  ");
      add(`  ${safeSeed}: ${topGenres}`, G);

      add(`→ COVER ${name} ON primary_genre = '${dna.primary_genre ?? "—"}'`, "#E8A830");
      const similarRes = await gq(`COVER ${name} ON primary_genre = '${dna.primary_genre}'`);
      const similar = similarRes.records || [];
      add(`  Found ${similar.length} artist(s) with same primary genre`, G);

      add("", "");
      add(`DEMO COMPLETE — bundle '${name}' live on GIGI Stream`, G);

      setResults({
        data: dedupedData, dna, seedArtist, similar,
        kVal: kRes.value, kConf: kRes.confidence,
        queries: [
          `CURVATURE ${name}`,
          `SECTION ${name} AT artist='${safeSeed}'`,
          `COVER ${name} ON primary_genre = '${dna.primary_genre}'`,
        ],
      });
      setStage("done");
    } catch (e) {
      add(`ERROR: ${e.message}`, "#FF6040");
      setStage("error");
    }
  }

  // Radar chart
  useEffect(() => {
    if (!results || !radarRef.current || !window.Plotly) return;
    const { data, seedArtist } = results;
    const chosen = [seedArtist, ...results.similar.slice(0, 3).map(r => r.artist).filter(a => a !== seedArtist)].slice(0, 5);
    const palette = ["#E8A830", G, "#818cf8", "#f87171", "#22d3ee"];
    const categories = [...GENRE_FIELDS, GENRE_FIELDS[0]]; // close loop
    const traces = chosen.map((artist, i) => {
      const a = data.find(x => x.artist === artist);
      if (!a) return null;
      return {
        type: "scatterpolar", mode: "lines+markers",
        name: artist,
        r: [...GENRE_FIELDS.map(f => (a[f] ?? 0) * 100), (a[GENRE_FIELDS[0]] ?? 0) * 100],
        theta: categories,
        fill: "toself",
        fillcolor: `${palette[i]}22`,
        line: { color: palette[i], width: 2 },
        marker: { size: 6, color: palette[i] },
      };
    }).filter(Boolean);

    Plotly.newPlot(radarRef.current, traces, {
      paper_bgcolor: "transparent", plot_bgcolor: "transparent",
      font: { family: "DM Sans, sans-serif", color: "#e2e8f0", size: 11 },
      margin: { t: 30, b: 30, l: 30, r: 30 },
      polar: {
        bgcolor: "rgba(255,255,255,0.01)",
        radialaxis: { visible: true, range: [0, 100], gridcolor: "rgba(255,255,255,0.06)", tickfont: { size: 9 } },
        angularaxis: { gridcolor: "rgba(255,255,255,0.06)" },
      },
      legend: { orientation: "v", x: 1.08, font: { size: 10 } },
    }, { displayModeBar: false, responsive: true });
    return () => { if (radarRef.current) Plotly.purge(radarRef.current); };
  }, [results]);

  // Genre scatter map
  useEffect(() => {
    if (!results || !mapRef.current || !window.Plotly) return;
    const { data } = results;
    const genreList = [...new Set(data.map(a => a.primary_genre))];
    const gColors = {};
    const palette2 = ["#40E8A0", "#E8A830", "#818cf8", "#f87171", "#22d3ee", "#a3e635", "#fbbf24", "#c084fc", "#67e8f9", "#86efac"];
    genreList.forEach((g, i) => { gColors[g] = palette2[i % palette2.length]; });

    const traces = genreList.map(genre => {
      const artists = data.filter(a => a.primary_genre === genre);
      return {
        type: "scatter", mode: "markers+text",
        name: genre.replace(/_/g, " "),
        x: artists.map(a => a.electronic),
        y: artists.map(a => a.rock),
        text: artists.map(a => a.artist),
        textposition: "top center", textfont: { size: 9, color: "#94a3b8" },
        marker: { size: artists.map(a => 10 + (a.pop ?? 0) * 10), color: gColors[genre], opacity: 0.85, line: { width: 1, color: "rgba(255,255,255,0.2)" } },
        hovertemplate: "<b>%{text}</b><br>electronic=%{x:.2f}  rock=%{y:.2f}<extra></extra>",
      };
    });

    Plotly.newPlot(mapRef.current, traces, {
      paper_bgcolor: "transparent", plot_bgcolor: "rgba(255,255,255,0.015)",
      font: { family: "DM Sans, sans-serif", color: "#e2e8f0", size: 11 },
      margin: { t: 20, b: 50, l: 60, r: 20 },
      xaxis: { gridcolor: "rgba(255,255,255,0.05)", title: { text: "Electronic →", font: { size: 11 } }, range: [-0.05, 1.1] },
      yaxis: { gridcolor: "rgba(255,255,255,0.05)", title: { text: "Rock →", font: { size: 11 } }, range: [-0.05, 1.1] },
      legend: { font: { size: 9 }, tracegroupgap: 2 },
    }, { displayModeBar: false, responsive: true });
    return () => { if (mapRef.current) Plotly.purge(mapRef.current); };
  }, [results]);

  return (
    <div>
      <div style={{ display: "flex", gap: 12, alignItems: "flex-start", flexWrap: "wrap", marginBottom: 16 }}>
        <div style={{ flex: 1, minWidth: 260 }}>
          <div style={{ fontSize: 20, fontWeight: 800, color: "#E0E8F0", marginBottom: 6 }}>🎵 Music DNA</div>
          <div style={{ fontSize: 12.5, color: "#607080", lineHeight: 1.6 }}>
            40 iconic artists encoded as <strong style={{ color: "#A0B0C0" }}>8-dimensional genre vectors</strong> on a GIGI fiber bundle. Pick a seed artist — GIGI returns geometrically similar artists by querying the fiber over their genre coordinate. <strong style={{ color: "#A0B0C0" }}>No distance function. No embeddings. No cosine similarity.</strong> The manifold topology defines neighborhood.
          </div>
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 8, alignItems: "flex-end" }}>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <span style={{ fontSize: 11, color: "#607080" }}>Seed:</span>
            <select value={seedArtist} onChange={e => setSeedArtist(e.target.value)}
              style={{ background: "#0A0A14", border: "1px solid rgba(255,255,255,0.08)", borderRadius: 6, color: "#A0B0C0", fontSize: 12, padding: "6px 10px", cursor: "pointer" }}>
              {SEED_ARTISTS.map(a => <option key={a} value={a}>{a}</option>)}
            </select>
          </div>
          <RunBtn onClick={run} stage={stage} labels={["▶ Run Demo", "", "↻ Re-run"]} />
        </div>
      </div>

      {log.length > 0 && <DemoLog lines={log} />}

      {results && (<>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(4,1fr)", gap: 10, marginBottom: 16 }}>
          <StatBox v={MUSIC_DATA.filter((a, i, arr) => arr.findIndex(b => b.artist === a.artist) === i).length} l="Artists" sub={`${GENRE_FIELDS.length} genre dims`} />
          <StatBox v={`K = ${results.kVal?.toFixed(4)}`} l="Music Curvature" color="#818cf8" sub="genre diversity" />
          <StatBox v={results.similar.length} l="Genre Neighbors" sub={`same as ${results.seedArtist}'s genre`} />
          <StatBox v={results.dna?.primary_genre?.replace(/_/g, " ") ?? "—"} l={`${results.seedArtist}'s genre`} color="#E8A830" />
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, marginBottom: 16 }}>
          <div>
            <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.1em", color: "#384050", marginBottom: 8 }}>GENRE DNA — RADAR CHART (seed + neighbors)</div>
            <div style={{ background: "rgba(255,255,255,0.01)", border: "1px solid rgba(255,255,255,0.04)", borderRadius: 10, padding: 8 }}>
              <div ref={radarRef} style={{ width: "100%", height: 320 }} />
            </div>
          </div>
          <div>
            <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.1em", color: "#384050", marginBottom: 8 }}>MUSIC MANIFOLD — rock vs electronic (size = pop)</div>
            <div style={{ background: "rgba(255,255,255,0.01)", border: "1px solid rgba(255,255,255,0.04)", borderRadius: 10, padding: 8 }}>
              <div ref={mapRef} style={{ width: "100%", height: 320 }} />
            </div>
          </div>
        </div>

        {results.similar.length > 0 && (
          <div style={{ marginBottom: 12 }}>
            <div style={{ fontSize: 11, fontWeight: 700, color: "#A0B0C0", marginBottom: 8 }}>GIGI says these share the fiber with <span style={{ color: "#E8A830" }}>{results.seedArtist}</span></div>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              {results.similar.map((r, i) => (
                <div key={i} style={{ background: "rgba(64,232,160,0.03)", border: "1px solid rgba(64,232,160,0.1)", borderRadius: 8, padding: "8px 14px" }}>
                  <div style={{ fontSize: 12.5, fontWeight: 700, color: G }}>{r.artist}</div>
                  <div style={{ fontSize: 10, color: "#506070" }}>
                    {GENRE_FIELDS.map(f => `${f[0].toUpperCase()}:${((r[f] ?? 0) * 100).toFixed(0)}%`).join("  ")}
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        <GqlBox queries={results.queries} />
      </>)}
    </div>
  );
}

// ─────────────────────────────────────────
// DEMO 3 — USGS Earthquake Feed
// ─────────────────────────────────────────
function EarthquakeDemo() {
  const mapRef = useRef(null);
  const histRef = useRef(null);
  const [log, setLog] = useState([]);
  const [stage, setStage] = useState("idle");
  const [results, setResults] = useState(null);
  const bundleRef = useRef(null);
  const add = (text, color) => setLog(p => [...p, { text, color }]);

  useEffect(() => {
    return () => { if (bundleRef.current) rd(`/v1/bundles/${bundleRef.current}`).catch(() => {}); };
  }, []);

  async function run() {
    setStage("running");
    setLog([]);
    setResults(null);
    if (mapRef.current && window.Plotly) Plotly.purge(mapRef.current);
    if (histRef.current && window.Plotly) Plotly.purge(histRef.current);

    try {
      // 1. Fetch last 30 days all M1.0+ earthquakes
      add("Fetching last 30 days of earthquakes from USGS…", "#607080");
      const res = await fetch(
        "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/1.0_month.geojson"
      );
      if (!res.ok) throw new Error(`USGS: HTTP ${res.status}`);
      const raw = await res.json();
      const features = raw.features.filter(
        f => f.geometry?.coordinates?.length >= 2 && f.properties?.mag != null
      );
      add(`Got ${features.length} earthquakes (M≥1.0, last 30 days)`, G);

      // 2. Build records
      const records = features.map((f, i) => {
        const mag = f.properties.mag ?? 0;
        return {
          quake_id: i,
          time_ns: (f.properties.time ?? 0) * 1000000,
          magnitude: +mag.toFixed(2),
          depth_km: +(f.geometry.coordinates[2] ?? 0).toFixed(1),
          lat: +(f.geometry.coordinates[1] ?? 0).toFixed(4),
          lon: +(f.geometry.coordinates[0] ?? 0).toFixed(4),
          place: (f.properties.place ?? "unknown").slice(0, 40),
          quake_type: f.properties.type ?? "earthquake",
          significant: mag >= 5.0 ? "true" : mag >= 3.0 ? "moderate" : "minor",
        };
      });

      const sigCount = records.filter(r => r.significant === "true").length;
      const modCount = records.filter(r => r.significant === "moderate").length;
      add(`Classified: ${sigCount} significant (M≥5.0)  ${modCount} moderate (M≥3.0)`, "#E8A830");

      // 3. Create bundle
      const name = "quake_live_demo";
      add(`Creating GIGI bundle '${name}'…`, "#607080");
      await rd(`/v1/bundles/${name}`).catch(() => {});
      await rp("/v1/bundles", {
        name,
        schema: {
          fields: {
            quake_id: "numeric",
            time_ns: "numeric",
            magnitude: "numeric",
            depth_km: "numeric",
            lat: "numeric",
            lon: "numeric",
            place: "categorical",
            quake_type: "categorical",
            significant: "categorical",
          },
          keys: ["quake_id"],
        },
      });
      bundleRef.current = name;

      // 4. Ingest
      add(`Ingesting ${records.length} records…`, "#607080");
      const t0 = performance.now();
      const CHUNK = 500;
      for (let i = 0; i < records.length; i += CHUNK) {
        await rp(`/v1/bundles/${name}/insert`, { records: records.slice(i, i + CHUNK) });
      }
      const ms = performance.now() - t0;
      add(`Ingest done: ${records.length} records in ${ms.toFixed(0)}ms  (${(records.length / ms * 1000).toFixed(0)} rec/s)`, G);

      // 5. GQL
      add("", "");
      add("Running GQL queries…", "#606878");

      add(`→ CURVATURE ${name}`, "#E8A830");
      const kRes = await gq(`CURVATURE ${name}`);
      add(`  K = ${kRes.value?.toFixed(6)}   confidence = ${kRes.confidence?.toFixed(4)}`, G);

      add(`→ COVER ${name} ON significant = 'true'`, "#E8A830");
      const sigRes = await gq(`COVER ${name} ON significant = 'true'`);
      const sigQuakes = (sigRes.records || []).sort((a, b) => (b.magnitude ?? 0) - (a.magnitude ?? 0));
      add(`  Found ${sigRes.count ?? sigQuakes.length} significant earthquakes (M≥5.0)`, G);
      if (sigQuakes.length > 0) {
        const biggest = sigQuakes[0];
        add(`  Biggest: M${biggest.magnitude}  ${biggest.place}`, G);
      }

      add(`→ SECTION ${name} AT quake_id=0`, "#E8A830");
      const sec0 = await gq(`SECTION ${name} AT quake_id=0`);
      const q0 = sec0.section || sec0;
      add(`  Quake #0: M${q0.magnitude ?? "?"}  depth=${q0.depth_km ?? "?"}km  ${(q0.place ?? "").slice(0, 35)}`, G);

      add("", "");
      add(`DEMO COMPLETE — bundle '${name}' live on GIGI Stream`, G);

      setResults({
        records, kVal: kRes.value, kConf: kRes.confidence,
        sigQuakes, sigCount: sigRes.count ?? sigQuakes.length,
        queries: [
          `CURVATURE ${name}`,
          `COVER ${name} ON significant = 'true'`,
          `SECTION ${name} AT quake_id=0`,
        ],
      });
      setStage("done");
    } catch (e) {
      add(`ERROR: ${e.message}`, "#FF6040");
      setStage("error");
    }
  }

  // World map
  useEffect(() => {
    if (!results || !mapRef.current || !window.Plotly) return;
    const { records } = results;
    Plotly.newPlot(mapRef.current, [{
      type: "scattergeo", mode: "markers",
      lat: records.map(r => r.lat),
      lon: records.map(r => r.lon),
      text: records.map(r => `M${r.magnitude} — ${r.place}`),
      marker: {
        size: records.map(r => Math.max(4, r.magnitude * 3)),
        color: records.map(r => r.magnitude),
        colorscale: [[0, "#1a3040"], [0.3, "#40688A"], [0.6, "#E8A830"], [0.8, "#FF6040"], [1, "#FF1A1A"]],
        cmin: 1, cmax: 8,
        colorbar: { title: { text: "M", font: { size: 11 } }, thickness: 10, len: 0.5, tickfont: { size: 9 } },
        opacity: 0.8, line: { width: 0 },
      },
      hovertemplate: "<b>%{text}</b><extra></extra>",
    }], {
      paper_bgcolor: "transparent",
      margin: { t: 10, b: 10, l: 5, r: 5 },
      font: { color: "#e2e8f0" },
      geo: {
        projection: { type: "natural earth" },
        showland: true, landcolor: "#0e1a22", showocean: true, oceancolor: "#060e14",
        showcoastlines: true, coastlinecolor: "#1e3040", coastlinewidth: 0.7,
        showcountries: true, countrycolor: "#0e2030", countrywidth: 0.3,
        bgcolor: "transparent",
        lataxis: { gridcolor: "rgba(255,255,255,0.02)" },
        lonaxis: { gridcolor: "rgba(255,255,255,0.02)" },
      },
    }, { displayModeBar: false, responsive: true });
    return () => { if (mapRef.current) Plotly.purge(mapRef.current); };
  }, [results]);

  // Magnitude histogram
  useEffect(() => {
    if (!results || !histRef.current || !window.Plotly) return;
    const { records } = results;
    const mags = records.map(r => r.magnitude);
    Plotly.newPlot(histRef.current, [{
      type: "histogram", x: mags, nbinsx: 30,
      marker: {
        color: mags, colorscale: [[0, "#304050"], [0.4, "#E8A830"], [0.7, "#FF6040"], [1, "#FF1A1A"]],
        cmin: 1, cmax: 8, line: { width: 0 },
      },
      hovertemplate: "M %{x:.1f}: %{y} quakes<extra></extra>",
    }], {
      paper_bgcolor: "transparent", plot_bgcolor: "rgba(255,255,255,0.015)",
      font: { family: "DM Sans, sans-serif", color: "#e2e8f0", size: 11 },
      margin: { t: 10, b: 45, l: 50, r: 10 },
      xaxis: { gridcolor: "rgba(255,255,255,0.04)", title: { text: "Magnitude", font: { size: 11 } } },
      yaxis: { gridcolor: "rgba(255,255,255,0.04)", title: { text: "Count", font: { size: 11 } } },
    }, { displayModeBar: false, responsive: true });
    return () => { if (histRef.current) Plotly.purge(histRef.current); };
  }, [results]);

  return (
    <div>
      <div style={{ display: "flex", gap: 12, alignItems: "flex-start", flexWrap: "wrap", marginBottom: 16 }}>
        <div style={{ flex: 1, minWidth: 260 }}>
          <div style={{ fontSize: 20, fontWeight: 800, color: "#E0E8F0", marginBottom: 6 }}>🌍 USGS Live Earthquake Feed</div>
          <div style={{ fontSize: 12.5, color: "#607080", lineHeight: 1.6 }}>
            Live USGS GeoJSON feed — last 30 days, all M≥1.0. Every earthquake is a point on the seismic manifold.{" "}
            <strong style={{ color: "#A0B0C0" }}>GIGI curvature K spikes in active clusters</strong> (Pacific Ring of Fire, mid-ocean ridges).
            Significant quakes isolated via categorical fiber query. Updated every 5 minutes from USGS — no API key.
          </div>
        </div>
        <RunBtn onClick={run} stage={stage} labels={["▶ Fetch Live Data", "", "↻ Refresh Feed"]} />
      </div>

      {log.length > 0 && <DemoLog lines={log} />}

      {results && (<>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(4,1fr)", gap: 10, marginBottom: 14 }}>
          <StatBox v={results.records.length} l="Quakes (30d)" sub="M≥1.0 worldwide" />
          <StatBox v={`K = ${results.kVal?.toFixed(4)}`} l="Seismic Curvature" color="#E8A830" sub={`conf ${(results.kConf * 100)?.toFixed(1)}%`} />
          <StatBox v={results.sigCount} l="Significant (M≥5)" color="#FF4040" sub="major events" />
          <StatBox v={results.sigQuakes[0] ? `M${results.sigQuakes[0].magnitude}` : "—"} l="Largest" color="#FF4040" sub={(results.sigQuakes[0]?.place ?? "").slice(0, 24)} />
        </div>

        <div style={{ marginBottom: 14 }}>
          <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.1em", color: "#384050", marginBottom: 8 }}>GLOBAL SEISMICITY MAP — last 30 days (size × magnitude)</div>
          <div style={{ background: "rgba(0,0,0,0.3)", border: "1px solid rgba(255,255,255,0.04)", borderRadius: 10, overflow: "hidden" }}>
            <div ref={mapRef} style={{ width: "100%", height: 360 }} />
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, marginBottom: 14 }}>
          <div>
            <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.1em", color: "#384050", marginBottom: 8 }}>MAGNITUDE DISTRIBUTION</div>
            <div style={{ background: "rgba(255,255,255,0.01)", border: "1px solid rgba(255,255,255,0.04)", borderRadius: 10, padding: 8 }}>
              <div ref={histRef} style={{ width: "100%", height: 220 }} />
            </div>
          </div>
          <div>
            <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.1em", color: "#384050", marginBottom: 8 }}>SIGNIFICANT QUAKES (M≥5) — GIGI query result</div>
            <div style={{ background: "rgba(255,255,255,0.01)", border: "1px solid rgba(255,255,255,0.04)", borderRadius: 10, maxHeight: 240, overflowY: "auto" }}>
              {results.sigQuakes.length === 0 ? (
                <div style={{ padding: 20, fontSize: 12, color: "#384050", textAlign: "center" }}>No M≥5.0 quakes this period</div>
              ) : (
                <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11 }}>
                  <thead><tr>
                    {["Mag", "Depth", "Location"].map((h, i) => (
                      <th key={i} style={{ textAlign: i === 0 ? "center" : "left", padding: "8px 10px", borderBottom: "1px solid rgba(255,255,255,0.05)", color: "#506070", fontSize: 9.5, fontWeight: 600, textTransform: "uppercase" }}>{h}</th>
                    ))}
                  </tr></thead>
                  <tbody>{results.sigQuakes.slice(0, 12).map((q, i) => (
                    <tr key={i} style={{ background: i % 2 === 0 ? "rgba(255,255,255,0.008)" : "transparent" }}>
                      <td style={{ padding: "5px 10px", textAlign: "center", fontWeight: 800, color: q.magnitude >= 6 ? "#FF4040" : "#E8A830", fontFamily: "monospace" }}>M{q.magnitude}</td>
                      <td style={{ padding: "5px 10px", color: "#607080", fontFamily: "monospace" }}>{q.depth_km}km</td>
                      <td style={{ padding: "5px 10px", color: "#A0B0C0", fontSize: 10.5 }}>{(q.place ?? "").slice(0, 45)}</td>
                    </tr>
                  ))}</tbody>
                </table>
              )}
            </div>
          </div>
        </div>

        <GqlBox queries={results.queries} />
      </>)}
    </div>
  );
}

// ─────────────────────────────────────────
// DEMO 4 — Live Sensor Stream (WebSocket)
// ─────────────────────────────────────────
const N_SENSORS = 5;
const WARMUP_RECORDS = 30;  // skip plotting until K stabilizes
const WS_BASE = import.meta.env.DEV ? "ws://localhost:3142" : "wss://gigi-stream.fly.dev";

function mkReading(seqId, sensorId, anomaly = false) {
  return {
    seq_id: seqId,
    sensor_id: sensorId,
    ts_ms: Date.now(),
    temp_c: anomaly ? 140 + Math.random() * 30 : 22 + (Math.random() - 0.5) * 8,
    pressure_hpa: anomaly ? 1900 + Math.random() * 200 : 1013 + (Math.random() - 0.5) * 30,
    vibration_g: anomaly ? 8 + Math.random() * 4 : 0.10 + Math.random() * 0.08,
    rpm: anomaly ? 8500 + Math.random() * 1000 : 3000 + (Math.random() - 0.5) * 500,
    signal: anomaly ? "spike" : "normal",
  };
}

function StreamingDemo() {
  const chartRef = useRef(null);
  const [log, setLog] = useState([]);
  const [stage, setStage] = useState("idle");
  const [stats, setStats] = useState(null);
  const wsRef = useRef(null);
  const intervalRef = useRef(null);
  const bundleRef = useRef(null);
  const seqRef = useRef(0);
  const anomalyCountRef = useRef(0);
  const injectedCountRef = useRef(0);   // client-side injections count
  const chartReadyRef = useRef(false);
  const t0Ref = useRef(0);
  const prevKRef = useRef(null);
  const spikeCooldownRef = useRef(0);   // debounce: skip N events after a spike
  const add = (text, color) => setLog(p => [...p.slice(-80), { text, color }]);

  useEffect(() => () => stopStream(true), []);

  function stopStream(silent = false) {
    if (intervalRef.current) { clearInterval(intervalRef.current); intervalRef.current = null; }
    if (wsRef.current) { wsRef.current.close(); wsRef.current = null; }
    if (bundleRef.current && !silent) rd(`/v1/bundles/${bundleRef.current}`).catch(() => {});
    bundleRef.current = null;
    chartReadyRef.current = false;
    seqRef.current = 0;
    anomalyCountRef.current = 0;
    injectedCountRef.current = 0;
    prevKRef.current = null;
    spikeCooldownRef.current = 0;
  }

  async function startStream() {
    setStage("streaming");
    setLog([]);
    setStats(null);
    if (chartRef.current && window.Plotly) Plotly.purge(chartRef.current);
    const name = `sensor_${Date.now()}`;
    bundleRef.current = name;
    t0Ref.current = Date.now();

    try {
      add(`Creating bundle '${name}'…`, "#607080");
      await rd(`/v1/bundles/${name}`).catch(() => {});
      await rp("/v1/bundles", {
        name,
        schema: {
          fields: {
            seq_id: "numeric", sensor_id: "numeric", ts_ms: "numeric",
            temp_c: "numeric", pressure_hpa: "numeric",
            vibration_g: "numeric", rpm: "numeric", signal: "categorical",
          },
          keys: ["seq_id"],
        },
      });
      add("Bundle ready. Initialising chart…", "#607080");

      // Init chart BEFORE opening WS — avoids blocking the onopen handler
      if (chartRef.current && window.Plotly) {
        Plotly.newPlot(chartRef.current, [
          { type: "scatter", mode: "lines", name: "K (curvature)", x: [], y: [],
            line: { color: G, width: 2 } },
          { type: "scatter", mode: "markers", name: "⚠ Spike", x: [], y: [],
            marker: { color: "#FF4040", size: 11, symbol: "x",
                      line: { width: 2.5, color: "#fff" } } },
        ], {
          paper_bgcolor: "transparent",
          plot_bgcolor: "rgba(255,255,255,0.015)",
          font: { family: "DM Sans, sans-serif", color: "#e2e8f0", size: 11 },
          margin: { t: 20, b: 46, l: 76, r: 20 },
          xaxis: { gridcolor: "rgba(255,255,255,0.04)",
                   title: { text: "seconds elapsed", font: { size: 11 } } },
          yaxis: { gridcolor: "rgba(255,255,255,0.04)",
                   title: { text: "K (curvature)", font: { size: 11 } } },
          legend: { orientation: "h", y: 1.06, x: 0.5, xanchor: "center",
                    font: { size: 10 } },
        }, { displayModeBar: false, responsive: true });
        chartReadyRef.current = true;
      }

      add("Opening WebSocket…", "#607080");
      const ws = new WebSocket(`${WS_BASE}/v1/ws/${name}/dashboard`);
      wsRef.current = ws;

      ws.onopen = () => {
        add(`● WS open → /v1/ws/${name}/dashboard`, G);
        add(`Streaming ${N_SENSORS} sensors × 400ms…`, "#607080");
        intervalRef.current = setInterval(async () => {
          if (!bundleRef.current) return;
          const recs = Array.from({ length: N_SENSORS }, (_, i) =>
            mkReading(seqRef.current++, i));
          rp(`/v1/bundles/${name}/insert`, { records: recs }).catch(() => {});
        }, 400);
      };

      ws.onmessage = (e) => {
        try {
          const ev = JSON.parse(e.data);
          const t = +((ev.ts_ms - t0Ref.current) / 1000).toFixed(2);
          const K = ev.k_global ?? 0;
          const n = ev.record_count ?? 0;

          // Spike detection: k_global exceeds the server's own 2σ threshold.
          // The server's per-insert anomaly flag (is_anomaly) relies on running
          // field-stat ranges which adapt after many injections.
          // Using k_global vs k_threshold_2s on every event is more reliable.
          spikeCooldownRef.current = Math.max(0, spikeCooldownRef.current - 1);
          const thresh = ev.k_threshold_2s ?? 0;
          const kStd = ev.k_std ?? 0;
          const isSpike = n > WARMUP_RECORDS &&
            kStd > 1e-4 &&            // stats meaningful
            thresh > 1e-4 &&           // threshold meaningful
            K > thresh &&              // k_global above 2σ line
            spikeCooldownRef.current === 0;

          if (isSpike || ev.is_anomaly) {
            anomalyCountRef.current += 1;
            spikeCooldownRef.current = 8; // skip ~3s worth of events
            const z = ev.z_score ? `  z=${ev.z_score.toFixed(2)}` : "";
            const cf = ev.contributing_fields?.length
              ? `  [${ev.contributing_fields.join(", ")}]` : "";
            add(`\u26a0 K spike  K=${K.toFixed(4)} > 2\u03c3=${thresh.toFixed(4)}  n=${n}${z}${cf}`, "#FF4040");
          }

          setStats({
            K, kMean: ev.k_mean, kStd: ev.k_std,
            kThresh: Number.isFinite(ev.k_threshold_2s) && Math.abs(ev.k_threshold_2s) < 1e9
              ? ev.k_threshold_2s : null,
            count: n, conf: ev.global_confidence,
            anomalyCount: anomalyCountRef.current,
            injectedCount: injectedCountRef.current,
          });

          // Only plot once K has stabilized (skip warm-up period)
          // Also discard nonsensical values from near-empty bundle
          if (chartReadyRef.current && chartRef.current && window.Plotly &&
              n > WARMUP_RECORDS && Math.abs(K) < 1e6) {
            Plotly.extendTraces(chartRef.current, { x: [[t]], y: [[K]] }, [0], 200);
            if (isSpike || ev.is_anomaly) {
              Plotly.extendTraces(chartRef.current, { x: [[t]], y: [[K]] }, [1], 200);
            }
          }
        } catch (_) {}
      };

      ws.onerror = () => add("WS error", "#FF6060");
      ws.onclose = () => { if (bundleRef.current) add("WS closed", "#404050"); };
    } catch (e) {
      add(`ERROR: ${e.message}`, "#FF4040");
      setStage("idle");
      stopStream(true);
    }
  }

  function handleStop() {
    stopStream(false);
    setStage("idle");
    add("Stream stopped. Bundle deleted.", "#607080");
  }

  async function injectAnomaly() {
    if (!bundleRef.current) return;
    injectedCountRef.current += 1;
    // Send each record individually — server only runs anomaly detection for
    // single-record inserts (batch inserts skip the pre-insert K check).
    await Promise.all(
      Array.from({ length: N_SENSORS }, (_, i) =>
        rp(`/v1/bundles/${bundleRef.current}/insert`,
          { records: [mkReading(seqRef.current++, i, true)] }).catch(() => {})
      )
    );
    add(`⚡ Injection #${injectedCountRef.current} — ${N_SENSORS} extreme records sent`, "#E8A830");
  }

  return (
    <div>
      <div style={{ display: "flex", gap: 12, alignItems: "flex-start", flexWrap: "wrap", marginBottom: 16 }}>
        <div style={{ flex: 1, minWidth: 260 }}>
          <div style={{ fontSize: 20, fontWeight: 800, color: "#E0E8F0", marginBottom: 6 }}>📡 Live Sensor Stream</div>
          <div style={{ fontSize: 12.5, color: "#607080", lineHeight: 1.65 }}>
            {N_SENSORS} simulated IoT sensors insert readings every 400ms. A{" "}
            <strong style={{ color: G }}>WebSocket</strong> at{" "}
            <code style={{ color: "#E8A830", fontSize: 11 }}>/v1/ws/&#123;bundle&#125;/dashboard</code>{" "}
            pushes curvature updates back in real time as each batch lands.{" "}
            Hit <strong style={{ color: "#FF4040" }}>Inject Anomaly</strong> to send extreme
            sensor readings and watch <strong style={{ color: G }}>K spike</strong> instantly.
            No polling. No client-side math.
          </div>
        </div>
        <div style={{ display: "flex", gap: 8, flexDirection: "column", alignItems: "flex-end" }}>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            {stage === "idle" && (
              <RunBtn onClick={startStream} stage="idle" labels={["▶ Start Stream", "", "▶ Start Stream"]} />
            )}
            {stage === "streaming" && (<>
              <button onClick={injectAnomaly} style={{
                padding: "11px 16px", borderRadius: 8, border: "none",
                cursor: "pointer", background: "#FF4040", color: "#fff",
                fontSize: 13, fontWeight: 800,
              }}>⚡ Inject Anomaly</button>
              <button onClick={handleStop} style={{
                padding: "11px 20px", borderRadius: 8,
                border: "1px solid rgba(255,255,255,0.1)",
                cursor: "pointer", background: "transparent",
                color: "#A0B0C0", fontSize: 13, fontWeight: 700,
              }}>■ Stop</button>
            </>)}
          </div>
          {stage === "streaming" && (
            <div style={{ fontSize: 10, color: G, fontFamily: "monospace" }}>
              ● LIVE
            </div>
          )}
        </div>
      </div>

      {log.length > 0 && <DemoLog lines={log} />}

      {stats && (
        <div style={{ display: "grid", gridTemplateColumns: "repeat(4,1fr)", gap: 10, marginBottom: 14 }}>
          <StatBox
            v={`K=${stats.K.toFixed(5)}`}
            l="Live Curvature"
            color={stats.kThresh != null && stats.K > stats.kThresh ? "#FF4040" : G}
            sub={`conf ${(stats.conf * 100).toFixed(1)}%`}
          />
          <StatBox v={stats.count.toLocaleString()} l="Records Ingested" sub="live total" />
          <StatBox v={stats.anomalyCount} l="K Spikes" color="#FF4040" sub="K > 2σ threshold" />
          <StatBox v={stats.injectedCount ?? 0} l="Injections" color="#E8A830"
            sub={`${N_SENSORS} records/shot`} />
        </div>
      )}

      <div style={{
        background: "rgba(255,255,255,0.01)",
        border: "1px solid rgba(255,255,255,0.04)",
        borderRadius: 10, padding: 8, marginBottom: 12,
      }}>
        <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.1em",
          color: "#384050", marginBottom: 6, padding: "0 4px" }}>
          LIVE K(t) — curvature updated via WebSocket push (no polling)
        </div>
        {stage === "idle" && !stats ? (
          <div style={{ textAlign: "center", padding: "60px 0",
            fontSize: 12, color: "#252535", fontFamily: "monospace" }}>
            start stream →
          </div>
        ) : (
          <div ref={chartRef} style={{ width: "100%", height: 280 }} />
        )}
      </div>

      <GqlBox queries={[
        `wss://gigi-stream.fly.dev/v1/ws/{bundle}/dashboard`,
        `→ push: { type, k_global, k_mean, k_std, k_threshold_2s, is_anomaly, z_score, contributing_fields }`,
        `POST /v1/bundles/{bundle}/insert  (5 records × every 400ms)`,
      ]} />
    </div>
  );
}

// ─────────────────────────────────────────
// Main LiveDemosPage
// ─────────────────────────────────────────
const DEMOS = [
  {
    id: "btc",
    icon: "₿",
    title: "Bitcoin Crash Detector",
    tag: "LIVE  CoinGecko API",
    tagColor: "#F7931A",
    desc: "365 days BTC price → GIGI → curvature detects volatility regimes. Crash days isolated by geometry.",
  },
  {
    id: "music",
    icon: "🎵",
    title: "Music DNA",
    tag: "40 ARTISTS  8 GENRE DIMS",
    tagColor: "#818cf8",
    desc: "Artists as genre vectors on a fiber bundle. GIGI returns similar artists via geometric neighborhood query.",
  },
  {
    id: "quake",
    icon: "🌍",
    title: "USGS Earthquake Feed",
    tag: "LIVE  USGS GeoJSON",
    tagColor: "#FF6040",
    desc: "Live seismic data, updated every 5 minutes. Curvature detects active fault zones. World map, magnitude histogram.",
  },
  {
    id: "stream",
    icon: "📡",
    title: "Live Sensor Stream",
    tag: "WEBSOCKET  REAL-TIME K(t)",
    tagColor: G,
    desc: "5 IoT sensors stream readings every 400ms. WebSocket pushes curvature updates live. Inject anomalies to spike K instantly.",
  },
];

export default function LiveDemosPage() {
  const [active, setActive] = useState("btc");

  return (
    <div style={{ maxWidth: 1100, margin: "0 auto", padding: "32px 24px 64px" }}>
      {/* Header */}
      <div style={{ marginBottom: 28 }}>
        <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: "0.18em", color: "#E8A830", marginBottom: 6, fontFamily: "monospace" }}>LIVE DEMO GALLERY</div>
        <h1 style={{ fontSize: 28, fontWeight: 900, color: "#E0E8F0", margin: "0 0 8px", letterSpacing: "-0.02em" }}>Real data. Real GIGI. Live results.</h1>
        <p style={{ fontSize: 13.5, color: "#607080", margin: 0, maxWidth: 660, lineHeight: 1.65 }}>
          Four production demos hitting the live GIGI Rust engine at <strong style={{ color: G }}>gigi-stream.fly.dev</strong>. Real data, real-time WebSocket, live curvature. Every chart is GIGI query output — no pre-computed data.
        </p>
      </div>

      {/* Demo selector cards */}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(4,1fr)", gap: 12, marginBottom: 28 }}>
        {DEMOS.map(d => (
          <div key={d.id} onClick={() => setActive(d.id)} style={{
            background: active === d.id ? "rgba(64,232,160,0.04)" : "rgba(255,255,255,0.015)",
            border: active === d.id ? `1px solid ${G}33` : "1px solid rgba(255,255,255,0.04)",
            borderRadius: 10, padding: "16px 16px", cursor: "pointer", transition: "all 0.15s",
          }}>
            <div style={{ fontSize: 24, marginBottom: 6 }}>{d.icon}</div>
            <div style={{ fontSize: 14, fontWeight: 800, color: active === d.id ? G : "#C0D0E0", marginBottom: 4 }}>{d.title}</div>
            <div style={{ fontSize: 9.5, fontWeight: 700, letterSpacing: "0.06em", color: d.tagColor, marginBottom: 8 }}>{d.tag}</div>
            <div style={{ fontSize: 11.5, color: "#506070", lineHeight: 1.5 }}>{d.desc}</div>
          </div>
        ))}
      </div>

      {/* Divider */}
      <div style={{ borderBottom: "1px solid rgba(255,255,255,0.04)", marginBottom: 28 }} />

      {/* Active demo */}
      {active === "btc" && <BitcoinDemo />}
      {active === "music" && <MusicDNADemo />}
      {active === "quake" && <EarthquakeDemo />}
      {active === "stream" && <StreamingDemo />}

      {/* Footer note */}
      <div style={{ marginTop: 40, padding: "16px 0", borderTop: "1px solid rgba(255,255,255,0.03)", textAlign: "center", fontSize: 10.5, color: "#303040", fontFamily: "monospace" }}>
        All math runs on{" "}
        <a href="https://github.com/nurdymuny/gigi" target="_blank" rel="noopener noreferrer" style={{ color: G, textDecoration: "none" }}>GIGI's Rust engine</a>
        {" "}·  This page is a display shell — zero math in JSX  ·  Wire format:{" "}
        <a href="https://dhoom.dev" target="_blank" rel="noopener noreferrer" style={{ color: "#E8A830", textDecoration: "none" }}>DHOOM</a>
      </div>
    </div>
  );
}
