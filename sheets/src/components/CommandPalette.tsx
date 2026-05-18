import { useEffect, useMemo, useRef, useState } from "react";
import "./CommandPalette.css";

export interface Command {
  /** Stable id, used as React key + data-testid. */
  id: string;
  /** Section header (e.g. "Views", "File", "Bundles"). */
  section: string;
  /** Human-friendly label shown in the list and matched against the query. */
  label: string;
  /** Optional shortcut hint (e.g. "⌘⇧S"). */
  shortcut?: string;
  /** Optional secondary description shown after the label. */
  hint?: string;
  /** Fired when the user picks this command. */
  run: () => void;
}

export interface CommandPaletteProps {
  open: boolean;
  commands: Command[];
  onClose: () => void;
}

/**
 * ⌘K / Ctrl+K Spotlight-style command palette. Combines bundles + saved
 * views + menu actions into a single unified search surface. Pick a row
 * with the keyboard, click, or hit Enter for the first match.
 *
 * The parent assembles the `commands` list from whatever sources make
 * sense at the moment (engine bundles, saved views, dispatchMenu ids).
 */
export function CommandPalette({ open, commands, onClose }: CommandPaletteProps) {
  const [q, setQ] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (open) {
      setQ("");
      inputRef.current?.focus();
    }
  }, [open]);

  const filtered = useMemo(() => {
    const needle = q.trim().toLowerCase();
    if (!needle) return commands;
    return commands.filter((c) =>
      c.label.toLowerCase().includes(needle) ||
      (c.hint ? c.hint.toLowerCase().includes(needle) : false),
    );
  }, [q, commands]);

  // Build {section: Command[]} preserving original insertion order.
  const grouped = useMemo(() => {
    const m = new Map<string, Command[]>();
    for (const c of filtered) {
      const arr = m.get(c.section);
      if (arr) arr.push(c);
      else m.set(c.section, [c]);
    }
    return m;
  }, [filtered]);

  if (!open) return null;

  const pick = (cmd: Command) => {
    cmd.run();
    onClose();
  };

  return (
    <div
      className="cmd-bg"
      data-testid="command-bg"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className="cmd-modal"
        data-testid="command-palette"
        role="dialog"
        aria-label="Command palette"
      >
        <div className="cmd-input-row">
          <span className="cmd-prompt" aria-hidden="true">⌘</span>
          <input
            ref={inputRef}
            type="text"
            placeholder="Type a command, bundle name, or saved view…"
            value={q}
            onChange={(e) => setQ(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                e.preventDefault();
                onClose();
              } else if (e.key === "Enter" && filtered.length > 0) {
                e.preventDefault();
                pick(filtered[0]);
              }
            }}
            className="cmd-input"
            data-testid="command-input"
            aria-label="Command query"
          />
        </div>
        <div className="cmd-body">
          {filtered.length === 0 ? (
            <p className="cmd-empty" data-testid="command-empty">
              No commands match <code>{q}</code>.
            </p>
          ) : (
            Array.from(grouped.entries()).map(([section, items]) => (
              <section
                key={section}
                className="cmd-section"
                data-testid={`command-section-${section}`}
              >
                <h5 className="cmd-section-head">{section}</h5>
                <ul className="cmd-list">
                  {items.map((c) => (
                    <li key={c.id}>
                      <button
                        type="button"
                        className="cmd-item"
                        onClick={() => pick(c)}
                        data-testid={`command-item-${c.id}`}
                      >
                        <span className="cmd-item-label">{c.label}</span>
                        {c.hint ? (
                          <span className="cmd-item-hint">{c.hint}</span>
                        ) : null}
                        {c.shortcut ? (
                          <kbd className="cmd-item-shortcut">{c.shortcut}</kbd>
                        ) : null}
                      </button>
                    </li>
                  ))}
                </ul>
              </section>
            ))
          )}
        </div>
        <footer className="cmd-foot">
          <kbd>↵</kbd> select · <kbd>Esc</kbd> close · <kbd>⌘K</kbd> reopen
        </footer>
      </div>
    </div>
  );
}
