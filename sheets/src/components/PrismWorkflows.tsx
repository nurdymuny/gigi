import { useState } from "react";
import type { BundleSchema, RowMap, SheetsClient } from "../lib/gigi-client";
import {
  PRISM_WORKFLOWS,
  type WorkflowDef,
  type WorkflowResult,
} from "../lib/prism-workflows";
import type { PrismCredits } from "../lib/use-prism-credits";
import { PrismResultModal } from "./PrismResultModal";
import { PrismUpsellModal } from "./PrismUpsellModal";
import "./PrismWorkflows.css";

export interface PrismWorkflowsDrawerProps {
  open: boolean;
  onClose: () => void;
  schema: BundleSchema | null;
  rows: RowMap[];
  kappaMap: Map<string, number>;
  credits: PrismCredits;
  /** Opens the sign-in modal (for the "I have a Prism account" path). */
  onSignIn: () => void;
  /** Names of other bundles currently open as tabs — Books picks side B
   *  from here. Excludes the active bundle. */
  otherOpenTabs?: string[];
  /** Used by Books to fetch the secondary bundle's rows when picked, and
   *  by the result modal to save the artifact as a new bundle. */
  client?: SheetsClient;
  /** Active bundle name — used to slug the saved artifact bundle. */
  sourceBundle?: string;
  /** Open the saved-artifact bundle as a new tab. */
  onOpenSavedBundle?: (name: string) => void;
}

/**
 * Right-edge drawer listing Prism workflows the user can run on the
 * current bundle. Visually distinct from GIGI Sheets — Prism's navy +
 * blue palette so the upsell story reads as "this is a different
 * product you can also use."
 */
export function PrismWorkflowsDrawer({
  open,
  onClose,
  schema,
  rows,
  kappaMap,
  credits,
  onSignIn,
  otherOpenTabs = [],
  client,
  sourceBundle,
  onOpenSavedBundle,
}: PrismWorkflowsDrawerProps) {
  const [result, setResult] = useState<WorkflowResult | null>(null);
  const [upsellOpen, setUpsellOpen] = useState(false);
  const [booksPicker, setBooksPicker] = useState<{ open: boolean; chosen: string } | null>(null);
  const [busy, setBusy] = useState(false);

  if (!open && !result && !upsellOpen && !booksPicker?.open) return null;

  const showResult = (r: WorkflowResult) => {
    credits.consume();
    setResult(r);
  };

  const runBooks = async (side2: string) => {
    if (!schema || !client) return;
    setBusy(true);
    try {
      const [secondarySchema, section] = await Promise.all([
        client.schema(side2),
        client.section(side2, { limit: 1000 }),
      ]);
      const secondaryRows = section.rows ?? [];
      const w = PRISM_WORKFLOWS.find((x) => x.id === "books")!;
      const r = w.run({
        schema,
        rows,
        kappaMap,
        secondaryRows,
        secondaryName: secondarySchema.name,
      });
      setBooksPicker(null);
      showResult(r);
    } catch (err) {
      setResult({
        workflow: "books",
        headline: "Couldn't load the second bundle",
        stats: [],
        table: { columns: [], rows: [] },
        findings: [err instanceof Error ? err.message : String(err)],
        method:
          "Books needs to read the second bundle's rows to reconcile. Make sure it's reachable on the engine.",
      });
      setBooksPicker(null);
    } finally {
      setBusy(false);
    }
  };

  const runWorkflow = (w: WorkflowDef) => {
    if (!schema) return;
    if (!credits.canRun) {
      setUpsellOpen(true);
      return;
    }
    if (w.id === "books") {
      if (otherOpenTabs.length === 0) {
        setResult({
          workflow: "books",
          headline: "Open a second bundle to reconcile against",
          stats: [],
          table: { columns: [], rows: [] },
          findings: [
            "Books needs a second bundle. Open one in a tab (Sidebar → click another bundle → it opens in a new tab) and run Books again.",
            "Try the Chase + QuickBooks demo pair — they're shaped to find planted breaks.",
          ],
          method:
            "Production Prism Books matches on the shared key, then compares each matched pair column-by-column. Orphans (only-in-A or only-in-B) and conflicts are surfaced separately.",
        });
        return;
      }
      // Auto-pick if there's exactly one other tab open; otherwise open the picker.
      if (otherOpenTabs.length === 1) {
        void runBooks(otherOpenTabs[0]);
      } else {
        setBooksPicker({ open: true, chosen: otherOpenTabs[0] });
      }
      return;
    }
    const r = w.run({ schema, rows, kappaMap, secondaryRows: null });
    showResult(r);
  };

  return (
    <>
      {open ? (
        <div
          className="prism-drawer-bg"
          data-testid="prism-drawer-bg"
          onClick={(e) => {
            if (e.target === e.currentTarget) onClose();
          }}
        >
          <aside
            className="prism-drawer"
            data-testid="prism-drawer"
            role="dialog"
            aria-label="Prism workflows"
          >
            <header className="prism-drawer-head">
              <div>
                <div className="prism-brand">
                  <span className="prism-brand-mark" aria-hidden="true">◇</span>
                  PRISM
                  <span className="prism-brand-tag">workflows</span>
                </div>
                <h2>Run a Prism module on this bundle</h2>
                <p className="prism-drawer-sub">
                  Prism is the payment-reconciliation sibling of GIGI. These
                  workflows are normally enterprise-licensed — you've got{" "}
                  <strong>
                    {credits.unlimited
                      ? "unlimited runs"
                      : `${credits.remaining} of ${credits.limit} free runs`}
                  </strong>{" "}
                  while you try it.
                </p>
              </div>
              <button
                type="button"
                className="prism-drawer-close"
                onClick={onClose}
                aria-label="Close"
                data-testid="prism-drawer-close"
              >
                ×
              </button>
            </header>

            <div className="prism-drawer-body">
              {PRISM_WORKFLOWS.map((w) => {
                const ok = w.eligible(schema);
                const isBooks = w.id === "books";
                const booksReady = isBooks && otherOpenTabs.length > 0;
                return (
                  <article
                    key={w.id}
                    className={`prism-card ${ok ? "" : "prism-card-disabled"}`}
                    data-testid={`prism-workflow-${w.id}`}
                  >
                    <header className="prism-card-head">
                      <h4>{w.title}</h4>
                      <span className="prism-card-badge">Prism module</span>
                    </header>
                    <p className="prism-card-blurb">{w.blurb}</p>
                    <p className="prism-card-hint">
                      {isBooks
                        ? booksReady
                          ? `Will reconcile against: ${otherOpenTabs.length === 1 ? otherOpenTabs[0] : `${otherOpenTabs.length} other open tabs`}`
                          : "Open another bundle in a tab first — Books needs a side B."
                        : w.inputHint}
                    </p>
                    <button
                      type="button"
                      className="prism-card-run"
                      onClick={() => runWorkflow(w)}
                      disabled={!ok || busy}
                      data-testid={`prism-run-${w.id}`}
                    >
                      {ok
                        ? credits.canRun
                          ? busy && isBooks
                            ? "Reconciling…"
                            : "Run"
                          : "Sign in to run"
                        : "Not applicable to this bundle"}
                    </button>
                  </article>
                );
              })}
            </div>

            <footer className="prism-drawer-foot">
              <div className="prism-drawer-counter" data-testid="prism-credits">
                {credits.unlimited ? (
                  <>
                    <span className="prism-dot prism-dot-active" />
                    Prism subscriber — unlimited runs
                  </>
                ) : credits.remaining > 0 ? (
                  <>
                    <span className="prism-dot" />
                    {credits.remaining} of {credits.limit} free runs left
                  </>
                ) : (
                  <>
                    <span className="prism-dot prism-dot-empty" />
                    Free runs used — sign in to a Prism account for more
                  </>
                )}
              </div>
              <div className="prism-drawer-foot-links">
                {credits.used > 0 && !credits.unlimited ? (
                  <button
                    type="button"
                    className="prism-drawer-foot-reset"
                    onClick={() => credits.reset()}
                    data-testid="prism-drawer-reset"
                    title="Reset the free-run counter back to 3"
                  >
                    Reset trial
                  </button>
                ) : null}
                <a
                  href="https://useprism.sh"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="prism-drawer-foot-link"
                >
                  What is Prism? ↗
                </a>
              </div>
            </footer>
          </aside>
        </div>
      ) : null}

      {/* Books picker — only shows when 2+ other tabs are open. */}
      {booksPicker?.open ? (
        <div
          className="prism-drawer-bg"
          data-testid="prism-books-picker-bg"
          onClick={(e) => {
            if (e.target === e.currentTarget) setBooksPicker(null);
          }}
        >
          <div
            className="prism-books-picker"
            data-testid="prism-books-picker"
            role="dialog"
          >
            <header>
              <div className="prism-brand-sm">◇ PRISM · Books</div>
              <h3>Pick a second bundle to reconcile against</h3>
            </header>
            <ul>
              {otherOpenTabs.map((t) => (
                <li key={t}>
                  <button
                    type="button"
                    className="prism-books-picker-row"
                    onClick={() => setBooksPicker({ open: true, chosen: t })}
                    data-testid={`prism-books-pick-${t}`}
                    data-selected={booksPicker.chosen === t ? "true" : "false"}
                  >
                    <span className="prism-books-picker-dot" />
                    {t}
                  </button>
                </li>
              ))}
            </ul>
            <footer>
              <button
                type="button"
                className="prism-books-picker-cancel"
                onClick={() => setBooksPicker(null)}
              >
                Cancel
              </button>
              <button
                type="button"
                className="prism-books-picker-run"
                onClick={() => void runBooks(booksPicker.chosen)}
                disabled={busy}
                data-testid="prism-books-picker-run"
              >
                {busy ? "Reconciling…" : "Run Books"}
              </button>
            </footer>
          </div>
        </div>
      ) : null}

      <PrismResultModal
        result={result}
        client={client}
        sourceBundle={sourceBundle}
        onArtifactSaved={(name) => {
          setResult(null);
          onClose();
          onOpenSavedBundle?.(name);
        }}
        onClose={() => {
          setResult(null);
          // If consuming the last credit dropped us to 0, surface the
          // upsell right after the user closes the result.
          if (!credits.canRun) setUpsellOpen(true);
        }}
      />
      <PrismUpsellModal
        open={upsellOpen}
        onClose={() => setUpsellOpen(false)}
        onSignIn={() => {
          setUpsellOpen(false);
          onSignIn();
        }}
        onResetTrial={credits.reset}
      />
    </>
  );
}
