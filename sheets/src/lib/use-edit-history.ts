import { useCallback, useRef, useState } from "react";
import type { RowMap } from "./gigi-client";

/**
 * An entry on the undo timeline. Each variant captures the *before*
 * state in enough detail that `undo()` can hand back a single op that
 * fully restores the prior world.
 *
 *   cell    — `before` is the prior cell value
 *   delete  — `row` is the full row we removed (so undo can re-insert)
 *   insert  — `row` is what we inserted (so undo can delete it)
 */
export type EditRecord =
  | { kind?: "cell"; rowKey: string; field: string; before: unknown; after: unknown }
  | { kind: "delete"; rowKey: string; row: RowMap }
  | { kind: "insert"; rowKey: string; row: RowMap };

/**
 * The op the caller must apply to advance the world one step in the
 * undo or redo direction. App.tsx switches on `kind` to route to the
 * right SheetsClient call (update / insert / deleteRow).
 */
export type ApplyOp =
  | { kind: "cell"; rowKey: string; field: string; value: unknown }
  | { kind: "restore"; rowKey: string; row: RowMap }
  | { kind: "delete"; rowKey: string };

export interface EditHistory {
  /** True if there's at least one entry to undo. */
  canUndo: boolean;
  /** True if there's at least one undone entry to redo. */
  canRedo: boolean;
  /** Record a new edit. Wipes any pending redo stack. */
  push: (entry: EditRecord) => void;
  /**
   * Pop the most-recent entry. Returns the op to apply for an undo
   * (cell → write before, delete → restore row, insert → delete row),
   * or null if the stack is empty.
   */
  undo: () => ApplyOp | null;
  /**
   * Pop the most-recent undo. Returns the op to apply for a redo
   * (cell → write after, delete → delete again, insert → restore),
   * or null if there's nothing to redo.
   */
  redo: () => ApplyOp | null;
  /** Reset both stacks (e.g. on bundle change). */
  clear: () => void;
}

/** Internal normalized shape — every stored entry has an explicit `kind`. */
type Entry =
  | { kind: "cell"; rowKey: string; field: string; before: unknown; after: unknown }
  | { kind: "delete"; rowKey: string; row: RowMap }
  | { kind: "insert"; rowKey: string; row: RowMap };

function normalize(entry: EditRecord): Entry {
  if (entry.kind === "delete" || entry.kind === "insert") return entry;
  // Default kind is "cell" — keeps the legacy push({rowKey, field, before, after}) shape working.
  return {
    kind: "cell",
    rowKey: entry.rowKey,
    field: entry.field,
    before: entry.before,
    after: entry.after,
  };
}

/** Op that undoes this entry. */
function inverseOf(e: Entry): ApplyOp {
  switch (e.kind) {
    case "cell":
      return { kind: "cell", rowKey: e.rowKey, field: e.field, value: e.before };
    case "delete":
      return { kind: "restore", rowKey: e.rowKey, row: e.row };
    case "insert":
      return { kind: "delete", rowKey: e.rowKey };
  }
}

/** Op that re-applies this entry forward (redo). */
function forwardOf(e: Entry): ApplyOp {
  switch (e.kind) {
    case "cell":
      return { kind: "cell", rowKey: e.rowKey, field: e.field, value: e.after };
    case "delete":
      return { kind: "delete", rowKey: e.rowKey };
    case "insert":
      return { kind: "restore", rowKey: e.rowKey, row: e.row };
  }
}

/**
 * In-memory undo/redo stack for cell edits, row deletes, and row
 * inserts. The hook only manages the history — the caller is
 * responsible for actually applying the inverse op (writing the cell
 * back, re-inserting the deleted row, etc.) when undo/redo fires.
 * Capacity bounded so the memory footprint stays small.
 */
export function useEditHistory(maxSize: number = 50): EditHistory {
  const undoStack = useRef<Entry[]>([]);
  const redoStack = useRef<Entry[]>([]);
  const [, force] = useState(0);
  const ping = useCallback(() => force((n) => n + 1), []);

  const push = useCallback(
    (entry: EditRecord) => {
      undoStack.current.push(normalize(entry));
      if (undoStack.current.length > maxSize) {
        undoStack.current.shift(); // evict oldest
      }
      // Any new edit invalidates the redo timeline.
      redoStack.current = [];
      ping();
    },
    [maxSize, ping],
  );

  const undo = useCallback((): ApplyOp | null => {
    const top = undoStack.current.pop();
    if (!top) return null;
    redoStack.current.push(top);
    ping();
    return inverseOf(top);
  }, [ping]);

  const redo = useCallback((): ApplyOp | null => {
    const top = redoStack.current.pop();
    if (!top) return null;
    undoStack.current.push(top);
    ping();
    return forwardOf(top);
  }, [ping]);

  const clear = useCallback(() => {
    undoStack.current = [];
    redoStack.current = [];
    ping();
  }, [ping]);

  return {
    canUndo: undoStack.current.length > 0,
    canRedo: redoStack.current.length > 0,
    push,
    undo,
    redo,
    clear,
  };
}
