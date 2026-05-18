import { useEffect, useState } from "react";
import "./Toast.css";

export interface ToastMessage {
  id: number;
  kind: "info" | "success" | "error";
  text: string;
}

interface ToastProps {
  toast: ToastMessage | null;
  onDismiss: () => void;
  /**
   * Auto-dismiss timeout in ms. Default 5000 for info/success, 8000 for error
   * (longer so the user can actually read what broke).
   */
  durationMs?: number;
}

export function Toast({ toast, onDismiss, durationMs }: ToastProps) {
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    if (!toast) {
      setVisible(false);
      return;
    }
    setVisible(true);
    const ms = durationMs ?? (toast.kind === "error" ? 8000 : 5000);
    const timer = setTimeout(() => {
      setVisible(false);
      // Give the transition a beat before clearing the toast object.
      setTimeout(onDismiss, 200);
    }, ms);
    return () => clearTimeout(timer);
  }, [toast, durationMs, onDismiss]);

  if (!toast) return null;

  const dismiss = () => {
    setVisible(false);
    setTimeout(onDismiss, 200);
  };

  return (
    <div
      className={`toast ${visible ? "toast-visible" : ""} toast-${toast.kind}`}
      data-testid="toast"
      data-kind={toast.kind}
      role={toast.kind === "error" ? "alert" : "status"}
    >
      <span className="toast-icon" aria-hidden="true">
        {toast.kind === "success" ? (
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <path d="M20 6 9 17l-5-5" />
          </svg>
        ) : toast.kind === "error" ? (
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="12" cy="12" r="10" />
            <path d="M12 8v5M12 16h.01" />
          </svg>
        ) : (
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="12" cy="12" r="10" />
            <path d="M12 16v-5M12 8h.01" />
          </svg>
        )}
      </span>
      <span className="toast-text">{toast.text}</span>
      <button
        type="button"
        className="toast-close"
        onClick={dismiss}
        aria-label="Dismiss"
        data-testid="toast-close"
      >
        ✕
      </button>
    </div>
  );
}

let nextToastId = 1;
export function makeToast(
  kind: ToastMessage["kind"],
  text: string,
): ToastMessage {
  return { id: nextToastId++, kind, text };
}
