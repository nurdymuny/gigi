import { useEffect, useMemo, useRef, useState } from "react";
import { findInRows, type FindSpec } from "../lib/find";
import type { BundleSchema, FieldDescriptor, RowMap } from "../lib/gigi-client";
import "./FindModal.css";

type FindMode = "exact" | "canonical";

export interface FindModalProps {
  open: boolean;
  schema: BundleSchema | null;
  rows: RowMap[];
  onClose: () => void;
  /** Called with the row's primary-key string when the user picks a result. */
  onSelectRow: (key: string) => void;
  /**
   * Replace a single occurrence — fired by "Replace" and once per match
   * by "Replace All". Receives `(rowKey, field, newValue)` so the host
   * routes the write through the normal edit path (including undo).
   * If omitted, the Replace UI is hidden.
   */
  onReplace?: (rowKey: string, field: string, value: unknown) => void | Promise<void>;
}

const MAX_RESULTS = 50;

/**
 * Fast keyboard-driven row finder. Substring-matches the query against
 * every field's stringified value (skipping OPAQUE-encrypted fields,
 * which the engine itself doesn't see in plaintext). Enter picks the
 * first result; click picks the row that was clicked; Escape closes.
 */
export function FindModal({
  open,
  schema,
  rows,
  onClose,
  onSelectRow,
  onReplace,
}: FindModalProps) {
  const [q, setQ] = useState("");
  const [replacement, setReplacement] = useState("");
  const [mode, setMode] = useState<FindMode>("exact");
  const [showReplace, setShowReplace] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  // Auto-focus + reset on open.
  useEffect(() => {
    if (open) {
      setQ("");
      setReplacement("");
      setMode("exact");
      setShowReplace(false);
      inputRef.current?.focus();
    }
  }, [open]);

  const keyField = schema?.base_fields[0]?.name ?? "";

  // Pre-compute the searchable field set: every non-OPAQUE field.
  const searchableFields = useMemo<string[]>(() => {
    if (!schema) return [];
    const all = [...schema.base_fields, ...schema.fiber_fields];
    return all
      .filter((f: FieldDescriptor) => f.encryption !== "opaque")
      .map((f) => f.name);
  }, [schema]);

  const matches = useMemo<RowMap[]>(() => {
    const needle = q.trim();
    if (!needle || searchableFields.length === 0) return [];
    const spec: FindSpec =
      mode === "canonical"
        ? { mode: "canonical", query: needle }
        : { mode: "exact", query: needle };
    return findInRows(rows, spec, searchableFields);
  }, [q, mode, rows, searchableFields]);

  const visible = matches.slice(0, MAX_RESULTS);

  if (!open) return null;

  const pick = (row: RowMap) => {
    const key = String(row[keyField] ?? "");
    onSelectRow(key);
    onClose();
  };

  /**
   * Walk every match row and substitute `q` → `replacement` in the
   * first field where the (substring, case-insensitive) match landed.
   * Calls `onReplace(rowKey, field, newValue)` per substitution so the
   * host routes through the normal edit-history path. Replace All
   * processes every match; Replace First processes only the topmost.
   */
  const doReplace = (all: boolean): number => {
    if (!onReplace || !q) return 0;
    const needle = q.toLowerCase();
    const targets = all ? matches : matches.slice(0, 1);
    let n = 0;
    for (const row of targets) {
      const rk = String(row[keyField] ?? "");
      // Find the first non-key field whose stringified value contains
      // the needle (case-insensitive). Replace within that single
      // field — fan-out across all matching fields gets risky fast.
      let chosen: string | null = null;
      for (const f of searchableFields) {
        if (f === keyField) continue;
        const v = row[f];
        if (v == null) continue;
        if (String(v).toLowerCase().includes(needle)) {
          chosen = f;
          break;
        }
      }
      if (!chosen) continue;
      const original = String(row[chosen] ?? "");
      // Case-insensitive global replace of the substring.
      const re = new RegExp(escapeRegex(q), "gi");
      const next = original.replace(re, replacement);
      if (next === original) continue;
      // Fire-and-forget — `onReplace` may return a Promise but we don't
      // await it so all replacements queue in the same tick. The host's
      // edit-history handles ordering + undo coalescing.
      void onReplace(rk, chosen, next);
      n++;
    }
    return n;
  };

  return (
    <div
      className="find-bg"
      data-testid="find-bg"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="find-modal" data-testid="find-modal" role="dialog" aria-label="Find row">
        <div className="find-input-row">
          <SearchIcon />
          <input
            ref={inputRef}
            className="find-input"
            type="text"
            placeholder={
              mode === "canonical"
                ? "Canonical match — ignores spaces, dashes, casing…"
                : "Find in any field (substring match)…"
            }
            value={q}
            onChange={(e) => setQ(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                e.preventDefault();
                onClose();
              } else if (e.key === "Enter" && matches.length > 0) {
                e.preventDefault();
                pick(matches[0]);
              }
            }}
            data-testid="find-input"
            aria-label="Search query"
          />
          {q ? (
            <span className="find-count" data-testid="find-count">
              {matches.length} {matches.length === 1 ? "match" : "matches"}
            </span>
          ) : null}
          <button
            type="button"
            className="find-close"
            onClick={onClose}
            aria-label="Close"
            data-testid="find-close"
          >
            ×
          </button>
        </div>

        {/* Mode toggle. "Canonical" strips spaces/dashes/punctuation before
            matching — the same trick Prism Dedup uses, so `INV-2026-04823`
            finds `INV 2026 04823` and `INV2026/04823`. */}
        <div
          className="find-mode-row"
          role="radiogroup"
          aria-label="Match mode"
        >
          <button
            type="button"
            role="radio"
            aria-checked={mode === "exact"}
            className={`find-mode-btn ${mode === "exact" ? "find-mode-active" : ""}`}
            onClick={() => setMode("exact")}
            data-testid="find-mode-exact"
          >
            Exact
          </button>
          <button
            type="button"
            role="radio"
            aria-checked={mode === "canonical"}
            className={`find-mode-btn ${mode === "canonical" ? "find-mode-active" : ""}`}
            onClick={() => setMode("canonical")}
            data-testid="find-mode-canonical"
            title="Match after stripping spaces, dashes, dots, slashes, underscores — and uppercasing."
          >
            Canonical
          </button>
          {onReplace ? (
            <button
              type="button"
              className={`find-mode-btn find-replace-toggle ${showReplace ? "find-mode-active" : ""}`}
              onClick={() => setShowReplace((v) => !v)}
              data-testid="find-replace-toggle"
              aria-pressed={showReplace}
              title="Show the Replace row"
            >
              Replace
            </button>
          ) : null}
        </div>

        {showReplace && onReplace ? (
          <div className="find-replace-row" data-testid="find-replace-row">
            <input
              className="find-replace-input"
              type="text"
              placeholder="Replace with…"
              value={replacement}
              onChange={(e) => setReplacement(e.target.value)}
              data-testid="find-replace-input"
              aria-label="Replacement text"
            />
            <button
              type="button"
              className="find-replace-btn"
              onClick={() => doReplace(false)}
              disabled={matches.length === 0 || !q}
              data-testid="find-replace-one"
              title="Replace the first match's first matching field"
            >
              Replace
            </button>
            <button
              type="button"
              className="find-replace-btn find-replace-btn-all"
              onClick={() => doReplace(true)}
              disabled={matches.length === 0 || !q}
              data-testid="find-replace-all"
              title={`Replace in every matching row (${matches.length})`}
            >
              Replace all ({matches.length})
            </button>
          </div>
        ) : null}

        <div className="find-body">
          {!q ? (
            <p className="find-empty-hint" data-testid="find-empty-hint">
              Type any value — name, id, status, code — and we'll jump to the
              matching row.{" "}
              {mode === "canonical" ? (
                <>
                  <strong>Canonical</strong> ignores spaces, dashes, and
                  casing, so <code>INV-2026-04823</code> finds{" "}
                  <code>INV 2026 04823</code>.
                </>
              ) : (
                <>OPAQUE-encrypted fields are excluded from the search surface.</>
              )}
            </p>
          ) : matches.length === 0 ? (
            <p className="find-empty-results" data-testid="find-empty-results">
              No rows match <code>{q}</code>.
            </p>
          ) : (
            <ul className="find-results">
              {visible.map((r) => {
                const k = String(r[keyField] ?? "");
                return (
                  <li key={k}>
                    <button
                      type="button"
                      className="find-result"
                      onClick={() => pick(r)}
                      data-testid="find-result"
                      data-row-key={k}
                    >
                      <span className="find-result-key">{k}</span>
                      <span className="find-result-preview">
                        {previewRow(r, searchableFields, keyField, q)}
                      </span>
                    </button>
                  </li>
                );
              })}
              {matches.length > MAX_RESULTS ? (
                <li className="find-more" data-testid="find-more">
                  …and {matches.length - MAX_RESULTS} more. Refine your query.
                </li>
              ) : null}
            </ul>
          )}
        </div>
      </div>
    </div>
  );
}

/** Build a short preview of the row's non-key, non-empty values. */
function previewRow(
  row: RowMap,
  searchable: string[],
  keyField: string,
  q: string,
): string {
  const needle = q.trim().toLowerCase();
  // Prefer fields whose value contains the match — surface them first.
  const sorted = searchable
    .filter((f) => f !== keyField && row[f] != null)
    .sort((a, b) => {
      const ah = String(row[a]).toLowerCase().includes(needle) ? 0 : 1;
      const bh = String(row[b]).toLowerCase().includes(needle) ? 0 : 1;
      return ah - bh;
    })
    .slice(0, 3);
  return sorted.map((f) => `${f}=${row[f]}`).join(" · ");
}

/** Escape regex metacharacters so the user's literal query stays literal. */
function escapeRegex(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function SearchIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="find-search-icon"
      aria-hidden="true"
    >
      <circle cx="11" cy="11" r="7" />
      <path d="m20 20-3.5-3.5" />
    </svg>
  );
}
