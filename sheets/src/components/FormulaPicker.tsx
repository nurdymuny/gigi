import { useEffect, useMemo, useRef, useState } from "react";
import { evaluate, type FormulaContext } from "../lib/formula";
import {
  CATEGORY_LABELS,
  FORMULA_DOCS,
  assembleFormula,
  searchDocs,
  type FormulaCategory,
  type FormulaDoc,
} from "../lib/formula-docs";
import "./FormulaPicker.css";

export interface FormulaPickerProps {
  open: boolean;
  onClose: () => void;
  /**
   * Live-evaluation context. The picker uses this to show the "Result:"
   * preview as the user fills in arguments. Pass the same context as
   * the formula bar so the preview matches what the formula will return
   * once inserted.
   */
  context: FormulaContext;
  /**
   * Called with the assembled `=FN(...)` string when the user clicks
   * Insert. The host wires this to the formula bar (or directly to the
   * selected cell). Insert is disabled if no callback is provided.
   */
  onInsert?: (formula: string) => void;
}

type Mode = { kind: "list" } | { kind: "edit"; doc: FormulaDoc };

/**
 * Excel-style "Insert Function" dialog. Two steps:
 *
 *   1. Function list — search + category sidebar + clickable rows.
 *   2. Argument editor — one input per arg with help text, live preview
 *      of the evaluated result, Insert button to commit.
 *
 * The picker is presentational — it doesn't know about cells, the
 * formula bar, or the bundle. Callers (App.tsx) wire the `onInsert`
 * callback to drop the assembled formula wherever it should land.
 */
export function FormulaPicker({ open, onClose, context, onInsert }: FormulaPickerProps) {
  const [mode, setMode] = useState<Mode>({ kind: "list" });
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState<FormulaCategory | "all">("all");
  /**
   * One input string per argument slot for the current function. Sized
   * to the doc's `args` length when the user picks a function. Optional
   * args at the end can stay empty — assembleFormula trims them.
   */
  const [argValues, setArgValues] = useState<string[]>([]);
  const searchRef = useRef<HTMLInputElement>(null);

  // Reset to the list view + clear search whenever the dialog opens.
  useEffect(() => {
    if (!open) return;
    setMode({ kind: "list" });
    setQuery("");
    setCategory("all");
    setArgValues([]);
    // Focus the search box on next tick so the user can start typing.
    requestAnimationFrame(() => searchRef.current?.focus());
  }, [open]);

  // Esc closes — but if we're in the edit step, Esc returns to the list
  // first (one Esc = one navigation step, matches Excel/Sheets/Numbers).
  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key !== "Escape") return;
      if (mode.kind === "edit") setMode({ kind: "list" });
      else onClose();
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open, onClose, mode]);

  const filtered = useMemo(() => {
    const all = searchDocs(query);
    if (category === "all") return all;
    return all.filter((d) => d.category === category);
  }, [query, category]);

  const countsByCat = useMemo(() => {
    const m = new Map<FormulaCategory, number>();
    for (const d of FORMULA_DOCS) m.set(d.category, (m.get(d.category) ?? 0) + 1);
    return m;
  }, []);

  if (!open) return null;

  function selectDoc(d: FormulaDoc) {
    setMode({ kind: "edit", doc: d });
    setArgValues(d.args.map(() => ""));
  }

  function backToList() {
    setMode({ kind: "list" });
    setArgValues([]);
  }

  return (
    <div
      className="formula-picker-bg"
      data-testid="formula-picker-bg"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className="formula-picker"
        data-testid="formula-picker"
        role="dialog"
        aria-label="Insert function"
      >
        <button
          type="button"
          className="formula-picker-close"
          onClick={onClose}
          aria-label="Close"
          data-testid="formula-picker-close"
        >
          ✕
        </button>

        {mode.kind === "list" ? (
          <ListView
            searchRef={searchRef}
            query={query}
            onQueryChange={setQuery}
            category={category}
            onCategoryChange={setCategory}
            countsByCat={countsByCat}
            results={filtered}
            onSelect={selectDoc}
          />
        ) : (
          <EditView
            doc={mode.doc}
            args={argValues}
            onArgChange={(i, v) =>
              setArgValues((prev) => {
                const next = [...prev];
                next[i] = v;
                return next;
              })
            }
            context={context}
            onBack={backToList}
            onInsert={(formula) => {
              onInsert?.(formula);
              onClose();
            }}
            canInsert={Boolean(onInsert)}
          />
        )}
      </div>
    </div>
  );
}

/* ── list step ───────────────────────────────────────────────────── */

function ListView({
  searchRef,
  query,
  onQueryChange,
  category,
  onCategoryChange,
  countsByCat,
  results,
  onSelect,
}: {
  searchRef: React.RefObject<HTMLInputElement>;
  query: string;
  onQueryChange: (q: string) => void;
  category: FormulaCategory | "all";
  onCategoryChange: (c: FormulaCategory | "all") => void;
  countsByCat: Map<FormulaCategory, number>;
  results: FormulaDoc[];
  onSelect: (d: FormulaDoc) => void;
}) {
  return (
    <>
      <header className="formula-picker-header">
        <h2>Insert function</h2>
        <input
          ref={searchRef}
          className="formula-picker-search"
          type="text"
          placeholder="Search by name or what it does…"
          value={query}
          onChange={(e) => onQueryChange(e.target.value)}
          data-testid="formula-picker-search"
          spellCheck={false}
        />
      </header>
      <div className="formula-picker-body">
        <aside className="formula-picker-cats" data-testid="formula-picker-cats">
          <CategoryRow
            active={category === "all"}
            onClick={() => onCategoryChange("all")}
            label="All"
            count={FORMULA_DOCS.length}
          />
          {(Object.keys(CATEGORY_LABELS) as FormulaCategory[]).map((c) => (
            <CategoryRow
              key={c}
              active={category === c}
              onClick={() => onCategoryChange(c)}
              label={CATEGORY_LABELS[c]}
              count={countsByCat.get(c) ?? 0}
            />
          ))}
        </aside>
        <ul className="formula-picker-results" data-testid="formula-picker-results">
          {results.length === 0 ? (
            <li className="formula-picker-empty">No functions match "{query}".</li>
          ) : (
            results.map((d) => (
              <li
                key={d.name}
                className="formula-picker-result"
                data-testid={`formula-picker-result-${d.name}`}
              >
                <button
                  type="button"
                  className="formula-picker-result-btn"
                  onClick={() => onSelect(d)}
                >
                  <div className="formula-picker-result-row1">
                    <span className="formula-picker-name">{d.name}</span>
                    <span className="formula-picker-cat-pill">
                      {CATEGORY_LABELS[d.category]}
                    </span>
                  </div>
                  <div className="formula-picker-sig">{d.signature}</div>
                  <div className="formula-picker-desc">{d.description}</div>
                </button>
              </li>
            ))
          )}
        </ul>
      </div>
    </>
  );
}

function CategoryRow({
  active,
  onClick,
  label,
  count,
}: {
  active: boolean;
  onClick: () => void;
  label: string;
  count: number;
}) {
  return (
    <button
      type="button"
      className={`formula-picker-cat ${active ? "formula-picker-cat-active" : ""}`}
      onClick={onClick}
    >
      <span>{label}</span>
      <span className="formula-picker-cat-count">{count}</span>
    </button>
  );
}

/* ── edit step ───────────────────────────────────────────────────── */

function EditView({
  doc,
  args,
  onArgChange,
  context,
  onBack,
  onInsert,
  canInsert,
}: {
  doc: FormulaDoc;
  args: string[];
  onArgChange: (i: number, v: string) => void;
  context: FormulaContext;
  onBack: () => void;
  onInsert: (formula: string) => void;
  canInsert: boolean;
}) {
  const assembled = assembleFormula(doc.name, args);
  const result = evaluate(assembled, context);
  // Disable insert when no args are filled AND the function isn't zero-
  // arity (TODAY()). Keeps users from inserting `=SUM()` (a #ERROR!).
  const hasContent = doc.args.length === 0 || args.some((a) => a.trim() !== "");
  const insertDisabled = !canInsert || !hasContent;

  return (
    <>
      <header className="formula-picker-header formula-picker-header-edit">
        <button
          type="button"
          className="formula-picker-back"
          onClick={onBack}
          data-testid="formula-picker-back"
          aria-label="Back to list"
        >
          ← Back
        </button>
        <div className="formula-picker-edit-titles">
          <h2 className="formula-picker-edit-name">{doc.name}</h2>
          <div className="formula-picker-edit-sig">{doc.signature}</div>
        </div>
      </header>
      <div className="formula-picker-edit-body">
        <p className="formula-picker-edit-desc">{doc.description}</p>
        {doc.details ? (
          <p className="formula-picker-edit-details">{doc.details}</p>
        ) : null}

        {doc.args.length === 0 ? (
          <p className="formula-picker-no-args">
            This function takes no arguments.
          </p>
        ) : (
          <div className="formula-picker-args">
            {doc.args.map((a, i) => (
              <label key={a.name} className="formula-picker-arg">
                <span className="formula-picker-arg-name">
                  {a.name}
                  {a.optional ? (
                    <span className="formula-picker-arg-opt"> (optional)</span>
                  ) : null}
                </span>
                <input
                  className="formula-picker-arg-input"
                  type="text"
                  value={args[i] ?? ""}
                  placeholder={a.placeholder}
                  onChange={(e) => onArgChange(i, e.target.value)}
                  data-testid={`formula-picker-arg-${a.name}`}
                  spellCheck={false}
                />
                <span className="formula-picker-arg-desc">{a.description}</span>
              </label>
            ))}
          </div>
        )}

        <div className="formula-picker-preview" data-testid="formula-picker-preview">
          <div className="formula-picker-preview-row">
            <span className="formula-picker-preview-label">Formula</span>
            <code className="formula-picker-preview-formula">{assembled}</code>
          </div>
          <div className="formula-picker-preview-row">
            <span className="formula-picker-preview-label">Result</span>
            {result.error ? (
              <span
                className="formula-picker-preview-err"
                data-testid="formula-picker-preview-err"
              >
                {result.error}
              </span>
            ) : (
              <span
                className="formula-picker-preview-value"
                data-testid="formula-picker-preview-value"
              >
                {formatPreview(result.value)}
              </span>
            )}
          </div>
        </div>

        {doc.example ? (
          <div className="formula-picker-example">
            <span className="formula-picker-example-label">Example</span>
            <code className="formula-picker-example-code">{doc.example}</code>
            {doc.exampleResult ? (
              <span className="formula-picker-example-result">
                → {doc.exampleResult}
              </span>
            ) : null}
          </div>
        ) : null}
      </div>
      <footer className="formula-picker-footer">
        <button
          type="button"
          className="formula-picker-cancel"
          onClick={onBack}
        >
          Cancel
        </button>
        <button
          type="button"
          className="formula-picker-insert"
          onClick={() => onInsert(assembled)}
          disabled={insertDisabled}
          data-testid="formula-picker-insert"
        >
          Insert
        </button>
      </footer>
    </>
  );
}

function formatPreview(v: unknown): string {
  if (v === null || v === undefined) return "";
  if (typeof v === "number") {
    if (Number.isInteger(v)) return String(v);
    return v.toFixed(Math.abs(v) < 1 ? 4 : 2);
  }
  return String(v);
}
