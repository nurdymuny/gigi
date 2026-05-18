import { useCallback, useEffect, useState } from "react";

/**
 * Bound the length of a server-supplied error / status message before
 * we surface it in the UI. React's text-node rendering already escapes
 * markup, but an unbounded server string is still a denial-of-screen
 * vector (one MB of "x" in an inline toast can lock the layout). 240
 * chars covers any reasonable human-readable response.
 */
const MAX_SERVER_MESSAGE = 240;
function safeMessage(raw: unknown, fallback: string): string {
  if (typeof raw !== "string" || raw.length === 0) return fallback;
  const trimmed = raw.trim();
  if (trimmed.length === 0) return fallback;
  if (trimmed.length <= MAX_SERVER_MESSAGE) return trimmed;
  return trimmed.slice(0, MAX_SERVER_MESSAGE) + "…";
}

/**
 * Account state mirror for davisgeometric.com's magic-link auth.
 *
 * Wire contract (server-side handlers live in math-website/api/auth):
 *   GET  /api/auth/session      → { authenticated, email?, subscription? }
 *   POST /api/auth/magic-link   → { success, expiresAt } | 429 { error, message }
 *   POST /api/auth/logout       → 204
 *
 * The `dg_session` cookie is httpOnly + Secure + SameSite=Strict, set by
 * /api/auth/verify when the user clicks the magic link in their email.
 * All requests here send `credentials: 'include'` so the cookie rides on
 * cross-origin deployments (api lives at davisgeometric.com, sheets may
 * live at a subdomain or path).
 */

export interface Subscription {
  tier?: string;
  status?: string;
  stripeCustomerId?: string;
}

export type AccountState = "loading" | "guest" | "user";

export interface Account {
  state: AccountState;
  email?: string;
  subscription?: Subscription | null;
  /** 1-2 character avatar initials derived from the email. */
  initials: string;
  /** Send a magic link to the user's email. */
  signInWithEmail: (email: string) => Promise<SignInResult>;
  /** Drop the current session. */
  signOut: () => Promise<void>;
  /** Re-poll the session endpoint (e.g. after returning from a magic link). */
  refresh: () => Promise<void>;
}

export interface SignInResult {
  ok: boolean;
  /** Human-readable message — success or failure. */
  message?: string;
  /** Failure reason for non-ok results. */
  error?: string;
}

/**
 * Where the auth endpoints live. By default we read this from the
 * VITE_AUTH_BASE_URL env var (e.g. `https://davisgeometric.com`).
 *
 * When unset, the hook **skips the session fetch entirely** and lands in
 * "guest" state without making a network request — this keeps the dev
 * server (localhost:5177) clean of 404 noise on `/api/auth/session`.
 *
 * To enable auth locally:
 *   echo VITE_AUTH_BASE_URL=https://davisgeometric.com > sheets/.env.local
 */
/**
 * Allow-list of origins we'll send `credentials: 'include'` to. Anything
 * outside this list is silently rejected back to null (no auth wired) so
 * a misconfigured `VITE_AUTH_BASE_URL` cannot leak the session cookie to
 * an attacker-controlled origin at build time.
 *
 * The list intentionally hard-codes davisgeometric.com production +
 * common localhost dev ports. To add a new origin, edit this array.
 */
const ALLOWED_AUTH_ORIGINS: readonly string[] = [
  "https://davisgeometric.com",
  "https://www.davisgeometric.com",
  "https://staging.davisgeometric.com",
  "http://localhost:3000",
  "http://localhost:3001",
  "http://localhost:8787",
];

function sanitizeAuthBase(raw: string | null | undefined): string | null {
  if (raw == null) return null;
  // Empty string is the explicit "use relative same-origin paths" sentinel.
  // The browser keeps the cookie boundary; no cross-origin risk.
  if (raw === "") return "";
  let url: URL;
  try {
    url = new URL(raw);
  } catch {
    return null; // malformed URL
  }
  // Drop trailing slash; we always append `/api/...` ourselves.
  const origin = `${url.protocol}//${url.host}`;
  return ALLOWED_AUTH_ORIGINS.includes(origin) ? origin : null;
}

const DEFAULT_BASE: string | null = sanitizeAuthBase(
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (import.meta as any).env?.VITE_AUTH_BASE_URL ?? null,
);

/**
 * Derive avatar initials from an email. Tries to find two dotted segments
 * (firstname.lastname@…) and returns their first letters; otherwise just
 * returns the first letter of the local part.
 */
export function initialsFromEmail(email: string): string {
  if (!email) return "?";
  const local = email.split("@")[0] ?? email;
  const parts = local.split(/[._-]/).filter(Boolean);
  if (parts.length >= 2) {
    return (parts[0][0] + parts[1][0]).toUpperCase();
  }
  return (parts[0]?.[0] ?? "?").toUpperCase();
}

interface SessionResponse {
  authenticated?: boolean;
  email?: string;
  subscription?: Subscription | null;
}

export function useAccount(opts: { baseUrl?: string | null } = {}): Account {
  // Apply the same origin allow-list to a caller-supplied baseUrl so a
  // test or wrapper component can't accidentally widen the trust boundary.
  const base =
    opts.baseUrl === undefined
      ? DEFAULT_BASE
      : sanitizeAuthBase(opts.baseUrl);
  // When base is null the auth endpoints aren't configured (typical dev
  // setup) — start in "guest" without firing a request.
  const [state, setState] = useState<AccountState>(
    base === null ? "guest" : "loading",
  );
  const [email, setEmail] = useState<string | undefined>(undefined);
  const [subscription, setSubscription] = useState<Subscription | null | undefined>(undefined);

  const fetchSession = useCallback(async () => {
    if (base === null) {
      // Skip the fetch — no auth backend configured.
      setState("guest");
      setEmail(undefined);
      setSubscription(undefined);
      return;
    }
    try {
      const res = await fetch(`${base}/api/auth/session`, {
        method: "GET",
        credentials: "include",
        headers: { accept: "application/json" },
      });
      if (!res.ok) {
        setState("guest");
        setEmail(undefined);
        setSubscription(undefined);
        return;
      }
      const data = (await res.json()) as SessionResponse;
      if (data.authenticated && data.email) {
        setState("user");
        setEmail(data.email);
        setSubscription(data.subscription ?? null);
      } else {
        setState("guest");
        setEmail(undefined);
        setSubscription(undefined);
      }
    } catch {
      // Endpoint unreachable (no api yet, offline, etc.) — treat as guest
      // rather than locking the UI in "loading".
      setState("guest");
      setEmail(undefined);
      setSubscription(undefined);
    }
  }, [base]);

  useEffect(() => {
    void fetchSession();
  }, [fetchSession]);

  const signInWithEmail = useCallback(
    async (addr: string): Promise<SignInResult> => {
      if (base === null) {
        return {
          ok: false,
          error:
            "Sign-in is disabled in this build. Set VITE_AUTH_BASE_URL to enable.",
        };
      }
      try {
        const res = await fetch(`${base}/api/auth/magic-link`, {
          method: "POST",
          credentials: "include",
          headers: { "content-type": "application/json", accept: "application/json" },
          body: JSON.stringify({ email: addr }),
        });
        const data = (await res.json().catch(() => ({}))) as Record<string, unknown>;
        if (res.ok) {
          return {
            ok: true,
            message: safeMessage(data.message, "Magic link sent — check your email."),
          };
        }
        if (res.status === 429) {
          return {
            ok: false,
            error: safeMessage(data.message, "Rate limit hit — try again later."),
          };
        }
        return {
          ok: false,
          error: safeMessage(data.error, `Sign-in failed (${res.status}).`),
        };
      } catch (err) {
        return {
          ok: false,
          error: err instanceof Error ? safeMessage(err.message, "Network error") : "Network error",
        };
      }
    },
    [base],
  );

  const signOut = useCallback(async () => {
    if (base !== null) {
      try {
        await fetch(`${base}/api/auth/logout`, {
          method: "POST",
          credentials: "include",
        });
      } catch {
        /* always proceed to client-side guest state */
      }
    }
    setState("guest");
    setEmail(undefined);
    setSubscription(undefined);
  }, [base]);

  return {
    state,
    email,
    subscription,
    initials: email ? initialsFromEmail(email) : "•",
    signInWithEmail,
    signOut,
    refresh: fetchSession,
  };
}
