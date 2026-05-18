import { useEffect, useState } from "react";
import type {
  BundleSchema,
  FieldDescriptor,
  RowMap,
  SheetsClient,
} from "../lib/gigi-client";
import "./InsertRowModal.css";

export interface InsertRowModalProps {
  open: boolean;
  client: SheetsClient;
  bundle: string;
  schema: BundleSchema | null;
  /**
   * Existing rows — used to compute a smart auto-suggestion for the
   * primary key (next integer for numeric keys, next ID-pattern for
   * text keys). Optional; absent → key field stays empty as before.
   */
  rows?: RowMap[];
  onClose: () => void;
  onInserted: (key: string) => void;
}

type FormState = Record<string, string>;

function blankForm(schema: BundleSchema, suggestedKey: string): FormState {
  const out: FormState = {};
  for (const f of [...schema.base_fields, ...schema.fiber_fields]) {
    out[f.name] = "";
  }
  const keyField = schema.base_fields[0]?.name;
  if (keyField && suggestedKey) {
    out[keyField] = suggestedKey;
  }
  return out;
}

/**
 * Suggest a sensible next primary-key value based on existing rows.
 *
 *  - Numeric key:  max(existing) + 1, or `1` if empty
 *  - Text key with `<prefix><digits>` pattern (e.g. "T-001", "INV-2026-042"):
 *    increment the trailing digit block, preserving the zero-pad width.
 *  - Anything else: empty string (the user types whatever they want).
 *
 * This is a *suggestion*; the user can still edit before submitting. The
 * goal is to make "insert an empty row" a one-tap experience without
 * stealing the engine's right to enforce real uniqueness.
 */
export function suggestNextKey(
  rows: RowMap[],
  keyField: string,
  keyType: string,
): string {
  if (rows.length === 0) return keyType === "numeric" ? "1" : "";

  if (keyType === "numeric") {
    let max = 0;
    let any = false;
    for (const r of rows) {
      const v = r[keyField];
      if (typeof v === "number" && Number.isFinite(v)) {
        any = true;
        if (v > max) max = v;
      }
    }
    return any ? String(Math.floor(max) + 1) : "1";
  }

  // Text-style: find a trailing `(\d+)` group in the last row's key
  // (or any row) and increment it, preserving the pad width.
  const sample = rows.find((r) => r[keyField] != null);
  if (!sample) return "";
  const last = String(sample[keyField]);
  const m = last.match(/^(.*?)(\d+)([^\d]*)$/);
  if (!m) return "";
  const prefix = m[1];
  const digits = m[2];
  const tail = m[3];
  // Find the max trailing-number across all rows matching the same prefix.
  let max = parseInt(digits, 10);
  const re = new RegExp(`^${escapeRe(prefix)}(\\d+)${escapeRe(tail)}$`);
  for (const r of rows) {
    const v = r[keyField];
    if (v == null) continue;
    const mm = String(v).match(re);
    if (mm) {
      const n = parseInt(mm[1], 10);
      if (n > max) max = n;
    }
  }
  const next = String(max + 1).padStart(digits.length, "0");
  return `${prefix}${next}${tail}`;
}

function escapeRe(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function InsertRowModal({
  open,
  client,
  bundle,
  schema,
  rows,
  onClose,
  onInserted,
}: InsertRowModalProps) {
  const [form, setForm] = useState<FormState>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open && schema) {
      const keyField = schema.base_fields[0];
      const suggested = keyField && rows
        ? suggestNextKey(rows, keyField.name, keyField.type)
        : "";
      setForm(blankForm(schema, suggested));
      setError(null);
      setBusy(false);
    }
  }, [open, schema, rows]);

  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open || !schema) return null;

  const allFields = [
    ...schema.base_fields.map((f) => ({ ...f, isKey: true })),
    ...schema.fiber_fields.map((f) => ({ ...f, isKey: false })),
  ];
  const keyField = schema.base_fields[0]?.name;

  const submit = async () => {
    if (!keyField) {
      setError("Bundle has no key field — cannot insert.");
      return;
    }
    const keyValue = form[keyField]?.trim();
    if (!keyValue) {
      setError(`Field ${keyField} (the primary key) is required.`);
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const row: Record<string, unknown> = {};
      for (const f of allFields) {
        const raw = form[f.name] ?? "";
        row[f.name] = coerce(raw, f);
      }
      await client.insert(bundle, row);
      onInserted(keyValue);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div
      className="insert-row-bg"
      data-testid="insert-row-bg"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="insert-row-modal" data-testid="insert-row-modal" role="dialog">
        <header className="insert-row-head">
          <h3>
            Add row to <code>{bundle}</code>
          </h3>
          <button
            type="button"
            className="insert-row-close"
            onClick={onClose}
            aria-label="Close"
          >
            ✕
          </button>
        </header>
        <p className="insert-row-hint">
          Only the primary key is required. Leave other fields blank to insert
          an empty row, then fill them in by clicking cells in the grid.
        </p>
        {error ? (
          <div className="insert-row-error" role="alert" data-testid="insert-row-error">
            {error}
          </div>
        ) : null}
        <form
          className="insert-row-body"
          onSubmit={(e) => {
            e.preventDefault();
            submit();
          }}
        >
          {allFields.map((f) => (
            <label
              key={f.name}
              className="insert-row-field"
              data-testid={`insert-row-field-${f.name}`}
            >
              <span className="insert-row-label">
                {f.name}
                <span className="insert-row-type">· {f.type}</span>
                {f.isKey ? <span className="insert-row-key">key</span> : null}
              </span>
              {renderInput(f, form[f.name] ?? "", (v) =>
                setForm({ ...form, [f.name]: v }),
              )}
            </label>
          ))}
          <footer className="insert-row-foot">
            <button
              type="submit"
              className="insert-row-btn insert-row-btn-primary"
              disabled={busy}
              data-testid="insert-row-submit"
            >
              {busy ? "Inserting…" : "Insert row"}
            </button>
            <button
              type="button"
              className="insert-row-btn"
              onClick={onClose}
            >
              Cancel
            </button>
          </footer>
        </form>
      </div>
    </div>
  );
}

function renderInput(
  field: FieldDescriptor,
  value: string,
  onChange: (next: string) => void,
) {
  if (field.type === "boolean") {
    return (
      <select
        className="insert-row-input"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        data-testid={`insert-row-input-${field.name}`}
      >
        <option value=""></option>
        <option value="true">true</option>
        <option value="false">false</option>
      </select>
    );
  }
  return (
    <input
      type={field.type === "numeric" ? "number" : "text"}
      step={field.type === "numeric" ? "any" : undefined}
      className="insert-row-input"
      value={value}
      onChange={(e) => onChange(e.target.value)}
      data-testid={`insert-row-input-${field.name}`}
    />
  );
}

function coerce(raw: string, f: FieldDescriptor): unknown {
  if (raw === "") return null;
  if (f.type === "numeric") {
    const n = Number(raw);
    return Number.isFinite(n) ? n : raw;
  }
  if (f.type === "boolean") {
    if (raw === "true") return true;
    if (raw === "false") return false;
    return null;
  }
  return raw;
}
