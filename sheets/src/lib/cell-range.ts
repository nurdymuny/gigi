/**
 * Range-selection geometry — the cell-rectangle model for drag-select,
 * copy/paste, drag-fill, and range-stats in the formula bar.
 *
 * A `CellRange` is encoded as (anchor cell, focus cell). The anchor is
 * the cell where the drag/click started; the focus is the current
 * mouse cell. The normalized "top-left to bottom-right" bbox is
 * derived in the helpers — callers don't have to think about whether
 * the user dragged up-left or down-right.
 */

export interface CellRange {
  anchorRowKey: string;
  anchorField: string;
  focusRowKey: string;
  focusField: string;
}

export interface CellAddr {
  rowKey: string;
  field: string;
}

/**
 * Normalize a range against the *visible* row + column order. Returns
 * the inclusive top-left / bottom-right indices, or `null` if either
 * endpoint isn't visible (e.g. the row got filtered out under the user).
 */
function normalize(
  range: CellRange,
  rowOrder: string[],
  fieldOrder: string[],
): { r1: number; r2: number; c1: number; c2: number } | null {
  const ar = rowOrder.indexOf(range.anchorRowKey);
  const fr = rowOrder.indexOf(range.focusRowKey);
  const ac = fieldOrder.indexOf(range.anchorField);
  const fc = fieldOrder.indexOf(range.focusField);
  if (ar < 0 || fr < 0 || ac < 0 || fc < 0) return null;
  return {
    r1: Math.min(ar, fr),
    r2: Math.max(ar, fr),
    c1: Math.min(ac, fc),
    c2: Math.max(ac, fc),
  };
}

/**
 * True if `(rowKey, field)` falls within the normalized bbox of
 * `range`. Out-of-order or out-of-view inputs return false rather
 * than throwing — the grid renders many cells per frame and a thrown
 * error would unmount the whole view.
 */
export function isCellInRange(
  range: CellRange | null,
  rowKey: string,
  field: string,
  rowOrder: string[],
  fieldOrder: string[],
): boolean {
  if (!range) return false;
  const bbox = normalize(range, rowOrder, fieldOrder);
  if (!bbox) return false;
  const r = rowOrder.indexOf(rowKey);
  const c = fieldOrder.indexOf(field);
  if (r < 0 || c < 0) return false;
  return r >= bbox.r1 && r <= bbox.r2 && c >= bbox.c1 && c <= bbox.c2;
}

/**
 * Enumerate every `(rowKey, field)` pair in the range, in row-major
 * order (rows top-to-bottom, cells left-to-right within each row).
 * Used by copy, drag-fill, range-stats, conditional-formatting.
 */
export function cellsInRange(
  range: CellRange | null,
  rowOrder: string[],
  fieldOrder: string[],
): CellAddr[] {
  if (!range) return [];
  const bbox = normalize(range, rowOrder, fieldOrder);
  if (!bbox) return [];
  const out: CellAddr[] = [];
  for (let r = bbox.r1; r <= bbox.r2; r++) {
    for (let c = bbox.c1; c <= bbox.c2; c++) {
      out.push({ rowKey: rowOrder[r], field: fieldOrder[c] });
    }
  }
  return out;
}

/** Just the row-key axis of the range, top-to-bottom. */
export function rangeRowKeys(range: CellRange | null, rowOrder: string[]): string[] {
  if (!range) return [];
  const bbox = normalize(range, rowOrder, []);
  // normalize() returns null when fieldOrder is empty — so re-derive
  // just the row half here without involving the field axis.
  const ar = rowOrder.indexOf(range.anchorRowKey);
  const fr = rowOrder.indexOf(range.focusRowKey);
  if (ar < 0 || fr < 0) return [];
  const [r1, r2] = ar <= fr ? [ar, fr] : [fr, ar];
  void bbox;
  return rowOrder.slice(r1, r2 + 1);
}

/** Just the field-key axis of the range, left-to-right. */
export function rangeFields(range: CellRange | null, fieldOrder: string[]): string[] {
  if (!range) return [];
  const ac = fieldOrder.indexOf(range.anchorField);
  const fc = fieldOrder.indexOf(range.focusField);
  if (ac < 0 || fc < 0) return [];
  const [c1, c2] = ac <= fc ? [ac, fc] : [fc, ac];
  return fieldOrder.slice(c1, c2 + 1);
}
