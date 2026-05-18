import { useState } from "react";
import { applyWorkflow, type ApplyProgress } from "../lib/apply-workflow";
import type { SheetsClient } from "../lib/gigi-client";
import {
  WORKFLOW_TEMPLATES,
  type WorkflowTemplate,
} from "../lib/workflow-templates";
import "./WorkflowPicker.css";

export interface WorkflowPickerProps {
  client: SheetsClient;
  /** Called once the workflow is applied — passes the bundle name to open. */
  onApplied: (bundleName: string, template: WorkflowTemplate) => void;
}

type PickerState =
  | { kind: "idle" }
  | { kind: "applying"; id: string; progress: ApplyProgress | null }
  | { kind: "error"; id: string; message: string };

/**
 * "Start with a workflow" card grid. Six templates that match classic
 * Airtable use cases (project tracker, content calendar, CRM, event
 * planning, inventory, recruiting). One click → bundle(s) created,
 * seeded, and the user lands on the default view.
 *
 * Re-using the word "workflow" intentionally — Prism uses it for
 * reconcile workflows over bundles; we use it here for bundle-shaping
 * templates. Both run on the same engine substrate.
 */
export function WorkflowPicker({ client, onApplied }: WorkflowPickerProps) {
  const [state, setState] = useState<PickerState>({ kind: "idle" });

  const apply = async (template: WorkflowTemplate) => {
    setState({ kind: "applying", id: template.id, progress: null });
    try {
      const result = await applyWorkflow(template, client, (p) =>
        setState({ kind: "applying", id: template.id, progress: p }),
      );
      onApplied(result.defaultBundle, template);
      setState({ kind: "idle" });
    } catch (err) {
      setState({
        kind: "error",
        id: template.id,
        message: err instanceof Error ? err.message : String(err),
      });
    }
  };

  return (
    <section className="workflow-picker" data-testid="workflow-picker">
      <header className="workflow-picker-head">
        <h3>Start with a workflow</h3>
        <p>
          Six classic spreadsheet workflows, pre-baked and ready in one click.
          Same substrate as the demos — every workflow ships with the κ overlay,
          field encryption, and Prism analytics already wired in.
        </p>
      </header>
      <ul className="workflow-grid">
        {WORKFLOW_TEMPLATES.map((t) => {
          const isApplying = state.kind === "applying" && state.id === t.id;
          const errored = state.kind === "error" && state.id === t.id;
          const progress =
            isApplying && state.progress
              ? Math.round((100 * state.progress.done) / state.progress.total)
              : 0;
          return (
            <li
              key={t.id}
              className="workflow-card"
              data-testid={`workflow-${t.id}`}
              data-state={isApplying ? "applying" : errored ? "error" : "idle"}
            >
              <div className="workflow-card-head">
                <span className="workflow-icon" aria-hidden="true">
                  {t.icon}
                </span>
                <div className="workflow-card-title">
                  <h4>{t.title}</h4>
                  <small>{t.blurb}</small>
                </div>
              </div>
              <p className="workflow-card-pitch">{t.pitch}</p>
              <div className="workflow-card-better">
                <span className="workflow-card-better-label">GIGI edge</span>
                <span className="workflow-card-better-text">{t.gigiBetter}</span>
              </div>
              <ul className="workflow-card-meta">
                <li>
                  <strong>Bundles:</strong>{" "}
                  {t.bundles.map((b) => b.name).join(", ")}
                </li>
                <li>
                  <strong>Default view:</strong> {t.defaultView}
                </li>
                {t.prismWireUp.length > 0 ? (
                  <li>
                    <strong>Prism:</strong> {t.prismWireUp.join(" · ")}
                  </li>
                ) : null}
              </ul>
              <div className="workflow-card-foot">
                {isApplying ? (
                  <div
                    className="workflow-progress"
                    data-testid={`workflow-progress-${t.id}`}
                  >
                    <div
                      className="workflow-progress-fill"
                      style={{ width: `${progress}%` }}
                    />
                    <span className="workflow-progress-label">
                      {state.progress
                        ? `Loading ${state.progress.bundleName} (${state.progress.done}/${state.progress.total})…`
                        : "Creating bundles…"}
                    </span>
                  </div>
                ) : (
                  <button
                    type="button"
                    className="workflow-apply"
                    onClick={() => apply(t)}
                    disabled={state.kind === "applying"}
                    data-testid={`workflow-apply-${t.id}`}
                  >
                    Use this workflow
                  </button>
                )}
              </div>
              {errored ? (
                <p className="workflow-error" role="alert">
                  {state.message}
                </p>
              ) : null}
            </li>
          );
        })}
      </ul>
    </section>
  );
}
