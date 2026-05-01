#!/usr/bin/env node
/**
 * One-shot restore script for the jg_kv → OPAQUE-payload migration.
 *
 * Sequence:
 *   1. (manual) `COLLAPSE BUNDLE jg_kv` via /v1/gql.
 *   2. (manual) `flyctl deploy --remote-only` — the bootstrap will
 *      recreate jg_kv under the new manifest with gauge_key.
 *   3. Run THIS script: `node _local/restore_jg_kv_post_encryption.mjs`.
 *      It reads the local backup and re-inserts every record. Each
 *      insert flows through gigi-stream's encrypt_fiber path and
 *      writes the payload as Opaque ciphertext at rest.
 *
 * Idempotency: insert returns "already exists" on duplicate `key`,
 * which is treated as success (the record was already restored).
 *
 * Usage:
 *   node _local/restore_jg_kv_post_encryption.mjs [--dry-run]
 */

import { readFileSync } from 'node:fs';

const GIGI_URL = process.env.GIGI_URL || 'https://gigi-stream.fly.dev';
const BACKUP   = process.argv[2] && !process.argv[2].startsWith('--')
  ? process.argv[2]
  : '_local/jg_kv_pre_encryption_backup.json';
const DRY_RUN  = process.argv.includes('--dry-run');

const data = JSON.parse(readFileSync(BACKUP, 'utf8'));
const records = data.data || [];
console.log(`backup: ${records.length} records from ${BACKUP}`);

if (records.length === 0) {
  console.log('nothing to restore');
  process.exit(0);
}

let restored = 0;
let already_present = 0;
let failed = 0;

for (const r of records) {
  // Strip server-added meta fields before re-inserting.
  const clean = { ...r };
  for (const meta of ['__bp__', '__base_point__', '__index__']) {
    delete clean[meta];
  }

  console.log(`  ${restored + already_present + failed + 1}/${records.length}: ${String(clean.key).slice(0, 60)}`);
  if (DRY_RUN) continue;

  const resp = await fetch(`${GIGI_URL}/v1/bundles/jg_kv/insert`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ records: [clean] }),
  });

  if (resp.ok) {
    restored++;
  } else {
    const body = await resp.text();
    if (/exist|duplicate|conflict/i.test(body)) {
      already_present++;
    } else {
      console.error(`    FAIL ${resp.status}: ${body}`);
      failed++;
    }
  }
}

console.log(`\n${restored} restored, ${already_present} already present, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
