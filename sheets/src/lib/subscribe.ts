/**
 * Wire-format parser for `gigi-stream` SubscriptionEvent frames.
 *
 * The engine sends one of two line-oriented messages over /ws:
 *
 *   EVENT <bundle> <op> <record_json> K=<kappa> C=<confidence>
 *   NOTICE <bundle> lagged=<count>
 *
 * Where `op ∈ {insert, update, delete, upsert, bulk_update, bulk_delete}`
 * and `record_json` is the full affected record (or an array for bulk).
 *
 * This module is pure — no DOM, no WebSocket. The actual subscribe()
 * method on the client owns the socket lifecycle and calls back into
 * `parseSubscriptionFrame` for each incoming message.
 */

export type MutationOp =
  | "insert"
  | "update"
  | "delete"
  | "upsert"
  | "bulk_update"
  | "bulk_delete";

export type RecordPayload = Record<string, unknown> | Array<Record<string, unknown>>;

export interface MutationEvent {
  kind: "event";
  bundle: string;
  op: MutationOp;
  record: RecordPayload;
  kappa: number;
  confidence: number;
}

export interface LagNotice {
  kind: "notice";
  bundle: string;
  lagged: number;
}

export type SubscriptionFrame = MutationEvent | LagNotice;

const VALID_OPS = new Set<string>([
  "insert",
  "update",
  "delete",
  "upsert",
  "bulk_update",
  "bulk_delete",
]);

/**
 * Parse a single line from the WS stream. Returns `null` for anything
 * not recognized — callers should silently drop those rather than panic.
 */
export function parseSubscriptionFrame(line: string): SubscriptionFrame | null {
  const trimmed = line.trim();
  if (!trimmed) return null;

  if (trimmed.startsWith("EVENT ")) {
    return parseEvent(trimmed.slice("EVENT ".length));
  }
  if (trimmed.startsWith("NOTICE ")) {
    return parseNotice(trimmed.slice("NOTICE ".length));
  }
  return null;
}

function parseEvent(rest: string): MutationEvent | null {
  // Strip optional " K=<num> C=<num>" suffix from the end, balance-aware
  // so we don't confuse a trailing K=... inside the JSON body.
  const { head, kappa, confidence } = stripKCSuffix(rest);
  // Now `head` is: "<bundle> <op> <record_json>"
  const firstSp = head.indexOf(" ");
  if (firstSp <= 0) return null;
  const bundle = head.slice(0, firstSp);
  const afterBundle = head.slice(firstSp + 1);
  const secondSp = afterBundle.indexOf(" ");
  if (secondSp <= 0) return null;
  const opRaw = afterBundle.slice(0, secondSp);
  if (!VALID_OPS.has(opRaw)) return null;
  const op = opRaw as MutationOp;
  const recordText = afterBundle.slice(secondSp + 1).trim();
  let record: RecordPayload;
  try {
    record = JSON.parse(recordText);
  } catch {
    return null;
  }
  return { kind: "event", bundle, op, record, kappa, confidence };
}

function parseNotice(rest: string): LagNotice | null {
  // "<bundle> lagged=<num>"
  const m = rest.match(/^(\S+)\s+lagged=(\d+)\s*$/);
  if (!m) return null;
  return { kind: "notice", bundle: m[1], lagged: Number.parseInt(m[2], 10) };
}

/**
 * Find the trailing ` K=<num> C=<num>` portion and split it off. We do
 * this from the right so a `record_json` body containing `K=` strings
 * doesn't get sliced apart.
 */
function stripKCSuffix(s: string): {
  head: string;
  kappa: number;
  confidence: number;
} {
  // Match the last occurrence of " K=NUM C=NUM" anchored to end.
  const m = s.match(/^(.*?)\s+K=([-+0-9.eE]+)\s+C=([-+0-9.eE]+)\s*$/);
  if (!m) return { head: s, kappa: 0, confidence: 0 };
  return {
    head: m[1],
    kappa: parseNumOr(m[2], 0),
    confidence: parseNumOr(m[3], 0),
  };
}

function parseNumOr(s: string, fallback: number): number {
  const n = Number(s);
  return Number.isFinite(n) ? n : fallback;
}

/**
 * Apply a mutation event to a row map keyed by `keyField`.
 * Pure function; returns a new array — never mutates the input.
 *
 * For `bulk_*` operations, `record` is expected to be an array;
 * we apply each entry in order.
 */
export function applyEventToRows<R extends Record<string, unknown>>(
  rows: R[],
  keyField: string,
  ev: MutationEvent,
): R[] {
  if (Array.isArray(ev.record)) {
    let next: R[] = rows;
    for (const r of ev.record) {
      next = applyOneRecord(next, keyField, ev.op, r as R);
    }
    return next;
  }
  return applyOneRecord(rows, keyField, ev.op, ev.record as R);
}

function applyOneRecord<R extends Record<string, unknown>>(
  rows: R[],
  keyField: string,
  op: MutationOp,
  record: R,
): R[] {
  const key = String(record[keyField] ?? "");
  if (op === "delete" || op === "bulk_delete") {
    return rows.filter((r) => String(r[keyField] ?? "") !== key);
  }
  const idx = rows.findIndex((r) => String(r[keyField] ?? "") === key);
  if (idx < 0) {
    // insert / upsert / update of a row we don't have yet → append
    return rows.concat(record);
  }
  // update / upsert / bulk_update → merge over existing
  const next = rows.slice();
  next[idx] = { ...rows[idx], ...record };
  return next;
}
