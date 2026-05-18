import "./Banner.css";

export interface BannerMessage {
  kind: "info" | "warn" | "error";
  text: string;
  /** Optional primary action button. */
  action?: { label: string; onClick: () => void };
}

export interface BannerProps {
  message: BannerMessage | null;
  onDismiss: () => void;
}

/**
 * Dismissable notice strip. Lives at the top of the main view to surface
 * realtime/alert information without modal interruption:
 *   - "3 anomalies detected in the last hour"
 *   - "Realtime stream reconnecting…"
 *   - "Schema changed on the server"
 *
 * The parent owns the message; this component only renders + dismisses.
 */
export function Banner({ message, onDismiss }: BannerProps) {
  if (!message) return null;

  return (
    <div
      className={`banner banner-${message.kind}`}
      data-testid="banner"
      data-kind={message.kind}
      role={message.kind === "error" ? "alert" : "status"}
    >
      <span className="banner-icon" aria-hidden="true">
        {message.kind === "error" ? "⚠" : message.kind === "warn" ? "!" : "ⓘ"}
      </span>
      <span className="banner-text">{message.text}</span>
      {message.action ? (
        <button
          type="button"
          className="banner-action"
          onClick={message.action.onClick}
          data-testid="banner-action"
        >
          {message.action.label}
        </button>
      ) : null}
      <button
        type="button"
        className="banner-dismiss"
        onClick={onDismiss}
        aria-label="Dismiss"
        data-testid="banner-dismiss"
      >
        ×
      </button>
    </div>
  );
}
