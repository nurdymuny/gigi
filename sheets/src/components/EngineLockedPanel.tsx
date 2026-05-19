import gigiIconUrl from "../assets/gigi-icon.svg";
import "./EngineLockedPanel.css";

/**
 * Rendered when the engine refuses the davisgeometric session: either
 * the user isn't on the allowlist (403 from /api/gigi/token, kind:
 * "denied") or the token endpoint itself failed (kind: "error"). Sheets
 * has no useful data to show in this state, so we surface a clear
 * "private deployment" panel instead of a wall of failed requests.
 */
export interface EngineLockedPanelProps {
  /** Headline / body copy varies by reason. */
  reason: "denied" | "error" | "loading";
  /** Server-supplied message. Falls back to a generic one per reason. */
  message?: string;
  /** Allow the user to head back to the picker / sign-out flow. */
  onOpenAccount?: () => void;
  /** "Sign out" shortcut so a wrong account can be swapped quickly. */
  onSignOut?: () => void;
}

export function EngineLockedPanel({
  reason,
  message,
  onOpenAccount,
  onSignOut,
}: EngineLockedPanelProps) {
  return (
    <div className="engine-locked-shell">
      <div className="engine-locked-panel" data-testid="engine-locked-panel">
        <img
          src={gigiIconUrl}
          className="engine-locked-icon"
          alt="GIGI"
          draggable={false}
        />
        {reason === "loading" ? (
          <>
            <h1 className="engine-locked-h1">Unlocking your workspace…</h1>
            <p className="engine-locked-p">
              Checking your access to the engine.
            </p>
          </>
        ) : null}
        {reason === "denied" ? (
          <>
            <div className="engine-locked-badge">Private deployment</div>
            <h1 className="engine-locked-h1">This workspace is owner-only</h1>
            <p className="engine-locked-p">
              {message ??
                "GIGI Sheets engine access is limited to the deployment owner."}
            </p>
            <p className="engine-locked-fine">
              Already a collaborator? Sign out and use the email that's on
              the allowlist.
            </p>
          </>
        ) : null}
        {reason === "error" ? (
          <>
            <h1 className="engine-locked-h1">Couldn't reach the engine</h1>
            <p className="engine-locked-p">
              {message ??
                "Something went wrong checking your engine access. Try refreshing."}
            </p>
          </>
        ) : null}
        <div className="engine-locked-actions">
          {onOpenAccount ? (
            <button
              type="button"
              className="engine-locked-btn"
              onClick={onOpenAccount}
            >
              View account
            </button>
          ) : null}
          {onSignOut ? (
            <button
              type="button"
              className="engine-locked-btn engine-locked-btn-danger"
              onClick={onSignOut}
            >
              Sign out
            </button>
          ) : null}
        </div>
      </div>
    </div>
  );
}
