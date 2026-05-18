import { useMemo, useState } from "react";
import type { BundleSchema, RowMap } from "../lib/gigi-client";
import { kappaClass, type KappaClass } from "../lib/kappa";
import "./Kanban.css";

export interface KanbanProps {
  schema: BundleSchema | null;
  rows: RowMap[];
  kappaMap: Map<string, number>;
  coverField: string;
  onRowSelect: (key: string) => void;
}

/**
 * Status-board view. Defaults to grouping rows by κ class (healthy /
 * drift / anomaly) — the universal "is this row OK?" axis that works on
 * every bundle. The user can switch to grouping by any plain categorical
 * field via the dropdown.
 */
export function Kanban({
  schema,
  rows,
  kappaMap,
  coverField,
  onRowSelect,
}: KanbanProps) {
  const [groupBy, setGroupBy] = useState<string>("__kappa__");

  if (!schema) {
    return (
      <div className="kanban kanban-empty" data-testid="kanban-empty">
        <p>Loading bundle…</p>
      </div>
    );
  }

  const keyField = schema.base_fields[0]?.name ?? "";
  const categoricalChoices = [
    ...schema.fiber_fields,
    ...schema.base_fields,
  ]
    .filter(
      (f) =>
        (f.type === "categorical" || f.type === "text") &&
        (!f.encryption || f.encryption === "none"),
    )
    .map((f) => f.name);

  return (
    <div className="kanban" data-testid="kanban">
      <header className="kanban-toolbar">
        <label className="kanban-groupby-label">
          Group by
          <select
            value={groupBy}
            onChange={(e) => setGroupBy(e.target.value)}
            data-testid="kanban-groupby"
            className="kanban-groupby"
          >
            <option value="__kappa__">κ class (healthy / drift / anomaly)</option>
            {categoricalChoices.map((f) => (
              <option key={f} value={f}>
                {f}
              </option>
            ))}
          </select>
        </label>
        <span className="kanban-hint">
          Click any card to focus the row in the inspector. Default grouping
          (<span className="mono">κ class</span>) uses cover ={" "}
          <span className="mono">{coverField || "(none)"}</span>.
        </span>
      </header>
      <KanbanBoard
        rows={rows}
        keyField={keyField}
        kappaMap={kappaMap}
        groupBy={groupBy}
        onRowSelect={onRowSelect}
      />
    </div>
  );
}

interface BoardProps {
  rows: RowMap[];
  keyField: string;
  kappaMap: Map<string, number>;
  groupBy: string;
  onRowSelect: (key: string) => void;
}

function KanbanBoard({
  rows,
  keyField,
  kappaMap,
  groupBy,
  onRowSelect,
}: BoardProps) {
  // Columns: when grouping by κ class, we use a fixed ordered set
  // (ok→warn→bad) so the board reads "healthy → drift → anomaly" L to R.
  // When grouping by a real field, columns are sorted by descending size.
  const groups = useMemo(() => {
    const map = new Map<string, RowMap[]>();
    for (const r of rows) {
      let group: string;
      if (groupBy === "__kappa__") {
        const k = kappaMap.get(String(r[keyField] ?? "")) ?? 0;
        group = kappaClass(k);
      } else {
        group = String(r[groupBy] ?? "—");
      }
      const arr = map.get(group);
      if (arr) arr.push(r);
      else map.set(group, [r]);
    }
    return map;
  }, [rows, kappaMap, keyField, groupBy]);

  const orderedKeys: string[] = useMemo(() => {
    if (groupBy === "__kappa__") {
      const fixed: KappaClass[] = ["ok", "warn", "bad"];
      return fixed.filter((k) => groups.has(k));
    }
    return Array.from(groups.keys()).sort(
      (a, b) => (groups.get(b)?.length ?? 0) - (groups.get(a)?.length ?? 0),
    );
  }, [groups, groupBy]);

  const columnLabel = (k: string): string => {
    if (groupBy !== "__kappa__") return k;
    if (k === "ok") return "Healthy";
    if (k === "warn") return "Drift";
    if (k === "bad") return "Anomaly";
    return k;
  };

  return (
    <div className="kanban-board">
      {orderedKeys.map((k) => {
        const items = groups.get(k) ?? [];
        return (
          <section
            key={k}
            className={`kanban-col kanban-col-${k}`}
            data-testid={`kanban-col-${k}`}
            data-group={k}
          >
            <header className="kanban-col-head">
              <span className={`kanban-col-dot kanban-col-dot-${k}`} aria-hidden="true" />
              <h4>{columnLabel(k)}</h4>
              <span
                className="kanban-col-count"
                data-testid={`kanban-col-${k}-count`}
              >
                {items.length}
              </span>
            </header>
            <ul className="kanban-cards">
              {items.map((r) => {
                const rowKey = String(r[keyField] ?? "");
                const kappa = kappaMap.get(rowKey);
                const klass = kappa !== undefined ? kappaClass(kappa) : "ok";
                return (
                  <li key={rowKey}>
                    <button
                      type="button"
                      className="kanban-card"
                      onClick={() => onRowSelect(rowKey)}
                      data-testid={`kanban-card-${rowKey}`}
                      data-kappa-class={klass}
                    >
                      <span className="kanban-card-key">{rowKey}</span>
                      <span
                        className={`kanban-card-kappa kanban-card-kappa-${klass}`}
                      >
                        κ {kappa !== undefined ? kappa.toFixed(2) : "—"}
                      </span>
                    </button>
                  </li>
                );
              })}
              {items.length === 0 ? (
                <li className="kanban-empty-col">No rows.</li>
              ) : null}
            </ul>
          </section>
        );
      })}
    </div>
  );
}
