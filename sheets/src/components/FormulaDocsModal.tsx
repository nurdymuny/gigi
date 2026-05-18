import { useEffect, useMemo, useRef, useState } from "react";
import {
  CATEGORY_LABELS,
  FORMULA_DOCS,
  searchDocs,
  type FormulaCategory,
  type FormulaDoc,
} from "../lib/formula-docs";
import "./FormulaDocsModal.css";

export interface FormulaDocsModalProps {
  open: boolean;
  onClose: () => void;
}

/**
 * Formula reference — a read-only docs view auto-generated from the
 * same `FORMULA_DOCS` registry that powers the picker. Single source
 * of truth: add a function to the engine + registry, both the picker
 * and this docs page surface it automatically.
 *
 * Open via Help → Formula reference. Searchable + category-filtered;
 * each entry shows the signature, full description, per-argument help,
 * and a worked example with its evaluated result.
 */
export function FormulaDocsModal({ open, onClose }: FormulaDocsModalProps) {
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState<FormulaCategory | "all">("all");
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!open) return;
    setQuery("");
    setCategory("all");
    requestAnimationFrame(() => searchRef.current?.focus());
  }, [open]);

  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  const results = useMemo(() => {
    const all = searchDocs(query);
    if (category === "all") return all;
    return all.filter((d) => d.category === category);
  }, [query, category]);

  const counts = useMemo(() => {
    const m = new Map<FormulaCategory, number>();
    for (const d of FORMULA_DOCS) m.set(d.category, (m.get(d.category) ?? 0) + 1);
    return m;
  }, []);

  if (!open) return null;

  return (
    <div
      className="formula-docs-bg"
      data-testid="formula-docs-bg"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className="formula-docs"
        data-testid="formula-docs"
        role="dialog"
        aria-label="Formula reference"
      >
        <button
          type="button"
          className="formula-docs-close"
          onClick={onClose}
          aria-label="Close"
          data-testid="formula-docs-close"
        >
          ✕
        </button>

        <header className="formula-docs-header">
          <div className="formula-docs-titles">
            <h2>Formula reference</h2>
            <p className="formula-docs-sub">
              Every function the engine supports. {FORMULA_DOCS.length} total
              across 8 categories. Search by name or by what it does.
            </p>
          </div>
          <input
            ref={searchRef}
            className="formula-docs-search"
            type="text"
            placeholder="Search functions…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            data-testid="formula-docs-search"
            spellCheck={false}
          />
        </header>

        <div className="formula-docs-body">
          <aside className="formula-docs-cats">
            <button
              type="button"
              className={`formula-docs-cat ${category === "all" ? "formula-docs-cat-active" : ""}`}
              onClick={() => setCategory("all")}
            >
              <span>All</span>
              <span className="formula-docs-cat-count">{FORMULA_DOCS.length}</span>
            </button>
            {(Object.keys(CATEGORY_LABELS) as FormulaCategory[]).map((c) => (
              <button
                key={c}
                type="button"
                className={`formula-docs-cat ${category === c ? "formula-docs-cat-active" : ""}`}
                onClick={() => setCategory(c)}
              >
                <span>{CATEGORY_LABELS[c]}</span>
                <span className="formula-docs-cat-count">{counts.get(c) ?? 0}</span>
              </button>
            ))}
          </aside>
          <div className="formula-docs-list" data-testid="formula-docs-list">
            {results.length === 0 ? (
              <p className="formula-docs-empty">No functions match "{query}".</p>
            ) : (
              results.map((d) => <DocEntry key={d.name} doc={d} />)
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function DocEntry({ doc }: { doc: FormulaDoc }) {
  return (
    <article className="formula-docs-entry" data-testid={`formula-docs-entry-${doc.name}`}>
      <header className="formula-docs-entry-head">
        <h3 className="formula-docs-entry-name">{doc.name}</h3>
        <span className="formula-docs-entry-cat">
          {CATEGORY_LABELS[doc.category]}
        </span>
      </header>
      <code className="formula-docs-entry-sig">{doc.signature}</code>
      <p className="formula-docs-entry-desc">{doc.description}</p>
      {doc.details ? (
        <p className="formula-docs-entry-details">{doc.details}</p>
      ) : null}
      {doc.args.length > 0 ? (
        <dl className="formula-docs-entry-args">
          {doc.args.map((a) => (
            <div key={a.name} className="formula-docs-entry-arg">
              <dt>
                <code className="formula-docs-arg-name">{a.name}</code>
                {a.optional ? (
                  <span className="formula-docs-arg-opt"> optional</span>
                ) : null}
              </dt>
              <dd>{a.description}</dd>
            </div>
          ))}
        </dl>
      ) : (
        <p className="formula-docs-entry-no-args">No arguments.</p>
      )}
      <div className="formula-docs-entry-example">
        <code className="formula-docs-entry-example-code">{doc.example}</code>
        {doc.exampleResult ? (
          <span className="formula-docs-entry-example-arrow">→</span>
        ) : null}
        {doc.exampleResult ? (
          <code className="formula-docs-entry-example-result">
            {doc.exampleResult}
          </code>
        ) : null}
      </div>
    </article>
  );
}
