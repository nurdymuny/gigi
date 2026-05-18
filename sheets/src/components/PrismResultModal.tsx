import { useEffect, useState } from "react";
import type { WorkflowResult } from "../lib/prism-workflows";
import type { SheetsClient } from "../lib/gigi-client";
import "./PrismResultModal.css";

export interface PrismResultModalProps {
  result: WorkflowResult | null;
  onClose: () => void;
  /** Engine client — used to persist the result as a real bundle the user can keep. */
  client?: SheetsClient;
  /** Name of the bundle the workflow ran against (used to slug the artifact bundle). */
  sourceBundle?: string;
  /** When the artifact is saved, open it as a new tab. */
  onArtifactSaved?: (newBundleName: string) => void;
}

const TITLES: Record<string, string> = {
  dedup: "Dedup · result",
  forecast: "Forecast · result",
  monitor: "Monitor · result",
  books: "Books · result",
};

/**
 * Slug the workflow id + source bundle + a short timestamp into a valid
 * engine bundle name. Output looks like `dedup_iris_0515_1043`.
 */
function artifactBundleName(workflow: string, source: string | undefined): string {
  const now = new Date();
  const mm = String(now.getMonth() + 1).padStart(2, "0");
  const dd = String(now.getDate()).padStart(2, "0");
  const hh = String(now.getHours()).padStart(2, "0");
  const mi = String(now.getMinutes()).padStart(2, "0");
  const stem = source ? source.replace(/[^A-Za-z0-9_]/g, "_") : "result";
  return `${workflow}_${stem}_${mm}${dd}_${hh}${mi}`.slice(0, 60);
}

/**
 * Shows what a Prism workflow produced. Navy-on-cream Prism brand styling
 * to make the "you're using a different product" story unambiguous.
 *
 * Each result can be saved as a real bundle on the engine — the user
 * gets an artifact they can keep, browse, share, and feed into other
 * GIGI workflows. That's the "free run leaves something behind" idea.
 */
export function PrismResultModal({
  result,
  onClose,
  client,
  sourceBundle,
  onArtifactSaved,
}: PrismResultModalProps) {
  const [saveState, setSaveState] = useState<
    | { kind: "idle" }
    | { kind: "saving" }
    | { kind: "saved"; name: string }
    | { kind: "error"; message: string }
  >({ kind: "idle" });

  useEffect(() => {
    if (!result) return;
    setSaveState({ kind: "idle" });
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [result, onClose]);

  if (!result) return null;

  const saveArtifact = async () => {
    if (!client || result.table.rows.length === 0) return;
    setSaveState({ kind: "saving" });
    const name = artifactBundleName(result.workflow, sourceBundle);
    // Infer column types from the first row.
    const first = result.table.rows[0];
    const fields: Record<string, string> = {};
    for (const col of result.table.columns) {
      const v = first?.[col];
      fields[col] =
        typeof v === "number" ? "numeric" : col === "row" || col === "group" ? "text" : "text";
    }
    // Pick a key — prefer "row" or "group" or "period", else the first column.
    const preferredKey = ["row", "group", "period"].find((k) =>
      result.table.columns.includes(k),
    );
    const keyCol = preferredKey ?? result.table.columns[0];
    fields[keyCol] = "text"; // force key column to text for safety

    try {
      await client.createBundle({ name, fields, keys: [keyCol] });
      const records = result.table.rows.map((r) => {
        const out: Record<string, unknown> = {};
        for (const c of result.table.columns) {
          out[c] = r[c] ?? null;
        }
        return out;
      });
      await client.insert(name, records);
      setSaveState({ kind: "saved", name });
    } catch (err) {
      setSaveState({
        kind: "error",
        message: err instanceof Error ? err.message : String(err),
      });
    }
  };

  const canSave = client !== undefined && result.table.rows.length > 0;

  return (
    <div
      className="prism-result-bg"
      data-testid="prism-result-bg"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className="prism-result-modal"
        data-testid="prism-result-modal"
        role="dialog"
      >
        <header className="prism-result-head">
          <div>
            <div className="prism-brand-sm">
              <span aria-hidden="true">◇</span> PRISM
            </div>
            <h2 data-testid="prism-result-title">
              {TITLES[result.workflow] ?? result.workflow}
            </h2>
            <p className="prism-result-headline" data-testid="prism-result-headline">
              {result.headline}
            </p>
          </div>
          <button
            type="button"
            className="prism-result-close"
            onClick={onClose}
            aria-label="Close"
            data-testid="prism-result-close"
          >
            ×
          </button>
        </header>

        <div className="prism-result-body">
          {result.stats.length > 0 ? (
            <section className="prism-result-stats">
              {result.stats.map((s, i) => (
                <div
                  key={i}
                  className={`prism-stat prism-stat-${s.kind ?? "neutral"}`}
                  data-testid={`prism-stat-${s.label}`}
                >
                  <span className="prism-stat-label">{s.label}</span>
                  <span className="prism-stat-value">{s.value}</span>
                </div>
              ))}
            </section>
          ) : null}

          {result.findings.length > 0 ? (
            <section className="prism-result-findings">
              <h4>Findings</h4>
              <ul>
                {result.findings.map((f, i) => (
                  <li key={i}>{f}</li>
                ))}
              </ul>
            </section>
          ) : null}

          {result.table.rows.length > 0 ? (
            <section className="prism-result-table-wrap">
              <h4>Details</h4>
              <div className="prism-result-table-scroll">
                <table className="prism-result-table" data-testid="prism-result-table">
                  <thead>
                    <tr>
                      {result.table.columns.map((c) => (
                        <th key={c}>{c}</th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {result.table.rows.map((r, i) => (
                      <tr key={i}>
                        {result.table.columns.map((c) => (
                          <td key={c}>{String(r[c] ?? "")}</td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </section>
          ) : null}

          <section className="prism-result-method">
            <h4>Method</h4>
            <p>{result.method}</p>
          </section>
        </div>

        <footer className="prism-result-foot">
          <span className="prism-result-foot-note">
            {saveState.kind === "saved" ? (
              <>
                ✓ Saved as <code>{saveState.name}</code> — open it in a new tab to
                inspect, export, or feed it into another workflow.
              </>
            ) : saveState.kind === "error" ? (
              <span className="prism-result-foot-err">
                Save failed: {saveState.message}
              </span>
            ) : (
              <>
                Demo run on the synthetic Prism adapter. Full Prism engine runs
                against a Riemannian manifold with audit certificates.
              </>
            )}
          </span>
          <div className="prism-result-foot-actions">
            {canSave && saveState.kind !== "saved" ? (
              <button
                type="button"
                className="prism-result-foot-btn prism-result-foot-btn-secondary"
                onClick={() => void saveArtifact()}
                disabled={saveState.kind === "saving"}
                data-testid="prism-result-save"
              >
                {saveState.kind === "saving"
                  ? "Saving…"
                  : "Save as bundle"}
              </button>
            ) : null}
            {saveState.kind === "saved" && onArtifactSaved ? (
              <button
                type="button"
                className="prism-result-foot-btn"
                onClick={() => {
                  onArtifactSaved(saveState.name);
                  onClose();
                }}
                data-testid="prism-result-open"
              >
                Open bundle →
              </button>
            ) : null}
            <button
              type="button"
              className="prism-result-foot-btn prism-result-foot-btn-plain"
              onClick={onClose}
            >
              Close
            </button>
          </div>
        </footer>
      </div>
    </div>
  );
}
