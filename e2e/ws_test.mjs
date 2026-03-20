const ws = new WebSocket('ws://localhost:3142/v1/ws/demo/dashboard');
ws.addEventListener('open', () => {
  console.log('WS OPEN - readyState:', ws.readyState);
  fetch('http://localhost:3142/v1/bundles/demo/points', {
    method: 'POST',
    headers: {'Content-Type':'application/json'},
    body: JSON.stringify({ records: [{ id: 9001, sensor: 'alpha', temp_c: 22.5, humidity: 50.0, pressure: 1010.0, co2_ppm: 450.0 }] })
  }).then(r => { console.log('INSERT status:', r.status); return r.json(); }).then(d => console.log('INSERT body:', JSON.stringify(d)));
});
ws.addEventListener('message', e => {
  console.log('WS MESSAGE:', e.data.slice(0, 300));
});
ws.addEventListener('close', e => {
  console.log('WS CLOSE: code=', e.code, 'reason=', e.reason, 'wasClean=', e.wasClean);
  process.exit(0);
});
ws.addEventListener('error', e => {
  console.log('WS ERROR: type=', e.type, 'message=', e.message);
});
setTimeout(() => {
  console.log('4s Timeout - readyState:', ws.readyState, '(0=CONNECTING 1=OPEN 2=CLOSING 3=CLOSED)');
  process.exit(0);
}, 4000);
