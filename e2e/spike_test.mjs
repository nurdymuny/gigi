/**
 * e2e test: do injections actually produce detectable anomaly events?
 * Run: node e2e/spike_test.mjs
 */
import WebSocket from "ws";

const BASE = "https://gigi-stream.fly.dev";
const WS_BASE = "wss://gigi-stream.fly.dev";
const BUNDLE = `e2e_spike_${Date.now()}`;

async function post(path, body) {
  const r = await fetch(`${BASE}${path}`, {
    method: "POST", headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!r.ok) throw new Error(`POST ${path} → ${r.status} ${await r.text()}`);
  return r.json();
}
async function del(path) {
  await fetch(`${BASE}${path}`, { method: "DELETE" });
}

function normal(seq, sensor) {
  return {
    seq_id: seq, sensor_id: sensor, ts_ms: Date.now(),
    temp_c: 22 + (Math.random() - 0.5) * 8,
    pressure_hpa: 1013 + (Math.random() - 0.5) * 30,
    vibration_g: 0.10 + Math.random() * 0.08,
    rpm: 3000 + (Math.random() - 0.5) * 500,
    signal: "normal",
  };
}
function spike(seq, sensor) {
  return {
    seq_id: seq, sensor_id: sensor, ts_ms: Date.now(),
    temp_c: 140 + Math.random() * 30,
    pressure_hpa: 1900 + Math.random() * 200,
    vibration_g: 8 + Math.random() * 4,
    rpm: 8500 + Math.random() * 1000,
    signal: "spike",
  };
}

// ── main ──────────────────────────────────────────────────────────────────────
console.log(`Bundle: ${BUNDLE}`);

// 1. Create bundle
await post("/v1/bundles", {
  name: BUNDLE,
  schema: {
    fields: {
      seq_id: "numeric", sensor_id: "numeric", ts_ms: "numeric",
      temp_c: "numeric", pressure_hpa: "numeric",
      vibration_g: "numeric", rpm: "numeric", signal: "categorical",
    },
    keys: ["seq_id"],
  },
});
console.log("Bundle created.");

// 2. Open WS dashboard subscription
let wsEvents = [];
const ws = new WebSocket(`${WS_BASE}/v1/ws/${BUNDLE}/dashboard`);

await new Promise((resolve, reject) => {
  ws.on("open", resolve);
  ws.on("error", reject);
  setTimeout(() => reject(new Error("WS timeout")), 10000);
});
ws.on("message", (data) => {
  try {
    const ev = JSON.parse(data.toString());
    wsEvents.push(ev);
  } catch (_) {}
});
console.log("WS connected.");

// 3. Insert 60 normal records (one at a time to build up field stats)
let seq = 0;
console.log("Inserting 60 normal records...");
for (let i = 0; i < 60; i++) {
  await post(`/v1/bundles/${BUNDLE}/insert`, { records: [normal(seq++, i % 5)] });
}
console.log(`After normals: ${wsEvents.length} WS events received.`);

// Snapshot of last event before injections
const lastNormal = wsEvents[wsEvents.length - 1];
console.log(`Last normal event: k_global=${lastNormal?.k_global?.toFixed(5)}, k_threshold_2s=${lastNormal?.k_threshold_2s?.toFixed(5)}, k_mean=${lastNormal?.k_mean?.toFixed(5)}, k_std=${lastNormal?.k_std?.toFixed(5)}, k_count=${wsEvents.length}`);

// 4. Insert 5 injection records and watch for anomaly events
const preCount = wsEvents.length;
console.log("\nInsert 5 injection records...");
for (let i = 0; i < 5; i++) {
  await post(`/v1/bundles/${BUNDLE}/insert`, { records: [spike(seq++, i)] });
  await new Promise(r => setTimeout(r, 150)); // wait for WS event
}
await new Promise(r => setTimeout(r, 1000)); // let final events arrive

const newEvents = wsEvents.slice(preCount);
console.log(`\nNew events after injection (${newEvents.length}):`);
for (const ev of newEvents) {
  const flag = ev.is_anomaly ? "⚠ ANOMALY" : "  insert ";
  console.log(`  ${flag}  type=${ev.type}  k_global=${ev.k_global?.toFixed(5)}  k_threshold=${ev.k_threshold_2s?.toFixed(5)}  is_anomaly=${ev.is_anomaly}  local_k=${ev.local_curvature?.toFixed(5) ?? "none"}  z=${ev.z_score?.toFixed(2) ?? "none"}`);
}

const anomalyEvents = newEvents.filter(e => e.is_anomaly);
console.log(`\nAnomaly events: ${anomalyEvents.length} / ${newEvents.length}`);

// 5. Also check: does k_global > k_threshold_2s on any event?
const aboveThresh = newEvents.filter(e =>
  e.k_global != null && e.k_threshold_2s != null &&
  e.k_global > e.k_threshold_2s && e.k_global > 0
);
console.log(`k_global > k_threshold_2s events: ${aboveThresh.length}`);
if (aboveThresh.length) {
  aboveThresh.forEach(e => console.log(`  k=${e.k_global?.toFixed(5)} thresh=${e.k_threshold_2s?.toFixed(5)}`));
}

// 6. Cleanup
ws.close();
await del(`/v1/bundles/${BUNDLE}`);
console.log("\nBundle deleted. Test done.");
