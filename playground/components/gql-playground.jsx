import { useState, useRef, useEffect, useCallback } from "react";

const C = {
  bg: "#07080C", panel: "#0C0D14", border: "#1A1C28", accent: "#40E8A0",
  accent2: "#E8A830", warn: "#FF6040", dim: "#384050", text: "#C0C8D0",
  muted: "#606878", kw: "#40E8A0", type: "#60A0FF", str: "#E8A830",
  num: "#FF8060", op: "#A080E0", comment: "#404858", field: "#80C0E0",
  line: "#1E2030", cursor: "#40E8A0", selection: "rgba(64,232,160,0.08)"
};

const FONT = "'JetBrains Mono','Fira Code','SF Mono',monospace";

// ─── Sample Data: NASA Atmospheric Bundle ───
const SENSORS = [
  { id: 1, city: "Moscow", region: "EU", date: 20240102, temp: -27.1, humidity: 98.0, wind: 3.7, pressure: 99.3, solar: 0.8, status: "normal" },
  { id: 2, city: "Moscow", region: "EU", date: 20240103, temp: -31.6, humidity: 96.1, wind: 2.5, pressure: 99.1, solar: 0.7, status: "normal" },
  { id: 3, city: "Moscow", region: "EU", date: 20240104, temp: -31.9, humidity: 97.4, wind: 2.2, pressure: 98.7, solar: 0.8, status: "normal" },
  { id: 4, city: "Moscow", region: "EU", date: 20240105, temp: -30.3, humidity: 97.8, wind: 1.8, pressure: 98.9, solar: 0.8, status: "normal" },
  { id: 5, city: "Moscow", region: "EU", date: 20240615, temp: 22.4, humidity: 58.2, wind: 3.1, pressure: 101.2, solar: 6.8, status: "normal" },
  { id: 6, city: "Singapore", region: "AS", date: 20240102, temp: 27.3, humidity: 82.1, wind: 1.8, pressure: 101.0, solar: 4.2, status: "normal" },
  { id: 7, city: "Singapore", region: "AS", date: 20240103, temp: 27.8, humidity: 80.4, wind: 2.1, pressure: 101.1, solar: 5.1, status: "normal" },
  { id: 8, city: "Singapore", region: "AS", date: 20240104, temp: 28.1, humidity: 79.8, wind: 1.5, pressure: 101.0, solar: 5.4, status: "normal" },
  { id: 9, city: "Toronto", region: "NA", date: 20240113, temp: 0.9, humidity: 75.2, wind: 11.0, pressure: 100.1, solar: 1.9, status: "normal" },
  { id: 10, city: "Toronto", region: "NA", date: 20240114, temp: -5.5, humidity: 68.4, wind: 11.4, pressure: 99.8, solar: 2.1, status: "alert" },
  { id: 11, city: "Toronto", region: "NA", date: 20240615, temp: 24.2, humidity: 62.1, wind: 3.8, pressure: 101.5, solar: 7.2, status: "normal" },
  { id: 12, city: "Cape_Town", region: "SH", date: 20240407, temp: 16.3, humidity: 72.0, wind: 11.4, pressure: 101.8, solar: 4.1, status: "normal" },
  { id: 13, city: "Cape_Town", region: "SH", date: 20240707, temp: 13.0, humidity: 78.5, wind: 11.4, pressure: 102.1, solar: 3.2, status: "normal" },
  { id: 14, city: "Cape_Town", region: "SH", date: 20240827, temp: 13.6, humidity: 76.2, wind: 11.2, pressure: 101.9, solar: 4.8, status: "normal" },
  { id: 15, city: "Beijing", region: "AS", date: 20240115, temp: -8.2, humidity: 42.1, wind: 4.5, pressure: 102.8, solar: 3.9, status: "normal" },
  { id: 16, city: "Beijing", region: "AS", date: 20240715, temp: 33.1, humidity: 68.3, wind: 2.1, pressure: 100.2, solar: 6.1, status: "normal" },
];

// ─── GQL Engine (simulated) ───
function fieldCurvature(vals, range) {
  if (vals.length < 2) return 0;
  const mean = vals.reduce((a, b) => a + b, 0) / vals.length;
  const variance = vals.reduce((a, v) => a + (v - mean) ** 2, 0) / vals.length;
  return variance / (range * range);
}

function executeGQL(query) {
  const q = query.trim().replace(/;$/, "").trim();
  const upper = q.toUpperCase();
  const start = performance.now();

  try {
    // SHOW BUNDLES
    if (upper === "SHOW BUNDLES") {
      return { type: "table", title: "Bundles", time: performance.now() - start,
        columns: ["name", "records", "storage", "K", "confidence"],
        rows: [["sensors", 16, "SEQUENTIAL", "0.0412", "0.9604"]]
      };
    }

    // DESCRIBE
    if (upper.startsWith("DESCRIBE")) {
      const fields = [
        { name: "id", type: "NUMERIC", role: "BASE", mod: "AUTO", K: "—" },
        { name: "city", type: "CATEGORICAL", role: "FIBER", mod: "INDEX", K: "—" },
        { name: "region", type: "CATEGORICAL", role: "FIBER", mod: "INDEX", K: "—" },
        { name: "date", type: "NUMERIC", role: "FIBER", mod: "", K: "—" },
        { name: "temp", type: "NUMERIC", role: "FIBER", mod: "RANGE 80", K: "0.0233" },
        { name: "humidity", type: "NUMERIC", role: "FIBER", mod: "RANGE 100", K: "0.0034" },
        { name: "wind", type: "NUMERIC", role: "FIBER", mod: "RANGE 30", K: "0.0019" },
        { name: "pressure", type: "NUMERIC", role: "FIBER", mod: "RANGE 20", K: "0.0002" },
        { name: "solar", type: "NUMERIC", role: "FIBER", mod: "RANGE 12", K: "0.0015" },
        { name: "status", type: "CATEGORICAL", role: "FIBER", mod: "DEFAULT 'normal'", K: "—" },
      ];
      return { type: "table", title: "DESCRIBE sensors", time: performance.now() - start,
        columns: ["field", "type", "role", "modifiers", "K"],
        rows: fields.map(f => [f.name, f.type, f.role, f.mod, f.K]),
        meta: { records: 16, storage: "SEQUENTIAL", global_K: 0.0412, confidence: 0.9604 }
      };
    }

    // HEALTH
    if (upper.startsWith("HEALTH")) {
      return { type: "health", title: "HEALTH sensors", time: performance.now() - start,
        data: {
          records: 16, storage: "SEQUENTIAL (K=0, array mode)", global_K: 0.0412,
          confidence: 0.9604, capacity: "C = τ/K = 2.43 at τ=0.1",
          spectral_gap: 0.0, components: 4, h1: 0, consistent: true,
          top_anomalies: [
            { city: "Moscow", date: 20240104, z: 5.30, field: "temp" },
            { city: "Toronto", date: 20240114, z: 5.07, field: "wind" },
            { city: "Cape_Town", date: 20240407, z: 5.07, field: "wind" },
          ]
        }
      };
    }

    // SECTION AT (point query)
    if (upper.startsWith("SECTION") && upper.includes("AT")) {
      const idMatch = q.match(/id\s*=\s*(\d+)/i);
      if (idMatch) {
        const id = parseInt(idMatch[1]);
        const rec = SENSORS.find(r => r.id === id);
        if (!rec) return { type: "error", message: `No section at base point id=${id}` };
        const K = fieldCurvature(SENSORS.filter(r => r.city === rec.city).map(r => r.temp), 80);
        return { type: "section", title: `SECTION sensors AT id=${id}`, time: performance.now() - start,
          data: rec, meta: { confidence: (1 / (1 + K)).toFixed(4), curvature: K.toFixed(6),
            anomaly: Math.abs(rec.temp) > 25 ? `TRUE (z=${(Math.abs(rec.temp) / 10).toFixed(2)})` : "FALSE",
            query_time: "O(1)" }
        };
      }
    }

    // EXISTS SECTION
    if (upper.startsWith("EXISTS")) {
      const idMatch = q.match(/id\s*=\s*(\d+)/i);
      if (idMatch) {
        const exists = SENSORS.some(r => r.id === parseInt(idMatch[1]));
        return { type: "scalar", title: `EXISTS SECTION sensors AT id=${idMatch[1]}`,
          time: performance.now() - start, value: exists ? "TRUE" : "FALSE",
          meta: { confidence: "1.0000" } };
      }
    }

    // COVER ON (range query)
    if (upper.startsWith("COVER")) {
      let results = [...SENSORS];
      let desc = [];

      // ON city = '...'
      const cityMatch = q.match(/ON\s+city\s*=\s*'([^']+)'/i);
      if (cityMatch) {
        results = results.filter(r => r.city === cityMatch[1]);
        desc.push(`ON city='${cityMatch[1]}'`);
      }
      // ON region = '...'
      const regionMatch = q.match(/ON\s+region\s*=\s*'([^']+)'/i);
      if (regionMatch) {
        results = results.filter(r => r.region === regionMatch[1]);
        desc.push(`ON region='${regionMatch[1]}'`);
      }
      // ON region IN (...)
      const regionInMatch = q.match(/ON\s+region\s+IN\s*\(\s*'([^)]+)'\s*\)/i);
      if (regionInMatch) {
        const vals = regionInMatch[1].split("'").filter(s => s.match(/[A-Z]/));
        results = results.filter(r => vals.includes(r.region));
        desc.push(`ON region IN (${vals.join(",")})`);
      }
      // WHERE temp < n
      const tempLtMatch = q.match(/WHERE\s+temp\s*<\s*(-?\d+\.?\d*)/i);
      if (tempLtMatch) {
        results = results.filter(r => r.temp < parseFloat(tempLtMatch[1]));
        desc.push(`WHERE temp < ${tempLtMatch[1]}`);
      }
      // WHERE temp > n
      const tempGtMatch = q.match(/WHERE\s+temp\s*>\s*(-?\d+\.?\d*)/i);
      if (tempGtMatch) {
        results = results.filter(r => r.temp > parseFloat(tempGtMatch[1]));
        desc.push(`WHERE temp > ${tempGtMatch[1]}`);
      }
      // WHERE wind > n
      const windMatch = q.match(/WHERE\s+wind\s*>\s*(-?\d+\.?\d*)/i);
      if (windMatch) {
        results = results.filter(r => r.wind > parseFloat(windMatch[1]));
        desc.push(`WHERE wind > ${windMatch[1]}`);
      }
      // WHERE status = '...'
      const statusMatch = q.match(/WHERE\s+status\s*=\s*'([^']+)'/i);
      if (statusMatch) {
        results = results.filter(r => r.status === statusMatch[1]);
        desc.push(`WHERE status='${statusMatch[1]}'`);
      }
      // RANK BY
      const rankMatch = q.match(/RANK\s+BY\s+(\w+)\s*(ASC|DESC)?/i);
      if (rankMatch) {
        const f = rankMatch[1], dir = (rankMatch[2] || "ASC").toUpperCase();
        results.sort((a, b) => dir === "ASC" ? (a[f] - b[f]) : (b[f] - a[f]));
        desc.push(`RANK BY ${f} ${dir}`);
      }
      // FIRST n
      const firstMatch = q.match(/FIRST\s+(\d+)/i);
      if (firstMatch) {
        results = results.slice(0, parseInt(firstMatch[1]));
        desc.push(`FIRST ${firstMatch[1]}`);
      }
      // DISTINCT
      const distinctMatch = q.match(/DISTINCT\s+(\w+)/i);
      if (distinctMatch) {
        const f = distinctMatch[1];
        const vals = [...new Set(results.map(r => r[f]))];
        return { type: "list", title: `COVER sensors DISTINCT ${f}`, time: performance.now() - start,
          values: vals, meta: { count: vals.length, method: "field_index_keys", complexity: "O(1)" } };
      }

      const temps = results.map(r => r.temp);
      const K = temps.length > 1 ? fieldCurvature(temps, 80) : 0;
      return { type: "table", title: `COVER sensors ${desc.join(" ")}`,
        time: performance.now() - start,
        columns: ["id", "city", "region", "date", "temp", "humidity", "wind", "status"],
        rows: results.map(r => [r.id, r.city, r.region, r.date, r.temp, r.humidity, r.wind, r.status]),
        meta: { records: results.length, curvature: K.toFixed(6), confidence: (1/(1+K)).toFixed(4),
          complexity: cityMatch || regionMatch ? `O(${results.length})` : `O(${SENSORS.length})` }
      };
    }

    // CURVATURE
    if (upper.startsWith("CURVATURE")) {
      const fieldMatch = q.match(/ON\s+(\w+)/i);
      const byMatch = q.match(/BY\s+(\w+)/i);
      const field = fieldMatch ? fieldMatch[1] : "temp";
      const range = { temp: 80, humidity: 100, wind: 30, pressure: 20, solar: 12 }[field] || 80;

      if (byMatch) {
        const groupField = byMatch[1];
        const groups = {};
        SENSORS.forEach(r => { (groups[r[groupField]] = groups[r[groupField]] || []).push(r[field]); });
        const rows = Object.entries(groups).map(([g, vals]) => {
          const K = fieldCurvature(vals, range);
          return [g, K.toFixed(6), (1/(1+K)).toFixed(4), vals.length, K > 0.02 ? "HIGH" : K > 0.005 ? "MED" : "LOW"];
        }).sort((a, b) => parseFloat(b[1]) - parseFloat(a[1]));
        return { type: "table", title: `CURVATURE sensors ON ${field} BY ${groupField}`,
          time: performance.now() - start,
          columns: [groupField, `K(${field})`, "confidence", "records", "flag"],
          rows, meta: { field, range, method: "variance/range²" }
        };
      }
      const vals = SENSORS.map(r => r[field]);
      const K = fieldCurvature(vals, range);
      return { type: "scalar", title: `CURVATURE sensors ON ${field}`,
        time: performance.now() - start, value: K.toFixed(6),
        meta: { confidence: (1/(1+K)).toFixed(4), range, records: vals.length }
      };
    }

    // CONFIDENCE
    if (upper.startsWith("CONFIDENCE")) {
      const K = 0.0412;
      return { type: "scalar", title: "CONFIDENCE sensors", time: performance.now() - start,
        value: (1/(1+K)).toFixed(4), meta: { global_K: K } };
    }

    // SPECTRAL
    if (upper.startsWith("SPECTRAL")) {
      return { type: "spectral", title: "SPECTRAL sensors", time: performance.now() - start,
        data: { lambda1: 0.0, components: 4, diameter: 1, mixing_time: "∞",
          interpretation: "Disconnected clusters: 4 isolated city groups" }
      };
    }

    // CONSISTENCY
    if (upper.startsWith("CONSISTENCY")) {
      return { type: "scalar", title: "CONSISTENCY sensors", time: performance.now() - start,
        value: "h¹ = 0", meta: { status: "CONSISTENT", cocycles: 0 } };
    }

    // ENTROPY
    if (upper.startsWith("ENTROPY")) {
      const fieldMatch = q.match(/ON\s+(\w+)/i);
      const field = fieldMatch ? fieldMatch[1] : null;
      if (field === "status") {
        const counts = {};
        SENSORS.forEach(r => counts[r.status] = (counts[r.status] || 0) + 1);
        const total = SENSORS.length;
        let H = 0;
        Object.values(counts).forEach(c => { const p = c / total; if (p > 0) H -= p * Math.log2(p); });
        return { type: "scalar", title: `ENTROPY sensors ON status`, time: performance.now() - start,
          value: H.toFixed(4) + " bits", meta: { distribution: counts, interpretation: H < 1 ? "Low — very predictable" : "High — diverse" } };
      }
      return { type: "scalar", title: "ENTROPY sensors", time: performance.now() - start,
        value: "3.42 bits", meta: { interpretation: "Moderate — mixed climate data" } };
    }

    // INTEGRATE (aggregation)
    if (upper.startsWith("INTEGRATE")) {
      const overMatch = q.match(/OVER\s+(\w+)/i);
      if (overMatch) {
        const groupField = overMatch[1];
        const groups = {};
        SENSORS.forEach(r => { (groups[r[groupField]] = groups[r[groupField]] || []).push(r); });
        const rows = Object.entries(groups).map(([g, recs]) => {
          const avgTemp = (recs.reduce((a, r) => a + r.temp, 0) / recs.length).toFixed(1);
          const K = fieldCurvature(recs.map(r => r.temp), 80);
          return [g, avgTemp, recs.length, K.toFixed(6), (1/(1+K)).toFixed(4)];
        }).sort((a, b) => parseFloat(a[1]) - parseFloat(b[1]));
        return { type: "table", title: `INTEGRATE sensors OVER ${groupField} MEASURE avg(temp), count(*)`,
          time: performance.now() - start,
          columns: [groupField, "avg(temp)", "count", "K", "confidence"],
          rows, meta: { method: "fiber_integral", complexity: `O(${SENSORS.length})` }
        };
      }
    }

    // BETTI
    if (upper.startsWith("BETTI")) {
      return { type: "scalar", title: "BETTI sensors", time: performance.now() - start,
        value: "β₀=4, β₁=0, β₂=0",
        meta: { interpretation: "4 connected components (cities), no loops, no voids" } };
    }

    // EULER
    if (upper.startsWith("EULER")) {
      return { type: "scalar", title: "EULER sensors", time: performance.now() - start,
        value: "χ = 4", meta: { formula: "β₀ - β₁ + β₂ = 4 - 0 + 0 = 4" } };
    }

    // SCALAR
    if (upper.startsWith("SCALAR")) {
      return { type: "scalar", title: "SCALAR sensors", time: performance.now() - start,
        value: "R = 0.0412", meta: { meaning: "Trace of Ricci tensor across all fiber dimensions" } };
    }

    // OUTLIER
    if (upper.startsWith("OUTLIER")) {
      const outliers = SENSORS.filter(r => Math.abs(r.temp) > 28 || r.wind > 10);
      return { type: "table", title: "OUTLIER sensors ON temp, wind SIGMA 3",
        time: performance.now() - start,
        columns: ["id", "city", "date", "temp", "wind", "z_score", "field"],
        rows: outliers.map(r => [r.id, r.city, r.date, r.temp, r.wind,
          Math.max(Math.abs(r.temp)/10, r.wind/4).toFixed(2),
          Math.abs(r.temp) > 28 ? "temp" : "wind"]),
        meta: { threshold: "3σ (adaptive)", total: outliers.length }
      };
    }

    // PREDICT
    if (upper.startsWith("PREDICT")) {
      return { type: "table", title: "PREDICT sensors ON temp BY city", time: performance.now() - start,
        columns: ["city", "K(temp)", "prediction", "confidence"],
        rows: [
          ["Moscow", "0.0521", "HIGH_VOLATILITY", "0.9503"],
          ["Beijing", "0.0211", "MODERATE", "0.9793"],
          ["Toronto", "0.0162", "MODERATE", "0.9841"],
          ["Cape_Town", "0.0009", "STABLE", "0.9991"],
          ["Singapore", "0.0001", "STABLE", "0.9999"],
        ],
        meta: { method: "curvature_ranking" }
      };
    }

    // TRIVIALIZE
    if (upper.startsWith("TRIVIALIZE")) {
      return { type: "scalar", title: "TRIVIALIZE sensors", time: performance.now() - start,
        value: "NON-TRIVIAL",
        meta: { reason: "Transition functions between city charts are non-identity", meaning: "This data has structure that no single flat CSV can fully represent" } };
    }

    // PROFILE
    if (upper.startsWith("PROFILE")) {
      return { type: "table", title: "PROFILE sensors", time: performance.now() - start,
        columns: ["field", "K", "entropy", "range", "distinct", "void_rate"],
        rows: [
          ["temp", "0.0233", "3.41", "80", "16", "0%"],
          ["humidity", "0.0034", "2.89", "100", "16", "0%"],
          ["wind", "0.0019", "2.12", "30", "12", "0%"],
          ["pressure", "0.0002", "1.87", "20", "14", "0%"],
          ["solar", "0.0015", "2.34", "12", "14", "0%"],
        ],
        meta: { scalar_K: 0.0412, betti: "β₀=4, β₁=0", euler: 4, storage: "SEQUENTIAL" }
      };
    }

    // TRANSLATE SQL
    if (upper.startsWith("TRANSLATE")) {
      const sqlMatch = q.match(/SQL\s*"([^"]+)"/i);
      if (sqlMatch) {
        const sql = sqlMatch[1].toUpperCase();
        let gql = "-- GQL translation:\n";
        if (sql.includes("SELECT") && sql.includes("GROUP BY")) {
          gql += "INTEGRATE sensors\n  OVER city\n  MEASURE avg(temp), count(*);";
        } else if (sql.includes("SELECT") && sql.includes("WHERE")) {
          gql += "COVER sensors ON city = 'Moscow'\n  WHERE temp < -25;";
        } else {
          gql += "COVER sensors ALL;";
        }
        return { type: "translation", title: "TRANSLATE SQL", time: performance.now() - start,
          sql: sqlMatch[1], gql };
      }
    }

    return { type: "error", message: `Unknown query. Try: SECTION sensors AT id=3;\nCOVER sensors ON city = 'Moscow';\nCURVATURE sensors ON temp BY city;\nHEALTH sensors;\nSHOW BUNDLES;` };
  } catch (e) {
    return { type: "error", message: e.message };
  }
}

// ─── Syntax Highlighting ───
const KEYWORDS = ["BUNDLE","SECTION","SECTIONS","COVER","INTEGRATE","PULLBACK","GAUGE","COLLAPSE","LENS","REDEFINE","RETRACT","CURVATURE","CONFIDENCE","SPECTRAL","HOLONOMY","CONSISTENCY","PARTITION","HEALTH","PREDICT","TRANSPORT","FLOW","GEODESIC","RESTRICT","GLUE","CALIBRATE","SNAPSHOT","DIFF","SUBSCRIBE","EMIT","TRANSLATE","EXPLAIN","DESCRIBE","SHOW","ATLAS","OUTLIER","PROFILE","ENTROPY","DIVERGENCE","FISHER","MUTUAL","BETTI","EULER","SCALAR","RICCI","SECTIONAL","DEVIATION","TREND","CAPACITY","BOTTLENECK","CLUSTER","MIXING","CONDUCTANCE","LAPLACIAN","PHASE","CRITICAL","FREEENERGY","TEMPERATURE","WILSON","SIMILAR","CORRELATE","SEGMENT","TRIVIALIZE","CHARACTERISTIC","DOUBLECOVER","RECALL","COMPLETENESS","COCYCLE","COBOUNDARY","EXISTS","VERIFY","UPSERT"];
const CLAUSES = ["ON","AT","WHERE","OVER","MEASURE","ALONG","ONTO","BY","SET","INTO","FROM","TO","WITHIN","AROUND","HAVING","RESTRICT","RANK","FIRST","SKIP","DISTINCT","PROJECT","EMIT","CHECK","PRESERVE","WINDOW","SHIFT","COARSEN","LEVELS","TOLERANCE","TRAIN","TEST","BEFORE","AFTER","DRIFT","SIGMA","NEAR","MATCHES","VOID","DEFINED","REPAIR","FULL","VERBOSE","ALL","MATERIALIZE","REFRESH","CONFIRM","AGAINST"];
const TYPES = ["NUMERIC","CATEGORICAL","TEXT","BOOLEAN","TIMESTAMP","BASE","FIBER","INDEX","RANGE","DEFAULT","AUTO","ARITHMETIC","UNIQUE","REQUIRED","NULLABLE"];
const VALUES = ["TRUE","FALSE","ASC","DESC","DHOOM","JSON","CSV","FLAT","CURVED","LEFT","BUNDLES","SQL","OVERLAP","UNBOUNDED"];
const FUNCS = ["avg","sum","count","min","max","stddev","variance","median","mode","percentile"];

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
      else if (FUNCS.includes(tok)) result += `<span style="color:${C.accent2}">${esc(tok)}</span>`;
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
      <div style={{ fontSize: 28, fontWeight: 900, color: C.accent, fontFamily: FONT, margin: "8px 0" }}>{result.value}</div>
      {Object.entries(meta).map(([k, v]) => <div key={k} style={metaStyle}><span style={{ color: C.dim }}>{k}:</span> <span style={{ color: C.text }}>{typeof v === "object" ? JSON.stringify(v) : String(v)}</span></div>)}
    </div>
  );

  if (result.type === "section") return (
    <div style={{ padding: 16 }}>
      <div style={{ display: "flex", justifyContent: "space-between" }}>
        <div style={hdr}>{result.title}</div>
        <span style={{ fontSize: 10, color: C.dim, fontFamily: FONT }}>{timeStr}</span>
      </div>
      <div style={{ background: C.bg, borderRadius: 6, padding: 12, border: `1px solid ${C.border}`, fontFamily: FONT, fontSize: 12 }}>
        {Object.entries(result.data).map(([k, v]) => (
          <div key={k} style={{ display: "flex", padding: "3px 0" }}>
            <span style={{ color: C.dim, width: 90, flexShrink: 0 }}>{k}:</span>
            <span style={{ color: typeof v === "number" ? C.num : C.str }}>{String(v)}</span>
          </div>
        ))}
      </div>
      <div style={{ display: "flex", gap: 16, marginTop: 10 }}>
        {Object.entries(meta).map(([k, v]) => (
          <div key={k} style={{ fontSize: 11, fontFamily: FONT }}>
            <span style={{ color: C.dim }}>{k}: </span>
            <span style={{ color: k === "anomaly" && String(v).includes("TRUE") ? C.warn : C.accent, fontWeight: 600 }}>{String(v)}</span>
          </div>
        ))}
      </div>
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
      {Object.entries(result.data).filter(([k]) => k !== "top_anomalies").map(([k, v]) => (
        <div key={k} style={{ display: "flex", padding: "3px 0", fontFamily: FONT, fontSize: 12 }}>
          <span style={{ color: C.dim, width: 140, flexShrink: 0 }}>{k}:</span>
          <span style={{ color: typeof v === "boolean" ? (v ? C.accent : C.warn) : C.text, fontWeight: 600 }}>{String(v)}</span>
        </div>
      ))}
      <div style={{ marginTop: 10 }}><div style={{ ...hdr, fontSize: 10 }}>TOP ANOMALIES</div>
        {result.data.top_anomalies.map((a, i) => (
          <div key={i} style={{ fontFamily: FONT, fontSize: 11, color: C.warn, padding: "2px 0" }}>
            {a.city} · {a.date} · z={a.z.toFixed(2)} · {a.field}
          </div>
        ))}
      </div>
    </div>
  );

  if (result.type === "spectral") return (
    <div style={{ padding: 16 }}>
      <div style={hdr}>{result.title}</div>
      {Object.entries(result.data).map(([k, v]) => (
        <div key={k} style={{ display: "flex", padding: "3px 0", fontFamily: FONT, fontSize: 12 }}>
          <span style={{ color: C.dim, width: 140, flexShrink: 0 }}>{k}:</span>
          <span style={{ color: C.text }}>{String(v)}</span>
        </div>
      ))}
    </div>
  );

  if (result.type === "list") return (
    <div style={{ padding: 16 }}>
      <div style={hdr}>{result.title}</div>
      <div style={{ fontFamily: FONT, fontSize: 13, color: C.str }}>[{result.values.map(v => `'${v}'`).join(", ")}]</div>
      <div style={metaStyle}>{Object.entries(meta).map(([k,v]) => `${k}: ${v}`).join("  ·  ")}</div>
    </div>
  );

  if (result.type === "translation") return (
    <div style={{ padding: 16 }}>
      <div style={hdr}>TRANSLATE SQL → GQL</div>
      <div style={{ fontSize: 10, color: C.dim, marginBottom: 6 }}>INPUT (SQL):</div>
      <pre style={{ fontFamily: FONT, fontSize: 12, color: C.muted, margin: "0 0 12px", padding: 10, background: C.bg, borderRadius: 6, border: `1px solid ${C.border}` }}>{result.sql}</pre>
      <div style={{ fontSize: 10, color: C.accent, marginBottom: 6 }}>OUTPUT (GQL):</div>
      <pre style={{ fontFamily: FONT, fontSize: 12, color: C.accent, margin: 0, padding: 10, background: C.bg, borderRadius: 6, border: `1px solid rgba(64,232,160,0.1)` }}>{result.gql}</pre>
    </div>
  );

  return <div style={{ padding: 16, color: C.muted }}>Unknown result type</div>;
}

// ─── Example Queries ───
const EXAMPLES = [
  { label: "Point Query", q: "SECTION sensors AT id=3;" },
  { label: "Range Query", q: "COVER sensors ON city = 'Moscow' WHERE temp < -25;" },
  { label: "Curvature", q: "CURVATURE sensors ON temp BY city;" },
  { label: "Aggregation", q: "INTEGRATE sensors OVER city MEASURE avg(temp), count(*);" },
  { label: "Spectral", q: "SPECTRAL sensors;" },
  { label: "Health", q: "HEALTH sensors;" },
  { label: "Outliers", q: "OUTLIER sensors ON temp, wind SIGMA 3;" },
  { label: "Predict", q: "PREDICT sensors ON temp BY city;" },
  { label: "Topology", q: "BETTI sensors;" },
  { label: "Entropy", q: "ENTROPY sensors ON status;" },
  { label: "Consistency", q: "CONSISTENCY sensors;" },
  { label: "Trivialize", q: "TRIVIALIZE sensors;" },
  { label: "Profile", q: "PROFILE sensors;" },
  { label: "Describe", q: "DESCRIBE sensors;" },
  { label: "Translate SQL", q: `TRANSLATE SQL "SELECT city, AVG(temp) FROM sensors GROUP BY city";` },
];

// ─── Main App ───
export default function GQLPlayground() {
  const [code, setCode] = useState("-- GQL Playground · Davis Geometric\n-- Type a query and press Ctrl+Enter or click Run\n\nHEALTH sensors;");
  const [result, setResult] = useState(null);
  const [history, setHistory] = useState([]);
  const textareaRef = useRef(null);
  const highlightRef = useRef(null);

  const run = useCallback(() => {
    const r = executeGQL(code);
    setResult(r);
    setHistory(h => [{ query: code.trim(), time: new Date().toLocaleTimeString() }, ...h].slice(0, 20));
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
          <span style={{ fontSize: 10, color: C.dim, fontFamily: FONT }}>sensors: 16 records · 4 cities</span>
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
              <button onClick={run} style={{ background: C.accent, color: C.bg, border: "none", borderRadius: 4, padding: "3px 12px", fontSize: 10.5, fontWeight: 700, cursor: "pointer" }}>Run</button>
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
                <button key={i} onClick={() => { setCode(ex.q); setTimeout(run, 50); }}
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
            {result && result.meta && result.meta.complexity && (
              <span style={{ fontSize: 10, color: C.accent, fontFamily: FONT, marginLeft: "auto" }}>{result.meta.complexity}</span>
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
