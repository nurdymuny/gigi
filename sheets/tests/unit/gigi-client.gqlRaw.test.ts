import { describe, expect, it, vi } from "vitest";
import {
  SheetsClient,
  type Fetcher,
} from "../../src/lib/gigi-client";

function jsonResponse(payload: unknown, status = 200) {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { "content-type": "application/json" },
  });
}

describe("SheetsClient.gqlRaw", () => {
  it("POSTs {query} and returns the raw body + status + elapsed", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(jsonResponse({ rows: [{ a: 1 }], count: 1 })) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    const r = await client.gqlRaw("SECTION sensors LIMIT 1;");
    expect(r.status).toBe(200);
    expect(r.body).toEqual({ rows: [{ a: 1 }], count: 1 });
    expect(r.elapsedMs).toBeGreaterThanOrEqual(0);
    const [url, init] = (fakeFetch as unknown as ReturnType<typeof vi.fn>).mock
      .calls[0];
    expect(url).toBe("http://localhost:3142/v1/gql");
    expect(JSON.parse(init.body)).toEqual({ query: "SECTION sensors LIMIT 1;" });
  });

  it("does NOT throw on 4xx — returns the response with the status code", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(
        jsonResponse({ error: "Parse error: unexpected token" }, 400),
      ) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    const r = await client.gqlRaw("BAD GQL");
    expect(r.status).toBe(400);
    expect((r.body as { error: string }).error).toContain("Parse error");
  });

  it("does NOT throw on 5xx — returns the body for the UI to surface", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(jsonResponse({ error: "exec failed" }, 500)) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    const r = await client.gqlRaw("SECTION sensors;");
    expect(r.status).toBe(500);
    expect(r.body).toEqual({ error: "exec failed" });
  });

  it("returns body=null when the response is not JSON", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(
        new Response("<html>oops</html>", {
          status: 200,
          headers: { "content-type": "text/html" },
        }),
      ) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    const r = await client.gqlRaw("SECTION sensors;");
    expect(r.status).toBe(200);
    expect(r.body).toBeNull();
  });

  it("throws code='timeout' on abort", async () => {
    const fakeFetch: Fetcher = (_url, init) =>
      new Promise((_resolve, reject) => {
        init?.signal?.addEventListener("abort", () => {
          const err = new Error("aborted");
          err.name = "AbortError";
          reject(err);
        });
      });
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
      timeoutMs: 10,
    });
    await expect(client.gqlRaw("SECTION slow;")).rejects.toMatchObject({
      code: "timeout",
    });
  });
});
