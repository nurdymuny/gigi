import { describe, expect, it, vi } from "vitest";
import {
  SheetsClient,
  SheetsClientError,
  type Fetcher,
} from "../../src/lib/gigi-client";

function jsonResponse(payload: unknown, status = 200) {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { "content-type": "application/json" },
  });
}

describe("SheetsClient.createBundle", () => {
  it("POSTs the expected schema shape to /v1/bundles", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(jsonResponse({ status: "ok" })) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    await client.createBundle({
      name: "sensors",
      fields: { id: "text", temp: "numeric" },
      keys: ["id"],
      indexed: ["id"],
    });
    const [url, init] = (fakeFetch as unknown as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("http://localhost:3142/v1/bundles");
    expect(JSON.parse(init.body)).toEqual({
      name: "sensors",
      schema: {
        fields: { id: "text", temp: "numeric" },
        keys: ["id"],
        indexed: ["id"],
      },
    });
  });

  it("rejects unsafe bundle names without hitting the engine", async () => {
    const fakeFetch = vi.fn() as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    await expect(
      client.createBundle({
        name: "bad-name; DROP",
        fields: { id: "text" },
        keys: ["id"],
      }),
    ).rejects.toBeInstanceOf(SheetsClientError);
    expect(fakeFetch).not.toHaveBeenCalled();
  });
});

describe("SheetsClient.insert", () => {
  it("wraps a single record in { records: [...] }", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(jsonResponse({ status: "ok" })) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    await client.insert("sensors", { id: "S-001", temp: 22 });
    const [url, init] = (fakeFetch as unknown as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("http://localhost:3142/v1/bundles/sensors/insert");
    expect(JSON.parse(init.body)).toEqual({
      records: [{ id: "S-001", temp: 22 }],
    });
  });

  it("forwards an array of records as-is", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(jsonResponse({ status: "ok" })) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    await client.insert("sensors", [
      { id: "S-001", temp: 22 },
      { id: "S-002", temp: 23 },
    ]);
    const init = (fakeFetch as unknown as ReturnType<typeof vi.fn>).mock.calls[0][1];
    expect(JSON.parse(init.body).records).toHaveLength(2);
  });
});

describe("SheetsClient.deleteRow", () => {
  it("POSTs { key } to /v1/bundles/:name/delete", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(jsonResponse({ status: "ok" })) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    await client.deleteRow("sensors", { sensor_id: "S-001" });
    const [url, init] = (fakeFetch as unknown as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("http://localhost:3142/v1/bundles/sensors/delete");
    expect(JSON.parse(init.body)).toEqual({ key: { sensor_id: "S-001" } });
  });

  it("surfaces engine errors", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(new Response("not found", { status: 404 })) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    await expect(
      client.deleteRow("sensors", { sensor_id: "missing" }),
    ).rejects.toMatchObject({ code: "http_error", status: 404 });
  });
});
