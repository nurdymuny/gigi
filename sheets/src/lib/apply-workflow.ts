/**
 * Apply-workflow handler — turn a `WorkflowTemplate` into a real
 * workspace on the engine.
 *
 * Flow:
 *   1. For each bundle spec in the template:
 *      - createBundle on the engine
 *      - parse the seed CSV
 *      - chunk-insert the rows
 *   2. Return the `defaultBundle` so the caller can navigate there.
 *
 * Reuses the same chunked-insert pattern the demo loader uses
 * (`DemoBundles.tsx` / `SidebarDemos.tsx`) so behavior is identical.
 */

import { parseCsv } from "./csv";
import { type SheetsClient } from "./gigi-client";
import {
  workflowCsv,
  type WorkflowTemplate,
} from "./workflow-templates";

const CHUNK_SIZE = 200;

export interface ApplyProgress {
  /** Bundle currently being written. */
  bundleName: string;
  /** Rows already inserted into this bundle. */
  done: number;
  /** Total rows for this bundle. */
  total: number;
  /** Which bundle (1-indexed) of how many. */
  bundleIndex: number;
  bundleCount: number;
}

export interface ApplyResult {
  /** Name of the bundle the user should land on. */
  defaultBundle: string;
  /** Total rows inserted across all bundles. */
  rowCount: number;
}

/**
 * Create every bundle in the template, seed each from its CSV, and
 * return the default bundle for navigation. `onProgress` fires per
 * chunk so the picker can show a progress bar.
 */
export async function applyWorkflow(
  template: WorkflowTemplate,
  client: SheetsClient,
  onProgress?: (p: ApplyProgress) => void,
): Promise<ApplyResult> {
  let rowCount = 0;
  for (let b = 0; b < template.bundles.length; b++) {
    const bundle = template.bundles[b];
    const csv = workflowCsv(bundle);
    const parsed = parseCsv(csv);

    await client.createBundle({
      name: bundle.name,
      fields: bundle.fields,
      keys: bundle.keys,
    });

    const total = parsed.rows.length;
    for (let i = 0; i < parsed.rows.length; i += CHUNK_SIZE) {
      const chunk = parsed.rows.slice(i, i + CHUNK_SIZE);
      await client.insert(bundle.name, chunk);
      onProgress?.({
        bundleName: bundle.name,
        done: Math.min(i + chunk.length, total),
        total,
        bundleIndex: b + 1,
        bundleCount: template.bundles.length,
      });
    }
    rowCount += parsed.rows.length;
  }
  return {
    defaultBundle: template.defaultBundle,
    rowCount,
  };
}
