import { useState } from "react";
import { parseCsv, type InferredType } from "../lib/csv";
import { DEMO_DATASETS, type DemoDataset } from "../lib/demo-datasets";
import { registerOverlay } from "../lib/demo-encryption-overlay";
import { type SheetsClient } from "../lib/gigi-client";
import { pathForBundle } from "../lib/route";
import "./SidebarDemos.css";

type EngineFieldType =
  | "text"
  | "numeric"
  | "boolean"
  | "categorical"
  | "timestamp";

const ENGINE_TYPE: Record<InferredType, EngineFieldType> = {
  text: "text",
  numeric: "numeric",
  boolean: "boolean",
  categorical: "categorical",
  timestamp: "timestamp",
};

const CHUNK_SIZE = 200;

type LoadState =
  | { kind: "idle" }
  | { kind: "loading"; id: string }
  | { kind: "error"; id: string; message: string };

export interface SidebarDemosProps {
  client: SheetsClient;
  /** Client-side navigate after the demo is loaded. */
  onPickBundle?: (name: string) => void;
}

/**
 * Compact sidebar list of demo bundles. Same loader semantics as
 * `<DemoBundles>` (parse CSV, create bundle, chunk-insert, navigate)
 * but rendered as a tight one-row-per-demo list that fits the 220px
 * sidebar without scrolling. Used in guest mode below the sign-in CTA
 * so the empty rail has something useful in it.
 */
export function SidebarDemos({ client, onPickBundle }: SidebarDemosProps) {
  const [state, setState] = useState<LoadState>({ kind: "idle" });

  const load = async (demo: DemoDataset) => {
    setState({ kind: "loading", id: demo.id });
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
      if (demo.encryption) registerOverlay(demo.id, demo.encryption);
      for (let i = 0; i < parsed.rows.length; i += CHUNK_SIZE) {
        const chunk = parsed.rows.slice(i, i + CHUNK_SIZE);
        await client.insert(demo.id, chunk);
      }
      if (onPickBundle) {
        onPickBundle(demo.id);
      } else {
        window.location.href = pathForBundle(demo.id);
      }
      setState({ kind: "idle" });
    } catch (err) {
      setState({
        kind: "error",
        id: demo.id,
        message: err instanceof Error ? err.message : String(err),
      });
    }
  };

  return (
    <div className="sidebar-demos" data-testid="sidebar-demos">
      <div className="sidebar-demos-head">
        <span className="sidebar-demos-title">Try a demo</span>
        <span className="sidebar-demos-sub">
          One-click sample data. No sign-up.
        </span>
      </div>
      <ul className="sidebar-demos-list">
        {DEMO_DATASETS.map((d) => {
          const isLoading = state.kind === "loading" && state.id === d.id;
          const errored = state.kind === "error" && state.id === d.id;
          return (
            <li key={d.id}>
              <button
                type="button"
                className="sidebar-demo-row"
                onClick={() => load(d)}
                disabled={state.kind === "loading"}
                data-testid={`sidebar-demo-${d.id}`}
                title={d.blurb}
              >
                <span className="sidebar-demo-title-row">
                  <span className="sidebar-demo-title">{d.title}</span>
                  {d.badge ? (
                    <span className="sidebar-demo-badge">{d.badge}</span>
                  ) : null}
                </span>
                <span className="sidebar-demo-meta">
                  {isLoading
                    ? "Loading…"
                    : `${d.records.toLocaleString()} rows · ${d.fields} fields`}
                </span>
                {errored ? (
                  <span className="sidebar-demo-error">{state.message}</span>
                ) : null}
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
