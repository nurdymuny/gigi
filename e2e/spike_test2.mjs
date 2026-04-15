/**
 * Realistic e2e: 300 normal records, then inject spikes
 */
import WebSocket from "ws";

const BASE = "https://gigi-stream.fly.dev";
const WS_BASE = "wss://gigi-stream.fly.dev";
const BUNDLE = `e2e_spike2_${Date.now()}`;

async function post(path, body) {
  const r = await fetch(`${BASE}${path}`, {
    method: "POST", headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!r.ok) throw new Error(`POST ${path} → ${r.status} ${await r.text()}`);
  return r.json();
}
async function del(path) { await fetch(`${BASE}${path}`, { method: "DELETE" }); }

function normal(seq, sensor) {
  return { seq_id: seq, sensor_id: sensor, ts_ms: Date.now(),
    temp_c: 22 + (Math.random()-0.5)*8, pressure_hpa: 1013 + (Math.random()-0.5)*30,
    vibration_g: 0.10 + Math.random()*0.08, rpm: 3000 + (Math.random()-0.5)*500,
    signal: "normal" };
}
function spike(seq, sensor) {
  return { seq_id: seq, sensor_id: sensor, ts_ms: Date.now(),
    temp_c: 140 + Math.random()*30, pressure_hpa: 1900 + Math.random()*200,
    vibration_g: 8 + Math.random()*4, rpm: 8500 + Math.random()*1000,
    signal: "spike" };
}

await post("/v1/bundles", {
  name: BUNDLE,
  schema: {
    fields: { seq_id:"numeric", sensor_id:"numeric", ts_ms:"numeric",
      temp_c:"numeric", pressure_hpa:"numeric", vibration_g:"numeric",
      rpm:"numeric", signal:"categorical" },
    keys: ["seq_id"],
  },
});
console.log("Bundle created.");

let wsEvents = [];
const ws = new WebSocket(`${WS_BASE}/v1/ws/${BUNDLE}/dashboard`);
await new Promise((resolve, reject) => {
  ws.on("open", resolve); ws.on("error", reject);
  setTimeout(() => reject(new Error("timeout")), 10000);
});
ws.on("message", (data) => { try { wsEvents.push(JSON.parse(data.toString())); } catch(_) {} });
console.log("WS connected.");

let seq = 0;
// Insert 300 normal records in batches of 5 (like the demo does)
console.log("Inserting 300 normal records (batch of 5)...");
for (let i = 0; i < 60; i++) {
  const recs = Array.from({length:5}, (_,j) => normal(seq++, j));
  await post(`/v1/bundles/${BUNDLE}/insert`, { records: recs });
}
await new Promise(r => setTimeout(r, 500));
const lastN = wsEvents[wsEvents.length-1];
console.log(`After 300 normals: events=${wsEvents.length}`);
console.log(`  k_global=${lastN?.k_global?.toFixed(5)}, k_threshold_2s=${lastN?.k_threshold_2s?.toFixed(5)}`);
console.log(`  k_mean=${lastN?.k_mean?.toFixed(5)}, k_std=${lastN?.k_std?.toFixed(5)}`);

// Now inject 10 spikes, ONE RECORD AT A TIME (like new demo)
const pre = wsEvents.length;
console.log("\nInjecting 10 single-record spikes...");
for (let i = 0; i < 10; i++) {
  await post(`/v1/bundles/${BUNDLE}/insert`, { records: [spike(seq++, i%5)] });
  await new Promise(r => setTimeout(r, 200));
}
await new Promise(r => setTimeout(r, 1000));

const newEvs = wsEvents.slice(pre);
const anomalies = newEvs.filter(e => e.is_anomaly);
console.log(`\nPost-injection events (${newEvs.length}):`);
newEvs.forEach((e,i) => {
  const flag = e.is_anomaly ? "⚠ ANOMALY" : "  insert ";
  console.log(`  [${i+1}] ${flag} type=${e.type} k_global=${e.k_global?.toFixed(4)} `
    + `thresh=${e.k_threshold_2s?.toFixed(4)} local_k=${e.local_curvature?.toFixed(4)??"-"} z=${e.z_score?.toFixed(2)??"-"}`);
});
console.log(`\n=== Result: ${anomalies.length} anomaly events from 10 single-record injections ===`);
console.log(`=== k_global < 0 on all events? ${newEvs.every(e => e.k_global < 0)} ===`);

ws.close();
await del(`/v1/bundles/${BUNDLE}`);
console.log("Done.");
