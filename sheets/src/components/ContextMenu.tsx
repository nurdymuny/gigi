import { useEffect, useRef } from "react";
import "./ContextMenu.css";

export interface ContextMenuItem {
  /** Stable id (used as testid suffix). */
  id: string;
  label: string;
  /** Optional shortcut hint shown right-aligned. */
  shortcut?: string;
  /** Section divider above this item. */
  separator?: boolean;
  disabled?: boolean;
  onSelect: () => void;
}

export interface ContextMenuProps {
  open: boolean;
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
  /** Optional header line (e.g., the row key being acted on). */
  header?: string;
}

const MENU_W = 240;
const MENU_H_ESTIMATE = 320;

export function ContextMenu({
  open,
  x,
  y,
  items,
  onClose,
  header,
}: ContextMenuProps) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    function onDocClick(e: MouseEvent) {
      if (!ref.current?.contains(e.target as Node)) onClose();
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    // Wait one tick so the click that opened the menu doesn't immediately close it.
    const t = setTimeout(() => {
      document.addEventListener("mousedown", onDocClick);
      document.addEventListener("contextmenu", onDocClick);
    }, 0);
    document.addEventListener("keydown", onKey);
    return () => {
      clearTimeout(t);
      document.removeEventListener("mousedown", onDocClick);
      document.removeEventListener("contextmenu", onDocClick);
      document.removeEventListener("keydown", onKey);
    };
  }, [open, onClose]);

  if (!open) return null;

  // Flip if near viewport edge.
  const vw = typeof window !== "undefined" ? window.innerWidth : 1024;
  const vh = typeof window !== "undefined" ? window.innerHeight : 768;
  const left = x + MENU_W > vw ? Math.max(4, x - MENU_W) : x;
  const top = y + MENU_H_ESTIMATE > vh ? Math.max(4, y - MENU_H_ESTIMATE) : y;

  return (
    <div
      ref={ref}
      className="context-menu"
      role="menu"
      data-testid="context-menu"
      style={{ left, top }}
      onContextMenu={(e) => e.preventDefault()}
    >
      {header ? (
        <div className="context-menu-header" data-testid="context-menu-header">
          {header}
        </div>
      ) : null}
      <ul className="context-menu-list">
        {items.map((it) => (
          <li key={it.id}>
            {it.separator ? <div className="context-menu-sep" /> : null}
            <button
              type="button"
              role="menuitem"
              className="context-menu-item"
              data-testid={`context-menu-${it.id}`}
              disabled={it.disabled}
              onClick={() => {
                if (it.disabled) return;
                it.onSelect();
                onClose();
              }}
            >
              <span className="context-menu-label">{it.label}</span>
              {it.shortcut ? (
                <span className="context-menu-shortcut">{it.shortcut}</span>
              ) : null}
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
