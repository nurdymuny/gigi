import { useEffect, useMemo, useState } from "react";
import type { BundleSchema, RowMap } from "../lib/gigi-client";
import { kappaClass, numericFiberFields } from "../lib/kappa";
import { coverColor } from "../lib/projection";
import { Scatter } from "./Scatter";
import { TermInfo } from "./TermInfo";
import "./Geometry.css";

export interface GeometryProps {
  schema: BundleSchema | null;
  rows: RowMap[];
  kappaMap: Map<string, number>;
  coverField: string;
  selectedRowKey: string | null;
  onRowSelect: (key: string) => void;
}

export function Geometry({
  schema,
  rows,
  kappaMap,
  coverField,
  selectedRowKey,
  onRowSelect,
}: GeometryProps) {
  const numericFields = useMemo(
    () => (schema ? numericFiberFields(schema) : []),
    [schema],
  );

  const [xField, setXField] = useState<string>("");
  const [yField, setYField] = useState<string>("");
  const [sidebarOpen, setSidebarOpen] = useState<boolean>(true);

  // Pick sensible defaults the first time numericFields is non-empty.
  useEffect(() => {
    if (numericFields.length === 0) return;
    if (!xField || !numericFields.includes(xField)) {
      setXField(numericFields[0]);
    }
    if (!yField || !numericFields.includes(yField)) {
      // Prefer a second distinct field, but fall back to the same field
      // (1D view) when the bundle only has one numeric column.
      setYField(numericFields[1] ?? numericFields[0]);
    }
  }, [numericFields, xField, yField]);

  const keyField = schema?.base_fields[0]?.name ?? "";

  const coverStats = useMemo(
    () => buildCoverStats(rows, keyField, coverField, kappaMap),
    [rows, keyField, coverField, kappaMap],
  );

  if (!schema) {
    return (
      <div className="geometry geometry-empty" data-testid="geometry-empty">
        <p>Loading bundle…</p>
      </div>
    );
  }

  if (numericFields.length === 0) {
    return (
      <div className="geometry geometry-empty" data-testid="geometry-empty">
        <h3>No numeric fields yet</h3>
        <p>
          The Geometry view plots rows against two numeric fibers, then colors them
          by your <strong>{coverField || "cover"}</strong> field. Once your bundle
          has at least one numeric column we can render it here.
        </p>
        <p className="geometry-empty-hint">
          Open <em>Schema</em> from the top bar to add a numeric field, or import a
          CSV with numeric columns.
        </p>
      </div>
    );
  }

  return (
    <div className="geometry" data-testid="geometry">
      <header className="geometry-toolbar">
        <FieldSelect
          label="X axis"
          value={xField}
          options={numericFields}
          onChange={setXField}
          testid="x-field-select"
        />
        <FieldSelect
          label="Y axis"
          value={yField}
          options={numericFields}
          onChange={setYField}
          testid="y-field-select"
        />
        <span className="geometry-hint">
          color by <span className="mono">{coverField}</span>
          <TermInfo term="cover" />
          · size by κ
          <TermInfo term="kappa" />
          · dashed line = TRANSPORT
          <TermInfo term="transport" />
          to nearest peer
        </span>
        <button
          type="button"
          className="geometry-sidebar-toggle"
          data-testid="geometry-sidebar-toggle"
          aria-pressed={sidebarOpen}
          onClick={() => setSidebarOpen((v) => !v)}
          title={sidebarOpen ? "Hide cover stats" : "Show cover stats"}
        >
          {sidebarOpen ? "Hide stats" : "Show stats"}
        </button>
      </header>
      <div
        className={`geometry-body ${sidebarOpen ? "" : "geometry-body-no-sidebar"}`}
      >
        <div className="geometry-plot">
          {xField && yField ? (
            <Scatter
              rows={rows}
              keyField={keyField}
              coverField={coverField}
              xField={xField}
              yField={yField}
              kappaMap={kappaMap}
              selectedRowKey={selectedRowKey}
              onRowSelect={onRowSelect}
            />
          ) : null}
        </div>
        {sidebarOpen ? (
        <aside className="geometry-sidebar" data-testid="geometry-sidebar">
          <h4>Cover stats</h4>
          {coverStats.length === 0 ? (
            <p className="geometry-sidebar-empty">No cover groups in view.</p>
          ) : (
            <ul className="cover-list">
              {coverStats.map((s) => (
                <li
                  key={s.label}
                  className="cover-row"
                  data-testid={`cover-${s.label}`}
                >
                  <span
                    className="cover-swatch"
                    style={{ background: coverColor(s.label) }}
                    aria-hidden="true"
                  />
                  <span className="cover-label">{s.label}</span>
                  <span className="cover-counts">
                    <span className="cover-count" data-testid={`cover-${s.label}-size`}>
                      {s.size}
                    </span>
                    {s.anomalies > 0 ? (
                      <span
                        className="cover-pill cover-pill-bad"
                        data-testid={`cover-${s.label}-anom`}
                      >
                        {s.anomalies}
                      </span>
                    ) : null}
                    {s.drifts > 0 ? (
                      <span
                        className="cover-pill cover-pill-warn"
                        data-testid={`cover-${s.label}-drift`}
                      >
                        {s.drifts}
                      </span>
                    ) : null}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </aside>
        ) : null}
      </div>
    </div>
  );
}

function FieldSelect({
  label,
  value,
  options,
  onChange,
  testid,
}: {
  label: string;
  value: string;
  options: string[];
  onChange: (v: string) => void;
  testid: string;
}) {
  return (
    <label className="geometry-axis-select">
      <span className="geometry-axis-label">{label}</span>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        data-testid={testid}
      >
        {options.map((opt) => (
          <option key={opt} value={opt}>
            {opt}
          </option>
        ))}
      </select>
    </label>
  );
}

interface CoverStat {
  label: string;
  size: number;
  anomalies: number;
  drifts: number;
}

function buildCoverStats(
  rows: RowMap[],
  keyField: string,
  coverField: string,
  kappaMap: Map<string, number>,
): CoverStat[] {
  if (!keyField || !coverField) return [];
  const map = new Map<string, CoverStat>();
  for (const r of rows) {
    const label = String(r[coverField] ?? "—");
    let s = map.get(label);
    if (!s) {
      s = { label, size: 0, anomalies: 0, drifts: 0 };
      map.set(label, s);
    }
    s.size += 1;
    const k = kappaMap.get(String(r[keyField] ?? "")) ?? 0;
    const klass = kappaClass(k);
    if (klass === "bad") s.anomalies += 1;
    else if (klass === "warn") s.drifts += 1;
  }
  return Array.from(map.values()).sort((a, b) => b.size - a.size);
}
