import { useEffect, useRef, useState } from "react";
import { evaluate, type FormulaContext, type FormulaResult } from "../lib/formula";
import "./FormulaBar.css";

export interface FormulaBarProps {
  /** Lookups required by the evaluator: cell values, sameness, κ, cohort. */
  context: FormulaContext;
  /**
   * Optional initial value — when the user clicks a cell in the grid, the
   * parent can pass that cell's raw value here so the formula bar mirrors
   * what's selected.
   */
  initial?: string;
  /**
   * Called when the user hits Enter or Tab on a formula. `move` tells
   * the parent which direction to advance the active cell after the
   * commit lands — `down` for Enter, `right` for Tab, `null` for the
   * legacy no-move signal. Mirrors Excel's "commit + step" behavior.
   */
  onCommit?: (
    formula: string,
    result: FormulaResult,
    move?: "down" | "right" | null,
  ) => void;
  /**
   * Bump this number to imperatively focus + select-all in the input.
   * Used by `Insert → Formula` and the per-row "Insert formula" context-
   * menu item to drop the user straight into formula-typing without
   * needing to click the bar. The parent owns the counter; the bar just
   * watches for changes.
   */
  focusToken?: number;
  /**
   * When `focusToken` bumps and `prefill` is set, the input is replaced
   * with this text *before* focusing — that's how Insert → Formula puts
   * `=` in place and parks the cursor at the end.
   */
  prefill?: string;
  /**
   * Optional "this view is sorted/filtered" indicator shown as a small
   * pill at the right edge of the bar. Reminds the user that cell refs
   * like `A3` resolve against the *currently visible* row list, so
   * `=temperature[3]` means "the 3rd row I'm looking at" — not "row 3
   * of the bundle." Pass `null` to hide the pill.
   */
  viewStatus?: { label: string; tooltip: string } | null;
  /**
   * Click handler for the fx label. When provided, the label becomes a
   * button that opens the FormulaPicker. Without it the label is just
   * a static decoration.
   */
  onFxClick?: () => void;
  /**
   * Aggregate stats for the current multi-row selection. When set, the
   * bar's right-hand panel switches from the formula-eval result to a
   * "Sum · Avg · Count" strip so the user gets instant feedback on the
   * selected range — Excel's status-bar idiom, in line.
   */
  rangeStats?: RangeStats | null;
}

export interface RangeStats {
  /** Number of cells in the selection (counts all rows, including blanks). */
  count: number;
  /** Number of cells with a numeric value (drives whether sum/avg/min/max are surfaced). */
  numericCount: number;
  /** Aggregates over the numeric cells, undefined when numericCount === 0. */
  sum?: number;
  avg?: number;
  min?: number;
  max?: number;
  /** Field name that's being aggregated — shown for context. */
  field?: string;
}

/**
 * The formula bar — Excel's `fx` field, GIGI-flavored.
 *
 * Accepts ordinary literals (numbers, strings) or formulas starting with `=`.
 * The right-hand panel shows the live evaluated result so you can build a
 * formula and watch it compute as you type.
 *
 * GIGI primitives surfaced:
 *   `=SAME(A1, B1)`   Davis sameness
 *   `=DIST(A1, B1)`   Davis distance (derived from SAME via the identity)
 *   `=K(A1)`          κ-curvature of the row containing A1
 *   `=COHORT("col")`  cohort label for a column
 */
export function FormulaBar({
  context,
  initial = "",
  onCommit,
  focusToken,
  prefill,
  viewStatus,
  onFxClick,
  rangeStats,
}: FormulaBarProps) {
  const [text, setText] = useState(initial);
  const inputRef = useRef<HTMLInputElement | null>(null);
  // Re-mirror when the parent's selected-cell value changes.
  useEffect(() => {
    setText(initial);
  }, [initial]);
  // Imperative focus + prefill, triggered by a token bump from the parent.
  useEffect(() => {
    if (focusToken === undefined) return;
    if (prefill !== undefined) setText(prefill);
    // Defer to next frame so the controlled-input value update lands first.
    requestAnimationFrame(() => {
      const el = inputRef.current;
      if (!el) return;
      el.focus();
      // Park the cursor at the end (typing `=AVG(` then continuing is the
      // expected flow — select-all would erase what we just prefilled).
      const end = el.value.length;
      el.setSelectionRange(end, end);
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [focusToken]);

  const result = evaluate(text, context);
  const isFormula = text.startsWith("=");

  return (
    <div className="formula-bar" data-testid="formula-bar">
      {onFxClick ? (
        <button
          type="button"
          className="formula-fx formula-fx-btn"
          onClick={onFxClick}
          title="Insert function — walk through every available formula"
          data-testid="formula-fx-btn"
          aria-label="Insert function"
        >
          f<i>x</i>
        </button>
      ) : (
        <span className="formula-fx" aria-hidden="true">
          f<i>x</i>
        </span>
      )}
      <input
        ref={inputRef}
        className="formula-input"
        type="text"
        value={text}
        spellCheck={false}
        placeholder={"=SAME(A1, A2)  ·  =K(A1)  ·  =SUM(A1:A10)"}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            // Shift+Enter moves up rather than down (Excel parity).
            onCommit?.(text, result, e.shiftKey ? null : "down");
          } else if (e.key === "Tab") {
            e.preventDefault();
            onCommit?.(text, result, "right");
          } else if (e.key === "Escape") {
            // Bail out without committing — restore the mirrored value
            // from the selected cell.
            e.preventDefault();
            setText(initial);
            inputRef.current?.blur();
          }
        }}
        data-testid="formula-input"
        aria-label="Formula bar"
      />
      {viewStatus ? (
        <span
          className="formula-view-status"
          data-testid="formula-view-status"
          title={viewStatus.tooltip}
        >
          {viewStatus.label}
        </span>
      ) : null}
      {rangeStats ? (
        <div className="formula-range-stats" data-testid="formula-range-stats">
          <span className="formula-range-stat">
            <span className="formula-range-stat-label">Count</span>
            <span className="formula-range-stat-value">{rangeStats.count}</span>
          </span>
          {rangeStats.numericCount > 0 ? (
            <>
              <span className="formula-range-stat">
                <span className="formula-range-stat-label">Sum</span>
                <span className="formula-range-stat-value">{formatStat(rangeStats.sum)}</span>
              </span>
              <span className="formula-range-stat">
                <span className="formula-range-stat-label">Avg</span>
                <span className="formula-range-stat-value">{formatStat(rangeStats.avg)}</span>
              </span>
              <span className="formula-range-stat">
                <span className="formula-range-stat-label">Min</span>
                <span className="formula-range-stat-value">{formatStat(rangeStats.min)}</span>
              </span>
              <span className="formula-range-stat">
                <span className="formula-range-stat-label">Max</span>
                <span className="formula-range-stat-value">{formatStat(rangeStats.max)}</span>
              </span>
            </>
          ) : null}
        </div>
      ) : (
        <div className="formula-result" data-testid="formula-result">
          {isFormula ? (
            result.error ? (
              <span className="formula-err" title={result.error}>
                {result.error}
              </span>
            ) : (
              <span className="formula-ok">
                <span className="formula-result-label">=</span>
                <span className="formula-result-value">
                  {formatResult(result.value)}
                </span>
              </span>
            )
          ) : null}
        </div>
      )}
    </div>
  );
}

function formatResult(v: unknown): string {
  if (v === null || v === undefined) return "";
  if (typeof v === "number") {
    // 4 sig figs unless it's an integer
    if (Number.isInteger(v)) return String(v);
    return v.toFixed(Math.abs(v) < 1 ? 4 : 2);
  }
  return String(v);
}

function formatStat(v: number | undefined): string {
  if (v === undefined || !Number.isFinite(v)) return "—";
  if (Number.isInteger(v)) return v.toLocaleString();
  // 2 decimal places is enough for the status-bar surface; precise math
  // belongs in formulas. Use locale formatting so 1,234.56 reads.
  return v.toLocaleString(undefined, { maximumFractionDigits: 2 });
}
