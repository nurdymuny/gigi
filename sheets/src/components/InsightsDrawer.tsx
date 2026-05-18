import { useEffect, useMemo } from "react";
import { computeInsights, type Insight, type InsightsInput } from "../lib/insights";
import "./InsightsDrawer.css";

export interface InsightsDrawerProps extends InsightsInput {
  open: boolean;
  onClose: () => void;
  /** Called when the user clicks the "Copy GQL" button on an insight. */
  onCopyGql?: (gql: string) => void;
}

export function InsightsDrawer({
  open,
  onClose,
  onCopyGql,
  ...input
}: InsightsDrawerProps) {
  const insights = useMemo<Insight[]>(
    () => computeInsights(input),
    [input.bundle, input.schema, input.rows, input.kappaMap, input.coverField, input.meanCurvature],
  );

  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  const copy = (gql: string) => {
    navigator.clipboard.writeText(gql).then(() => onCopyGql?.(gql));
  };

  return (
    <>
      <div
        className="insights-drawer-bg"
        data-testid="insights-drawer-bg"
        onClick={onClose}
      />
      <aside
        className="insights-drawer"
        data-testid="insights-drawer"
        role="dialog"
        aria-label="Insights"
      >
        <header className="insights-drawer-head">
          <div>
            <h3>Insights</h3>
            <p className="insights-drawer-sub">
              <code>{input.bundle}</code> · {insights.length} observation
              {insights.length === 1 ? "" : "s"}
            </p>
          </div>
          <button
            type="button"
            className="insights-drawer-close"
            onClick={onClose}
            aria-label="Close"
          >
            ✕
          </button>
        </header>
        {insights.length === 0 ? (
          <p className="insights-drawer-empty" data-testid="insights-drawer-empty">
            Nothing notable yet. Try editing a row or switching the cover field.
          </p>
        ) : (
          <ul className="insights-list" data-testid="insights-list">
            {insights.map((it) => (
              <li
                key={it.id}
                className="insight"
                data-testid={`insight-${it.id}`}
                data-tag={it.tag}
              >
                <span className={`insight-tag insight-tag-${it.tag}`}>
                  {tagLabel(it.tag)}
                </span>
                <p className="insight-body">{it.body}</p>
                {it.gql ? (
                  <div className="insight-gql-wrap">
                    <code className="insight-gql">{it.gql}</code>
                    <button
                      type="button"
                      className="insight-copy"
                      onClick={() => copy(it.gql!)}
                      data-testid={`insight-copy-${it.id}`}
                      aria-label="Copy GQL"
                    >
                      Copy
                    </button>
                  </div>
                ) : null}
              </li>
            ))}
          </ul>
        )}
      </aside>
    </>
  );
}

function tagLabel(t: Insight["tag"]): string {
  if (t === "bad") return "anomaly";
  if (t === "warn") return "watch";
  if (t === "geo") return "geometry";
  return "info";
}
