import { useEffect, useState } from "react";
import type { BundleSchema, FieldDescriptor, SheetsClient } from "../lib/gigi-client";
import "./SchemaModal.css";

export interface SchemaModalProps {
  open: boolean;
  client: SheetsClient;
  bundle: string;
  schema: BundleSchema | null;
  onClose: () => void;
  onMutated: () => void;
}

type Mode = "list" | "add";
type FieldKind = "text" | "numeric" | "categorical" | "boolean" | "timestamp";
type Encryption = "none" | "opaque" | "indexed" | "affine";

interface AddForm {
  name: string;
  type: FieldKind;
  defaultValue: string;
  encryption: Encryption;
}

const INITIAL_ADD: AddForm = {
  name: "",
  type: "text",
  defaultValue: "",
  encryption: "none",
};

export function SchemaModal({
  open,
  client,
  bundle,
  schema,
  onClose,
  onMutated,
}: SchemaModalProps) {
  const [mode, setMode] = useState<Mode>("list");
  const [form, setForm] = useState<AddForm>(INITIAL_ADD);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setMode("list");
      setForm(INITIAL_ADD);
      setError(null);
      setBusy(null);
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

  if (!open || !schema) return null;

  const allFields: Array<FieldDescriptor & { isKey: boolean }> = [
    ...schema.base_fields.map((f) => ({ ...f, isKey: true })),
    ...schema.fiber_fields.map((f) => ({ ...f, isKey: false })),
  ];

  const onAdd = async () => {
    if (!form.name) {
      setError("Field name is required.");
      return;
    }
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(form.name)) {
      setError("Field name must match [A-Za-z_][A-Za-z0-9_]*");
      return;
    }
    setBusy("add");
    setError(null);
    try {
      await client.addField(bundle, {
        name: form.name,
        type: form.type,
        ...(form.defaultValue
          ? { default: form.type === "numeric" ? Number(form.defaultValue) : form.defaultValue }
          : {}),
      });
      onMutated();
      setMode("list");
      setForm(INITIAL_ADD);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(null);
    }
  };

  const onDrop = async (name: string) => {
    if (!confirm(`Drop field '${name}'? This cannot be undone.`)) return;
    setBusy(`drop:${name}`);
    setError(null);
    try {
      await client.dropField(bundle, name);
      onMutated();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(null);
    }
  };

  return (
    <div
      className="schema-modal-bg"
      data-testid="schema-modal-bg"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="schema-modal" data-testid="schema-modal" role="dialog">
        <header className="schema-modal-head">
          <h2>
            <code>{schema.name}</code> · schema
          </h2>
          <button
            type="button"
            className="schema-modal-close"
            onClick={onClose}
            aria-label="Close"
          >
            ✕
          </button>
        </header>

        {error ? (
          <div
            className="schema-modal-error"
            role="alert"
            data-testid="schema-modal-error"
          >
            {error}
          </div>
        ) : null}

        {mode === "list" ? (
          <>
            <div className="schema-modal-toolbar">
              <button
                type="button"
                className="schema-modal-btn schema-modal-btn-primary"
                onClick={() => setMode("add")}
                data-testid="schema-add"
              >
                + Add field
              </button>
              <span className="schema-modal-hint">
                {schema.records.toLocaleString()} rows · {allFields.length} fields
              </span>
            </div>
            <ul className="schema-field-list" data-testid="schema-field-list">
              {allFields.map((f) => (
                <li
                  key={f.name}
                  className="schema-field-row"
                  data-testid={`schema-field-${f.name}`}
                >
                  <span className="schema-field-name">{f.name}</span>
                  <span className="schema-field-meta">
                    <span className="schema-field-type">
                      {f.type.charAt(0).toUpperCase() + f.type.slice(1)}
                    </span>
                    {f.isKey ? (
                      <span className="schema-field-tag schema-field-tag-key">key</span>
                    ) : null}
                    {schema.indexed_fields.includes(f.name) ? (
                      <span className="schema-field-tag schema-field-tag-idx">indexed</span>
                    ) : null}
                    {f.encryption && f.encryption !== "none" ? (
                      <span className="schema-field-tag schema-field-tag-enc">
                        encrypted · {f.encryption}
                      </span>
                    ) : null}
                  </span>
                  <button
                    type="button"
                    className="schema-field-drop"
                    onClick={() => onDrop(f.name)}
                    disabled={f.isKey || busy === `drop:${f.name}`}
                    title={f.isKey ? "Primary key fields cannot be dropped" : "Drop this field"}
                    data-testid={`schema-drop-${f.name}`}
                  >
                    {busy === `drop:${f.name}` ? "…" : "Drop"}
                  </button>
                </li>
              ))}
            </ul>
          </>
        ) : (
          <form
            className="schema-add-form"
            data-testid="schema-add-form"
            onSubmit={(e) => {
              e.preventDefault();
              onAdd();
            }}
          >
            <label className="schema-add-row">
              <span>Name</span>
              <input
                type="text"
                value={form.name}
                autoFocus
                onChange={(e) => setForm({ ...form, name: e.target.value })}
                placeholder="e.g. pressure_hpa"
                data-testid="schema-form-name"
              />
            </label>
            <label className="schema-add-row">
              <span>Type</span>
              <select
                value={form.type}
                onChange={(e) =>
                  setForm({ ...form, type: e.target.value as FieldKind })
                }
                data-testid="schema-form-type"
              >
                <option value="text">text</option>
                <option value="numeric">numeric</option>
                <option value="categorical">categorical</option>
                <option value="boolean">boolean</option>
                <option value="timestamp">timestamp</option>
              </select>
            </label>
            <label className="schema-add-row">
              <span>Encryption</span>
              <select
                value={form.encryption}
                onChange={(e) =>
                  setForm({ ...form, encryption: e.target.value as Encryption })
                }
                data-testid="schema-form-encryption"
              >
                <option value="none">none</option>
                <option value="opaque">opaque — value masked in UI</option>
                <option value="indexed">indexed — equality lookups</option>
                <option value="affine">affine — numeric gauge</option>
              </select>
            </label>
            <label className="schema-add-row">
              <span>Default</span>
              <input
                type="text"
                value={form.defaultValue}
                onChange={(e) =>
                  setForm({ ...form, defaultValue: e.target.value })
                }
                placeholder="(optional)"
                data-testid="schema-form-default"
              />
            </label>
            <div className="schema-modal-toolbar">
              <button
                type="submit"
                className="schema-modal-btn schema-modal-btn-primary"
                disabled={busy === "add"}
                data-testid="schema-form-submit"
              >
                {busy === "add" ? "Adding…" : "Add field"}
              </button>
              <button
                type="button"
                className="schema-modal-btn"
                onClick={() => setMode("list")}
                data-testid="schema-form-cancel"
              >
                Cancel
              </button>
            </div>
            {form.encryption !== "none" ? (
              <p className="schema-modal-note">
                <strong>Heads up — demo overlay:</strong> setting an
                encryption mode here only tags the field with a UI marker
                (lock icon + masked rendering). Actual cryptography is
                enforced by the GIGI engine; this demo client doesn't
                encrypt data before sending it. Use against a real
                engine-side schema for production data.
              </p>
            ) : null}
          </form>
        )}
      </div>
    </div>
  );
}
