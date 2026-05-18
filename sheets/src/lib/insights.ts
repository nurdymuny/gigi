/**
 * Client-side insights — pure rules computed against the rows + κ map.
 *
 * S9 ships these rules locally; when the engine adds an `INSIGHTS` verb
 * (E-S9a), the drawer can switch to consuming engine output instead and
 * this module turns into a fallback / preview.
 */

import type { BundleSchema, RowMap } from "./gigi-client";
import { kappaClass } from "./kappa";

export type InsightTag = "bad" | "warn" | "geo" | "info";

export interface Insight {
  id: string;
  tag: InsightTag;
  body: string;
  /** Suggested follow-up GQL — copyable for the user. */
  gql?: string;
  /** Higher = more interesting; ranked desc. */
  score: number;
}

export interface InsightsInput {
  bundle: string;
  schema: BundleSchema | null;
  rows: RowMap[];
  kappaMap: Map<string, number>;
  coverField: string;
  meanCurvature: number;
}

/**
 * Compute the ranked list of insights for the current view. Pure — no I/O.
 */
export function computeInsights(input: InsightsInput): Insight[] {
  const { bundle, schema, rows, kappaMap, coverField, meanCurvature } = input;
  if (!schema || rows.length === 0) return [];
  const keyField = schema.base_fields[0]?.name;
  if (!keyField) return [];

  const out: Insight[] = [];

  /* ── 1. Cohort with the most anomalies ───────────────────────────── */
  const cohorts = new Map<string, { size: number; bad: number; warn: number }>();
  for (const r of rows) {
    const cv = String(r[coverField] ?? "—");
    let c = cohorts.get(cv);
    if (!c) {
      c = { size: 0, bad: 0, warn: 0 };
      cohorts.set(cv, c);
    }
    c.size += 1;
    const k = kappaMap.get(String(r[keyField])) ?? 0;
    const cls = kappaClass(k);
    if (cls === "bad") c.bad += 1;
    if (cls === "warn") c.warn += 1;
  }
  const totalBad = Array.from(cohorts.values()).reduce((s, c) => s + c.bad, 0);
  const cohortRanked = Array.from(cohorts.entries())
    .map(([label, c]) => ({ label, ...c }))
    .sort((a, b) => b.bad - a.bad);
  const top = cohortRanked[0];
  if (top && top.bad > 0) {
    const pct = Math.round((top.bad / totalBad) * 100);
    out.push({
      id: "cohort-top-anomalies",
      tag: "bad",
      body: `Cohort ${top.label} holds ${pct}% of all anomalies (${top.bad} of ${totalBad}).`,
      gql: `SECTION ${bundle} WHERE ${coverField}='${esc(top.label)}' ORDER BY κ DESC;`,
      score: 90 + top.bad,
    });
  }

  /* ── 2. Highest-κ row ───────────────────────────────────────────── */
  const ranked = rows
    .map((r) => ({ key: String(r[keyField] ?? ""), k: kappaMap.get(String(r[keyField] ?? "")) ?? 0 }))
    .sort((a, b) => b.k - a.k);
  const top1 = ranked[0];
  if (top1 && top1.k > 0.8) {
    out.push({
      id: "top-kappa",
      tag: top1.k >= 2 ? "bad" : "warn",
      body: `Highest κ in this view: ${top1.key} at ${top1.k.toFixed(2)}. Confidence ${(1 / (1 + top1.k)).toFixed(2)}.`,
      gql: `SECTION ${bundle} AT (${keyField}='${esc(top1.key)}') WITH κ, confidence;`,
      score: 80 + top1.k,
    });
  }

  /* ── 3. Loose-cohort warning ─────────────────────────────────────── */
  // Identify cohorts where every row is "warn" or "bad" — suggests the
  // whole cohort is structurally off, not just an outlier.
  const looseCohort = cohortRanked.find(
    (c) => c.size >= 3 && c.warn + c.bad >= Math.ceil(c.size * 0.6),
  );
  if (looseCohort && looseCohort.label !== top?.label) {
    out.push({
      id: "loose-cohort",
      tag: "geo",
      body: `Cohort ${looseCohort.label} is loose: ${looseCohort.warn + looseCohort.bad} of ${looseCohort.size} rows are drifting or anomalous.`,
      gql: `SPECTRAL ${bundle} WHERE ${coverField}='${esc(looseCohort.label)}';`,
      score: 60 + looseCohort.size,
    });
  }

  /* ── 4. Bundle-wide κ̄ summary ───────────────────────────────────── */
  const meanK =
    Array.from(kappaMap.values()).reduce((s, k) => s + k, 0) /
    Math.max(1, kappaMap.size);
  out.push({
    id: "mean-kappa",
    tag: meanCurvature >= 1 ? "warn" : "info",
    body: `κ̄ across ${rows.length} sections is ${meanK.toFixed(2)} (engine reports ${meanCurvature.toFixed(2)} bundle-wide). Editing any cell recomputes it live.`,
    gql: `CURVATURE ${bundle};`,
    score: 30,
  });

  /* ── 5. Encrypted-field reminder ─────────────────────────────────── */
  const encFields = [...schema.base_fields, ...schema.fiber_fields].filter(
    (f) => f.encryption && f.encryption !== "none",
  );
  if (encFields.length > 0) {
    out.push({
      id: "encrypted-fields",
      tag: "geo",
      body: `${encFields.length} field${encFields.length > 1 ? "s are" : " is"} encrypted (${encFields.map((f) => f.name).join(", ")}). κ + λ₁ are computed over the ciphertext at native speed.`,
      score: 20,
    });
  }

  out.sort((a, b) => b.score - a.score);
  return out;
}

function esc(s: string): string {
  return s.replace(/'/g, "''");
}
