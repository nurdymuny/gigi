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
  | {
      kind: "granted";
      /**
       * Per-user namespace tag — `ns_<12-hex>` derived from sha256(email)
       * on the davisgeometric side. Sheets uses this to filter the
       * bundle picker + prefix newly-created bundles so each user has
       * an isolated workspace. The deployment owner has `isOwner=true`
       * and sees every bundle, prefixed or not.
       */
      namespace: string;
      isOwner: boolean;
      email: string;
    }
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
      // Engine stays locked; clear any credentials so a stale key/
      // token from a prior signed-in session can't ride on a guest's
      // requests.
      client.setApiKey(null);
      client.setBearerToken(null);
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
          client.setBearerToken(null);
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
          client.setBearerToken(null);
          setState({
            kind: "error",
            message: `Couldn't reach the engine (status ${res.status}).`,
          });
          return;
        }
        const data = (await res.json().catch(() => ({}))) as {
          mode?: "apiKey" | "token";
          // Phase A / owner path: raw engine API key.
          key?: string;
          // Phase B / tenant path: HMAC-signed bearer token.
          token?: string;
          namespace?: string;
          isOwner?: boolean;
          email?: string;
        };
        // Mode-driven dispatch. Pre-Phase-B deploys don't send `mode`;
        // infer from which credential field is present so the client
        // keeps working against either response shape.
        const mode =
          data.mode ?? (data.token ? "token" : data.key ? "apiKey" : null);
        if (mode === "apiKey" && data.key) {
          client.setApiKey(data.key);
        } else if (mode === "token" && data.token) {
          client.setBearerToken(data.token);
        } else {
          client.setApiKey(null);
          client.setBearerToken(null);
          setState({
            kind: "error",
            message: "Engine token response missing credential.",
          });
          return;
        }
        // Older DG deploys may not surface namespace/isOwner — fall
        // back to owner semantics so we don't accidentally hide
        // bundles from an un-upgraded deployment.
        const namespace = data.namespace ?? "";
        const isOwner = data.isOwner ?? mode === "apiKey";
        client.setNamespace(namespace, isOwner);
        setState({
          kind: "granted",
          namespace,
          isOwner,
          email: data.email ?? account.email ?? "",
        });
      } catch (e) {
        if (cancelled) return;
        client.setApiKey(null);
        client.setBearerToken(null);
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
