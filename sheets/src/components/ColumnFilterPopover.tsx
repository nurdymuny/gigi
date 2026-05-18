import { useEffect, useRef, useState } from "react";
import type { Filter } from "../lib/filter";
import type { FieldDescriptor } from "../lib/gigi-client";
import "./ColumnFilterPopover.css";

export interface ColumnFilterPopoverProps {
  /** The column being filtered. */
  field: FieldDescriptor;
  /** The current filter on this column, if any. */
  filter: Filter | null;
  /** Called when the user applies (or clears) the column filter. */
  onChange: (filter: Filter | null) => void;
  /** Close the popover (Esc / backdrop / done). */
  onClose: () => void;
  /** Anchor element — the popover positions itself relative to it. */
  anchorEl: HTMLElement | null;
}

/**
 * Per-column filter popover. Type-aware UI:
 *
 *   text / categorical / timestamp   → "Contains" substring
 *   numeric                          → Min / Max range
 *   boolean                          → true / false / both
 *
 * Designed to live above the grid, anchored under a column header's
 * funnel icon. The host (Grid) decides which column's popover is open
 * and provides the anchor element for positioning.
 */
export function ColumnFilterPopover({
  field,
  filter,
  onChange,
  onClose,
  anchorEl,
}: ColumnFilterPopoverProps) {
  const popRef = useRef<HTMLDivElement>(null);
  // Position the popover under the anchor.
  const [pos, setPos] = useState<{ top: number; left: number }>({ top: 0, left: 0 });
  useEffect(() => {
    if (!anchorEl) return;
    const r = anchorEl.getBoundingClientRect();
    setPos({ top: r.bottom + 4, left: r.left });
  }, [anchorEl]);

  // Dismiss on outside click or Escape.
  useEffect(() => {
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
  }, [onClose, anchorEl]);

  const isNumeric = field.type === "numeric";
  const isBoolean = field.type === "boolean";

  // Local form state, seeded from the existing filter.
  const [contains, setContains] = useState<string>(
    filter?.kind === "text" ? filter.value : "",
  );
  const [min, setMin] = useState<string>(
    filter?.kind === "range" && filter.min != null ? String(filter.min) : "",
  );
  const [max, setMax] = useState<string>(
    filter?.kind === "range" && filter.max != null ? String(filter.max) : "",
  );
  const [boolVal, setBoolVal] = useState<"both" | "true" | "false">(
    filter?.kind === "text" && (filter.value === "true" || filter.value === "false")
      ? (filter.value as "true" | "false")
      : "both",
  );

  function apply() {
    if (isNumeric) {
      const minN = min.trim() === "" ? undefined : Number(min);
      const maxN = max.trim() === "" ? undefined : Number(max);
      if (minN == null && maxN == null) {
        onChange(null);
      } else if (
        (minN != null && !Number.isFinite(minN)) ||
        (maxN != null && !Number.isFinite(maxN))
      ) {
        // Bad input → no-op.
        return;
      } else {
        onChange({ kind: "range", column: field.name, min: minN, max: maxN });
      }
    } else if (isBoolean) {
      if (boolVal === "both") {
        onChange(null);
      } else {
        onChange({
          kind: "text",
          column: field.name,
          op: "equals",
          value: boolVal,
        });
      }
    } else {
      if (contains.trim() === "") {
        onChange(null);
      } else {
        onChange({
          kind: "text",
          column: field.name,
          op: "contains",
          value: contains,
        });
      }
    }
    onClose();
  }

  function clear() {
    onChange(null);
    onClose();
  }

  return (
    <div
      ref={popRef}
      className="col-filter-pop"
      data-testid="col-filter-pop"
      style={{ top: pos.top, left: pos.left }}
      role="dialog"
      aria-label={`Filter ${field.name}`}
    >
      <header className="col-filter-pop-head">
        <span className="col-filter-pop-title">{field.name}</span>
        <span className="col-filter-pop-type">{field.type}</span>
      </header>
      <div className="col-filter-pop-body">
        {isNumeric ? (
          <>
            <label className="col-filter-pop-row">
              <span>Min</span>
              <input
                type="number"
                value={min}
                onChange={(e) => setMin(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") apply();
                }}
                data-testid="col-filter-min"
                placeholder="—"
                autoFocus
              />
            </label>
            <label className="col-filter-pop-row">
              <span>Max</span>
              <input
                type="number"
                value={max}
                onChange={(e) => setMax(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") apply();
                }}
                data-testid="col-filter-max"
                placeholder="—"
              />
            </label>
          </>
        ) : isBoolean ? (
          <label className="col-filter-pop-row">
            <span>Value</span>
            <select
              value={boolVal}
              onChange={(e) => setBoolVal(e.target.value as typeof boolVal)}
              data-testid="col-filter-bool"
              autoFocus
            >
              <option value="both">Both</option>
              <option value="true">true</option>
              <option value="false">false</option>
            </select>
          </label>
        ) : (
          <label className="col-filter-pop-row">
            <span>Contains</span>
            <input
              type="text"
              value={contains}
              onChange={(e) => setContains(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") apply();
              }}
              data-testid="col-filter-contains"
              placeholder="substring…"
              autoFocus
              spellCheck={false}
            />
          </label>
        )}
      </div>
      <footer className="col-filter-pop-foot">
        <button
          type="button"
          className="col-filter-pop-clear"
          onClick={clear}
          data-testid="col-filter-clear"
        >
          Clear
        </button>
        <button
          type="button"
          className="col-filter-pop-apply"
          onClick={apply}
          data-testid="col-filter-apply"
        >
          Apply
        </button>
      </footer>
    </div>
  );
}
