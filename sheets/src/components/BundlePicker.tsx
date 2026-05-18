import { useEffect, useState } from "react";
import {
  SheetsClient,
  SheetsClientError,
  type BundleListEntry,
} from "../lib/gigi-client";
import { pathForBundle } from "../lib/route";
import { DemoBundles } from "./DemoBundles";
import { WorkflowPicker } from "./WorkflowPicker";
import "./BundlePicker.css";

export interface BundlePickerProps {
  client: SheetsClient;
  /** Bundle the user asked for, if any. Drives the "not found" header. */
  requestedBundle?: string | null;
  /** Engine error from the failed load, if any (typically 404). */
  loadError?: SheetsClientError | null;
  /**
   * Client-side navigate. When provided, the picker uses it instead of
   * letting <a href> trigger a full page reload.
   */
  onPickBundle?: (name: string) => void;
}

type PickerState =
  | { kind: "loading" }
  | { kind: "ok"; bundles: BundleListEntry[] }
  | { kind: "error"; error: SheetsClientError };

export function BundlePicker({
  client,
  requestedBundle,
  loadError,
  onPickBundle,
}: BundlePickerProps) {
  const [state, setState] = useState<PickerState>({ kind: "loading" });

  useEffect(() => {
    let cancelled = false;
    setState({ kind: "loading" });
    client
      .listBundles()
      .then((bundles) => {
        if (cancelled) return;
        setState({ kind: "ok", bundles });
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        const e =
          err instanceof SheetsClientError
            ? err
            : new SheetsClientError(String(err), "network_error");
        setState({ kind: "error", error: e });
      });
    return () => {
      cancelled = true;
    };
  }, [client]);

  return (
    <div className="bundle-picker" data-testid="bundle-picker">
      <header className="bundle-picker-head">
        {requestedBundle && loadError ? (
          <>
            <h2>
              Bundle <code>{requestedBundle}</code> isn't here.
            </h2>
            <p>
              The engine returned <code>HTTP {loadError.status ?? "?"}</code>.
              Pick one of the bundles below, or create <code>{requestedBundle}</code>{" "}
              from the GQL console.
            </p>
          </>
        ) : (
          <>
            <h2>Pick a bundle</h2>
            <p>Anything below is loaded from the running engine.</p>
          </>
        )}
      </header>

      {state.kind === "loading" ? (
        <div className="bundle-picker-loading" data-testid="bundle-picker-loading">
          Loading bundles…
        </div>
      ) : null}

      {state.kind === "error" ? (
        <div
          className="bundle-picker-error"
          role="alert"
          data-testid="bundle-picker-error"
        >
          <strong>Couldn't reach the engine.</strong>
          <p>{state.error.message}</p>
          <small>
            code: {state.error.code}
            {state.error.status ? ` · status ${state.error.status}` : ""}
          </small>
        </div>
      ) : null}

      {state.kind === "ok" ? (
        <>
          <WorkflowPicker
            client={client}
            onApplied={(bundleName) => onPickBundle?.(bundleName)}
          />
          <DemoBundles
            client={client}
            existing={new Set(state.bundles.map((b) => b.name))}
            onPickBundle={onPickBundle}
          />
          <BundleGrid bundles={state.bundles} onPickBundle={onPickBundle} />
        </>
      ) : null}
    </div>
  );
}

function BundleGrid({
  bundles,
  onPickBundle,
}: {
  bundles: BundleListEntry[];
  onPickBundle?: (name: string) => void;
}) {
  if (bundles.length === 0) {
    return (
      <div className="bundle-picker-empty" data-testid="bundle-picker-empty">
        <p>No bundles on this engine yet.</p>
        <small>
          Create one with <code>CREATE BUNDLE …</code> from the GQL console.
        </small>
      </div>
    );
  }

  // Split out the engine's internal _gigi_* log bundles so they don't
  // clutter the main list — but show them in a collapsed section.
  const user = bundles.filter((b) => !b.name.startsWith("_"));
  const system = bundles.filter((b) => b.name.startsWith("_"));

  return (
    <div className="bundle-picker-body">
      <ul className="bundle-list" data-testid="bundle-list">
        {user.map((b) => (
          <BundleRow key={b.name} bundle={b} onPick={onPickBundle} />
        ))}
      </ul>

      {system.length > 0 ? (
        <details className="bundle-system" data-testid="bundle-system">
          <summary>
            System bundles ({system.length})
          </summary>
          <ul className="bundle-list">
            {system.map((b) => (
              <BundleRow key={b.name} bundle={b} system onPick={onPickBundle} />
            ))}
          </ul>
        </details>
      ) : null}
    </div>
  );
}

function BundleRow({
  bundle,
  system,
  onPick,
}: {
  bundle: BundleListEntry;
  system?: boolean;
  onPick?: (name: string) => void;
}) {
  const href = pathForBundle(bundle.name);
  return (
    <li className={`bundle-row ${system ? "bundle-row-system" : ""}`}>
      <a
        href={href}
        className="bundle-link"
        data-testid={`bundle-pick-${bundle.name}`}
        onClick={(e) => {
          // Let cmd/ctrl/middle-click + new-tab work as expected.
          if (onPick && !e.metaKey && !e.ctrlKey && !e.shiftKey && e.button === 0) {
            e.preventDefault();
            onPick(bundle.name);
          }
        }}
      >
        <span className="bundle-name">{bundle.name}</span>
        <span className="bundle-meta">
          <span className="bundle-count" data-testid={`bundle-count-${bundle.name}`}>
            {bundle.records.toLocaleString()} rows
          </span>
          <span className="bundle-fields">{bundle.fields} fields</span>
        </span>
      </a>
    </li>
  );
}
