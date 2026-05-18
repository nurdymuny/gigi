import { useCallback, useMemo, useRef, useState } from "react";
import {
  SheetsClient,
  type BundleSchema,
  type GqlRawResult,
} from "../lib/gigi-client";
import { formatGql } from "../lib/gql-format";
import { buildGqlSamples } from "../lib/gql-samples";
import "./GqlView.css";

export interface GqlViewProps {
  client: SheetsClient;
  /** Initial query (or last-run query) — owned by the App so it survives tab swaps. */
  query: string;
  onQueryChange: (next: string) => void;
  /** Active bundle schema — drives the sample-query chips. */
  schema?: BundleSchema | null;
  /** Cohort field for the bundle — drives `COVER` / `AROUND` substitutions. */
  coverField?: string;
  /**
   * Real row keys from the bundle. First is used for `SECTION ... AT (...)`
   * and `TRANSPORT FROM (...)`; second for the `TO (...)` clause. Pass
   * the first two visible-row keys.
   */
  sampleRowKey?: string | null;
  secondRowKey?: string | null;
}

interface RunState {
  loading: boolean;
  result: GqlRawResult | null;
  error: string | null;
}

const INITIAL: RunState = { loading: false, result: null, error: null };

export function GqlView({
  client,
  query,
  onQueryChange,
  schema = null,
  coverField = "",
  sampleRowKey = null,
  secondRowKey = null,
}: GqlViewProps) {
  const [run, setRun] = useState<RunState>(INITIAL);
  const taRef = useRef<HTMLTextAreaElement>(null);

  const samples = useMemo(
    () => buildGqlSamples({ schema, coverField, sampleRowKey, secondRowKey }),
    [schema, coverField, sampleRowKey, secondRowKey],
  );

  const execute = useCallback(async () => {
    const q = query.trim();
    if (!q) return;
    setRun({ loading: true, result: null, error: null });
    try {
      const result = await client.gqlRaw(q);
      setRun({ loading: false, result, error: null });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setRun({ loading: false, result: null, error: msg });
    }
  }, [client, query]);

  const format = useCallback(() => {
    onQueryChange(formatGql(query));
  }, [query, onQueryChange]);

  const onKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
        e.preventDefault();
        execute();
      }
    },
    [execute],
  );

  return (
    <div className="gql-view" data-testid="gql-view">
      <div className="gql-toolbar">
        <button
          type="button"
          className="gql-btn gql-btn-primary"
          onClick={execute}
          disabled={run.loading || !query.trim()}
          data-testid="gql-run"
        >
          {run.loading ? "Running…" : "Run"}
        </button>
        <button
          type="button"
          className="gql-btn"
          onClick={format}
          disabled={!query.trim()}
          data-testid="gql-format"
        >
          Format
        </button>
        <span className="gql-hint">⌘↵ to run</span>
      </div>

      {samples.length > 0 ? (
        <div className="gql-samples" data-testid="gql-samples">
          <span className="gql-samples-label">Samples</span>
          {samples.map((s) => (
            <button
              key={s.id}
              type="button"
              className="gql-sample-chip"
              data-testid={`gql-sample-${s.id}`}
              title={`${s.description}\n\n${s.query}`}
              onClick={() => {
                // Drop the sample into the editor and focus it so the
                // user can either tweak or hit ⌘↵ immediately.
                onQueryChange(s.query);
                requestAnimationFrame(() => {
                  taRef.current?.focus();
                  const len = s.query.length;
                  taRef.current?.setSelectionRange(len, len);
                });
              }}
            >
              {s.label}
            </button>
          ))}
        </div>
      ) : null}

      <textarea
        ref={taRef}
        className="gql-editor"
        data-testid="gql-editor"
        value={query}
        spellCheck={false}
        autoCapitalize="off"
        autoCorrect="off"
        onChange={(e) => onQueryChange(e.target.value)}
        onKeyDown={onKeyDown}
        placeholder={[
          "CURVATURE sensors;",
          "CURVATURE sensors BY site_id;",
          "BETTI sensors;",
          "SPECTRAL sensors;",
          "SECTION sensors AT sensor_id='S-001';",
          "INTEGRATE sensors OVER site_id MEASURE AVG(temp);",
          "HOLONOMY sensors ON FIBER (temp, humidity) AROUND site_id;",
        ].join("\n")}
      />

      <ResultPanel state={run} />
    </div>
  );
}

function ResultPanel({ state }: { state: RunState }) {
  if (state.loading) {
    return (
      <div className="gql-result gql-result-loading" data-testid="gql-result-loading">
        Running query…
      </div>
    );
  }
  if (state.error) {
    return (
      <div className="gql-result gql-result-error" role="alert" data-testid="gql-result-error">
        <strong>Client error.</strong>
        <p>{state.error}</p>
      </div>
    );
  }
  if (!state.result) {
    return (
      <div className="gql-result gql-result-empty" data-testid="gql-result-empty">
        Press <kbd>Run</kbd> or <kbd>⌘↵</kbd> to execute the query.
      </div>
    );
  }
  return <RenderedResult result={state.result} />;
}

function RenderedResult({ result }: { result: GqlRawResult }) {
  const body = result.body;
  const isError = result.status >= 400;
  return (
    <div
      className={`gql-result ${isError ? "gql-result-engine-error" : ""}`}
      data-testid="gql-result"
      data-status={result.status}
    >
      <MetaRow result={result} />
      <ResultBody body={body} />
    </div>
  );
}

function MetaRow({ result }: { result: GqlRawResult }) {
  const body = (result.body ?? {}) as Record<string, unknown>;
  const rows = Array.isArray(body.rows) ? body.rows : null;
  const count = typeof body.count === "number" ? body.count : rows?.length ?? 0;
  return (
    <div className="gql-meta" data-testid="gql-meta">
      <Meta label="status" value={String(result.status)} testid="meta-status" />
      <Meta label="rows" value={count.toLocaleString()} testid="meta-rows" />
      <Meta
        label="elapsed"
        value={`${result.elapsedMs.toFixed(1)} ms`}
        testid="meta-elapsed"
      />
      {typeof body.curvature === "number" ? (
        <Meta label="κ" value={(body.curvature as number).toFixed(3)} testid="meta-kappa" />
      ) : null}
      {typeof body.confidence === "number" ? (
        <Meta
          label="conf"
          value={(body.confidence as number).toFixed(3)}
          testid="meta-conf"
        />
      ) : null}
    </div>
  );
}

function Meta({
  label,
  value,
  testid,
}: {
  label: string;
  value: string;
  testid: string;
}) {
  return (
    <div className="gql-meta-item" data-testid={testid}>
      <span className="gql-meta-label">{label}</span>
      <span className="gql-meta-value">{value}</span>
    </div>
  );
}

function ResultBody({ body }: { body: unknown }) {
  if (body == null) {
    return (
      <div className="gql-result-empty" data-testid="gql-result-noop">
        (no body)
      </div>
    );
  }
  if (typeof body !== "object") {
    return (
      <pre className="gql-result-scalar" data-testid="gql-result-scalar">
        {String(body)}
      </pre>
    );
  }

  const obj = body as Record<string, unknown>;
  // Engine error shape: { error: "..." }
  if (typeof obj.error === "string") {
    return (
      <div className="gql-result-engine-msg" role="alert" data-testid="gql-result-engine-msg">
        <strong>Engine error:</strong> {obj.error}
      </div>
    );
  }
  if (Array.isArray(obj.rows)) {
    return <RowsTable rows={obj.rows as Array<Record<string, unknown>>} />;
  }
  if ("value" in obj) {
    return (
      <pre className="gql-result-scalar" data-testid="gql-result-scalar">
        {JSON.stringify(obj.value, null, 2)}
      </pre>
    );
  }
  if ("affected" in obj) {
    return (
      <div className="gql-result-affected" data-testid="gql-result-affected">
        Affected: <b>{String(obj.affected)}</b>
      </div>
    );
  }
  // Fallback: pretty JSON.
  return (
    <pre className="gql-result-json" data-testid="gql-result-json">
      {JSON.stringify(body, null, 2)}
    </pre>
  );
}

function RowsTable({ rows }: { rows: Array<Record<string, unknown>> }) {
  if (rows.length === 0) {
    return (
      <div className="gql-result-empty" data-testid="gql-result-zero-rows">
        Query returned 0 rows.
      </div>
    );
  }
  // Union of keys across all rows, in first-seen order.
  const keys: string[] = [];
  const seen = new Set<string>();
  for (const r of rows) {
    for (const k of Object.keys(r)) {
      if (!seen.has(k)) {
        seen.add(k);
        keys.push(k);
      }
    }
  }

  return (
    <div className="gql-table-wrap">
      <table className="gql-table" data-testid="gql-table">
        <thead>
          <tr>
            {keys.map((k) => (
              <th key={k} data-testid={`gql-th-${k}`}>{k}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.slice(0, 200).map((row, i) => (
            <tr key={i} data-testid="gql-tr">
              {keys.map((k) => (
                <td key={k}>{formatCell(row[k])}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      {rows.length > 200 ? (
        <p className="gql-table-truncated">
          Showing first 200 of {rows.length.toLocaleString()} rows.
        </p>
      ) : null}
    </div>
  );
}

function formatCell(v: unknown): string {
  if (v == null) return "—";
  if (typeof v === "number") return String(v);
  if (typeof v === "string") return v;
  return JSON.stringify(v);
}
