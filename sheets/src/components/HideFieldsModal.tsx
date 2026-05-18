import { useEffect, useState } from "react";
import type { BundleSchema } from "../lib/gigi-client";
import "./HideFieldsModal.css";

export interface HideFieldsModalProps {
  open: boolean;
  schema: BundleSchema | null;
  hiddenFields: Set<string>;
  onClose: () => void;
  onChange: (hidden: Set<string>) => void;
}

export function HideFieldsModal({
  open,
  schema,
  hiddenFields,
  onClose,
  onChange,
}: HideFieldsModalProps) {
  const [local, setLocal] = useState<Set<string>>(hiddenFields);

  useEffect(() => {
    if (open) setLocal(new Set(hiddenFields));
  }, [open, hiddenFields]);

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

  const toggle = (name: string) => {
    const next = new Set(local);
    if (next.has(name)) next.delete(name);
    else next.add(name);
    setLocal(next);
  };

  const showAll = () => setLocal(new Set());
  const hideAllButKey = () =>
    setLocal(
      new Set(allFields.filter((f) => !f.isKey).map((f) => f.name)),
    );

  const apply = () => {
    onChange(new Set(local));
    onClose();
  };

  return (
    <div
      className="hide-fields-bg"
      data-testid="hide-fields-bg"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="hide-fields-modal" data-testid="hide-fields-modal" role="dialog">
        <header className="hide-fields-head">
          <h3>Show / hide fields</h3>
          <button
            type="button"
            className="hide-fields-close"
            onClick={onClose}
            aria-label="Close"
          >
            ✕
          </button>
        </header>
        <div className="hide-fields-toolbar">
          <button
            type="button"
            className="hide-fields-btn"
            onClick={showAll}
            data-testid="hide-fields-show-all"
          >
            Show all
          </button>
          <button
            type="button"
            className="hide-fields-btn"
            onClick={hideAllButKey}
            data-testid="hide-fields-hide-non-key"
          >
            Hide everything but the key
          </button>
          <span className="hide-fields-hint">
            {local.size} of {allFields.length} hidden
          </span>
        </div>
        <ul className="hide-fields-list" data-testid="hide-fields-list">
          {allFields.map((f) => (
            <li key={f.name} className="hide-fields-row">
              <label className="hide-fields-label">
                <input
                  type="checkbox"
                  checked={!local.has(f.name)}
                  onChange={() => toggle(f.name)}
                  disabled={f.isKey}
                  data-testid={`hide-fields-check-${f.name}`}
                />
                <span className="hide-fields-name">{f.name}</span>
                <span className="hide-fields-type">· {f.type}</span>
                {f.isKey ? (
                  <span className="hide-fields-tag">key · always shown</span>
                ) : null}
              </label>
            </li>
          ))}
        </ul>
        <footer className="hide-fields-foot">
          <button
            type="button"
            className="hide-fields-btn hide-fields-btn-primary"
            onClick={apply}
            data-testid="hide-fields-apply"
          >
            Apply
          </button>
          <button
            type="button"
            className="hide-fields-btn"
            onClick={onClose}
          >
            Cancel
          </button>
        </footer>
      </div>
    </div>
  );
}
