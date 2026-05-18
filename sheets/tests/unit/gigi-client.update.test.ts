import { describe, expect, it, vi } from "vitest";
import {
  SheetsClient,
  SheetsClientError,
  type Fetcher,
} from "../../src/lib/gigi-client";

/**
 * S1 acceptance tests for SheetsClient.update.
 *
 * Wire contract per the audit (GIGI_SHEETS_SPRINT_SPEC_ADDENDUM_v0.1.md §Q2):
 *   POST /v1/bundles/{name}/update
 *   body:     { key, fields, returning, expected_version? }
 *   response: { status, data?, total, curvature, confidence, version? }
 */
describe("SheetsClient.update", () => {
  function mockResponse(payload: unknown, status = 200) {
    return vi.fn().mockResolvedValue(
      new Response(JSON.stringify(payload), {
        status,
        headers: { "content-type": "application/json" },
      }),
    ) as unknown as Fetcher;
  }

  it("POSTs key + fields + returning=true to /update and returns κ + confidence", async () => {
    const fakeFetch = mockResponse({
      status: "updated",
      data: { sensor_id: "S-001", temp: 24.0, humidity: 55 },
      total: 1284,
      curvature: 0.31,
      confidence: 0.76,
      version: 7,
    });
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });

    const result = await client.update("sensors", {
      key: { sensor_id: "S-001" },
      fields: { temp: 24.0 },
    });

    expect(result.status).toBe("updated");
    expect(result.data).toMatchObject({ sensor_id: "S-001", temp: 24.0 });
    expect(result.curvature).toBeCloseTo(0.31);
    expect(result.confidence).toBeCloseTo(0.76);
    expect(result.version).toBe(7);

    const [calledUrl, calledInit] = (fakeFetch as unknown as ReturnType<typeof vi.fn>)
      .mock.calls[0];
    expect(calledUrl).toBe("http://localhost:3142/v1/bundles/sensors/update");
    const body = JSON.parse((calledInit as RequestInit).body as string);
    expect(body).toEqual({
      key: { sensor_id: "S-001" },
      fields: { temp: 24.0 },
      returning: true,
    });
  });

  it("forwards expected_version when caller asks for optimistic concurrency", async () => {
    const fakeFetch = mockResponse({
      status: "updated",
      total: 1,
      curvature: 0.1,
      confidence: 0.9,
      version: 8,
    });
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    await client.update("sensors", {
      key: { sensor_id: "S-001" },
      fields: { temp: 24.0 },
      expected_version: 7,
    });
    const body = JSON.parse(
      (fakeFetch as unknown as ReturnType<typeof vi.fn>).mock.calls[0][1].body,
    );
    expect(body.expected_version).toBe(7);
  });

  it("throws code='no_key' when the key is empty", async () => {
    const fakeFetch = mockResponse({ status: "updated" });
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    await expect(
      client.update("sensors", { key: {}, fields: { temp: 1 } }),
    ).rejects.toMatchObject({
      name: "SheetsClientError",
      code: "no_key",
    });
    expect(fakeFetch).not.toHaveBeenCalled();
  });

  it("throws code='version_conflict' when the engine returns a conflict status", async () => {
    const fakeFetch = mockResponse({
      status: "version_conflict",
      total: 0,
      curvature: 0,
      confidence: 0,
    });
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    await expect(
      client.update("sensors", {
        key: { sensor_id: "S-001" },
        fields: { temp: 24 },
        expected_version: 3,
      }),
    ).rejects.toMatchObject({
      name: "SheetsClientError",
      code: "version_conflict",
    });
  });

  it("surfaces HTTP errors as SheetsClientError with the status code", async () => {
    const fakeFetch = mockResponse({ error: "bad fields" }, 400);
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    await expect(
      client.update("sensors", {
        key: { sensor_id: "S-001" },
        fields: { temp: NaN },
      }),
    ).rejects.toMatchObject({
      name: "SheetsClientError",
      code: "http_error",
      status: 400,
    });
  });

  it("instances are SheetsClientError, never raw Error", async () => {
    const fakeFetch = mockResponse({ status: "version_conflict" });
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    try {
      await client.update("sensors", {
        key: { sensor_id: "S-001" },
        fields: { temp: 1 },
      });
      throw new Error("should have thrown");
    } catch (err) {
      expect(err).toBeInstanceOf(SheetsClientError);
    }
  });
});
