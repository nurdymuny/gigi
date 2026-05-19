import { useEffect, useState } from "react";
import type { SheetsClient } from "./gigi-client";
import type { Account } from "./use-account";

/**
 * Bridges the davisgeometric session to the gigi-stream engine.
 *
 * The engine sits behind an X-API-Key gate (see auth_middleware in
 * src/bin/gigi_stream.rs). davisgeometric.com/api/gigi/token issues
 * the key only to allowlisted emails, so this hook:
 *
 *   1. Waits for the account hook to settle.
 *   2. When the user is signed in, fetches the engine token.
 *   3. On 200, hands the key to the client (setApiKey) — every later
 *      request goes through with auth, and any open WS reconnects pick
 *      up the new wsUrl on next subscribe.
 *   4. On 403, surfaces a "denied" state the picker uses to render a
 *      "private deployment" panel instead of the bundle list.
 *   5. On any other failure, surfaces a generic "error" state with the
 *      operator-facing message.
 *
 * Guests stay in "anonymous" — the engine is locked, but the public
 * landing page lives off-engine (DemoBundles + LandingPage), so guests
 * can still browse the marketing surface before signing in.
 */

export type EngineAccessState =
  | { kind: "loading" }
  | { kind: "anonymous" }
  | { kind: "granted" }
  | { kind: "denied"; message: string }
  | { kind: "error"; message: string };

function resolveAuthBase(): string {
  const raw = (import.meta.env?.VITE_AUTH_BASE_URL ?? "") as string;
  return raw === "" ? "" : raw.replace(/\/$/, "");
}

export function useEngineAccess(
  client: SheetsClient,
  account: Account,
): EngineAccessState {
  const [state, setState] = useState<EngineAccessState>({ kind: "loading" });

  useEffect(() => {
    let cancelled = false;

    if (account.state === "loading") {
      setState({ kind: "loading" });
      return;
    }

    if (account.state === "guest") {
      // Engine stays locked; keep the previous key cleared so a stale
      // key from a prior signed-in session can't accidentally ride on
      // a guest's requests.
      client.setApiKey(null);
      setState({ kind: "anonymous" });
      return;
    }

    setState({ kind: "loading" });
    const base = resolveAuthBase();
    (async () => {
      try {
        const res = await fetch(`${base}/api/gigi/token`, {
          method: "GET",
          credentials: "include",
          headers: { accept: "application/json" },
        });
        if (cancelled) return;
        if (res.status === 403) {
          const data = (await res.json().catch(() => ({}))) as {
            message?: string;
          };
          client.setApiKey(null);
          setState({
            kind: "denied",
            message:
              data.message ??
              "Engine access is limited to the deployment owner.",
          });
          return;
        }
        if (!res.ok) {
          client.setApiKey(null);
          setState({
            kind: "error",
            message: `Couldn't reach the engine (status ${res.status}).`,
          });
          return;
        }
        const data = (await res.json().catch(() => ({}))) as { key?: string };
        if (!data.key) {
          client.setApiKey(null);
          setState({
            kind: "error",
            message: "Engine token response missing key.",
          });
          return;
        }
        client.setApiKey(data.key);
        setState({ kind: "granted" });
      } catch (e) {
        if (cancelled) return;
        client.setApiKey(null);
        setState({
          kind: "error",
          message:
            e instanceof Error
              ? e.message
              : "Network error fetching engine token.",
        });
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [account.state, account.email, client]);

  return state;
}
