import { useState } from "react";
import { parseCsv, type InferredType } from "../lib/csv";
import { DEMO_DATASETS, type DemoDataset } from "../lib/demo-datasets";
import { registerOverlay } from "../lib/demo-encryption-overlay";
import { type SheetsClient } from "../lib/gigi-client";
import { pathForBundle } from "../lib/route";
import "./DemoBundles.css";

export interface DemoBundlesProps {
  client: SheetsClient;
  /** Names of bundles already on the engine, so we can mark demos as imported. */
  existing: Set<string>;
  /** Called when an import succeeds, so the parent can refresh its list. */
  onImported?: (name: string) => void;
  /**
   * Client-side navigate. When provided, the loader switches to the new
   * bundle in-place instead of reloading the page.
   */
  onPickBundle?: (name: string) => void;
}

type EngineFieldType = "text" | "numeric" | "boolean" | "categorical" | "timestamp";

const ENGINE_TYPE: Record<InferredType, EngineFieldType> = {
  text: "text",
  numeric: "numeric",
  boolean: "boolean",
  categorical: "categorical",
  timestamp: "timestamp",
};

const CHUNK_SIZE = 200;

type LoadingState =
  | { kind: "idle" }
  | { kind: "loading"; id: string; done: number; total: number }
  | { kind: "error"; id: string; message: string };

export function DemoBundles({
  client,
  existing,
  onImported,
  onPickBundle,
}: DemoBundlesProps) {
  const [state, setState] = useState<LoadingState>({ kind: "idle" });

  const load = async (demo: DemoDataset) => {
    setState({ kind: "loading", id: demo.id, done: 0, total: demo.records });
    try {
      const parsed = parseCsv(demo.csv);
      const fields: Record<string, string> = {};
      parsed.headers.forEach((h, i) => {
        fields[h] = ENGINE_TYPE[parsed.types[i] ?? "text"];
      });
      await client.createBundle({
        name: demo.id,
        fields,
        keys: [demo.suggestedKey],
      });
      // Demo-only: tag fields client-side until the engine supports
      // per-field encryption in the schema response (addendum E-S8a).
      if (demo.encryption) {
        registerOverlay(demo.id, demo.encryption);
      }
      for (let i = 0; i < parsed.rows.length; i += CHUNK_SIZE) {
        const chunk = parsed.rows.slice(i, i + CHUNK_SIZE);
        await client.insert(demo.id, chunk);
        setState({
          kind: "loading",
          id: demo.id,
          done: Math.min(i + chunk.length, parsed.rows.length),
          total: parsed.rows.length,
        });
      }
      onImported?.(demo.id);
      // Navigate to the freshly-loaded bundle. Prefer client-side nav so
      // we don't slam the page; fall back to a real assignment so the
      // demo loader still works when used outside the routed app.
      if (onPickBundle) {
        onPickBundle(demo.id);
      } else {
        window.location.href = pathForBundle(demo.id);
      }
    } catch (err) {
      setState({
        kind: "error",
        id: demo.id,
        message: err instanceof Error ? err.message : String(err),
      });
    }
  };

  return (
    <section className="demo-bundles" data-testid="demo-bundles">
      <header className="demo-bundles-head">
        <h3>Start with a demo</h3>
        <p>
          One-click load a recognizable public dataset. Each one ships with a
          suggested <code>Cover</code> field so the geometry view lights up
          right away — including a <strong>PHI-shaped</strong> dataset that
          demonstrates GIGI's gauge encryption.
        </p>
      </header>
      <ul className="demo-list">
        {DEMO_DATASETS.map((d) => {
          const isLoaded = existing.has(d.id);
          const isLoading = state.kind === "loading" && state.id === d.id;
          const errored = state.kind === "error" && state.id === d.id;
          return (
            <li
              key={d.id}
              className="demo-card"
              data-testid={`demo-${d.id}`}
              data-state={
                isLoaded ? "imported" : isLoading ? "loading" : errored ? "error" : "idle"
              }
            >
              <div className="demo-card-badge-slot">
                {d.badge ? (
                  <span
                    className="demo-card-badge"
                    data-testid={`demo-badge-${d.id}`}
                  >
                    🔒 {d.badge}
                  </span>
                ) : (
                  <span className="demo-card-badge demo-card-badge-plain">
                    public dataset
                  </span>
                )}
              </div>
              <div className="demo-card-head">
                <h4>{d.title}</h4>
                <span className="demo-card-stats">
                  {d.records.toLocaleString()} rows · {d.fields} fields
                </span>
              </div>
              <p className="demo-card-blurb">{d.blurb}</p>
              <dl className="demo-card-meta">
                <div>
                  <dt>Cover</dt>
                  <dd><code>{d.suggestedCover}</code></dd>
                </div>
                <div>
                  <dt>Source</dt>
                  <dd>{d.source}</dd>
                </div>
              </dl>
              <div className="demo-card-foot">
                {isLoaded ? (
                  <a
                    href={pathForBundle(d.id)}
                    className="demo-btn demo-btn-imported"
                    data-testid={`demo-open-${d.id}`}
                    onClick={(e) => {
                      if (
                        onPickBundle &&
                        !e.metaKey &&
                        !e.ctrlKey &&
                        !e.shiftKey &&
                        e.button === 0
                      ) {
                        e.preventDefault();
                        onPickBundle(d.id);
                      }
                    }}
                  >
                    Open bundle →
                  </a>
                ) : isLoading ? (
                  <div className="demo-progress" data-testid={`demo-progress-${d.id}`}>
                    <div
                      className="demo-progress-fill"
                      style={{
                        width: `${state.total > 0 ? (100 * state.done) / state.total : 0}%`,
                      }}
                    />
                    <span className="demo-progress-label">
                      Ingesting {state.done} / {state.total}…
                    </span>
                  </div>
                ) : (
                  <button
                    type="button"
                    className="demo-btn demo-btn-primary"
                    onClick={() => load(d)}
                    disabled={state.kind === "loading"}
                    data-testid={`demo-load-${d.id}`}
                  >
                    Load demo
                  </button>
                )}
              </div>
              {errored ? (
                <p className="demo-card-error" role="alert">
                  {state.message}
                </p>
              ) : null}
            </li>
          );
        })}
      </ul>
    </section>
  );
}
