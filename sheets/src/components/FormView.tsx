import { useMemo, useState } from "react";
import type { BundleSchema, FieldDescriptor } from "../lib/gigi-client";
import "./FormView.css";

export interface FormViewProps {
  schema: BundleSchema | null;
  /**
   * Submit handler — receives the typed field values keyed by name.
   * Returns a Promise so the form can show a spinner / disable submit.
   */
  onSubmit: (values: Record<string, unknown>) => Promise<void> | void;
}

/**
 * Single-row intake form generated from the bundle's schema. Each
 * non-opaque, non-bool-flag fiber field gets an input matched to its type.
 *
 * §14 in FEATURE_PARITY.md calls for a pre-insert κ check that warns when
 * the submission looks anomalous against the cohort. The hook is reserved
 * here (see `previewKappa` placeholder); wiring it requires the embedder
 * bridge that the Phase 3 plan covers.
 */
export function FormView({ schema, onSubmit }: FormViewProps) {
  const [values, setValues] = useState<Record<string, string>>({});
  const [submitting, setSubmitting] = useState(false);
  const [confirmation, setConfirmation] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const fields = useMemo<FieldDescriptor[]>(() => {
    if (!schema) return [];
    return [...schema.base_fields, ...schema.fiber_fields].filter(
      (f) => !f.encryption || f.encryption === "none",
    );
  }, [schema]);

  if (!schema) {
    return (
      <div className="form-view form-view-empty" data-testid="form-view-empty">
        <p>Loading bundle…</p>
      </div>
    );
  }
  if (fields.length === 0) {
    return (
      <div className="form-view form-view-empty" data-testid="form-view-empty">
        <p>No writable fields in this bundle.</p>
      </div>
    );
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setConfirmation(null);
    // Parse the strings into the right types per the schema.
    const parsed: Record<string, unknown> = {};
    for (const f of fields) {
      const raw = values[f.name];
      if (raw == null || raw === "") continue;
      if (f.type === "numeric") {
        const n = Number(raw);
        if (!Number.isFinite(n)) {
          setError(`"${f.name}" expects a number, got "${raw}"`);
          return;
        }
        parsed[f.name] = n;
      } else if (f.type === "boolean") {
        parsed[f.name] = raw === "true" || raw === "1" || raw === "yes";
      } else {
        parsed[f.name] = raw;
      }
    }
    setSubmitting(true);
    try {
      await onSubmit(parsed);
      setConfirmation(
        `Row submitted with ${Object.keys(parsed).length} field${Object.keys(parsed).length === 1 ? "" : "s"}.`,
      );
      setValues({});
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <form
      className="form-view"
      data-testid="form-view"
      onSubmit={handleSubmit}
      noValidate
    >
      <div className="form-view-head">
        <h2>New row in {schema.name}</h2>
        <p>
          Fill in the fields below and submit. Schema-derived types are enforced
          on submit — non-numeric values into a numeric field return an error
          before the engine even sees them.
        </p>
      </div>

      <div className="form-view-fields">
        {fields.map((f) => (
          <FieldInput
            key={f.name}
            field={f}
            value={values[f.name] ?? ""}
            onChange={(v) => setValues((prev) => ({ ...prev, [f.name]: v }))}
          />
        ))}
      </div>

      {error ? (
        <p className="form-view-error" role="alert" data-testid="form-view-error">
          {error}
        </p>
      ) : null}
      {confirmation ? (
        <p
          className="form-view-confirmation"
          role="status"
          data-testid="form-view-confirmation"
        >
          {confirmation}
        </p>
      ) : null}

      <div className="form-view-actions">
        <button
          type="submit"
          className="form-view-submit"
          disabled={submitting}
          data-testid="form-view-submit"
        >
          {submitting ? "Submitting…" : "Submit row"}
        </button>
        <button
          type="button"
          className="form-view-reset"
          disabled={submitting}
          onClick={() => {
            setValues({});
            setConfirmation(null);
            setError(null);
          }}
          data-testid="form-view-reset"
        >
          Reset
        </button>
      </div>
    </form>
  );
}

function FieldInput({
  field,
  value,
  onChange,
}: {
  field: FieldDescriptor;
  value: string;
  onChange: (v: string) => void;
}) {
  const inputId = `form-field-${field.name}`;
  const common = {
    id: inputId,
    "data-testid": `form-field-${field.name}`,
    value,
    onChange: (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) =>
      onChange(e.target.value),
  };
  return (
    <label className="form-view-field" htmlFor={inputId}>
      <span className="form-view-field-label">
        {field.name}
        <small>{field.type}</small>
      </span>
      {field.type === "boolean" ? (
        <select {...common}>
          <option value="">—</option>
          <option value="true">true</option>
          <option value="false">false</option>
        </select>
      ) : field.type === "numeric" ? (
        <input type="number" step="any" {...common} />
      ) : field.type === "timestamp" ? (
        <input type="date" {...common} />
      ) : (
        <input type="text" {...common} />
      )}
    </label>
  );
}
