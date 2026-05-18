import { useEffect, useState } from "react";
import {
  SheetsClient,
  SheetsClientError,
  type BundleListEntry,
} from "../lib/gigi-client";
import { pathForBundle } from "../lib/route";
import { listViews, type NamedView } from "../lib/view";
import { SidebarDemos } from "./SidebarDemos";
import "./Sidebar.css";

export interface SidebarProps {
  client: SheetsClient;
  /** The bundle that's currently open (highlighted in the list). */
  currentBundle: string;
  /**
   * If false, the sidebar swaps the bundle list for a "sign in to save"
   * CTA card. Server-fetched bundles + saved views are a signed-in
   * feature; demos are still available from the main BundlePicker.
   */
  signedIn: boolean;
  /** Called when the user clicks Sign in on the guest CTA. */
  onSignIn?: () => void;
  /** When the user clicks a saved view, apply it to the current bundle. */
  onApplyView?: (view: NamedView) => void;
  /** When the user clicks +New bundle. */
  onNewBundle?: () => void;
  /** When the user clicks +Saved view, opens the views drawer. */
  onOpenViews?: () => void;
  /** Client-side navigate (no page reload). */
  onPickBundle?: (name: string) => void;
}

/**
 * Left-rail navigation. Two modes:
 *   • signedIn=true  → bundles (fetched from /v1/bundles), saved views,
 *                      system bundles (collapsed).
 *   • signedIn=false → a CTA card explaining what signing in unlocks
 *                      (sync bundles + cloud saved views). The user can
 *                      still use the app fully without signing in — the
 *                      sidebar just doesn't have anything personalized
 *                      to show them yet.
 */
export function Sidebar({
  client,
  currentBundle,
  signedIn,
  onSignIn,
  onApplyView,
  onNewBundle,
  onOpenViews,
  onPickBundle,
}: SidebarProps) {
  const [bundles, setBundles] = useState<BundleListEntry[]>([]);
  const [error, setError] = useState<SheetsClientError | null>(null);
  const [views, setViews] = useState<NamedView[]>([]);

  // Only fetch the bundle list for signed-in users. Engine endpoints
  // are still reachable for guests via the demo flow + direct URLs;
  // the sidebar just doesn't enumerate them.
  useEffect(() => {
    if (!signedIn) {
      setBundles([]);
      setError(null);
      return;
    }
    let cancelled = false;
    client
      .listBundles()
      .then((b) => {
        if (!cancelled) setBundles(b);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setError(
          err instanceof SheetsClientError
            ? err
            : new SheetsClientError(String(err), "network_error"),
        );
      });
    return () => {
      cancelled = true;
    };
  }, [client, signedIn]);

  useEffect(() => {
    setViews(signedIn ? listViews(currentBundle) : []);
  }, [currentBundle, signedIn]);

  if (!signedIn) {
    return (
      <aside className="sidebar" data-testid="sidebar" data-mode="guest">
        <div className="sidebar-cta" data-testid="sidebar-cta">
          <div className="sidebar-cta-icon" aria-hidden="true">✨</div>
          <h4>Save your work to the cloud</h4>
          <p>
            GIGI Sheets is free without an account. Sign in to:
          </p>
          <ul className="sidebar-cta-list">
            <li>Sync bundles across devices</li>
            <li>Save views to the cloud</li>
            <li>Share with collaborators</li>
            <li>Unlock Prism workflows</li>
          </ul>
          <button
            type="button"
            className="sidebar-cta-btn"
            onClick={onSignIn}
            data-testid="sidebar-cta-signin"
          >
            Sign in with email
          </button>
          <p className="sidebar-cta-fine">
            Same account as davisgeometric.com
          </p>
        </div>
        {/* Fill the empty rail below the CTA with one-click sample data. */}
        <SidebarDemos client={client} onPickBundle={onPickBundle} />
      </aside>
    );
  }

  const userBundles = bundles.filter((b) => !b.name.startsWith("_"));
  const systemBundles = bundles.filter((b) => b.name.startsWith("_"));

  return (
    <aside className="sidebar" data-testid="sidebar" data-mode="user">
      <div className="side-section">
        <div className="side-title">
          <span>Bundles</span>
          {onNewBundle ? (
            <button
              type="button"
              className="side-title-btn"
              onClick={onNewBundle}
              title="New bundle"
              data-testid="sidebar-new-bundle"
              aria-label="New bundle"
            >
              +
            </button>
          ) : null}
        </div>
        {error ? (
          <div className="side-error" data-testid="sidebar-error">
            <small>{error.message}</small>
          </div>
        ) : null}
        <ul className="side-list" data-testid="sidebar-bundle-list">
          {userBundles.map((b) => (
            <li key={b.name}>
              <a
                href={pathForBundle(b.name)}
                className={`side-row ${b.name === currentBundle ? "side-row-active" : ""}`}
                data-testid={`sidebar-bundle-${b.name}`}
                data-active={b.name === currentBundle ? "true" : "false"}
                onClick={(e) => {
                  if (
                    onPickBundle &&
                    !e.metaKey &&
                    !e.ctrlKey &&
                    !e.shiftKey &&
                    e.button === 0
                  ) {
                    e.preventDefault();
                    onPickBundle(b.name);
                  }
                }}
              >
                <span className="side-dot" aria-hidden="true" />
                <span className="side-name">{b.name}</span>
                <span className="side-count">{b.records.toLocaleString()}</span>
              </a>
            </li>
          ))}
        </ul>
      </div>

      <div className="side-section">
        <div className="side-title">
          <span>Saved views</span>
          {onOpenViews ? (
            <button
              type="button"
              className="side-title-btn"
              onClick={onOpenViews}
              title="Manage saved views"
              data-testid="sidebar-open-views"
              aria-label="Manage saved views"
            >
              +
            </button>
          ) : null}
        </div>
        {views.length === 0 ? (
          <p className="side-empty">
            No saved views yet. Open <strong>Views</strong> to save one.
          </p>
        ) : (
          <ul className="side-list" data-testid="sidebar-views-list">
            {views.map((v) => (
              <li key={v.id}>
                <button
                  type="button"
                  className="side-row side-row-button"
                  onClick={() => onApplyView?.(v)}
                  data-testid={`sidebar-view-${v.id}`}
                >
                  <span className="side-dot side-dot-view" aria-hidden="true" />
                  <span className="side-name">{v.name}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>

      {systemBundles.length > 0 ? (
        <details className="side-section side-section-system">
          <summary className="side-title">
            <span>System ({systemBundles.length})</span>
          </summary>
          <ul className="side-list">
            {systemBundles.map((b) => (
              <li key={b.name}>
                <a
                  href={pathForBundle(b.name)}
                  className={`side-row side-row-system ${b.name === currentBundle ? "side-row-active" : ""}`}
                  data-testid={`sidebar-bundle-${b.name}`}
                  onClick={(e) => {
                    if (
                      onPickBundle &&
                      !e.metaKey &&
                      !e.ctrlKey &&
                      !e.shiftKey &&
                      e.button === 0
                    ) {
                      e.preventDefault();
                      onPickBundle(b.name);
                    }
                  }}
                >
                  <span className="side-dot side-dot-system" aria-hidden="true" />
                  <span className="side-name">{b.name}</span>
                  <span className="side-count">{b.records.toLocaleString()}</span>
                </a>
              </li>
            ))}
          </ul>
        </details>
      ) : null}
    </aside>
  );
}
