import { useEffect } from "react";

/**
 * A document-level keyboard shortcut binding.
 *
 * By default, shortcuts do NOT fire when focus is inside an <input>,
 * <textarea>, <select>, or contenteditable element — otherwise typing in
 * the search box would trigger commands. Set `allowInInput: true` for
 * shortcuts that should always fire (e.g. Escape to close a modal even
 * when its input has focus).
 */
export interface ShortcutBinding {
  /** Single character or named key (e.g. "f", "Escape", "/"). Letters match case-insensitively. */
  key: string;
  /** Require ⌘ (macOS) or Ctrl (Windows/Linux). */
  meta?: boolean;
  /** Require Shift. */
  shift?: boolean;
  /** Require Alt/Option. */
  alt?: boolean;
  /** Call e.preventDefault() before invoking the handler. Default false. */
  preventDefault?: boolean;
  /** Fire even if focus is inside an input/textarea/contenteditable. Default false. */
  allowInInput?: boolean;
  /** What to do when the combo fires. */
  handler: (e: KeyboardEvent) => void;
}

function focusIsInInput(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
  if (target.isContentEditable) return true;
  return false;
}

function matches(b: ShortcutBinding, e: KeyboardEvent): boolean {
  const eKey = e.key.length === 1 ? e.key.toLowerCase() : e.key;
  const bKey = b.key.length === 1 ? b.key.toLowerCase() : b.key;
  if (eKey !== bKey) return false;
  const meta = Boolean(e.metaKey || e.ctrlKey);
  if (Boolean(b.meta) !== meta) return false;
  if (Boolean(b.shift) !== e.shiftKey) return false;
  if (Boolean(b.alt) !== e.altKey) return false;
  return true;
}

/**
 * Install document-level shortcut handlers. The hook re-installs the
 * listener whenever the bindings array identity changes — for stable
 * behavior, memoize the array if it lives inside a re-rendering component.
 */
export function useGlobalShortcuts(bindings: ShortcutBinding[]): void {
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      for (const b of bindings) {
        if (!matches(b, e)) continue;
        if (!b.allowInInput && focusIsInInput(e.target)) continue;
        if (b.preventDefault) e.preventDefault();
        b.handler(e);
        return; // first match wins
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [bindings]);
}
