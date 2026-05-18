import { useEffect, useRef, useState } from "react";
import "./ConditionalFormatModal.css";

/**
 * A conditional-format rule attached to a column.
 *
 *   `kappaThreshold` — highlight cells whose row's κ is ≥ this value
 *   `color`          — preset color name; the Grid maps to a CSS class
 *
 * v1 supports a single κ-threshold rule per column. The math layer
 * (`lib/format.ts`) ships richer rule grammar (`[κ>τ]"⚠️ "0.00`); this
 * UI gives users a starting point without exposing format-string syntax.
 */
export type ConditionalFormatColor = "red" | "amber" | "green" | "blue" | "purple";

export interface ConditionalFormatRule {
  kappaThreshold: number;
  color: ConditionalFormatColor;
}

export interface ConditionalFormatModalProps {
  open: boolean;
  /** Which column the rule applies to. */
  field: string;
  /** Existing rule on this column, if any. */
  rule: ConditionalFormatRule | null;
  /** Called when the user applies or clears the rule. */
  onChange: (rule: ConditionalFormatRule | null) => void;
  onClose: () => void;
  /** Anchor — popover positions itself relative to this element. */
  anchorEl: HTMLElement | null;
}

const COLOR_LABELS: Record<ConditionalFormatColor, string> = {
  red: "Red",
  amber: "Amber",
  green: "Green",
  blue: "Blue",
  purple: "Purple",
};

export function ConditionalFormatModal({
  open,
  field,
  rule,
  onChange,
  onClose,
  anchorEl,
}: ConditionalFormatModalProps) {
  const popRef = useRef<HTMLDivElement>(null);
  const [threshold, setThreshold] = useState<number>(rule?.kappaThreshold ?? 0.3);
  const [color, setColor] = useState<ConditionalFormatColor>(rule?.color ?? "red");
  const [pos, setPos] = useState<{ top: number; left: number }>({ top: 0, left: 0 });

  useEffect(() => {
    if (!open) return;
    setThreshold(rule?.kappaThreshold ?? 0.3);
    setColor(rule?.color ?? "red");
  }, [open, rule]);

  useEffect(() => {
    if (!anchorEl) return;
    const r = anchorEl.getBoundingClientRect();
    setPos({ top: r.bottom + 4, left: r.left });
  }, [anchorEl]);

  useEffect(() => {
    if (!open) return;
    function onDoc(e: MouseEvent) {
      if (!popRef.current) return;
      if (
        popRef.current.contains(e.target as Node) ||
        (anchorEl && anchorEl.contains(e.target as Node))
      ) {
        return;
      }
      onClose();
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [open, anchorEl, onClose]);

  if (!open) return null;

  function apply() {
    onChange({ kappaThreshold: threshold, color });
    onClose();
  }

  function clear() {
    onChange(null);
    onClose();
  }

  return (
    <div
      ref={popRef}
      className="cf-pop"
      data-testid="cf-pop"
      style={{ top: pos.top, left: pos.left }}
      role="dialog"
      aria-label={`Conditional format for ${field}`}
    >
      <header className="cf-pop-head">
        <span className="cf-pop-title">{field}</span>
        <span className="cf-pop-sub">Highlight cells when κ ≥ threshold</span>
      </header>
      <div className="cf-pop-body">
        <label className="cf-pop-row">
          <span>κ ≥</span>
          <input
            type="number"
            min="0"
            max="3"
            step="0.05"
            value={threshold}
            onChange={(e) => setThreshold(Number(e.target.value))}
            data-testid="cf-threshold"
            autoFocus
          />
          <span className="cf-pop-presets">
            <button
              type="button"
              className="cf-pop-preset-btn"
              onClick={() => setThreshold(0.1)}
              data-testid="cf-preset-drift"
              title="Drift threshold"
            >
              0.1 drift
            </button>
            <button
              type="button"
              className="cf-pop-preset-btn"
              onClick={() => setThreshold(0.3)}
              data-testid="cf-preset-bad"
              title="Anomaly threshold"
            >
              0.3 bad
            </button>
          </span>
        </label>
        <fieldset className="cf-pop-row cf-pop-colors">
          <span>Color</span>
          <div className="cf-pop-swatches" role="radiogroup" aria-label="Highlight color">
            {(Object.keys(COLOR_LABELS) as ConditionalFormatColor[]).map((c) => (
              <button
                key={c}
                type="button"
                role="radio"
                aria-checked={color === c}
                className={`cf-pop-swatch cf-pop-swatch-${c} ${color === c ? "cf-pop-swatch-active" : ""}`}
                onClick={() => setColor(c)}
                data-testid={`cf-swatch-${c}`}
                title={COLOR_LABELS[c]}
              />
            ))}
          </div>
        </fieldset>
        <p className="cf-pop-preview-label">Preview</p>
        <div className={`cf-pop-preview cf-cond-${color}`} data-testid="cf-pop-preview">
          sample value
        </div>
      </div>
      <footer className="cf-pop-foot">
        <button
          type="button"
          className="cf-pop-clear"
          onClick={clear}
          data-testid="cf-clear"
        >
          Clear
        </button>
        <button
          type="button"
          className="cf-pop-apply"
          onClick={apply}
          data-testid="cf-apply"
        >
          Apply
        </button>
      </footer>
    </div>
  );
}
