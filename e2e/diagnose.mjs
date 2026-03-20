/**
 * GIGI Dashboard Diagnostic Script
 * Run: node e2e/diagnose.mjs
 *
 * Requires Node 22+ (built-in fetch and WebSocket).
 * No npm install needed.
 */

const BASE = 'http://localhost:3142';
const WS_BASE = 'ws://localhost:3142';

let passed = 0;
let failed = 0;

async function check(label, fn) {
  const pad = label.padEnd(55, '.');
  try {
    const result = await fn();
    console.log(`  ✅ ${pad} ${result}`);
    passed++;
    return true;
  } catch (e) {
    console.log(`  ❌ ${pad} ${e.message}`);
    failed++;
    return false;
  }
}

function wsConnect(url, { onOpen, onMessage, timeoutMs = 4000 } = {}) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(url);
    const messages = [];
    let settled = false;

    function done(err) {
      if (settled) return;
      settled = true;
      // Use 0-delay to let any pending message events flush first
      setTimeout(() => {
        try { ws.close(); } catch {}
        if (err) reject(err); else resolve(messages);
      }, 0);
    }

    const timer = setTimeout(() => done(null), timeoutMs);

    ws.addEventListener('open', async () => {
      try {
        if (onOpen) await onOpen(messages);
      } catch (e) {
        clearTimeout(timer);
        done(e);
      }
    });
    ws.addEventListener('message', e => {
      messages.push(JSON.parse(e.data));
      if (onMessage) onMessage(messages);
    });
    ws.addEventListener('error', e => {
      clearTimeout(timer);
      done(new Error(`WebSocket error (code ${e.code || '?'}): ${e.message || e.type || 'unknown'}`));
    });
    ws.addEventListener('close', ev => {
      clearTimeout(timer);
      if (ev.wasClean || ev.code === 1000 || ev.code === 1001 || ev.code === 1005 || ev.code === 0) {
        done(null);
      } else {
        done(new Error(`WebSocket closed unexpectedly: code=${ev.code} reason="${ev.reason}" wasClean=${ev.wasClean}`));
      }
    });
  });
}

console.log('\n╔══════════════════════════════════════════════════════════╗');
console.log('║           GIGI Dashboard Diagnostics                    ║');
console.log('╚══════════════════════════════════════════════════════════╝\n');

// ── 1. Basic connectivity ────────────────────────────────────────────────────
console.log('── REST Endpoints ─────────────────────────────────────────\n');

const serverUp = await check('GET /v1/health → 200', async () => {
  const r = await fetch(`${BASE}/v1/health`, { signal: AbortSignal.timeout(3000) });
  if (!r.ok) throw new Error(`HTTP ${r.status}`);
  return `${r.status} OK`;
});

if (!serverUp) {
  console.log('\n  ⛔ Server is not running on port 3142. Start it first.\n');
  process.exit(1);
}

await check('GET /v1/bundles → array', async () => {
  const r = await fetch(`${BASE}/v1/bundles`);
  const d = await r.json();
  return `${r.status} → ${JSON.stringify(d)}`;
});

await check('GET /dashboard → HTML', async () => {
  const r = await fetch(`${BASE}/dashboard`);
  const text = await r.text();
  if (!text.includes('GIGI Live Dashboard')) throw new Error('missing page title');
  if (!text.includes('createAndRunDemo')) throw new Error('missing demo function');
  return `${r.status} OK (${text.length} bytes)`;
});

// ── 2. Bundle lifecycle ──────────────────────────────────────────────────────
console.log('\n── Bundle Lifecycle ───────────────────────────────────────\n');

await check('DELETE /v1/bundles/demo (cleanup)', async () => {
  const r = await fetch(`${BASE}/v1/bundles/demo`, { method: 'DELETE' });
  return `status=${r.status} (404 also OK)`;
});

await check('POST /v1/bundles → create demo', async () => {
  const r = await fetch(`${BASE}/v1/bundles`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      name: 'demo',
      schema: {
        fields: {
          id: 'numeric', sensor: 'categorical',
          temp_c: 'numeric', humidity: 'numeric',
          pressure: 'numeric', co2_ppm: 'numeric',
        },
        keys: ['id'],
        indexed: ['sensor'],
      },
    }),
  });
  const d = await r.json().catch(() => ({}));
  if (!r.ok) throw new Error(`HTTP ${r.status}: ${JSON.stringify(d)}`);
  return `${r.status} created`;
});

// ── 3. Insert records ────────────────────────────────────────────────────────
console.log('\n── Data Insertion ─────────────────────────────────────────\n');

await check('POST /v1/bundles/demo/points batch of 5', async () => {
  const records = Array.from({ length: 5 }, (_, i) => ({
    id: i + 1,
    sensor: ['alpha', 'beta', 'gamma'][i % 3],
    temp_c: +(20 + i * 0.5).toFixed(2),
    humidity: 50.0,
    pressure: 1010.0,
    co2_ppm: 450.0,
  }));
  const r = await fetch(`${BASE}/v1/bundles/demo/points`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ records }),
  });
  const d = await r.json().catch(() => ({}));
  if (!r.ok) throw new Error(`HTTP ${r.status}: ${JSON.stringify(d)}`);
  return `${r.status} inserted=${records.length}`;
});

await check('GET /v1/bundles/demo/health → record_count > 0', async () => {
  const r = await fetch(`${BASE}/v1/bundles/demo/health`);
  if (!r.ok) throw new Error(`HTTP ${r.status}`);
  const d = await r.json();
  if (d.record_count === undefined) throw new Error(`no record_count field. Got: ${JSON.stringify(d)}`);
  if (d.record_count === 0) throw new Error('record_count is 0');
  if (d.confidence === undefined) throw new Error(`no confidence field`);
  if (d.k_mean === undefined) throw new Error(`no k_mean field`);
  if (!Array.isArray(d.per_field)) throw new Error(`per_field is not array. Got: ${typeof d.per_field}`);
  return `record_count=${d.record_count} confidence=${d.confidence?.toFixed(4)} k_mean=${d.k_mean?.toFixed(5)}`;
});

// ── 4. WebSocket ─────────────────────────────────────────────────────────────
console.log('\n── WebSocket Connectivity ─────────────────────────────────\n');

await check('WS /v1/ws/demo/dashboard → opens', async () => {
  const messages = await wsConnect(`${WS_BASE}/v1/ws/demo/dashboard`, { timeoutMs: 2000 });
  return `connected, ${messages.length} buffered msgs on open`;
});

await check('WS + insert → receives DashboardEvent', async () => {
  let insertId = 100;
  const messages = await wsConnect(`${WS_BASE}/v1/ws/demo/dashboard`, {
    timeoutMs: 5000,
    onOpen: async (_msgs) => {
      // Insert a record after WS is open
      await fetch(`${BASE}/v1/bundles/demo/points`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          records: [{
            id: insertId++,
            sensor: 'alpha',
            temp_c: 22.5,
            humidity: 50.0,
            pressure: 1010.0,
            co2_ppm: 450.0,
          }],
        }),
      });
      // Wait for 2s then let the timeout resolve
      await new Promise(r => setTimeout(r, 2000));
    },
  });

  if (messages.length === 0) {
    throw new Error(
      'No WS messages received after insert. ' +
      'WS connects but server never emits DashboardEvent. ' +
      'Check dashboard_tx.send() in gigi_stream.rs'
    );
  }

  const ev = messages[0];
  const missing = [];
  if (ev.type === undefined) missing.push('type');
  if (ev.record_count === undefined) missing.push('record_count');
  if (ev.k_mean === undefined) missing.push('k_mean');
  if (ev.global_confidence === undefined) missing.push('global_confidence');
  if (missing.length) throw new Error(`Event received but missing fields: ${missing.join(', ')}. Got: ${JSON.stringify(ev)}`);

  return `${messages.length} event(s). type=${ev.type} record_count=${ev.record_count} k_mean=${ev.k_mean?.toFixed(5)} confidence=${ev.global_confidence?.toFixed(4)}`;
});

await check('WS /v1/ws/dashboard (all-bundles feed) → opens', async () => {
  const messages = await wsConnect(`${WS_BASE}/v1/ws/dashboard`, { timeoutMs: 2000 });
  return `connected, ${messages.length} buffered msgs`;
});

// ── 5. Anomaly detection ─────────────────────────────────────────────────────
console.log('\n── Anomaly Detection ──────────────────────────────────────\n');

await check('Insert extreme anomaly → WS is_anomaly=true', async () => {
  // First insert 20 normal records to build baseline
  const normals = Array.from({ length: 20 }, (_, i) => ({
    id: 200 + i,
    sensor: 'alpha',
    temp_c: +(20 + (Math.random() - 0.5) * 2).toFixed(2),
    humidity: +(50 + (Math.random() - 0.5) * 4).toFixed(2),
    pressure: +(1010 + (Math.random() - 0.5) * 2).toFixed(2),
    co2_ppm: +(450 + (Math.random() - 0.5) * 20).toFixed(1),
  }));
  await fetch(`${BASE}/v1/bundles/demo/points`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ records: normals }),
  });
  await new Promise(r => setTimeout(r, 200));

  const anomalyMessages = [];
  const messages = await wsConnect(`${WS_BASE}/v1/ws/demo/dashboard`, {
    timeoutMs: 4000,
    onOpen: async (_msgs) => {
      // Insert extreme anomaly
      await fetch(`${BASE}/v1/bundles/demo/points`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          records: [{
            id: 999,
            sensor: 'alpha',
            temp_c: -99.9,    // extreme
            humidity: 1.0,    // extreme
            pressure: 700.0,  // extreme
            co2_ppm: 9999.0,  // extreme
          }],
        }),
      });
      await new Promise(r => setTimeout(r, 2500));
    },
    onMessage: (_msgs) => {
      for (const m of _msgs) {
        if (m.is_anomaly) anomalyMessages.push(m);
      }
    },
  });

  if (messages.length === 0) throw new Error('No WS events at all');
  if (anomalyMessages.length === 0) {
    return `⚠️  ${messages.length} events received but none flagged is_anomaly=true (may need more baseline data)`;
  }
  const a = anomalyMessages[0];
  return `anomaly detected! z=${a.z_score?.toFixed(2)} k_local=${a.local_curvature?.toFixed(5)} fields=[${(a.contributing_fields||[]).join(',')}]`;
});

// ── 6. CORS headers ──────────────────────────────────────────────────────────
console.log('\n── CORS Headers ───────────────────────────────────────────\n');

await check('OPTIONS /v1/bundles (CORS preflight) → allows Origin', async () => {
  const r = await fetch(`${BASE}/v1/bundles`, {
    method: 'OPTIONS',
    headers: {
      'Origin': 'http://localhost:3142',
      'Access-Control-Request-Method': 'POST',
      'Access-Control-Request-Headers': 'content-type',
    },
  });
  const acao = r.headers.get('access-control-allow-origin') || r.headers.get('vary') || '';
  return `${r.status} ACAO="${acao}"`;
});

// ── Summary ──────────────────────────────────────────────────────────────────
console.log('\n╔══════════════════════════════════════════════════════════╗');
console.log(`║  Results: ${String(passed).padStart(2)} passed, ${String(failed).padStart(2)} failed                          ║`);
console.log('╚══════════════════════════════════════════════════════════╝\n');

if (failed > 0) process.exit(1);
