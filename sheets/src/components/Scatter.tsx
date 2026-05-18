import { useMemo } from "react";
import type { RowMap } from "../lib/gigi-client";
import { kappaClass, type KappaClass } from "../lib/kappa";
import {
  axisTicks,
  computeProjectionDomain,
  coverColor,
  nearestNeighbor,
  projectRows,
  type ProjectedPoint,
  type ViewBox,
} from "../lib/projection";

export interface ScatterProps {
  rows: RowMap[];
  keyField: string;
  coverField: string;
  xField: string;
  yField: string;
  kappaMap: Map<string, number>;
  selectedRowKey: string | null;
  onRowSelect: (key: string) => void;
  /** Pixel width of the SVG viewBox. Default 720. */
  width?: number;
  /** Pixel height of the SVG viewBox. Default 480. */
  height?: number;
}

const DEFAULT_VIEW: ViewBox = {
  width: 720,
  height: 480,
  margin: { l: 54, r: 18, t: 18, b: 38 },
};

const BASE_RADIUS = 4;
const KAPPA_RADIUS_SCALE = 1.3;

export function Scatter({
  rows,
  keyField,
  coverField,
  xField,
  yField,
  kappaMap,
  selectedRowKey,
  onRowSelect,
  width,
  height,
}: ScatterProps) {
  const view: ViewBox = useMemo(
    () => ({
      ...DEFAULT_VIEW,
      width: width ?? DEFAULT_VIEW.width,
      height: height ?? DEFAULT_VIEW.height,
    }),
    [width, height],
  );

  const domain = useMemo(
    () => computeProjectionDomain(rows, xField, yField),
    [rows, xField, yField],
  );

  const points = useMemo<ProjectedPoint[]>(
    () =>
      domain ? projectRows(rows, keyField, xField, yField, domain, view) : [],
    [rows, keyField, xField, yField, domain, view],
  );

  // Map projected point → its row's cover value (for coloring).
  const pointMeta = useMemo(() => {
    const meta = new Map<
      string,
      { cover: string; kappa: number; klass: KappaClass }
    >();
    for (const r of rows) {
      const key = String(r[keyField] ?? "");
      const k = kappaMap.get(key) ?? 0;
      meta.set(key, {
        cover: String(r[coverField] ?? ""),
        kappa: k,
        klass: kappaClass(k),
      });
    }
    return meta;
  }, [rows, keyField, coverField, kappaMap]);

  // Selected point + its nearest neighbor for the transport overlay.
  const selected = selectedRowKey
    ? points.find((p) => p.key === selectedRowKey) ?? null
    : null;
  const peer = selected ? nearestNeighbor(selected, points, selected.key) : null;

  if (!domain) {
    return (
      <div className="scatter-empty" data-testid="scatter-empty">
        <p>No numeric data to plot.</p>
        <small>Pick fields with finite numeric values.</small>
      </div>
    );
  }

  const { l, r, t, b } = view.margin;
  const innerLeft = l;
  const innerRight = view.width - r;
  const innerTop = t;
  const innerBottom = view.height - b;

  const xTicks = axisTicks(domain.x.min, domain.x.max, 5);
  const yTicks = axisTicks(domain.y.min, domain.y.max, 5);

  return (
    <svg
      className="scatter"
      data-testid="scatter"
      viewBox={`0 0 ${view.width} ${view.height}`}
      preserveAspectRatio="xMidYMid meet"
      role="img"
      aria-label={`Scatter of ${xField} vs ${yField}`}
    >
      {/* Grid lines */}
      <g className="scatter-grid">
        {xTicks.map((tv, i) => {
          const x = innerLeft + ((tv - domain.x.min) / (domain.x.max - domain.x.min)) * (innerRight - innerLeft);
          return (
            <line key={`vx-${i}`} x1={x} x2={x} y1={innerTop} y2={innerBottom} stroke="#eef0f4" />
          );
        })}
        {yTicks.map((tv, i) => {
          const y = innerBottom - ((tv - domain.y.min) / (domain.y.max - domain.y.min)) * (innerBottom - innerTop);
          return (
            <line key={`hy-${i}`} x1={innerLeft} x2={innerRight} y1={y} y2={y} stroke="#eef0f4" />
          );
        })}
      </g>

      {/* Axis lines */}
      <line x1={innerLeft} x2={innerRight} y1={innerBottom} y2={innerBottom} stroke="#cbd2dc" />
      <line x1={innerLeft} x2={innerLeft} y1={innerTop} y2={innerBottom} stroke="#cbd2dc" />

      {/* Tick labels */}
      <g className="scatter-ticks" fontSize={10} fill="#8a93a4">
        {xTicks.map((tv, i) => {
          const x = innerLeft + ((tv - domain.x.min) / (domain.x.max - domain.x.min)) * (innerRight - innerLeft);
          return (
            <text key={`tx-${i}`} x={x} y={innerBottom + 14} textAnchor="middle">
              {formatTick(tv)}
            </text>
          );
        })}
        {yTicks.map((tv, i) => {
          const y = innerBottom - ((tv - domain.y.min) / (domain.y.max - domain.y.min)) * (innerBottom - innerTop);
          return (
            <text key={`ty-${i}`} x={innerLeft - 8} y={y + 3} textAnchor="end">
              {formatTick(tv)}
            </text>
          );
        })}
      </g>

      {/* Axis labels */}
      <text
        x={(innerLeft + innerRight) / 2}
        y={view.height - 8}
        textAnchor="middle"
        fontSize={11}
        fill="#4b5565"
        data-testid="x-axis-label"
      >
        {xField}
      </text>
      <text
        x={14}
        y={(innerTop + innerBottom) / 2}
        textAnchor="middle"
        fontSize={11}
        fill="#4b5565"
        transform={`rotate(-90 14 ${(innerTop + innerBottom) / 2})`}
        data-testid="y-axis-label"
      >
        {yField}
      </text>

      {/* Transport overlay: dashed line + θ between selected and nearest peer */}
      {selected && peer ? (
        <g className="scatter-transport" data-testid="transport-overlay">
          <line
            x1={selected.px}
            y1={selected.py}
            x2={peer.px}
            y2={peer.py}
            stroke="#4f46e5"
            strokeWidth={1.4}
            strokeDasharray="4 4"
            opacity={0.75}
          />
          <PeerLabel from={selected} to={peer} />
        </g>
      ) : null}

      {/* Halos for warn/bad rows (drawn before the points so they sit underneath) */}
      <g className="scatter-halos">
        {points.map((p) => {
          const m = pointMeta.get(p.key);
          if (!m || m.klass === "ok") return null;
          const radius = pointRadius(m.kappa) + 6;
          const color = m.klass === "bad" ? "#b91c1c" : "#b45309";
          return (
            <circle
              key={`halo-${p.key}`}
              cx={p.px}
              cy={p.py}
              r={radius}
              fill={color}
              opacity={0.18}
              data-testid={`halo-${p.key}`}
            />
          );
        })}
      </g>

      {/* Points */}
      <g className="scatter-points">
        {points.map((p) => {
          const m = pointMeta.get(p.key);
          const klass: KappaClass = m?.klass ?? "ok";
          const isSelected = selectedRowKey === p.key;
          const radius = pointRadius(m?.kappa ?? 0);
          const fill = coverColor(m?.cover ?? "");
          return (
            <g key={p.key}>
              {isSelected ? (
                <circle
                  cx={p.px}
                  cy={p.py}
                  r={radius + 4}
                  fill="none"
                  stroke={fill}
                  strokeWidth={2}
                  data-testid={`ring-${p.key}`}
                />
              ) : null}
              <circle
                cx={p.px}
                cy={p.py}
                r={radius}
                fill={fill}
                style={{ cursor: "pointer" }}
                onClick={() => onRowSelect(p.key)}
                data-testid={`point-${p.key}`}
                data-cover={m?.cover ?? ""}
                data-kappa-class={klass}
                data-kappa={(m?.kappa ?? 0).toFixed(3)}
              >
                <title>
                  {p.key} · {xField}={formatTick(p.x)} · {yField}={formatTick(p.y)} · κ={(m?.kappa ?? 0).toFixed(2)}
                </title>
              </circle>
            </g>
          );
        })}
      </g>
    </svg>
  );
}

function PeerLabel({
  from,
  to,
}: {
  from: ProjectedPoint;
  to: ProjectedPoint;
}) {
  const midx = (from.px + to.px) / 2;
  const midy = (from.py + to.py) / 2;
  return (
    <g data-testid="peer-label">
      <rect
        x={midx - 36}
        y={midy - 10}
        width={72}
        height={18}
        rx={9}
        fill="#0b1220"
      />
      <text
        x={midx}
        y={midy + 3}
        textAnchor="middle"
        fontSize={10}
        fill="#fff"
        fontFamily="JetBrains Mono, ui-monospace, monospace"
      >
        {to.key}
      </text>
    </g>
  );
}

function pointRadius(kappa: number): number {
  return BASE_RADIUS + Math.min(5, Math.max(0, kappa)) * KAPPA_RADIUS_SCALE;
}

function formatTick(v: number): string {
  if (Math.abs(v) >= 1000) return v.toFixed(0);
  if (Math.abs(v) >= 100) return v.toFixed(1);
  return v.toFixed(2).replace(/\.?0+$/, "");
}
