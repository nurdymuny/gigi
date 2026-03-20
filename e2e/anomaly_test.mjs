/**
 * Targeted anomaly detection test.
 * Run: node e2e/anomaly_test.mjs
 */
const BASE = 'http://localhost:3142';
const WS_BASE = 'ws://localhost:3142';

// Create fresh bundle
await fetch(`${BASE}/v1/bundles/atest`, { method: 'DELETE' });
const cr = await fetch(`${BASE}/v1/bundles`, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    name: 'atest',
    schema: {
      fields: { id: 'numeric', sensor: 'categorical', temp_c: 'numeric', humidity: 'numeric', pressure: 'numeric', co2_ppm: 'numeric' },
      keys: ['id'], indexed: ['sensor'],
    },
  }),
});
console.log('Create bundle:', cr.status);

// Insert 100 normal records in batches of 20
const SENSORS = ['alpha', 'beta', 'gamma', 'delta', 'epsilon'];
const rand = (lo, hi) => lo + Math.random() * (hi - lo);
let id = 1;
for (let batch = 0; batch < 5; batch++) {
  const records = [];
  for (let i = 0; i < 20; i++, id++) {
    records.push({ id, sensor: SENSORS[id % SENSORS.length], temp_c: +rand(18, 26).toFixed(2), humidity: +rand(40, 65).toFixed(2), pressure: +rand(1005, 1020).toFixed(2), co2_ppm: +rand(400, 600).toFixed(1) });
  }
  const r = await fetch(`${BASE}/v1/bundles/atest/points`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ records }) });
  console.log(`Batch ${batch + 1} status: ${r.status}`);
}

// Check health after 100 normal records
const h1 = await fetch(`${BASE}/v1/bundles/atest/health`).then(r => r.json());
console.log('\nAfter 100 normal records:');
console.log('  record_count:', h1.record_count);
console.log('  k_mean:     ', h1.k_mean?.toFixed(6));
console.log('  k_std:      ', h1.k_std?.toFixed(6));
console.log('  k_threshold_2s:', h1.k_threshold_2s?.toFixed(6));
console.log('  confidence: ', h1.confidence?.toFixed(4));

// Connect WS and insert anomaly, watch for events
console.log('\nConnecting WS and inserting anomaly...');
const events = [];
const wsResult = await new Promise((resolve) => {
  const ws = new WebSocket(`${WS_BASE}/v1/ws/atest/dashboard`);
  let settled = false;
  const done = () => { if (!settled) { settled = true; try { ws.close(); } catch {} resolve(events); } };
  
  ws.addEventListener('open', async () => {
    console.log('WS open');
    // Insert extreme anomaly
    const ar = await fetch(`${BASE}/v1/bundles/atest/points`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ records: [{ id: id++, sensor: 'alpha', temp_c: -99.9, humidity: 1.0, pressure: 700.0, co2_ppm: 9999.0 }] }),
    });
    console.log('Anomaly insert status:', ar.status);
    const body = await ar.json();
    console.log('Anomaly insert response:', JSON.stringify(body));
    // Wait 2s for events
    await new Promise(r => setTimeout(r, 2000));
    done();
  });

  ws.addEventListener('message', e => {
    const ev = JSON.parse(e.data);
    events.push(ev);
    console.log(`WS event: type=${ev.type} is_anomaly=${ev.is_anomaly} k_mean=${ev.k_mean?.toFixed(5)} threshold=${ev.k_threshold_2s?.toFixed(5)} local_k=${ev.local_curvature?.toFixed(5)} z=${ev.z_score?.toFixed(2)}`);
  });

  ws.addEventListener('error', e => { console.log('WS error:', e.type); done(); });
  ws.addEventListener('close', () => done());
  setTimeout(done, 5000);
});

console.log(`\nTotal WS events received: ${events.length}`);
const anomalyEvents = events.filter(e => e.is_anomaly);
console.log(`Anomaly events: ${anomalyEvents.length}`);

// Check health after anomaly
const h2 = await fetch(`${BASE}/v1/bundles/atest/health`).then(r => r.json());
console.log('\nAfter anomaly:');
console.log('  record_count:', h2.record_count);
console.log('  k_mean:', h2.k_mean?.toFixed(6));
console.log('  k_std:', h2.k_std?.toFixed(6));

await fetch(`${BASE}/v1/bundles/atest`, { method: 'DELETE' });
console.log('\nCleanup done.');
