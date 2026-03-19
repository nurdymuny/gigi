import { useState, useRef, useCallback, useEffect } from "react";

const C = {
  bg: "#07080C", panel: "#0C0D14", border: "#1A1C28", accent: "#40E8A0",
  accent2: "#E8A830", warn: "#FF6040", dim: "#384050", text: "#C0C8D0",
  muted: "#606878", kw: "#40E8A0", type: "#60A0FF", str: "#E8A830",
  num: "#FF8060", op: "#A080E0", comment: "#404858", field: "#80C0E0",
  line: "#1E2030", cursor: "#40E8A0", selection: "rgba(64,232,160,0.08)"
};

const FONT = "'JetBrains Mono','Fira Code','SF Mono',monospace";
const API_URL = import.meta.env.DEV ? "http://localhost:3142" : "/api/gigi-playground";
const API_LABEL = import.meta.env.DEV ? "localhost:3142" : "gigi-playground.fly.dev";

// ─── GQL Engine (real server) ───
async function executeGQL(query) {
  const q = query.trim().replace(/;$/, "").trim();
  if (!q) return { type: "error", message: "Empty query" };
  const start = performance.now();
  try {
    const res = await fetch(`${API_URL}/v1/gql`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ query: q }),
    });
    const data = await res.json();
    const elapsed = performance.now() - start;

    if (!res.ok || data.error) {
      return { type: "error", message: data.error || `HTTP ${res.status}`, time: elapsed };
    }

    // Route response to the right display type based on what the server returned
    if (data.rows !== undefined) {
      // Rows response: {"rows": [...], "count": n}
      const rows = data.rows || [];
      if (rows.length === 0) {
        return { type: "scalar", title: q.split(/\s+/).slice(0, 4).join(" "), time: elapsed,
          value: "∅ (no results)", meta: { count: 0 } };
      }
      const columns = Object.keys(rows[0]);
      return {
        type: "table", title: q.split(/\s+/).slice(0, 6).join(" "), time: elapsed,
        columns, rows: rows.map(r => columns.map(c => r[c] ?? "")),
        meta: { records: data.count }
      };
    }

    if (data.bundles !== undefined) {
      // Bundles list: {"bundles": [...]}
      const bundles = data.bundles || [];
      if (bundles.length === 0) {
        return { type: "scalar", title: "SHOW BUNDLES", time: elapsed,
          value: "No bundles", meta: {} };
      }
      const columns = Object.keys(bundles[0]);
      return {
        type: "table", title: "SHOW BUNDLES", time: elapsed,
        columns, rows: bundles.map(b => columns.map(c => b[c] ?? "")),
        meta: { count: bundles.length }
      };
    }

    if (data.curvature !== undefined) {
      // Stats response: {"curvature": ..., "confidence": ..., ...}
      return {
        type: "health", title: q.split(/\s+/).slice(0, 3).join(" "), time: elapsed,
        data: {
          records: data.record_count,
          storage: data.storage_mode,
          global_K: data.curvature,
          confidence: data.confidence,
          base_fields: data.base_fields,
          fiber_fields: data.fiber_fields,
        }
      };
    }

    if (data.value !== undefined) {
      // Scalar/Bool response: {"value": ...}
      return {
        type: "scalar", title: q.split(/\s+/).slice(0, 4).join(" "), time: elapsed,
        value: String(data.value), meta: {}
      };
    }

    if (data.affected !== undefined) {
      // Count response: {"affected": n}
      return {
        type: "scalar", title: q.split(/\s+/).slice(0, 4).join(" "), time: elapsed,
        value: `${data.affected} affected`, meta: {}
      };
    }

    if (data.status === "ok") {
      return {
        type: "scalar", title: q.split(/\s+/).slice(0, 4).join(" "), time: elapsed,
        value: "OK ✓", meta: {}
      };
    }

    // Fallback
    return {
      type: "scalar", title: "Response", time: elapsed,
      value: JSON.stringify(data, null, 2), meta: {}
    };
  } catch (e) {
    return {
      type: "error",
      message: e.message.includes("fetch") || e.message.includes("Failed")
        ? `Cannot reach GIGI server at ${API_URL}\nMake sure gigi_stream is running.`
        : e.message,
      time: performance.now() - start,
    };
  }
}

// ─── Syntax Highlighting ───
const KEYWORDS = ["BUNDLE","SECTION","SECTIONS","COVER","INTEGRATE","PULLBACK","GAUGE","COLLAPSE","LENS","REDEFINE","RETRACT","CURVATURE","CONFIDENCE","SPECTRAL","HOLONOMY","CONSISTENCY","PARTITION","HEALTH","PREDICT","TRANSPORT","FLOW","GEODESIC","RESTRICT","GLUE","CALIBRATE","SNAPSHOT","DIFF","SUBSCRIBE","EMIT","TRANSLATE","EXPLAIN","DESCRIBE","SHOW","ATLAS","OUTLIER","PROFILE","ENTROPY","DIVERGENCE","FISHER","MUTUAL","BETTI","EULER","SCALAR","RICCI","SECTIONAL","DEVIATION","TREND","CAPACITY","BOTTLENECK","CLUSTER","MIXING","CONDUCTANCE","LAPLACIAN","PHASE","CRITICAL","FREEENERGY","TEMPERATURE","WILSON","SIMILAR","CORRELATE","SEGMENT","TRIVIALIZE","CHARACTERISTIC","DOUBLECOVER","RECALL","COMPLETENESS","COCYCLE","COBOUNDARY","EXISTS","VERIFY","UPSERT","WEAVE","UNWEAVE","GRANT","REVOKE","POLICY","AUDIT","COMPACT","ANALYZE","VACUUM","REBUILD","CHECK","REPAIR","STORAGE","INGEST","TRANSPLANT","GENERATE","FILL","PREPARE","EXECUTE","DEALLOCATE","BACKUP","RESTORE","ITERATE","COMMENT","DROP","TRIGGER","BEFORE","AFTER","RESET"];
const CLAUSES = ["ON","AT","WHERE","OVER","MEASURE","ALONG","ONTO","BY","SET","INTO","FROM","TO","WITHIN","AROUND","HAVING","RESTRICT","RANK","FIRST","SKIP","DISTINCT","PROJECT","EMIT","CHECK","PRESERVE","WINDOW","SHIFT","COARSEN","LEVELS","TOLERANCE","TRAIN","TEST","BEFORE","AFTER","DRIFT","SIGMA","NEAR","MATCHES","VOID","DEFINED","REPAIR","FULL","VERBOSE","ALL","MATERIALIZE","REFRESH","CONFIRM","AGAINST","PASSWORD","INHERITS","SUPERWEAVE","ROLE","FORMAT","USING","STEP","START","UNTIL","DEPTH","SOURCE","COMPRESS","INCREMENTAL","SINCE","SNAPSHOT","AS","IS","INDEX","CONSTRAIN","UNCONSTRAIN","CASCADE","MORPHISM"];
const TYPES = ["NUMERIC","CATEGORICAL","TEXT","BOOLEAN","TIMESTAMP","BASE","FIBER","INDEX","RANGE","DEFAULT","AUTO","ARITHMETIC","UNIQUE","REQUIRED","NULLABLE"];
const VALUES = ["TRUE","FALSE","ASC","DESC","DHOOM","JSON","CSV","JSONL","SQL","FLAT","CURVED","LEFT","BUNDLES","OVERLAP","UNBOUNDED","STDIN","LINEAR","TRANSPORT"];
const FUNCS = ["avg","sum","count","min","max","stddev","variance","median","mode","percentile","ABS","ROUND","CEIL","FLOOR","TRUNC","POWER","SQRT","LOG","EXP","MOD","SIGN","UPPER","LOWER","LENGTH","SUBSTR","CONCAT","REPLACE","TRIM","REVERSE","NOW","TODAY","EPOCH","DATEPART","DATEADD","DATEDIFF","CAST","IF","RESOLVE","VOIDIF","GREATEST","LEAST","ARRAY","ARRAY_AGG","ARRAY_LENGTH","MD5","PI","RANDOM"];

function highlight(text) {
  return text.split("\n").map(line => {
    if (line.trim().startsWith("--")) return `<span style="color:${C.comment}">${esc(line)}</span>`;
    let result = "";
    const tokens = line.split(/(\s+|[(),;:=<>!'])/);
    let inString = false;
    for (const tok of tokens) {
      if (tok === "'" && !inString) { inString = true; result += `<span style="color:${C.str}">'`; continue; }
      if (tok === "'" && inString) { inString = false; result += `'</span>`; continue; }
      if (inString) { result += esc(tok); continue; }
      const up = tok.toUpperCase();
      if (KEYWORDS.includes(up)) result += `<span style="color:${C.kw};font-weight:700">${esc(tok)}</span>`;
      else if (CLAUSES.includes(up)) result += `<span style="color:${C.op}">${esc(tok)}</span>`;
      else if (TYPES.includes(up)) result += `<span style="color:${C.type}">${esc(tok)}</span>`;
      else if (VALUES.includes(up)) result += `<span style="color:${C.num}">${esc(tok)}</span>`;
      else if (FUNCS.includes(tok) || FUNCS.includes(up)) result += `<span style="color:${C.accent2}">${esc(tok)}</span>`;
      else if (/^-?\d+\.?\d*$/.test(tok)) result += `<span style="color:${C.num}">${esc(tok)}</span>`;
      else if (/^[(),;=<>!]+$/.test(tok)) result += `<span style="color:${C.dim}">${esc(tok)}</span>`;
      else result += `<span style="color:${C.field}">${esc(tok)}</span>`;
    }
    return result;
  }).join("\n");
}
function esc(s) { return s.replace(/&/g,"&amp;").replace(/</g,"&lt;").replace(/>/g,"&gt;"); }

// ─── Result Renderers ───
function ResultView({ result }) {
  if (!result) return <div style={{ color: C.muted, fontSize: 13, padding: 20, fontStyle: "italic" }}>Run a query to see results. Try the examples below.</div>;
  const meta = result.meta || {};
  const hdr = { fontSize: 11, fontWeight: 700, color: C.accent, letterSpacing: "0.06em", marginBottom: 8 };
  const metaStyle = { fontSize: 11, color: C.muted, fontFamily: FONT, marginTop: 8, lineHeight: 1.6 };
  const timeStr = result.time !== undefined ? `${result.time.toFixed(2)}ms` : "";

  if (result.type === "error") return (
    <div style={{ padding: 16 }}>
      <div style={hdr}>ERROR</div>
      <pre style={{ color: C.warn, fontSize: 12, fontFamily: FONT, whiteSpace: "pre-wrap", margin: 0 }}>{result.message}</pre>
    </div>
  );

  if (result.type === "scalar") return (
    <div style={{ padding: 16 }}>
      <div style={{ display: "flex", justifyContent: "space-between" }}>
        <div style={hdr}>{result.title}</div>
        <span style={{ fontSize: 10, color: C.dim, fontFamily: FONT }}>{timeStr}</span>
      </div>
      <div style={{ fontSize: 28, fontWeight: 900, color: C.accent, fontFamily: FONT, margin: "8px 0", whiteSpace: "pre-wrap" }}>{result.value}</div>
      {Object.entries(meta).map(([k, v]) => <div key={k} style={metaStyle}><span style={{ color: C.dim }}>{k}:</span> <span style={{ color: C.text }}>{typeof v === "object" ? JSON.stringify(v) : String(v)}</span></div>)}
    </div>
  );

  if (result.type === "table") return (
    <div style={{ padding: 16 }}>
      <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 6 }}>
        <div style={hdr}>{result.title}</div>
        <span style={{ fontSize: 10, color: C.dim, fontFamily: FONT }}>{timeStr}</span>
      </div>
      <div style={{ overflowX: "auto" }}>
        <table style={{ width: "100%", borderCollapse: "collapse", fontFamily: FONT, fontSize: 11.5 }}>
          <thead><tr>{result.columns.map((c, i) => <th key={i} style={{ textAlign: "left", padding: "5px 8px", borderBottom: `1px solid ${C.border}`, color: C.dim, fontWeight: 600, fontSize: 10, textTransform: "uppercase" }}>{c}</th>)}</tr></thead>
          <tbody>{result.rows.map((row, i) => <tr key={i} style={{ background: i % 2 === 0 ? "transparent" : "rgba(255,255,255,0.01)" }}>
            {row.map((v, j) => <td key={j} style={{ padding: "4px 8px", borderBottom: `1px solid ${C.border}08`, color: j === 0 ? C.field : typeof v === "number" || /^-?\d/.test(String(v)) ? C.num : C.text }}>{String(v)}</td>)}
          </tr>)}</tbody>
        </table>
      </div>
      {Object.keys(meta).length > 0 && <div style={{ display: "flex", gap: 16, marginTop: 8, flexWrap: "wrap" }}>
        {Object.entries(meta).map(([k, v]) => <div key={k} style={{ fontSize: 10.5, fontFamily: FONT }}><span style={{ color: C.dim }}>{k}: </span><span style={{ color: C.accent }}>{String(v)}</span></div>)}
      </div>}
    </div>
  );

  if (result.type === "health") return (
    <div style={{ padding: 16 }}>
      <div style={{ display: "flex", justifyContent: "space-between" }}>
        <div style={hdr}>{result.title}</div>
        <span style={{ fontSize: 10, color: C.dim, fontFamily: FONT }}>{timeStr}</span>
      </div>
      {Object.entries(result.data).map(([k, v]) => (
        <div key={k} style={{ display: "flex", padding: "3px 0", fontFamily: FONT, fontSize: 12 }}>
          <span style={{ color: C.dim, width: 140, flexShrink: 0 }}>{k}:</span>
          <span style={{ color: typeof v === "boolean" ? (v ? C.accent : C.warn) : C.text, fontWeight: 600 }}>{typeof v === "number" ? (Number.isInteger(v) ? v : v.toFixed(6)) : String(v)}</span>
        </div>
      ))}
    </div>
  );

  return <div style={{ padding: 16, color: C.muted }}>Unknown result type</div>;
}

// ─── Bootstrap: seed sensors bundle if missing ───
const SEED_BUNDLE = "BUNDLE sensors BASE (id NUMERIC) FIBER (city CATEGORICAL INDEX, region CATEGORICAL INDEX, date NUMERIC, temp NUMERIC RANGE 80, humidity NUMERIC RANGE 100, wind NUMERIC RANGE 30, pressure NUMERIC RANGE 20, status CATEGORICAL DEFAULT 'normal')";
const SEED_DATA = `SECTIONS sensors (id, city, region, date, temp, humidity, wind, pressure, status)
  (1, 'Moscow', 'EU', 20240102, -27.1, 98.0, 3.7, 99.3, 'normal'),
  (2, 'Moscow', 'EU', 20240103, -31.6, 96.1, 2.5, 99.1, 'normal'),
  (3, 'Moscow', 'EU', 20240104, -31.9, 97.4, 2.2, 98.7, 'normal'),
  (4, 'Moscow', 'EU', 20240105, -30.3, 97.8, 1.8, 98.9, 'normal'),
  (5, 'Moscow', 'EU', 20240615, 22.4, 58.2, 3.1, 101.2, 'normal'),
  (6, 'Singapore', 'AS', 20240102, 27.3, 82.1, 1.8, 101.0, 'normal'),
  (7, 'Singapore', 'AS', 20240103, 27.8, 80.4, 2.1, 101.1, 'normal'),
  (8, 'Singapore', 'AS', 20240104, 28.1, 79.8, 1.5, 101.0, 'normal'),
  (9, 'Toronto', 'NA', 20240113, 0.9, 75.2, 11.0, 100.1, 'normal'),
  (10, 'Toronto', 'NA', 20240114, -5.5, 68.4, 11.4, 99.8, 'alert'),
  (11, 'Toronto', 'NA', 20240615, 24.2, 62.1, 3.8, 101.5, 'normal'),
  (12, 'Cape_Town', 'SH', 20240407, 16.3, 72.0, 11.4, 101.8, 'normal'),
  (13, 'Cape_Town', 'SH', 20240707, 13.0, 78.5, 11.4, 102.1, 'normal'),
  (14, 'Cape_Town', 'SH', 20240827, 13.6, 76.2, 11.2, 101.9, 'normal'),
  (15, 'Beijing', 'AS', 20240115, -8.2, 42.1, 4.5, 102.8, 'normal'),
  (16, 'Beijing', 'AS', 20240715, 33.1, 68.3, 2.1, 100.2, 'normal')`;

async function bootstrapSensors() {
  try {
    const res = await fetch(`${API_URL}/v1/gql`, {
      method: "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ query: "SHOW BUNDLES" }),
    });
    const data = await res.json();
    const hasSensors = (data.bundles || []).some(b => b.name === "sensors");
    if (hasSensors) return "ready";

    // Create bundle
    await fetch(`${API_URL}/v1/gql`, {
      method: "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ query: SEED_BUNDLE }),
    });
    // Seed data
    await fetch(`${API_URL}/v1/gql`, {
      method: "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ query: SEED_DATA }),
    });
    return "seeded";
  } catch {
    return "offline";
  }
}

// ─── Example Queries ───
const EXAMPLES = [
  { label: "Show Bundles", q: "SHOW BUNDLES;" },
  { label: "Create Bundle", q: "BUNDLE sensors BASE (id NUMERIC) FIBER (city CATEGORICAL INDEX, region CATEGORICAL INDEX, date NUMERIC, temp NUMERIC RANGE 80, humidity NUMERIC RANGE 100, wind NUMERIC RANGE 30, pressure NUMERIC RANGE 20, status CATEGORICAL DEFAULT 'normal');" },
  { label: "Insert", q: "SECTION sensors (id: 1, city: 'Moscow', region: 'EU', date: 20240102, temp: -27.1, humidity: 98.0, wind: 3.7, pressure: 99.3, status: 'normal');" },
  { label: "Batch Insert", q: "SECTIONS sensors (id, city, region, date, temp, humidity, wind, pressure, status)\n  (2, 'Moscow', 'EU', 20240103, -31.6, 96.1, 2.5, 99.1, 'normal'),\n  (3, 'Moscow', 'EU', 20240104, -31.9, 97.4, 2.2, 98.7, 'normal'),\n  (4, 'Moscow', 'EU', 20240105, -30.3, 97.8, 1.8, 98.9, 'normal'),\n  (5, 'Moscow', 'EU', 20240615, 22.4, 58.2, 3.1, 101.2, 'normal'),\n  (6, 'Singapore', 'AS', 20240102, 27.3, 82.1, 1.8, 101.0, 'normal'),\n  (7, 'Singapore', 'AS', 20240103, 27.8, 80.4, 2.1, 101.1, 'normal'),\n  (8, 'Singapore', 'AS', 20240104, 28.1, 79.8, 1.5, 101.0, 'normal'),\n  (9, 'Toronto', 'NA', 20240113, 0.9, 75.2, 11.0, 100.1, 'normal'),\n  (10, 'Toronto', 'NA', 20240114, -5.5, 68.4, 11.4, 99.8, 'alert'),\n  (11, 'Toronto', 'NA', 20240615, 24.2, 62.1, 3.8, 101.5, 'normal'),\n  (12, 'Cape_Town', 'SH', 20240407, 16.3, 72.0, 11.4, 101.8, 'normal'),\n  (13, 'Cape_Town', 'SH', 20240707, 13.0, 78.5, 11.4, 102.1, 'normal'),\n  (14, 'Cape_Town', 'SH', 20240827, 13.6, 76.2, 11.2, 101.9, 'normal'),\n  (15, 'Beijing', 'AS', 20240115, -8.2, 42.1, 4.5, 102.8, 'normal'),\n  (16, 'Beijing', 'AS', 20240715, 33.1, 68.3, 2.1, 100.2, 'normal');" },
  { label: "Point Query", q: "SECTION sensors AT id=3;" },
  { label: "Range Query", q: "COVER sensors ON city = 'Moscow' WHERE temp < -25;" },
  { label: "All Records", q: "COVER sensors ALL;" },
  { label: "Distinct", q: "COVER sensors DISTINCT city;" },
  { label: "Aggregation", q: "INTEGRATE sensors OVER city MEASURE avg(temp), count(*);" },
  { label: "Curvature", q: "CURVATURE sensors;" },
  { label: "Spectral", q: "SPECTRAL sensors;" },
  { label: "Health", q: "HEALTH sensors;" },
  { label: "Describe", q: "DESCRIBE sensors;" },
  { label: "Update", q: "REDEFINE sensors AT id=1 SET (temp: -28.5);" },
  { label: "Exists", q: "EXISTS SECTION sensors AT id=42;" },
];

// ─── Main App ───
export default function GQLPlayground() {
  const [code, setCode] = useState("-- GQL Playground \u00b7 Davis Geometric\n-- Connected to GIGI Playground at " + API_LABEL + "\n-- Type a query and press Ctrl+Enter or click Run\n\nSHOW BUNDLES;");
  const [result, setResult] = useState(null);
  const [history, setHistory] = useState([]);
  const [running, setRunning] = useState(false);
  const [serverStatus, setServerStatus] = useState("connecting");
  const textareaRef = useRef(null);
  const highlightRef = useRef(null);

  useEffect(() => {
    bootstrapSensors().then(s => {
      setServerStatus(s === "offline" ? "offline" : "connected");
      if (s === "seeded") setResult({ type: "scalar", title: "Auto-Setup", value: "sensors bundle created with 16 records. Try any example!", meta: {} });
    });
  }, []);

  const run = useCallback(async () => {
    setRunning(true);
    const r = await executeGQL(code);
    setResult(r);
    setHistory(h => [{ query: code.trim(), time: new Date().toLocaleTimeString() }, ...h].slice(0, 20));
    setRunning(false);
  }, [code]);

  const handleKeyDown = (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") { e.preventDefault(); run(); }
    if (e.key === "Tab") {
      e.preventDefault();
      const ta = textareaRef.current;
      const start = ta.selectionStart;
      setCode(code.substring(0, start) + "  " + code.substring(ta.selectionEnd));
      setTimeout(() => { ta.selectionStart = ta.selectionEnd = start + 2; }, 0);
    }
  };

  const syncScroll = () => {
    if (highlightRef.current && textareaRef.current)
      highlightRef.current.scrollTop = textareaRef.current.scrollTop;
  };

  return (
    <div style={{ height: "100vh", display: "flex", flexDirection: "column", background: C.bg, color: C.text, fontFamily: "'DM Sans',system-ui,sans-serif" }}>
      <link href="https://fonts.googleapis.com/css2?family=DM+Sans:wght@400;500;600;700;800;900&family=JetBrains+Mono:wght@400;500;600;700&display=swap" rel="stylesheet" />

      {/* Header */}
      <div style={{ height: 48, display: "flex", alignItems: "center", justifyContent: "space-between", padding: "0 16px", borderBottom: `1px solid ${C.border}`, flexShrink: 0, background: C.panel }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <span style={{ fontSize: 15, fontWeight: 900, color: C.accent, letterSpacing: "-0.02em" }}>GQL</span>
          <span style={{ fontSize: 11, color: C.dim }}>Geometric Query Language Playground</span>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span style={{ fontSize: 10, color: serverStatus === "connected" ? C.accent : serverStatus === "offline" ? C.warn : C.accent2, fontFamily: FONT }}>
            {serverStatus === "connected" ? "LIVE" : serverStatus === "offline" ? "OFFLINE" : "CONNECTING..."} {"\u00b7"} {API_LABEL}
          </span>
          <span style={{ fontSize: 10, color: C.dim }}>·</span>
          <span style={{ fontSize: 10, color: C.dim, fontFamily: FONT }}>Davis Geometric · 2026</span>
        </div>
      </div>

      {/* Main split */}
      <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>

        {/* Left: Editor */}
        <div style={{ flex: 1, display: "flex", flexDirection: "column", borderRight: `1px solid ${C.border}` }}>
          <div style={{ height: 32, display: "flex", alignItems: "center", justifyContent: "space-between", padding: "0 12px", background: C.panel, borderBottom: `1px solid ${C.border}` }}>
            <span style={{ fontSize: 10, fontWeight: 700, color: C.dim, letterSpacing: "0.1em" }}>EDITOR</span>
            <div style={{ display: "flex", gap: 6 }}>
              <span style={{ fontSize: 10, color: C.dim, fontFamily: FONT }}>Ctrl+Enter to run</span>
              <button onClick={run} disabled={running} style={{ background: running ? C.dim : C.accent, color: C.bg, border: "none", borderRadius: 4, padding: "3px 12px", fontSize: 10.5, fontWeight: 700, cursor: running ? "wait" : "pointer" }}>
                {running ? "Running..." : "Run"}
              </button>
            </div>
          </div>

          {/* Code editor with overlay highlighting */}
          <div style={{ flex: 1, position: "relative", overflow: "hidden" }}>
            <pre ref={highlightRef} aria-hidden="true" style={{
              position: "absolute", top: 0, left: 0, right: 0, bottom: 0, margin: 0,
              padding: "12px 14px", fontFamily: FONT, fontSize: 13, lineHeight: 1.65,
              overflow: "auto", pointerEvents: "none", whiteSpace: "pre-wrap", wordWrap: "break-word",
              color: C.text, background: "transparent"
            }} dangerouslySetInnerHTML={{ __html: highlight(code) }} />
            <textarea ref={textareaRef} value={code}
              onChange={e => setCode(e.target.value)} onKeyDown={handleKeyDown} onScroll={syncScroll}
              spellCheck={false}
              style={{
                position: "absolute", top: 0, left: 0, width: "100%", height: "100%",
                padding: "12px 14px", fontFamily: FONT, fontSize: 13, lineHeight: 1.65,
                background: "transparent", color: "transparent", caretColor: C.cursor,
                border: "none", outline: "none", resize: "none", whiteSpace: "pre-wrap", wordWrap: "break-word"
              }} />
          </div>

          {/* Examples */}
          <div style={{ borderTop: `1px solid ${C.border}`, padding: "8px 12px", background: C.panel, flexShrink: 0 }}>
            <div style={{ fontSize: 9, fontWeight: 700, color: C.dim, letterSpacing: "0.1em", marginBottom: 6 }}>EXAMPLES</div>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
              {EXAMPLES.map((ex, i) => (
                <button key={i} onClick={() => { setCode(ex.q); }}
                  style={{ background: "rgba(255,255,255,0.03)", border: `1px solid ${C.border}`, borderRadius: 4,
                    padding: "3px 8px", fontSize: 10, color: C.muted, cursor: "pointer", fontFamily: FONT,
                    transition: "all 0.15s" }}
                  onMouseEnter={e => { e.target.style.color = C.accent; e.target.style.borderColor = "rgba(64,232,160,0.2)"; }}
                  onMouseLeave={e => { e.target.style.color = C.muted; e.target.style.borderColor = C.border; }}
                >{ex.label}</button>
              ))}
            </div>
          </div>
        </div>

        {/* Right: Results */}
        <div style={{ flex: 1, display: "flex", flexDirection: "column" }}>
          <div style={{ height: 32, display: "flex", alignItems: "center", padding: "0 12px", background: C.panel, borderBottom: `1px solid ${C.border}` }}>
            <span style={{ fontSize: 10, fontWeight: 700, color: C.dim, letterSpacing: "0.1em" }}>RESULTS</span>
            {result && result.meta && result.meta.records !== undefined && (
              <span style={{ fontSize: 10, color: C.accent, fontFamily: FONT, marginLeft: "auto" }}>{result.meta.records} records</span>
            )}
          </div>
          <div style={{ flex: 1, overflow: "auto" }}>
            <ResultView result={result} />
          </div>

          {/* History */}
          {history.length > 0 && (
            <div style={{ borderTop: `1px solid ${C.border}`, padding: "8px 12px", background: C.panel, maxHeight: 100, overflow: "auto", flexShrink: 0 }}>
              <div style={{ fontSize: 9, fontWeight: 700, color: C.dim, letterSpacing: "0.1em", marginBottom: 4 }}>HISTORY</div>
              {history.slice(0, 5).map((h, i) => (
                <div key={i} onClick={() => setCode(h.query)} style={{ fontSize: 10.5, fontFamily: FONT, color: C.muted, padding: "2px 0", cursor: "pointer", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
                  onMouseEnter={e => e.target.style.color = C.accent}
                  onMouseLeave={e => e.target.style.color = C.muted}
                ><span style={{ color: C.dim, marginRight: 6 }}>{h.time}</span>{h.query.replace(/\n/g, " ").substring(0, 80)}</div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
