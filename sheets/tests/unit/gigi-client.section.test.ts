import { describe, expect, it, vi } from "vitest";
import {
  SheetsClient,
  SheetsClientError,
  type Fetcher,
} from "../../src/lib/gigi-client";

/**
 * S0 acceptance tests for SheetsClient.section.
 *
 * These tests pin the contract this app expects from
 *   POST /v1/bundles/{name}/query
 * on gigi-stream. The shape was confirmed in the engine audit
 * (GIGI_SHEETS_SPRINT_SPEC_ADDENDUM_v0.1.md §Q1/Q2). When the engine
 * response shape changes, these tests should be updated in lockstep with
 * the SDK fixtures.
 */
describe("SheetsClient.section", () => {
  it("parses a happy-path SECTION response into rows + meta", async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          data: [
            { sensor_id: "S-001", temp: 22.5, humidity: 60.1 },
            { sensor_id: "S-002", temp: 19.3, humidity: 71.4 },
          ],
          total: 2,
          curvature: 0.12,
          confidence: 0.89,
        }),
        { status: 200, headers: { "content-type": "application/json" } },
      ),
    ) as unknown as Fetcher;

    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });

    const result = await client.section("sensors", { limit: 25 });

    expect(result.rows).toHaveLength(2);
    expect(result.rows[0]).toMatchObject({ sensor_id: "S-001", temp: 22.5 });
    expect(result.total).toBe(2);
    expect(result.curvature).toBeCloseTo(0.12);
    expect(result.confidence).toBeCloseTo(0.89);

    expect(fakeFetch).toHaveBeenCalledOnce();
    const [calledUrl, calledInit] = (fakeFetch as unknown as ReturnType<typeof vi.fn>)
      .mock.calls[0];
    expect(calledUrl).toBe("http://localhost:3142/v1/bundles/sensors/query");
    expect(calledInit).toMatchObject({ method: "POST" });
    expect(JSON.parse((calledInit as RequestInit).body as string)).toEqual({
      limit: 25,
    });
  });

  it("tolerates the alternate 'results' key (engine compat)", async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({ results: [{ sensor_id: "S-001" }], total: 1 }),
        { status: 200, headers: { "content-type": "application/json" } },
      ),
    ) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    const result = await client.section("sensors");
    expect(result.rows).toHaveLength(1);
  });

  it("throws a typed SheetsClientError with status on 4xx", async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response("not found", { status: 404 }),
    ) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });

    await expect(client.section("missing")).rejects.toMatchObject({
      name: "SheetsClientError",
      code: "http_error",
      status: 404,
    });
    await expect(client.section("missing")).rejects.toBeInstanceOf(
      SheetsClientError,
    );
  });

  it("throws code='timeout' when the request exceeds timeoutMs", async () => {
    // Resolves only on abort — guarantees the AbortController path runs.
    const fakeFetch: Fetcher = (_url, init) =>
      new Promise((_resolve, reject) => {
        const signal = init?.signal;
        if (!signal) return;
        signal.addEventListener("abort", () => {
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

    await expect(client.section("slow")).rejects.toMatchObject({
      name: "SheetsClientError",
      code: "timeout",
    });
  });

  it("throws code='parse_error' if the body is not JSON", async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response("<html>oops</html>", {
        status: 200,
        headers: { "content-type": "text/html" },
      }),
    ) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    await expect(client.section("sensors")).rejects.toMatchObject({
      name: "SheetsClientError",
      code: "parse_error",
    });
  });

  it("throws code='parse_error' if rows is not an array", async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ data: "not an array" }), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    ) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    await expect(client.section("sensors")).rejects.toMatchObject({
      code: "parse_error",
    });
  });

  it("defaults curvature and confidence to 0 if missing", async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ data: [], total: 0 }), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    ) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    const r = await client.section("sensors");
    expect(r.curvature).toBe(0);
    expect(r.confidence).toBe(0);
  });
});
