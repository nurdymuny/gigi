import { describe, expect, it, vi } from "vitest";
import { SheetsClient, type Fetcher } from "../../src/lib/gigi-client";

/**
 * Regression — 2026-06-04.
 *
 * The davisgeometric `/api/gigi/token` endpoint has been observed in
 * the wild returning the engine API key with a trailing newline. That
 * newline survived as `%0A` on every WS upgrade URL and as a literal
 * trailing byte on every `X-API-Key` HTTP header, causing the engine
 * to refuse the request with 401 (the auth middleware does a byte-
 * exact comparison against the env var).
 *
 * Live console evidence (Bee, 2026-06-04):
 *   WebSocket connection to
 *     'wss://gigi-stream.fly.dev/ws?api_key=YAx...EGGYqp1YOR%0A'
 *   failed
 *
 * `SheetsClient.setApiKey` and `setBearerToken` now trim incoming
 * credentials so the engine's byte-exact comparison still succeeds
 * even if upstream sends a sloppy newline. These tests pin that
 * behavior so a future refactor of those setters can't reintroduce
 * the same 401 storm.
 */
describe("SheetsClient credential trimming (regression — 2026-06-04 401 storm)", () => {
  function jsonResponse(body: unknown, status = 200) {
    return new Response(JSON.stringify(body), {
      status,
      headers: { "content-type": "application/json" },
    });
  }

  function fetchSpy() {
    return vi
      .fn()
      .mockResolvedValue(jsonResponse([])) as unknown as Fetcher;
  }

  it("strips trailing newlines from an API key (the davisgeometric case)", async () => {
    const fakeFetch = fetchSpy();
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    client.setApiKey("supersecret\n");
    await client.listBundles();

    const calls = (fakeFetch as unknown as { mock: { calls: unknown[][] } }).mock.calls;
    expect(calls).toHaveLength(1);
    const init = calls[0][1] as RequestInit;
    const headers = (init.headers ?? {}) as Record<string, string>;
    expect(headers["x-api-key"]).toBe("supersecret");
    expect(headers["x-api-key"]).not.toContain("\n");
  });

  it("strips leading + trailing whitespace from the subprotocol credential", () => {
    const client = new SheetsClient({ baseUrl: "http://localhost:3142" });
    client.setApiKey(" \t key-with-padding \r\n");
    // The WS URL no longer carries the credential (subprotocol-header
    // path). Verify the credential lives in the protocol list cleanly.
    const wsUrl = client.wsUrl("/ws");
    expect(wsUrl).not.toContain("api_key");
    expect(wsUrl).not.toContain("%0A");
    expect(wsUrl).not.toContain("%0D");

    const protocols = client.wsProtocols();
    expect(protocols).toEqual(["gigi.v1", "gigi.apikey.key-with-padding"]);
    // No whitespace escaped into the subprotocol string either.
    for (const p of protocols) {
      expect(p).not.toMatch(/[\s]/);
    }
  });

  it("treats a key that is all whitespace as no credential", () => {
    const client = new SheetsClient({ baseUrl: "http://localhost:3142" });
    client.setApiKey("\n\t  \r\n");
    expect(client.hasApiKey()).toBe(false);
    expect(client.hasCredential()).toBe(false);
  });

  it("strips trailing newlines from a bearer token (HMAC tokens are even more fragile)", async () => {
    const fakeFetch = fetchSpy();
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    client.setBearerToken("eyJhbGciOiJIUzI1NiJ9.payload.sig\n");
    await client.listBundles();

    const calls = (fakeFetch as unknown as { mock: { calls: unknown[][] } }).mock.calls;
    expect(calls).toHaveLength(1);
    const init = calls[0][1] as RequestInit;
    const headers = (init.headers ?? {}) as Record<string, string>;
    expect(headers["authorization"]).toBe("Bearer eyJhbGciOiJIUzI1NiJ9.payload.sig");
    expect(headers["authorization"]).not.toContain("\n");
  });

  it("setApiKey(null) clears the credential", () => {
    const client = new SheetsClient({ baseUrl: "http://localhost:3142" });
    client.setApiKey("real-key\n");
    expect(client.hasApiKey()).toBe(true);
    client.setApiKey(null);
    expect(client.hasApiKey()).toBe(false);
  });
});
