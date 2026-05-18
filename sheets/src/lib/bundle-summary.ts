/**
 * Build a high-fidelity markdown summary of a bundle for an AI agent.
 *
 * The output is small (target < 8 KB) and self-contained: schema,
 * shape, κ stats, a stratified sample, and a quick note about which
 * Prism workflows are applicable. An AI given this blob can answer
 * "what is this?" with real specifics — schema, distributions,
 * representative rows — without having to crawl the SPA.
 *
 * Encryption is respected: OPAQUE columns appear in the schema with
 * type + name, but their values are masked. See CRAWLABILITY_SPEC.md.
 */

import type { BundleSchema, FieldDescriptor, RowMap } from "./gigi-client";
import { kappaClass } from "./kappa";

export interface BundleSummaryOpts {
  bundle: string;
  schema: BundleSchema;
  rows: RowMap[];
  /** Per-row κ values, keyed by primary key. */
  kappaMap: Map<string, number>;
  /** Field names hidden from view; omitted from the summary too. */
  hiddenFields?: Set<string>;
  /**
   * Optional encryption overlay keyed by field name. Maps a field name
   * to a mode string (det / ored / opaque). When the engine's
   * field-level encryption ships, this can read from the schema directly.
   */
  encryption?: Map<string, string>;
  /** Cover field — used to stratify the sample. Falls back to round-robin. */
  coverField?: string | null;
  /**
   * If provided, the URL where the bundle lives (e.g.
   * "https://gigi.davisgeometric.com/gigi/sheets/payments_q2"). Included
   * verbatim in the summary header.
   */
  url?: string;
  /** Sample size. Default 10. Capped at 25. */
  sampleSize?: number;
}

const MAX_SAMPLE = 25;
const DEFAULT_SAMPLE = 10;
const OPAQUE_DISPLAY = "••••••••";

/** Public entry point. Returns a markdown string. */
export function buildBundleSummary(opts: BundleSummaryOpts): string {
  const {
    bundle,
    schema,
    rows,
    kappaMap,
    hiddenFields,
    encryption,
    coverField,
    url,
    sampleSize,
  } = opts;
  const sampleN = Math.min(sampleSize ?? DEFAULT_SAMPLE, MAX_SAMPLE);

  const visibleFields = filterFields(schema, hiddenFields);
  const keyField = schema.base_fields[0]?.name ?? "";
  const lines: string[] = [];

  lines.push(`# Bundle: ${bundle}`);
  if (url) lines.push(`URL: ${url}`);
  lines.push(`Source: GIGI Sheets`);
  lines.push(`Rows: ${rows.length.toLocaleString()}`);
  lines.push(`Fields: ${visibleFields.length}`);
  lines.push("");

  // ── Schema table ─────────────────────────────────────────────────
  lines.push(`## Schema`);
  lines.push("");
  lines.push("| Field | Type | Encryption | Sample |");
  lines.push("|---|---|---|---|");
  for (const f of visibleFields) {
    const isKey = f.name === keyField;
    const typeLabel = isKey ? `${f.type} (key)` : f.type;
    const enc = encryptionLabel(f, encryption);
    const sample = sampleValue(f, rows, encryption);
    lines.push(
      `| ${escapeMd(f.name)} | ${typeLabel} | ${enc} | ${escapeMd(sample)} |`,
    );
  }
  lines.push("");

  // ── Shape: cohorts + κ distribution ──────────────────────────────
  lines.push(`## Shape`);
  lines.push("");
  if (coverField && visibleFields.some((f) => f.name === coverField)) {
    const cohorts = groupBy(rows, coverField);
    const labels = Array.from(cohorts.keys()).slice(0, 8);
    const summary = labels
      .map((l) => `${l} (${cohorts.get(l)!.length})`)
      .join(", ");
    const more =
      cohorts.size > labels.length
        ? `, +${cohorts.size - labels.length} more`
        : "";
    lines.push(`- Cover field: \`${coverField}\``);
    lines.push(`- Cohorts: ${summary}${more}`);
  } else if (coverField) {
    lines.push(`- Cover field: \`${coverField}\` (hidden from view)`);
  } else {
    lines.push(`- Cover field: not set`);
  }

  // κ distribution
  if (kappaMap.size > 0) {
    let healthy = 0;
    let drift = 0;
    let anomaly = 0;
    let sum = 0;
    let max = 0;
    let maxKey = "";
    for (const [k, v] of kappaMap) {
      const cls = kappaClass(v);
      if (cls === "ok") healthy++;
      else if (cls === "warn") drift++;
      else anomaly++;
      sum += v;
      if (v > max) {
        max = v;
        maxKey = k;
      }
    }
    const mean = sum / kappaMap.size;
    lines.push(
      `- κ distribution: ${healthy} healthy · ${drift} drift · ${anomaly} anomaly`,
    );
    lines.push(`- κ̄ (mean curvature): ${mean.toFixed(3)}`);
    if (maxKey) {
      lines.push(`- Max κ: ${max.toFixed(3)} (${escapeMd(maxKey)})`);
    }
  }
  lines.push("");

  // ── Representative sample ────────────────────────────────────────
  lines.push(`## Sample (${Math.min(sampleN, rows.length)} of ${rows.length} rows)`);
  lines.push("");
  const sample = stratifiedSample(rows, coverField, sampleN);
  for (const r of sample) {
    lines.push(`- ${formatRowOneLine(r, visibleFields, keyField, encryption)}`);
  }
  lines.push("");

  // ── Prism workflow note ──────────────────────────────────────────
  lines.push(`## Prism workflows`);
  lines.push("");
  lines.push(prismApplicability(visibleFields, rows.length));
  lines.push("");

  // ── Davis math snapshot ─────────────────────────────────────────
  lines.push(`## Davis math`);
  lines.push("");
  lines.push(
    "GIGI Sheets uses the Davis double-cover identity: **S + d² = 1**, where",
  );
  lines.push("");
  lines.push("- `S(a, b) = (1 + cos θ)/2` — sameness between two rows in [0, 1]");
  lines.push("- `d(a, b) = √(1 − S)` — Davis distance in [0, 1]");
  lines.push(
    "- `κ(r) = 1 − S(r, cohort_centroid)` — curvature, how far a row sits from its cohort",
  );
  lines.push("");
  lines.push(
    "Anomaly bands above are computed using κ thresholds (warn ≥ 0.8, bad ≥ 2.0).",
  );

  return lines.join("\n");
}

/** Wrap the summary in a tiny prompt prefix so the user can paste it
 *  into any AI tool and get a useful answer. */
export function buildAiPrompt(summary: string, bundle: string): string {
  return `I'm sharing a data bundle from GIGI Sheets called "${bundle}". Below is a high-fidelity summary including the schema, distribution, and a representative sample. Please read it carefully and help me understand: (1) what this bundle is, (2) what's interesting or unusual about it, and (3) what questions I should ask of the data.

---

${summary}
`;
}

// ── helpers ────────────────────────────────────────────────────────

function filterFields(
  schema: BundleSchema,
  hidden: Set<string> | undefined,
): FieldDescriptor[] {
  const all = [...schema.base_fields, ...schema.fiber_fields];
  if (!hidden || hidden.size === 0) return all;
  const keyField = schema.base_fields[0]?.name;
  return all.filter((f) => f.name === keyField || !hidden.has(f.name));
}

function encryptionLabel(
  f: FieldDescriptor,
  overlay?: Map<string, string>,
): string {
  const mode = overlay?.get(f.name) ?? f.encryption ?? "none";
  if (mode === "opaque") return "**opaque** (masked)";
  if (mode === "ored") return "ored (equality-searchable)";
  if (mode === "det") return "det";
  return "none";
}

function sampleValue(
  f: FieldDescriptor,
  rows: RowMap[],
  overlay?: Map<string, string>,
): string {
  const mode = overlay?.get(f.name) ?? f.encryption ?? "none";
  if (mode === "opaque") return OPAQUE_DISPLAY;
  if (mode === "ored") return "<encrypted>";
  // First non-null value, capped to 32 chars.
  for (const r of rows) {
    const v = r[f.name];
    if (v == null || v === "") continue;
    const s = String(v);
    return s.length > 32 ? s.slice(0, 30) + "…" : s;
  }
  return "—";
}

function groupBy(rows: RowMap[], field: string): Map<string, RowMap[]> {
  const out = new Map<string, RowMap[]>();
  for (const r of rows) {
    const k = String(r[field] ?? "—");
    const arr = out.get(k) ?? [];
    arr.push(r);
    out.set(k, arr);
  }
  return out;
}

/**
 * Stratified sample: pick `n` rows, distributing as evenly as possible
 * across the cover field's distinct values. Falls back to evenly-spaced
 * row indices if no cover field is set.
 */
export function stratifiedSample(
  rows: RowMap[],
  coverField: string | null | undefined,
  n: number,
): RowMap[] {
  if (rows.length === 0) return [];
  const take = Math.min(n, rows.length);
  if (!coverField) {
    // Evenly-spaced indices: row 0, ⌊len/take⌋, 2⌊len/take⌋, …
    const out: RowMap[] = [];
    const step = Math.max(1, Math.floor(rows.length / take));
    for (let i = 0; out.length < take && i < rows.length; i += step) {
      out.push(rows[i]);
    }
    return out;
  }
  const groups = groupBy(rows, coverField);
  const groupKeys = Array.from(groups.keys());
  if (groupKeys.length === 0) return rows.slice(0, take);
  const out: RowMap[] = [];
  // Round-robin one row from each cohort until we hit `take`.
  const cursors = new Map<string, number>(groupKeys.map((k) => [k, 0]));
  while (out.length < take) {
    let progressed = false;
    for (const k of groupKeys) {
      if (out.length >= take) break;
      const idx = cursors.get(k)!;
      const arr = groups.get(k)!;
      if (idx < arr.length) {
        out.push(arr[idx]);
        cursors.set(k, idx + 1);
        progressed = true;
      }
    }
    if (!progressed) break;
  }
  return out;
}

function formatRowOneLine(
  r: RowMap,
  fields: FieldDescriptor[],
  keyField: string,
  overlay?: Map<string, string>,
): string {
  const parts: string[] = [];
  const keyVal = keyField ? String(r[keyField] ?? "") : "";
  if (keyVal) parts.push(keyVal);
  for (const f of fields) {
    if (f.name === keyField) continue;
    const v = r[f.name];
    if (v == null || v === "") continue;
    const mode = overlay?.get(f.name) ?? f.encryption ?? "none";
    if (mode === "opaque") {
      parts.push(`${f.name}=${OPAQUE_DISPLAY}`);
      continue;
    }
    if (mode === "ored") {
      parts.push(`${f.name}=<encrypted>`);
      continue;
    }
    const s = String(v);
    parts.push(`${f.name}=${s.length > 28 ? s.slice(0, 26) + "…" : s}`);
  }
  return parts.join(" · ");
}

function prismApplicability(
  fields: FieldDescriptor[],
  rowCount: number,
): string {
  const names = fields.map((f) => f.name.toLowerCase());
  const has = (pat: RegExp) => names.some((n) => pat.test(n));
  const lines: string[] = [];
  if (has(/reference|memo|invoice/) && has(/amount|usd|value/)) {
    lines.push(
      "- **Dedup** — looks well-suited: reference + amount fields present for canonical match",
    );
  }
  if (fields.some((f) => f.type === "numeric") && rowCount >= 10) {
    lines.push(
      "- **Forecast** — applicable: pick any numeric column, get a 7-step projection",
    );
  }
  if (fields.some((f) => f.type === "categorical")) {
    lines.push(
      "- **Monitor** — applicable: cohorts from the categorical fields enable per-cohort drift",
    );
  }
  lines.push(
    "- **Books** — needs a second bundle to reconcile against (canonical sameness-join)",
  );
  return lines.length === 0 ? "- (no Prism wireup obvious from schema)" : lines.join("\n");
}

/** Escape pipes and newlines so the value doesn't break the markdown table. */
function escapeMd(s: string): string {
  return String(s)
    .replace(/\|/g, "\\|")
    .replace(/\n/g, " ")
    .replace(/\r/g, "");
}
