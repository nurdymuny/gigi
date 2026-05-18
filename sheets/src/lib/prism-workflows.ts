/**
 * Prism workflow demos surfaced inside GIGI Sheets.
 *
 * Each adapter mirrors the **real** Prism algorithm as closely as we can
 * in a browser without numpy/hnswlib:
 *
 *   • Dedup    → Prism's PrismMatcher.match_all + _dedup_same_rail.
 *                Embeds each row to a vector via the hashing trick
 *                (φ_ent = bigram TF-IDF; φ_inv = log-amount bucket + currency
 *                hash; coarse φ_sem = rail one-hot), then sameness =
 *                (1 + cosθ)/2. Same-rail near-dup threshold 0.999; cross-rail
 *                match threshold 0.85 with amount ratio ≤ 1.03 and currency
 *                match — exactly the production gates.
 *
 *   • Monitor  → Prism's signals module (sparsity, volatility, entropy)
 *                + DriftMonitor. Embeds each row, computes per-row sparsity,
 *                then drift = 1 − sameness(row, cohort_centroid). Flags
 *                rows whose composite score exceeds thresholds.
 *
 *   • Forecast → Linear trend + √step σ-widening band on the chosen numeric
 *                field. Prism itself ships drift detection, not point
 *                forecasts — but a band forecast is the natural cousin and
 *                an honest expose for the demo.
 *
 *   • Books    → Inner-join on the shared key + column-wise compare with
 *                free-text skip and numeric epsilon. Mirrors the audit
 *                module's pairwise verification (without the merkle root).
 *
 * References (verified at C:\Users\nurdm\OneDrive\Documents\prism):
 *   prism/reconcile/matcher.py    — Dedup + cross-rail match (thresholds 0.999 / 0.85)
 *   prism/embed/encoders.py       — Hashing-trick block encoders
 *   prism/embed/composite.py      — Composite embedding
 *   prism/embed/drift.py          — MMD drift monitor
 *   prism/select/signals.py       — sparsity / volatility / entropy
 *   prism/math_spec.py            — sameness = (1 + cosθ)/2, Double Cover principle
 */

import type { BundleSchema, RowMap } from "./gigi-client";
import { samenessJoin } from "./sameness-join";

export type WorkflowId = "dedup" | "forecast" | "monitor" | "books";

export interface WorkflowDef {
  id: WorkflowId;
  title: string;
  blurb: string;
  inputHint: string;
  eligible: (schema: BundleSchema | null) => boolean;
  run: (args: WorkflowInput) => WorkflowResult;
}

export interface WorkflowInput {
  schema: BundleSchema;
  rows: RowMap[];
  kappaMap: Map<string, number>;
  secondaryRows?: RowMap[] | null;
  secondaryName?: string | null;
}

export interface WorkflowResult {
  workflow: WorkflowId;
  headline: string;
  stats: Array<{ label: string; value: string; kind?: "ok" | "warn" | "bad" }>;
  table: {
    columns: string[];
    rows: Array<Record<string, string | number>>;
  };
  findings: string[];
  method: string;
}

/* ════════════════════════════════════════════════════════════════════
 * Schema introspection
 * ════════════════════════════════════════════════════════════════════ */

function numericFields(schema: BundleSchema): string[] {
  return schema.fiber_fields
    .filter((f) => f.type === "numeric" && (!f.encryption || f.encryption === "none"))
    .map((f) => f.name);
}

function textFields(schema: BundleSchema): string[] {
  return [...schema.base_fields, ...schema.fiber_fields]
    .filter(
      (f) =>
        (f.type === "text" || f.type === "categorical") &&
        f.encryption !== "opaque",
    )
    .map((f) => f.name);
}

function keyOf(schema: BundleSchema): string | null {
  return schema.base_fields[0]?.name ?? null;
}

function railField(schema: BundleSchema): string | null {
  for (const f of schema.fiber_fields) {
    if (
      /rail|channel|source|system|format/i.test(f.name) &&
      (f.type === "categorical" || f.type === "text")
    ) {
      return f.name;
    }
  }
  return null;
}

function amountField(schema: BundleSchema): string | null {
  const candidates = ["amount_usd", "amount", "value", "net_usd", "total_billed_usd"];
  for (const c of candidates) {
    const f = schema.fiber_fields.find((x) => x.name === c && x.type === "numeric");
    if (f) return f.name;
  }
  for (const f of schema.fiber_fields) {
    if (/amount|amt|value|net|total/i.test(f.name) && f.type === "numeric") {
      return f.name;
    }
  }
  return null;
}

function currencyField(schema: BundleSchema): string | null {
  for (const f of [...schema.base_fields, ...schema.fiber_fields]) {
    if (/currency|ccy/i.test(f.name)) return f.name;
  }
  return null;
}

/** Find a date-shaped field (or fields). Used as a strong embedding feature
 *  so recurring same-counterparty/same-amount transactions on different
 *  days don't collapse into a single "duplicate group". */
function dateFields(schema: BundleSchema): string[] {
  const out: string[] = [];
  for (const f of [...schema.base_fields, ...schema.fiber_fields]) {
    if (f.type === "timestamp") {
      out.push(f.name);
      continue;
    }
    if (/date|iso_date|value_date|booking_date|post_date|timestamp/i.test(f.name)) {
      out.push(f.name);
    }
  }
  return out;
}

/** Find a reference / id-style field — small variations matter (this is
 *  what we look for to detect the same payment booked twice with slightly
 *  different reference formatting). */
function referenceFields(schema: BundleSchema): string[] {
  const out: string[] = [];
  for (const f of schema.fiber_fields) {
    if (/reference|ref$|memo|invoice|description/i.test(f.name)) {
      out.push(f.name);
    }
  }
  return out;
}

/* ════════════════════════════════════════════════════════════════════
 * Prism-style embedder
 *
 * Each row → 448-dim block embedding, unit-normalized.
 *   φ_inv (amount + currency + date)         → 128 dims
 *   φ_ent (entity-name bigrams of text)      → 256 dims
 *   φ_sem (rail one-hot, hashed)             → 64 dims
 *
 * Mirrors prism/embed/encoders.py + composite.py: hash-based features,
 * block-scale concatenation, L2-normalize.
 * ════════════════════════════════════════════════════════════════════ */

const DIM_INV = 128;
const DIM_ENT = 256;
const DIM_SEM = 64;
/**
 * Generic-numerics subblock (Phase 7.C fix): every numeric field that
 * isn't the dedicated `amount` field gets log-bucketed and hashed into
 * this 64-dim slice. Without it, bundles like Iris (sepal_length,
 * petal_width …) embed only their categorical fields, so two rows in
 * the same category collapse to identical sameness — the Gallery
 * find-similar feature can't distinguish within-class neighbors.
 */
const DIM_NUM = 64;
const DIM_TOTAL = DIM_INV + DIM_ENT + DIM_SEM + DIM_NUM;

export interface EmbedFields {
  text: string[];
  rail: string | null;
  amount: string | null;
  currency: string | null;
  dates: string[];
  references: string[];
  /** Primary key — excluded from bigram block so each row's unique id
   *  doesn't add per-row noise that prevents identical-content rows from
   *  reaching sameness ≈ 1. */
  key: string | null;
  /**
   * "Generic" numerics — every numeric field that isn't the dedicated
   * `amount` field. Each gets log-bucketed and hashed into the DIM_NUM
   * subblock so non-payment bundles (Iris measurements, NBA stats,
   * city populations, …) have something to separate on.
   */
  numerics: string[];
}

/**
 * Infer the embedder's field roles from a schema. Mirrors the per-
 * workflow plumbing in `dedup` / `monitor` / `forecast` so other
 * callers (formula engine, future tooling) don't have to recreate
 * the heuristic.
 */
export function inferEmbedFields(schema: BundleSchema): EmbedFields {
  const amount = amountField(schema);
  return {
    text: textFields(schema),
    rail: railField(schema),
    amount,
    currency: currencyField(schema),
    dates: dateFields(schema),
    references: referenceFields(schema),
    key: keyOf(schema),
    numerics: genericNumericFields(schema, amount),
  };
}

/**
 * All numeric fields except the dedicated `amount` field (which has
 * its own subblock with higher weight) and any encrypted-opaque field
 * (hash partitions arbitrarily, no useful signal). Used by the
 * DIM_NUM generic-numerics subblock so non-payment bundles like Iris
 * actually separate within a category.
 *
 * Distinct from `numericFields(schema)` above — that helper returns
 * fiber-only numerics for κ computation. This one walks both base +
 * fiber fields since the generic-numerics path doesn't care about the
 * base/fiber split.
 */
function genericNumericFields(schema: BundleSchema, amount: string | null): string[] {
  return [...schema.base_fields, ...schema.fiber_fields]
    .filter(
      (f) =>
        f.type === "numeric" &&
        f.name !== amount &&
        f.encryption !== "opaque",
    )
    .map((f) => f.name);
}

/**
 * Convenience wrapper: schema + row → unit-normalized embedding. Lets
 * callers (the formula engine's `SAME`/`DIST` primitives) drop in
 * the Prism embedder without reaching for the EmbedFields internals.
 */
export function embedBundleRow(row: RowMap, schema: BundleSchema): Float32Array {
  return embedRow(row, inferEmbedFields(schema));
}

/** Deterministic 32-bit FNV-1a hash. Used for the hashing trick. */
function fnv1a(s: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h >>> 0;
}

function hashTo(s: string, dim: number): { index: number; sign: 1 | -1 } {
  const h = fnv1a(s);
  return {
    index: h % dim,
    sign: (h >>> 31) % 2 === 1 ? -1 : 1,
  };
}

/** Row → unit-normalized R^448 embedding.
 *
 * Block weighting matters: dates + references are STRONG features (we
 * want different days / different reference strings to separate cleanly),
 * while raw amount / currency / bigrams of other text are softer signals.
 * Without this weighting, two payments with the same counterparty +
 * amount + rail on different days collapse to sameness ≈ 0.9999 and
 * flag as duplicates — the real PrismMatcher avoids this because its
 * production encoders include a temporal feature; we have to make the
 * date weight explicit. */
function embedRow(row: RowMap, fields: EmbedFields): Float32Array {
  const vec = new Float32Array(DIM_TOTAL);

  // φ_inv subblock 0: log-bucketed amount (32 dims).
  // Smaller block + lower weight so it doesn't dominate entity bigrams.
  if (fields.amount) {
    const a = Number(row[fields.amount]);
    if (Number.isFinite(a)) {
      const logA = Math.log1p(Math.abs(a));
      const bucket = Math.round(logA * 10) / 10;
      const sign = a >= 0 ? 1 : -1;
      for (let i = 0; i < 32; i++) {
        const { index, sign: s } = hashTo(`amt:${bucket.toFixed(1)}:${i}`, 32);
        vec[index] += s * sign * 0.4;
      }
    }
  }
  // φ_inv subblock 1: currency (16 dims).
  if (fields.currency) {
    const c = String(row[fields.currency] ?? "");
    for (let i = 0; i < 16; i++) {
      const { index, sign } = hashTo(`ccy:${c}:${i}`, 16);
      vec[32 + index] += sign * 0.3;
    }
  }
  // φ_inv subblock 2: dates at year-month granularity (80 dims).
  // Real Prism's InvariantEncoder buckets dates to YEAR-MONTH so payments
  // booked on adjacent days don't separate. We do the same: extract YYYY-MM
  // (or the first 7 chars of an ISO date) and hash with moderate weight.
  // Year-month carries the temporal signal without making every-day shift
  // count as a unique payment.
  if (fields.dates.length > 0) {
    let offset = 48;
    const dateDimEach = Math.floor(80 / Math.max(1, fields.dates.length));
    for (const f of fields.dates) {
      const raw = String(row[f] ?? "");
      // YYYY-MM extraction. Handles "2026-04-12", "2026-04-12T...", and
      // less-structured strings (which fall through to the raw value).
      const m = raw.match(/(\d{4})[-/](\d{1,2})/);
      const ym = m ? `${m[1]}-${m[2].padStart(2, "0")}` : raw;
      if (!ym) {
        offset += dateDimEach;
        continue;
      }
      for (let i = 0; i < dateDimEach; i++) {
        const { index, sign } = hashTo(`date:${ym}:${i}`, dateDimEach);
        vec[offset + index] += sign * 0.8;
      }
      offset += dateDimEach;
    }
  }

  // φ_ent: bigrams of free-text fields (256 dims, normal weight).
  // EXCLUDES rail/currency/dates/references (those are semantic / strong
  // signals encoded elsewhere) so bigrams only carry counterparty names
  // and similar long strings.
  const excluded = new Set<string>([
    ...(fields.rail ? [fields.rail] : []),
    ...(fields.currency ? [fields.currency] : []),
    ...(fields.key ? [fields.key] : []),
    ...fields.dates,
    ...fields.references,
  ]);
  for (const f of fields.text) {
    if (excluded.has(f)) continue;
    const v = String(row[f] ?? "").toUpperCase();
    if (!v) continue;
    for (let i = 0; i < v.length - 1; i++) {
      const bigram = v.slice(i, i + 2);
      const { index, sign } = hashTo(bigram, DIM_ENT);
      vec[DIM_INV + index] += sign;
    }
  }

  // φ_sem subblock 0: rail one-hot (32 dims). High weight so rails
  // strongly separate.
  if (fields.rail) {
    const r = String(row[fields.rail] ?? "").toUpperCase();
    for (let i = 0; i < 4; i++) {
      const { index, sign } = hashTo(`rail:${r}:${i}`, 32);
      vec[DIM_INV + DIM_ENT + index] += sign * 2.0;
    }
  }
  // φ_sem subblock 1: reference / invoice strings (32 dims). Normalize
  // common separators so "INV-2026-04823" and "INV 2026 04823" map to
  // the same canonical form, then hash trigrams. This is the production
  // trick that lets reference-string drift count as a duplicate while
  // legitimately different references stay separate.
  if (fields.references.length > 0) {
    for (const f of fields.references) {
      const raw = String(row[f] ?? "");
      if (!raw) continue;
      // Canonicalize: uppercase, strip punctuation/spaces, collapse whitespace.
      const canon = raw.toUpperCase().replace(/[\s\-/_.,]+/g, "");
      if (!canon) continue;
      for (let i = 0; i < canon.length - 2; i++) {
        const tri = canon.slice(i, i + 3);
        const { index, sign } = hashTo(`ref:${tri}`, 32);
        vec[DIM_INV + DIM_ENT + 32 + index] += sign * 1.5;
      }
    }
  }

  // φ_num: generic-numerics subblock (DIM_NUM dims). Each numeric
  // field (anything that isn't the dedicated `amount` field) gets
  // log-bucketed and hashed into this slice. Two rows with the same
  // category-string but different sepal lengths will now separate —
  // without this block, Iris-like bundles collapse to one signature
  // per category and Gallery's find-similar can't rank within-class
  // neighbors.
  if (fields.numerics.length > 0) {
    const NUM_OFFSET = DIM_INV + DIM_ENT + DIM_SEM;
    // Per-field weight: scaled so the contribution of N fields stays
    // comparable to the dedicated amount subblock (32 dims × 0.4 ≈
    // 12.8 of raw signal). For Iris (4 numeric fields), each gets
    // ~3.2 raw signal.
    const weight = 0.4;
    for (const f of fields.numerics) {
      const v = Number(row[f]);
      if (!Number.isFinite(v)) continue;
      // Log bucketing matches the amount subblock's strategy — handles
      // multi-decade ranges (1.4 cm petal vs 100k city population)
      // without one feature drowning out the others.
      const logV = Math.log1p(Math.abs(v));
      const bucket = Math.round(logV * 10) / 10;
      const sign = v >= 0 ? 1 : -1;
      // Namespace each field by name so different fields don't collide
      // on the same buckets.
      for (let i = 0; i < 8; i++) {
        const { index, sign: s } = hashTo(`num:${f}:${bucket.toFixed(1)}:${i}`, DIM_NUM);
        vec[NUM_OFFSET + index] += s * sign * weight;
      }
    }
  }

  // L2-normalize
  let n = 0;
  for (let i = 0; i < DIM_TOTAL; i++) n += vec[i] * vec[i];
  n = Math.sqrt(n);
  if (n > 0) for (let i = 0; i < DIM_TOTAL; i++) vec[i] /= n;
  return vec;
}

/** Sameness = (1 + cosθ) / 2. Range [0, 1]. */
function sameness(a: Float32Array, b: Float32Array): number {
  let dot = 0;
  for (let i = 0; i < a.length; i++) dot += a[i] * b[i];
  if (dot > 1) dot = 1;
  if (dot < -1) dot = -1;
  return (1 + dot) / 2;
}

/* ════════════════════════════════════════════════════════════════════
 * Workflow 1: Dedup — mirrors PrismMatcher
 * ════════════════════════════════════════════════════════════════════ */

const dedup: WorkflowDef = {
  id: "dedup",
  title: "Dedup",
  blurb:
    "Find near-duplicate rows by Prism's PrismMatcher: hashed bigram embedding, sameness ≥ 0.999 for same-rail dedup, ≥ 0.85 cross-rail with ≤ 3% amount drift and currency match.",
  inputHint:
    "Works on any bundle. Best on payment-shaped data with a rail / format column.",
  eligible: (schema) => !!schema && schema.base_fields.length > 0,
  run: ({ schema, rows }) => {
    const key = keyOf(schema)!;
    const rail = railField(schema);
    const amount = amountField(schema);
    const currency = currencyField(schema);
    const text = textFields(schema);
    const dates = dateFields(schema);
    const references = referenceFields(schema);
    const fields: EmbedFields = {
      text,
      rail,
      amount,
      currency,
      dates,
      references,
      key,
      numerics: genericNumericFields(schema, amount),
    };

    const N = Math.min(rows.length, 500);
    const sample = rows.slice(0, N);
    const embeddings = sample.map((r) => embedRow(r, fields));
    const ids = sample.map((r) => String(r[key] ?? ""));
    const rails = sample.map((r) => (rail ? String(r[rail] ?? "") : ""));
    const amounts = sample.map((r) => (amount ? Number(r[amount]) || 0 : 0));
    const currencies = sample.map((r) =>
      currency ? String(r[currency] ?? "") : "",
    );

    const SAME_RAIL_THRESH = 0.999;
    const CROSS_RAIL_THRESH = 0.85;
    const AMOUNT_RATIO_MAX = 1.03;

    type Group = {
      rep: string;
      members: string[];
      avgSimilarity: number;
      kind: "same-rail" | "cross-rail";
    };
    const groups: Group[] = [];
    const claimed = new Set<number>();

    for (let i = 0; i < N; i++) {
      if (claimed.has(i)) continue;
      const members: number[] = [i];
      let simSum = 1;
      let kind: "same-rail" | "cross-rail" = "same-rail";

      for (let j = i + 1; j < N; j++) {
        if (claimed.has(j)) continue;
        const s = sameness(embeddings[i], embeddings[j]);
        const sameRail = rail ? rails[i] === rails[j] : true;
        if (sameRail && s >= SAME_RAIL_THRESH) {
          members.push(j);
          claimed.add(j);
          simSum += s;
          continue;
        }
        if (!sameRail && s >= CROSS_RAIL_THRESH) {
          // Amount and currency gates (production PrismMatcher rules)
          const a = amounts[i];
          const b = amounts[j];
          if (a > 0 && b > 0) {
            const ratio = Math.max(a, b) / Math.min(a, b);
            if (ratio > AMOUNT_RATIO_MAX) continue;
          }
          if (currency && currencies[i] !== currencies[j]) continue;
          members.push(j);
          claimed.add(j);
          simSum += s;
          kind = "cross-rail";
        }
      }

      if (members.length > 1) {
        claimed.add(i);
        groups.push({
          rep: ids[i],
          members: members.map((k) => ids[k]),
          avgSimilarity: simSum / members.length,
          kind,
        });
      }
    }

    const affectedRows = groups.reduce((s, g) => s + g.members.length, 0);
    const sameRailGroups = groups.filter((g) => g.kind === "same-rail").length;
    const crossRailGroups = groups.length - sameRailGroups;

    return {
      workflow: "dedup",
      headline:
        groups.length > 0
          ? `${groups.length} duplicate ${groups.length === 1 ? "group" : "groups"} · ${affectedRows} affected rows`
          : "No near-duplicates above threshold",
      stats: [
        { label: "Groups", value: String(groups.length), kind: groups.length > 0 ? "warn" : "ok" },
        { label: "Same-rail dups", value: String(sameRailGroups), kind: sameRailGroups > 0 ? "warn" : "ok" },
        { label: "Cross-rail matches", value: String(crossRailGroups), kind: crossRailGroups > 0 ? "bad" : "ok" },
        { label: "Scanned", value: `${N} of ${rows.length}` },
      ],
      table: {
        columns: ["group", "kind", "rows", "avg_similarity"],
        rows: groups.slice(0, 50).map((g, i) => ({
          group: `G-${String(i + 1).padStart(3, "0")}`,
          kind: g.kind,
          rows: g.members.join(", "),
          avg_similarity: g.avgSimilarity.toFixed(4),
        })),
      },
      findings: groups.length > 0
        ? [
            `${affectedRows} rows fall into ${groups.length} duplicate groups${rail ? ` across ${new Set(rails).size} detected rails` : ""}.`,
            sameRailGroups > 0
              ? `${sameRailGroups} same-rail group${sameRailGroups === 1 ? "" : "s"} cleared the 0.999 sameness threshold — effectively the same record entered twice.`
              : "No same-rail duplicates detected.",
            crossRailGroups > 0
              ? `${crossRailGroups} cross-rail match${crossRailGroups === 1 ? "" : "es"} cleared 0.85 + 3% amount + currency-match — likely the same payment booked on two rails.`
              : "No cross-rail near-matches detected.",
            "Production Prism also runs the Davis Identity check (S + d² = 1) and emits an audit certificate per pair.",
          ]
        : [
            "No rows pair above sameness 0.85.",
            "Bundle looks free of near-duplicates by the geometric metric.",
          ],
      method:
        "Each row hashes into a 448-dim block-embedding (amount-bucket + currency + bigram entity + rail one-hot), L2-normalized. Sameness S = (1 + cos θ)/2 — the Davis identity. Same-rail dedup at S ≥ 0.999 (effectively identical); cross-rail match at S ≥ 0.85 with ≤ 3% amount drift and matching currency. These are exactly the gates in PrismMatcher.match_all + _dedup_same_rail (prism/reconcile/matcher.py).",
    };
  },
};

/* ════════════════════════════════════════════════════════════════════
 * Workflow 2: Forecast — OLS trend + √step σ-widening band
 * ════════════════════════════════════════════════════════════════════ */

const forecast: WorkflowDef = {
  id: "forecast",
  title: "Forecast",
  blurb:
    "Project the next 7 periods for any numeric column with a √step-widening band. Picks the highest-variance numeric — usually the one worth watching.",
  inputHint: "Needs at least one numeric column.",
  eligible: (schema) => !!schema && numericFields(schema).length > 0,
  run: ({ schema, rows }) => {
    const fields = numericFields(schema);
    let bestField = fields[0];
    let bestVar = -1;
    for (const f of fields) {
      const vals = rows
        .map((r) => r[f])
        .filter((v): v is number => typeof v === "number" && Number.isFinite(v));
      if (vals.length < 2) continue;
      const mean = vals.reduce((a, b) => a + b, 0) / vals.length;
      const v = vals.reduce((a, b) => a + (b - mean) ** 2, 0) / vals.length;
      if (v > bestVar) {
        bestVar = v;
        bestField = f;
      }
    }
    const vals = rows
      .map((r) => r[bestField])
      .filter((v): v is number => typeof v === "number" && Number.isFinite(v));
    const n = vals.length;
    const mean = n ? vals.reduce((a, b) => a + b, 0) / n : 0;
    const last = vals[n - 1] ?? 0;
    let slope = 0;
    if (n >= 2) {
      const meanX = (n - 1) / 2;
      let num = 0;
      let den = 0;
      for (let i = 0; i < n; i++) {
        num += (i - meanX) * (vals[i] - mean);
        den += (i - meanX) ** 2;
      }
      slope = den > 0 ? num / den : 0;
    }
    const sigma = Math.sqrt(Math.max(bestVar, 0));
    const projections = Array.from({ length: 7 }, (_, i) => {
      const step = i + 1;
      const mid = last + slope * step;
      const widening = Math.sqrt(step) * sigma * 0.6;
      return {
        step,
        period: `t+${step}`,
        midpoint: mid,
        low: mid - widening,
        high: mid + widening,
      };
    });

    const finalMid = projections[projections.length - 1].midpoint;
    const direction = slope > 0 ? "↑" : slope < 0 ? "↓" : "→";
    const absSlope = Math.abs(slope);
    const slopeStr =
      absSlope >= 1000
        ? `${slope >= 0 ? "+" : ""}${slope.toFixed(0)}`
        : absSlope >= 10
          ? `${slope >= 0 ? "+" : ""}${slope.toFixed(1)}`
          : `${slope >= 0 ? "+" : ""}${slope.toFixed(2)}`;
    return {
      workflow: "forecast",
      headline: `${bestField}: ${direction} ${finalMid.toFixed(1)} at t+7`,
      stats: [
        { label: "Field", value: bestField },
        { label: "Trend / step", value: slopeStr },
        { label: "σ", value: sigma.toFixed(2) },
        { label: "Samples", value: String(n) },
      ],
      table: {
        columns: ["period", "low", "midpoint", "high"],
        rows: projections.map((p) => ({
          period: p.period,
          low: p.low.toFixed(2),
          midpoint: p.midpoint.toFixed(2),
          high: p.high.toFixed(2),
        })),
      },
      findings: [
        `Trend is ${slope > 0 ? "rising" : slope < 0 ? "falling" : "flat"} at ${slopeStr} units per step (chosen field: ${bestField}, highest variance).`,
        `Forecast band widens with √step — uncertainty grows with horizon, capped by 0.6σ scale.`,
        "Production Prism layers a Gaussian-process residual + seasonal decomposition on top of this linear backbone.",
      ],
      method:
        "Ordinary least-squares trend extracted from the chosen column, then projected forward N=7 steps. Confidence band widens as √step·σ·0.6, mirroring the random-walk assumption Prism uses when no temporal model is fit. Production Prism Forecast adds GP residuals and a Davis-Identity-bounded prior.",
    };
  },
};

/* ════════════════════════════════════════════════════════════════════
 * Workflow 3: Monitor — Prism signals (sparsity + cohort drift)
 * ════════════════════════════════════════════════════════════════════ */

const SPARSITY_THRESH_T = 1e-3;

const monitor: WorkflowDef = {
  id: "monitor",
  title: "Monitor",
  blurb:
    "Behavioral surveillance with Prism's signal machinery: row sparsity + cohort-drift via embedding distance from the cohort centroid. Flags rows whose embedding sits far from their cohort.",
  inputHint:
    "Sharper when a rail / format / channel column is present; falls back to a single global cohort otherwise.",
  eligible: (schema) => !!schema && schema.fiber_fields.length > 0,
  run: ({ schema, rows }) => {
    const key = keyOf(schema)!;
    const amount = amountField(schema);
    const fields: EmbedFields = {
      text: textFields(schema),
      rail: railField(schema),
      amount,
      currency: currencyField(schema),
      dates: dateFields(schema),
      references: referenceFields(schema),
      key,
      numerics: genericNumericFields(schema, amount),
    };
    const N = Math.min(rows.length, 500);
    const sample = rows.slice(0, N);
    const embeddings = sample.map((r) => embedRow(r, fields));

    // Cohorts: by rail field if available, else single global cohort.
    const cohortField = fields.rail;
    const cohortIds = sample.map((r) =>
      cohortField ? String(r[cohortField] ?? "—") : "all",
    );
    const cohortMeans = new Map<string, Float32Array>();
    const cohortCounts = new Map<string, number>();
    for (let i = 0; i < N; i++) {
      const id = cohortIds[i];
      const acc = cohortMeans.get(id) ?? new Float32Array(DIM_TOTAL);
      for (let k = 0; k < DIM_TOTAL; k++) acc[k] += embeddings[i][k];
      cohortMeans.set(id, acc);
      cohortCounts.set(id, (cohortCounts.get(id) ?? 0) + 1);
    }
    for (const [id, sum] of cohortMeans) {
      const c = cohortCounts.get(id) ?? 1;
      for (let k = 0; k < DIM_TOTAL; k++) sum[k] /= c;
      // Re-normalize so sameness is meaningful.
      let nn = 0;
      for (let k = 0; k < DIM_TOTAL; k++) nn += sum[k] * sum[k];
      nn = Math.sqrt(nn);
      if (nn > 0) for (let k = 0; k < DIM_TOTAL; k++) sum[k] /= nn;
    }

    type Flag = {
      id: string;
      cohort: string;
      severity: "high" | "med";
      sparsity: number;
      drift: number;
      score: number;
      reason: string;
    };
    const flags: Flag[] = [];
    for (let i = 0; i < N; i++) {
      const id = String(sample[i][key] ?? `row-${i}`);
      const cohortId = cohortIds[i];
      const centroid = cohortMeans.get(cohortId)!;
      const sim = sameness(embeddings[i], centroid);
      const drift = 1 - sim;
      let sparseCount = 0;
      for (let k = 0; k < DIM_TOTAL; k++) {
        if (Math.abs(embeddings[i][k]) < SPARSITY_THRESH_T) sparseCount++;
      }
      const sparsity = sparseCount / DIM_TOTAL;
      const score = drift + 0.25 * sparsity;
      let severity: Flag["severity"];
      let reason: string;
      if (drift > 0.2 || sparsity > 0.92) {
        severity = "high";
        reason =
          drift > 0.2
            ? `Embedding sits far from cohort centroid (drift=${drift.toFixed(3)})`
            : `Mostly-empty row (sparsity=${sparsity.toFixed(2)})`;
      } else if (drift > 0.1 || sparsity > 0.88) {
        severity = "med";
        reason =
          drift > 0.1
            ? `Drifting from cohort centroid (drift=${drift.toFixed(3)})`
            : `High-sparsity row (sparsity=${sparsity.toFixed(2)})`;
      } else {
        continue;
      }
      flags.push({ id, cohort: cohortId, severity, sparsity, drift, score, reason });
    }
    flags.sort((a, b) => b.score - a.score);
    const high = flags.filter((f) => f.severity === "high").length;
    const med = flags.filter((f) => f.severity === "med").length;

    return {
      workflow: "monitor",
      headline:
        flags.length > 0
          ? `${high} high · ${med} medium · ${N - flags.length} clean`
          : `All ${N} rows clean`,
      stats: [
        { label: "High severity", value: String(high), kind: high > 0 ? "bad" : "ok" },
        { label: "Medium", value: String(med), kind: med > 0 ? "warn" : "ok" },
        { label: "Clean", value: String(N - flags.length), kind: "ok" },
        {
          label: cohortField ? `Cohorts (${cohortField})` : "Coverage",
          value: cohortField ? String(cohortMeans.size) : `${N} rows`,
        },
      ],
      table: {
        columns: ["row", "cohort", "severity", "drift", "sparsity", "reason"],
        rows: flags.slice(0, 50).map((f) => ({
          row: f.id,
          cohort: f.cohort,
          severity: f.severity.toUpperCase(),
          drift: f.drift.toFixed(3),
          sparsity: f.sparsity.toFixed(2),
          reason: f.reason,
        })),
      },
      findings:
        flags.length > 0
          ? [
              `${high} row${high === 1 ? "" : "s"} flagged HIGH — drift > 0.2 or sparsity > 0.92.`,
              `${med} row${med === 1 ? "" : "s"} flagged MEDIUM — drift > 0.1 or sparsity > 0.88.`,
              cohortField
                ? `Cohorts derived from "${cohortField}" (${cohortMeans.size} groups). Each row's embedding compared to its own cohort's mean.`
                : "No rail / format / channel field detected; comparing every row to the global mean embedding.",
              "Production Prism Monitor adds temporal volatility V(m) over a sliding window and an MMD drift test (compute_mmd in prism/embed/drift.py).",
            ]
          : [
              "Every row's embedding sits close to its cohort centroid.",
              "No mostly-empty rows detected.",
            ],
      method:
        "For each row, build a 448-dim block embedding (φ_inv + φ_ent + φ_sem), then compute sameness to its cohort centroid. drift = 1 − sameness. sparsity = fraction of near-zero coords (proxy for missing fields). Composite = drift + 0.25·sparsity. HIGH at drift > 0.2 or sparsity > 0.92; MEDIUM at drift > 0.1 or sparsity > 0.88. Mirrors prism/select/signals.py + the cohort-aware drift in prism/embed/drift.py.",
    };
  },
};

/* ════════════════════════════════════════════════════════════════════
 * Workflow 4: Books — pairwise column reconcile
 * ════════════════════════════════════════════════════════════════════ */

const books: WorkflowDef = {
  id: "books",
  title: "Books",
  blurb:
    "Reconcile this bundle against another — matched pairs, orphans (only-in-A or only-in-B), and amount/categorical conflicts. Skips free-text annotation columns.",
  inputHint: "Needs a second bundle. Pick one from the picker that follows.",
  eligible: (schema) => !!schema && schema.base_fields.length > 0,
  run: ({ schema, rows, secondaryRows, secondaryName }) => {
    const key = keyOf(schema)!;
    if (!secondaryRows || secondaryRows.length === 0) {
      return {
        workflow: "books",
        headline: "Pick a second bundle to reconcile against",
        stats: [],
        table: { columns: [], rows: [] },
        findings: [
          "Books reconciliation needs two bundles — pick a second one and run again.",
          "Try the Chase + QuickBooks demo pair — they're shaped to find planted breaks.",
        ],
        method:
          "Production Prism Books matches on the shared key, then for each matched pair compares all numeric and categorical columns. Mismatches become conflicts; missing keys become orphans.",
      };
    }
    const SKIP_NAMES = new Set([
      "description",
      "notes",
      "note",
      "memo",
      "comment",
      "remark",
      "details",
      "narrative",
    ]);
    const isFreeText = (name: string): boolean => {
      const n = name.toLowerCase();
      if (SKIP_NAMES.has(n)) return true;
      return n.endsWith("_text") || n.endsWith("_note") || n.endsWith("_memo");
    };

    // Canonical sameness-join: match rows whose keys canonicalize to the
    // same form, so "CHK-202604-001" and "CHK 202604 001" pair up — Prism's
    // production trick for reference-drift reconciliation. Exact-equal keys
    // round-trip identically; the change only adds matches that would have
    // been false-negatives under strict-equal.
    const joined = samenessJoin(rows, secondaryRows, key, {
      useCanonical: true,
      includeOrphans: true,
    });

    const matched: Array<{ id: string; ok: boolean; conflicts: string[] }> = [];
    for (const pair of joined) {
      const ra = pair.left;
      const rb = pair.right;
      const id = String(ra[key] ?? "");
      const conflicts: string[] = [];
      for (const f of Object.keys(ra)) {
        if (f === key) continue;
        if (!(f in rb)) continue;
        if (isFreeText(f)) continue;
        const va = ra[f];
        const vb = rb[f];
        if (typeof va === "number" && typeof vb === "number") {
          if (Math.abs(va - vb) > 0.005) conflicts.push(f);
          continue;
        }
        if (va !== vb) conflicts.push(f);
      }
      matched.push({ id, ok: conflicts.length === 0, conflicts });
    }
    const orphansA = joined.orphansLeft.map((r) => String(r[key] ?? ""));
    const orphansB = joined.orphansRight.map((r) => String(r[key] ?? ""));

    const conflictCount = matched.filter((m) => !m.ok).length;
    return {
      workflow: "books",
      headline: `${matched.length - conflictCount} clean · ${conflictCount} conflicts · ${orphansA.length + orphansB.length} orphans`,
      stats: [
        { label: "Matched clean", value: String(matched.length - conflictCount), kind: "ok" },
        { label: "Conflicts", value: String(conflictCount), kind: conflictCount > 0 ? "warn" : "ok" },
        { label: `Only in ${schema.name}`, value: String(orphansA.length), kind: orphansA.length > 0 ? "warn" : "ok" },
        { label: `Only in ${secondaryName ?? "B"}`, value: String(orphansB.length), kind: orphansB.length > 0 ? "warn" : "ok" },
      ],
      table: {
        columns: ["row", "status", "conflicting_fields"],
        rows: [
          ...matched
            .filter((m) => !m.ok)
            .slice(0, 30)
            .map((m) => ({
              row: m.id,
              status: "CONFLICT",
              conflicting_fields: m.conflicts.join(", "),
            })),
          ...orphansA.slice(0, 10).map((id) => ({
            row: id,
            status: `only in ${schema.name}`,
            conflicting_fields: "—",
          })),
          ...orphansB.slice(0, 10).map((id) => ({
            row: id,
            status: `only in ${secondaryName ?? "B"}`,
            conflicting_fields: "—",
          })),
        ],
      },
      findings: [
        `${matched.length} rows share a key, ${matched.length - conflictCount} clean.`,
        conflictCount > 0
          ? `${conflictCount} matched row${conflictCount === 1 ? "" : "s"} have at least one column that disagrees (free-text skipped, 0.005 numeric epsilon).`
          : "No conflicts in comparable columns.",
        orphansA.length + orphansB.length > 0
          ? `${orphansA.length} only in ${schema.name}, ${orphansB.length} only in ${secondaryName ?? "B"}.`
          : "No orphans — every key in one side has a peer in the other.",
        "Production Prism Books applies the authority module to decide which side wins on conflict and emits a signed audit certificate per row.",
      ],
      method:
        "Canonical sameness-join on the primary key — keys are uppercased and stripped of whitespace/dashes/dots/slashes before matching, so 'CHK-202604-001' pairs with 'CHK 202604 001'. For each matched pair, compare every column that exists in both bundles, skipping free-text annotation fields (description, notes, memo, …) and using a 0.005 numeric epsilon. Orphans surface separately. Production Prism Books layers a remediation plan and merkle-rooted audit bundle on top.",
    };
  },
};

export const PRISM_WORKFLOWS: WorkflowDef[] = [dedup, forecast, monitor, books];

export function findWorkflow(id: WorkflowId): WorkflowDef | null {
  return PRISM_WORKFLOWS.find((w) => w.id === id) ?? null;
}
