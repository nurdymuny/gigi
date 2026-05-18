import { useEffect, useMemo, useRef, useState } from "react";
import type { SheetsClient } from "../lib/gigi-client";
import { parseCsv, pickKeyField, type InferredType } from "../lib/csv";
import { pathForBundle } from "../lib/route";
import "./ImportCsvModal.css";

export interface ImportCsvModalProps {
  open: boolean;
  client: SheetsClient;
  onClose: () => void;
  /** Called after a successful import. Receives the new bundle name. */
  onImported?: (name: string) => void;
}

/**
 * Multi-step CSV / TSV import:
 *   1. Paste text or pick a .csv / .tsv file
 *   2. Confirm: edit bundle name, pick key column, override types
 *   3. Submit: createBundle → bulk insert in chunks → navigate to it
 */

type Step = "input" | "preview" | "ingesting" | "done";
type EngineFieldType = "text" | "numeric" | "boolean" | "categorical" | "timestamp";

const ENGINE_TYPE: Record<InferredType, EngineFieldType> = {
  text: "text",
  numeric: "numeric",
  boolean: "boolean",
  categorical: "categorical",
  timestamp: "timestamp",
};

const CHUNK_SIZE = 200;

export function ImportCsvModal({
  open,
  client,
  onClose,
  onImported,
}: ImportCsvModalProps) {
  const [step, setStep] = useState<Step>("input");
  const [pasted, setPasted] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [bundleName, setBundleName] = useState("imported");
  const [keyField, setKeyField] = useState<string>("");
  const [types, setTypes] = useState<EngineFieldType[]>([]);
  const [progress, setProgress] = useState({ done: 0, total: 0 });
  const fileRef = useRef<HTMLInputElement>(null);

  // Parse on demand — when text changes, re-parse for the preview.
  const parsed = useMemo(() => {
    if (!pasted.trim()) return null;
    try {
      return parseCsv(pasted);
    } catch (err) {
      return { error: err instanceof Error ? err.message : String(err) };
    }
  }, [pasted]);

  useEffect(() => {
    if (open) {
      setStep("input");
      setPasted("");
      setError(null);
      setBundleName("imported");
      setKeyField("");
      setTypes([]);
      setProgress({ done: 0, total: 0 });
    }
  }, [open]);

  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  const proceedToPreview = () => {
    if (!parsed || "error" in parsed) {
      setError(parsed && "error" in parsed ? parsed.error : "Nothing to parse");
      return;
    }
    if (parsed.rows.length === 0) {
      setError("CSV has no data rows — only a header row was found.");
      return;
    }
    setError(null);
    setTypes(parsed.types.map((t) => ENGINE_TYPE[t]));
    setKeyField(pickKeyField(parsed.headers, parsed.rows) ?? parsed.headers[0]);
    setStep("preview");
  };

  // 50 MB is enough for ~250k rows of typical sheets data and small
  // enough that we won't blow up a tab on a hostile drag-and-drop.
  // The parsed `rows[]` array takes ~3-5x the source size in memory,
  // so this caps tab footprint around 250 MB worst-case.
  const MAX_CSV_BYTES = 50 * 1024 * 1024;

  const onFile = async (file: File) => {
    setError(null);
    if (file.size > MAX_CSV_BYTES) {
      const mb = (file.size / (1024 * 1024)).toFixed(1);
      setError(
        `File is ${mb} MB — the import path caps at ${MAX_CSV_BYTES / (1024 * 1024)} MB. ` +
          `For larger bundles, use the engine's bulk-import GQL endpoint directly.`,
      );
      return;
    }
    try {
      const text = await file.text();
      setPasted(text);
      // If we have a sensible default name from the filename, use it.
      const base = file.name.replace(/\.[^.]+$/, "").replace(/[^A-Za-z0-9_]/g, "_");
      if (base) setBundleName(base.replace(/^(\d)/, "_$1"));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const submit = async () => {
    if (!parsed || "error" in parsed) return;
    if (!bundleName) {
      setError("Bundle name is required.");
      return;
    }
    if (!keyField) {
      setError("Pick a primary-key column.");
      return;
    }
    setError(null);
    setStep("ingesting");
    setProgress({ done: 0, total: parsed.rows.length });

    const fields: Record<string, string> = {};
    parsed.headers.forEach((h, i) => {
      fields[h] = types[i] ?? "text";
    });

    try {
      await client.createBundle({
        name: bundleName,
        fields,
        keys: [keyField],
      });
      // Bulk-ingest in chunks of 200 so we don't pay the round-trip
      // penalty or send a giant body.
      for (let i = 0; i < parsed.rows.length; i += CHUNK_SIZE) {
        const chunk = parsed.rows.slice(i, i + CHUNK_SIZE);
        await client.insert(bundleName, chunk);
        setProgress({ done: i + chunk.length, total: parsed.rows.length });
      }
      setStep("done");
      onImported?.(bundleName);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStep("preview");
    }
  };

  return (
    <div
      className="import-bg"
      data-testid="import-bg"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="import-modal" data-testid="import-modal" role="dialog">
        <header className="import-head">
          <h2>Import CSV / TSV</h2>
          <button
            type="button"
            className="import-close"
            onClick={onClose}
            aria-label="Close"
          >
            ✕
          </button>
        </header>

        {error ? (
          <div className="import-error" role="alert" data-testid="import-error">
            {error}
          </div>
        ) : null}

        {step === "input" ? (
          <div className="import-body">
            <p className="import-hint">
              Paste a CSV or TSV (with a header row) or upload a file.
              Commas, tabs, and quoted fields with embedded commas are all OK.
            </p>
            <input
              ref={fileRef}
              type="file"
              accept=".csv,.tsv,text/csv,text/tab-separated-values"
              onChange={(e) => {
                const f = e.target.files?.[0];
                if (f) onFile(f);
              }}
              data-testid="import-file"
            />
            <textarea
              className="import-paste"
              placeholder={"id,name,age,city\n1,Alice,30,Paris\n2,Bob,45,Lagos\n…"}
              value={pasted}
              onChange={(e) => setPasted(e.target.value)}
              spellCheck={false}
              data-testid="import-paste"
            />
            <footer className="import-foot">
              <button
                type="button"
                className="import-btn import-btn-primary"
                onClick={proceedToPreview}
                disabled={!pasted.trim()}
                data-testid="import-preview"
              >
                Preview →
              </button>
              <button type="button" className="import-btn" onClick={onClose}>
                Cancel
              </button>
            </footer>
          </div>
        ) : null}

        {step === "preview" && parsed && !("error" in parsed) ? (
          <div className="import-body">
            <div className="import-row">
              <label className="import-label">Bundle name</label>
              <input
                type="text"
                value={bundleName}
                onChange={(e) => setBundleName(e.target.value)}
                className="import-input"
                data-testid="import-bundle-name"
              />
            </div>
            <div className="import-row">
              <label className="import-label">Key column</label>
              <select
                value={keyField}
                onChange={(e) => setKeyField(e.target.value)}
                className="import-input"
                data-testid="import-key-field"
              >
                {parsed.headers.map((h) => (
                  <option key={h} value={h}>
                    {h}
                  </option>
                ))}
              </select>
            </div>
            <p className="import-hint">
              {parsed.rows.length.toLocaleString()} rows · {parsed.headers.length} fields ·
              delimiter <code>{parsed.delimiter === "\t" ? "\\t" : ","}</code>
            </p>
            <div className="import-fields">
              <div className="import-fields-head">
                <span>Field</span>
                <span>Inferred type</span>
              </div>
              {parsed.headers.map((h, i) => (
                <label
                  key={h}
                  className="import-field-row"
                  data-testid={`import-field-${h}`}
                >
                  <span className="import-field-name">
                    {h}
                    {h === keyField ? (
                      <span className="import-field-key">key</span>
                    ) : null}
                  </span>
                  <select
                    value={types[i] ?? "text"}
                    onChange={(e) => {
                      const next = types.slice();
                      next[i] = e.target.value as EngineFieldType;
                      setTypes(next);
                    }}
                    data-testid={`import-type-${h}`}
                  >
                    <option value="text">text</option>
                    <option value="numeric">numeric</option>
                    <option value="categorical">categorical</option>
                    <option value="boolean">boolean</option>
                    <option value="timestamp">timestamp</option>
                  </select>
                </label>
              ))}
            </div>
            <details className="import-preview-rows">
              <summary>
                Sample (first 5 of {parsed.rows.length.toLocaleString()} rows)
              </summary>
              <pre>{JSON.stringify(parsed.rows.slice(0, 5), null, 2)}</pre>
            </details>
            <footer className="import-foot">
              <button
                type="button"
                className="import-btn import-btn-primary"
                onClick={submit}
                data-testid="import-submit"
              >
                Create bundle + ingest
              </button>
              <button
                type="button"
                className="import-btn"
                onClick={() => setStep("input")}
              >
                Back
              </button>
            </footer>
          </div>
        ) : null}

        {step === "ingesting" ? (
          <div className="import-body import-progress" data-testid="import-progress">
            <p>
              Ingesting <b>{progress.done.toLocaleString()}</b> of{" "}
              {progress.total.toLocaleString()} rows…
            </p>
            <div className="import-progress-bar">
              <div
                className="import-progress-fill"
                style={{
                  width: `${progress.total > 0 ? (100 * progress.done) / progress.total : 0}%`,
                }}
              />
            </div>
          </div>
        ) : null}

        {step === "done" ? (
          <div className="import-body" data-testid="import-done">
            <p>
              ✓ Imported <b>{progress.total.toLocaleString()}</b> rows into{" "}
              <code>{bundleName}</code>.
            </p>
            <footer className="import-foot">
              <a
                href={pathForBundle(bundleName)}
                className="import-btn import-btn-primary"
                data-testid="import-open"
              >
                Open bundle →
              </a>
              <button type="button" className="import-btn" onClick={onClose}>
                Close
              </button>
            </footer>
          </div>
        ) : null}
      </div>
    </div>
  );
}
