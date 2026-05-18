import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import { useAccount } from "../../src/lib/use-account";

/**
 * Contract mirrored from davisgeometric.com:
 *   GET  /api/auth/session       → { authenticated: bool, email?, subscription? }
 *   POST /api/auth/magic-link    → { success, expiresAt } | 429 { error, message }
 *   POST /api/auth/logout        → 204
 */

const FETCH_SCRIPT: Array<() => Response | Promise<Response>> = [];
let realFetch: typeof global.fetch | undefined;

beforeEach(() => {
  FETCH_SCRIPT.length = 0;
  realFetch = global.fetch;
  global.fetch = vi.fn((..._args: unknown[]) => {
    const next = FETCH_SCRIPT.shift();
    if (!next) {
      return Promise.resolve(
        new Response(JSON.stringify({ authenticated: false }), {
          status: 200,
          headers: { "content-type": "application/json" },
        }),
      );
    }
    return Promise.resolve(next());
  }) as unknown as typeof fetch;
});
afterEach(() => {
  if (realFetch) global.fetch = realFetch;
});

function jsonRes(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

describe("useAccount", () => {
  it("starts in 'loading' state and resolves to 'guest' for an unauthenticated user", async () => {
    FETCH_SCRIPT.push(() => jsonRes({ authenticated: false }));
    const { result } = renderHook(() => useAccount({ baseUrl: "" }));
    expect(result.current.state).toBe("loading");
    await waitFor(() => expect(result.current.state).toBe("guest"));
    expect(result.current.email).toBeUndefined();
  });

  it("resolves to 'user' with email when the session endpoint authenticates", async () => {
    FETCH_SCRIPT.push(() =>
      jsonRes({
        authenticated: true,
        email: "bee@davisgeometric.com",
        subscription: { tier: "founders", status: "active" },
      }),
    );
    const { result } = renderHook(() => useAccount({ baseUrl: "" }));
    await waitFor(() => expect(result.current.state).toBe("user"));
    expect(result.current.email).toBe("bee@davisgeometric.com");
    expect(result.current.subscription?.tier).toBe("founders");
  });

  it("computes initials from the email's local part for the avatar", async () => {
    FETCH_SCRIPT.push(() =>
      jsonRes({ authenticated: true, email: "bee.rosa.davis@example.com" }),
    );
    const { result } = renderHook(() => useAccount({ baseUrl: "" }));
    await waitFor(() => expect(result.current.state).toBe("user"));
    // bee.rosa.davis → BR (first letter of first two dotted segments)
    expect(result.current.initials).toBe("BR");
  });

  it("falls back to a single-letter initial when the local part is one segment", async () => {
    FETCH_SCRIPT.push(() => jsonRes({ authenticated: true, email: "bee@x.com" }));
    const { result } = renderHook(() => useAccount({ baseUrl: "" }));
    await waitFor(() => expect(result.current.state).toBe("user"));
    expect(result.current.initials).toBe("B");
  });

  it("signInWithEmail() POSTs to /api/auth/magic-link with the email", async () => {
    FETCH_SCRIPT.push(() => jsonRes({ authenticated: false }));
    FETCH_SCRIPT.push(() =>
      jsonRes({ success: true, expiresAt: "2026-05-15T01:00:00Z" }),
    );
    const { result } = renderHook(() => useAccount({ baseUrl: "" }));
    await waitFor(() => expect(result.current.state).toBe("guest"));
    let out: Awaited<ReturnType<typeof result.current.signInWithEmail>> | undefined;
    await act(async () => {
      out = await result.current.signInWithEmail("bee@davisgeometric.com");
    });
    expect(out?.ok).toBe(true);
    const calls = (global.fetch as unknown as ReturnType<typeof vi.fn>).mock.calls;
    const post = calls[1];
    expect(post[0]).toBe("/api/auth/magic-link");
    expect(post[1].method).toBe("POST");
    expect(JSON.parse(post[1].body)).toEqual({ email: "bee@davisgeometric.com" });
  });

  it("signInWithEmail() surfaces a 429 rate-limit error", async () => {
    FETCH_SCRIPT.push(() => jsonRes({ authenticated: false }));
    FETCH_SCRIPT.push(() =>
      jsonRes({ error: "Too many requests", message: "Try again in 1h" }, 429),
    );
    const { result } = renderHook(() => useAccount({ baseUrl: "" }));
    await waitFor(() => expect(result.current.state).toBe("guest"));
    let out: Awaited<ReturnType<typeof result.current.signInWithEmail>> | undefined;
    await act(async () => {
      out = await result.current.signInWithEmail("bee@x.com");
    });
    expect(out?.ok).toBe(false);
    expect(out?.error).toMatch(/rate|429|try again/i);
  });

  it("signOut() POSTs to /api/auth/logout and flips state to 'guest'", async () => {
    FETCH_SCRIPT.push(() =>
      jsonRes({ authenticated: true, email: "bee@x.com" }),
    );
    FETCH_SCRIPT.push(() => new Response("", { status: 204 }));
    const { result } = renderHook(() => useAccount({ baseUrl: "" }));
    await waitFor(() => expect(result.current.state).toBe("user"));
    await act(async () => {
      await result.current.signOut();
    });
    expect(result.current.state).toBe("guest");
    expect(result.current.email).toBeUndefined();
    const calls = (global.fetch as unknown as ReturnType<typeof vi.fn>).mock.calls;
    expect(calls[1][0]).toBe("/api/auth/logout");
    expect(calls[1][1].method).toBe("POST");
  });

  it("session fetch sends credentials: 'include' so the dg_session cookie rides", async () => {
    FETCH_SCRIPT.push(() => jsonRes({ authenticated: false }));
    renderHook(() => useAccount({ baseUrl: "" }));
    await waitFor(() => {
      const calls = (global.fetch as unknown as ReturnType<typeof vi.fn>).mock.calls;
      expect(calls[0]?.[1]?.credentials).toBe("include");
    });
  });

  it("recovers from a session endpoint network error by landing in 'guest' state (no infinite loading)", async () => {
    FETCH_SCRIPT.push(() => {
      throw new Error("network");
    });
    const { result } = renderHook(() => useAccount({ baseUrl: "" }));
    await waitFor(() => expect(result.current.state).toBe("guest"));
  });
});
