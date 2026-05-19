import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import gigiIconUrl from "../assets/gigi-icon.svg";
import type { Account } from "../lib/use-account";
import "./WelcomePage.css";

/**
 * Sheets-branded post-magic-link landing.
 *
 * Flow:
 *   1. On mount, GET /api/agreement/check.
 *   2. If unauthenticated → show a "sign in to continue" panel.
 *   3. If authenticated AND already on the current terms version →
 *      redirect to `?next=` (a /gigi/sheets/* path) immediately.
 *   4. If authenticated AND terms unsigned → render the legal text +
 *      signature canvas in the sheets visual language. Submit POSTs to
 *      /api/agreement/submit, then redirects to `next`.
 *
 * The legal text is identical to davisgeometric.com/members/agreement
 * (same termsVersion in the backend), but rendered as a sheets page so
 * the user never bounces off-brand.
 */

const TERMS_VERSION = "2026-03-19-v1";

export interface WelcomePageProps {
  account: Account;
  /** Where to send the user once they're signed in + agreed. */
  next: string;
  /** Open the sheets sign-in modal. */
  onRequestSignIn: () => void;
  /** Bounce here when the user is already done. */
  onDone: (next: string) => void;
}

type CheckState =
  | { kind: "loading" }
  | { kind: "needs_signin" }
  | { kind: "needs_agreement" }
  | { kind: "done" }
  | { kind: "error"; message: string };

function safeNext(raw: string): string {
  if (typeof raw !== "string" || raw.length === 0) return "/gigi/sheets/";
  if (raw === "/gigi/sheets" || raw === "/gigi/sheets/") return raw;
  if (raw.startsWith("/gigi/sheets/")) return raw;
  return "/gigi/sheets/";
}

function resolveAuthBase(): string {
  const raw = (import.meta.env?.VITE_AUTH_BASE_URL ?? "") as string;
  // Empty string is the sentinel for same-origin — matches use-account.ts.
  return raw === "" ? "" : raw.replace(/\/$/, "");
}

export function WelcomePage({
  account,
  next,
  onRequestSignIn,
  onDone,
}: WelcomePageProps) {
  const [check, setCheck] = useState<CheckState>({ kind: "loading" });
  const [agreed, setAgreed] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const drewSomething = useRef(false);

  const safeDest = useMemo(() => safeNext(next), [next]);

  const runCheck = useCallback(async () => {
    if (account.state === "loading") return;
    if (account.state === "guest") {
      setCheck({ kind: "needs_signin" });
      return;
    }
    setCheck({ kind: "loading" });
    const base = resolveAuthBase();
    try {
      const res = await fetch(`${base}/api/agreement/check`, {
        method: "GET",
        credentials: "include",
        headers: { accept: "application/json" },
      });
      if (res.status === 401) {
        setCheck({ kind: "needs_signin" });
        return;
      }
      if (!res.ok) {
        setCheck({
          kind: "error",
          message: `Couldn't verify your account (${res.status}). Try again in a moment.`,
        });
        return;
      }
      const data = (await res.json().catch(() => ({}))) as {
        hasSigned?: boolean;
        agreementRequired?: boolean;
      };
      if (data.hasSigned) {
        setCheck({ kind: "done" });
        // Tiny delay so the user briefly sees the "you're all set" panel
        // instead of a hard flicker through to the bundle picker.
        window.setTimeout(() => onDone(safeDest), 350);
      } else {
        setCheck({ kind: "needs_agreement" });
      }
    } catch {
      setCheck({
        kind: "error",
        message: "Network error verifying your account. Check your connection.",
      });
    }
  }, [account.state, onDone, safeDest]);

  useEffect(() => {
    void runCheck();
  }, [runCheck]);

  // Signature canvas wiring — pointer events so it works for mouse + touch
  // + pen without separate code paths. We draw at devicePixelRatio so the
  // PNG submitted to the server is crisp on retina displays.
  useEffect(() => {
    if (check.kind !== "needs_agreement") return;
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const cssWidth = canvas.clientWidth;
    const cssHeight = canvas.clientHeight;
    canvas.width = Math.round(cssWidth * dpr);
    canvas.height = Math.round(cssHeight * dpr);
    ctx.scale(dpr, dpr);
    ctx.lineWidth = 2;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    ctx.strokeStyle = "#0b1220";

    let drawing = false;
    let lastX = 0;
    let lastY = 0;

    function pos(e: PointerEvent) {
      const rect = canvas!.getBoundingClientRect();
      return { x: e.clientX - rect.left, y: e.clientY - rect.top };
    }
    function start(e: PointerEvent) {
      drawing = true;
      const p = pos(e);
      lastX = p.x;
      lastY = p.y;
      canvas!.setPointerCapture(e.pointerId);
    }
    function move(e: PointerEvent) {
      if (!drawing) return;
      const p = pos(e);
      ctx!.beginPath();
      ctx!.moveTo(lastX, lastY);
      ctx!.lineTo(p.x, p.y);
      ctx!.stroke();
      lastX = p.x;
      lastY = p.y;
      drewSomething.current = true;
    }
    function end(e: PointerEvent) {
      drawing = false;
      try {
        canvas!.releasePointerCapture(e.pointerId);
      } catch {
        /* fine */
      }
    }
    canvas.addEventListener("pointerdown", start);
    canvas.addEventListener("pointermove", move);
    canvas.addEventListener("pointerup", end);
    canvas.addEventListener("pointercancel", end);
    canvas.addEventListener("pointerleave", end);
    return () => {
      canvas.removeEventListener("pointerdown", start);
      canvas.removeEventListener("pointermove", move);
      canvas.removeEventListener("pointerup", end);
      canvas.removeEventListener("pointercancel", end);
      canvas.removeEventListener("pointerleave", end);
    };
  }, [check.kind]);

  const clearSignature = () => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    drewSomething.current = false;
  };

  const submit = async () => {
    setSubmitError(null);
    if (!agreed) {
      setSubmitError("Tick the checkbox to confirm you agree.");
      return;
    }
    const canvas = canvasRef.current;
    if (!canvas || !drewSomething.current) {
      setSubmitError("Please sign in the box above.");
      return;
    }
    const signature = canvas.toDataURL("image/png");
    setSubmitting(true);
    const base = resolveAuthBase();
    try {
      const res = await fetch(`${base}/api/agreement/submit`, {
        method: "POST",
        credentials: "include",
        headers: {
          "content-type": "application/json",
          accept: "application/json",
        },
        body: JSON.stringify({ signature }),
      });
      if (!res.ok) {
        const data = (await res.json().catch(() => ({}))) as { error?: string };
        setSubmitting(false);
        setSubmitError(
          data.error ?? `Couldn't record your agreement (${res.status}).`,
        );
        return;
      }
      setCheck({ kind: "done" });
      window.setTimeout(() => onDone(safeDest), 400);
    } catch {
      setSubmitting(false);
      setSubmitError("Network error submitting your signature.");
    }
  };

  return (
    <div className="welcome-shell">
      <header className="welcome-topbar">
        <div className="welcome-brand">
          <img
            src={gigiIconUrl}
            className="welcome-brand-icon"
            alt="GIGI"
            draggable={false}
          />
          <span className="welcome-brand-name">GIGI Sheets</span>
          <span className="welcome-brand-sub">fiber bundles · for humans</span>
        </div>
      </header>

      <main className="welcome-main">
        {check.kind === "loading" ? (
          <Panel>
            <h1 className="welcome-h1">Welcome back</h1>
            <p className="welcome-p">Checking your account…</p>
          </Panel>
        ) : null}

        {check.kind === "needs_signin" ? (
          <Panel>
            <h1 className="welcome-h1">Sign in to continue</h1>
            <p className="welcome-p">
              Your session expired or you haven't signed in yet. Send yourself a
              magic link to land back here.
            </p>
            <button
              type="button"
              className="welcome-btn welcome-btn-primary"
              onClick={onRequestSignIn}
              data-testid="welcome-signin"
            >
              Send sign-in link
            </button>
            <p className="welcome-fine">
              Already clicked the link? Refresh this page once your inbox
              opens.
            </p>
          </Panel>
        ) : null}

        {check.kind === "needs_agreement" ? (
          <Panel wide>
            <div className="welcome-eyebrow">One last step</div>
            <h1 className="welcome-h1">Terms &amp; conditions</h1>
            <p className="welcome-p welcome-p-lead">
              Hi {account.email ? <strong>{account.email}</strong> : "there"} —
              before we drop you into your workspace, please read and sign the
              Davis Geometric terms below. Same terms that apply across
              davisgeometric.com; we just want a record of your agreement
              before sheets touches any of your bundles.
            </p>

            <div className="welcome-version">Version: {TERMS_VERSION}</div>

            <article className="welcome-terms">
              <h2>1. Acceptance of Terms</h2>
              <p>
                By accessing the Davis Geometric Members Portal (which includes
                GIGI Sheets), you agree to be bound by these Terms and
                Conditions. If you do not agree to all terms, you may not
                access or use the service.
              </p>

              <h2>2. Membership &amp; Access</h2>
              <p>
                Your membership provides access to exclusive mathematical
                content, the GIGI AI assistant, the GIGI Sheets workspace,
                and other member benefits based on your subscription tier.
                Access is granted for personal, non-commercial use.
              </p>

              <h2>3. Intellectual Property</h2>
              <p>All content provided through the service, including:</p>
              <ul>
                <li>Mathematical research, proofs, and derivations</li>
                <li>Educational materials and explanations</li>
                <li>Code, schemas, and algorithms</li>
                <li>AI-generated responses and explanations</li>
              </ul>
              <p>
                …remains the exclusive intellectual property of Dr. Bee Davis
                and Davis Geometric. You may not reproduce, distribute, modify,
                or create derivative works without explicit written permission.
                Your own data, bundles, and views remain yours.
              </p>

              <h2>4. Prohibited Uses</h2>
              <p>You agree NOT to:</p>
              <ul>
                <li>Share, redistribute, or resell any member-exclusive content</li>
                <li>Use automated systems to scrape or download content</li>
                <li>Attempt to reverse-engineer the GIGI AI system or sheets engine</li>
                <li>Share login credentials with non-members</li>
                <li>Use the service for any unlawful purpose</li>
              </ul>

              <h2>5. GIGI AI Assistant</h2>
              <p>
                GIGI provides educational and analytic support but is not a
                replacement for professional mathematical consultation.
                Responses are generated programmatically and may contain
                errors. Davis Geometric makes no warranty regarding the
                accuracy of AI-generated content.
              </p>

              <h2>6. Subscription &amp; Billing</h2>
              <p>
                Subscriptions are billed via Stripe. You may cancel at any time
                through the member portal. Refunds are handled on a case-by-case
                basis at Davis Geometric's discretion.
              </p>

              <h2>7. Privacy &amp; Data</h2>
              <p>
                We collect and store: your email address, subscription status,
                conversation history with GIGI, IP address at time of
                agreement, and electronic signature. Bundles and views you
                create in GIGI Sheets are stored encrypted. This data is used
                solely for service provision and legal compliance.
              </p>

              <h2>8. Limitation of Liability</h2>
              <p>
                Davis Geometric shall not be liable for any indirect,
                incidental, special, consequential, or punitive damages arising
                from your use of the service. Our total liability shall not
                exceed the amount paid for your subscription in the preceding
                12 months.
              </p>

              <h2>9. Modifications</h2>
              <p>
                We reserve the right to modify these terms at any time.
                Continued use after modifications constitutes acceptance of
                updated terms. You will be notified of significant changes
                via email.
              </p>

              <h2>10. Governing Law</h2>
              <p>
                These terms shall be governed by the laws of the State of
                California, United States. Any disputes shall be resolved in
                the courts of San Francisco County, California.
              </p>

              <h2>11. Contact</h2>
              <p>
                For questions about these terms, contact{" "}
                <a href="mailto:legal@davisgeometric.com">
                  legal@davisgeometric.com
                </a>
                .
              </p>
            </article>

            <div className="welcome-sig-section">
              <div className="welcome-sig-label">Electronic signature</div>
              <p className="welcome-sig-help">
                Sign below with your mouse, finger, or pen to indicate your
                agreement.
              </p>
              <div className="welcome-sig-canvas-wrap">
                <canvas
                  ref={canvasRef}
                  className="welcome-sig-canvas"
                  data-testid="welcome-sig-canvas"
                />
                <button
                  type="button"
                  className="welcome-sig-clear"
                  onClick={clearSignature}
                  disabled={submitting}
                >
                  Clear
                </button>
              </div>

              <label className="welcome-agree">
                <input
                  type="checkbox"
                  checked={agreed}
                  onChange={(e) => setAgreed(e.target.checked)}
                  data-testid="welcome-agree"
                />
                <span>
                  I have read, understood, and agree to the terms above. I
                  understand my electronic signature is legally binding.
                </span>
              </label>

              {submitError ? (
                <p className="welcome-error" role="alert">
                  {submitError}
                </p>
              ) : null}

              <button
                type="button"
                className="welcome-btn welcome-btn-primary"
                onClick={() => void submit()}
                disabled={submitting}
                data-testid="welcome-submit"
              >
                {submitting ? "Saving…" : "Agree and open my sheets workspace"}
              </button>
            </div>
          </Panel>
        ) : null}

        {check.kind === "done" ? (
          <Panel>
            <div className="welcome-icon-success" aria-hidden="true">✓</div>
            <h1 className="welcome-h1">You're all set</h1>
            <p className="welcome-p">Opening your workspace…</p>
          </Panel>
        ) : null}

        {check.kind === "error" ? (
          <Panel>
            <h1 className="welcome-h1">Something went sideways</h1>
            <p className="welcome-p">{check.message}</p>
            <button
              type="button"
              className="welcome-btn"
              onClick={() => void runCheck()}
            >
              Try again
            </button>
          </Panel>
        ) : null}
      </main>

      <footer className="welcome-footer">
        GIGI Sheets · a Davis Geometric property ·{" "}
        <a
          href="https://davisgeometric.com"
          target="_blank"
          rel="noopener noreferrer"
        >
          davisgeometric.com
        </a>
      </footer>
    </div>
  );
}

function Panel({
  children,
  wide,
}: {
  children: React.ReactNode;
  wide?: boolean;
}) {
  return (
    <div className={`welcome-panel${wide ? " welcome-panel-wide" : ""}`}>
      {children}
    </div>
  );
}
