import type { TutorialStep } from "../components/Tutorial";
import { applyWorkflow } from "./apply-workflow";
import type { SheetsClient } from "./gigi-client";
import { findWorkflowTemplate } from "./workflow-templates";

export interface ProjectTrackerTourDeps {
  client: SheetsClient;
  /** Navigate to a bundle. Wired from useBundleRoute. */
  navigateToBundle: (name: string) => void;
}

/**
 * BundleApp registers its setters here on mount so tutorial step actions
 * can call them without us lifting state up to App-level. Cleared on
 * BundleApp unmount.
 */
export interface TourRegistry {
  setActiveView?: (
    v: "grid" | "geometry" | "charts" | "kanban" | "gallery" | "form" | "gql",
  ) => void;
  openPrismDrawer?: () => void;
}

declare global {
  interface Window {
    __gigi_tour__?: TourRegistry;
  }
}

function tourRegistry(): TourRegistry {
  if (typeof window === "undefined") return {};
  if (!window.__gigi_tour__) window.__gigi_tour__ = {};
  return window.__gigi_tour__;
}

/** Used by BundleApp's useEffect to register/clean its setters. */
export function registerTourHelpers(helpers: TourRegistry): () => void {
  const r = tourRegistry();
  Object.assign(r, helpers);
  return () => {
    for (const k of Object.keys(helpers) as Array<keyof TourRegistry>) {
      delete r[k];
    }
  };
}

/**
 * The flagship walkthrough. Loads the Project Tracker workflow on the
 * engine, then steps the user through every surface that matters:
 * kanban → form → Prism. Each step's `action` advances the app state
 * so the spotlight lands on a real, freshly-mounted element.
 *
 * Each step's `target` is a data-testid we already render in App.tsx /
 * Sidebar / Toolbar — no new test ids required.
 */
export function buildProjectTrackerTour(
  deps: ProjectTrackerTourDeps,
): TutorialStep[] {
  const template = findWorkflowTemplate("project_tracker")!;
  return [
    {
      title: "Welcome to GIGI Sheets",
      body: (
        <>
          <p>
            This tour loads the <strong>Project tracker</strong> workflow on
            the engine and walks you through the surfaces a real team uses
            every day.
          </p>
          <p>
            About a minute total. Hit <code>Skip</code> any time.
          </p>
        </>
      ),
      target: null,
    },
    {
      title: "Loading the workflow…",
      body: (
        <p>
          Creating <code>workflow_projects</code> on the engine and seeding
          25 sample tasks. This is the same one-click flow available from
          the picker — we're just running it for you.
        </p>
      ),
      target: null,
      settleMs: 250,
      action: async () => {
        // Idempotent: if the bundle already exists, the engine 409s and we
        // swallow — the workflow is already on the engine, navigate to it.
        try {
          await applyWorkflow(template, deps.client);
        } catch {
          /* already created */
        }
        deps.navigateToBundle(template.defaultBundle);
      },
    },
    {
      title: "Top-bar: the bundle is loaded",
      body: (
        <p>
          The crumbs in the topbar show the active bundle. Every read,
          edit, and Prism workflow now operates on{" "}
          <code>workflow_projects</code>.
        </p>
      ),
      target: "brand-logo",
    },
    {
      title: "Kanban — tasks grouped by status",
      body: (
        <>
          <p>
            The default view for project tracking. Each card is a row;
            columns are the status field's distinct values.
          </p>
          <p>
            Card color encodes <strong>κ-curvature</strong> — red cards are
            anomalies (stalled, mismatched estimates, weird priority/owner
            pairs). Always on, nothing to configure.
          </p>
        </>
      ),
      target: "tab-kanban",
      action: async () => {
        tourRegistry().setActiveView?.("kanban");
      },
    },
    {
      title: "Gallery — same data, browsable",
      body: (
        <p>
          The same rows as cards. Useful when you want to scan all tickets
          at once instead of reading them grouped by status. The κ-tinted
          left border is the same anomaly signal.
        </p>
      ),
      target: "tab-gallery",
      action: async () => {
        tourRegistry().setActiveView?.("gallery");
      },
    },
    {
      title: "Form — submit a new task",
      body: (
        <p>
          A schema-driven intake form. Non-engineers can add tasks here
          without touching the grid. Numeric fields enforce numeric input;
          dates get a date picker; booleans become a select.
        </p>
      ),
      target: "tab-form",
      action: async () => {
        tourRegistry().setActiveView?.("form");
      },
    },
    {
      title: "Grid — full edit surface",
      body: (
        <>
          <p>
            The classic spreadsheet. Click a column header to sort (asc →
            desc → none). Click the <code>κ</code> column header to sort by
            curvature — anomalies first.
          </p>
          <p>
            Above the grid is the formula bar — try{" "}
            <code>=K(A1)</code> for a row's curvature, <code>=SAME(A1,A2)</code>{" "}
            for sameness between rows.
          </p>
        </>
      ),
      target: "tab-grid",
      action: async () => {
        tourRegistry().setActiveView?.("grid");
      },
    },
    {
      title: "Prism workflows — built-in analytics",
      body: (
        <>
          <p>
            Click here any time to open the Prism drawer. Four workflows
            run inline against the current bundle:
          </p>
          <p>
            <strong>Dedup</strong> · <strong>Forecast</strong> ·{" "}
            <strong>Monitor</strong> · <strong>Books</strong>
          </p>
          <p>
            On Project tracker, <strong>Monitor</strong> surfaces tasks
            with estimate/actual ratios far from the cohort — your
            stalled-or-bloated tickets, ranked.
          </p>
        </>
      ),
      target: "menu-data",
    },
    {
      title: "That's the tour.",
      body: (
        <>
          <p>
            Everything else (find, copy/paste, charts, GQL editor, share)
            uses the same substrate. The bundle is yours to keep — explore
            freely.
          </p>
          <p>
            <strong>You're on the Project tracker workflow right now.</strong>{" "}
            Sign in to sync it across devices.
          </p>
        </>
      ),
      target: null,
    },
  ];
}
