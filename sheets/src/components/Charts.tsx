import { useMemo, useState } from "react";
import type { BundleSchema, RowMap } from "../lib/gigi-client";
import { kappaClass, numericFiberFields } from "../lib/kappa";
import "./Charts.css";

export interface ChartsProps {
  schema: BundleSchema | null;
  rows: RowMap[];
  kappaMap: Map<string, number>;
  coverField: string;
}

/**
 * Charts view — four small-multiples cards offering bundle-level overviews:
 *   1. Counts by cover field (horizontal bar)
 *   2. Histogram of a chosen numeric fiber
 *   3. κ-per-row (vertical bar)
 *   4. Confidence vs κ (scatter)
 *
 * Pure SVG, no D3, no chart library. Reuses the kappa palette for visual
 * consistency with the grid + geometry views.
 */
export function Charts({ schema, rows, kappaMap, coverField }: ChartsProps) {
  const numeric = useMemo(
    () => (schema ? numericFiberFields(schema) : []),
    [schema],
  );
  const [histField, setHistField] = useState<string>(numeric[0] ?? "");
  // Snap histField if the schema changes.
  useMemo(() => {
    if (numeric.length === 0) return;
    if (!histField || !numeric.includes(histField)) setHistField(numeric[0]);
  }, [numeric, histField]);

  if (!schema) {
    return (
      <div className="charts charts-empty" data-testid="charts-empty">
        <p>Loading bundle…</p>
      </div>
    );
  }

  const keyField = schema.base_fields[0]?.name ?? "";

  return (
    <div className="charts" data-testid="charts">
      <div className="charts-grid">
        <CoverCountsCard rows={rows} coverField={coverField} />
        <HistogramCard
          rows={rows}
          field={histField}
          options={numeric}
          onChange={setHistField}
        />
        <KappaByRowCard rows={rows} keyField={keyField} kappaMap={kappaMap} />
        <ConfidenceCard kappaMap={kappaMap} />
      </div>
    </div>
  );
}

/* ── Card 1: counts by cover field ───────────────────────────────────── */
function CoverCountsCard({
  rows,
  coverField,
}: {
  rows: RowMap[];
  coverField: string;
}) {
  const counts = useMemo(() => {
    const m = new Map<string, number>();
    if (!coverField) return [] as Array<{ label: string; n: number }>;
    for (const r of rows) {
      const k = String(r[coverField] ?? "—");
      m.set(k, (m.get(k) ?? 0) + 1);
    }
    return Array.from(m, ([label, n]) => ({ label, n })).sort((a, b) => b.n - a.n);
  }, [rows, coverField]);

  const max = counts[0]?.n ?? 1;

  return (
    <div className="charts-card" data-testid="charts-cover-counts">
      <header>
        <h3>Rows by {coverField || "cover"}</h3>
        <p>Count of records per cover value.</p>
      </header>
      {counts.length === 0 ? (
        <p className="charts-empty-line">No cover field selected.</p>
      ) : (
        <ul className="charts-bars">
          {counts.slice(0, 12).map((c) => (
            <li key={c.label} data-testid={`charts-bar-${c.label}`}>
              <span className="charts-bar-label">{c.label}</span>
              <span className="charts-bar-track">
                <span
                  className="charts-bar-fill"
                  style={{ width: `${(100 * c.n) / max}%` }}
                />
              </span>
              <span className="charts-bar-count">{c.n}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

/* ── Card 2: histogram of one numeric field ──────────────────────────── */
function HistogramCard({
  rows,
  field,
  options,
  onChange,
}: {
  rows: RowMap[];
  field: string;
  options: string[];
  onChange: (next: string) => void;
}) {
  const bins = useMemo(() => buildHistogram(rows, field, 16), [rows, field]);

  return (
    <div className="charts-card" data-testid="charts-histogram">
      <header>
        <h3>
          Distribution of{" "}
          <select
            value={field}
            onChange={(e) => onChange(e.target.value)}
            data-testid="charts-hist-field"
            disabled={options.length === 0}
          >
            {options.length === 0 ? (
              <option value="">—</option>
            ) : (
              options.map((o) => (
                <option key={o} value={o}>
                  {o}
                </option>
              ))
            )}
          </select>
        </h3>
        <p>Histogram across all rows in this view.</p>
      </header>
      {bins.length === 0 || !field ? (
        <p className="charts-empty-line">No numeric fields available.</p>
      ) : (
        <svg className="charts-svg" viewBox="0 0 400 160" preserveAspectRatio="none">
          {bins.map((b, i) => {
            const maxN = Math.max(...bins.map((x) => x.n), 1);
            const x = (i * 400) / bins.length;
            const w = 400 / bins.length - 2;
            const h = (140 * b.n) / maxN;
            const y = 150 - h;
            return (
              <rect
                key={i}
                x={x}
                y={y}
                width={w}
                height={h}
                fill="#4f46e5"
                opacity={0.85}
                data-testid={`charts-hist-bin-${i}`}
              >
                <title>
                  [{b.lo.toFixed(2)} – {b.hi.toFixed(2)}]: {b.n}
                </title>
              </rect>
            );
          })}
          <line x1="0" y1="150" x2="400" y2="150" stroke="#e6e8ee" strokeWidth="1" />
        </svg>
      )}
    </div>
  );
}

/* ── Card 3: κ per row ───────────────────────────────────────────────── */
function KappaByRowCard({
  rows,
  keyField,
  kappaMap,
}: {
  rows: RowMap[];
  keyField: string;
  kappaMap: Map<string, number>;
}) {
  const data = useMemo(() => {
    if (!keyField) return [] as Array<{ key: string; k: number }>;
    return rows
      .map((r) => ({
        key: String(r[keyField] ?? ""),
        k: kappaMap.get(String(r[keyField] ?? "")) ?? 0,
      }))
      .sort((a, b) => b.k - a.k);
  }, [rows, keyField, kappaMap]);

  const max = data[0]?.k ?? 0;
  const cap = Math.max(max, 0.1);

  return (
    <div className="charts-card" data-testid="charts-kappa-by-row">
      <header>
        <h3>κ by row</h3>
        <p>Top-20 most curved rows. Red = anomaly · amber = drift · green = healthy.</p>
      </header>
      {data.length === 0 ? (
        <p className="charts-empty-line">
          No κ data available — pick a cover field with multiple peers first.
        </p>
      ) : (
        <svg className="charts-svg" viewBox="0 0 400 160" preserveAspectRatio="none">
          {data.slice(0, 20).map((d, i) => {
            const x = (i * 400) / 20;
            const w = 400 / 20 - 2;
            const h = (140 * d.k) / cap;
            const y = 150 - h;
            const klass = kappaClass(d.k);
            const fill =
              klass === "bad" ? "#b91c1c" : klass === "warn" ? "#b45309" : "#047857";
            return (
              <rect
                key={d.key}
                x={x}
                y={y}
                width={w}
                height={h}
                fill={fill}
                opacity={0.85}
                data-testid={`charts-kappa-bar-${d.key}`}
              >
                <title>
                  {d.key}: κ = {d.k.toFixed(2)} ({klass})
                </title>
              </rect>
            );
          })}
          <line x1="0" y1="150" x2="400" y2="150" stroke="#e6e8ee" strokeWidth="1" />
        </svg>
      )}
    </div>
  );
}

/* ── Card 4: confidence vs κ scatter ─────────────────────────────────── */
function ConfidenceCard({ kappaMap }: { kappaMap: Map<string, number> }) {
  const points = useMemo(() => {
    return Array.from(kappaMap.entries()).map(([key, k]) => ({
      key,
      k,
      conf: 1 / (1 + k),
    }));
  }, [kappaMap]);

  return (
    <div className="charts-card" data-testid="charts-conf-kappa">
      <header>
        <h3>Confidence vs κ</h3>
        <p>
          conf = 1/(1+κ). Points near the bottom-right of the curve need
          attention.
        </p>
      </header>
      {points.length === 0 ? (
        <p className="charts-empty-line">No κ data yet.</p>
      ) : (
        <svg className="charts-svg" viewBox="0 0 400 160" preserveAspectRatio="none">
          {/* axes */}
          <line x1="30" y1="140" x2="390" y2="140" stroke="#e6e8ee" strokeWidth="1" />
          <line x1="30" y1="10" x2="30" y2="140" stroke="#e6e8ee" strokeWidth="1" />
          {/* The hyperbolic confidence curve */}
          <path
            d={confidencePath(points)}
            stroke="#cfd2ff"
            strokeWidth="1.5"
            fill="none"
          />
          {points.map((p) => {
            const klass = kappaClass(p.k);
            const x = 30 + Math.min(p.k, 5) * (360 / 5);
            const y = 140 - p.conf * 130;
            const fill =
              klass === "bad" ? "#b91c1c" : klass === "warn" ? "#b45309" : "#047857";
            return (
              <circle
                key={p.key}
                cx={x}
                cy={y}
                r="3"
                fill={fill}
                fillOpacity="0.85"
                stroke="#fff"
                strokeWidth="0.5"
                data-testid={`charts-conf-point-${p.key}`}
              >
                <title>
                  {p.key}: κ = {p.k.toFixed(2)}, conf = {p.conf.toFixed(2)}
                </title>
              </circle>
            );
          })}
          {/* tick labels */}
          <text x="30" y="155" fontSize="9" fill="#8a93a4">κ=0</text>
          <text x="370" y="155" fontSize="9" fill="#8a93a4">5+</text>
          <text x="6" y="14" fontSize="9" fill="#8a93a4">1.0</text>
          <text x="6" y="144" fontSize="9" fill="#8a93a4">0</text>
        </svg>
      )}
    </div>
  );
}

/* ── helpers ─────────────────────────────────────────────────────────── */
function buildHistogram(
  rows: RowMap[],
  field: string,
  binCount: number,
): Array<{ lo: number; hi: number; n: number }> {
  if (!field) return [];
  const values = rows
    .map((r) => r[field])
    .filter((v): v is number => typeof v === "number" && Number.isFinite(v));
  if (values.length === 0) return [];
  const lo = Math.min(...values);
  const hi = Math.max(...values);
  if (!Number.isFinite(lo) || !Number.isFinite(hi) || lo === hi) {
    return [{ lo, hi, n: values.length }];
  }
  const step = (hi - lo) / binCount;
  const bins = Array.from({ length: binCount }, (_, i) => ({
    lo: lo + i * step,
    hi: lo + (i + 1) * step,
    n: 0,
  }));
  for (const v of values) {
    const idx = Math.min(binCount - 1, Math.floor((v - lo) / step));
    bins[idx].n += 1;
  }
  return bins;
}

function confidencePath(_points: { k: number; conf: number }[]): string {
  // Render the theoretical curve conf = 1/(1+κ) for κ ∈ [0, 5] so users
  // can see where each row sits relative to the expected relationship.
  const segs: string[] = [];
  for (let i = 0; i <= 50; i++) {
    const k = (i * 5) / 50;
    const conf = 1 / (1 + k);
    const x = 30 + k * (360 / 5);
    const y = 140 - conf * 130;
    segs.push(`${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`);
  }
  return segs.join(" ");
}
