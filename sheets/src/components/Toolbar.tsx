import type { BundleSchema } from "../lib/gigi-client";
import { TermInfo } from "./TermInfo";
import "./Toolbar.css";

export interface ToolbarProps {
  schema: BundleSchema | null;
  coverField: string;
  onCoverFieldChange: (field: string) => void;
  overlayOn: boolean;
  onOverlayChange: (next: boolean) => void;
  /** Optional summary: counts the rows currently classed bad/warn. */
  anomalyCount?: number;
  driftCount?: number;
  /** When true, the grid only shows rows classed κ-bad. */
  anomaliesOnly?: boolean;
  onAnomaliesOnlyChange?: (next: boolean) => void;
}

export function Toolbar({
  schema,
  coverField,
  onCoverFieldChange,
  overlayOn,
  onOverlayChange,
  anomalyCount,
  driftCount,
  anomaliesOnly = false,
  onAnomaliesOnlyChange,
}: ToolbarProps) {
  const coverChoices = collectCoverChoices(schema);

  return (
    <div className="toolbar" data-testid="toolbar">
      <label className="toolbar-field">
        <span className="toolbar-label">
          Cover
          <TermInfo term="cover" />
        </span>
        <select
          data-testid="cover-field-select"
          value={coverField}
          onChange={(e) => onCoverFieldChange(e.target.value)}
          disabled={coverChoices.length === 0}
        >
          {coverChoices.length === 0 ? (
            <option value="">—</option>
          ) : (
            coverChoices.map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))
          )}
        </select>
      </label>

      <button
        type="button"
        className={`toolbar-chip ${anomaliesOnly ? "toolbar-chip-active" : ""}`}
        onClick={() => onAnomaliesOnlyChange?.(!anomaliesOnly)}
        data-testid="filter-anomalies-only"
        aria-pressed={anomaliesOnly}
        title="Show only rows flagged as κ-bad anomalies"
        disabled={!onAnomaliesOnlyChange}
      >
        <span className="toolbar-chip-dot toolbar-chip-dot-bad" aria-hidden="true" />
        Anomalies only
        {anomalyCount ? (
          <span className="toolbar-chip-count">{anomalyCount}</span>
        ) : null}
      </button>

      <div className="toolbar-overlay-group">
        <button
          type="button"
          className={`toolbar-toggle ${overlayOn ? "toolbar-toggle-on" : ""}`}
          onClick={() => onOverlayChange(!overlayOn)}
          data-testid="overlay-toggle"
          aria-pressed={overlayOn}
          title="Tint rows by κ (curvature): green = healthy, amber = drift, red = anomaly"
        >
          <span className="toolbar-sw" aria-hidden="true" />
          <span>Geometry overlay</span>
        </button>
        <TermInfo term="kappa" label="What does the geometry overlay show?" />
      </div>

      {overlayOn ? (
        <div className="toolbar-overlay-legend" data-testid="overlay-legend" aria-live="polite">
          <span className="toolbar-overlay-swatch toolbar-overlay-swatch-ok" />
          healthy
          <span className="toolbar-overlay-swatch toolbar-overlay-swatch-warn" />
          drift
          <span className="toolbar-overlay-swatch toolbar-overlay-swatch-bad" />
          anomaly
        </div>
      ) : null}

      {(anomalyCount || driftCount) ? (
        <div className="toolbar-stats" data-testid="toolbar-stats">
          {anomalyCount ? (
            <span className="toolbar-stat toolbar-stat-bad" data-testid="anom-count">
              {anomalyCount} {anomalyCount === 1 ? "anomaly" : "anomalies"}
            </span>
          ) : null}
          {driftCount ? (
            <span className="toolbar-stat toolbar-stat-warn" data-testid="drift-count">
              {driftCount} drift
            </span>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

function collectCoverChoices(schema: BundleSchema | null): string[] {
  if (!schema) return [];
  const out: string[] = [];
  for (const f of schema.fiber_fields) {
    if (f.encryption && f.encryption !== "none") continue;
    if (f.type === "categorical" || f.type === "text") out.push(f.name);
  }
  // Include the primary key as a degenerate option (every row is its own cohort).
  const key = schema.base_fields[0]?.name;
  if (key && !out.includes(key)) out.push(key);
  return out;
}
