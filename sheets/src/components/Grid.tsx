import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type {
  BundleSchema,
  FieldDescriptor,
  RowMap,
  SheetsClientError,
} from "../lib/gigi-client";
import { type CellRange, isCellInRange } from "../lib/cell-range";
import { asError } from "../lib/formula";
import { defaultFormatFor, formatValue } from "../lib/format";
import { kappaClass, type KappaClass } from "../lib/kappa";
import "./Grid.css";

export interface RowClickModifiers {
  meta: boolean;
  shift: boolean;
  alt: boolean;
}

/**
 * Excel-style column letter for the index. 0 → "A", 25 → "Z",
 * 26 → "AA", 27 → "AB", …, 701 → "ZZ". Anything beyond two letters
 * keeps extending (e.g. 702 → "AAA") so the helper never returns
 * the empty string for a sane index.
 */
export function indexToLetter(index: number): string {
  let n = Math.max(0, Math.floor(index));
  let out = "";
  while (true) {
    out = String.fromCharCode(65 + (n % 26)) + out;
    if (n < 26) break;
    n = Math.floor(n / 26) - 1;
  }
  return out;
}

export interface GridEmptyActions {
  onInsertRow?: () => void;
  onImportCsv?: () => void;
  onOpenSchema?: () => void;
}

export interface GridProps {
  schema: BundleSchema | null;
  rows: RowMap[];
  loading: boolean;
  error?: SheetsClientError | null;
  /** Per-row κ keyed by base_fields[0]. Empty map → gutter shows "—". */
  kappaMap?: Map<string, number>;
  /**
   * Fiber-field names that should not render in the grid. The primary
   * key (base_fields[0]) is never hidden even if listed here.
   */
  hiddenFields?: Set<string>;
  /** Currently-focused row key (drives the inspector). */
  selectedRowKey?: string | null;
  /**
   * Multi-selection set (visual highlight). When omitted, falls back to
   * `{selectedRowKey}` so old callers keep working.
   */
  selectedKeys?: Set<string>;
  /**
   * Called on plain click — the legacy single-select callback. If
   * `onRowClick` is also passed, this one is ignored.
   */
  onRowSelect?: (rowKey: string) => void;
  /**
   * Fine-grained click handler with modifier keys. App decides what
   * Ctrl/Shift mean. If both this and onRowSelect are passed, this wins.
   */
  onRowClick?: (rowKey: string, mods: RowClickModifiers) => void;
  /**
   * Right-click handler. Called with the row key and the viewport
   * coordinates of the click. Should typically open a context menu.
   */
  onRowContextMenu?: (rowKey: string, x: number, y: number) => void;
  /**
   * Called when the user commits an inline edit (Enter or blur).
   * The Grid stays optimistic — it does not wait for this to resolve before
   * clearing the editor state.
   */
  onCellEdit?: (rowKey: string, field: string, value: unknown) => void;
  /**
   * Called when a cell's inline editor opens. The parent can use this to
   * sync the formula bar / inspector / context to the active cell — the
   * grid itself doesn't read it. Fires once per click-to-edit; no fire
   * when the editor closes.
   */
  onCellFocus?: (rowKey: string, field: string) => void;
  /** Actions surfaced from the empty state when rows.length === 0. */
  emptyActions?: GridEmptyActions;
  /**
   * The currently-selected column name. Cells in that column get the
   * `grid-cell-col-selected` class; the letter cell at the top shows
   * the active state. Pass `null` to clear.
   */
  selectedColumn?: string | null;
  /**
   * Clicking a column letter calls this. Pass null to deselect. The
   * grid does not maintain its own column-selection state — caller
   * owns it (parallel to row selection).
   */
  onColumnSelect?: (column: string | null) => void;
  /**
   * Right-clicking a column letter opens the per-column context menu.
   * Called with the column name + viewport coords.
   */
  onColumnContextMenu?: (column: string, x: number, y: number) => void;
  /**
   * Sidecar formula lookup. When this returns a non-null string for a
   * cell, the cell is treated as a formula: it still displays the
   * evaluated value (which is already in `rows[i][field]`), but gets
   * `data-has-formula="true"` for styling, and clicking it opens the
   * editor on the *formula text* rather than the displayed result.
   *
   * The grid never evaluates formulas itself. On commit it passes the
   * raw `=…` string up via `onCellEdit`; the parent is responsible for
   * routing it through the formula engine and updating the bundle row
   * + sidecar.
   */
  getFormulaText?: (rowKey: string, field: string) => string | null;
  /**
   * Cell-level range (Excel "marching ants" rectangle). The grid
   * highlights every cell inside the range; mouse-drag updates it.
   * Mouse-down starts a new range anchored at the clicked cell;
   * mouse-move while held updates the focus; mouse-up ends the drag.
   * Click-without-drag clears the range and opens the cell editor.
   */
  cellRange?: CellRange | null;
  onCellRangeChange?: (range: CellRange | null) => void;
  /**
   * Excel-style drag-fill — called when the user drags the fill handle
   * from the bottom-right of the active range. Receives the source
   * range, the target cell where the drag ended, and the visible-axis
   * lookup tables (so the host can compute the extension axis). The
   * host writes each filled cell through its normal edit path.
   */
  onDragFill?: (params: {
    source: CellRange;
    target: { rowKey: string; field: string };
    rowOrder: string[];
    fieldOrder: string[];
  }) => void;
  /**
   * Names of columns that currently have an active filter — the grid
   * renders a small "active" dot on the funnel icon of each. The Grid
   * itself doesn't own the filter state; the host (App.tsx) does.
   */
  activeFilterColumns?: Set<string>;
  /**
   * Clicked the funnel icon in a column header. Receives the field
   * name + the icon's DOM element so the host can position a popover.
   */
  onColumnFilterClick?: (field: string, anchorEl: HTMLElement) => void;
  /**
   * Per-column conditional-format rules. For each rule, cells in that
   * column whose row's κ ≥ `kappaThreshold` get a colored background.
   * Map keys are field names; missing keys → no rule.
   */
  conditionalFormats?: Map<
    string,
    { kappaThreshold: number; color: "red" | "amber" | "green" | "blue" | "purple" }
  >;
}

const ROW_HEIGHT = 34;

export function Grid({
  schema,
  rows,
  loading,
  error,
  kappaMap,
  hiddenFields,
  selectedRowKey,
  selectedKeys,
  onRowSelect,
  onRowClick,
  onRowContextMenu,
  onCellEdit,
  onCellFocus,
  emptyActions,
  selectedColumn,
  onColumnSelect,
  onColumnContextMenu,
  getFormulaText,
  cellRange,
  onCellRangeChange,
  onDragFill,
  activeFilterColumns,
  onColumnFilterClick,
  conditionalFormats,
}: GridProps) {
  const keyField = schema?.base_fields[0]?.name;
  const columns = useMemo<FieldDescriptor[]>(() => {
    if (!schema) return [];
    const all = [...schema.base_fields, ...schema.fiber_fields];
    if (!hiddenFields || hiddenFields.size === 0) return all;
    return all.filter((f) => f.name === keyField || !hiddenFields.has(f.name));
  }, [schema, hiddenFields, keyField]);

  if (error) {
    return (
      <div className="grid-error" role="alert" data-testid="grid-error">
        <strong>Couldn't load bundle.</strong>
        <p>{error.message}</p>
        <small>
          code: {error.code}
          {error.status ? ` · status ${error.status}` : ""}
        </small>
      </div>
    );
  }

  if (loading || !schema) {
    return <Skeleton />;
  }

  // Derive effective selection set — old callers passing just selectedRowKey
  // get a one-element set so highlighting still works.
  const effectiveSelected =
    selectedKeys ??
    (selectedRowKey ? new Set<string>([selectedRowKey]) : new Set<string>());

  return (
    <Body
      columns={columns}
      rows={rows}
      keyField={keyField}
      bundleName={schema.name}
      kappaMap={kappaMap}
      selectedRowKey={selectedRowKey ?? null}
      selectedKeys={effectiveSelected}
      onRowSelect={onRowSelect}
      onRowClick={onRowClick}
      onRowContextMenu={onRowContextMenu}
      onCellEdit={onCellEdit}
      emptyActions={emptyActions}
      schemaRecords={schema.records}
      selectedColumn={selectedColumn ?? null}
      onColumnSelect={onColumnSelect}
      onColumnContextMenu={onColumnContextMenu}
      getFormulaText={getFormulaText}
      onCellFocus={onCellFocus}
      cellRange={cellRange ?? null}
      onCellRangeChange={onCellRangeChange}
      onDragFill={onDragFill}
      activeFilterColumns={activeFilterColumns}
      onColumnFilterClick={onColumnFilterClick}
      conditionalFormats={conditionalFormats}
    />
  );
}

interface BodyProps {
  columns: FieldDescriptor[];
  rows: RowMap[];
  keyField: string | undefined;
  bundleName: string;
  kappaMap?: Map<string, number>;
  selectedRowKey: string | null;
  selectedKeys: Set<string>;
  onRowSelect?: GridProps["onRowSelect"];
  onRowClick?: GridProps["onRowClick"];
  onRowContextMenu?: GridProps["onRowContextMenu"];
  onCellEdit?: GridProps["onCellEdit"];
  emptyActions?: GridEmptyActions;
  schemaRecords: number;
  selectedColumn: string | null;
  onColumnSelect?: GridProps["onColumnSelect"];
  onColumnContextMenu?: GridProps["onColumnContextMenu"];
  getFormulaText?: GridProps["getFormulaText"];
  onCellFocus?: GridProps["onCellFocus"];
  cellRange?: CellRange | null;
  onCellRangeChange?: GridProps["onCellRangeChange"];
  onDragFill?: GridProps["onDragFill"];
  activeFilterColumns?: Set<string>;
  onColumnFilterClick?: GridProps["onColumnFilterClick"];
  conditionalFormats?: GridProps["conditionalFormats"];
}

interface EditingTarget {
  rowKey: string;
  field: string;
}

type SortDir = "asc" | "desc";
interface SortSpec {
  field: string;
  dir: SortDir;
}

function compareValues(a: unknown, b: unknown): number {
  const aNull = a === null || a === undefined || a === "";
  const bNull = b === null || b === undefined || b === "";
  if (aNull && bNull) return 0;
  if (aNull) return 1; // nulls always sink to the bottom
  if (bNull) return -1;
  if (typeof a === "number" && typeof b === "number") return a - b;
  if (typeof a === "boolean" && typeof b === "boolean") {
    return a === b ? 0 : a ? 1 : -1;
  }
  return String(a).localeCompare(String(b), undefined, {
    numeric: true,
    sensitivity: "base",
  });
}

function isSortable(field: FieldDescriptor): boolean {
  // OPAQUE columns are masked — sorting them would leak ordering signal.
  if (field.encryption === "opaque") return false;
  return true;
}

function Body({
  columns,
  rows,
  keyField,
  bundleName,
  kappaMap,
  selectedRowKey,
  selectedKeys,
  onRowSelect,
  onRowClick,
  onRowContextMenu,
  onCellEdit,
  emptyActions,
  schemaRecords,
  selectedColumn,
  onColumnSelect,
  onColumnContextMenu,
  getFormulaText,
  onCellFocus,
  cellRange,
  onCellRangeChange,
  onDragFill,
  activeFilterColumns,
  onColumnFilterClick,
  conditionalFormats,
}: BodyProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  // Detect print mode so the virtualizer is bypassed for the print job
  // (otherwise the printed page would only contain the visible window).
  const [printing, setPrinting] = useState(false);
  useEffect(() => {
    const before = () => setPrinting(true);
    const after = () => setPrinting(false);
    window.addEventListener("beforeprint", before);
    window.addEventListener("afterprint", after);
    return () => {
      window.removeEventListener("beforeprint", before);
      window.removeEventListener("afterprint", after);
    };
  }, []);

  const [editing, setEditing] = useState<EditingTarget | null>(null);
  const [sort, setSort] = useState<SortSpec | null>(null);

  // Range-select drag state. `dragAnchor` is the cell where the
  // mousedown landed; `dragMoved` flips to true the first time the
  // user enters a different cell, which suppresses the click-to-edit
  // that would otherwise fire on mouseup. Both live in refs so they
  // don't cause re-renders.
  const dragAnchor = useRef<{ rowKey: string; field: string } | null>(null);
  const dragMoved = useRef<boolean>(false);
  // Document-level mouseup so the drag ends even when the user
  // releases outside any cell (or outside the grid entirely).
  useEffect(() => {
    function onUp() {
      dragAnchor.current = null;
      // `dragMoved` stays true through the click-suppression check; the
      // cell's onClick reads + clears it.
    }
    document.addEventListener("mouseup", onUp);
    return () => document.removeEventListener("mouseup", onUp);
  }, []);

  const beginDrag = useCallback(
    (rowKey: string, field: string, shiftKey: boolean) => {
      if (!onCellRangeChange) return;
      if (shiftKey && cellRange) {
        // Shift+click extends the existing range from its anchor to
        // the clicked cell — Excel "click + Shift+click" idiom.
        dragAnchor.current = {
          rowKey: cellRange.anchorRowKey,
          field: cellRange.anchorField,
        };
        dragMoved.current = true; // treat as a range, not a click
        onCellRangeChange({
          anchorRowKey: cellRange.anchorRowKey,
          anchorField: cellRange.anchorField,
          focusRowKey: rowKey,
          focusField: field,
        });
        return;
      }
      dragAnchor.current = { rowKey, field };
      dragMoved.current = false;
      // Single-cell range to start; mouseenter on other cells extends.
      onCellRangeChange({
        anchorRowKey: rowKey,
        anchorField: field,
        focusRowKey: rowKey,
        focusField: field,
      });
    },
    [cellRange, onCellRangeChange],
  );

  const extendDrag = useCallback(
    (rowKey: string, field: string) => {
      const anchor = dragAnchor.current;
      if (!anchor || !onCellRangeChange) return;
      if (anchor.rowKey === rowKey && anchor.field === field) return;
      dragMoved.current = true;
      onCellRangeChange({
        anchorRowKey: anchor.rowKey,
        anchorField: anchor.field,
        focusRowKey: rowKey,
        focusField: field,
      });
    },
    [onCellRangeChange],
  );

  /**
   * Returns true if a click on (rowKey, field) should be **suppressed**
   * because the user just finished a drag. Consumes the flag so the
   * next click is treated normally.
   */
  const consumeDragSuppress = useCallback((): boolean => {
    if (dragMoved.current) {
      dragMoved.current = false;
      return true;
    }
    return false;
  }, []);

  // Field order — used for in-range detection in cells (visible-order
  // is what range geometry resolves against).
  const fieldOrder = useMemo(() => columns.map((c) => c.name), [columns]);

  // Ref to the latest sortedRows so fill-state callbacks (defined
  // here, before sortedRows is in scope) read the freshest list
  // without a TDZ.
  const sortedRowsRef = useRef<RowMap[]>([]);

  // Drag-fill state. `fillSource` is the source range snapshotted at
  // mousedown on the fill handle; `fillTarget` updates as the user
  // drags over cells; on mouseup we call `onDragFill` and clear.
  const [fillTarget, setFillTarget] = useState<{ rowKey: string; field: string } | null>(null);
  const fillSourceRef = useRef<CellRange | null>(null);
  const beginFill = useCallback(
    (e: React.MouseEvent) => {
      if (!cellRange || !onDragFill) return;
      e.preventDefault();
      e.stopPropagation();
      fillSourceRef.current = cellRange;
      setFillTarget(null);
    },
    [cellRange, onDragFill],
  );
  const extendFill = useCallback(
    (rowKey: string, field: string) => {
      if (!fillSourceRef.current) return;
      setFillTarget((prev) =>
        prev && prev.rowKey === rowKey && prev.field === field
          ? prev
          : { rowKey, field },
      );
    },
    [],
  );
  useEffect(() => {
    function onUp() {
      const src = fillSourceRef.current;
      const tgt = fillTarget;
      fillSourceRef.current = null;
      setFillTarget(null);
      if (src && tgt && onDragFill && keyField) {
        const rowOrder = sortedRowsRef.current.map((sr) => String(sr[keyField] ?? ""));
        onDragFill({ source: src, target: tgt, rowOrder, fieldOrder });
      }
    }
    document.addEventListener("mouseup", onUp);
    return () => document.removeEventListener("mouseup", onUp);
  }, [fillTarget, onDragFill, keyField, fieldOrder]);

  /**
   * User-overridden column widths from the drag-to-resize handles.
   * Keyed by field name; when a column isn't here, it falls back to a
   * sensible default based on its type.
   */
  const [colWidths, setColWidths] = useState<Map<string, number>>(new Map());

  // Cycle sort on a column: none → asc → desc → none. Clicking a different
  // column resets to asc on that column (Excel/Airtable behavior).
  const cycleSort = (field: string) => {
    setSort((prev) => {
      if (!prev || prev.field !== field) return { field, dir: "asc" };
      if (prev.dir === "asc") return { field, dir: "desc" };
      return null;
    });
  };

  const sortedRows = useMemo(() => {
    if (!sort) return rows;
    const dir = sort.dir === "asc" ? 1 : -1;
    // Special pseudo-field "__kappa__" sorts by the computed κ value,
    // since κ doesn't live on the row itself — it's in kappaMap, keyed
    // by the row's primary key.
    if (sort.field === "__kappa__") {
      return [...rows].sort((a, b) => {
        const ka = keyField ? kappaMap?.get(String(a[keyField] ?? "")) ?? 0 : 0;
        const kb = keyField ? kappaMap?.get(String(b[keyField] ?? "")) ?? 0 : 0;
        return dir * (ka - kb);
      });
    }
    return [...rows].sort(
      (a, b) => dir * compareValues(a[sort.field], b[sort.field]),
    );
  }, [rows, sort, kappaMap, keyField]);
  // Mirror to the ref consumed by drag-fill callbacks (declared
  // earlier in the body, before sortedRows is in scope).
  sortedRowsRef.current = sortedRows;

  /**
   * Bottom-right cell of the current range — that's the only cell
   * that renders the fill handle. Recomputes whenever the range or
   * the row/field order changes.
   */
  const rangeBottomRight = useMemo(() => {
    if (!cellRange || !keyField) return null;
    const rowOrder = sortedRows.map((sr) => String(sr[keyField] ?? ""));
    const ar = rowOrder.indexOf(cellRange.anchorRowKey);
    const fr = rowOrder.indexOf(cellRange.focusRowKey);
    const ac = fieldOrder.indexOf(cellRange.anchorField);
    const fc = fieldOrder.indexOf(cellRange.focusField);
    if (ar < 0 || fr < 0 || ac < 0 || fc < 0) return null;
    return {
      rowKey: rowOrder[Math.max(ar, fr)],
      field: fieldOrder[Math.max(ac, fc)],
    };
  }, [cellRange, sortedRows, keyField, fieldOrder]);

  /** Is `(rowKey, field)` inside the live drag-fill preview band? */
  const isInFillPreview = useCallback(
    (rowKey: string, field: string): boolean => {
      const src = fillSourceRef.current;
      if (!src || !fillTarget || !keyField) return false;
      const rowOrder = sortedRows.map((sr) => String(sr[keyField] ?? ""));
      const tgtR = rowOrder.indexOf(fillTarget.rowKey);
      const tgtC = fieldOrder.indexOf(fillTarget.field);
      const srcMaxR = Math.max(
        rowOrder.indexOf(src.anchorRowKey),
        rowOrder.indexOf(src.focusRowKey),
      );
      const srcMaxC = Math.max(
        fieldOrder.indexOf(src.anchorField),
        fieldOrder.indexOf(src.focusField),
      );
      const r = rowOrder.indexOf(rowKey);
      const c = fieldOrder.indexOf(field);
      if (r < 0 || c < 0) return false;
      // Extension is past the source's bottom-right corner, up to the target.
      if (r > srcMaxR && r <= tgtR && c >= 0 && c <= srcMaxC) return true;
      if (c > srcMaxC && c <= tgtC && r >= 0 && r <= srcMaxR) return true;
      return false;
    },
    [fillTarget, keyField, sortedRows, fieldOrder],
  );

  /**
   * Distinct existing values per categorical column — drives the
   * `<datalist>` autocomplete inside CellEditor. Capped at 100 distinct
   * values per column so a free-text column accidentally typed as
   * categorical doesn't blow up the dropdown.
   */
  const distinctValuesByField = useMemo(() => {
    const map = new Map<string, string[]>();
    for (const col of columns) {
      if (col.type !== "categorical") continue;
      const set = new Set<string>();
      for (const r of rows) {
        const v = r[col.name];
        if (v == null || v === "") continue;
        set.add(String(v));
        if (set.size >= 100) break;
      }
      map.set(col.name, Array.from(set).sort());
    }
    return map;
  }, [rows, columns]);

  const virtualizer = useVirtualizer({
    count: sortedRows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 8,
  });

  // Default widths, deliberately tight so most bundles fit without scroll.
  function defaultColWidth(col: FieldDescriptor, isKey: boolean): number {
    if (isKey) return 160;
    if (col.type === "numeric") return 100;
    if (col.type === "boolean") return 90;
    if (col.type === "timestamp") return 140;
    return 180; // text / categorical / encrypted
  }

  /** Build the grid column track template from the current widths.
   *  Two sticky gutter columns lead: row number (#), then κ. */
  const gridTemplate = [
    "44px", // row-number gutter
    "70px", // κ gutter (fits "X.XX" / "XX.X" / "XXX" with breathing room)
    ...columns.map((c, i) => {
      const w = colWidths.get(c.name) ?? defaultColWidth(c, i === 0);
      return `${w}px`;
    }),
  ].join(" ");

  /**
   * Drag-to-resize. Tracks the mouse down position + starting width,
   * updates as the mouse moves, commits on mouse up. Listeners live on
   * document so the cursor doesn't have to stay over the handle.
   */
  const resizeRef = useRef<{ field: string; startX: number; startW: number } | null>(null);
  const startResize = (field: string, startW: number) =>
    (e: React.MouseEvent<HTMLDivElement>) => {
      e.preventDefault();
      e.stopPropagation();
      resizeRef.current = { field, startX: e.clientX, startW };
      const onMove = (m: MouseEvent) => {
        if (!resizeRef.current) return;
        const delta = m.clientX - resizeRef.current.startX;
        const next = Math.max(48, resizeRef.current.startW + delta);
        setColWidths((prev) => {
          const m2 = new Map(prev);
          m2.set(resizeRef.current!.field, next);
          return m2;
        });
      };
      const onUp = () => {
        resizeRef.current = null;
        document.removeEventListener("mousemove", onMove);
        document.removeEventListener("mouseup", onUp);
        document.body.style.cursor = "";
      };
      document.body.style.cursor = "col-resize";
      document.addEventListener("mousemove", onMove);
      document.addEventListener("mouseup", onUp);
    };

  const commit = (rowKey: string, field: string, raw: string, originalType: FieldDescriptor["type"]) => {
    if (!onCellEdit) {
      setEditing(null);
      return;
    }
    // A leading `=` always means "this is a formula" — pass the raw text
    // through verbatim so the parent can hand it to the formula engine.
    // `parseValue(raw, "numeric")` would otherwise coerce `"=A1+B1"` to NaN
    // and we'd lose the formula entirely.
    const isFormula = raw.startsWith("=");
    const parsed = isFormula ? raw : parseValue(raw, originalType);
    onCellEdit(rowKey, field, parsed);
    setEditing(null);
  };

  return (
    <div className="grid-root" data-testid="grid" data-bundle={bundleName}>
      <div ref={scrollRef} className="grid-scroll">
        {/* Excel-style column letters row. Lives above the field-name
            header. Clicking a letter selects the entire column; right-
            clicking opens the column context menu. The first two slots
            (row-number + κ gutters) are blanks so the letters line up
            with their data columns. */}
        <div
          className="grid-letters-row"
          style={{ gridTemplateColumns: gridTemplate }}
          data-testid="grid-letters-row"
        >
          <div className="grid-letter-corner" aria-hidden="true" />
          <div className="grid-letter-corner" aria-hidden="true" />
          {columns.map((col, i) => {
            const isSelected = selectedColumn === col.name;
            return (
              <button
                key={col.name}
                type="button"
                className={`grid-letter-cell ${isSelected ? "grid-letter-cell-active" : ""}`}
                data-testid={`grid-letter-${col.name}`}
                data-column={col.name}
                data-letter={indexToLetter(i)}
                aria-label={`Select column ${col.name}`}
                title={`Column ${indexToLetter(i)} · ${col.name}\nClick to select column · right-click for actions`}
                onClick={() => {
                  if (!onColumnSelect) return;
                  // Click toggles: clicking the already-selected column clears.
                  onColumnSelect(isSelected ? null : col.name);
                }}
                onContextMenu={(e) => {
                  if (!onColumnContextMenu) return;
                  e.preventDefault();
                  // Select on right-click so the menu actions know what's targeted.
                  onColumnSelect?.(col.name);
                  onColumnContextMenu(col.name, e.clientX, e.clientY);
                }}
              >
                {indexToLetter(i)}
              </button>
            );
          })}
        </div>
        {/* Header lives inside the scroll container so it slides horizontally
            with the rows; sticky-top keeps it visible during vertical scroll. */}
        <div
          className="grid-header"
          style={{ gridTemplateColumns: gridTemplate }}
          data-testid="grid-header"
        >
          <div
            className="grid-hcell grid-hcell-row-number grid-cell-sticky-row-number"
            data-testid="header-row-number"
            title="Row number (1-indexed within the current sort/filter)"
          >
            #
          </div>
          {(() => {
            const kappaSortActive = sort?.field === "__kappa__";
            const kappaSortDir: SortDir | null = kappaSortActive ? sort!.dir : null;
            const kappaAriaSort: "ascending" | "descending" | "none" = kappaSortActive
              ? kappaSortDir === "asc" ? "ascending" : "descending"
              : "none";
            return (
              <div
                className={`grid-hcell grid-hcell-kappa grid-cell-sticky-kappa grid-hcell-sortable ${kappaSortActive ? "grid-hcell-sort-active" : ""}`}
                data-testid="header-kappa"
                data-sort={kappaSortDir ?? "none"}
                role="columnheader"
                aria-sort={kappaAriaSort}
                title={
                  kappaSortActive
                    ? `Sorted κ ${kappaSortDir === "asc" ? "ascending" : "descending"} — click to ${kappaSortDir === "asc" ? "reverse" : "clear"}`
                    : "Click to sort by κ (curvature)"
                }
                onClick={() => cycleSort("__kappa__")}
              >
                κ
                {kappaSortActive ? <SortIndicator dir={kappaSortDir!} /> : null}
              </div>
            );
          })()}
          {columns.map((col, i) => {
            const width =
              colWidths.get(col.name) ?? defaultColWidth(col, i === 0);
            const sortable = isSortable(col);
            const sortActive = sort?.field === col.name;
            const sortDir: SortDir | null = sortActive ? sort!.dir : null;
            const ariaSort: "ascending" | "descending" | "none" = sortActive
              ? sortDir === "asc"
                ? "ascending"
                : "descending"
              : "none";
            const headerClasses = [
              "grid-hcell",
              i === 0 ? "grid-cell-sticky-key" : "",
              sortable ? "grid-hcell-sortable" : "",
              sortActive ? "grid-hcell-sort-active" : "",
            ]
              .filter(Boolean)
              .join(" ");
            return (
              <div
                key={col.name}
                className={headerClasses}
                data-testid={`header-${col.name}`}
                data-sort={sortDir ?? "none"}
                role="columnheader"
                aria-sort={ariaSort}
                title={
                  sortable
                    ? sortActive
                      ? `Sorted ${sortDir === "asc" ? "ascending" : "descending"} — click to ${sortDir === "asc" ? "reverse" : "clear"}`
                      : "Click to sort"
                    : undefined
                }
                onClick={
                  sortable
                    ? (e) => {
                        // Don't sort when clicking the resize handle.
                        if ((e.target as HTMLElement).closest(".grid-col-resize")) return;
                        cycleSort(col.name);
                      }
                    : undefined
                }
              >
                <span className="hname">{col.name}</span>
                <span className="htype">· {col.type}</span>
                {sortActive ? (
                  <SortIndicator dir={sortDir!} />
                ) : sortable ? (
                  <span className="grid-hcell-sort-hint" aria-hidden="true" />
                ) : null}
                {onColumnFilterClick ? (
                  <button
                    type="button"
                    className={`grid-hcell-filter ${activeFilterColumns?.has(col.name) ? "grid-hcell-filter-active" : ""}`}
                    data-testid={`grid-filter-btn-${col.name}`}
                    aria-label={`Filter ${col.name}`}
                    title={
                      activeFilterColumns?.has(col.name)
                        ? `Active filter on ${col.name} — click to edit or clear`
                        : `Filter ${col.name}`
                    }
                    onClick={(e) => {
                      e.stopPropagation(); // don't trigger sort
                      onColumnFilterClick(col.name, e.currentTarget);
                    }}
                    onMouseDown={(e) => e.stopPropagation()}
                  >
                    ▾
                  </button>
                ) : null}
                <div
                  className="grid-col-resize"
                  role="separator"
                  aria-orientation="vertical"
                  aria-label={`Resize column ${col.name}`}
                  data-testid={`resize-${col.name}`}
                  onMouseDown={startResize(col.name, width)}
                  onClick={(e) => e.stopPropagation()}
                />
              </div>
            );
          })}
        </div>

        {sortedRows.length === 0 ? (
          <EmptyState
            bundleEmpty={schemaRecords === 0}
            onInsertRow={emptyActions?.onInsertRow}
            onImportCsv={emptyActions?.onImportCsv}
            onOpenSchema={emptyActions?.onOpenSchema}
            onContextMenu={(x, y) => onRowContextMenu?.("", x, y)}
          />
        ) : (
          <div
            className="grid-rows"
            style={{
              height: printing ? sortedRows.length * ROW_HEIGHT : virtualizer.getTotalSize(),
              position: "relative",
            }}
          >
            {(printing
              ? sortedRows.map((_, i) => ({ index: i, start: i * ROW_HEIGHT, key: i }))
              : virtualizer.getVirtualItems()
            ).map((vrow) => {
              const row = sortedRows[vrow.index];
              const rowKey = keyField ? String(row[keyField]) : String(vrow.index);
              const kRaw = kappaMap?.get(rowKey);
              const k = typeof kRaw === "number" ? kRaw : undefined;
              const kClass: KappaClass = k === undefined ? "ok" : kappaClass(k);
              const isSelected = selectedKeys.has(rowKey);
              const isFocused = selectedRowKey === rowKey;
              const rowClasses = [
                "grid-row",
                `kappa-${kClass}`,
                isSelected ? "grid-row-selected" : "",
                isFocused ? "grid-row-focused" : "",
              ]
                .filter(Boolean)
                .join(" ");
              return (
                <div
                  key={rowKey}
                  className={rowClasses}
                  data-testid="grid-row"
                  data-row-key={rowKey}
                  data-kappa-class={kClass}
                  data-selected={isSelected ? "true" : "false"}
                  data-focused={isFocused ? "true" : "false"}
                  style={{
                    transform: `translateY(${vrow.start}px)`,
                    gridTemplateColumns: gridTemplate,
                    height: ROW_HEIGHT,
                  }}
                  onClick={(e) => {
                    const target = e.target as HTMLElement;
                    // Don't interfere with the inline editor — clicks
                    // inside the input shouldn't trigger row selection.
                    if (target.closest(".grid-cell-editing")) return;
                    // For everything else, row selection fires. Clicks
                    // on editable cells ALSO start the editor (handled
                    // by the cell's own onClick) — both are legit.
                    if (onRowClick) {
                      onRowClick(rowKey, {
                        meta: e.metaKey || e.ctrlKey,
                        shift: e.shiftKey,
                        alt: e.altKey,
                      });
                    } else {
                      onRowSelect?.(rowKey);
                    }
                  }}
                  onContextMenu={(e) => {
                    if (!onRowContextMenu) return;
                    e.preventDefault();
                    onRowContextMenu(rowKey, e.clientX, e.clientY);
                  }}
                >
                  <div
                    className="grid-cell grid-cell-row-number grid-cell-sticky-row-number"
                    data-testid="row-number"
                    aria-hidden="true"
                  >
                    {vrow.index + 1}
                  </div>
                  <KappaCell kappa={k} kClass={kClass} sticky />
                  {columns.map((col, i) => {
                    const cfRule = conditionalFormats?.get(col.name);
                    const cfHit =
                      cfRule && (k ?? 0) >= cfRule.kappaThreshold
                        ? cfRule.color
                        : null;
                    return (
                    <Cell
                      key={col.name}
                      value={row[col.name]}
                      field={col}
                      rowKey={rowKey}
                      keyField={keyField}
                      kappa={k ?? 0}
                      sticky={i === 0}
                      cfHitColor={cfHit ?? undefined}
                      editable={isEditable(col, keyField) && Boolean(onCellEdit)}
                      isEditing={
                        editing?.rowKey === rowKey && editing?.field === col.name
                      }
                      columnSelected={selectedColumn === col.name}
                      suggestions={distinctValuesByField.get(col.name)}
                      formulaText={getFormulaText?.(rowKey, col.name) ?? null}
                      inRange={isCellInRange(
                        cellRange ?? null,
                        rowKey,
                        col.name,
                        sortedRows.map((sr) =>
                          keyField ? String(sr[keyField] ?? "") : "",
                        ),
                        fieldOrder,
                      )}
                      onMouseDownCell={(shiftKey) =>
                        beginDrag(rowKey, col.name, shiftKey)
                      }
                      onMouseEnterCell={() => {
                        extendDrag(rowKey, col.name);
                        extendFill(rowKey, col.name);
                      }}
                      consumeDragSuppress={consumeDragSuppress}
                      isFillCorner={
                        !!rangeBottomRight &&
                        rangeBottomRight.rowKey === rowKey &&
                        rangeBottomRight.field === col.name &&
                        Boolean(onDragFill)
                      }
                      inFillPreview={isInFillPreview(rowKey, col.name)}
                      onFillHandleMouseDown={beginFill}
                      onBeginEdit={() => {
                        setEditing({ rowKey, field: col.name });
                        onCellFocus?.(rowKey, col.name);
                      }}
                      onCancel={() => setEditing(null)}
                      onCommit={(raw) => commit(rowKey, col.name, raw, col.type)}
                    />
                    );
                  })}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

function KappaCell({
  kappa,
  kClass,
  sticky,
}: {
  kappa: number | undefined;
  kClass: KappaClass;
  sticky?: boolean;
}) {
  const stickyClass = sticky ? "grid-cell-sticky-kappa" : "";
  if (kappa === undefined) {
    return (
      <div
        className={`grid-cell grid-cell-kappa ${stickyClass}`}
        data-testid="kappa-cell"
        data-kappa-class={kClass}
      >
        —
      </div>
    );
  }
  // Format width-adaptively: < 10 shows "X.XX", 10-99 shows "XX.X", 100+
  // shows "XXX" (integer). Keeps the column readable for any magnitude.
  const display =
    kappa < 10
      ? kappa.toFixed(2)
      : kappa < 100
        ? kappa.toFixed(1)
        : Math.round(kappa).toString();
  return (
    <div
      className={`grid-cell grid-cell-kappa kappa-${kClass} ${stickyClass}`}
      data-testid="kappa-cell"
      data-kappa-class={kClass}
      data-kappa={kappa.toFixed(3)}
      title={`κ = ${kappa.toFixed(3)} · ${kClass}`}
    >
      <span className="kval">{display}</span>
    </div>
  );
}

interface CellProps {
  value: unknown;
  field: FieldDescriptor;
  rowKey: string;
  keyField: string | undefined;
  /** Curvature of the row this cell belongs to. Passed in so format
   *  strings with a `[κ>τ]` conditional prefix can light up anomalies
   *  inline (e.g. `[κ>0.3]"⚠️ "0.00`). 0 if not yet computed. */
  kappa: number;
  sticky?: boolean;
  editable: boolean;
  isEditing: boolean;
  /** Distinct existing values for the column — used for categorical autocomplete. */
  suggestions?: string[];
  /** Whether this cell's column is the currently-selected one (lights up). */
  columnSelected?: boolean;
  /**
   * The raw formula text for this cell from the sidecar, or null if the
   * cell isn't a formula. When set, the cell still displays the evaluated
   * value (already in `value`) but the editor opens on the formula text.
   */
  formulaText?: string | null;
  /** True when this cell sits inside the active range-select rectangle. */
  inRange?: boolean;
  /** Mouse-down on the cell — start a drag-range from here. */
  onMouseDownCell?: (shiftKey: boolean) => void;
  /** Mouse entered the cell while a drag is in progress — extend focus. */
  onMouseEnterCell?: () => void;
  /**
   * The Body uses this to tell the cell "the last click was the end of
   * a drag, suppress the editor". The cell consumes the flag on click;
   * returns true to mean "suppress this click".
   */
  consumeDragSuppress?: () => boolean;
  /**
   * True if this cell is the bottom-right corner of the active range —
   * in which case it renders the small drag-fill handle that extends
   * the range Excel-style.
   */
  isFillCorner?: boolean;
  /** True while a drag-fill is in progress and this cell is in the projected fill band. */
  inFillPreview?: boolean;
  /** Mousedown on the fill handle — starts a fill-drag. */
  onFillHandleMouseDown?: (e: React.MouseEvent) => void;
  /**
   * Conditional-format color preset to apply to this cell, or undefined
   * if no rule matches. Mapped to the `.cf-cond-<color>` class which
   * lives in ConditionalFormatModal.css.
   */
  cfHitColor?: "red" | "amber" | "green" | "blue" | "purple";
  onBeginEdit: () => void;
  onCancel: () => void;
  onCommit: (raw: string) => void;
}

function Cell({
  value,
  field,
  kappa,
  sticky,
  editable,
  isEditing,
  suggestions,
  columnSelected,
  formulaText,
  inRange,
  onMouseDownCell,
  onMouseEnterCell,
  consumeDragSuppress,
  isFillCorner,
  inFillPreview,
  onFillHandleMouseDown,
  cfHitColor,
  onBeginEdit,
  onCancel,
  onCommit,
}: CellProps) {
  const stickyClass = sticky ? "grid-cell-sticky-key" : "";
  const colSelClass = columnSelected ? "grid-cell-col-selected" : "";
  const hasFormula = typeof formulaText === "string" && formulaText.length > 0;
  if (isEditing) {
    return (
      <CellEditor
        field={field}
        initial={value}
        formulaText={formulaText ?? null}
        onCommit={onCommit}
        onCancel={onCancel}
        sticky={sticky}
        suggestions={suggestions}
      />
    );
  }
  if (value == null) {
    const nullClasses = [
      "grid-cell",
      "grid-cell-null",
      editable ? "grid-cell-editable" : "",
      hasFormula ? "grid-cell-has-formula" : "",
      inRange ? "grid-cell-in-range" : "",
      stickyClass,
      colSelClass,
    ]
      .filter(Boolean)
      .join(" ");
    return (
      <div
        className={nullClasses}
        data-testid={editable ? "editable-cell" : undefined}
        data-field={field.name}
        data-has-formula={hasFormula ? "true" : undefined}
        data-in-range={inRange ? "true" : undefined}
        title={editable ? "Click to edit (empty)" : undefined}
        onMouseDown={(e) => {
          if (e.button !== 0) return;
          if (e.metaKey || e.ctrlKey) return;
          onMouseDownCell?.(e.shiftKey);
        }}
        onMouseEnter={() => onMouseEnterCell?.()}
        onClick={
          editable
            ? (e) => {
                if (e.metaKey || e.ctrlKey || e.shiftKey) return;
                if (consumeDragSuppress?.()) return;
                onBeginEdit();
              }
            : undefined
        }
      >
        —
      </div>
    );
  }
  if (field.encryption && field.encryption !== "none") {
    return <EncryptedCell value={value} field={field} sticky={sticky} />;
  }
  // Formula-error sentinel detection. A cell whose value is one of the
  // engine's error strings (e.g. "#REF!", "#DIV0!") gets a red badge
  // and a tooltip explaining the failure mode, so users notice broken
  // formulas without having to open the editor.
  const errSentinel = asError(value);
  const isKeyColumn = !!sticky;
  const classes = [
    "grid-cell",
    field.type === "numeric" || typeof value === "number" ? "grid-cell-num" : "",
    editable ? "grid-cell-editable" : "",
    isKeyColumn && editable ? "grid-cell-key-editable" : "",
    hasFormula ? "grid-cell-has-formula" : "",
    errSentinel ? "grid-cell-error" : "",
    inRange ? "grid-cell-in-range" : "",
    inFillPreview ? "grid-cell-fill-preview" : "",
    cfHitColor ? `cf-cond-${cfHitColor}` : "",
    stickyClass,
    colSelClass,
  ]
    .filter(Boolean)
    .join(" ");
  // Schema-driven default format. `$_usd` columns get `$#,##0.00`,
  // `*_pct` get `0.0%`, date-shaped columns get ISO. Anything else
  // falls through to plain string render.
  const fmt = defaultFormatFor(field);
  const display = fmt
    ? formatValue(value, fmt, { kappa })
    : String(value);
  // Tooltip explains what happens on key edit so users aren't surprised
  // by the rename-flow confirmation. Formula cells get a hint so the
  // user knows clicking will reveal the formula text, not the value.
  const cellTitle = isKeyColumn && editable
    ? "Click to rename. Editing the primary key deletes the old row and inserts a new one."
    : errSentinel
      ? hasFormula
        ? `${errSentinel} — formula failed: ${formulaText} · click to edit`
        : `${errSentinel} — formula error · click to edit`
      : hasFormula
        ? `Formula: ${formulaText} · click to edit`
        : editable
          ? "Click to edit"
          : undefined;
  return (
    <div
      className={classes}
      data-testid={editable ? "editable-cell" : undefined}
      data-field={field.name}
      data-has-formula={hasFormula ? "true" : undefined}
      data-error={errSentinel ?? undefined}
      data-in-range={inRange ? "true" : undefined}
      title={cellTitle}
      onMouseDown={(e) => {
        // Left mouse only; range-drag uses primary button. Modifier-
        // less drags start a fresh range; shift+drag extends.
        if (e.button !== 0) return;
        if (e.metaKey || e.ctrlKey) return; // row-level multiselect path
        onMouseDownCell?.(e.shiftKey);
      }}
      onMouseEnter={() => onMouseEnterCell?.()}
      onClick={
        editable
          ? (e) => {
              // Modifier-clicks should reach the row for multi-select.
              if (e.metaKey || e.ctrlKey || e.shiftKey) return;
              // Suppress the editor when this click ended a drag-range.
              if (consumeDragSuppress?.()) return;
              onBeginEdit();
            }
          : undefined
      }
    >
      {errSentinel ? (
        <span className="grid-cell-error-badge" data-testid="cell-error-badge">
          {errSentinel}
        </span>
      ) : (
        display
      )}
      {isFillCorner ? (
        <span
          className="grid-cell-fill-handle"
          data-testid="grid-fill-handle"
          title="Drag to fill"
          onMouseDown={(e) => onFillHandleMouseDown?.(e)}
          // Suppress click so it doesn't bubble up and open the editor.
          onClick={(e) => e.stopPropagation()}
        />
      ) : null}
    </div>
  );
}

interface CellEditorProps {
  field: FieldDescriptor;
  initial: unknown;
  sticky?: boolean;
  /**
   * Distinct existing values for the field — used to populate the datalist
   * on categorical edits so the user gets autocomplete on the values
   * already in use. Empty/missing → no suggestions, free text only.
   */
  suggestions?: string[];
  /**
   * Raw formula text from the sidecar (e.g. `=A1+B1`). When non-null,
   * the editor opens with this as the initial draft instead of the
   * evaluated value, and forces text-mode input so the `=` survives a
   * field that's otherwise numeric/boolean/timestamp.
   */
  formulaText?: string | null;
  onCommit: (raw: string) => void;
  onCancel: () => void;
}

function CellEditor({
  field,
  initial,
  sticky,
  suggestions,
  formulaText,
  onCommit,
  onCancel,
}: CellEditorProps) {
  const [draft, setDraft] = useState<string>(
    formulaText != null && formulaText !== ""
      ? formulaText
      : initial == null
        ? ""
        : String(initial),
  );

  const handleKey = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      onCommit(draft);
    } else if (e.key === "Escape") {
      e.preventDefault();
      onCancel();
    }
  };

  // Formula mode short-circuits all the type-specific editors. A
  // `=`-prefixed draft, or a pre-existing formula in the sidecar, both
  // need a plain text input so the `=` and operators survive the
  // numeric/boolean/timestamp coercion that those inputs would otherwise
  // apply.
  const isFormulaMode =
    (formulaText != null && formulaText !== "") || draft.startsWith("=");
  if (isFormulaMode) {
    return (
      <div
        className={`grid-cell grid-cell-editing grid-cell-editing-formula ${sticky ? "grid-cell-sticky-key" : ""}`}
        data-testid="cell-editor"
      >
        <input
          autoFocus
          type="text"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={handleKey}
          onBlur={() => onCommit(draft)}
          data-testid="cell-editor-input"
          data-field={field.name}
          data-formula-mode="true"
        />
      </div>
    );
  }

  // Boolean: render a select with true / false / — (null).
  if (field.type === "boolean") {
    return (
      <div
        className={`grid-cell grid-cell-editing ${sticky ? "grid-cell-sticky-key" : ""}`}
        data-testid="cell-editor"
      >
        <select
          autoFocus
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={handleKey}
          onBlur={() => onCommit(draft)}
          data-testid="cell-editor-input"
          data-field={field.name}
        >
          <option value="">—</option>
          <option value="true">true</option>
          <option value="false">false</option>
        </select>
      </div>
    );
  }

  // Timestamp: native date picker. If the initial value parses as ISO,
  // pre-fill the date portion (YYYY-MM-DD); otherwise fall back to raw text.
  if (field.type === "timestamp") {
    const isoDate = toIsoDate(draft);
    return (
      <div
        className={`grid-cell grid-cell-editing ${sticky ? "grid-cell-sticky-key" : ""}`}
        data-testid="cell-editor"
      >
        <input
          autoFocus
          type="date"
          value={isoDate}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={handleKey}
          onBlur={() => onCommit(draft)}
          data-testid="cell-editor-input"
          data-field={field.name}
        />
      </div>
    );
  }

  // Categorical: text input with a datalist of existing values for
  // autocomplete — free-typing still allowed for new values.
  //
  // Numeric cells also use type="text" (with inputMode="decimal" to keep
  // the mobile numeric keyboard). `<input type="number">` silently strips
  // a leading `=`, which would block converting a numeric cell into a
  // formula. `parseValue` still coerces digits → number on commit, so
  // the data path is identical for non-formula edits.
  const isCategorical = field.type === "categorical";
  const isNumeric = field.type === "numeric";
  const listId = isCategorical
    ? `cell-editor-list-${field.name.replace(/[^A-Za-z0-9_]/g, "_")}`
    : undefined;

  return (
    <div
      className={`grid-cell grid-cell-editing ${sticky ? "grid-cell-sticky-key" : ""}`}
      data-testid="cell-editor"
    >
      <input
        autoFocus
        type="text"
        inputMode={isNumeric ? "decimal" : undefined}
        value={draft}
        list={listId}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={handleKey}
        onBlur={() => onCommit(draft)}
        data-testid="cell-editor-input"
        data-field={field.name}
      />
      {isCategorical && suggestions && suggestions.length > 0 ? (
        <datalist id={listId}>
          {suggestions.map((s) => (
            <option key={s} value={s} />
          ))}
        </datalist>
      ) : null}
    </div>
  );
}

function isEditable(field: FieldDescriptor, keyField: string | undefined): boolean {
  // The primary key IS editable, but committing one triggers a rename
  // flow (delete-old + insert-new) instead of a plain field update —
  // see App.tsx#onCellEdit. We mark it editable here so the click
  // surface works; the heavy lifting + confirmation lives upstream.
  void keyField;
  if (field.encryption && field.encryption !== "none") return false;
  return (
    field.type === "numeric" ||
    field.type === "text" ||
    field.type === "categorical" ||
    field.type === "boolean" ||
    field.type === "timestamp"
  );
}

function parseValue(raw: string, type: FieldDescriptor["type"]): unknown {
  if (raw === "") return null;
  if (type === "numeric") {
    const n = Number(raw);
    return Number.isFinite(n) ? n : raw;
  }
  if (type === "boolean") {
    if (raw === "true" || raw === "1" || raw === "yes") return true;
    if (raw === "false" || raw === "0" || raw === "no") return false;
    return null;
  }
  // text / categorical / timestamp pass through as strings. The engine
  // is responsible for any timestamp-specific parsing on its side; we
  // ship whatever the user typed so we don't lock in a format here.
  return raw;
}

/**
 * Encrypted cell — renders per the field's encryption mode without ever
 * leaking plaintext (or anything that looks like plaintext) for OPAQUE
 * fields, where the engine itself isn't supposed to surface the value.
 *
 *   OPAQUE   → ▒▒▒▒▒ blocks. No useful textContent.
 *   INDEXED  → short stable hash of the value (so equality reads stay readable).
 *   AFFINE   → the affine-transformed numeric, shown verbatim with the gauge tag.
 *   else     → fall through as a normal value-rendered cell.
 */
function EncryptedCell({
  value,
  field,
  sticky,
}: {
  value: unknown;
  field: FieldDescriptor;
  sticky?: boolean;
}) {
  const mode = field.encryption ?? "none";
  const stickyClass = sticky ? "grid-cell-sticky-key" : "";
  if (mode === "opaque") {
    return (
      <div
        className={`grid-cell grid-cell-enc grid-cell-enc-opaque ${stickyClass}`}
        title="OPAQUE — value masked in this UI. Real encryption is enforced by the engine; demo bundles use a display-only overlay."
        data-testid="encrypted-cell"
        data-encryption="opaque"
        aria-label="opaque encrypted value"
      >
        <LockIcon />
        <span className="enc-val" aria-hidden="true">▒▒▒▒▒▒</span>
      </div>
    );
  }
  if (mode === "indexed") {
    return (
      <div
        className={`grid-cell grid-cell-enc ${stickyClass}`}
        title="INDEXED — equality-lookup friendly. Real encryption is enforced by the engine; demo bundles use a display-only overlay."
        data-testid="encrypted-cell"
        data-encryption="indexed"
      >
        <LockIcon />
        <span className="enc-val mono">{String(value)}</span>
      </div>
    );
  }
  if (mode === "affine") {
    return (
      <div
        className={`grid-cell grid-cell-enc grid-cell-enc-affine ${stickyClass}`}
        title="AFFINE · numeric gauge · v ↦ a·v + b"
        data-testid="encrypted-cell"
        data-encryption="affine"
      >
        <LockIcon />
        <span className="enc-val mono">{String(value)}</span>
      </div>
    );
  }
  return (
    <div
      className={`grid-cell grid-cell-enc ${stickyClass}`}
      title={`encryption: ${mode}`}
      data-testid="encrypted-cell"
      data-encryption={mode}
    >
      <LockIcon />
      <span className="enc-val">{String(value)}</span>
    </div>
  );
}

function SortIndicator({ dir }: { dir: SortDir }) {
  return (
    <span
      className={`grid-sort-indicator grid-sort-${dir}`}
      data-testid="sort-indicator"
      data-sort-dir={dir}
      aria-hidden="true"
    >
      <svg width="9" height="9" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="2">
        {dir === "asc" ? (
          <path d="M3 8l3-4 3 4" strokeLinecap="round" strokeLinejoin="round" />
        ) : (
          <path d="M3 4l3 4 3-4" strokeLinecap="round" strokeLinejoin="round" />
        )}
      </svg>
    </span>
  );
}

function LockIcon() {
  return (
    <svg
      className="lock-icon"
      width="11"
      height="11"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      aria-hidden="true"
    >
      <rect x="5" y="11" width="14" height="9" rx="2" />
      <path d="M8 11V8a4 4 0 0 1 8 0v3" />
    </svg>
  );
}

function Skeleton() {
  return (
    <div className="grid-skeleton" data-testid="grid-skeleton" aria-busy="true">
      {Array.from({ length: 6 }, (_, i) => (
        <div
          key={i}
          className="skeleton-row"
          style={{ opacity: 1 - i * 0.1 }}
        />
      ))}
    </div>
  );
}

function EmptyState({
  bundleEmpty,
  onInsertRow,
  onImportCsv,
  onOpenSchema,
  onContextMenu,
}: {
  bundleEmpty: boolean;
  onInsertRow?: () => void;
  onImportCsv?: () => void;
  onOpenSchema?: () => void;
  onContextMenu?: (x: number, y: number) => void;
}) {
  return (
    <div
      className="grid-empty"
      data-testid="grid-empty"
      onContextMenu={(e) => {
        e.preventDefault();
        onContextMenu?.(e.clientX, e.clientY);
      }}
    >
      <div className="grid-empty-card">
        <h3>{bundleEmpty ? "This bundle is empty." : "No rows match the current view."}</h3>
        <p>
          {bundleEmpty
            ? "Add a row, paste a spreadsheet, or open the schema to add fields."
            : "Try clearing filters or switching cover field."}
        </p>
        <div className="grid-empty-actions">
          {onInsertRow ? (
            <button
              type="button"
              className="grid-empty-btn grid-empty-btn-primary"
              onClick={onInsertRow}
              data-testid="grid-empty-insert"
            >
              Add a row
            </button>
          ) : null}
          {onImportCsv ? (
            <button
              type="button"
              className="grid-empty-btn"
              onClick={onImportCsv}
              data-testid="grid-empty-import"
            >
              Import CSV / TSV
            </button>
          ) : null}
          {onOpenSchema ? (
            <button
              type="button"
              className="grid-empty-btn"
              onClick={onOpenSchema}
              data-testid="grid-empty-schema"
            >
              Edit schema
            </button>
          ) : null}
        </div>
        <p className="grid-empty-hint">
          Right-click anywhere in the grid for more options.
        </p>
      </div>
    </div>
  );
}

/** Extract a YYYY-MM-DD prefix from any reasonable date-shaped string.
 *  Used to pre-fill <input type="date"> when the engine returns ISO
 *  timestamps with time components. */
function toIsoDate(raw: string): string {
  if (!raw) return "";
  const m = raw.match(/^(\d{4})[-/](\d{1,2})[-/](\d{1,2})/);
  if (!m) return "";
  return `${m[1]}-${m[2].padStart(2, "0")}-${m[3].padStart(2, "0")}`;
}
