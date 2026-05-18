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

describe("SheetsClient.listBundles", () => {
  it("GETs /v1/bundles and returns typed entries", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(
        jsonResponse([
          { name: "sensors", records: 1284, fields: 6 },
          { name: "marcella_source_claims", records: 2908, fields: 9 },
        ]),
      ) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    const r = await client.listBundles();
    expect(r).toHaveLength(2);
    expect(r[0]).toEqual({ name: "sensors", records: 1284, fields: 6 });
    const [url] = (fakeFetch as unknown as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("http://localhost:3142/v1/bundles");
  });

  it("defaults missing numeric fields to 0", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(jsonResponse([{ name: "x" }])) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    const r = await client.listBundles();
    expect(r[0]).toEqual({ name: "x", records: 0, fields: 0 });
  });

  it("drops entries with empty/missing names rather than passing junk through", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(
        jsonResponse([
          { name: "real", records: 1, fields: 1 },
          { records: 0 },
          { name: "", records: 0 },
        ]),
      ) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    const r = await client.listBundles();
    expect(r.map((b) => b.name)).toEqual(["real"]);
  });

  it("throws parse_error when the body isn't an array", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(jsonResponse({ not: "array" })) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    await expect(client.listBundles()).rejects.toMatchObject({
      code: "parse_error",
    });
  });

  it("surfaces 5xx as http_error with status code", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(new Response("boom", { status: 503 })) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    await expect(client.listBundles()).rejects.toMatchObject({
      code: "http_error",
      status: 503,
    });
  });
});
