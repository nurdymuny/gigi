import { useEffect, useState } from "react";
import { lookupTerm, type GlossaryEntry } from "../lib/geometry-glossary";
import "./TermInfo.css";

export interface TermInfoProps {
  /** Glossary key — case-insensitive. */
  term: string;
  /** Optional aria-label; defaults to "About <term>". */
  label?: string;
  /** Optional className passed onto the button (positioning). */
  className?: string;
}

/**
 * A tiny (ⓘ) icon button that opens a modal explaining a geometric term
 * in plain English. Used inline next to verb buttons and dropdowns so
 * the user can ask "wait, what does this mean?" without leaving the app.
 */
export function TermInfo({ term, label, className }: TermInfoProps) {
  const entry = lookupTerm(term);
  const [open, setOpen] = useState(false);

  // Escape closes the modal.
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open]);

  if (!entry) return null;

  return (
    <>
      <button
        type="button"
        className={`term-info-btn ${className ?? ""}`}
        aria-label={label ?? `About ${entry.title}`}
        title={entry.summary}
        data-testid={`term-info-${term.toLowerCase()}`}
        onClick={(e) => {
          // Don't bubble — these icons live inside clickable rows/labels.
          e.preventDefault();
          e.stopPropagation();
          setOpen(true);
        }}
      >
        <svg
          width="13"
          height="13"
          viewBox="0 0 16 16"
          fill="none"
          aria-hidden="true"
        >
          <circle cx="8" cy="8" r="7" stroke="currentColor" strokeWidth="1.4" />
          <circle cx="8" cy="4.5" r="0.9" fill="currentColor" />
          <path
            d="M8 7v5"
            stroke="currentColor"
            strokeWidth="1.4"
            strokeLinecap="round"
          />
        </svg>
      </button>
      {open ? (
        <TermInfoModal entry={entry} term={term} onClose={() => setOpen(false)} />
      ) : null}
    </>
  );
}

function TermInfoModal({
  entry,
  term,
  onClose,
}: {
  entry: GlossaryEntry;
  term: string;
  onClose: () => void;
}) {
  return (
    <div
      className="term-info-bg"
      data-testid="term-info-bg"
      onClick={onClose}
      role="presentation"
    >
      <div
        className="term-info-modal"
        data-testid={`term-info-modal-${term.toLowerCase()}`}
        role="dialog"
        aria-modal="true"
        aria-labelledby={`term-info-title-${term.toLowerCase()}`}
        onClick={(e) => e.stopPropagation()}
      >
        <header className="term-info-head">
          <div>
            <h3 id={`term-info-title-${term.toLowerCase()}`} className="term-info-title">
              {entry.title}
            </h3>
            <p className="term-info-summary">{entry.summary}</p>
          </div>
          <button
            type="button"
            className="term-info-close"
            aria-label="Close"
            data-testid="term-info-close"
            onClick={onClose}
          >
            ×
          </button>
        </header>
        <div className="term-info-body">
          {entry.body.map((p, i) => (
            <p key={i}>{p}</p>
          ))}
          <div className="term-info-example">
            <h4>Example</h4>
            <p>
              <span className="term-info-example-tag">Setup</span> {entry.example.setup}
            </p>
            <p>
              <span className="term-info-example-tag">Result</span> {entry.example.result}
            </p>
          </div>
          {entry.gql ? (
            <div className="term-info-gql">
              <h4>GQL</h4>
              <pre>
                <code>{entry.gql}</code>
              </pre>
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}
