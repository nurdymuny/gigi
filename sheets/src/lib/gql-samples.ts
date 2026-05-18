/**
 * GQL sample-query builder for the chips above the GQL editor.
 *
 * Each chip carries a ready-to-run query targeted at the *current*
 * bundle — bundle name, key field, cover field, and a real sample row
 * key are substituted in so the user can click → Run without editing.
 *
 * Samples gracefully degrade: missing cover field → cohort + holonomy
 * chips drop; no row keys yet → SECTION + TRANSPORT drop; no numerics
 * → INTEGRATE + HOLONOMY drop. The bundle-wide ones (CURVATURE / BETTI
 * / SPECTRAL) always ship.
 */

import type { BundleSchema, FieldDescriptor } from "./gigi-client";

export type GqlSampleId =
  | "curvature"
  | "curvature-cover"
  | "betti"
  | "spectral"
  | "section"
  | "integrate"
  | "holonomy"
  | "transport";

export interface GqlSample {
  id: GqlSampleId;
  /** Short chip text (≤ ~24 chars). */
  label: string;
  /** Tooltip / popover detail. One sentence. */
  description: string;
  /** Full GQL query, ready to drop into the editor. Ends with `;`. */
  query: string;
}

export interface BuildGqlSamplesInput {
  schema: BundleSchema | null;
  coverField: string;
  /**
   * A real row key from the bundle, used for `SECTION ... AT (...)`
   * and `TRANSPORT FROM (...)`. Null if no rows are loaded yet.
   */
  sampleRowKey: string | null;
  /**
   * A second distinct row key, used for the `TO (...)` clause of
   * TRANSPORT. Null if the bundle only has one row.
   */
  secondRowKey: string | null;
}

/**
 * SQL-style single-quote escape (`'` → `''`). Used for row-key literals
 * inside `AT (key='...')` clauses — matches what `quoteIdent` does
 * upstream in the SheetsClient.
 */
function sqlQuote(s: string): string {
  return `'${s.replace(/'/g, "''")}'`;
}

function numericFiberFields(schema: BundleSchema): FieldDescriptor[] {
  return schema.fiber_fields.filter((f) => f.type === "numeric");
}

export function buildGqlSamples(input: BuildGqlSamplesInput): GqlSample[] {
  const { schema, coverField, sampleRowKey, secondRowKey } = input;
  if (!schema) return [];
  const bundle = schema.name;
  const keyField = schema.base_fields[0]?.name;
  const numerics = numericFiberFields(schema);
  const f1 = numerics[0]?.name;
  const f2 = numerics[1]?.name;

  const out: GqlSample[] = [];

  // ── Bundle-wide geometry (always available) ─────────────────────
  out.push({
    id: "curvature",
    label: "Curvature κ",
    description: "Bundle-wide cohort-relative curvature.",
    query: `CURVATURE ${bundle};`,
  });

  if (coverField) {
    out.push({
      id: "curvature-cover",
      label: "Per-cohort κ",
      description: `κ broken out by ${coverField} cohort.`,
      query: `CURVATURE ${bundle} BY ${coverField};`,
    });
  }

  out.push({
    id: "betti",
    label: "Betti b₀ b₁ b₂",
    description: "Topological invariants — connected components, loops, voids.",
    query: `BETTI ${bundle};`,
  });

  out.push({
    id: "spectral",
    label: "Spectral λ₁",
    description: "Smallest non-zero Laplacian eigenvalue — the algebraic connectivity.",
    query: `SPECTRAL ${bundle};`,
  });

  // ── Single-row + comparisons (require keys) ─────────────────────
  if (sampleRowKey && keyField) {
    out.push({
      id: "section",
      label: "Section a row",
      description: `Point query — fetch the row at ${keyField}='${sampleRowKey}'.`,
      // SECTION's `AT k=v` form takes bare key=val pairs (no parens) per
      // the engine's parse_kv_pairs grammar. Parens here would error
      // with "Expected '=' or ':' after '('".
      query: `SECTION ${bundle} AT ${keyField}=${sqlQuote(sampleRowKey)};`,
    });
  }

  // ── Field operations (require ≥ 1 numeric) ──────────────────────
  if (f1) {
    // Real grammar: INTEGRATE <bundle> [OVER <cover-field>] MEASURE <agg>(<field>).
    // Bundle comes first; the numeric field lives inside the MEASURE
    // aggregator. With a cover field we group by it; without one we
    // get a single global aggregate.
    out.push({
      id: "integrate",
      label: "Integrate field",
      description: coverField
        ? `Average ${f1} per ${coverField} cohort.`
        : `Bundle-wide average of ${f1}.`,
      query: coverField
        ? `INTEGRATE ${bundle} OVER ${coverField} MEASURE AVG(${f1});`
        : `INTEGRATE ${bundle} MEASURE AVG(${f1});`,
    });
  }

  if (f1 && f2 && coverField) {
    out.push({
      id: "holonomy",
      label: "Holonomy",
      description: `Parallel-transport holonomy around the ${coverField} cohort, in the (${f1}, ${f2}) plane.`,
      query: `HOLONOMY ${bundle} ON FIBER (${f1}, ${f2}) AROUND ${coverField};`,
    });
  }

  if (f1 && f2 && keyField && sampleRowKey && secondRowKey && sampleRowKey !== secondRowKey) {
    out.push({
      id: "transport",
      label: "Transport row→row",
      description: `Geodesic transport from ${sampleRowKey} to ${secondRowKey} in (${f1}, ${f2}).`,
      query: `TRANSPORT ${bundle} FROM (${keyField}=${sqlQuote(sampleRowKey)}) TO (${keyField}=${sqlQuote(secondRowKey)}) ON FIBER (${f1}, ${f2});`,
    });
  }

  return out;
}
