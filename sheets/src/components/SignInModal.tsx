import { useEffect, useState } from "react";
import type { SignInResult } from "../lib/use-account";
import "./SignInModal.css";

export interface SignInModalProps {
  open: boolean;
  onClose: () => void;
  /** Mirrors useAccount.signInWithEmail — POSTs the magic link request. */
  onSignIn: (email: string) => Promise<SignInResult>;
}

type Phase = "idle" | "sending" | "sent";

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

/**
 * Email-only sign-in modal that drives the Davis Geometric magic-link
 * flow. The user types their email, we POST /api/auth/magic-link, then
 * show a "check your inbox" confirmation. The actual session lands when
 * the user clicks the link in the email and the server sets the
 * `dg_session` cookie via /api/auth/verify.
 */
export function SignInModal({ open, onClose, onSignIn }: SignInModalProps) {
  const [email, setEmail] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [phase, setPhase] = useState<Phase>("idle");

  useEffect(() => {
    if (open) {
      setEmail("");
      setError(null);
      setPhase("idle");
    }
  }, [open]);

  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  const submit = async () => {
    setError(null);
    const e = email.trim().toLowerCase();
    if (!EMAIL_RE.test(e)) {
      setError("Enter a valid email address.");
      return;
    }
    setPhase("sending");
    const result = await onSignIn(e);
    if (result.ok) {
      setPhase("sent");
    } else {
      setPhase("idle");
      setError(result.error ?? "Sign-in failed.");
    }
  };

  return (
    <div
      className="signin-bg"
      data-testid="signin-bg"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className="signin-modal"
        data-testid="signin-modal"
        role="dialog"
        aria-labelledby="signin-title"
      >
        <button
          type="button"
          className="signin-close"
          onClick={onClose}
          aria-label="Close"
          data-testid="signin-close"
        >
          ×
        </button>
        {phase === "sent" ? (
          <div className="signin-sent" data-testid="signin-sent">
            <div className="signin-icon-success" aria-hidden="true">✓</div>
            <h2 id="signin-title">Check your inbox</h2>
            <p>
              We sent a magic link to <strong>{email}</strong>. Click it to
              finish signing in. The link expires in 15 minutes.
            </p>
            <button type="button" className="signin-btn" onClick={onClose}>
              Close
            </button>
          </div>
        ) : (
          <>
            <h2 id="signin-title">Sign in (optional)</h2>
            <p className="signin-sub">
              GIGI Sheets is free to use without an account — you only need
              to sign in to <strong>save views to the cloud</strong> or
              sync across devices. Type your email and we'll send you a
              magic link. Same account that works on davisgeometric.com.
            </p>
            <label className="signin-label">
              Email
              <input
                type="email"
                className="signin-email"
                value={email}
                onChange={(e) => {
                  setEmail(e.target.value);
                  setError(null);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    void submit();
                  }
                }}
                placeholder="you@example.com"
                data-testid="signin-email"
                autoFocus
                autoComplete="email"
                disabled={phase === "sending"}
              />
            </label>
            {error ? (
              <p className="signin-error" role="alert" data-testid="signin-error">
                {error}
              </p>
            ) : null}
            <button
              type="button"
              className="signin-btn signin-btn-primary"
              onClick={() => void submit()}
              disabled={phase === "sending"}
              data-testid="signin-submit"
            >
              {phase === "sending" ? "Sending…" : "Send magic link"}
            </button>
            <p className="signin-fineprint">
              By signing in you agree to the Davis Geometric terms. Your email
              is used only for sign-in.
            </p>
          </>
        )}
      </div>
    </div>
  );
}
