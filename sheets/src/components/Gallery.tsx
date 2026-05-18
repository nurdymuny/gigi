import { useMemo, useState } from "react";
import { defaultFormatFor, formatValue } from "../lib/format";
import type { BundleSchema, FieldDescriptor, RowMap } from "../lib/gigi-client";
import { kappaClass, type KappaClass } from "../lib/kappa";
import type { RowClickModifiers } from "./Grid";
import "./Gallery.css";

export interface GalleryProps {
  schema: BundleSchema | null;
  rows: RowMap[];
  kappaMap: Map<string, number>;
  /**
   * The primary categorical field, used for the card subtitle. Falls back
   * to the second column of the schema.
   */
  coverField: string;
  /**
   * Legacy single-select callback — called on plain click when
   * `onRowClick` isn't provided. Kept so older callers (Geometry/Form
   * tabs) keep working unchanged.
   */
  onRowSelect: (key: string) => void;
  /** Currently-focused row key, for visual highlight. */
  selectedRowKey?: string | null;
  /**
   * Multi-select set. When provided, every card whose key is in this
   * set gets the `gallery-card-selected` class. Falls back to
   * `{selectedRowKey}` so old callers still see their highlight.
   */
  selectedKeys?: Set<string>;
  /**
   * Modifier-aware click handler. When provided, takes precedence over
   * `onRowSelect` — the caller owns cmd/shift semantics (range select,
   * toggle-in-set, etc.). Mirrors the Grid's API so both views share a
   * selection model.
   */
  onRowClick?: (key: string, mods: RowClickModifiers) => void;
  /**
   * Right-click handler. Called with the card's row key and the viewport
   * coordinates of the click. Should typically open a context menu.
   */
  onRowContextMenu?: (key: string, x: number, y: number) => void;
  /**
   * When set, Gallery enters **find-similar mode**: cards reorder by
   * Davis sameness against this row (descending), the pivot card is
   * pinned to the top with a chip, and each card optionally shows its
   * sameness score. The toolbar's normal sort is disabled while a pivot
   * is set — clear it via `onClearSimilar` to restore.
   */
  similarPivot?: string | null;
  /** Caller-side reset for similar-mode (clears `similarPivot`). */
  onClearSimilar?: () => void;
  /**
   * (keyA, keyB) → Davis sameness lookup. Required only when
   * `similarPivot` is set; pass `buildBundleSameness({…})` from
   * `lib/formula-context.ts`. The Gallery doesn't know how to embed
   * rows on its own — the host wires this in.
   */
  sameness?: (keyA: string, keyB: string) => number;
}

type Density = "compact" | "standard" | "expanded";
type KappaFilter = "all" | "drift" | "bad";
type SortDir = "asc" | "desc";
/** Sort key: synthetic `"kappa"` / `"key"`, or any schema field name. */
type SortField = "kappa" | "key" | string;

/**
 * Card-grid view of the rows.
 *
 * Each row becomes a card with:
 *   - primary key as the title
 *   - cover field as the subtitle chip
 *   - body fields driven by the density toggle
 *   - κ-tinted border (green / amber / red)
 *
 * Phase 7 added the toolbar — group by / sort / κ-filter / density —
 * plus multi-select and right-click parity with the Grid. Future
 * revision (per FEATURE_PARITY §13): lay cards out by PCA on the
 * embedding matrix so similar rows cluster spatially.
 */
export function Gallery({
  schema,
  rows,
  kappaMap,
  coverField,
  onRowSelect,
  selectedRowKey,
  selectedKeys,
  onRowClick,
  onRowContextMenu,
  similarPivot,
  onClearSimilar,
  sameness,
}: GalleryProps) {
  const [density, setDensity] = useState<Density>("standard");
  const [kappaFilter, setKappaFilter] = useState<KappaFilter>("all");
  const [sortField, setSortField] = useState<SortField>("kappa");
  const [sortDir, setSortDir] = useState<SortDir>("desc");
  const [groupBy, setGroupBy] = useState<string>("");

  // Hooks must run before any conditional return, so compute even when
  // the schema isn't ready — the body short-circuits below.
  const keyField = schema?.base_fields[0]?.name ?? "";
  const allFields = useMemo(
    () => (schema ? [...schema.base_fields, ...schema.fiber_fields] : []),
    [schema],
  );

  // Body field selection per density level. Compact strips the body
  // entirely so cards are scannable in long lists; expanded shows
  // everything non-opaque so a single card carries the full row.
  const bodyFields = useMemo(() => {
    if (density === "compact") return [];
    const candidates = allFields.filter(
      (f) =>
        f.name !== keyField &&
        f.name !== coverField &&
        f.encryption !== "opaque",
    );
    return density === "expanded" ? candidates : candidates.slice(0, 4);
  }, [allFields, keyField, coverField, density]);

  // κ-class filter applies before sort/group so counts in group headers
  // match what's visible.
  const filteredRows = useMemo(() => {
    if (kappaFilter === "all") return rows;
    return rows.filter((r) => {
      const k = kappaMap.get(String(r[keyField] ?? "")) ?? 0;
      const c = kappaClass(k);
      if (kappaFilter === "bad") return c === "bad";
      // drift = "bad" or "warn" — the two κ classes that flag a row
      return c === "bad" || c === "warn";
    });
  }, [rows, kappaMap, keyField, kappaFilter]);

  /**
   * Per-row sameness against the find-similar pivot, keyed by row key.
   * Empty map when not in similar-mode. Computing once and threading
   * through sort + render avoids running the embedder per render.
   */
  const samenessByKey = useMemo(() => {
    const map = new Map<string, number>();
    if (!similarPivot || !sameness) return map;
    for (const r of filteredRows) {
      const k = String(r[keyField] ?? "");
      map.set(k, sameness(similarPivot, k));
    }
    return map;
  }, [similarPivot, sameness, filteredRows, keyField]);

  const sortedRows = useMemo(() => {
    // Similar-mode overrides the user's sort: cards always cascade by
    // sameness to the pivot, pivot pinned at the top (S=1 against
    // itself by construction).
    if (similarPivot) {
      return [...filteredRows].sort((a, b) => {
        const ka = String(a[keyField] ?? "");
        const kb = String(b[keyField] ?? "");
        const sa = samenessByKey.get(ka) ?? 0;
        const sb = samenessByKey.get(kb) ?? 0;
        return sb - sa; // desc
      });
    }
    const dir = sortDir === "asc" ? 1 : -1;
    const arr = [...filteredRows];
    arr.sort((a, b) => {
      if (sortField === "kappa") {
        const ka = kappaMap.get(String(a[keyField] ?? "")) ?? 0;
        const kb = kappaMap.get(String(b[keyField] ?? "")) ?? 0;
        return dir * (ka - kb);
      }
      if (sortField === "key") {
        return (
          dir *
          String(a[keyField] ?? "").localeCompare(String(b[keyField] ?? ""))
        );
      }
      const va = a[sortField];
      const vb = b[sortField];
      const nullA = va === null || va === undefined || va === "";
      const nullB = vb === null || vb === undefined || vb === "";
      if (nullA && nullB) return 0;
      if (nullA) return 1; // nulls sink to the bottom
      if (nullB) return -1;
      if (typeof va === "number" && typeof vb === "number") return dir * (va - vb);
      return dir * String(va).localeCompare(String(vb), undefined, { numeric: true });
    });
    return arr;
  }, [filteredRows, sortField, sortDir, keyField, kappaMap, similarPivot, samenessByKey]);

  // Group buckets: when groupBy is set, partition sortedRows by the
  // stringified value of that field. Preserve first-occurrence order
  // of group keys so the headers feel stable rather than alphabetical.
  const groups = useMemo<{ key: string; rows: RowMap[] }[]>(() => {
    if (!groupBy) return [{ key: "", rows: sortedRows }];
    const buckets = new Map<string, RowMap[]>();
    const order: string[] = [];
    for (const r of sortedRows) {
      const k = String(r[groupBy] ?? "—");
      let bucket = buckets.get(k);
      if (!bucket) {
        bucket = [];
        buckets.set(k, bucket);
        order.push(k);
      }
      bucket.push(r);
    }
    return order.map((k) => ({ key: k, rows: buckets.get(k) ?? [] }));
  }, [sortedRows, groupBy]);

  if (!schema) {
    return (
      <div className="gallery gallery-empty" data-testid="gallery-empty">
        <p>Loading bundle…</p>
      </div>
    );
  }
  if (rows.length === 0) {
    return (
      <div className="gallery gallery-empty" data-testid="gallery-empty">
        <p>No rows to render.</p>
      </div>
    );
  }

  const effectiveSelected: Set<string> =
    selectedKeys ??
    (selectedRowKey ? new Set<string>([selectedRowKey]) : new Set<string>());

  return (
    <div className="gallery" data-testid="gallery">
      <GalleryToolbar
        fields={allFields}
        keyField={keyField}
        groupBy={groupBy}
        onGroupByChange={setGroupBy}
        sortField={sortField}
        onSortFieldChange={setSortField}
        sortDir={sortDir}
        onSortDirChange={setSortDir}
        density={density}
        onDensityChange={setDensity}
        kappaFilter={kappaFilter}
        onKappaFilterChange={setKappaFilter}
        visibleCount={sortedRows.length}
        totalCount={rows.length}
        similarPivot={similarPivot ?? null}
        onClearSimilar={onClearSimilar}
      />
      {groupBy
        ? groups.map((g) => (
            <section
              key={g.key}
              className="gallery-group"
              data-testid={`gallery-group-${g.key}`}
            >
              <h3
                className="gallery-group-header"
                data-testid={`gallery-group-header-${g.key}`}
              >
                <span className="gallery-group-name">{g.key}</span>
                <span className="gallery-group-count">{g.rows.length}</span>
              </h3>
              <CardGrid
                rows={g.rows}
                keyField={keyField}
                coverField={coverField}
                bodyFields={bodyFields}
                kappaMap={kappaMap}
                density={density}
                effectiveSelected={effectiveSelected}
                onRowClick={onRowClick}
                onRowSelect={onRowSelect}
                onRowContextMenu={onRowContextMenu}
                similarPivot={similarPivot ?? null}
                samenessByKey={samenessByKey}
              />
            </section>
          ))
        : (
          <CardGrid
            rows={sortedRows}
            keyField={keyField}
            coverField={coverField}
            bodyFields={bodyFields}
            kappaMap={kappaMap}
            density={density}
            effectiveSelected={effectiveSelected}
            onRowClick={onRowClick}
            onRowSelect={onRowSelect}
            onRowContextMenu={onRowContextMenu}
            similarPivot={similarPivot ?? null}
            samenessByKey={samenessByKey}
          />
        )}
    </div>
  );
}

/* ── toolbar ─────────────────────────────────────────────────────── */

function GalleryToolbar({
  fields,
  keyField,
  groupBy,
  onGroupByChange,
  sortField,
  onSortFieldChange,
  sortDir,
  onSortDirChange,
  density,
  onDensityChange,
  kappaFilter,
  onKappaFilterChange,
  visibleCount,
  totalCount,
  similarPivot,
  onClearSimilar,
}: {
  fields: FieldDescriptor[];
  keyField: string;
  groupBy: string;
  onGroupByChange: (v: string) => void;
  sortField: SortField;
  onSortFieldChange: (v: SortField) => void;
  sortDir: SortDir;
  onSortDirChange: (v: SortDir) => void;
  density: Density;
  onDensityChange: (v: Density) => void;
  kappaFilter: KappaFilter;
  onKappaFilterChange: (v: KappaFilter) => void;
  visibleCount: number;
  totalCount: number;
  similarPivot: string | null;
  onClearSimilar?: () => void;
}) {
  // Group-by candidates: skip the primary key (every row would be its
  // own bucket — useless) and any encrypted-opaque columns (hashes
  // partition arbitrarily). Numerics are kept since some bundles use
  // them as discrete categories (rating, year, etc.).
  const groupCandidates = fields.filter(
    (f) => f.name !== keyField && f.encryption !== "opaque",
  );
  return (
    <div className="gallery-toolbar" data-testid="gallery-toolbar">
      <label className="gallery-toolbar-group">
        <span>Group by</span>
        <select
          value={groupBy}
          onChange={(e) => onGroupByChange(e.target.value)}
          data-testid="gallery-group-by"
        >
          <option value="">None</option>
          {groupCandidates.map((f) => (
            <option key={f.name} value={f.name}>
              {f.name}
            </option>
          ))}
        </select>
      </label>

      <label className="gallery-toolbar-group">
        <span>Sort</span>
        <select
          value={similarPivot ? "__similar__" : sortField}
          disabled={Boolean(similarPivot)}
          onChange={(e) => onSortFieldChange(e.target.value)}
          data-testid="gallery-sort-field"
        >
          {similarPivot ? (
            <option value="__similar__">sameness to {similarPivot}</option>
          ) : null}
          <option value="kappa">κ (curvature)</option>
          <option value="key">{keyField || "key"}</option>
          {fields
            .filter((f) => f.name !== keyField && f.encryption !== "opaque")
            .map((f) => (
              <option key={f.name} value={f.name}>
                {f.name}
              </option>
            ))}
        </select>
        <button
          type="button"
          className="gallery-toolbar-dir"
          onClick={() => onSortDirChange(sortDir === "asc" ? "desc" : "asc")}
          disabled={Boolean(similarPivot)}
          data-testid="gallery-sort-dir"
          title={sortDir === "asc" ? "Ascending — click to flip" : "Descending — click to flip"}
        >
          {sortDir}
        </button>
      </label>

      {similarPivot ? (
        <div
          className="gallery-toolbar-similar"
          data-testid="gallery-similar-chip"
        >
          <span className="gallery-toolbar-similar-label">Similar to</span>
          <span className="gallery-toolbar-similar-pivot">{similarPivot}</span>
          {onClearSimilar ? (
            <button
              type="button"
              className="gallery-toolbar-similar-clear"
              onClick={onClearSimilar}
              data-testid="gallery-similar-clear"
              title="Exit similar mode"
              aria-label="Exit similar mode"
            >
              ✕
            </button>
          ) : null}
        </div>
      ) : null}

      <label className="gallery-toolbar-group">
        <span>Density</span>
        <select
          value={density}
          onChange={(e) => onDensityChange(e.target.value as Density)}
          data-testid="gallery-density"
        >
          <option value="compact">Compact</option>
          <option value="standard">Standard</option>
          <option value="expanded">Expanded</option>
        </select>
      </label>

      <div
        className="gallery-toolbar-chips"
        data-testid="gallery-kappa-filter"
        role="group"
        aria-label="Filter by κ class"
      >
        <span className="gallery-toolbar-chips-label">κ-class:</span>
        <button
          type="button"
          className={`gallery-chip ${kappaFilter === "all" ? "gallery-chip-active" : ""}`}
          onClick={() => onKappaFilterChange("all")}
          data-testid="gallery-kappa-all"
        >
          All
        </button>
        <button
          type="button"
          className={`gallery-chip gallery-chip-drift ${kappaFilter === "drift" ? "gallery-chip-active" : ""}`}
          onClick={() => onKappaFilterChange("drift")}
          data-testid="gallery-kappa-drift"
        >
          Drift
        </button>
        <button
          type="button"
          className={`gallery-chip gallery-chip-bad ${kappaFilter === "bad" ? "gallery-chip-active" : ""}`}
          onClick={() => onKappaFilterChange("bad")}
          data-testid="gallery-kappa-bad"
        >
          Bad
        </button>
      </div>

      <span className="gallery-toolbar-count">
        {visibleCount === totalCount
          ? `${totalCount} cards`
          : `${visibleCount} of ${totalCount} cards`}
      </span>
    </div>
  );
}

/* ── card grid (shared between grouped + flat layouts) ──────────── */

function CardGrid({
  rows,
  keyField,
  coverField,
  bodyFields,
  kappaMap,
  density,
  effectiveSelected,
  onRowClick,
  onRowSelect,
  onRowContextMenu,
  similarPivot,
  samenessByKey,
}: {
  rows: RowMap[];
  keyField: string;
  coverField: string;
  bodyFields: FieldDescriptor[];
  kappaMap: Map<string, number>;
  density: Density;
  effectiveSelected: Set<string>;
  onRowClick?: (key: string, mods: RowClickModifiers) => void;
  onRowSelect: (key: string) => void;
  onRowContextMenu?: (key: string, x: number, y: number) => void;
  similarPivot: string | null;
  samenessByKey: Map<string, number>;
}) {
  return (
    <ul
      className={`gallery-grid gallery-grid-${density}`}
      data-testid="gallery-grid"
      role="list"
    >
      {rows.map((row) => {
        const k = String(row[keyField] ?? "");
        const kappa = kappaMap.get(k);
        const kClass: KappaClass = kappaClass(kappa ?? 0);
        const cover = coverField ? String(row[coverField] ?? "") : "";
        const isSelected = effectiveSelected.has(k);
        const isPivot = similarPivot === k;
        const samenessVal = similarPivot ? samenessByKey.get(k) : undefined;
        return (
          <li key={k}>
            <button
              type="button"
              className={`gallery-card kappa-${kClass} ${isSelected ? "gallery-card-selected" : ""} ${isPivot ? "gallery-card-pivot" : ""}`}
              data-testid="gallery-card"
              data-row-key={k}
              data-pivot={isPivot ? "true" : undefined}
              onClick={(e) => {
                if (onRowClick) {
                  onRowClick(k, {
                    meta: e.metaKey || e.ctrlKey,
                    shift: e.shiftKey,
                    alt: e.altKey,
                  });
                } else {
                  onRowSelect(k);
                }
              }}
              onContextMenu={(e) => {
                if (!onRowContextMenu) return;
                e.preventDefault();
                onRowContextMenu(k, e.clientX, e.clientY);
              }}
            >
              <header className="gallery-card-head">
                <span className="gallery-card-key" title={k}>
                  {isPivot ? <span className="gallery-card-pivot-mark" aria-hidden="true">⌖</span> : null}
                  {k}
                </span>
                {cover ? (
                  <span className="gallery-card-cover">{cover}</span>
                ) : null}
              </header>
              {samenessVal !== undefined ? (
                <div
                  className="gallery-card-sameness"
                  data-testid="gallery-card-sameness"
                  data-sameness={samenessVal.toFixed(4)}
                  title={`Davis sameness against ${similarPivot}: ${samenessVal.toFixed(4)}`}
                >
                  <span className="gallery-card-sameness-label">S</span>
                  <span className="gallery-card-sameness-bar">
                    <span
                      className="gallery-card-sameness-fill"
                      style={{ width: `${Math.round(samenessVal * 100)}%` }}
                    />
                  </span>
                  <span className="gallery-card-sameness-value">
                    {samenessVal.toFixed(3)}
                  </span>
                </div>
              ) : null}
              {bodyFields.length > 0 ? (
                <dl className="gallery-card-body">
                  {bodyFields.map((f) => (
                    <div key={f.name} className="gallery-card-row">
                      <dt>{f.name}</dt>
                      <dd>{renderField(row, f)}</dd>
                    </div>
                  ))}
                </dl>
              ) : null}
              {typeof kappa === "number" ? (
                <footer className="gallery-card-foot">
                  <span className="gallery-card-kappa-label">κ</span>
                  <span className="gallery-card-kappa-value">
                    {kappa.toFixed(2)}
                  </span>
                </footer>
              ) : null}
            </button>
          </li>
        );
      })}
    </ul>
  );
}

function renderField(row: RowMap, field: FieldDescriptor): string {
  const v = row[field.name];
  if (v == null || v === "") return "—";
  if (field.encryption && field.encryption !== "none") {
    return "🔒 encrypted";
  }
  const fmt = defaultFormatFor({ name: field.name, type: field.type as string });
  if (fmt) {
    return formatValue(v, fmt, { kappa: 0 });
  }
  return String(v);
}
